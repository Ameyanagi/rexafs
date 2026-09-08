//! Journal + undo/redo. Every applied change (parameter edits, tools, fits)
//! appends a one-line entry; parameter edits and derived-group changes carry
//! an inverse so ⌘Z / ⇧⌘Z walk them back. The journal is also the recipe a
//! batch run repeats.

use gpui::{ClickEvent, Context, IntoElement, ParentElement, Styled, div, prelude::*, px};

use super::MONO;
use crate::app::{DERIVED_BASE, ParamKey, StudioApp};
use crate::group_identity::{GroupId, GroupRegistry, GroupState};
use crate::params::{DerivedSpectrum, PipelineParams, Quantity};
use std::collections::BTreeSet;

/// Inverse of a recorded change.
#[allow(clippy::large_enum_variant)]
pub enum UndoOp {
    Label {
        id: GroupId,
        before: Option<String>,
        after: Option<String>,
    },
    Color {
        id: GroupId,
        before: Option<u8>,
        after: Option<u8>,
    },
    SpectrumColors {
        changes: Vec<(
            GroupId,
            Option<crate::spectrum_colors::Assignment>,
            Option<crate::spectrum_colors::Assignment>,
        )>,
    },
    FitModel {
        before: super::assistant_actions::ModelSettings,
        after: super::assistant_actions::ModelSettings,
    },
    Params {
        changes: Vec<(GroupId, Option<PipelineParams>, Option<PipelineParams>)>,
    },
    /// Pipeline parameters of `target` (a group override, or the globals).
    Param {
        target: Option<GroupId>,
        key: Option<ParamKey>,
        before: PipelineParams,
        after: PipelineParams,
    },
    DerivedQuantity {
        id: GroupId,
        before: (Quantity, bool),
        after: (Quantity, bool),
    },
    /// A derived group was created at `index`.
    DerivedAdd {
        index: usize,
        spectrum: DerivedSpectrum,
    },
    GroupRemove {
        snapshot: RemovalSnapshot,
        /// Undoing a creation uses the same transaction, with its direction reversed.
        restore_on_redo: bool,
    },
}

/// Restorable transaction, including source exclusions and independent UI identities.
pub struct RemovalSnapshot {
    ids: BTreeSet<GroupId>,
    derived: Vec<(usize, DerivedSpectrum)>,
    state: GroupState,
    sources: Vec<crate::group_identity::SourceGroup>,
    interaction: crate::app::group_rows::InteractionIds,
}

impl RemovalSnapshot {
    pub(crate) fn take(
        ids: BTreeSet<GroupId>,
        registry: &GroupRegistry,
        derived: &mut Vec<DerivedSpectrum>,
        state: &mut GroupState,
        interaction: crate::app::group_rows::InteractionIds,
    ) -> Self {
        let mut saved = state.clone();
        saved.labels.retain(|id, _| ids.contains(id));
        saved.colors.retain(|id, _| ids.contains(id));
        saved.plot_colors.retain(|id, _| ids.contains(id));
        saved.marked.retain(|id| ids.contains(id));
        saved.frozen.retain(|id| ids.contains(id));
        saved.overrides.retain(|(id, _)| ids.contains(id));
        saved.excluded.clear();
        saved.current = saved.current.filter(|id| ids.contains(id));
        let sources = registry
            .sources()
            .into_iter()
            .filter(|s| ids.contains(&s.id))
            .collect();
        let removed = derived
            .iter()
            .enumerate()
            .filter(|(_, d)| d.group_id.as_ref().is_some_and(|id| ids.contains(id)))
            .map(|(i, d)| (i, d.clone()))
            .collect();
        derived.retain(|d| d.group_id.as_ref().is_none_or(|id| !ids.contains(id)));
        state.labels.retain(|id, _| !ids.contains(id));
        state.colors.retain(|id, _| !ids.contains(id));
        state.plot_colors.retain(|id, _| !ids.contains(id));
        state.marked.retain(|id| !ids.contains(id));
        state.frozen.retain(|id| !ids.contains(id));
        state.overrides.retain(|(id, _)| !ids.contains(id));
        if state.current.as_ref().is_some_and(|id| ids.contains(id)) {
            state.current = None;
        }
        state.excluded.extend(ids.clone());
        registry.set_excluded(&state.excluded);
        registry.replace_derived(derived);
        Self {
            ids,
            derived: removed,
            state: saved,
            sources,
            interaction,
        }
    }

    fn restore_sources(&self, registry: &GroupRegistry, catalog: &mut crate::catalog::Catalog) {
        for source in &self.sources {
            let ix = catalog
                .find_by_canonical_path(&source.path)
                .unwrap_or_else(|| {
                    let ix = catalog.len();
                    catalog.extend(vec![crate::catalog::FileMeta {
                        dir: source
                            .path
                            .parent()
                            .unwrap_or(std::path::Path::new(""))
                            .to_string_lossy()
                            .as_ref()
                            .into(),
                        name: source
                            .path
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .as_ref()
                            .into(),
                        size: 0,
                    }]);
                    ix
                });
            registry.restore_source(source.clone(), ix);
        }
    }

    fn restore(
        &self,
        registry: &GroupRegistry,
        derived: &mut Vec<DerivedSpectrum>,
        state: &mut GroupState,
    ) {
        for (index, group) in &self.derived {
            derived.insert((*index).min(derived.len()), group.clone());
        }
        state.excluded.retain(|id| !self.ids.contains(id));
        state.labels.extend(self.state.labels.clone());
        state.colors.extend(self.state.colors.clone());
        state.plot_colors.extend(self.state.plot_colors.clone());
        state.marked.extend(self.state.marked.clone());
        state.frozen.extend(self.state.frozen.clone());
        state.overrides.extend(self.state.overrides.clone());
        if let Some(current) = &self.state.current {
            state.current = Some(current.clone());
        }
        registry.set_excluded(&state.excluded);
        registry.replace_derived(derived);
    }
}

impl UndoOp {
    fn apply_metadata(&self, state: &mut crate::group_identity::GroupState, forward: bool) {
        fn set<T: Clone>(
            map: &mut std::collections::BTreeMap<GroupId, T>,
            id: &GroupId,
            value: &Option<T>,
        ) {
            if let Some(value) = value {
                map.insert(id.clone(), value.clone());
            } else {
                map.remove(id);
            }
        }
        match self {
            Self::Label { id, before, after } => {
                set(&mut state.labels, id, if forward { after } else { before })
            }
            Self::Color { id, before, after } => {
                set(&mut state.colors, id, if forward { after } else { before })
            }
            Self::SpectrumColors { changes } => {
                for (id, before, after) in changes {
                    set(
                        &mut state.plot_colors,
                        id,
                        if forward { after } else { before },
                    );
                }
            }
            _ => {}
        }
    }

