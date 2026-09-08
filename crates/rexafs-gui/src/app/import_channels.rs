//! New interpretations own independent processing parameters and immutable IDs.
use super::{
    StudioApp,
    import_preview::{self, PreviewKey, SourceRevision},
    import_repair::{RepairScope, RepairTarget, RepairValidation, Validation, preflight},
    shell::tools::ToolTarget,
};
use crate::{
    import_mapping::{LayoutKey, MappingDraft},
    params::{DerivedSpectrum, DetectionMode, ImportConfig, PipelineParams, detect_import},
};
use gpui::Context;
use std::collections::BTreeSet;
use std::path::PathBuf;

#[derive(Clone)]
pub(crate) struct ChannelScope {
    pub targets: RepairScope,
    pub batch: Option<usize>,
    pub existing: BTreeSet<PathBuf>,
}

pub(crate) fn channel_params(mapping: ImportConfig) -> PipelineParams {
    PipelineParams {
        import: mapping,
        ..Default::default()
    }
}

pub(crate) fn validate(
    scope: ChannelScope,
    layout: LayoutKey,
    mapping: ImportConfig,
    revision: u64,
) -> RepairValidation {
    let results = scope
        .targets
        .targets
        .iter()
        .map(|entry| {
            if entry.changed {
                return Validation::Changed;
            }
            if scope.existing.contains(&entry.target.path) {
                return Validation::Existing;
            }
            let result = (|| -> Result<Validation, String> {
                let detected = detect_import(&entry.target.path, &entry.params.import)?
                    .ok_or("Could not inspect source columns.")?;
                if detected.resolved.mode == mapping.mode {
                    return Ok(Validation::Existing);
                }
                let preview = import_preview::load(
                    &PreviewKey {
                        group: entry
                            .target
                            .group_id
                            .clone()
                            .ok_or("Missing source identity.")?,
                        path: entry.target.path.clone(),
                        target_revision: entry.target.fingerprint,
                        draft_revision: revision,
                        source_revision: entry.source.clone()?,
                    },
                    &mapping,
                )?;
                if LayoutKey::from_preview(&preview.table) != layout {
                    return Err(
                        "Column order, names, units, parser or conversion metadata differ.".into(),
                    );
                }
                MappingDraft::new(&preview.table, &mapping).validate()?;
                preview.raw?;
                Ok(Validation::Ready(preview.table.diagnostics.warnings()))
            })();
            result.unwrap_or_else(Validation::Incompatible)
        })
        .collect();
    RepairValidation {
        draft_revision: revision,
        scope: scope.targets,
        results,
    }
}

impl StudioApp {
    fn source_has_channel(&self, path: &std::path::Path, mode: DetectionMode) -> bool {
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        let path = canonical.as_path();
        self.catalog.find_by_canonical_path(path).is_some_and(|ix| {
            self.valid_group_index(ix) && self.effective_params(ix).import.mode == mode
        }) || self.derived.iter().any(|d| {
            d.source.as_deref() == Some(path)
                && d.params.as_ref().is_some_and(|p| p.import.mode == mode)
        })
    }

    pub(crate) fn capture_channel_scope(
        &self,
        target: &ToolTarget,
        mode: DetectionMode,
        batch: bool,
    ) -> Option<ChannelScope> {
        let origin = target
            .group_id
            .as_ref()
            .and_then(|id| self.intake.origin(&target.path, id));
        let batch_id = origin.map(|o| o.batch);
        let ids = if batch {
            self.bind_intake_batch(batch_id?);
            let batch = self.intake.history.get(batch_id?)?;
            batch
                .sources
                .values()
                .filter_map(|source| {
                    source
                        .created
                        .iter()
                        .find(|id| self.menu_index(id).is_some())
                        .cloned()
                })
                .collect::<Vec<_>>()
        } else {
            vec![target.group_id.clone()?]
        };
        let mut seen = BTreeSet::new();
        let targets = ids
            .iter()
            .filter_map(|id| self.menu_index(id))
            .filter_map(|ix| {
                let target = self
                    .tool_target(ix)
                    .filter(|t| !t.path.as_os_str().is_empty())?;
                if !seen.insert(target.path.clone()) {
                    return None;
                }
                Some(RepairTarget {
                    source: SourceRevision::read(&target.path),
                    target,
                    params: self.effective_params(ix).clone(),
                    locked: false,
                    changed: false,
                })
            })
            .collect();
        let existing = seen
            .into_iter()
            .filter(|path| self.source_has_channel(path, mode))
            .collect();
        Some(ChannelScope {
            batch: batch_id,
            existing,
            targets: RepairScope {
                label: if batch {
                    format!("Add {} · import batch {}", mode.label(), batch_id? + 1)
                } else {
                    format!("Add {} · this file", mode.label())
                },
                targets,
            },
        })
    }

    pub(crate) fn refresh_channel_scope(
        &self,
        scope: &ChannelScope,
        mode: DetectionMode,
    ) -> ChannelScope {
        let mut scope = scope.clone();
        scope.targets = self.refresh_repair_scope(&scope.targets);
        for target in &mut scope.targets.targets {
            target.locked = false;
        }
        scope.existing = scope
            .targets
            .targets
            .iter()
            .filter(|t| self.source_has_channel(&t.target.path, mode))
            .map(|t| t.target.path.clone())
            .collect();
        scope
    }

