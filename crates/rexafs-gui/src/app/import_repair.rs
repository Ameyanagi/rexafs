//! Explicit repair validates a frozen set; later imports never join it.
use std::collections::BTreeSet;

use gpui::Context;

use super::shell::{journal::UndoOp, tools::ToolTarget};
use super::{
    StudioApp, evict_group_keys,
    import_preview::{self, PreviewKey, SourceRevision},
};
use crate::{
    group_identity::GroupId,
    import_mapping::{LayoutKey, MappingDraft},
    params::{DetectionMode, ImportConfig, PipelineParams, detect_import},
};

#[derive(Clone)]
pub(crate) struct RepairTarget {
    pub target: ToolTarget,
    pub params: PipelineParams,
    pub locked: bool,
    pub changed: bool,
    pub source: Result<SourceRevision, String>,
}

#[derive(Clone)]
pub(crate) struct RepairScope {
    pub label: String,
    pub targets: Vec<RepairTarget>,
}

fn application_targets(
    application: &crate::import_recipes::ImportApplication,
    channel: DetectionMode,
    mut resolve: impl FnMut(&crate::import_recipes::ApplicationMember) -> Option<RepairTarget>,
) -> Vec<RepairTarget> {
    application
        .members
        .iter()
        .filter(|member| member.channel == channel)
        .filter_map(|member| {
            let mut target = resolve(member)?;
            target.changed |= crate::import_recipes::mapping_revision(&target.params.import)
                != member.mapping_revision;
            Some(target)
        })
        .collect()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Validation {
    Ready(Vec<String>),
    Existing,
    Locked,
    Changed,
    Incompatible(String),
}

impl Validation {
    pub fn description(&self) -> String {
        match self {
            Self::Existing => "Channel already present · skipped".into(),
            Self::Ready(warnings) if warnings.is_empty() => "Compatible".into(),
            Self::Ready(warnings) => format!("Compatible · {}", warnings.join(" · ")),
            Self::Locked => "Processing locked · skipped".into(),
            Self::Changed => "Target changed · skipped; reopen to include its new revision".into(),
            Self::Incompatible(reason) => format!("Incompatible · {reason}"),
        }
    }
}

pub(crate) struct RepairValidation {
    pub draft_revision: u64,
    pub scope: RepairScope,
    pub results: Vec<Validation>,
}

impl RepairValidation {
    pub fn ready_count(&self) -> usize {
        self.results
            .iter()
            .filter(|v| matches!(v, Validation::Ready(_)))
            .count()
    }
    pub fn file_count(&self) -> usize {
        self.scope
            .targets
            .iter()
            .zip(&self.results)
            .filter(|(_, v)| matches!(v, Validation::Ready(_)))
            .map(|(t, _)| &t.target.path)
            .collect::<BTreeSet<_>>()
            .len()
    }
    pub fn summary(&self) -> String {
        let count = |predicate: fn(&Validation) -> bool| {
            self.results.iter().filter(|v| predicate(v)).count()
        };
        let mut summary = format!(
            "{} compatible files · {} groups · {} incompatible · {} locked · {} changed",
            self.file_count(),
            self.ready_count(),
            count(|v| matches!(v, Validation::Incompatible(_))),
            count(|v| matches!(v, Validation::Locked)),
            count(|v| matches!(v, Validation::Changed))
        );
        let existing = count(|v| matches!(v, Validation::Existing));
        if existing > 0 {
            summary.push_str(&format!(" · {existing} already present"));
        }
        summary
    }
}

/// Slot shifts and catalog appends are harmless; the identity and revision must match.
fn same_revision(a: &ToolTarget, b: &ToolTarget) -> bool {
    a.group_id == b.group_id
        && a.path == b.path
        && a.fingerprint == b.fingerprint
        && a.project_generation == b.project_generation
        && a.derived_id == b.derived_id
        && a.size == b.size
}

pub(crate) fn validate(
    scope: RepairScope,
    key: LayoutKey,
    mapping: ImportConfig,
    draft_revision: u64,
) -> RepairValidation {
    let results = scope
        .targets
        .iter()
        .map(|target| {
            if target.changed {
                return Validation::Changed;
            }
            if target.locked {
                return Validation::Locked;
            }
            let result = (|| {
                let source_revision = target.source.clone()?;
                let detected = detect_import(&target.target.path, &target.params.import)?
                    .ok_or("Could not resolve this group's channel.")?;
                if detected.resolved.mode != mapping.mode {
                    return Err(format!(
                        "{} channel, expected {}",
                        detected.resolved.mode.label(),
                        mapping.mode.label()
                    ));
                }
                let preview_key = PreviewKey {
                    group: target
                        .target
                        .group_id
                        .clone()
                        .ok_or("Missing group identity.")?,
                    path: target.target.path.clone(),
                    target_revision: target.target.fingerprint,
                    draft_revision,
                    source_revision,
                };
                let loaded = import_preview::load(&preview_key, &mapping)?;
                if LayoutKey::from_preview(&loaded.table) != key {
                    return Err(
                        "Column order, names, units, parser or conversion metadata differ.".into(),
                    );
                }
                MappingDraft::new(&loaded.table, &mapping).validate()?;
                loaded.raw?;
                Ok(loaded.table.diagnostics.warnings())
            })();
            match result {
                Ok(warnings) => Validation::Ready(warnings),
                Err(error) => Validation::Incompatible(error),
            }
        })
        .collect();
    RepairValidation {
        draft_revision,
        scope,
        results,
    }
}

pub(super) fn preflight(
    validation: &RepairValidation,
    draft_revision: u64,
    current: impl Fn(&GroupId) -> Option<(ToolTarget, bool)>,
) -> Result<Vec<(usize, ToolTarget)>, String> {
    if validation.draft_revision != draft_revision || validation.ready_count() == 0 {
        return Err("Validate the current draft before applying.".into());
    }
    // Preflight the entire accepted set before the first mutation.
    let mut accepted = Vec::new();
    for (index, (entry, result)) in validation
        .scope
        .targets
        .iter()
        .zip(&validation.results)
        .enumerate()
    {
        if !matches!(result, Validation::Ready(_)) {
            continue;
        }
        let (current, locked) = entry
            .target
            .group_id
            .as_ref()
            .and_then(|id| current(id))
            .ok_or("A target was removed; revalidate.")?;
        if !same_revision(&entry.target, &current)
            || locked
            || SourceRevision::read(&current.path).ok().as_ref() != entry.source.as_ref().ok()
        {
            return Err(
                "A target or source changed after validation; revalidate before applying.".into(),
            );
        }
        accepted.push((index, current));
    }
    Ok(accepted)
}

impl StudioApp {
    /// A receipt may refer to a lazy primary that has never been selected.
    /// Bind only the source identities in the explicitly chosen batch before
    /// resolving its members; otherwise unseen sources disappear from counts.
    pub(crate) fn bind_intake_batch(&self, batch_id: usize) {
        if let Some(batch) = self.intake.history.get(batch_id) {
            for (path, source) in &batch.sources {
                if let Some(ix) = self.catalog.find_by_canonical_path(path)
                    && self
                        .peek_group_id(ix)
                        .is_some_and(|id| source.created.contains(&id))
                {
                    self.group_id(ix);
                }
            }
        }
    }

    pub(crate) fn capture_repair_batch(
        &self,
        target: &ToolTarget,
        channel: DetectionMode,
    ) -> Option<RepairScope> {
        if let Some(application) = target
            .group_id
            .as_ref()
            .and_then(|group| self.imports.application_for(group))
        {
            let targets = application_targets(application, channel, |member| {
                let ix = self.intake_group_index(&member.path, &member.group)?;
                self.group_id(ix);
                let target = self.tool_target(ix)?;
                let params = self.effective_params(ix).clone();
                Some(RepairTarget {
                    changed: false,
                    source: SourceRevision::read(&target.path),
                    target,
                    params,
                    locked: self.frozen.contains(&ix),
                })
            });
            let label = self
                .imports
                .recipes
                .get(&application.recipe)
                .map(|recipe| recipe.label())
                .unwrap_or_else(|| "Saved import application".into());
            return Some(RepairScope {
                label: format!("{} · {} · exact application", label, channel.label()),
                targets,
            });
        }
        let origin = self
            .intake
            .origin(&target.path, target.group_id.as_ref()?)?;
        self.bind_intake_batch(origin.batch);
        let batch = self.intake.history.get(origin.batch)?;
        let mut seen = BTreeSet::new();
        let targets = batch
            .created()
            .filter(|id| seen.insert((*id).clone()))
            .filter_map(|id| self.menu_index(id))
            .filter(|&ix| ix != super::NO_ENTRY)
            .filter_map(|ix| {
                let params = self.effective_params(ix).clone();
                if params.import.mode != DetectionMode::Auto && params.import.mode != channel {
                    return None;
                }
                let target = self
                    .tool_target(ix)
                    .filter(|t| !t.path.as_os_str().is_empty())?;
                Some(RepairTarget {
                    source: SourceRevision::read(&target.path),
                    target,
                    params,
                    locked: self.frozen.contains(&ix),
                    changed: false,
                })
            })
            .collect::<Vec<_>>();
        (!targets.is_empty()).then(|| RepairScope {
            label: format!(
                "{} groups from import batch {}",
                channel.label(),
                origin.batch + 1
            ),
            targets,
        })
    }

    /// Revalidation refreshes source and lock state, without adding identities or
    /// silently accepting an independently edited group's processing revision.
    pub(crate) fn refresh_repair_scope(&self, scope: &RepairScope) -> RepairScope {
        let mut scope = scope.clone();
        for entry in &mut scope.targets {
            let current = entry
                .target
                .group_id
                .as_ref()
                .and_then(|id| self.menu_index(id))
                .and_then(|ix| self.tool_target(ix));
            entry.changed = entry.changed
                || current
                    .as_ref()
                    .is_none_or(|current| !same_revision(&entry.target, current));
            entry.locked = current
                .as_ref()
                .is_some_and(|current| self.frozen.contains(&current.ix));
            entry.source = SourceRevision::read(&entry.target.path);
        }
        scope
    }

    pub(crate) fn apply_repair(
        &mut self,
        validation: &RepairValidation,
        mapping: &ImportConfig,
        draft_revision: u64,
        cx: &mut Context<Self>,
    ) -> Result<usize, String> {
        // Preflight the entire accepted set before the first mutation.
        let accepted = preflight(validation, draft_revision, |id| {
            self.menu_index(id).and_then(|ix| {
                self.tool_target(ix)
                    .map(|target| (target, self.frozen.contains(&ix)))
            })
        })?;
        let mut changes = Vec::new();
        let mut indices = BTreeSet::new();
        for (index, target) in accepted {
            let result = &validation.results[index];
            let ix = target.ix;
            let before = self.custom_params(ix).cloned();
            let mut after = self.effective_params(ix).clone();
            after.import = mapping.clone();
            let after = (after != self.params).then_some(after);
            let id = target.group_id.unwrap();
            if before != after {
                self.set_custom_params(ix, after.clone());
                changes.push((id.clone(), before, after));
                indices.insert(ix);
            }
            if let Validation::Ready(warnings) = result {
                self.group_diagnostics.set_warnings(
                    id,
                    self.effective_fingerprint(ix),
                    warnings
                        .iter()
                        .map(|message| super::JobError::warning(&target.path, message.clone()))
                        .collect(),
                );
            }
        }
        // Validation failures stay associated with the unchanged group revision.
        for (entry, result) in validation.scope.targets.iter().zip(&validation.results) {
            if let Validation::Incompatible(reason) = result
                && let Some(id) = entry.target.group_id.as_ref()
                && let Some(ix) = self.menu_index(id)
                && self
                    .tool_target(ix)
                    .is_some_and(|t| same_revision(&entry.target, &t))
            {
                self.group_diagnostics.set_warnings(
                    id.clone(),
                    self.effective_fingerprint(ix),
                    vec![super::JobError::warning(
                        &entry.target.path,
                        format!("Mapping repair skipped: {reason}"),
                    )],
                );
            }
        }
        let count = changes.len();
        if count > 0 {
            self.record(
                format!("Re-map {count} groups · {}", validation.scope.label),
                Some(UndoOp::Params { changes }),
            );
            evict_group_keys(&mut self.cache, &indices, false);
            evict_group_keys(&mut self.raw_cache, &indices, false);
            self.schedule_recompute(cx);
            self.invalidate_explore_plots(cx);
            self.sync_param_fields(cx);
            self.sync_handles(cx);
        }
        self.status = format!("Updated {count} groups · {}", validation.summary()).into();
        cx.notify();
        Ok(count)
    }

    /// Materialized results retain their arrays. Follow provenance transitively
    /// so a repaired source also marks descendants of an unchanged intermediate.
    pub(crate) fn inputs_changed(&self, result: &crate::params::DerivedSpectrum) -> Option<String> {
        provenance_changed(
            result,
            &|id| {
                let ix = self.menu_index(id)?;
                Some((
                    self.effective_fingerprint(ix),
                    ix.checked_sub(super::DERIVED_BASE)
                        .and_then(|i| self.derived.get(i)),
                ))
            },
            &mut BTreeSet::new(),
        )
        .then(|| "Inputs changed · result retains the previously calculated data".into())
    }
}

fn provenance_changed<'a>(
    result: &crate::params::DerivedSpectrum,
    lookup: &impl Fn(&GroupId) -> Option<(u64, Option<&'a crate::params::DerivedSpectrum>)>,
    visited: &mut BTreeSet<GroupId>,
) -> bool {
    result.operation.as_ref().is_some_and(|op| {
        op.inputs.iter().any(|input| {
            let Some(id) = input.group_id.as_ref() else {
                return false;
            };
            if !visited.insert(id.clone()) {
                return false;
            }
            let Some((fingerprint, derived)) = lookup(id) else {
                return false;
            };
            fingerprint != input.fingerprint
                || derived.is_some_and(|d| provenance_changed(d, lookup, visited))
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::import_mapping::AxisConversion;

    #[test]
    fn application_repair_uses_exact_channel_members_and_skips_manual_mappings() {
        use crate::import_recipes::{
            ApplicationMember, ImportApplication, RecipeRef, mapping_revision,
        };
        let root = std::env::temp_dir()
            .join(crate::import_recipes::new_id("application-repair").replace(':', "-"));
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("source.dat");
        std::fs::write(&path, "# energy mu monitor\n8900 1 4\n9000 2 4\n9100 3 4\n").unwrap();
        let preview = crate::params::preview_import(&path, &ImportConfig::default()).unwrap();
        let mapping = MappingDraft::new(&preview, &ImportConfig::default())
            .config()
            .clone();
        let mut application = ImportApplication {
            id: "a".into(),
            batch: 1,
            recipe: RecipeRef {
                id: "r".into(),
                version: 1,
            },
            members: vec![],
        };
        let mut live = Vec::new();
        for i in 0..6 {
            let id = GroupId::new_result();
            let mut params = PipelineParams {
                import: mapping.clone(),
                ..Default::default()
            };
            if i == 1 {
                params.import.mu_col = Some(2);
            }
            if i == 2 {
                params.e0 = Some(9000.);
            }
            live.push(RepairTarget {
                target: ToolTarget::standalone(
                    Some(id.clone()),
                    path.clone(),
                    format!("group {i}"),
                    params.fingerprint(),
                    1,
                    1,
                ),
                params,
                locked: i == 3,
                changed: false,
                source: SourceRevision::read(&path),
            });
            if i < 5 {
                application.members.push(ApplicationMember {
                    source_id: id.clone(),
                    path: path.clone(),
                    group: id,
                    channel: if i == 4 {
                        DetectionMode::Reference
                    } else {
                        mapping.mode
                    },
                    mapping_revision: mapping_revision(&mapping),
                });
            }
        }
        let targets = application_targets(&application, mapping.mode, |member| {
            live.iter()
                .find(|target| target.target.group_id.as_ref() == Some(&member.group))
                .cloned()
        });
        assert_eq!(
            targets.len(),
            4,
            "other channels and later imports stay outside the scope"
        );
        let result = validate(
            RepairScope {
                label: "exact application".into(),
                targets,
            },
            LayoutKey::from_preview(&preview),
            mapping,
            1,
        );
        assert!(matches!(result.results[0], Validation::Ready(_)));
        assert_eq!(result.results[1], Validation::Changed);
        assert!(
            matches!(result.results[2], Validation::Ready(_)),
            "processing edits preserve mapping eligibility"
        );
        assert_eq!(result.results[3], Validation::Locked);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn input_changes_reach_materialized_descendants_and_clear_on_undo() {
        use crate::params::{DerivedSpectrum, Operation};
        let source = GroupId::new_result();
        let intermediate_id = GroupId::new_result();
        let input = |id, fingerprint| {
            ToolTarget::standalone(
                Some(id),
                "/source.dat".into(),
                "source".into(),
                fingerprint,
                1,
                1,
            )
            .operation_input()
        };
        let derived = |input| DerivedSpectrum {
            operation: Some(Operation {
                tool: "smooth".into(),
                parameters: serde_json::json!({}),
                inputs: vec![input],
                applied_energy_shift_ev: 0.,
            }),
            energy: vec![8900., 9000.],
            mu: vec![1., 2.],
            ..Default::default()
        };
        let intermediate = derived(input(source.clone(), 10));
        let result = derived(input(intermediate_id.clone(), 14));
        for revision in [10, 11, 10] {
            assert_eq!(
                provenance_changed(
                    &result,
                    &|id| {
                        if id == &source {
                            Some((revision, None))
                        } else if id == &intermediate_id {
                            Some((14, Some(&intermediate)))
                        } else {
                            None
                        }
                    },
                    &mut BTreeSet::new()
                ),
                revision == 11
            );
            assert_eq!(result.mu, vec![1., 2.]);
            assert_eq!(intermediate.mu, vec![1., 2.]);
        }
    }

    #[test]
    fn mixed_batch_validation_is_frozen_and_apply_preflight_is_atomic() {
        let root = std::env::temp_dir().join(format!("rexafs-bulk-repair-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let params = PipelineParams {
            import: ImportConfig {
                mode: DetectionMode::MuColumn,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut targets = Vec::new();
        for i in 0..41 {
            let path = root.join(format!("scan_{i}.dat"));
            let text = match i {
                38 => "# energy mu\n8900 1\n9000 2\n9100 3\n",
                39 => "# energy monitor mu\n8900 1 4\n9000 2 4\n9100 3 4\n",
                _ => "# energy mu monitor\n8900 1 4\n9000 2 4\n9100 3 4\n",
            };
            std::fs::write(&path, text).unwrap();
            let mut target = ToolTarget::standalone(
                Some(GroupId::source(&path, DetectionMode::MuColumn)),
                path.clone(),
                format!("scan {i}"),
                params.fingerprint(),
                1,
                1,
            );
            target.ix = i;
            targets.push(RepairTarget {
                target,
                params: params.clone(),
                locked: i == 40,
                changed: false,
                source: SourceRevision::read(&path),
            });
        }
        let table = crate::params::preview_import(&targets[0].target.path, &params.import).unwrap();
        let mut draft = MappingDraft::new(&table, &params.import);
        draft.set_axis(AxisConversion::EnergyEv);
        let scope = RepairScope {
            label: "Muon batch".into(),
            targets,
        };
        // Arrival of another file after capture must have no effect on membership.
        std::fs::write(
            root.join("late.dat"),
            "# energy mu monitor\n8900 1 4\n9000 2 4\n",
        )
        .unwrap();
        let validated = validate(
            scope,
            LayoutKey::from_preview(&table),
            draft.config().clone(),
            draft.revision,
        );
        assert_eq!(validated.ready_count(), 38);
        assert_eq!(validated.file_count(), 38);
        assert_eq!(validated.scope.targets.len(), 41);
        assert!(
            validated
                .summary()
                .contains("2 incompatible · 1 locked · 0 changed")
        );
        let lookup = |id: &GroupId| {
            validated
                .scope
                .targets
                .iter()
                .find(|t| t.target.group_id.as_ref() == Some(id))
                .map(|t| (t.target.clone(), t.locked))
        };
        assert_eq!(
            preflight(&validated, draft.revision, lookup).unwrap().len(),
            38
        );
        assert!(preflight(&validated, draft.revision + 1, lookup).is_err());
        for change in 0..4 {
            assert!(
                preflight(&validated, draft.revision, |id| {
                    let (mut target, locked) = lookup(id)?;
                    if target.ix != 37 {
                        return Some((target, locked));
                    }
                    match change {
                        0 => None,
                        1 => Some((target, true)),
                        2 => {
                            target.fingerprint += 1;
                            Some((target, false))
                        }
                        _ => {
                            target.group_id = Some(GroupId::new_result());
                            Some((target, false))
                        }
                    }
                })
                .is_err(),
                "change {change} must reject the entire set before mutation"
            );
        }
        // Harmless catalog growth/slot shifts do not authorize new members or
        // invalidate identities already captured in this transaction.
        assert_eq!(
            preflight(&validated, draft.revision, |id| {
                let (mut target, locked) = lookup(id)?;
                target.catalog_generation += 1;
                target.ix += 10;
                Some((target, locked))
            })
            .unwrap()
            .len(),
            38
        );
        std::fs::write(
            &validated.scope.targets[37].target.path,
            "# energy mu monitor\n8900 9 4\n9000 9 4\n",
        )
        .unwrap();
        assert!(preflight(&validated, draft.revision, lookup).is_err());
        let mut changed_scope = validated.scope.clone();
        changed_scope.targets[0].changed = true;
        let revalidated = validate(
            changed_scope,
            LayoutKey::from_preview(&table),
            draft.config().clone(),
            draft.revision,
        );
        assert_eq!(revalidated.results[0], Validation::Changed);
        std::fs::remove_dir_all(root).unwrap();
    }
}