    fn apply_quantity(&self, derived: &mut [DerivedSpectrum], forward: bool) {
        if let Self::DerivedQuantity {
            id, before, after, ..
        } = self
            && let Some(group) = derived.iter_mut().find(|g| {
                g.group_id
                    .as_ref()
                    .cloned()
                    .unwrap_or_else(|| GroupId::legacy_result(g.id))
                    == *id
            })
        {
            (group.quantity, group.quantity_unconfirmed) = if forward { *after } else { *before };
        }
    }

    fn param_snapshot(&self, forward: bool) -> Option<(Option<GroupId>, PipelineParams)> {
        match self {
            Self::Param {
                target,
                before,
                after,
                ..
            } => Some((target.clone(), if forward { after } else { before }.clone())),
            _ => None,
        }
    }
}

pub struct JournalEntry {
    pub text: String,
}

#[derive(Default)]
pub struct JournalState {
    pub entries: Vec<JournalEntry>,
    pub undo: Vec<UndoOp>,
    pub redo: Vec<UndoOp>,
    pub open: bool,
    pub(crate) receipt_revision: u64,
}

const JOURNAL_CAPACITY: usize = 500;

impl JournalState {
    fn changed(&mut self) {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        self.receipt_revision = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    pub(crate) fn take_history(&mut self, redo: bool) -> Option<UndoOp> {
        let op = if redo {
            self.redo.pop()
        } else {
            self.undo.pop()
        }?;
        self.changed();
        Some(op)
    }

    pub(crate) fn confirm_quantity(
        &mut self,
        _index: usize,
        group: &mut DerivedSpectrum,
        quantity: Quantity,
    ) -> bool {
        let before = (group.quantity, group.quantity_unconfirmed);
        if !group.confirm_quantity(quantity) {
            return false;
        }
        self.record(
            format!("Confirm quantity: {}", group.display_label()),
            Some(UndoOp::DerivedQuantity {
                id: group
                    .group_id
                    .clone()
                    .unwrap_or_else(|| GroupId::legacy_result(group.id)),
                before,
                after: (group.quantity, group.quantity_unconfirmed),
            }),
        );
        true
    }

    /// Append a journal line, optionally with its inverse.
    pub(crate) fn record(&mut self, text: impl Into<String>, op: Option<UndoOp>) {
        self.changed();
        let text = text.into();
        self.entries.push(JournalEntry { text });
        if self.entries.len() > JOURNAL_CAPACITY {
            self.entries.remove(0);
        }
        if let Some(op) = op {
            self.undo.push(op);
            self.redo.clear();
        }
    }

    /// Record a parameter edit. Consecutive edits of the same parameter on
    /// the same target (a drag, a stepper burst) collapse into one step.
    pub(crate) fn record_param_edit(
        &mut self,
        target: Option<GroupId>,
        key: Option<ParamKey>,
        before: PipelineParams,
        after: PipelineParams,
        text: String,
    ) {
        if before == after {
            return;
        }
        if let Some(UndoOp::Param {
            target: t,
            key: k,
            after: a,
            ..
        }) = self.undo.last_mut()
            && *t == target
            && key.is_some()
            && *k == key
        {
            *a = after;
            if let Some(last) = self.entries.last_mut() {
                last.text = text;
            }
            self.redo.clear();
            self.changed();
            return;
        }
        self.record(
            text,
            Some(UndoOp::Param {
                target,
                key,
                before,
                after,
            }),
        );
    }
}

impl StudioApp {
    pub(crate) fn record(&mut self, text: impl Into<String>, op: Option<UndoOp>) {
        self.journal.record(text, op);
    }

    pub(crate) fn record_param_edit(
        &mut self,
        target: Option<usize>,
        key: Option<ParamKey>,
        before: PipelineParams,
        after: PipelineParams,
        text: String,
    ) {
        let id = target.and_then(|ix| self.group_id(ix));
        if target.is_some() && id.is_none() {
            return;
        }
        self.journal.record_param_edit(id, key, before, after, text);
    }

    pub(crate) fn apply_params_to(&mut self, target: Option<usize>, params: PipelineParams) {
        match target {
            Some(ix) => {
                self.set_custom_params(ix, (params != self.params).then_some(params));
            }
            None => self.params = params,
        }
    }

    pub(crate) fn remove_groups(
        &mut self,
        ids: BTreeSet<GroupId>,
        text: String,
        cx: &mut Context<Self>,
    ) {
        if let Some(snapshot) = self.take_groups(ids, cx) {
            self.record(
                text,
                Some(UndoOp::GroupRemove {
                    snapshot,
                    restore_on_redo: false,
                }),
            );
            cx.notify();
        }
    }

    fn take_groups(
        &mut self,
        ids: BTreeSet<GroupId>,
        cx: &mut Context<Self>,
    ) -> Option<RemovalSnapshot> {
        let ids: BTreeSet<_> = ids
            .into_iter()
            .filter(|id| self.menu_index(id).is_some())
            .collect();
        if ids.is_empty() {
            return None;
        }
        let before = crate::app::group_rows::build_rows_active(
            &self.catalog,
            &self.derived,
            |_| Some(true),
            None,
            "",
            self.standalone_path(),
            &BTreeSet::new(),
            &self.group_registry.excluded_indices(),
        );
        let current = self.current_group_index();
        let was_current =
            current.is_some_and(|g| self.group_id(g).is_some_and(|id| ids.contains(&id)));
        self.bind_joint_sources();
        let interaction = self.capture_groups();
        let removed_indices: BTreeSet<_> =
            ids.iter().filter_map(|id| self.menu_index(id)).collect();
        let previous_registry = self.group_registry.clone();
        let snapshot = RemovalSnapshot::take(
            ids,
            &self.group_registry,
            &mut self.derived,
            &mut self.group_state,
            interaction.clone(),
        );
        self.invalidate_removal_jobs(
            &snapshot.ids,
            &removed_indices,
            !snapshot.derived.is_empty(),
        );
        self.resolve_group_state();
        self.resolve_interaction(interaction);
        if was_current {
            let after = self.interaction_rows();
            self.selected = current.and_then(|g| {
                crate::app::group_rows::removal_survivor(
                    &before,
                    &after,
                    g,
                    &previous_registry,
                    &self.group_registry,
                    self.standalone_path().is_some(),
                )
            });
            if self.focus_group.is_none() {
                self.focus_group = self.selected;
            }
        }
        self.refresh_removed_groups(cx);
        Some(snapshot)
    }

