//! Extract regular package files into a private directory and verify ownership.
use super::common::hash_file;
use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
};

const MAX_FILES: usize = 20_000;
const MAX_BYTES: u64 = 2 * 1024 * 1024 * 1024;
pub(super) const MANIFEST: &str = "update-manifest.json";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    version: u32,
    files: Vec<OwnedFile>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OwnedFile {
    path: String,
    sha256: String,
}

/// Package paths must be portable between the supported desktop filesystems.
fn safe_path(name: &str) -> Result<PathBuf, String> {
    if name.is_empty() || name.len() > 1024 || name.contains(['\\', ':', '\0']) {
        return Err(format!("Unsafe update path: {name:?}"));
    }
    for part in name.split('/') {
        let base = part.split('.').next().unwrap_or("").to_ascii_uppercase();
        if part.is_empty()
            || part == "."
            || part == ".."
            || part.ends_with(['.', ' '])
            || part
                .chars()
                .any(|c| c.is_control() || "<>\"|?*".contains(c))
            || matches!(base.as_str(), "CON" | "PRN" | "AUX" | "NUL")
            || (base.len() == 4
                && (base.starts_with("COM") || base.starts_with("LPT"))
                && matches!(base.as_bytes()[3], b'1'..=b'9'))
        {
            return Err(format!("Unsafe update path: {name:?}"));
        }
    }
    Ok(PathBuf::from(name))
}

/// Reject links, including Windows directory junctions, rather than following
/// them into user data outside the installation or the update transaction.
pub(super) fn regular_metadata(path: &Path) -> Result<fs::Metadata, String> {
    let metadata = fs::symlink_metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 != 0 {
            return Err(format!(
                "Update does not follow reparse points: {}",
                path.display()
            ));
        }
    }
    if !(metadata.is_file() || metadata.is_dir()) || metadata.file_type().is_symlink() {
        return Err(format!(
            "Update does not follow links or special files: {}",
            path.display()
        ));
    }
    Ok(metadata)
}

pub(super) fn files(root: &Path) -> Result<BTreeMap<String, PathBuf>, String> {
    regular_metadata(root)?;
    let mut result = BTreeMap::new();
    let mut size = 0u64;
    let mut names = BTreeSet::new();
    for entry in walkdir::WalkDir::new(root).min_depth(1).follow_links(false) {
        let entry = entry.map_err(|e| e.to_string())?;
        let metadata = regular_metadata(entry.path())?;
        let name = entry
            .path()
            .strip_prefix(root)
            .map_err(|e| e.to_string())?
            .to_str()
            .ok_or("Update requires Unicode file names")?
            .replace(std::path::MAIN_SEPARATOR, "/");
        safe_path(&name)?;
        if !names.insert(name.to_lowercase()) || names.len() > MAX_FILES {
            return Err("The app folder contains duplicate paths or too many files.".into());
        }
        if metadata.is_file() {
            size = size
                .checked_add(metadata.len())
                .ok_or("App folder is too large")?;
            if size > MAX_BYTES {
                return Err("The app folder exceeds the 2 GiB update limit.".into());
            }
            result.insert(name, entry.into_path());
        }
    }
    Ok(result)
}

