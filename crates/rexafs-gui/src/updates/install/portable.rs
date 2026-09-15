//! Native Windows installer updates and Windows/Linux folder replacement with rollback.
use super::{
    UpdateHandoff,
    common::{hash_file, run},
    payload, replace_and_launch,
};
use crate::updates::{self, AvailableRelease, UpdateChannel};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File, OpenOptions},
    path::{Path, PathBuf},
    process::Command,
    thread,
    time::{Duration, Instant},
};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Plan {
    target: PathBuf,
    parent_pid: u32,
    previous_hash: String,
    manifest_hash: String,
    tag: String,
    channel: UpdateChannel,
    setup_hash: Option<String>,
}

pub(crate) struct PreparedUpdate {
    directory: tempfile::TempDir,
    plan: Plan,
    lock: File,
}

fn binary_name() -> &'static str {
    if cfg!(windows) {
        "rexafs.exe"
    } else {
        "rexafs"
    }
}

pub(crate) fn installed_app() -> Result<PathBuf, String> {
    let executable = std::env::current_exe()
        .map_err(|e| e.to_string())?
        .canonicalize()
        .map_err(|e| e.to_string())?;
    let target = executable
        .parent()
        .ok_or("Missing app folder")?
        .to_path_buf();
    if executable.file_name().is_none_or(|n| n != binary_name())
        || !target.join(payload::MANIFEST).is_file()
    {
        return Err(
            "Install an official rexafs 0.2.8 or newer package to enable Update and restart."
                .into(),
        );
    }
    verify_identity(
        &target,
        updates::installed_tag(),
        updates::installed_channel(),
    )?;
    Ok(target)
}

