//! Preserve the per-user Inno Setup registration during Windows upgrades.
use super::{
    common::{hash_file, run},
    portable::launch,
};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
use windows::{
    Win32::{
        Foundation::{CloseHandle, ERROR_FILE_NOT_FOUND, HANDLE, WAIT_OBJECT_0},
        System::{
            Registry::{HKEY_CURRENT_USER, RRF_RT_REG_SZ, RRF_SUBKEY_WOW6464KEY, RegGetValueW},
            SystemInformation::GetSystemDirectoryW,
            Threading::{OpenProcess, PROCESS_SYNCHRONIZE, WaitForSingleObject},
        },
    },
    core::{PCWSTR, w},
};

fn subkey() -> String {
    let id = if crate::updates::installed_channel() == crate::updates::UpdateChannel::Nightly {
        "rexafs.desktop.nightly"
    } else {
        "rexafs.desktop"
    };
    format!("Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\{id}_is1")
}

pub(super) fn registered_install(target: &Path) -> Result<bool, String> {
    let key: Vec<_> = subkey().encode_utf16().chain(Some(0)).collect();
    let mut buffer = vec![0u16; 32768];
    let mut bytes = (buffer.len() * 2) as u32;
    // SAFETY: key is NUL terminated; buffer is writable for exactly `bytes`.
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            PCWSTR(key.as_ptr()),
            w!("InstallLocation"),
            RRF_RT_REG_SZ | RRF_SUBKEY_WOW6464KEY,
            None,
            Some(buffer.as_mut_ptr().cast()),
            Some(&mut bytes),
        )
    };
    if status == ERROR_FILE_NOT_FOUND {
        return Ok(false);
    }
    status
        .ok()
        .map_err(|e| format!("Cannot read rexafs installer registration: {e}"))?;
    buffer.truncate((bytes as usize) / 2);
    while buffer.last() == Some(&0) {
        buffer.pop();
    }
    use std::os::windows::ffi::OsStringExt;
    let location = PathBuf::from(std::ffi::OsString::from_wide(&buffer));
    // A separate portable copy must not redirect an existing installation.
    Ok(location.canonicalize().is_ok_and(|path| path == target))
}

fn registry_tool() -> Result<PathBuf, String> {
    let mut buffer = vec![0u16; 32768];
    // SAFETY: the API receives the exact length of the writable slice.
    let length = unsafe { GetSystemDirectoryW(Some(&mut buffer)) } as usize;
    if length == 0 || length >= buffer.len() {
        return Err("Windows system directory unavailable".into());
    }
    use std::os::windows::ffi::OsStringExt;
    Ok(PathBuf::from(std::ffi::OsString::from_wide(&buffer[..length])).join("reg.exe"))
}

pub(super) fn backup_registration(root: &Path) -> Result<(), String> {
    run(Command::new(registry_tool()?)
        .arg("export")
        .arg(format!("HKCU\\{}", subkey()))
        .arg(root.join("registration.reg"))
        .arg("/reg:64"))?;
    Ok(())
}

pub(super) fn install(
    root: &Path,
    target: &Path,
    verify_and_launch: impl FnOnce() -> Result<(), String>,
) -> Result<(), String> {
    let registration = root.join("registration.reg");
    let registration_hash = hash_file(&registration)?;
    let path = target
        .to_str()
        .ok_or("Installer requires a Unicode app path")?;
    // Inno Setup expects a DOS/UNC path rather than Rust's canonical extended
    // path prefix. Arguments remain separate OS strings; no shell is invoked.
    let path = if let Some(unc) = path.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{unc}")
    } else {
        path.strip_prefix(r"\\?\").unwrap_or(path).to_owned()
    };
    // Never kill Setup on a timeout: its child may still be writing files.
    // Rollback starts only after Setup has actually exited.
    let result = Command::new(root.join("setup.exe"))
        .args([
            "/VERYSILENT",
            "/SUPPRESSMSGBOXES",
            "/NORESTART",
            "/RESTARTEXITCODE=9",
            "/NOCLOSEAPPLICATIONS",
            "/NOFORCECLOSEAPPLICATIONS",
            "/NORESTARTAPPLICATIONS",
            "/SP-",
        ])
        .arg(format!("/DIR={path}"))
        .arg(format!("/LOG={}", root.join("setup.log").display()))
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map_err(|e| e.to_string())
        .and_then(|status| {
            if status.success() {
                Ok(())
            } else {
                Err(format!("Windows Setup failed ({status}); see setup.log"))
            }
        })
        .and_then(|_| verify_and_launch());
    if let Err(error) = result {
        let previous = root.join("previous");
        let failed = root.join("failed");
        if target.exists() {
            fs::rename(target, &failed).map_err(|e| {
                format!(
                    "Update failed: {error}. Previous app retained at {}: {e}",
                    previous.display()
                )
            })?;
        }
        fs::rename(&previous, target).map_err(|e| {
            format!(
                "Update failed: {error}. Restore {}: {e}",
                previous.display()
            )
        })?;
        if hash_file(&registration)? != registration_hash {
            return Err("Installer registration backup changed; it was not imported.".into());
        }
        run(Command::new(registry_tool()?)
            .arg("import")
            .arg(registration)
            .arg("/reg:64"))?;
        let _ = launch(target, &root.join("Update recovery.rxs"));
        return Err(format!(
            "Update failed; the previous app and installer registration were restored: {error}"
        ));
    }
    Ok(())
}

pub(super) struct ParentProcess(HANDLE);
impl ParentProcess {
    pub(super) fn open(pid: u32) -> Result<Self, String> {
        // SAFETY: only synchronization access is requested, never termination.
        unsafe { OpenProcess(PROCESS_SYNCHRONIZE, false, pid) }
            .map(Self)
            .map_err(|e| e.to_string())
    }
    pub(super) fn wait(&self) -> Result<(), String> {
        // SAFETY: this owned handle remains open throughout the bounded wait.
        if unsafe { WaitForSingleObject(self.0, 60_000) } == WAIT_OBJECT_0 {
            Ok(())
        } else {
            Err("rexafs did not quit; the installed app is unchanged.".into())
        }
    }
}
impl Drop for ParentProcess {
    fn drop(&mut self) {
        // SAFETY: the handle is owned and closed exactly once.
        let _ = unsafe { CloseHandle(self.0) };
    }
}