    pub(crate) fn apply_channels(
        &mut self,
        scope: &ChannelScope,
        validation: &RepairValidation,
        mapping: &ImportConfig,
        revision: u64,
        cx: &mut Context<Self>,
    ) -> Result<usize, String> {
        let targets = preflight(validation, revision, |id| {
            self.menu_index(id)
                .and_then(|ix| self.tool_target(ix))
                .map(|t| (t, false))
        })?;
        // A sibling channel arriving after validation must be reviewed as already
        // present, never duplicated or overwritten.
        if targets
            .iter()
            .any(|(_, t)| self.source_has_channel(&t.path, mapping.mode))
        {
            return Err(
                "A channel was added after validation; revalidate the missing-channel counts."
                    .into(),
            );
        }
        let start = self.derived.len();
        let mut created = BTreeSet::new();
        for (index, target) in targets {
            let mut group = DerivedSpectrum {
                id: self.next_group_id(),
                label: target
                    .path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned(),
                source: Some(
                    target
                        .path
                        .canonicalize()
                        .unwrap_or_else(|_| target.path.clone()),
                ),
                params: Some(channel_params(mapping.clone())),
                ..Default::default()
            };
            self.group_registry
                .assign_group(&mut group, &self.project_source_origins);
            let id = group.group_id.clone().unwrap();
            self.group_state
                .colors
                .entry(id.clone())
                .or_insert_with(|| super::group_rows::color_index(&id) as u8);
            if let Some(batch) = scope.batch.and_then(|id| self.intake.history.get_mut(id)) {
                batch
                    .sources
                    .entry(target.path.clone())
                    .or_default()
                    .created
                    .push(id.clone());
            }
            if let Validation::Ready(warnings) = &validation.results[index] {
                self.group_diagnostics.set_warnings(
                    id.clone(),
                    group.fingerprint(group.params.as_ref().unwrap()),
                    warnings
                        .iter()
                        .map(|message| super::JobError::warning(&target.path, message.clone()))
                        .collect(),
                );
            }
            created.insert(id);
            self.derived.push(group);
        }
        let count = created.len();
        self.group_registry.append_derived(&self.derived, start);
        if count > 0 {
            self.record_created_groups(
                created,
                format!("Add {count} {} groups", mapping.mode.label()),
            );
            self.group_registry.replace_derived(&self.derived);
            self.invalidate_explore_plots(cx);
        }
        self.status = format!(
            "Added {count} {} groups · {}",
            mapping.mode.label(),
            validation.summary()
        )
        .into();
        cx.notify();
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::group_identity::GroupId;

    #[test]
    fn forty_sources_with_three_existing_references_plan_exactly_thirty_seven() {
        let root =
            std::env::temp_dir().join(format!("rexafs-channel-batch-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let mut existing = BTreeSet::new();
        let mut targets = Vec::new();
        let sample = PipelineParams {
            e0: Some(8980.),
            edge_step: Some(42.),
            bkg_ek0: Some(8980.),
            import: ImportConfig {
                mode: DetectionMode::Transmission,
                ..Default::default()
            },
            ..Default::default()
        };
        for i in 0..40 {
            let path = root.join(format!("scan_{i}.dat"));
            std::fs::write(
                &path,
                "# energy i0 it ir\n8900 20 10 5\n9000 22 10 4\n9100 24 10 3\n",
            )
            .unwrap();
            if i < 3 {
                existing.insert(path.clone());
            }
            let mut target = ToolTarget::standalone(
                Some(GroupId::source(&path, DetectionMode::Transmission)),
                path.clone(),
                format!("scan {i}"),
                sample.fingerprint(),
                1,
                1,
            );
            target.ix = i;
            targets.push(RepairTarget {
                target,
                params: sample.clone(),
                source: SourceRevision::read(&path),
                locked: true,
                changed: false,
            });
        }
        let scope = ChannelScope {
            targets: RepairScope {
                label: "Reference · batch 1".into(),
                targets,
            },
            batch: Some(0),
            existing,
        };
        let mapping = ImportConfig {
            mode: DetectionMode::Reference,
            ..Default::default()
        };
        let table =
            crate::params::preview_import(&scope.targets.targets[0].target.path, &mapping).unwrap();
        let draft = MappingDraft::new(&table, &mapping);
        let result = validate(
            scope.clone(),
            LayoutKey::from_preview(&table),
            draft.config().clone(),
            draft.revision,
        );
        assert_eq!(result.ready_count(), 37);
        assert_eq!(result.file_count(), 37);
        assert!(result.summary().contains("3 already present"));
        assert!(
            result.results[..3]
                .iter()
                .all(|v| *v == Validation::Existing)
        );
        // Source processing locks are not copied to the independent new group.
        let params = channel_params(draft.config().clone());
        let mut expected = sample.clone();
        expected.import = params.import.clone();
        assert!(params != expected);
        assert_eq!(params.e0, None);
        assert_eq!(params.edge_step, None);
        assert_eq!(params.bkg_ek0, None);
        let mut defaults = PipelineParams::default();
        defaults.import = params.import.clone();
        assert!(params == defaults);
        let mut repeated = scope;
        repeated.existing = repeated
            .targets
            .targets
            .iter()
            .map(|t| t.target.path.clone())
            .collect();
        let result = validate(
            repeated,
            LayoutKey::from_preview(&table),
            draft.config().clone(),
            draft.revision,
        );
        assert_eq!(result.ready_count(), 0);
        assert!(result.results.iter().all(|v| *v == Validation::Existing));
        std::fs::remove_dir_all(root).unwrap();
    }
}
