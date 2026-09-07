//! Journal + undo/redo. Every applied change (parameter edits, tools, fits)
//! appends a one-line entry; parameter edits and derived-group changes carry
//! an inverse so ⌘Z / ⇧⌘Z walk them back. The journal is also the recipe a
//! batch run repeats.

use gpui::{ClickEvent, Context, IntoElement, ParentElement, Styled, div, prelude::*, px};

use super::MONO;
use crate::app::{DERIVED_BASE, ParamKey, StudioApp};
use crate::params::{DerivedSpectrum, PipelineParams, Quantity};

/// Inverse of a recorded change.
#[allow(clippy::large_enum_variant)]
pub enum UndoOp {
    FitModel {
        before: super::assistant_actions::ModelSettings,
        after: super::assistant_actions::ModelSettings,
    },
    Params {
        changes: Vec<(usize, Option<PipelineParams>, Option<PipelineParams>)>,
    },
    /// Pipeline parameters of `target` (a group override, or the globals).
    Param {
        target: Option<usize>,
        key: Option<ParamKey>,
        before: PipelineParams,
        after: PipelineParams,
    },
    DerivedQuantity {
        index: usize,
        id: u64,
        before: (Quantity, bool),
        after: (Quantity, bool),
    },
    /// A derived group was created at `index`.
    DerivedAdd {
        index: usize,
        spectrum: DerivedSpectrum,
    },
    /// A derived group was removed from `index`.
    DerivedRemove {
        index: usize,
        spectrum: DerivedSpectrum,
    },
}

impl UndoOp {
    fn remap_quantity_index(&mut self, derived: &[DerivedSpectrum]) {
        if let Self::DerivedQuantity { index, id, .. } = self
            && let Some(current) = derived.iter().position(|g| g.id == *id)
        {
            *index = current;
        }
    }

    fn apply_quantity(&self, derived: &mut [DerivedSpectrum], forward: bool) {
        if let Self::DerivedQuantity {
            index,
            id,
            before,
            after,
        } = self
            && let Some(group) = derived.get_mut(*index).filter(|g| g.id == *id)
        {
            (group.quantity, group.quantity_unconfirmed) = if forward { *after } else { *before };
        }
    }

