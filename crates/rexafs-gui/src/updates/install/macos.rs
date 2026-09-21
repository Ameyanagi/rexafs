use super::{
    UpdateHandoff, app_for_executable,
    common::{hash_file, run},
    replace_and_launch,
};
use crate::updates::{AvailableRelease, UpdateChannel};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File, OpenOptions},
    path::{Component, Path, PathBuf},
    process::Command,
    thread,
    time::{Duration, Instant},
};

// Official Developer ID used by the signed Stable and Nightly release pipelines.
// Never accept an arbitrary valid Apple signature from a different publisher.
const TEAM: &str = "XXN44W8X56";

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Plan {
    target: PathBuf,
    staged: PathBuf,
    previous: PathBuf,
    parent_pid: u32,
    previous_hash: String,
    tag: String,
    channel: UpdateChannel,
}

pub(crate) struct PreparedUpdate {
    directory: tempfile::TempDir,
    plan: Plan,
    lock: File,
}

/// Build a private transaction beside the installed app. This proves that the
/// destination is writable without requesting administrator access, and keeps
/// the eventual bundle renames on one filesystem.
pub(crate) fn prepare(
    release: &AvailableRelease,
    archive: &Path,
    mut cancelled: impl FnMut() -> Result<(), String>,
) -> Result<PreparedUpdate, String> {
    let executable = std::env::current_exe().map_err(|e| e.to_string())?;
    let target = app_for_executable(&executable)?
        .canonicalize()
        .map_err(|e| e.to_string())?;
    let parent = target
        .parent()
        .ok_or("The installed app has no parent folder")?;
    let lock = installation_lock(&target)?;
    let channel = if release.tag.starts_with("nightly-") {
        UpdateChannel::Nightly
    } else {
        UpdateChannel::Stable
    };
    if channel != crate::updates::installed_channel() {
        return Err("Use the download to install the other channel beside this app.".into());
    }
    let directory = tempfile::Builder::new()
        .prefix(".rexafs-update-")
        .tempdir_in(parent)
        .map_err(|e| {
            format!(
                "Cannot update this app in {}. Move it to a writable Applications folder: {e}",
                parent.display()
            )
        })?;
    cancelled()?;
    validate_archive(archive)?;
    let extracted = directory.path().join("extracted");
    fs::create_dir(&extracted).map_err(|e| e.to_string())?;
    run(Command::new("/usr/bin/ditto")
        .args(["-x", "-k"])
        .arg(archive)
        .arg(&extracted))?;
    let apps = walkdir::WalkDir::new(&extracted)
        .min_depth(1)
        .max_depth(3)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| {
            entry.file_type().is_dir() && entry.path().extension().is_some_and(|ext| ext == "app")
        })
        .map(|entry| entry.into_path())
        .collect::<Vec<_>>();
    let [app] = apps.as_slice() else {
        return Err("The download must contain exactly one rexafs app.".into());
    };
    cancelled()?;
    verify_app(app, &release.tag, channel)?;
    // Detect incomplete packages before touching the installed app.
    run(Command::new(app.join("Contents/MacOS/rexafs")).arg("--self-check"))?;
    let staged = directory.path().join("incoming.app");
    fs::rename(app, &staged).map_err(|e| e.to_string())?;
    Ok(PreparedUpdate {
        plan: Plan {
            target,
            staged,
            previous: directory.path().join("previous.app"),
            parent_pid: std::process::id(),
            previous_hash: hash_file(&executable)?,
            tag: release.tag.clone(),
            channel,
        },
        directory,
        lock,
    })
}

impl PreparedUpdate {
    pub(crate) fn recovery_path(&self) -> PathBuf {
        self.directory.path().join("Update recovery.rxs")
    }

    /// Arm a separate copy of this executable. The caller quits only after the
    /// helper acknowledges the validated plan. Recovery and the old bundle are
    /// retained in the private transaction directory, including after errors.
    pub(crate) fn start(self) -> Result<UpdateHandoff, String> {
        let root = self.directory.keep();
        let helper = stage_helper(&self.plan.target, &root, &self.plan.previous_hash)?;
        fs::write(
            root.join("plan.json"),
            serde_json::to_vec(&self.plan).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        let log = File::create(root.join("install.log")).map_err(|e| e.to_string())?;
        let mut child = Command::new(helper)
            .arg("--finish-update")
            .arg(&root)
            // flock is held across the handoff by the inherited open file
            // description. The helper never reads stdin.
            .stdin(self.lock)
            .stdout(log.try_clone().map_err(|e| e.to_string())?)
            .stderr(log)
            .spawn()
            .map_err(|e| format!("Cannot start the update helper: {e}"))?;
        let started = Instant::now();
        loop {
            if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
                return Err(format!(
                    "The update helper stopped ({status}). Your app is unchanged; details: {}",
                    root.join("install.log").display()
                ));
            }
            if root.join("ready").is_file() {
                return Ok(UpdateHandoff {
                    _directory: root,
                    _lock: None,
                });
            }
            if started.elapsed() > Duration::from_secs(15) {
                let _ = child.kill();
                let _ = child.wait();
                return Err(
                    "The update helper did not become ready. Your app is unchanged.".into(),
                );
            }
            thread::sleep(Duration::from_millis(50));
        }
    }
}

