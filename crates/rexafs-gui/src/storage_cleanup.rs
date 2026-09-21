//! Inspect and remove managed installer downloads and inactive updater copies.
//! Scientific data, project extraction folders, settings and recovery files are
//! never cleanup candidates. Scans and removals run on a background worker.

use std::{
    fs,
    path::{Path, PathBuf},
    time::SystemTime,
};

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Candidate {
    pub path: PathBuf,
    pub bytes: u64,
    modified: SystemTime,
    files: u64,
    target: Option<PathBuf>,
}

#[derive(Clone, Default)]
pub(crate) struct Report {
    pub folders: Vec<PathBuf>,
    pub candidates: Vec<Candidate>,
    pub warnings: Vec<String>,
}

impl Report {
    pub fn bytes(&self) -> u64 {
        self.candidates.iter().map(|item| item.bytes).sum()
    }
}

pub(crate) fn size(bytes: u64) -> String {
    if bytes >= 1024 * 1024 * 1024 {
        format!("{:.2} GiB", bytes as f64 / (1024. * 1024. * 1024.))
    } else {
        format!("{:.1} MiB", bytes as f64 / (1024. * 1024.))
    }
}

/// Serialize cleanup with this version's verified-download workers. The lock
/// file is retained so concurrent processes keep referring to the same inode.
pub(crate) fn download_lock(root: &Path) -> Result<fs::File, String> {
    let path = root.join("installer-cache.lock");
    let mut options = fs::OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
    }
    if fs::symlink_metadata(&path).is_ok_and(|m| m.file_type().is_symlink()) {
        return Err("Installer cache lock cannot be a symbolic link.".into());
    }
    let file = options.open(path).map_err(|e| e.to_string())?;
    file.try_lock()
        .map_err(|_| "An installer download or cleanup is already running.".to_string())?;
    Ok(file)
}

fn real_directory(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|m| m.is_dir() && !m.file_type().is_symlink())
}

fn release_tag(name: &str) -> bool {
    name.strip_prefix('v')
        .is_some_and(|v| semver::Version::parse(v).is_ok())
        || name.strip_prefix("nightly-").is_some_and(|tail| {
            let mut parts = tail.split('-');
            parts
                .next()
                .is_some_and(|date| date.len() == 8 && date.bytes().all(|b| b.is_ascii_digit()))
                && parts
                    .next()
                    .is_some_and(|id| !id.is_empty() && id.bytes().all(|b| b.is_ascii_digit()))
                && parts.next().is_none()
        })
}

fn archive(name: &str) -> bool {
    name.starts_with("rexafs-")
        && [".zip", ".dmg", ".exe", ".msi", ".tar.gz"]
            .iter()
            .any(|suffix| name.ends_with(suffix))
}

#[cfg(target_os = "macos")]
fn extracted_download(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    let version = name.strip_prefix("rexafs-").and_then(|s| {
        s.strip_suffix("-aarch64-apple-darwin")
            .or_else(|| s.strip_suffix("-x86_64-apple-darwin"))
    });
    if !real_directory(path) || !version.is_some_and(|v| semver::Version::parse(v).is_ok()) {
        return false;
    }
    let Ok(entries) = fs::read_dir(path) else {
        return false;
    };
    let mut apps = 0;
    for entry in entries {
        let Ok(entry) = entry else {
            return false;
        };
        match entry.file_name().to_str() {
            Some("rexafs.app" | "rexafs Nightly.app") => {
                if !real_directory(&entry.path())
                    || !entry.path().join("Contents/MacOS/rexafs").is_file()
                {
                    return false;
                }
                apps += 1;
            }
            Some(
                "LICENSE-APACHE"
                | "LICENSE-MIT"
                | "dependencies.json"
                | ".DS_Store"
                | "licenses"
                | "README.txt"
                | "build.json"
                | "linked-libraries.txt"
                | "THIRD_PARTY_NOTICES.md",
            ) => {}
            // An extracted distribution used to store a project is kept intact.
            _ => return false,
        }
    }
    apps == 1
}