    fn param_snapshot(&self, forward: bool) -> Option<(Option<usize>, PipelineParams)> {
        match self {
            Self::Param {
                target,
                before,
                after,
                ..
            } => Some((*target, if forward { after } else { before }.clone())),
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
}

const JOURNAL_CAPACITY: usize = 500;

impl JournalState {
    pub(crate) fn confirm_quantity(
        &mut self,
        index: usize,
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
                index,
                id: group.id,
                before,
                after: (group.quantity, group.quantity_unconfirmed),
            }),
        );
        true
    }

    /// Append a journal line, optionally with its inverse.
    pub(crate) fn record(&mut self, text: impl Into<String>, op: Option<UndoOp>) {
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
        target: Option<usize>,
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
        self.journal
            .record_param_edit(target, key, before, after, text);
    }

    pub(crate) fn apply_params_to(&mut self, target: Option<usize>, params: PipelineParams) {
        match target {
            Some(ix) => {
                self.set_custom_params(ix, (params != self.params).then_some(params));
            }
            None => self.params = params,
        }
    }

    fn remap_derived_indices(&mut self, index: usize, insert: bool) {
        let map = |ix: usize| {
            if ix < DERIVED_BASE || ix - DERIVED_BASE < index {
                Some(ix)
            } else if insert {
                Some(ix + 1)
            } else if ix - DERIVED_BASE == index {
                None
            } else {
                Some(ix - 1)
            }
        };
        for op in self.journal.undo.iter_mut().chain(&mut self.journal.redo) {
            op.remap_quantity_index(&self.derived);
        }
        self.selection = self.selection.iter().copied().filter_map(map).collect();
        self.frozen = self.frozen.iter().copied().filter_map(map).collect();
        self.selected = self.selected.and_then(map);
        // An in-flight result carries its old vector index. Retire it before
        // inserting/removing groups so it cannot populate another group's cache.
        self.generation += 1;
        self.compare_gen += 1;
        self.load_running = false;
        self.compare_running = false;
        self.cache.clear();
        self.raw_cache.clear();
        self.thumbs = None;
    }
    fn insert_derived(&mut self, index: usize, spectrum: DerivedSpectrum, cx: &mut Context<Self>) {
        let index = index.min(self.derived.len());
        self.derived.insert(index, spectrum);
        self.remap_derived_indices(index, true);
        self.select_entry(DERIVED_BASE + index, cx);
        self.sync_param_fields(cx);
    }

    pub(crate) fn take_derived(
        &mut self,
        index: usize,
        cx: &mut Context<Self>,
    ) -> Option<DerivedSpectrum> {
        if index >= self.derived.len() {
            return None;
        }
        let was_active = self.selected == Some(DERIVED_BASE + index);
        let spectrum = self.derived.remove(index);
        self.remap_derived_indices(index, false);
        if was_active {
            self.current_path.clear();
            self.spectrum_path.clear();
            self.spectrum = None;
            self.spectrum_label = "no spectrum".into();
            self.stale_plots = None;
            self.quadrants.clear();
            self.quad_bindings.clear();
            self.import_preview = None;
            self.import_preview_gen += 1;
        }
        if let Some(ix) = self.selected {
            self.select_entry(ix, cx);
        } else if !self.catalog.is_empty() {
            self.select_entry(0, cx);
        } else if !self.derived.is_empty() {
            self.select_entry(DERIVED_BASE, cx);
        }
        self.ensure_compare_loaded(cx);
        self.invalidate_explore_plots(cx);
        self.sync_param_fields(cx);
        Some(spectrum)
    }

    pub(crate) fn undo(&mut self, cx: &mut Context<Self>) {
        let Some(op) = self.journal.undo.pop() else {
            self.status = "nothing to undo".into();
            cx.notify();
            return;
        };
        if let Some((target, params)) = op.param_snapshot(false) {
            self.apply_params_to(target, params);
            self.after_param_undo(cx);
        }
        let inverse = match op {
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
                    self.set_custom_params(*ix, before.clone());
                }
                self.after_param_undo(cx);
                UndoOp::Params { changes }
            }
            UndoOp::DerivedAdd { index, spectrum } => {
                let spectrum = self.take_derived(index, cx).unwrap_or(spectrum);
                UndoOp::DerivedAdd { index, spectrum }
            }
            UndoOp::DerivedRemove { index, spectrum } => {
                self.insert_derived(index, spectrum.clone(), cx);
                UndoOp::DerivedRemove { index, spectrum }
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
        let Some(op) = self.journal.redo.pop() else {
            self.status = "nothing to redo".into();
            cx.notify();
            return;
        };
        if let Some((target, params)) = op.param_snapshot(true) {
            self.apply_params_to(target, params);
            self.after_param_undo(cx);
        }
        let forward = match op {
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
                    self.set_custom_params(*ix, before.clone());
                }
                self.after_param_undo(cx);
                UndoOp::Params { changes }
            }
            UndoOp::DerivedAdd { index, spectrum } => {
                self.insert_derived(index, spectrum.clone(), cx);
                UndoOp::DerivedAdd { index, spectrum }
            }
            UndoOp::DerivedRemove { index, spectrum } => {
                let spectrum = self.take_derived(index, cx).unwrap_or(spectrum);
                UndoOp::DerivedRemove { index, spectrum }
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
    use super::*;
    use crate::params::DetectionMode;

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
        for op in &mut journal.redo {
            op.remap_quantity_index(&groups);
        }
        let op = journal.redo.pop().unwrap();
        op.apply_quantity(&mut groups, true);
        journal.undo.push(op);
        assert_eq!(groups[1].fingerprint(&params), confirmed);
        assert_eq!(groups[0].quantity, Quantity::RawMu);

        // Removing and reinserting this group must not retarget its history.
        let saved = groups.remove(1);
        for op in &mut journal.undo {
            op.remap_quantity_index(&groups);
        }
        groups.insert(0, saved);
        for op in &mut journal.undo {
            op.remap_quantity_index(&groups);
        }
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
    fn mapping_column_steps_coalesce_and_alignment_round_trips() {
        for target in [Some(0), Some(DERIVED_BASE), None] {
            let mut journal = JournalState::default();
            let mut initial = PipelineParams::default();
            initial.import.i0_col = Some(1);
            let mut before = initial.clone();
            for column in 2..=6 {
                let mut after = before.clone();
                after.import.i0_col = Some(column);
                journal.record_param_edit(
                    target,
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
                target,
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
        for target in [Some(0), Some(DERIVED_BASE), None] {
            let mut journal = JournalState::default();
            let mut before = PipelineParams::default();
            before.import.mode = DetectionMode::Reference;
            let mut after = before.clone();
            after.import.ir_col = Some(7);
            journal.record_param_edit(
                target,
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
                target,
                None,
                after.clone(),
                before.clone(),
                "Reset mapping".into(),
            );
            assert_eq!(journal.undo.len(), 2);
            journal.redo.push(journal.undo.pop().unwrap());
            journal.record_param_edit(
                target,
                None,
                after.clone(),
                after.clone(),
                "No change".into(),
            );
            assert_eq!(journal.redo.len(), 1);
            journal.record_param_edit(target, None, after, before, "New mapping".into());
            assert!(journal.redo.is_empty());
        }
    }
}