    fn invalidate_removal_jobs(
        &mut self,
        ids: &BTreeSet<GroupId>,
        indices: &BTreeSet<usize>,
        derived_changed: bool,
    ) {
        use crate::app::retire_job;
        self.tools.invalidate_bindings();
        // Catalog slots do not shift. Only these cache keys became unavailable.
        crate::app::evict_group_keys(&mut self.cache, indices, false);
        crate::app::evict_group_keys(&mut self.raw_cache, indices, false);
        if derived_changed {
            self.invalidate_index_bindings(crate::app::IndexChange::DerivedOnly);
        } else {
            self.generation += 1;
            self.compare_gen += 1;
            self.load_running = false;
            self.compare_running = false;
        }
        let interrupted = crate::app::CatalogBindings {
            generations: [
                &mut self.operando_gen,
                &mut self.batch_gen,
                &mut self.merge_gen,
                &mut self.fit_gen,
                &mut self.lcf_gen,
                &mut self.filter_gen,
            ],
            running: [
                &mut self.operando_running,
                &mut self.batch_running,
                &mut self.merge_running,
                &mut self.fit_running,
                &mut self.lcf_running,
            ],
            cancellations: [
                &mut self.operando_cancel,
                &mut self.batch_cancel,
                &mut self.merge_cancel,
                &mut self.lcf_cancel,
            ],
            filtered: &mut self.filtered,
        }
        .remove(&self.job_inputs, ids);
        crate::app::finish_interrupted_results(
            self.batch_fit.as_mut(),
            self.series_lcf.as_mut(),
            interrupted,
        );
        // A removed unsampled frame also changes sampling and cursor geometry.
        let overview_affected = self.operando.as_ref().is_some_and(|data| {
            self.catalog.scans.get(data.scan).is_some_and(|scan| {
                indices
                    .range(scan.start..scan.start.saturating_add(scan.len))
                    .next()
                    .is_some()
            })
        }) || self
            .active_scan
            .and_then(|i| self.catalog.scans.get(i))
            .is_some_and(|scan| {
                indices
                    .range(scan.start..scan.start.saturating_add(scan.len))
                    .next()
                    .is_some()
            });
        if overview_affected {
            if self.job_inputs[0].is_disjoint(ids) {
                retire_job(
                    &mut self.operando_gen,
                    &mut self.operando_running,
                    Some(&mut self.operando_cancel),
                );
            }
            self.operando = None;
            self.operando_plots = None;
        }
        self.import_preview_gen += 1;
    }

    /// Journal the fresh identity introduced by an explicit re-import.
    pub(crate) fn record_reimport(&mut self, ids: BTreeSet<GroupId>) {
        self.record_created_groups(ids, "Re-import removed file groups".into());
    }

    pub(crate) fn record_created_groups(&mut self, ids: BTreeSet<GroupId>, label: String) {
        let snapshot = RemovalSnapshot::take(
            ids,
            &self.group_registry,
            &mut self.derived,
            &mut self.group_state,
            crate::app::group_rows::InteractionIds::capture([None; 3], |_| None),
        );
        snapshot.restore(
            &self.group_registry,
            &mut self.derived,
            &mut self.group_state,
        );
        self.record(
            label,
            Some(UndoOp::GroupRemove {
                snapshot,
                restore_on_redo: true,
            }),
        );
    }

    fn refresh_removed_groups(&mut self, cx: &mut Context<Self>) {
        if let Some(ix) = self.selected.or_else(|| {
            self.group_state
                .current
                .as_ref()
                .and_then(|id| self.menu_index(id))
        }) {
            let (focus, anchor, reveal) = (self.focus_group, self.mark_anchor, self.reveal_current);
            let expanded = self.expanded_sources.clone();
            self.select_entry(ix, cx);
            self.expanded_sources = expanded;
            (self.focus_group, self.mark_anchor, self.reveal_current) = (focus, anchor, reveal);
        } else {
            self.current_path.clear();
            self.spectrum_path.clear();
            self.spectrum = None;
            self.spectrum_label = "no spectrum".into();
            self.stale_plots = None;
            self.quadrants.clear();
            self.quad_bindings.clear();
            self.import_preview = None;
        }
        self.group_state.current = self.current_group_index().and_then(|g| self.group_id(g));
        self.ensure_compare_loaded(cx);
        self.invalidate_explore_plots(cx);
        self.sync_param_fields(cx);
        if self.workspace == crate::app::Workspace::Operando {
            self.ensure_operando(cx);
        }
    }

    fn replay_removal(
        &mut self,
        snapshot: RemovalSnapshot,
        remove: bool,
        cx: &mut Context<Self>,
    ) -> RemovalSnapshot {
        if remove {
            return self
                .take_groups(snapshot.ids.clone(), cx)
                .unwrap_or(snapshot);
        }
        self.capture_groups();
        snapshot.restore_sources(&self.group_registry, &mut self.catalog);
        snapshot.restore(
            &self.group_registry,
            &mut self.derived,
            &mut self.group_state,
        );
        let indices = self.group_registry.indices(&snapshot.ids);
        self.invalidate_removal_jobs(&snapshot.ids, &indices, !snapshot.derived.is_empty());
        self.resolve_group_state();
        self.resolve_interaction(snapshot.interaction.clone());
        self.refresh_removed_groups(cx);
        snapshot
    }

