//! Bounded file hashing and child-process checks used before installation.
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::Read,
    path::Path,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

pub(super) fn hash_file(path: &Path) -> Result<String, String> {
    let mut file = File::open(path).map_err(|e| e.to_string())?;
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
    }
    Ok(crate::updates::hex(&hash.finalize()))
}

/// Bound external tools without a pipe deadlock; never execute a shell string.
pub(super) fn run(command: &mut Command) -> Result<Vec<u8>, String> {
    let mut output = tempfile::tempfile().map_err(|e| e.to_string())?;
    let mut errors = tempfile::tempfile().map_err(|e| e.to_string())?;
    let mut child = command
        .stdin(Stdio::null())
        .stdout(output.try_clone().map_err(|e| e.to_string())?)
        .stderr(errors.try_clone().map_err(|e| e.to_string())?)
        .spawn()
        .map_err(|e| e.to_string())?;
    let started = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            break status;
        }
        if started.elapsed() > Duration::from_secs(60) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!(
                "{} timed out",
                command.get_program().to_string_lossy()
            ));
        }
        thread::sleep(Duration::from_millis(50));
    };
    use std::io::{Seek, SeekFrom};
    errors.seek(SeekFrom::Start(0)).map_err(|e| e.to_string())?;
    if !status.success() {
        let mut text = String::new();
        let _ = errors.take(4096).read_to_string(&mut text);
        return Err(format!(
            "{} failed: {}",
            command.get_program().to_string_lossy(),
            text.trim()
        ));
    }
    output.seek(SeekFrom::Start(0)).map_err(|e| e.to_string())?;
    let mut bytes = Vec::new();
    output
        .take(4 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 4 * 1024 * 1024 {
        return Err("Update tool output exceeded its limit".into());
    }
    Ok(bytes)
}