fn verify_identity(root: &Path, tag: &str, channel: UpdateChannel) -> Result<(), String> {
    let path = root.join("build.json");
    if payload::regular_metadata(&path)?.len() > 64 * 1024 {
        return Err("Invalid package metadata size".into());
    }
    let info: serde_json::Value =
        serde_json::from_slice(&fs::read(path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    if info["release_tag"] != tag
        || info["channel"] != serde_json::to_value(channel).map_err(|e| e.to_string())?
        || Some(info["target"].as_str().unwrap_or("")) != updates::desktop_target()
    {
        return Err("The app package does not match this release, channel or architecture.".into());
    }
    Ok(())
}

fn verify_runtime(root: &Path, tag: &str, channel: UpdateChannel) -> Result<(), String> {
    verify_identity(root, tag, channel)?;
    verify_architecture(&root.join(binary_name()))?;
    let info: serde_json::Value = serde_json::from_slice(&run(Command::new(
        root.join(binary_name()),
    )
    .arg("--build-info"))?)
    .map_err(|e| e.to_string())?;
    if info["release_tag"] != tag
        || info["channel"] != serde_json::to_value(channel).map_err(|e| e.to_string())?
    {
        return Err("The downloaded executable does not match the selected release.".into());
    }
    run(Command::new(root.join(binary_name())).arg("--self-check"))?;
    Ok(())
}

fn verify_architecture(executable: &Path) -> Result<(), String> {
    use std::io::{Read, Seek, SeekFrom};
    let mut file = File::open(executable).map_err(|e| e.to_string())?;
    let mut header = [0u8; 64];
    file.read_exact(&mut header).map_err(|e| e.to_string())?;
    if cfg!(windows) {
        let offset = u32::from_le_bytes(header[60..64].try_into().unwrap()) as u64;
        if &header[..2] != b"MZ" || !(64..=1024 * 1024).contains(&offset) {
            return Err("Invalid Windows executable header".into());
        }
        file.seek(SeekFrom::Start(offset))
            .map_err(|e| e.to_string())?;
        let mut pe = [0u8; 6];
        file.read_exact(&mut pe).map_err(|e| e.to_string())?;
        let expected = if std::env::consts::ARCH == "aarch64" {
            0xaa64
        } else {
            0x8664
        };
        if &pe[..4] != b"PE\0\0" || u16::from_le_bytes([pe[4], pe[5]]) != expected {
            return Err("The Windows executable has the wrong architecture.".into());
        }
    } else {
        let expected = if std::env::consts::ARCH == "aarch64" {
            183
        } else {
            62
        };
        if &header[..4] != b"\x7fELF"
            || header[4] != 2
            || header[5] != 1
            || u16::from_le_bytes([header[18], header[19]]) != expected
        {
            return Err("The Linux executable has the wrong architecture.".into());
        }
    }
    Ok(())
}

pub(crate) fn prepare(
    release: &AvailableRelease,
    archive: &Path,
    mut cancelled: impl FnMut() -> Result<(), String>,
) -> Result<PreparedUpdate, String> {
    let target = installed_app()?;
    let channel = if release.tag.starts_with("nightly-") {
        UpdateChannel::Nightly
    } else {
        UpdateChannel::Stable
    };
    if channel != updates::installed_channel() {
        return Err("Download the other channel to install it separately.".into());
    }
    let lock = installation_lock(&target)?;
    cancelled()?;
    payload::verify(&target, true)?;
    let directory = tempfile::Builder::new()
        .prefix(".rexafs-update-")
        .tempdir_in(target.parent().ok_or("Missing app parent")?)
        .map_err(|e| {
            format!("The app folder must be writable to update without administrator access: {e}")
        })?;
    let asset = release.asset.as_ref().ok_or("Missing update archive")?;
    // Copy the verified cache into the private transaction and check it again
    // before extraction; no executable is run from the shared download cache.
    let private_archive = directory.path().join("download");
    fs::copy(archive, &private_archive).map_err(|e| e.to_string())?;
    updates::verify_cached_download(
        &private_archive,
        asset.size,
        updates::checksum(asset)?,
        |_, _| cancelled(),
    )?;
    let incoming = directory.path().join("incoming");
    payload::extract(&private_archive, &asset.name, &incoming)?;
    cancelled()?;
    verify_runtime(&incoming, &release.tag, channel)?;
    payload::preserve_extras(&target, &incoming, false)?;
    #[cfg(windows)]
    let setup_hash = if super::windows::registered_install(&target)? {
        let setup = release
            .installer
            .as_ref()
            .ok_or("The matching Windows installer is not published yet.")?;
        let mut setup_release = release.clone();
        setup_release.asset = Some(setup.clone());
        let download = updates::download_with_progress(&setup_release, |_, _| cancelled())?;
        let private = directory.path().join("setup.exe");
        fs::copy(download, &private).map_err(|e| e.to_string())?;
        updates::verify_cached_download(
            &private,
            setup.size,
            updates::checksum(setup)?,
            |_, _| cancelled(),
        )?;
        Some(hash_file(&private)?)
    } else {
        None
    };
    #[cfg(not(windows))]
    let setup_hash = None;
    cancelled()?;
    let plan = Plan {
        previous_hash: hash_file(&target.join(binary_name()))?,
        manifest_hash: hash_file(&incoming.join(payload::MANIFEST))?,
        target,
        parent_pid: std::process::id(),
        tag: release.tag.clone(),
        channel,
        setup_hash,
    };
    Ok(PreparedUpdate {
        directory,
        plan,
        lock,
    })
}

impl PreparedUpdate {
    pub(crate) fn recovery_path(&self) -> PathBuf {
        self.directory.path().join("Update recovery.rxs")
    }

    pub(crate) fn start(self) -> Result<UpdateHandoff, String> {
        let root = self.directory.keep();
        let helper_dir = root.join("helper");
        fs::create_dir(&helper_dir).map_err(|e| e.to_string())?;
        let helper = helper_dir.join(binary_name());
        fs::copy(self.plan.target.join(binary_name()), &helper).map_err(|e| e.to_string())?;
        #[cfg(windows)]
        for entry in fs::read_dir(&self.plan.target).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            if entry
                .path()
                .extension()
                .and_then(std::ffi::OsStr::to_str)
                .is_some_and(|e| e.eq_ignore_ascii_case("dll"))
            {
                payload::regular_metadata(&entry.path())?;
                fs::copy(entry.path(), helper_dir.join(entry.file_name()))
                    .map_err(|e| e.to_string())?;
            }
        }
        fs::write(
            root.join("plan.json"),
            serde_json::to_vec(&self.plan).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        let log = File::create(root.join("install.log")).map_err(|e| e.to_string())?;
        let mut child = Command::new(helper)
            .arg("--finish-update")
            .arg(&root)
            .current_dir(&root)
            .stdin(std::process::Stdio::null())
            .stdout(log.try_clone().map_err(|e| e.to_string())?)
            .stderr(log)
            .spawn()
            .map_err(|e| format!("Cannot start the update helper: {e}"))?;
        let started = Instant::now();
        loop {
            if child.try_wait().map_err(|e| e.to_string())?.is_some() {
                return Err(format!(
                    "The update helper stopped. Your app is unchanged; details: {}",
                    root.join("install.log").display()
                ));
            }
            if root.join("ready").is_file() {
                return Ok(UpdateHandoff {
                    _directory: root,
                    _lock: Some(self.lock),
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

pub(crate) fn finish_update(root: &Path) -> Result<(), String> {
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    if std::env::current_exe()
        .map_err(|e| e.to_string())?
        .canonicalize()
        .map_err(|e| e.to_string())?
        != root.join("helper").join(binary_name())
    {
        return Err("Run updates only from the private helper copy.".into());
    }
    let bytes = fs::read(root.join("plan.json")).map_err(|e| e.to_string())?;
    if bytes.len() > 16 * 1024 {
        return Err("Invalid update plan size".into());
    }
    let plan: Plan = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    if plan.target.parent() != root.parent()
        || plan.target == root
        || plan.parent_pid <= 1
        || plan.target.canonicalize().map_err(|e| e.to_string())? != plan.target
        || hash_file(&std::env::current_exe().map_err(|e| e.to_string())?)? != plan.previous_hash
        || !root.join("Update recovery.rxs").is_file()
    {
        return Err("Invalid update transaction".into());
    }
    // Open the Windows process handle before acknowledging readiness, avoiding
    // PID reuse between GUI exit and the helper's first wait.
    #[cfg(windows)]
    let parent = super::windows::ParentProcess::open(plan.parent_pid)?;
    fs::write(root.join("ready"), b"ready").map_err(|e| e.to_string())?;
    #[cfg(windows)]
    parent.wait()?;
    #[cfg(not(windows))]
    wait_for_exit(plan.parent_pid)?;
    let _lock = installation_lock(&plan.target)?;
    let incoming = root.join("incoming");
    let previous = root.join("previous");
    let recovery = root.join("Update recovery.rxs");
    let mut replacing = false;
    let result = (|| {
        if hash_file(&plan.target.join(binary_name()))? != plan.previous_hash
            || hash_file(&incoming.join(payload::MANIFEST))? != plan.manifest_hash
        {
            return Err("The installed app or update changed; replacement was cancelled.".into());
        }
        payload::verify(&plan.target, true)?;
        payload::verify(&incoming, false)?;
        verify_runtime(&incoming, &plan.tag, plan.channel)?;
        payload::preserve_extras(&plan.target, &incoming, false)?;
        #[cfg(windows)]
        if let Some(hash) = &plan.setup_hash {
            if !super::windows::registered_install(&plan.target)?
                || hash_file(&root.join("setup.exe"))? != *hash
            {
                return Err(
                    "The installer or Windows registration changed; update cancelled.".into(),
                );
            }
            // Copy rather than hard-link: Setup overwrites files in place.
            payload::copy_tree(&plan.target, &previous)?;
            super::windows::backup_registration(&root)?;
            replacing = true;
            return super::windows::install(&root, &plan.target, || {
                payload::retire_obsolete(&previous, &plan.target, &root.join("retired"))?;
                payload::verify(&plan.target, true)?;
                if hash_file(&plan.target.join(payload::MANIFEST))? != plan.manifest_hash {
                    return Err("Installer payload differs from the verified archive".into());
                }
                verify_runtime(&plan.target, &plan.tag, plan.channel)?;
                launch(&plan.target, &recovery)
            });
        }
        payload::preserve_extras(&plan.target, &incoming, true)?;
        replacing = true;
        replace_and_launch(&plan.target, &incoming, &previous, |target| {
            launch(target, &recovery)
        })
    })();
    fs::write(
        root.join("result.txt"),
        result.as_ref().map_or_else(
            |e| format!("Update failed: {e}"),
            |_| {
                format!(
                    "Installed {}. Previous app and recovery project retained here.",
                    plan.tag
                )
            },
        ),
    )
    .map_err(|e| e.to_string())?;
    if result.is_err() && !replacing {
        let _ = launch(&plan.target, &recovery);
    }
    result
}

pub(super) fn launch(target: &Path, recovery: &Path) -> Result<(), String> {
    let log = File::create(recovery.with_file_name("reopened.log")).map_err(|e| e.to_string())?;
    let mut child = Command::new(target.join(binary_name()))
        .arg(recovery)
        .current_dir(target)
        .stdin(std::process::Stdio::null())
        .stdout(log.try_clone().map_err(|e| e.to_string())?)
        .stderr(log)
        .spawn()
        .map_err(|e| e.to_string())?;
    // The receipt supports troubleshooting without collecting any project data.
    let _ = fs::write(
        recovery.with_file_name("launched-pid.txt"),
        child.id().to_string(),
    );
    // Catch loader errors and immediate startup failures before committing.
    thread::sleep(Duration::from_millis(750));
    if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
        return Err(format!("The updated app exited during startup ({status})"));
    }
    Ok(())
}

fn installation_lock(target: &Path) -> Result<File, String> {
    let name = target
        .file_name()
        .ok_or("Missing app name")?
        .to_string_lossy();
    let path = target.with_file_name(format!(".{name}.update-lock"));
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.custom_flags(0x00200000);
    }
    let lock = options
        .open(&path)
        .map_err(|e| format!("Cannot update this app folder: {e}"))?;
    if !payload::regular_metadata(&path)?.is_file() {
        return Err("Invalid update lock".into());
    }
    lock.try_lock()
        .map_err(|_| "Another rexafs update is already using this application.")?;
    Ok(lock)
}

#[cfg(not(windows))]
fn wait_for_exit(pid: u32) -> Result<(), String> {
    let started = Instant::now();
    loop {
        // SAFETY: signal 0 only tests existence; it never signals or kills.
        if unsafe { libc::kill(pid as libc::pid_t, 0) } != 0
            && std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH)
        {
            return Ok(());
        }
        if started.elapsed() > Duration::from_secs(60) {
            return Err("rexafs did not quit; the app is unchanged.".into());
        }
        thread::sleep(Duration::from_millis(100));
    }
}