    pub(crate) fn undo(&mut self, cx: &mut Context<Self>) {
        let Some(op) = self.journal.take_history(false) else {
            self.status = "nothing to undo".into();
            cx.notify();
            return;
        };
        if let Some((target, params)) = op.param_snapshot(false) {
            match target {
                Some(id) => {
                    if let Some(ix) = self.group_registry.index(&id) {
                        self.apply_params_to(Some(ix), params);
                    }
                }
                None => self.apply_params_to(None, params),
            }
            self.after_param_undo(cx);
        }
        let inverse = match op {
            op @ (UndoOp::Label { .. } | UndoOp::Color { .. } | UndoOp::SpectrumColors { .. }) => {
                op.apply_metadata(&mut self.group_state, false);
                self.invalidate_explore_plots(cx);
                op
            }
            op @ UndoOp::DerivedQuantity { .. } => {
                op.apply_quantity(&mut self.derived, false);
                self.after_param_undo(cx);
                op
            }
            UndoOp::FitModel { before, after } => {
                self.restore_model_settings(&before, cx);
                UndoOp::FitModel { before, after }
            }
            op @ UndoOp::Param { .. } => op,
            UndoOp::Params { changes } => {
                for (ix, before, _) in &changes {
                    if let Some(ix) = self.group_registry.index(ix) {
                        self.set_custom_params(ix, before.clone());
                    }
                }
                self.after_param_undo(cx);
                UndoOp::Params { changes }
            }
            UndoOp::DerivedAdd { index, spectrum } => {
                if let Some(snapshot) = spectrum
                    .group_id
                    .as_ref()
                    .and_then(|id| self.take_groups(BTreeSet::from([id.clone()]), cx))
                {
                    UndoOp::GroupRemove {
                        snapshot,
                        restore_on_redo: true,
                    }
                } else {
                    UndoOp::DerivedAdd { index, spectrum }
                }
            }
            UndoOp::GroupRemove {
                snapshot,
                restore_on_redo,
            } => {
                let snapshot = self.replay_removal(snapshot, restore_on_redo, cx);
                UndoOp::GroupRemove {
                    snapshot,
                    restore_on_redo,
                }
            }
        };
        self.journal.redo.push(inverse);
        self.journal.entries.push(JournalEntry {
            text: "undo".into(),
        });
        self.status = "undone".into();
        cx.notify();
    }

    pub(crate) fn redo(&mut self, cx: &mut Context<Self>) {
        let Some(op) = self.journal.take_history(true) else {
            self.status = "nothing to redo".into();
            cx.notify();
            return;
        };
        if let Some((target, params)) = op.param_snapshot(true) {
            match target {
                Some(id) => {
                    if let Some(ix) = self.group_registry.index(&id) {
                        self.apply_params_to(Some(ix), params);
                    }
                }
                None => self.apply_params_to(None, params),
            }
            self.after_param_undo(cx);
        }
        let forward = match op {
            op @ (UndoOp::Label { .. } | UndoOp::Color { .. } | UndoOp::SpectrumColors { .. }) => {
                op.apply_metadata(&mut self.group_state, true);
                self.invalidate_explore_plots(cx);
                op
            }
            op @ UndoOp::DerivedQuantity { .. } => {
                op.apply_quantity(&mut self.derived, true);
                self.after_param_undo(cx);
                op
            }
            UndoOp::FitModel { before, after } => {
                self.restore_model_settings(&after, cx);
                UndoOp::FitModel { before, after }
            }
            op @ UndoOp::Param { .. } => op,
            UndoOp::Params { changes } => {
                for (ix, _, before) in &changes {
                    if let Some(ix) = self.group_registry.index(ix) {
                        self.set_custom_params(ix, before.clone());
                    }
                }
                self.after_param_undo(cx);
                UndoOp::Params { changes }
            }
            op @ UndoOp::DerivedAdd { .. } => op,
            UndoOp::GroupRemove {
                snapshot,
                restore_on_redo,
            } => {
                let snapshot = self.replay_removal(snapshot, !restore_on_redo, cx);
                UndoOp::GroupRemove {
                    snapshot,
                    restore_on_redo,
                }
            }
        };
        self.journal.undo.push(forward);
        self.journal.entries.push(JournalEntry {
            text: "redo".into(),
        });
        self.status = "redone".into();
        cx.notify();
    }

    fn after_param_undo(&mut self, cx: &mut Context<Self>) {
        self.sync_param_fields(cx);
        self.schedule_recompute(cx);
        self.sync_handles(cx);
        self.invalidate_explore_plots(cx);
    }

