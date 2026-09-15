//! In-place desktop updates. Preparation never changes the installed application.
//! A copied helper waits for the GUI process to exit, replaces the bundle on
//! the same filesystem, and rolls back if replacement or launch fails.

use std::path::{Path, PathBuf};
mod common;
#[cfg(any(target_os = "windows", target_os = "linux", test))]
mod payload;
#[cfg(any(target_os = "windows", target_os = "linux"))]
mod portable;
#[cfg(target_os = "windows")]
mod windows;

/// Kept by the GUI until it exits. Windows file locks cannot be inherited by
/// the helper; it acquires its own lock after the parent process has stopped.
pub(crate) struct UpdateHandoff {
    _directory: PathBuf,
    _lock: Option<std::fs::File>,
}

/// Source builds and other platforms retain the manual download workflow.
#[cfg(target_os = "macos")]
pub(crate) fn installed_app() -> Result<PathBuf, String> {
    app_for_executable(&std::env::current_exe().map_err(|e| e.to_string())?)
}

#[cfg(any(target_os = "macos", test))]
fn app_for_executable(executable: &Path) -> Result<PathBuf, String> {
    let app = executable
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .filter(|app| app.extension().is_some_and(|ext| ext == "app"))
        .filter(|app| app.join("Contents/MacOS/rexafs") == executable)
        .ok_or("Install rexafs in Applications to enable Update and restart.")?;
    if app.starts_with("/Volumes")
        || app
            .components()
            .any(|c| c.as_os_str() == "AppTranslocation")
    {
        return Err("Move rexafs to Applications and reopen it before updating.".into());
    }
    Ok(app.to_path_buf())
}

fn replace_and_launch(
    target: &Path,
    staged: &Path,
    backup: &Path,
    mut launch: impl FnMut(&Path) -> Result<(), String>,
) -> Result<(), String> {
    if backup.exists() {
        return Err("The update backup already exists; it was preserved.".into());
    }
    if let Err(error) = std::fs::rename(target, backup) {
        let _ = launch(target);
        return Err(format!("Could not preserve the previous app: {error}"));
    }
    if let Err(error) = std::fs::rename(staged, target) {
        std::fs::rename(backup, target).map_err(|rollback| {
            format!(
                "Install failed: {error}. Restore {} to {}: {rollback}",
                backup.display(),
                target.display()
            )
        })?;
        let _ = launch(target);
        return Err(format!(
            "Install failed; the previous app was restored: {error}"
        ));
    }
    if let Err(error) = launch(target) {
        // Preserve the failed new bundle too; never recursively delete an app
        // as part of rollback. Both destinations are private transaction paths.
        std::fs::rename(target, staged)
            .and_then(|_| std::fs::rename(backup, target))
            .map_err(|rollback| {
                format!(
                    "Launch failed: {error}. Previous app: {}. Rollback failed: {rollback}",
                    backup.display()
                )
            })?;
        let _ = launch(target);
        return Err(format!(
            "Launch failed; the previous app was restored: {error}"
        ));
    }
    Ok(())
}

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub(crate) use macos::{finish_update, prepare};
#[cfg(any(target_os = "windows", target_os = "linux"))]
pub(crate) use portable::{finish_update, installed_app, prepare};

pub(crate) fn can_install(release: &crate::updates::AvailableRelease) -> bool {
    #[cfg(target_os = "windows")]
    if release.installer.is_none()
        && installed_app().is_ok_and(|target| windows::registered_install(&target).unwrap_or(true))
    {
        return false;
    }
    release.asset.is_some() && installed_app().is_ok()
}

pub(crate) fn download_size(release: &crate::updates::AvailableRelease) -> u64 {
    #[cfg(windows)]
    let needs_setup =
        installed_app().is_ok_and(|target| windows::registered_install(&target).unwrap_or(false));
    #[cfg(not(windows))]
    let needs_setup = false;
    release.asset.as_ref().map_or(0, |asset| asset.size)
        + if needs_setup {
            release.installer.as_ref().map_or(0, |asset| asset.size)
        } else {
            0
        }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_installed_bundle_executables_can_be_replaced() {
        assert_eq!(
            app_for_executable(Path::new("/Applications/rexafs.app/Contents/MacOS/rexafs"))
                .unwrap(),
            Path::new("/Applications/rexafs.app")
        );
        for path in [
            "/tmp/target/release/rexafs",
            "/Volumes/rexafs/rexafs.app/Contents/MacOS/rexafs",
            "/private/var/AppTranslocation/id/rexafs.app/Contents/MacOS/rexafs",
            "/Applications/rexafs.app/other/rexafs",
        ] {
            assert!(app_for_executable(Path::new(path)).is_err(), "{path}");
        }
    }

    #[test]
    fn replacement_keeps_backup_and_rolls_back_failed_install_or_launch() {
        let root =
            std::env::temp_dir().join(format!("rexafs-replacement-test-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let target = root.join("application with spaces.app");
        let staged = root.join("new.app");
        let backup = root.join("previous.app");
        std::fs::write(&target, "old").unwrap();
        assert!(replace_and_launch(&target, &staged, &backup, |_| Ok(())).is_err());
        assert_eq!(std::fs::read(&target).unwrap(), b"old");
        std::fs::write(&staged, "new").unwrap();
        let mut launches = 0;
        assert!(
            replace_and_launch(&target, &staged, &backup, |_| {
                launches += 1;
                Err("launch rejected".into())
            })
            .is_err()
        );
        assert_eq!(launches, 2);
        assert_eq!(std::fs::read(&target).unwrap(), b"old");
        assert_eq!(std::fs::read(&staged).unwrap(), b"new");
        replace_and_launch(&target, &staged, &backup, |_| Ok(())).unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), b"new");
        assert_eq!(std::fs::read(&backup).unwrap(), b"old");
        assert!(replace_and_launch(&target, &staged, &backup, |_| Ok(())).is_err());
        assert_eq!(std::fs::read(&target).unwrap(), b"new");
        std::fs::remove_dir_all(root).unwrap();
    }
}