fn helper_executable(root: &Path) -> PathBuf {
    root.join("helper.app/Contents/MacOS/rexafs")
}

/// Preserve the signed Info.plist and resource seal when moving the helper.
/// A bare executable copied out of a Developer ID bundle is rejected by macOS.
fn stage_helper(target: &Path, root: &Path, expected_hash: &str) -> Result<PathBuf, String> {
    let bundle = root.join("helper.app");
    run(Command::new("/usr/bin/ditto").arg(target).arg(&bundle))?;
    run(Command::new("/usr/bin/codesign")
        .args(["--verify", "--deep", "--strict"])
        .arg(&bundle))?;
    let helper = helper_executable(root);
    if hash_file(&helper)? != expected_hash {
        return Err("The updater copy differs from the running app; update cancelled.".into());
    }
    Ok(helper)
}

/// Exercise helper copying and launch from an installed, signed package.
/// This checks the same helper layout used by updates without replacing an app.
pub(crate) fn check_helper() -> Result<(), String> {
    let executable = std::env::current_exe().map_err(|e| e.to_string())?;
    let target = app_for_executable(&executable)?;
    let directory = tempfile::Builder::new()
        .prefix("rexafs-updater-check-")
        .tempdir()
        .map_err(|e| e.to_string())?;
    let helper = stage_helper(&target, directory.path(), &hash_file(&executable)?)?;
    let info: serde_json::Value =
        serde_json::from_slice(&run(Command::new(helper).arg("--build-info"))?)
            .map_err(|e| e.to_string())?;
    if info != crate::updates::build_info() {
        return Err("The updater helper reported a different build identity.".into());
    }
    Ok(())
}

/// Internal helper entry point, executed before any GUI or Assistant starts.
pub(crate) fn finish_update(root: &Path) -> Result<(), String> {
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    if std::env::current_exe()
        .map_err(|e| e.to_string())?
        .canonicalize()
        .map_err(|e| e.to_string())?
        != helper_executable(&root)
    {
        return Err("Update installation must run from its private helper copy.".into());
    }
    let bytes = fs::read(root.join("plan.json")).map_err(|e| e.to_string())?;
    if bytes.len() > 16 * 1024 {
        return Err("Invalid update plan size".into());
    }
    let plan: Plan = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    if plan.target.parent() != root.parent()
        || plan.staged != root.join("incoming.app")
        || plan.previous != root.join("previous.app")
        || plan.parent_pid <= 1
        || plan.target.extension().is_none_or(|ext| ext != "app")
        || plan.target.canonicalize().map_err(|e| e.to_string())? != plan.target
        || hash_file(&helper_executable(&root))? != plan.previous_hash
    {
        return Err("Invalid update transaction paths".into());
    }
    use std::os::{fd::FromRawFd, unix::fs::MetadataExt};
    // SAFETY: F_GETFD inspects a descriptor without modifying it.
    if unsafe { libc::fcntl(libc::STDIN_FILENO, libc::F_GETFD) } < 0 {
        return Err("The helper has no inherited update lock.".into());
    }
    // SAFETY: stdin is an open descriptor inherited from our launcher. Own it
    // for this helper's lifetime; it is never read or accessed through std::io.
    let lock = unsafe { File::from_raw_fd(libc::STDIN_FILENO) };
    let lock_metadata = lock.metadata().map_err(|e| e.to_string())?;
    let name = plan
        .target
        .file_name()
        .ok_or("Missing app name")?
        .to_string_lossy();
    let expected_lock =
        fs::symlink_metadata(plan.target.with_file_name(format!(".{name}.update-lock")))
            .map_err(|e| e.to_string())?;
    if !lock_metadata.is_file()
        || expected_lock.file_type().is_symlink()
        || lock_metadata.ino() != expected_lock.ino()
        || lock_metadata.dev() != expected_lock.dev()
    {
        return Err("The helper did not inherit the application update lock.".into());
    }
    let recovery = root.join("Update recovery.rxs");
    if !recovery.is_file() {
        return Err("The recovery project is missing; the app was not replaced.".into());
    }
    fs::write(root.join("ready"), b"ready").map_err(|e| e.to_string())?;
    let started = Instant::now();
    loop {
        // SAFETY: signal 0 only checks whether the parent still exists; no
        // signal is delivered, and the updater never kills the user's app.
        let exists = unsafe { libc::kill(plan.parent_pid as libc::pid_t, 0) } == 0;
        if !exists && std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH) {
            break;
        }
        if started.elapsed() > Duration::from_secs(60) {
            return Err("rexafs did not quit; the installed app was left unchanged.".into());
        }
        thread::sleep(Duration::from_millis(100));
    }
    let mut replacing = false;
    let result = (|| {
        if hash_file(&plan.target.join("Contents/MacOS/rexafs"))? != plan.previous_hash {
            return Err(
                "The installed app changed during the download; it was not replaced.".into(),
            );
        }
        verify_app(&plan.staged, &plan.tag, plan.channel)?;
        replacing = true;
        replace_and_launch(&plan.target, &plan.staged, &plan.previous, |target| {
            run(Command::new("/usr/bin/open")
                .arg("-n")
                .arg(target)
                .arg("--args")
                .arg(&recovery))
            .map(|_| ())
        })
    })();
    fs::write(
        root.join("result.txt"),
        result.as_ref().map_or_else(
            |error| format!("Update failed: {error}"),
            |_| {
                format!(
                    "Installed {}. Previous app and recovery project retained here.",
                    plan.tag
                )
            },
        ),
    )
    .map_err(|e| e.to_string())?;
    if result.is_err() && !replacing && plan.target.exists() && !plan.previous.exists() {
        // Validation can fail after the parent has exited. Reopen the intact
        // app as well as the recovery project rather than leaving no window.
        let _ = run(Command::new("/usr/bin/open")
            .arg("-n")
            .arg(&plan.target)
            .arg("--args")
            .arg(&recovery));
    }
    result
}