fn candidate(path: PathBuf, target: Option<PathBuf>) -> Result<Candidate, String> {
    let metadata = fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
    if metadata.file_type().is_symlink() {
        return Err(format!("Skipped symbolic link: {}", path.display()));
    }
    let mut item = Candidate {
        path,
        bytes: 0,
        modified: metadata.modified().map_err(|e| e.to_string())?,
        files: 0,
        target,
    };
    for entry in walkdir::WalkDir::new(&item.path).follow_links(false) {
        let entry = entry.map_err(|e| e.to_string())?;
        let meta = fs::symlink_metadata(entry.path()).map_err(|e| e.to_string())?;
        item.modified = item
            .modified
            .max(meta.modified().map_err(|e| e.to_string())?);
        if !meta.is_dir() {
            item.bytes = item.bytes.saturating_add(meta.len());
            item.files += 1;
        }
    }
    Ok(item)
}

fn add(report: &mut Report, path: PathBuf, target: Option<PathBuf>) {
    match candidate(path, target) {
        Ok(item) => report.candidates.push(item),
        Err(error) => report.warnings.push(error),
    }
}

fn scan_downloads(root: &Path, report: &mut Report) {
    let downloads = root.join("updates");
    if !real_directory(root) || !real_directory(&downloads) {
        return;
    }
    report.folders.push(downloads.clone());
    let Ok(tags) = fs::read_dir(&downloads) else {
        report
            .warnings
            .push("Cannot read installer downloads.".into());
        return;
    };
    for tag in tags.flatten() {
        if !release_tag(&tag.file_name().to_string_lossy()) || !real_directory(&tag.path()) {
            continue;
        }
        let Ok(entries) = fs::read_dir(tag.path()) else {
            continue;
        };
        for entry in entries.flatten() {
            if archive(&entry.file_name().to_string_lossy())
                && entry.file_type().is_ok_and(|t| t.is_file())
            {
                add(report, entry.path(), None);
            }
        }
    }
}

#[cfg(target_os = "macos")]
fn transaction_target(root: &Path) -> Option<PathBuf> {
    if !real_directory(root)
        || !root
            .file_name()?
            .to_string_lossy()
            .starts_with(".rexafs-update-")
    {
        return None;
    }
    let plan_path = root.join("plan.json");
    let meta = fs::symlink_metadata(&plan_path).ok()?;
    if !meta.is_file() || meta.len() > 16 * 1024 {
        return None;
    }
    let plan: serde_json::Value = serde_json::from_slice(&fs::read(plan_path).ok()?).ok()?;
    let target = PathBuf::from(plan["target"].as_str()?);
    if target.parent() != root.parent()
        || !matches!(
            target.file_name()?.to_str()?,
            "rexafs.app" | "rexafs Nightly.app"
        )
        || !real_directory(&target)
        || !target.join("Contents/MacOS/rexafs").is_file()
        || Path::new(plan["staged"].as_str()?) != root.join("incoming.app")
        || Path::new(plan["previous"].as_str()?) != root.join("previous.app")
    {
        return None;
    }
    Some(target)
}

#[cfg(target_os = "macos")]
fn scan_transactions(parent: &Path, commands: &str, report: &mut Report) {
    if !real_directory(parent) {
        return;
    }
    let Ok(entries) = fs::read_dir(parent) else {
        return;
    };
    for entry in entries.flatten() {
        let root = entry.path();
        let Some(target) = transaction_target(&root) else {
            continue;
        };
        // A manually launched old app/helper is also in use, even without a lock.
        if commands.contains(root.to_string_lossy().as_ref()) {
            continue;
        }
        let Ok(_lock) = crate::updates::install::cleanup_lock(&target) else {
            continue;
        };
        report.folders.push(root.clone());
        for name in [
            "previous.app",
            "incoming.app",
            "helper.app",
            "installer",
            "extracted",
        ] {
            let path = root.join(name);
            if path.exists() {
                add(report, path, Some(target.clone()));
            }
        }
    }
}

