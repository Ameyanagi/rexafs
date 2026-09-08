//! Unconfirmed sources stay outside the catalog until an explicit review commits.
use std::{collections::BTreeMap, path::PathBuf};

use super::{StudioApp, import_repair::RepairTarget, shell::tools::ToolTarget};
use super::{
    import_preview::{self, PreviewKey, SourceRevision},
    import_repair::{RepairScope, RepairValidation, Validation},
};
use crate::{group_identity::GroupId, params::PipelineParams};
use crate::{
    import_mapping::{LayoutKey, MappingDraft},
    params::{DetectionMode, ImportConfig, ImportDetection},
};
use gpui::Context;

#[derive(Clone)]
pub(crate) struct PendingSource {
    pub detection: Option<ImportDetection>,
    pub reason: String,
}

#[derive(Clone)]
pub(crate) struct ReviewCluster {
    pub key: Option<LayoutKey>,
    pub files: Vec<PathBuf>,
    pub reason: String,
    pub primary: DetectionMode,
}

impl ReviewCluster {
    pub fn label(&self) -> String {
        match &self.key {
            Some(key) => format!(
                "{} files · {} columns · {}",
                self.files.len(),
                key.column_count,
                key.names
                    .as_ref()
                    .map(|names| names.iter().take(4).cloned().collect::<Vec<_>>().join(", "))
                    .unwrap_or("unnamed columns".into())
            ),
            None => format!(
                "Source needs repair · {}",
                self.files[0]
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
            ),
        }
    }
}

pub(crate) fn clusters(
    sources: impl IntoIterator<Item = (PathBuf, PendingSource)>,
) -> Vec<ReviewCluster> {
    let mut clusters: Vec<ReviewCluster> = Vec::new();
    for (path, pending) in sources {
        let key = pending.detection.as_ref().map(LayoutKey::from_detection);
        // An unreadable file has no compatibility evidence; never group failures
        // merely because their error messages or filename patterns resemble each other.
        if let Some(cluster) = key.as_ref().and_then(|key| {
            clusters
                .iter_mut()
                .find(|cluster| cluster.key.as_ref() == Some(key))
        }) {
            cluster.files.push(path);
        } else {
            clusters.push(ReviewCluster {
                key,
                files: vec![path],
                reason: pending.reason,
                primary: pending
                    .detection
                    .map(|d| d.resolved.mode)
                    .unwrap_or(DetectionMode::Transmission),
            });
        }
    }
    clusters
}

#[derive(Clone)]
pub(crate) struct ReviewChoice {
    pub primary: DetectionMode,
    pub channels: Vec<ImportConfig>,
}

impl ReviewChoice {
    pub fn primary_config(&self) -> Option<&ImportConfig> {
        self.channels
            .iter()
            .find(|config| config.mode == self.primary)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.primary == DetectionMode::Auto || self.primary_config().is_none() {
            return Err("Choose a primary channel included in the output set.".into());
        }
        let mut modes = Vec::new();
        for config in &self.channels {
            if config.mode == DetectionMode::Auto || modes.contains(&config.mode) {
                return Err("Choose distinct, explicit output channels.".into());
            }
            modes.push(config.mode);
        }
        Ok(())
    }
}

#[derive(Clone)]
pub(crate) struct ReviewScope {
    pub batch: usize,
    pub cluster: usize,
    pub cluster_count: usize,
    pub project_generation: u64,
    pub targets: RepairScope,
    pub primary: DetectionMode,
    pub reason: String,
}

#[derive(Clone)]
pub(crate) struct Approval {
    pub batch: usize,
    pub choice: ReviewChoice,
    pub layout: LayoutKey,
    pub source: SourceRevision,
}

pub(crate) type Approvals = BTreeMap<PathBuf, Approval>;

pub(crate) fn validate(
    scope: RepairScope,
    layout: LayoutKey,
    choice: ReviewChoice,
    revision: u64,
) -> RepairValidation {
    let results = scope
        .targets
        .iter()
        .map(|target| {
            if target.changed {
                return Validation::Changed;
            }
            let result = (|| -> Result<Vec<String>, String> {
                choice.validate()?;
                let mut warnings = Vec::new();
                for mapping in &choice.channels {
                    let loaded = import_preview::load(
                        &PreviewKey {
                            group: target
                                .target
                                .group_id
                                .clone()
                                .ok_or("Missing pending source identity.")?,
                            path: target.target.path.clone(),
                            target_revision: target.target.fingerprint,
                            draft_revision: revision,
                            source_revision: target.source.clone()?,
                        },
                        mapping,
                    )?;
                    if LayoutKey::from_preview(&loaded.table) != layout {
                        return Err(
                            "This source now has a different layout; review it separately.".into(),
                        );
                    }
                    MappingDraft::new(&loaded.table, mapping).validate()?;
                    loaded.raw?;
                    warnings.extend(loaded.table.diagnostics.warnings());
                }
                warnings.sort();
                warnings.dedup();
                Ok(warnings)
            })();
            match result {
                Ok(warnings) => Validation::Ready(warnings),
                Err(error) => Validation::Incompatible(error),
            }
        })
        .collect();
    RepairValidation {
        draft_revision: revision,
        scope,
        results,
    }
}