    /// Bottom strip listing the journal, newest first.
    pub(crate) fn journal_panel(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let t = self.theme;
        let mut list = div()
            .id("journal-list")
            .flex_1()
            .min_h_0()
            .min_w_0()
            .overflow_y_scroll()
            .px_3()
            .py_1();
        for (i, entry) in self.journal.entries.iter().enumerate().rev() {
            list = list.child(
                div()
                    .h(px(20.))
                    .flex()
                    .items_center()
                    .gap_2()
                    .text_size(px(11.))
                    .child(
                        div()
                            .w(px(36.))
                            .font_family(MONO)
                            .text_color(t.text_muted)
                            .child(format!("{}", i + 1)),
                    )
                    .child(div().text_color(t.text).child(entry.text.clone())),
            );
        }
        div()
            .h(px(150.))
            .flex_none()
            .flex()
            .flex_col()
            .bg(t.surface)
            .border_t_1()
            .border_color(t.border)
            .child(
                div()
                    .px_3()
                    .py_1()
                    .flex()
                    .items_center()
                    .gap_2()
                    .text_size(px(11.))
                    .text_color(t.text_muted)
                    .child(super::section_label(&t, "Journal"))
                    .child(format!(
                        "{} steps · {} undoable",
                        self.journal.entries.len(),
                        self.journal.undo.len()
                    ))
                    .child(div().flex_1())
                    .child(
                        div()
                            .id("journal-close")
                            .px_1()
                            .cursor_pointer()
                            .hover(|d| d.text_color(t.text))
                            .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                                this.journal.open = false;
                                cx.notify();
                            }))
                            .child("close"),
                    ),
            )
            .child(list)
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn removal_standalone_undo_restores_membership_marks_and_only_removed_current() {
        use crate::app::group_rows::{InteractionIds, build_rows_active};
        for was_current in [false, true] {
            let registry = GroupRegistry::default();
            let a = registry.register_source(
                None,
                "/data/a.dat".into(),
                DetectionMode::Auto,
                &Default::default(),
            );
            let mut state = GroupState::default();
            state.marked.insert(a.clone());
            state.frozen.insert(a.clone());
            state.current = was_current.then_some(a.clone());
            let mut derived = vec![];
            let snapshot = RemovalSnapshot::take(
                BTreeSet::from([a.clone()]),
                &registry,
                &mut derived,
                &mut state,
                InteractionIds::capture([None; 3], |_| None),
            );
            let mut catalog = crate::catalog::Catalog::default();
            catalog.extend(vec![crate::catalog::FileMeta {
                dir: "/data".into(),
                name: "b.dat".into(),
                size: 0,
            }]);
            let b = registry.register_source(
                Some(0),
                catalog.path(0),
                DetectionMode::Auto,
                &Default::default(),
            );
            state.current = Some(b.clone());
            snapshot.restore_sources(&registry, &mut catalog);
            snapshot.restore(&registry, &mut derived, &mut state);
            assert_eq!(registry.index(&a), Some(1));
            assert_eq!(registry.indices(&state.marked), BTreeSet::from([1]));
            assert_eq!(registry.indices(&state.frozen), BTreeSet::from([1]));
            assert_eq!(state.current, Some(if was_current { a } else { b }));
            let rows = build_rows_active(
                &catalog,
                &derived,
                |_| None,
                None,
                "",
                None,
                &BTreeSet::new(),
                &registry.excluded_indices(),
            );
            assert_eq!(rows.shown().collect::<Vec<_>>(), vec![0, 1]);
        }
    }

    #[test]
    fn removal_undo_preserves_a_new_current_when_removing_another_group() {
        let registry = GroupRegistry::default();
        let ids: Vec<_> = (0..3)
            .map(|ix| {
                registry.register_source(
                    Some(ix),
                    format!("/data/{ix}.dat").into(),
                    DetectionMode::Auto,
                    &Default::default(),
                )
            })
            .collect();
        let mut state = GroupState {
            current: Some(ids[1].clone()),
            ..Default::default()
        };
        let mut derived = vec![];
        let snapshot = RemovalSnapshot::take(
            BTreeSet::from([ids[0].clone()]),
            &registry,
            &mut derived,
            &mut state,
            crate::app::group_rows::InteractionIds::capture([None; 3], |_| None),
        );
        state.current = Some(ids[2].clone());
        snapshot.restore(&registry, &mut derived, &mut state);
        assert_eq!(state.current, Some(ids[2].clone()));
    }

    #[test]
    fn removal_reimport_creation_is_undoable_without_restoring_historical_input() {
        let registry = GroupRegistry::default();
        let path = std::path::PathBuf::from("/data/a.dat");
        let old = registry.register_source(
            Some(0),
            path.clone(),
            DetectionMode::Auto,
            &Default::default(),
        );
        let mut state = GroupState::default();
        let mut derived = vec![];
        let interaction = || crate::app::group_rows::InteractionIds::capture([None; 3], |_| None);
        let removal = RemovalSnapshot::take(
            BTreeSet::from([old.clone()]),
            &registry,
            &mut derived,
            &mut state,
            interaction(),
        );
        let fresh = registry.reimport_source(&path, Some(0)).unwrap();
        let reference = GroupId::new_result();
        derived.push(DerivedSpectrum {
            group_id: Some(reference.clone()),
            source: Some(path),
            ..Default::default()
        });
        let fresh_ids = BTreeSet::from([fresh.clone(), reference]);
        let creation = RemovalSnapshot::take(
            fresh_ids.clone(),
            &registry,
            &mut derived,
            &mut state,
            interaction(),
        );
        creation.restore(&registry, &mut derived, &mut state);
        assert_eq!(registry.id(0), Some(fresh.clone()));
        assert!(registry.is_excluded(&old));
        let undo_creation = RemovalSnapshot::take(
            fresh_ids.clone(),
            &registry,
            &mut derived,
            &mut state,
            interaction(),
        );
        assert_eq!(registry.id(0), None);
        assert!(derived.is_empty());
        undo_creation.restore(&registry, &mut derived, &mut state);
        assert_eq!(derived.len(), 1);
        assert_eq!(registry.id(0), Some(fresh.clone()));
        RemovalSnapshot::take(
            fresh_ids,
            &registry,
            &mut derived,
            &mut state,
            interaction(),
        );
        let mut catalog = crate::catalog::Catalog::default();
        removal.restore_sources(&registry, &mut catalog);
        removal.restore(&registry, &mut derived, &mut state);
        assert_eq!(registry.id(0), Some(old));
    }

    use super::*;
    use crate::params::DetectionMode;

    #[test]
    fn removal_derived_input_keeps_operation_when_its_slot_is_reused() {
        use crate::app::group_rows::{InteractionIds, input_missing};
        use crate::params::{Operation, OperationInput};
        let input = GroupId::new_result();
        let result = GroupId::new_result();
        let operation = Operation {
            tool: "merge".into(),
            parameters: serde_json::Value::Null,
            inputs: vec![OperationInput {
                group_id: Some(input.clone()),
                label: "Aligned Cu".into(),
                path: Default::default(),
                derived_id: Some(1),
                fingerprint: 0,
                size: None,
            }],
            applied_energy_shift_ev: 0.,
        };
        let mut derived = vec![
            DerivedSpectrum {
                group_id: Some(input.clone()),
                id: 1,
                ..Default::default()
            },
            DerivedSpectrum {
                group_id: Some(result.clone()),
                operation: Some(operation.clone()),
                ..Default::default()
            },
        ];
        let registry = GroupRegistry::default();
        registry.replace_derived(&derived);
        let mut state = GroupState::default();
        state.labels.insert(input.clone(), "renamed copy".into());
        state.colors.insert(input.clone(), 6);
        state.marked.insert(input.clone());
        state.frozen.insert(input.clone());
        let snapshot = RemovalSnapshot::take(
            BTreeSet::from([input.clone()]),
            &registry,
            &mut derived,
            &mut state,
            InteractionIds::capture([Some(DERIVED_BASE); 3], |i| registry.id(i)),
        );
        assert_eq!(registry.id(DERIVED_BASE), Some(result));
        assert_eq!(registry.index(&input), None);
        assert_eq!(
            input_missing(&derived[0], |id| registry.is_excluded(id)).as_deref(),
            Some("input missing: Aligned Cu")
        );
        assert_eq!(derived[0].operation, Some(operation.clone()));
        snapshot.restore(&registry, &mut derived, &mut state);
        assert_eq!(state.labels[&input], "renamed copy");
        assert_eq!(state.colors[&input], 6);
        assert!(state.marked.contains(&input) && state.frozen.contains(&input));
        assert_eq!(registry.id(DERIVED_BASE), Some(input));
        assert_eq!(derived[1].operation, Some(operation));
        assert!(input_missing(&derived[1], |id| registry.is_excluded(id)).is_none());
    }

    #[test]
    fn removal_promotes_marked_siblings_and_restores_metadata_and_identity() {
        use crate::app::group_rows::{InteractionIds, Row, build_rows_active, input_missing};
        use crate::params::{DetectionMode, Operation, OperationInput};
        use std::collections::BTreeMap;
        let mut catalog = crate::catalog::Catalog::default();
        catalog.extend(
            ["a.dat", "b.dat"]
                .map(|name| crate::catalog::FileMeta {
                    dir: "/data".into(),
                    name: name.into(),
                    size: 1,
                })
                .into(),
        );
        let mut derived: Vec<_> = [DetectionMode::Fluorescence, DetectionMode::Reference]
            .into_iter()
            .map(|mode| {
                let mut params = PipelineParams::default();
                params.import.mode = mode;
                DerivedSpectrum {
                    label: mode.label().into(),
                    source: Some(catalog.path(0)),
                    params: Some(params),
                    ..Default::default()
                }
            })
            .collect();
        let registry = GroupRegistry::rebuild(
            (0..2).map(|i| (i, catalog.path(i), DetectionMode::Auto)),
            &mut derived,
            &mut vec![],
            &BTreeMap::new(),
        );
        let primary = registry.id(0).unwrap();
        let fluo = registry.id(DERIVED_BASE).unwrap();
        let reference = registry.id(DERIVED_BASE + 1).unwrap();
        let params = PipelineParams {
            e0: Some(8979.),
            ..Default::default()
        };
        let mut state = GroupState {
            labels: BTreeMap::from([(primary.clone(), "Cu foil".into())]),
            colors: BTreeMap::from([(primary.clone(), 7)]),
            marked: BTreeSet::from([primary.clone(), fluo.clone(), reference.clone()]),
            frozen: BTreeSet::from([primary.clone()]),
            overrides: vec![(primary.clone(), params.clone())],
            current: Some(primary.clone()),
            ..Default::default()
        };
        let operation = Operation {
            tool: "merge".into(),
            parameters: serde_json::Value::Null,
            inputs: vec![OperationInput {
                group_id: Some(primary.clone()),
                label: "Cu foil".into(),
                path: catalog.path(0),
                derived_id: None,
                fingerprint: 10,
                size: Some(1),
            }],
            applied_energy_shift_ev: 0.,
        };
        derived.push(DerivedSpectrum {
            group_id: Some(GroupId::new_result()),
            operation: Some(operation.clone()),
            energy: vec![1., 2.],
            mu: vec![3., 4.],
            ..Default::default()
        });
        registry.replace_derived(&derived);
        for ids in [
            BTreeSet::from([primary.clone()]),
            BTreeSet::from([primary.clone(), fluo.clone(), reference.clone()]),
        ] {
            let interaction = InteractionIds::capture(
                [Some(0), Some(DERIVED_BASE + 1), Some(DERIVED_BASE)],
                |i| registry.id(i),
            );
            let snapshot = RemovalSnapshot::take(
                ids.clone(),
                &registry,
                &mut derived,
                &mut state,
                interaction,
            );
            assert_eq!(state.excluded, ids);
            assert!(registry.id(0).is_none());
            assert!(registry.index(&primary).is_none());
            assert!(
                state.labels.is_empty()
                    && state.colors.is_empty()
                    && state.frozen.is_empty()
                    && state.overrides.is_empty()
            );
            let rows = build_rows_active(
                &catalog,
                &derived,
                |_| Some(true),
                None,
                "",
                None,
                &BTreeSet::new(),
                &registry.excluded_indices(),
            );
            if ids.len() == 1 {
                assert_eq!(
                    rows.row_at(0),
                    Some(Row::Primary {
                        group: DERIVED_BASE,
                        expanded: true,
                        extra_channels: 1
                    })
                );
                assert_eq!(
                    rows.row_at(1),
                    Some(Row::Child {
                        group: DERIVED_BASE + 1,
                        parent: DERIVED_BASE
                    })
                );
                assert_eq!(
                    registry.indices(&state.marked),
                    BTreeSet::from([DERIVED_BASE, DERIVED_BASE + 1])
                );
                assert_eq!(registry.id(DERIVED_BASE + 1), Some(reference.clone()));
            } else {
                assert_eq!(rows.shown().collect::<Vec<_>>(), vec![1, DERIVED_BASE]);
                assert!(state.marked.is_empty());
                assert_eq!(
                    registry.index(&reference),
                    None,
                    "a shifted result cannot become the removed input"
                );
            }
            let result = derived.last().unwrap();
            assert_eq!(
                input_missing(result, |id| registry.is_excluded(id)).as_deref(),
                Some("input missing: Cu foil")
            );
            assert_eq!(result.operation, Some(operation.clone()));
            assert_eq!(result.mu, vec![3., 4.]);
            snapshot.restore(&registry, &mut derived, &mut state);
            assert_eq!(
                registry.indices(&state.marked),
                BTreeSet::from([0, DERIVED_BASE, DERIVED_BASE + 1])
            );
            assert_eq!(state.labels[&primary], "Cu foil");
            assert_eq!(state.colors[&primary], 7);
            assert_eq!(state.frozen, BTreeSet::from([primary.clone()]));
            assert!(state.resolved_overrides(&registry)[&0] == params);
            assert_eq!(state.current, Some(primary.clone()));
            assert_eq!(
                snapshot.interaction.resolve(|id| registry.index(id)),
                [Some(0), Some(DERIVED_BASE + 1), Some(DERIVED_BASE)]
            );
            assert!(
                input_missing(derived.last().unwrap(), |id| registry.is_excluded(id)).is_none()
            );
            let rows = build_rows_active(
                &catalog,
                &derived,
                |_| Some(true),
                None,
                "",
                None,
                &BTreeSet::new(),
                &registry.excluded_indices(),
            );
            assert_eq!(
                rows.row_at(0),
                Some(Row::Primary {
                    group: 0,
                    expanded: true,
                    extra_channels: 2
                })
            );
        }
    }

    #[test]
    fn removal_catalog_rescan_cannot_resurrect_an_exclusion_or_retarget_history() {
        use crate::params::DetectionMode;
        use std::collections::BTreeMap;
        let mut catalog = crate::catalog::Catalog::default();
        catalog.extend(vec![crate::catalog::FileMeta {
            dir: "/data".into(),
            name: "a.dat".into(),
            size: 0,
        }]);
        let registry = GroupRegistry::rebuild(
            [(0, catalog.path(0), DetectionMode::Auto)],
            &mut [],
            &mut vec![],
            &BTreeMap::new(),
        );
        let id = registry.id(0).unwrap();
        let prepared = GroupRegistry::prepare_catalog(&catalog, &registry.sources());
        let mut state = GroupState::default();
        let mut derived = vec![];
        let snapshot = RemovalSnapshot::take(
            BTreeSet::from([id.clone()]),
            &registry,
            &mut derived,
            &mut state,
            crate::app::group_rows::InteractionIds::capture([None; 3], |_| None),
        );
        registry.replace_catalog(prepared); // prepared before removal, delivered afterwards
        registry.append_catalog(&catalog, 0);
        registry.register_source(
            Some(0),
            catalog.path(0),
            DetectionMode::Auto,
            &BTreeMap::new(),
        );
        assert!(registry.id(0).is_none());
        assert_eq!(registry.excluded_indices(), BTreeSet::from([0]));
        let mut changed = crate::catalog::Catalog::default();
        changed.extend(
            ["0.dat", "a.dat"]
                .map(|name| crate::catalog::FileMeta {
                    dir: "/data".into(),
                    name: name.into(),
                    size: 0,
                })
                .into(),
        );
        registry.replace_catalog(GroupRegistry::prepare_catalog(
            &changed,
            &registry.sources(),
        ));
        snapshot.restore(&registry, &mut derived, &mut state);
        assert_eq!(registry.index(&id), Some(1));
        assert!(registry.id(0).is_none());
    }

    #[test]
    fn palette_assignments_persist_and_undo_as_one_identity_transaction() {
        use crate::spectrum_colors::{Assignment, Palette};
        let first = GroupId::legacy_result(8);
        let second = GroupId::legacy_result(9);
        let untouched = GroupId::legacy_result(10);
        let before = Assignment {
            palette: Palette::Tab10,
            index: 3,
            count: 4,
            reversed: false,
        };
        let mut state = GroupState {
            current: Some(second.clone()),
            marked: [first.clone()].into(),
            ..Default::default()
        };
        state.plot_colors.insert(first.clone(), before);
        state.plot_colors.insert(untouched.clone(), before);
        let op = UndoOp::SpectrumColors {
            changes: vec![
                (
                    first.clone(),
                    Some(before),
                    Some(Assignment {
                        palette: Palette::Viridis,
                        index: 0,
                        count: 2,
                        reversed: true,
                    }),
                ),
                (
                    second.clone(),
                    None,
                    Some(Assignment {
                        palette: Palette::Viridis,
                        index: 1,
                        count: 2,
                        reversed: true,
                    }),
                ),
            ],
        };
        op.apply_metadata(&mut state, true);
        let project = crate::project::ProjectFile {
            group_state: state,
            ..Default::default()
        };
        let encoded = serde_json::to_vec(&project).unwrap();
        let restored: crate::project::ProjectFile = serde_json::from_slice(&encoded).unwrap();
        let mut state = restored.group_state;
        assert_eq!(state.plot_colors[&first].palette, Palette::Viridis);
        assert_eq!(state.plot_colors[&second].index, 1);
        op.apply_metadata(&mut state, false);
        assert_eq!(state.plot_colors[&first], before);
        assert!(!state.plot_colors.contains_key(&second));
        op.apply_metadata(&mut state, true);
        assert_eq!(state.plot_colors[&first].index, 0);
        assert_eq!(state.plot_colors[&second].index, 1);
        assert_eq!(state.plot_colors[&untouched], before);
        assert_eq!(state.current, Some(second));
        assert_eq!(state.marked, [first].into());
        let legacy: GroupState = serde_json::from_str("{}").unwrap();
        assert!(legacy.plot_colors.is_empty());
    }

    #[test]
    fn metadata_undo_redo_is_identity_based_and_preserves_current_marks() {
        use crate::group_identity::GroupState;
        let id = GroupId::legacy_result(5);
        let mut state = GroupState {
            current: Some(id.clone()),
            marked: [id.clone()].into(),
            ..Default::default()
        };
        let mut journal = JournalState::default();
        for op in [
            UndoOp::Label {
                id: id.clone(),
                before: None,
                after: Some("Cu foil".into()),
            },
            UndoOp::Color {
                id: id.clone(),
                before: None,
                after: Some(7),
            },
        ] {
            op.apply_metadata(&mut state, true);
            journal.record("metadata", Some(op));
        }
        assert_eq!(state.labels[&id], "Cu foil");
        assert_eq!(state.colors[&id], 7);
        for _ in 0..2 {
            let op = journal.undo.pop().unwrap();
            op.apply_metadata(&mut state, false);
            journal.redo.push(op);
        }
        assert!(state.labels.is_empty() && state.colors.is_empty());
        for _ in 0..2 {
            let op = journal.redo.pop().unwrap();
            op.apply_metadata(&mut state, true);
            journal.undo.push(op);
        }
        assert_eq!(state.labels[&id], "Cu foil");
        assert_eq!(state.colors[&id], 7);
        assert_eq!(state.current, Some(id.clone()));
        assert_eq!(state.marked, [id].into());
    }

    #[test]
    fn quantity_confirmation_undo_redo_and_reconfirmation_follow_identity() {
        let params = PipelineParams::default();
        let mut groups = vec![DerivedSpectrum {
            id: 7,
            quantity_unconfirmed: true,
            ..Default::default()
        }];
        let original = groups[0].fingerprint(&params);
        let mut journal = JournalState::default();
        journal.record("earlier edit", Some(UndoOp::Params { changes: vec![] }));
        assert!(journal.confirm_quantity(0, &mut groups[0], Quantity::ChiK));
        let confirmed = groups[0].fingerprint(&params);
        assert_ne!(original, confirmed);
        assert_eq!(journal.undo.len(), 2);
        let op = journal.undo.pop().unwrap();
        op.apply_quantity(&mut groups, false);
        journal.redo.push(op);
        assert_eq!(groups[0].fingerprint(&params), original);
        assert!(groups[0].quantity_unconfirmed);
        assert!(matches!(journal.undo.last(), Some(UndoOp::Params { .. })));

        // Re-key while the confirmation sits in redo, then restore by identity.
        groups.insert(
            0,
            DerivedSpectrum {
                id: 8,
                ..Default::default()
            },
        );
        let op = journal.redo.pop().unwrap();
        op.apply_quantity(&mut groups, true);
        journal.undo.push(op);
        assert_eq!(groups[1].fingerprint(&params), confirmed);
        assert_eq!(groups[0].quantity, Quantity::RawMu);

        // Removing and reinserting this group must not retarget its history.
        let saved = groups.remove(1);
        groups.insert(0, saved);
        assert!(journal.confirm_quantity(0, &mut groups[0], Quantity::RawMu));
        assert!(groups[0].processing_block_reason().is_none());
        assert!(!journal.confirm_quantity(0, &mut groups[0], Quantity::RawMu));
        let correction = journal.undo.pop().unwrap();
        correction.apply_quantity(&mut groups, false);
        assert_eq!(groups[0].fingerprint(&params), confirmed);
        correction.apply_quantity(&mut groups, true);
        assert_eq!(groups[0].quantity, Quantity::RawMu);
        assert!(!groups[0].quantity_unconfirmed);
        let confirmation = journal.undo.pop().unwrap();
        confirmation.apply_quantity(&mut groups, false);
        assert_eq!(groups[0].fingerprint(&params), original);
    }

    #[test]
    fn group_identity_history_resolves_after_reordering_and_missing_targets() {
        use crate::group_identity::GroupRegistry;
        let mut sources = Vec::new();
        let build = |paths: &[&str], sources: &mut Vec<_>| {
            GroupRegistry::rebuild(
                paths
                    .iter()
                    .enumerate()
                    .map(|(ix, p)| (ix, (*p).into(), DetectionMode::Auto)),
                &mut [],
                sources,
                &Default::default(),
            )
        };
        let old = build(&["/a", "/b"], &mut sources);
        let id = old.id(1).unwrap().clone();
        let mut journal = JournalState::default();
        let before = PipelineParams::default();
        let after = PipelineParams {
            e0: Some(42.),
            ..before.clone()
        };
        journal.record_param_edit(Some(id.clone()), None, before, after, "edit B".into());
        let missing = build(&["/c", "/a"], &mut sources);
        let op = journal.undo.pop().unwrap();
        assert!(
            missing
                .index(op.param_snapshot(false).unwrap().0.as_ref().unwrap())
                .is_none()
        );
        journal.redo.push(op);
        let restored = build(&["/b", "/c", "/a"], &mut sources);
        let op = journal.redo.pop().unwrap();
        let (target, params) = op.param_snapshot(true).unwrap();
        assert_eq!(restored.index(target.as_ref().unwrap()), Some(0));
        assert_eq!(params.e0, Some(42.));
        let bulk = UndoOp::Params {
            changes: vec![(id, None, Some(params))],
        };
        let UndoOp::Params { changes } = bulk else {
            unreachable!()
        };
        assert_eq!(restored.index(&changes[0].0), Some(0));
    }

    #[test]
    fn mapping_column_steps_coalesce_and_alignment_round_trips() {
        for target in [
            Some(GroupId::source(
                std::path::Path::new("/sample"),
                DetectionMode::Auto,
            )),
            Some(GroupId::legacy_result(1)),
            None,
        ] {
            let mut journal = JournalState::default();
            let mut initial = PipelineParams::default();
            initial.import.i0_col = Some(1);
            let mut before = initial.clone();
            for column in 2..=6 {
                let mut after = before.clone();
                after.import.i0_col = Some(column);
                journal.record_param_edit(
                    target.clone(),
                    Some(ParamKey::ImpI0Col),
                    before,
                    after.clone(),
                    format!("I0 = {column}"),
                );
                before = after;
            }
            assert_eq!(journal.entries.len(), 1);
            assert_eq!(journal.entries[0].text, "I0 = 6");
            assert_eq!(journal.undo.len(), 1);
            let op = journal.undo.last().unwrap();
            assert!(op.param_snapshot(false).unwrap().1 == initial);
            assert!(op.param_snapshot(true).unwrap().1 == before);
            let after = crate::app::prepare_parameter_edit(&before, |p| {
                p.align_to_ref = !p.align_to_ref;
                Ok(())
            })
            .unwrap()
            .unwrap();
            journal.record_param_edit(
                target.clone(),
                None,
                before.clone(),
                after.clone(),
                "Toggle reference alignment".into(),
            );
            assert_eq!(journal.undo.len(), 2);
            let op = journal.undo.last().unwrap();
            assert!(op.param_snapshot(false).unwrap().1 == before);
            assert!(op.param_snapshot(true).unwrap().1 == after);
        }
    }

    #[test]
    fn mapping_journal_round_trip_keeps_catalog_channel_and_global_targets() {
        for target in [
            Some(GroupId::source(
                std::path::Path::new("/sample"),
                DetectionMode::Auto,
            )),
            Some(GroupId::legacy_result(1)),
            None,
        ] {
            let mut journal = JournalState::default();
            let mut before = PipelineParams::default();
            before.import.mode = DetectionMode::Reference;
            let mut after = before.clone();
            after.import.ir_col = Some(7);
            journal.record_param_edit(
                target.clone(),
                None,
                before.clone(),
                after.clone(),
                "Reference mapping".into(),
            );
            assert_eq!(journal.entries[0].text, "Reference mapping");
            let op = journal.undo.pop().unwrap();
            let (restored_target, restored) = op.param_snapshot(false).unwrap();
            assert_eq!(restored_target, target);
            assert!(restored == before);
            journal.redo.push(op);
            let op = journal.redo.pop().unwrap();
            let (restored_target, restored) = op.param_snapshot(true).unwrap();
            assert_eq!(restored_target, target);
            assert!(restored == after);
            journal.undo.push(op);
            // Separate mapping commands do not coalesce; a new edit retires redo.
            journal.record_param_edit(
                target.clone(),
                None,
                after.clone(),
                before.clone(),
                "Reset mapping".into(),
            );
            assert_eq!(journal.undo.len(), 2);
            journal.redo.push(journal.undo.pop().unwrap());
            journal.record_param_edit(
                target.clone(),
                None,
                after.clone(),
                after.clone(),
                "No change".into(),
            );
            assert_eq!(journal.redo.len(), 1);
            journal.record_param_edit(target.clone(), None, after, before, "New mapping".into());
            assert!(journal.redo.is_empty());
        }
    }
}
