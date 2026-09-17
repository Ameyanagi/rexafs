//! Immutable, bounded JSON artifacts shared by scientific analysis histories.
use serde::{Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};
use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
};

pub fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

pub fn retain<T: Serialize>(root: &Path, record: &T) -> Result<(PathBuf, String), String> {
    let mut dirs = std::fs::DirBuilder::new();
    dirs.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        dirs.mode(0o700);
    }
    dirs.create(root).map_err(|e| e.to_string())?;
    if std::fs::symlink_metadata(root)
        .map_err(|e| e.to_string())?
        .file_type()
        .is_symlink()
    {
        return Err("Analysis storage cannot be a symlink".into());
    }
    let compressed = LimitedWriter {
        inner: Vec::new(),
        remaining: 128 * 1024 * 1024,
    };
    let gzip = flate2::write::GzEncoder::new(compressed, flate2::Compression::default());
    let mut expanded = LimitedWriter {
        inner: gzip,
        remaining: 256 * 1024 * 1024,
    };
    serde_json::to_writer(&mut expanded, record).map_err(|e| e.to_string())?;
    let bytes = expanded.inner.finish().map_err(|e| e.to_string())?.inner;
    let hash = digest(&bytes);
    let path = root.join(format!("{hash}.json.gz"));
    let mut temporary = tempfile::NamedTempFile::new_in(root).map_err(|e| e.to_string())?;
    temporary.write_all(&bytes).map_err(|e| e.to_string())?;
    temporary.as_file().sync_all().map_err(|e| e.to_string())?;
    match temporary.persist_noclobber(&path) {
        Ok(_) => {}
        Err(error) if error.error.kind() == std::io::ErrorKind::AlreadyExists => {
            if digest(&std::fs::read(&path).map_err(|e| e.to_string())?) != hash {
                return Err("Historical analysis artifact changed".into());
            }
        }
        Err(error) => return Err(error.to_string()),
    }
    #[cfg(unix)]
    std::fs::File::open(root)
        .and_then(|dir| dir.sync_all())
        .map_err(|e| e.to_string())?;
    Ok((path, hash))
}

/// The same compressed/expanded limits apply while writing and reading artifacts.
struct LimitedWriter<W> {
    inner: W,
    remaining: usize,
}
impl<W: Write> Write for LimitedWriter<W> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.len() > self.remaining {
            return Err(std::io::Error::other(
                "Analysis artifact exceeds its storage limit",
            ));
        }
        let written = self.inner.write(bytes)?;
        self.remaining -= written;
        Ok(written)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

pub fn read<T: DeserializeOwned>(path: &Path, expected_digest: &str) -> Result<T, String> {
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut bytes = Vec::new();
    file.take(128 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 128 * 1024 * 1024 {
        return Err("Analysis artifact exceeds 128 MiB".into());
    }
    if expected_digest != digest(&bytes) {
        return Err("Retained analysis checksum mismatch".into());
    }
    let mut json = Vec::new();
    flate2::read::GzDecoder::new(&bytes[..])
        .take(256 * 1024 * 1024 + 1)
        .read_to_end(&mut json)
        .map_err(|e| e.to_string())?;
    if json.len() > 256 * 1024 * 1024 {
        return Err("Expanded analysis artifact exceeds 256 MiB".into());
    }
    serde_json::from_slice(&json).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn artifact_writer_enforces_limit() {
        let mut writer = LimitedWriter {
            inner: Vec::new(),
            remaining: 3,
        };
        writer.write_all(b"abc").unwrap();
        assert!(writer.write_all(b"d").is_err());
        assert_eq!(writer.inner, b"abc");
    }
}