pub(super) fn manifest(root: &Path) -> Result<BTreeMap<String, String>, String> {
    let path = root.join(MANIFEST);
    if regular_metadata(&path)?.len() > 4 * 1024 * 1024 {
        return Err("Update manifest is too large".into());
    }
    let manifest: Manifest = serde_json::from_slice(&fs::read(path).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    if manifest.version != 1 || manifest.files.is_empty() || manifest.files.len() > MAX_FILES {
        return Err("Unsupported update manifest".into());
    }
    let mut result = BTreeMap::new();
    let mut names = BTreeSet::new();
    for file in manifest.files {
        safe_path(&file.path)?;
        if file.path == MANIFEST
            || !names.insert(file.path.to_lowercase())
            || file.sha256.len() != 64
            || !file.sha256.bytes().all(|b| b.is_ascii_hexdigit())
        {
            return Err("Invalid or duplicate update manifest entry".into());
        }
        result.insert(file.path, file.sha256);
    }
    Ok(result)
}

pub(super) fn verify(root: &Path, allow_extra: bool) -> Result<BTreeMap<String, String>, String> {
    let actual = files(root)?;
    let manifest = manifest(root)?;
    for (name, hash) in &manifest {
        let path = actual
            .get(name)
            .ok_or_else(|| format!("Missing package file: {name}"))?;
        if hash_file(path)? != *hash {
            return Err(format!("Package file changed: {name}"));
        }
    }
    if !allow_extra
        && actual
            .keys()
            .any(|name| name != MANIFEST && !manifest.contains_key(name))
    {
        return Err("Unlisted files in the downloaded package".into());
    }
    Ok(manifest)
}

#[cfg(any(windows, test))]
pub(super) fn copy_tree(source: &Path, destination: &Path) -> Result<(), String> {
    fs::create_dir(destination).map_err(|e| e.to_string())?;
    for entry in walkdir::WalkDir::new(source)
        .min_depth(1)
        .follow_links(false)
    {
        let entry = entry.map_err(|e| e.to_string())?;
        let metadata = regular_metadata(entry.path())?;
        let path = destination.join(
            entry
                .path()
                .strip_prefix(source)
                .map_err(|e| e.to_string())?,
        );
        if metadata.is_dir() {
            fs::create_dir(&path).map_err(|e| e.to_string())?;
        } else {
            fs::copy(entry.path(), &path).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

/// User files are retained; a new package cannot claim an existing user path.
pub(super) fn preserve_extras(current: &Path, incoming: &Path, copy: bool) -> Result<(), String> {
    let owned = manifest(current)?;
    let new_files = files(incoming)?;
    let new_names: BTreeSet<_> = new_files.keys().map(|name| name.to_lowercase()).collect();
    for (name, source) in files(current)? {
        if name == MANIFEST || owned.contains_key(&name) {
            continue;
        }
        if new_names.contains(&name.to_lowercase()) || incoming.join(&name).exists() {
            return Err(format!(
                "The update would overwrite your file {name}. Move it out of the app folder first."
            ));
        }
        // Include parent/child collisions (for example user file 'resources').
        let prefix = format!("{}/", name.to_lowercase());
        if new_names.iter().any(|new| {
            new.starts_with(&prefix) || name.to_lowercase().starts_with(&format!("{new}/"))
        }) {
            return Err(format!(
                "The update conflicts with your file {name}. Move it out of the app folder first."
            ));
        }
        if copy {
            let target = incoming.join(&name);
            fs::create_dir_all(target.parent().ok_or("Missing destination parent")?)
                .map_err(|e| e.to_string())?;
            fs::copy(source, target).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

struct Extractor<'a> {
    destination: &'a Path,
    root: &'a str,
    names: BTreeSet<String>,
    bytes: u64,
}
impl Extractor<'_> {
    fn entry(
        &mut self,
        name: &str,
        size: u64,
        directory: bool,
        mode: u32,
        reader: impl Read,
    ) -> Result<(), String> {
        let name = name.strip_suffix('/').unwrap_or(name);
        safe_path(name)?;
        if !self.names.insert(name.to_lowercase()) || self.names.len() > MAX_FILES {
            return Err("Duplicate archive paths or too many entries".into());
        }
        if name == self.root && directory {
            return Ok(());
        }
        let name = name
            .strip_prefix(self.root)
            .and_then(|n| n.strip_prefix('/'))
            .ok_or("Incorrect archive root")?;
        let relative = safe_path(name)?;
        self.bytes = self.bytes.checked_add(size).ok_or("Archive is too large")?;
        if self.bytes > MAX_BYTES {
            return Err("Expanded archive exceeds 2 GiB".into());
        }
        let path = self.destination.join(relative);
        if directory {
            fs::create_dir_all(path).map_err(|e| e.to_string())?;
        } else {
            fs::create_dir_all(path.parent().ok_or("Missing archive parent")?)
                .map_err(|e| e.to_string())?;
            let mut file = File::create_new(&path).map_err(|e| e.to_string())?;
            let written = std::io::copy(&mut reader.take(size.saturating_add(1)), &mut file)
                .map_err(|e| e.to_string())?;
            if written != size {
                return Err("Archive entry has an incorrect length".into());
            }
            file.flush().map_err(|e| e.to_string())?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(
                    &path,
                    fs::Permissions::from_mode(if mode & 0o111 != 0 { 0o755 } else { 0o644 }),
                )
                .map_err(|e| e.to_string())?;
            }
            #[cfg(not(unix))]
            let _ = mode;
        }
        Ok(())
    }
}

pub(super) fn extract(archive: &Path, name: &str, destination: &Path) -> Result<(), String> {
    let root = name
        .strip_suffix(".tar.gz")
        .or_else(|| name.strip_suffix(".zip"))
        .ok_or("Unsupported update archive")?;
    safe_path(root)?;
    fs::create_dir(destination).map_err(|e| e.to_string())?;
    let mut extractor = Extractor {
        destination,
        root,
        names: BTreeSet::new(),
        bytes: 0,
    };
    let file = File::open(archive).map_err(|e| e.to_string())?;
    if name.ends_with(".zip") {
        let mut zip = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
        if zip.len() > MAX_FILES || zip.has_overlapping_files().map_err(|e| e.to_string())? {
            return Err("Unsafe ZIP directory".into());
        }
        for index in 0..zip.len() {
            let mut entry = zip.by_index(index).map_err(|e| e.to_string())?;
            let mode = entry.unix_mode().unwrap_or(0);
            if entry.is_symlink()
                || entry.encrypted()
                || !matches!(mode & 0o170000, 0 | 0o100000 | 0o040000)
            {
                return Err("Unsupported archive link, encryption or special file".into());
            }
            let name = std::str::from_utf8(entry.name_raw())
                .map_err(|e| e.to_string())?
                .to_owned();
            extractor.entry(&name, entry.size(), entry.is_dir(), mode, &mut entry)?;
        }
    } else {
        let mut tar = tar::Archive::new(flate2::read::GzDecoder::new(file));
        for entry in tar.entries().map_err(|e| e.to_string())? {
            let mut entry = entry.map_err(|e| e.to_string())?;
            let kind = entry.header().entry_type();
            if !(kind.is_file() || kind.is_dir()) {
                return Err("Unsupported archive link or special file".into());
            }
            let name = std::str::from_utf8(&entry.path_bytes())
                .map_err(|e| e.to_string())?
                .to_owned();
            extractor.entry(
                &name,
                entry.size(),
                kind.is_dir(),
                entry.header().mode().map_err(|e| e.to_string())?,
                &mut entry,
            )?;
        }
        // Read the gzip trailer too; truncation after the tar end marker fails.
        std::io::copy(
            &mut tar.into_inner().take(1024 * 1024),
            &mut std::io::sink(),
        )
        .map_err(|e| e.to_string())?;
    }
    verify(destination, false)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};

    fn inventory(root: &Path) {
        let entries: Vec<_> = files(root)
            .unwrap()
            .into_iter()
            .filter(|(name, _)| name != MANIFEST)
            .map(|(name, path)| serde_json::json!({"path":name,"sha256":hash_file(&path).unwrap()}))
            .collect();
        fs::write(
            root.join(MANIFEST),
            serde_json::to_vec(&serde_json::json!({"version":1,"files":entries})).unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn archives_round_trip_inventory_and_executable_modes() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("rexafs-package");
        fs::create_dir(&source).unwrap();
        fs::write(source.join("rexafs"), b"packaged executable").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(source.join("rexafs"), fs::Permissions::from_mode(0o755)).unwrap();
        }
        inventory(&source);
        for extension in [".zip", ".tar.gz"] {
            let name = format!("rexafs-package{extension}");
            let archive = directory.path().join(&name);
            if extension == ".zip" {
                let mut writer = zip::ZipWriter::new(File::create(&archive).unwrap());
                for (name, path) in files(&source).unwrap() {
                    writer
                        .start_file(
                            format!("rexafs-package/{name}"),
                            zip::write::SimpleFileOptions::default().unix_permissions(0o755),
                        )
                        .unwrap();
                    writer.write_all(&fs::read(path).unwrap()).unwrap();
                }
                writer.finish().unwrap();
            } else {
                let mut writer = tar::Builder::new(flate2::write::GzEncoder::new(
                    File::create(&archive).unwrap(),
                    flate2::Compression::default(),
                ));
                writer.append_dir_all("rexafs-package", &source).unwrap();
                writer.into_inner().unwrap().finish().unwrap();
            }
            let destination = directory.path().join(format!("extracted{extension}"));
            extract(&archive, &name, &destination).unwrap();
            assert_eq!(
                fs::read(destination.join("rexafs")).unwrap(),
                b"packaged executable"
            );
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                assert_ne!(
                    fs::metadata(destination.join("rexafs"))
                        .unwrap()
                        .permissions()
                        .mode()
                        & 0o111,
                    0
                );
            }
            fs::write(destination.join("rexafs"), "damaged").unwrap();
            assert!(verify(&destination, false).unwrap_err().contains("changed"));
        }
    }

    #[test]
    fn archive_paths_links_and_collisions_cannot_escape_the_transaction() {
        let directory = tempfile::tempdir().unwrap();
        for (index, names) in [
            vec!["../outside"],
            vec!["/outside"],
            vec!["pkg/../outside"],
            vec!["other/file"],
            vec!["pkg/C:\\outside"],
            vec!["pkg/NUL.txt"],
            vec!["pkg/trailing."],
            vec!["pkg/File", "pkg/file"],
            vec!["pkg/dir", "pkg/dir/file"],
        ]
        .iter()
        .enumerate()
        {
            let archive = directory.path().join(format!("bad-{index}.zip"));
            let mut writer = zip::ZipWriter::new(File::create(&archive).unwrap());
            for name in names {
                writer
                    .start_file(*name, zip::write::SimpleFileOptions::default())
                    .unwrap();
                writer.write_all(b"data").unwrap();
            }
            writer.finish().unwrap();
            assert!(
                extract(
                    &archive,
                    "pkg.zip",
                    &directory.path().join(format!("out-{index}"))
                )
                .is_err(),
                "{names:?}"
            );
        }
        let archive = directory.path().join("links.tar.gz");
        let mut writer = tar::Builder::new(flate2::write::GzEncoder::new(
            File::create(&archive).unwrap(),
            flate2::Compression::default(),
        ));
        let mut header = tar::Header::new_gnu();
        header.set_entry_type(tar::EntryType::Symlink);
        header.set_size(0);
        header.set_mode(0o777);
        writer
            .append_link(&mut header, "pkg/link", "../../outside")
            .unwrap();
        writer.into_inner().unwrap().finish().unwrap();
        assert!(
            extract(&archive, "pkg.tar.gz", &directory.path().join("links"))
                .unwrap_err()
                .contains("link")
        );
        assert!(!directory.path().join("outside").exists());
    }

    #[test]
    fn extras_survive_and_new_package_cannot_claim_user_data() {
        let directory = tempfile::tempdir().unwrap();
        let current = directory.path().join("current");
        let incoming = directory.path().join("incoming");
        for root in [&current, &incoming] {
            fs::create_dir(root).unwrap();
            fs::write(root.join("rexafs"), b"binary").unwrap();
            inventory(root);
        }
        fs::create_dir(current.join("my analyses")).unwrap();
        fs::write(current.join("my analyses/日本語.rxs"), "original project").unwrap();
        verify(&current, true).unwrap();
        assert!(verify(&current, false).is_err());
        preserve_extras(&current, &incoming, false).unwrap();
        assert!(!incoming.join("my analyses").exists());
        preserve_extras(&current, &incoming, true).unwrap();
        assert_eq!(
            fs::read(incoming.join("my analyses/日本語.rxs")).unwrap(),
            b"original project"
        );
        let backup = directory.path().join("backup");
        copy_tree(&current, &backup).unwrap();
        fs::write(current.join("rexafs"), "installer overwrote file").unwrap();
        assert_eq!(
            fs::read(backup.join("rexafs")).unwrap(),
            b"binary",
            "backups must not be hard links"
        );
        inventory(&incoming); // A future package tries to own the user's path.
        assert!(
            preserve_extras(&backup, &incoming, false)
                .unwrap_err()
                .contains("overwrite")
        );
    }

    #[test]
    fn manifest_rejects_ambiguous_ownership_and_unlisted_files() {
        let directory = tempfile::tempdir().unwrap();
        let hash = crate::updates::hex(&Sha256::digest(b"data"));
        fs::write(directory.path().join("file"), b"data").unwrap();
        let entry = serde_json::json!({"path":"file", "sha256":hash});
        fs::write(
            directory.path().join(MANIFEST),
            serde_json::to_vec(&serde_json::json!({"version":1,"files":[entry.clone(),entry]}))
                .unwrap(),
        )
        .unwrap();
        assert!(
            manifest(directory.path())
                .unwrap_err()
                .contains("duplicate")
        );
        inventory(directory.path());
        fs::write(directory.path().join("extra"), "unlisted code").unwrap();
        assert!(
            verify(directory.path(), false)
                .unwrap_err()
                .contains("Unlisted")
        );
    }

    #[cfg(unix)]
    #[test]
    fn links_in_the_current_app_are_never_followed_into_user_files() {
        let directory = tempfile::tempdir().unwrap();
        let app = directory.path().join("app");
        fs::create_dir(&app).unwrap();
        let outside = directory.path().join("private.rxs");
        fs::write(&outside, "project").unwrap();
        std::os::unix::fs::symlink(&outside, app.join("linked.rxs")).unwrap();
        assert!(files(&app).unwrap_err().contains("links"));
        assert_eq!(fs::read(outside).unwrap(), b"project");
    }
}
