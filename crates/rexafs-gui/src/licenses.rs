//! Offline notices shipped with the desktop application.
use std::path::{Path, PathBuf};

pub(crate) struct Document {
    pub title: String,
    source: Source,
}

enum Source {
    Embedded(&'static str),
    File(PathBuf),
    Inventory(PathBuf),
}

impl Document {
    pub fn read(&self) -> Result<String, String> {
        match &self.source {
            Source::Embedded(text) => Ok((*text).into()),
            Source::File(path) => std::fs::read(path)
                .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
                .map_err(|error| format!("Could not read {}: {error}", path.display())),
            Source::Inventory(path) => {
                let bytes = std::fs::read(path).map_err(|error| error.to_string())?;
                let packages: Vec<serde_json::Value> =
                    serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
                let mut text =
                    "Resolved dependencies, including build dependencies.\n\n".to_string();
                for package in packages {
                    for key in ["name", "version", "license", "repository"] {
                        if let Some(value) = package[key].as_str() {
                            text.push_str(value);
                            text.push('\n');
                        }
                    }
                    text.push('\n');
                }
                Ok(text)
            }
        }
    }
}

fn notices_root(executable: &Path) -> Option<PathBuf> {
    let directory = executable.parent()?;
    [
        directory.join("../Resources/notices"),
        directory.to_path_buf(),
    ]
    .into_iter()
    .find(|path| path.join("dependencies.json").is_file() && path.join("licenses").is_dir())
}

fn documents_for(executable: &Path) -> Vec<Document> {
    let mut documents = vec![
        Document {
            title: "rexafs · MIT".into(),
            source: Source::Embedded(include_str!("../../../LICENSE-MIT")),
        },
        Document {
            title: "rexafs · Apache-2.0".into(),
            source: Source::Embedded(include_str!("../../../LICENSE-APACHE")),
        },
    ];
    if let Some(root) = notices_root(executable) {
        documents.push(Document {
            title: "Dependency inventory".into(),
            source: Source::Inventory(root.join("dependencies.json")),
        });
        let directory = root.join("licenses");
        for entry in walkdir::WalkDir::new(&directory)
            .follow_links(false)
            .sort_by_file_name()
            .into_iter()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().is_file())
        {
            documents.push(Document {
                title: entry
                    .path()
                    .strip_prefix(&directory)
                    .unwrap_or(entry.path())
                    .components()
                    .map(|part| part.as_os_str().to_string_lossy())
                    .collect::<Vec<_>>()
                    .join(" · "),
                source: Source::File(entry.into_path()),
            });
        }
    } else {
        documents.push(Document {
            title: "Third-party notices".into(),
            source: Source::Embedded(
                "Third-party notices are included in the packaged desktop application.",
            ),
        });
    }
    if let Some(directory) = executable.parent() {
        for path in [
            directory.join("../Resources/examples/PROVENANCE.md"),
            directory.join("resources/examples/PROVENANCE.md"),
        ] {
            if path.is_file() {
                documents.push(Document {
                    title: "Example data · provenance".into(),
                    source: Source::File(path),
                });
                break;
            }
        }
    }
    documents
}

pub(crate) fn documents() -> Vec<Document> {
    documents_for(&std::env::current_exe().unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notices_survive_app_only_install_and_work_in_flat_desktop_layouts() {
        let root = std::env::temp_dir().join(format!(
            "rexafs-license-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        for (binary, notices) in [
            (
                "rexafs.app/Contents/MacOS/rexafs",
                "rexafs.app/Contents/Resources/notices",
            ),
            ("linux/rexafs", "linux"),
            ("windows/rexafs.exe", "windows"),
        ] {
            std::fs::create_dir_all(root.join(binary).parent().unwrap()).unwrap();
            let directory = root.join(notices);
            std::fs::create_dir_all(directory.join("licenses/example-1.0")).unwrap();
            std::fs::write(
                directory.join("dependencies.json"),
                r#"[{"name":"example","version":"1.0","license":"MIT"}]"#,
            )
            .unwrap();
            std::fs::write(
                directory.join("licenses/example-1.0/LICENSE"),
                "Original third-party notice\n",
            )
            .unwrap();
            let documents = documents_for(&root.join(binary));
            assert!(
                documents[0]
                    .read()
                    .unwrap()
                    .contains("Permission is hereby granted")
            );
            assert!(documents[1].read().unwrap().contains("Apache License"));
            assert!(
                documents
                    .iter()
                    .any(|document| document.title == "Dependency inventory"
                        && document.read().unwrap().contains("example\n1.0\nMIT"))
            );
            assert!(
                documents
                    .iter()
                    .any(|document| document.read().unwrap() == "Original third-party notice\n")
            );
        }
        std::fs::remove_dir_all(root).unwrap();
    }
}
