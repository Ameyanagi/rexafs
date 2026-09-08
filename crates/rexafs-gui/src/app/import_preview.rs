//! Full-file raw previews belong to an exact draft and source revision.
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::group_identity::GroupId;
use crate::params::{ImportConfig, ImportPreview, RawData, preview_import_raw};

pub const DEBOUNCE: std::time::Duration = std::time::Duration::from_millis(200);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceRevision {
    bytes: u64,
    modified: SystemTime,
}

impl SourceRevision {
    pub fn read(path: &Path) -> Result<Self, String> {
        let metadata = std::fs::metadata(path).map_err(|e| e.to_string())?;
        Ok(Self {
            bytes: metadata.len(),
            modified: metadata.modified().map_err(|e| e.to_string())?,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreviewKey {
    pub group: GroupId,
    pub path: PathBuf,
    pub target_revision: u64,
    pub draft_revision: u64,
    pub source_revision: SourceRevision,
}

pub struct PreviewResult {
    pub table: ImportPreview,
    pub raw: Result<RawData, String>,
}

pub fn load(key: &PreviewKey, config: &ImportConfig) -> Result<PreviewResult, String> {
    let check = || -> Result<(), String> {
        if SourceRevision::read(&key.path)? != key.source_revision {
            Err("Source changed while reviewing mapping; reload the preview.".into())
        } else {
            Ok(())
        }
    };
    check()?;
    let (table, raw) = preview_import_raw(&key.path, config)?;
    check()?;
    Ok(PreviewResult { table, raw })
}

#[derive(Default)]
pub struct PreviewState {
    pub key: Option<PreviewKey>,
    pub result: Option<Result<PreviewResult, String>>,
}

impl PreviewState {
    /// Invalidation immediately removes the preceding draft's plot and Apply.
    pub fn begin(&mut self, key: PreviewKey) {
        self.key = Some(key);
        self.result = None;
    }

    pub fn invalidate(&mut self) {
        self.key = None;
        self.result = None;
    }

    pub fn finish(&mut self, key: &PreviewKey, result: Result<PreviewResult, String>) -> bool {
        if self.key.as_ref() != Some(key) {
            return false;
        }
        self.result = Some(result);
        true
    }

    pub fn ready(&self, key: &PreviewKey) -> bool {
        self.key.as_ref() == Some(key)
            && matches!(&self.result, Some(Ok(PreviewResult { raw: Ok(_), .. })))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::DetectionMode;

    #[test]
    fn preview_requires_exact_target_draft_and_unchanged_source() {
        let path =
            std::env::temp_dir().join(format!("rexafs-raw-preview-{}.dat", std::process::id()));
        let mut text = String::from("# energy i0 it\n");
        for i in 0..6000 {
            text.push_str(&format!("{} 10 5\n", 8900 + i));
        }
        text.push_str("15000 10 0\n");
        std::fs::write(&path, text).unwrap();
        let mut key = PreviewKey {
            group: GroupId::source(&path, DetectionMode::Transmission),
            path: path.clone(),
            target_revision: 1,
            draft_revision: 1,
            source_revision: SourceRevision::read(&path).unwrap(),
        };
        let mut state = PreviewState::default();
        state.begin(key.clone());
        assert!(!state.ready(&key));
        let result = load(&key, &ImportConfig::default()).unwrap();
        assert_eq!(result.raw.as_ref().unwrap().energy.len(), 6000);
        assert_eq!(result.table.rows.len(), 3);
        assert_eq!(result.table.diagnostics.excluded_signal_points.count, 1);
        assert!(state.finish(&key, Ok(result)));
        assert!(state.ready(&key));
        let prior = key.clone();
        key.draft_revision += 1;
        state.begin(key.clone());
        assert!(!state.ready(&prior));
        assert!(!state.finish(&prior, load(&prior, &ImportConfig::default())));
        let mut different_target = key.clone();
        different_target.target_revision += 1;
        assert!(!state.finish(&different_target, Err("late".into())));
        std::fs::write(&path, "# energy i0 it\n8900 10 5\n9000 10 5\n").unwrap();
        assert!(load(&key, &ImportConfig::default()).is_err());
        state.invalidate();
        assert!(!state.finish(&key, Err("closed".into())));
        std::fs::remove_file(path).unwrap();
    }
}