impl StudioApp {
    pub(crate) fn locate_pending(
        &mut self,
        scope: &ReviewScope,
        old: &std::path::Path,
        path: PathBuf,
    ) -> Result<ReviewScope, String> {
        if scope.project_generation != self.project_generation {
            return Err("The project changed; reopen review.".into());
        }
        if self.catalog.find_by_canonical_path(&path).is_some() {
            return Err("This source is already in the project; use its existing group.".into());
        }
        let batch = self
            .intake
            .history
            .get_mut(scope.batch)
            .ok_or("The import batch was removed.")?;
        if old != path && batch.sources.contains_key(&path) {
            return Err("This source is already in this import batch.".into());
        }
        if batch.sources.get(old).is_none_or(|s| s.pending.is_none()) {
            return Err("This source is no longer pending.".into());
        }
        let mut outcome = batch.sources.remove(old).unwrap();
        outcome.failed = None;
        if let Some(pending) = &mut outcome.pending {
            pending.reason = "Source located; confirm its columns.".into();
            pending.detection = None;
        }
        batch.sources.insert(path.clone(), outcome);
        for requested in &mut batch.paths {
            if requested == old {
                *requested = path.clone();
            }
        }
        self.intake.approved.remove(old);
        let mut next = scope.clone();
        next.reason = "Source located; confirm its columns.".into();
        next.targets.label = format!(
            "Layout {} of {} · {} captured files",
            next.cluster + 1,
            next.cluster_count,
            next.targets.targets.len()
        );
        for target in &mut next.targets.targets {
            if target.target.path == old {
                target.target.path = path.clone();
                target.target.label = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned();
                target.target.group_id = Some(GroupId::source(&path, DetectionMode::Auto));
                target.source = SourceRevision::read(&path);
            }
        }
        Ok(next)
    }

    pub(crate) fn pending_clusters(&self, batch: usize) -> Vec<ReviewCluster> {
        self.intake
            .history
            .get(batch)
            .map(|batch| {
                clusters(
                    batch
                        .sources
                        .iter()
                        .filter(|(path, _)| !self.intake.approved.contains_key(*path))
                        .filter_map(|(path, source)| {
                            source.pending.clone().map(|p| (path.clone(), p))
                        }),
                )
            })
            .unwrap_or_default()
    }

    pub(crate) fn capture_review_scope(&self, batch: usize, cluster: usize) -> Option<ReviewScope> {
        let clusters = self.pending_clusters(batch);
        let cluster_count = clusters.len();
        let layout = clusters.get(cluster)?;
        let params = PipelineParams {
            import: ImportConfig {
                mode: layout.primary,
                ..Default::default()
            },
            ..Default::default()
        };
        let targets = layout
            .files
            .iter()
            .map(|path| RepairTarget {
                target: ToolTarget::standalone(
                    Some(GroupId::source(path, DetectionMode::Auto)),
                    path.clone(),
                    path.file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned(),
                    0,
                    self.project_generation,
                    0,
                ),
                params: params.clone(),
                locked: false,
                changed: false,
                source: SourceRevision::read(path),
            })
            .collect();
        Some(ReviewScope {
            batch,
            cluster,
            cluster_count,
            project_generation: self.project_generation,
            targets: RepairScope {
                label: format!(
                    "Layout {} of {} · {}",
                    cluster + 1,
                    cluster_count,
                    layout.label()
                ),
                targets,
            },
            primary: layout.primary,
            reason: layout.reason.clone(),
        })
    }

    pub(crate) fn refresh_review_scope(&self, scope: &ReviewScope) -> RepairScope {
        let mut targets = scope.targets.clone();
        for target in &mut targets.targets {
            target.changed = self.project_generation != scope.project_generation
                || self
                    .intake
                    .history
                    .get(scope.batch)
                    .and_then(|b| b.sources.get(&target.target.path))
                    .is_none_or(|s| s.pending.is_none());
            target.source = SourceRevision::read(&target.target.path);
        }
        targets
    }