#[cfg(target_os = "macos")]
fn process_commands() -> Result<String, String> {
    let output = std::process::Command::new("/bin/ps")
        .args(["-ww", "-axo", "comm="])
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err("Cannot check whether old app copies are still running.".into());
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

pub(crate) fn scan() -> Result<Report, String> {
    let root = crate::settings::app_dir().ok_or("Application storage folder is unavailable.")?;
    let mut report = Report {
        folders: vec![root.clone()],
        ..Default::default()
    };
    scan_downloads(&root, &mut report);
    #[cfg(target_os = "macos")]
    {
        let commands = process_commands()?;
        let downloads = root.join("updates");
        if real_directory(&downloads)
            && let Ok(tags) = fs::read_dir(&downloads)
        {
            for tag in tags.flatten() {
                if !release_tag(&tag.file_name().to_string_lossy()) || !real_directory(&tag.path())
                {
                    continue;
                }
                if let Ok(entries) = fs::read_dir(tag.path()) {
                    for entry in entries.flatten() {
                        if extracted_download(&entry.path())
                            && !commands.contains(entry.path().to_string_lossy().as_ref())
                        {
                            add(&mut report, entry.path(), None);
                        }
                    }
                }
            }
        }
        let mut parents = std::collections::BTreeSet::from([PathBuf::from("/Applications")]);
        if let Some(home) = crate::settings::home_dir() {
            parents.insert(home.join("Applications"));
        }
        if let Ok(app) = crate::updates::install::installed_app()
            && let Some(parent) = app.parent()
        {
            parents.insert(parent.to_owned());
        }
        for parent in parents {
            scan_transactions(&parent, &commands, &mut report);
        }
    }
    report.candidates.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(report)
}

/// Remove only unchanged items from the displayed report, after fresh discovery.
/// Files created since the user reviewed the report cannot be swept into cleanup.
pub(crate) fn clean(reviewed: &Report) -> Result<(u64, Vec<String>), String> {
    let root = crate::settings::app_dir().ok_or("Application storage folder is unavailable.")?;
    let _download_lock = download_lock(&root)?;
    let current = scan()?;
    let mut freed = 0;
    let mut errors = current.warnings;
    for item in reviewed
        .candidates
        .iter()
        .filter(|old| current.candidates.contains(old))
    {
        match remove_checked(item, &root) {
            Ok(bytes) => freed += bytes,
            Err(error) => errors.push(format!("{}: {error}", item.path.display())),
        }
    }
    Ok((freed, errors))
}

fn remove_checked(item: &Candidate, app_root: &Path) -> Result<u64, String> {
    if item.target.is_none() {
        let tag = item.path.parent().ok_or("Missing installer folder")?;
        let allowed = item
            .path
            .file_name()
            .is_some_and(|n| archive(&n.to_string_lossy()))
            && item.path.is_file();
        #[cfg(target_os = "macos")]
        let allowed = allowed
            || (extracted_download(&item.path)
                && !process_commands()?.contains(item.path.to_string_lossy().as_ref()));
        if tag.parent() != Some(app_root.join("updates").as_path())
            || !real_directory(app_root)
            || !real_directory(&app_root.join("updates"))
            || !real_directory(tag)
            || !tag
                .file_name()
                .is_some_and(|n| release_tag(&n.to_string_lossy()))
            || !allowed
        {
            return Err("Installer location changed; refresh Storage before cleanup.".into());
        }
    }
    #[cfg(target_os = "macos")]
    let _install_lock = if let Some(target) = &item.target {
        let parent = item.path.parent().ok_or("Missing updater folder")?;
        if transaction_target(parent).as_ref() != Some(target)
            || !matches!(
                item.path.file_name().and_then(|n| n.to_str()),
                Some("previous.app" | "incoming.app" | "helper.app" | "installer" | "extracted")
            )
            || process_commands()?.contains(parent.to_string_lossy().as_ref())
        {
            return Err("An updater copy is in use or changed; it was kept.".into());
        }
        Some(crate::updates::install::cleanup_lock(target)?)
    } else {
        None
    };
    #[cfg(not(target_os = "macos"))]
    if item.target.is_some() {
        return Err("App-copy cleanup is available on macOS only.".into());
    }
    if candidate(item.path.clone(), item.target.clone()).as_ref() != Ok(item) {
        return Ok(0);
    }
    if item.path.is_dir() {
        fs::remove_dir_all(&item.path)
    } else {
        fs::remove_file(&item.path)
    }
    .map_err(|e| e.to_string())?;
    Ok(item.bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_os = "macos")]
    #[test]
    fn extracted_installers_with_user_projects_are_kept() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("rexafs-0.1.2-aarch64-apple-darwin");
        fs::create_dir_all(path.join("rexafs.app/Contents/MacOS")).unwrap();
        fs::write(path.join("rexafs.app/Contents/MacOS/rexafs"), b"old app").unwrap();
        assert!(extracted_download(&path));
        fs::write(path.join("My analysis.rxs"), b"user project").unwrap();
        assert!(!extracted_download(&path));
    }

    #[test]
    fn only_release_archives_are_disposable() {
        let temp = tempfile::tempdir().unwrap();
        for name in [
            "updates/v0.2.11/rexafs-macos.zip",
            "updates/nightly-20260919-42/rexafs-linux.tar.gz",
            "updates/v0.2.11/Update recovery.rxs",
            "updates/v0.2.11/.download-123.part",
            "updates/work/rexafs-macos.zip",
            "project-data/raw.xdi",
            "rmc/checkpoint.json",
        ] {
            let path = temp.path().join(name);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, b"keep or cache").unwrap();
        }
        let mut report = Report::default();
        scan_downloads(temp.path(), &mut report);
        assert_eq!(report.candidates.len(), 2);
        assert_eq!(report.bytes(), 26);
        let lock = download_lock(temp.path()).unwrap();
        assert!(download_lock(temp.path()).is_err());
        drop(lock);
        assert!(download_lock(temp.path()).is_ok());
        let candidate = report.candidates[0].clone();
        assert_eq!(remove_checked(&candidate, temp.path()).unwrap(), 13);
        assert!(!candidate.path.exists());
        assert_eq!(
            fs::read(temp.path().join("updates/v0.2.11/Update recovery.rxs")).unwrap(),
            b"keep or cache"
        );
        assert!(temp.path().join("rmc/checkpoint.json").exists());
    }

    #[cfg(unix)]
    #[test]
    fn redirected_release_folders_and_files_are_not_candidates() {
        use std::os::unix::fs::symlink;
        let temp = tempfile::tempdir().unwrap();
        let other = tempfile::tempdir().unwrap();
        fs::create_dir_all(temp.path().join("updates/v0.2.11")).unwrap();
        fs::write(other.path().join("rexafs-macos.zip"), b"external").unwrap();
        symlink(other.path(), temp.path().join("updates/v0.2.10")).unwrap();
        symlink(
            other.path().join("rexafs-macos.zip"),
            temp.path().join("updates/v0.2.11/rexafs-macos.zip"),
        )
        .unwrap();
        let mut report = Report::default();
        scan_downloads(temp.path(), &mut report);
        assert!(report.candidates.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn redirect_after_review_cannot_delete_an_external_archive() {
        use std::os::unix::fs::symlink;
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let tag = root.path().join("updates/v0.2.11");
        fs::create_dir_all(&tag).unwrap();
        fs::write(tag.join("rexafs-macos.zip"), b"installer").unwrap();
        let mut report = Report::default();
        scan_downloads(root.path(), &mut report);
        fs::rename(&tag, outside.path().join("moved")).unwrap();
        symlink(outside.path().join("moved"), &tag).unwrap();
        assert!(remove_checked(&report.candidates[0], root.path()).is_err());
        assert_eq!(
            fs::read(outside.path().join("moved/rexafs-macos.zip")).unwrap(),
            b"installer"
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn update_copies_preserve_recovery_and_running_or_locked_transactions() {
        let temp = tempfile::tempdir().unwrap();
        let target = temp.path().join("rexafs.app");
        fs::create_dir_all(target.join("Contents/MacOS")).unwrap();
        fs::write(target.join("Contents/MacOS/rexafs"), b"current app").unwrap();
        let root = temp.path().join(".rexafs-update-test");
        fs::create_dir(&root).unwrap();
        fs::write(root.join("plan.json"), serde_json::to_vec(&serde_json::json!({"target":target,"staged":root.join("incoming.app"),"previous":root.join("previous.app")})).unwrap()).unwrap();
        fs::write(root.join("installer"), b"old helper").unwrap();
        fs::write(root.join("Update recovery.rxs"), b"precious").unwrap();
        let mut report = Report::default();
        scan_transactions(temp.path(), "", &mut report);
        assert_eq!(report.candidates.len(), 1);
        assert_eq!(report.candidates[0].path, root.join("installer"));
        let mut busy = Report::default();
        scan_transactions(temp.path(), root.to_str().unwrap(), &mut busy);
        assert!(busy.candidates.is_empty());
        let _lock = crate::updates::install::cleanup_lock(&target).unwrap();
        scan_transactions(temp.path(), "", &mut busy);
        assert!(busy.candidates.is_empty());
    }
}