pub(crate) fn installation_lock(target: &Path) -> Result<File, String> {
    use std::os::{fd::AsRawFd, unix::fs::OpenOptionsExt};
    let name = target
        .file_name()
        .ok_or("Missing application name")?
        .to_string_lossy();
    let path = target.with_file_name(format!(".{name}.update-lock"));
    // Do not unlink this file: simultaneous updaters must lock the same inode.
    // O_NOFOLLOW rejects a symlink in a writable application directory.
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)
        .map_err(|e| format!("Cannot update this app folder: {e}"))?;
    // SAFETY: flock operates on our owned, live file descriptor. Closing all
    // copies (including the helper's stdin) releases the kernel-managed lock.
    if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
        return Err("Another rexafs update is already using this application.".into());
    }
    Ok(lock)
}

fn verify_app(app: &Path, tag: &str, channel: UpdateChannel) -> Result<(), String> {
    let identifier = match channel {
        UpdateChannel::Stable => "com.rexafs.desktop",
        UpdateChannel::Nightly => "com.rexafs.nightly",
    };
    let requirement = format!(
        "=anchor apple generic and identifier \"{identifier}\" and certificate leaf[subject.OU] = \"{TEAM}\" and certificate leaf[field.1.2.840.113635.100.6.1.13] exists"
    );
    run(Command::new("/usr/bin/codesign")
        .args(["--verify", "--deep", "--strict", "-R"])
        .arg(requirement)
        .arg(app))?;
    run(Command::new("/usr/sbin/spctl")
        .args(["--assess", "--type", "execute"])
        .arg(app))?;
    let executable = app.join("Contents/MacOS/rexafs");
    let arch = if std::env::consts::ARCH == "aarch64" {
        "arm64"
    } else {
        "x86_64"
    };
    run(Command::new("/usr/bin/lipo")
        .arg(&executable)
        .arg("-verify_arch")
        .arg(arch))?;
    let info: serde_json::Value =
        serde_json::from_slice(&run(Command::new(executable).arg("--build-info"))?)
            .map_err(|e| e.to_string())?;
    if info["release_tag"] != tag || info["channel"] != serde_json::to_value(channel).unwrap() {
        return Err("The signed app does not match the requested release/channel.".into());
    }
    Ok(())
}