    pub(crate) fn accept_review(
        &mut self,
        scope: &ReviewScope,
        validated: &RepairValidation,
        choice: &ReviewChoice,
        layout: &LayoutKey,
        revision: u64,
        cx: &mut Context<Self>,
    ) -> Result<usize, String> {
        choice.validate()?;
        if scope.project_generation != self.project_generation
            || validated.draft_revision != revision
            || validated.ready_count() == 0
        {
            return Err("The project or review changed; validate the current selection.".into());
        }
        let mut approvals = Vec::new();
        for (target, result) in validated.scope.targets.iter().zip(&validated.results) {
            if !matches!(result, Validation::Ready(_)) {
                continue;
            }
            let path = &target.target.path;
            if self
                .intake
                .history
                .get(scope.batch)
                .and_then(|b| b.sources.get(path))
                .is_none_or(|s| s.pending.is_none())
                || self.intake.approved.contains_key(path)
            {
                return Err(
                    "A pending source was removed or accepted; review the remaining sources."
                        .into(),
                );
            }
            let source = SourceRevision::read(path)?;
            if target.source.as_ref().ok() != Some(&source) {
                return Err("A source changed after validation; reload and validate again.".into());
            }
            approvals.push((
                path.clone(),
                Approval {
                    batch: scope.batch,
                    choice: choice.clone(),
                    layout: layout.clone(),
                    source,
                },
            ));
        }
        let paths: Vec<_> = approvals.iter().map(|(path, _)| path.clone()).collect();
        let count = paths.len();
        self.intake.approved.extend(approvals);
        self.intake.enqueue_review(scope.batch, paths);
        self.start_queued_import(cx);
        cx.notify();
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::{detect_import, preview_import};

    #[test]
    fn mixed_layouts_remain_separate_and_partial_review_validates_every_output() {
        let root =
            std::env::temp_dir().join(format!("rexafs-layout-review-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let mut pending = Vec::new();
        for (name, text) in [
            ("a1.dat", "8900 10 5\n9000 12 5\n9100 14 5\n"),
            ("a2.dat", "8900 10 4\n9000 12 4\n9100 14 4\n"),
            (
                "b1.dat",
                "# coordinate first second last\n8900 10 5 2\n9000 12 5 2\n9100 14 5 2\n",
            ),
            ("broken1.dat", "not a numeric source\n"),
            ("broken2.dat", "not a numeric source\n"),
        ] {
            let path = root.join(name);
            std::fs::write(&path, text).unwrap();
            let detection = detect_import(&path, &ImportConfig::default())
                .ok()
                .flatten();
            let reason = detection
                .as_ref()
                .and_then(|d| d.review_reason())
                .unwrap_or("Unreadable".into());
            pending.push((path, PendingSource { detection, reason }));
        }
        let grouped = clusters(pending.clone());
        assert_eq!(grouped.len(), 4);
        assert_eq!(grouped[0].files.len(), 2);
        assert_eq!(grouped[1].files.len(), 1);
        assert!(
            grouped[2..]
                .iter()
                .all(|c| c.key.is_none() && c.files.len() == 1)
        );
        let table = preview_import(&grouped[0].files[0], &ImportConfig::default()).unwrap();
        let draft = MappingDraft::new(&table, &ImportConfig::default());
        let choice = ReviewChoice {
            primary: DetectionMode::Transmission,
            channels: vec![
                draft.config().clone(),
                ImportConfig {
                    mode: DetectionMode::MuColumn,
                    energy_col: Some(0),
                    mu_col: Some(2),
                    ..Default::default()
                },
            ],
        };
        let targets = pending[..3]
            .iter()
            .map(|(path, _)| RepairTarget {
                target: ToolTarget::standalone(
                    Some(GroupId::source(path, DetectionMode::Auto)),
                    path.clone(),
                    path.display().to_string(),
                    0,
                    1,
                    0,
                ),
                params: PipelineParams::default(),
                locked: false,
                changed: false,
                source: SourceRevision::read(path),
            })
            .collect();
        let scope = RepairScope {
            label: "Mixed folder".into(),
            targets,
        };
        let result = validate(
            scope.clone(),
            LayoutKey::from_preview(&table),
            choice.clone(),
            0,
        );
        assert_eq!(result.ready_count(), 2);
        assert!(matches!(result.results[2], Validation::Incompatible(_)));
        let mut invalid_outputs = choice;
        invalid_outputs.channels.push(ImportConfig {
            mode: DetectionMode::Reference,
            ir_col: Some(9),
            ..Default::default()
        });
        assert_eq!(
            validate(
                scope.clone(),
                LayoutKey::from_preview(&table),
                invalid_outputs,
                1
            )
            .ready_count(),
            0
        );
        std::fs::write(
            &scope.targets[0].target.path,
            "8900 100 50\n9000 120 50\n9100 140 50\n",
        )
        .unwrap();
        assert_eq!(
            validate(
                scope,
                LayoutKey::from_preview(&table),
                ReviewChoice {
                    primary: DetectionMode::Transmission,
                    channels: vec![draft.config().clone()]
                },
                2
            )
            .ready_count(),
            1
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}