fn validate_archive(archive: &Path) -> Result<(), String> {
    let listing = run(Command::new("/usr/bin/zipinfo").arg("-1").arg(archive))?;
    let listing = std::str::from_utf8(&listing).map_err(|e| e.to_string())?;
    if listing.is_empty()
        || listing.lines().any(|name| {
            Path::new(name).components().any(|part| {
                matches!(
                    part,
                    Component::ParentDir | Component::RootDir | Component::Prefix(_)
                )
            })
        })
    {
        return Err("The update archive contains an unsafe path.".into());
    }
    let details = run(Command::new("/usr/bin/zipinfo").arg("-l").arg(archive))?;
    if String::from_utf8_lossy(&details).lines().any(|line| {
        matches!(
            line.as_bytes().first(),
            Some(b'l' | b'b' | b'c' | b'p' | b's')
        )
    }) {
        return Err("The update archive contains unsupported links or special files.".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn helper_preserves_signed_bundle_context_and_rejects_changed_resources() {
        let directory = tempfile::tempdir().unwrap();
        let app = directory.path().join("source.app");
        let binary = app.join("Contents/MacOS/rexafs");
        fs::create_dir_all(binary.parent().unwrap()).unwrap();
        let source = directory.path().join("helper.c");
        fs::write(
            &source,
            "#include <stdio.h>\nint main(void) { puts(\"helper-ok\"); return 0; }\n",
        )
        .unwrap();
        run(Command::new("/usr/bin/cc")
            .arg(&source)
            .arg("-o")
            .arg(&binary))
        .unwrap();
        let info = app.join("Contents/Info.plist");
        fs::write(
            &info,
            r#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0"><dict>
<key>CFBundleIdentifier</key><string>org.rexafs.test.updater</string>
<key>CFBundleExecutable</key><string>rexafs</string>
<key>CFBundlePackageType</key><string>APPL</string>
</dict></plist>"#,
        )
        .unwrap();
        run(Command::new("/usr/bin/codesign")
            .args(["--force", "--sign", "-"])
            .arg(&app))
        .unwrap();
        let expected = hash_file(&binary).unwrap();

        // This is the previous layout: the executable alone loses Info.plist's seal.
        let bare = directory.path().join("bare-helper");
        fs::copy(&binary, &bare).unwrap();
        assert!(
            run(Command::new("/usr/bin/codesign")
                .args(["--verify", "--strict"])
                .arg(&bare))
            .is_err()
        );

        let transaction = directory.path().join("transaction");
        fs::create_dir(&transaction).unwrap();
        let helper = stage_helper(&app, &transaction, &expected).unwrap();
        assert_eq!(run(&mut Command::new(&helper)).unwrap(), b"helper-ok\n");
        assert_eq!(
            fs::read(
                helper
                    .parent()
                    .unwrap()
                    .parent()
                    .unwrap()
                    .join("Info.plist")
            )
            .unwrap(),
            fs::read(&info).unwrap()
        );

        fs::write(&info, "changed after signing").unwrap();
        let tampered = directory.path().join("tampered");
        fs::create_dir(&tampered).unwrap();
        assert!(stage_helper(&app, &tampered, &expected).is_err());
    }

    #[test]
    fn concurrent_updates_cannot_prepare_the_same_app() {
        let directory = tempfile::tempdir().unwrap();
        let target = directory.path().join("rexafs.app");
        let lock = installation_lock(&target).unwrap();
        assert!(installation_lock(&target).is_err());
        drop(lock);
        assert!(installation_lock(&target).is_ok());
    }

    #[test]
    fn untrusted_bundle_is_rejected_before_its_executable_runs() {
        use std::os::unix::fs::PermissionsExt;
        let directory = tempfile::tempdir().unwrap();
        let app = directory.path().join("untrusted.app");
        let binary = app.join("Contents/MacOS/rexafs");
        fs::create_dir_all(binary.parent().unwrap()).unwrap();
        // If verification accidentally executes the program first, the marker
        // exposes that error. No downloaded or user-owned executable is used.
        fs::write(
            &binary,
            "#!/bin/sh\nprintf ran > \"$(dirname \"$0\")/executed\"\n",
        )
        .unwrap();
        fs::set_permissions(&binary, fs::Permissions::from_mode(0o755)).unwrap();
        assert!(verify_app(&app, "v0.2.7", UpdateChannel::Stable).is_err());
        assert!(!binary.with_file_name("executed").exists());
    }

    #[test]
    fn archives_with_symbolic_links_are_rejected_before_extraction() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("source");
        fs::create_dir(&source).unwrap();
        fs::write(source.join("data"), b"safe file").unwrap();
        let good = directory.path().join("good.zip");
        run(Command::new("/usr/bin/ditto")
            .args(["-c", "-k", "--keepParent"])
            .arg(&source)
            .arg(&good))
        .unwrap();
        validate_archive(&good).unwrap();
        std::os::unix::fs::symlink("../outside", source.join("link")).unwrap();
        let bad = directory.path().join("bad.zip");
        run(Command::new("/usr/bin/ditto")
            .args(["-c", "-k", "--keepParent"])
            .arg(&source)
            .arg(&bad))
        .unwrap();
        assert!(validate_archive(&bad).unwrap_err().contains("links"));
    }
}
