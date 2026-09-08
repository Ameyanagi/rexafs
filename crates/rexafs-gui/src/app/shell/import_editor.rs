//! A modal owns a draft and immutable target; focus/current/marks are independent.
use std::collections::HashMap;

use gpui::{
    AppContext, ClickEvent, Context, Entity, FocusHandle, Focusable, IntoElement, ParentElement,
    Render, Styled, WeakEntity, Window, div, prelude::*, px,
};
use ruviz::prelude::Plot;
use ruviz_gpui::{RuvizPlot, plot_builder};

use super::tools::ToolTarget;

fn check_target(
    expected: &ToolTarget,
    current: Option<&ToolTarget>,
    locked: bool,
) -> Result<(), String> {
    if current != Some(expected) {
        return Err("The target changed; close and reopen its mapping editor.".into());
    }
    if locked {
        return Err("Processing is now locked.".into());
    }
    Ok(())
}
use crate::app::{
    NO_ENTRY, StudioApp,
    import_channels::{self, ChannelScope},
    import_preview::{self, PreviewKey, PreviewState, SourceRevision},
    import_repair::{self, RepairScope, RepairValidation},
    import_review::{self, ReviewChoice, ReviewScope},
};
use crate::import_mapping::{AxisConversion, ColumnRole, LayoutKey, MappingDraft};
use crate::params::{DetectionMode, PipelineParams};
use crate::theme::Theme;
use crate::widgets::text_input::{InputEvent, NextField, PrevField, TextInput};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum Action {
    Close,
    Cancel,
    Reset,
    Apply,
    Reload,
    Axis(u8),
    Role(ColumnRole),
    Pick(usize),
    Roi(usize),
    ReferenceRatio,
    ThisGroup,
    Batch,
    Validate,
    Targets,
    Details,
    ReviewOutput(u8),
    ReviewPrimary(u8),
    ReviewEdit(u8),
    ReviewFile(bool),
    ReviewLayout(bool),
    ReviewLocate,
}

pub(crate) struct ImportEditor {
    studio: WeakEntity<StudioApp>,
    target: ToolTarget,
    params: PipelineParams,
    theme: Theme,
    opener: Option<FocusHandle>,
    focus: FocusHandle,
    controls: HashMap<Action, FocusHandle>,
    visible_focus: Vec<FocusHandle>,
    locked: bool,
    draft: Option<MappingDraft>,
    table: Option<crate::params::ImportPreview>,
    preview: PreviewState,
    plot: Option<Entity<RuvizPlot>>,
    error: Option<String>,
    source_revision: Option<SourceRevision>,
    generation: u64,
    open_role: Option<ColumnRole>,
    spacing: Entity<TextInput>,
    bulk_scope: Option<RepairScope>,
    validation: Option<RepairValidation>,
    validation_generation: u64,
    validating: bool,
    show_targets: bool,
    show_details: bool,
    batch_available: bool,
    creation: Option<DetectionMode>,
    channel_scope: Option<ChannelScope>,
    batch_selected: bool,
    review_scope: Option<ReviewScope>,
    review_configs: Vec<crate::params::ImportConfig>,
    review_outputs: Vec<DetectionMode>,
    review_file: usize,
}

const REVIEW_CHANNELS: [DetectionMode; 4] = [
    DetectionMode::Transmission,
    DetectionMode::Fluorescence,
    DetectionMode::Reference,
    DetectionMode::MuColumn,
];

impl StudioApp {
    pub(crate) fn open_import_review(
        &mut self,
        batch: usize,
        cluster: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(scope) = self.capture_review_scope(batch, cluster) else {
            return;
        };
        let Some(target) = scope.targets.targets.first() else {
            return;
        };
        let studio = cx.weak_entity();
        let theme = self.theme;
        let editor = cx.new(|cx| {
            ImportEditor::new(
                studio,
                target.target.clone(),
                target.params.clone(),
                theme,
                false,
                window,
                cx,
            )
        });
        editor.update(cx, |editor, cx| {
            editor.review_configs = REVIEW_CHANNELS
                .iter()
                .map(|&mode| crate::params::ImportConfig {
                    mode,
                    ..Default::default()
                })
                .collect();
            editor.review_outputs = vec![scope.primary];
            editor.bulk_scope = Some(scope.targets.clone());
            editor.review_scope = Some(scope);
            editor.reload(cx);
        });
        self.import_editor = Some(editor);
        cx.notify();
    }
    pub(crate) fn open_channel_editor(
        &mut self,
        ix: usize,
        mode: DetectionMode,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(target) = self
            .tool_target(ix)
            .filter(|t| !t.path.as_os_str().is_empty())
        else {
            return;
        };
        let Some(scope) = self.capture_channel_scope(&target, mode, false) else {
            return;
        };
        self.open_import_editor(ix, window, cx);
        if let Some(editor) = self.import_editor.clone() {
            editor.update(cx, |editor, cx| {
                editor.creation = Some(mode);
                editor.channel_scope = Some(scope.clone());
                editor.bulk_scope = Some(scope.targets);
                editor.locked = false;
                editor
                    .spacing
                    .update(cx, |input, cx| input.set_enabled(true, cx));
                editor.params.import.mode = mode;
                editor.params = import_channels::channel_params(editor.params.import.clone());
                editor.reload(cx);
            });
        }
    }

    pub(crate) fn open_import_editor(
        &mut self,
        ix: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(target) = self
            .tool_target(ix)
            .filter(|t| !t.path.as_os_str().is_empty())
        else {
            self.status = "This result has no source columns to re-map.".into();
            cx.notify();
            return;
        };
        let studio = cx.weak_entity();
        let params = self.effective_params(ix).clone();
        let theme = self.theme;
        let locked = self.frozen.contains(&ix);
        let editor =
            cx.new(|cx| ImportEditor::new(studio, target, params, theme, locked, window, cx));
        self.import_editor = Some(editor.clone());
        editor.update(cx, |editor, cx| editor.reload(cx));
        cx.notify();
    }
}

impl ImportEditor {
    fn new(
        studio: WeakEntity<StudioApp>,
        target: ToolTarget,
        params: PipelineParams,
        theme: Theme,
        locked: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let opener = window.focused(cx);
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        let spacing = cx.new(|cx| TextInput::new("d spacing (Å)", "", theme, cx));
        spacing.update(cx, |input, cx| input.set_enabled(!locked, cx));
        cx.subscribe(&spacing, |this, _, event, cx| {
            if let InputEvent::Edited(text) = event {
                let value = text
                    .parse::<f64>()
                    .ok()
                    .filter(|v| v.is_finite() && *v > 0.);
                if let Some(draft) = &mut this.draft {
                    match draft.config().axis {
                        AxisConversion::AngleDegrees { .. } => {
                            draft.set_axis(AxisConversion::AngleDegrees {
                                d_spacing: value.unwrap_or(0.),
                            })
                        }
                        AxisConversion::AngleRadians { .. } => {
                            draft.set_axis(AxisConversion::AngleRadians {
                                d_spacing: value.unwrap_or(0.),
                            })
                        }
                        _ => return,
                    }
                    this.refresh(cx);
                }
            }
        })
        .detach();
        Self {
            studio,
            target,
            params,
            theme,
            opener,
            focus,
            spacing,
            locked,
            controls: HashMap::new(),
            visible_focus: Vec::new(),
            draft: None,
            table: None,
            preview: PreviewState::default(),
            plot: None,
            error: None,
            source_revision: None,
            generation: 0,
            open_role: None,
            bulk_scope: None,
            validation: None,
            validation_generation: 0,
            validating: false,
            show_targets: false,
            show_details: false,
            batch_available: false,
            creation: None,
            channel_scope: None,
            batch_selected: false,
            review_scope: None,
            review_configs: vec![],
            review_outputs: vec![],
            review_file: 0,
        }
    }

    fn key(&self, revision: u64) -> Option<PreviewKey> {
        Some(PreviewKey {
            group: self.target.group_id.clone()?,
            path: self.target.path.clone(),
            target_revision: self.target.fingerprint,
            draft_revision: revision,
            source_revision: self.source_revision.clone()?,
        })
    }

    fn reload(&mut self, cx: &mut Context<Self>) {
        self.invalidate_validation();
        self.generation += 1;
        self.draft = None;
        self.table = None;
        self.plot = None;
        self.preview.invalidate();
        self.error = None;
        self.open_role = None;
        self.source_revision = match SourceRevision::read(&self.target.path) {
            Ok(revision) => Some(revision),
            Err(error) => {
                self.error = Some(error);
                cx.notify();
                return;
            }
        };
        let Some(key) = self.key(0) else {
            return;
        };
        self.start_preview(key, self.params.import.clone(), true, cx);
    }

    fn refresh(&mut self, cx: &mut Context<Self>) {
        self.save_review_draft();
        self.invalidate_validation();
        self.generation += 1;
        self.preview.invalidate();
        self.plot = None;
        self.error = self.draft.as_ref().and_then(|d| d.validate().err());
        if self.error.is_none()
            && let Some(draft) = &self.draft
            && let Some(key) = self.key(draft.revision)
        {
            self.start_preview(key, draft.config().clone(), false, cx);
        }
        cx.notify();
    }

    fn start_preview(
        &mut self,
        key: PreviewKey,
        config: crate::params::ImportConfig,
        initial: bool,
        cx: &mut Context<Self>,
    ) {
        self.preview.begin(key.clone());
        let generation = self.generation;
        cx.spawn(async move |this, cx| {
            if !initial {
                cx.background_executor()
                    .timer(import_preview::DEBOUNCE)
                    .await;
            }
            if this
                .read_with(cx, |this, _| this.generation != generation)
                .unwrap_or(true)
            {
                return;
            }
            let result = cx
                .background_executor()
                .spawn({
                    let key = key.clone();
                    let config = config.clone();
                    async move { import_preview::load(&key, &config) }
                })
                .await;
            this.update(cx, |this, cx| {
                if generation != this.generation || !this.preview.finish(&key, result) {
                    return;
                }
                if let Some(Ok(result)) = &this.preview.result {
                    if initial {
                        this.table = Some(result.table.clone());
                        this.draft = Some(MappingDraft::new(&result.table, &config));
                        this.batch_available = this
                            .studio
                            .read_with(cx, |studio, _| {
                                this.target
                                    .group_id
                                    .as_ref()
                                    .and_then(|id| studio.intake.origin(&this.target.path, id))
                                    .is_some()
                            })
                            .unwrap_or(false);
                        let spacing = match config.axis {
                            AxisConversion::AngleDegrees { d_spacing }
                            | AxisConversion::AngleRadians { d_spacing } => d_spacing.to_string(),
                            _ => result
                                .table
                                .xdi
                                .as_ref()
                                .and_then(|h| h.get("mono.d_spacing"))
                                .unwrap_or("")
                                .to_string(),
                        };
                        this.spacing
                            .update(cx, |input, cx| input.set_text(spacing, cx));
                    }
                    this.error = this
                        .draft
                        .as_ref()
                        .and_then(|d| d.validate().err())
                        .or_else(|| result.raw.as_ref().err().cloned());
                    if this.error.is_none()
                        && let Ok(raw) = &result.raw
                    {
                        let plot: Plot = Plot::new()
                            .theme(this.theme.plot_theme())
                            .line(&raw.energy, &raw.mu)
                            .label("Raw μ(E)")
                            .into();
                        this.plot = Some(
                            plot_builder(
                                plot.xlabel("Energy (eV)").ylabel("μ(E)").size_px(420, 280),
                            )
                            .interactive()
                            .build(cx),
                        );
                    }
                } else if let Some(Err(error)) = &this.preview.result {
                    this.error = Some(error.clone());
                }
                if initial {
                    this.save_review_draft();
                }
                if initial && (this.creation.is_some() || this.review_scope.is_some()) {
                    this.validate_targets(cx);
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.invalidate_validation();
        self.generation += 1;
        self.preview.invalidate();
        self.studio
            .update(cx, |studio, cx| {
                studio.import_editor = None;
                cx.notify();
            })
            .ok();
        if let Some(opener) = &self.opener {
            window.focus(opener, cx);
        }
    }

    fn apply(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.creation.is_some() && self.channel_scope.is_none() {
            return;
        }
        let Some(draft) = &self.draft else {
            return;
        };
        let Some(key) = self.key(draft.revision) else {
            return;
        };
        if self.locked || draft.validate().is_err() || !self.preview.ready(&key) {
            return;
        }
        if SourceRevision::read(&key.path).ok().as_ref() != Some(&key.source_revision) {
            self.error = Some("Source changed; reload the preview before applying.".into());
            self.preview.invalidate();
            self.plot = None;
            cx.notify();
            return;
        }
        if let Some(scope) = &self.review_scope {
            let Some(validated) = &self.validation else {
                return;
            };
            let Some(choice) = self.review_choice() else {
                return;
            };
            let Some(table) = &self.table else {
                return;
            };
            let layout = LayoutKey::from_preview(table);
            let revision = draft.revision;
            let result = self.studio.update(cx, |studio, cx| {
                studio.accept_review(scope, validated, &choice, &layout, revision, cx)
            });
            let batch = scope.batch;
            let cluster = scope.cluster;
            match result {
                Ok(Ok(_)) => {
                    let studio = self.studio.clone();
                    self.close(window, cx);
                    studio
                        .update(cx, |studio, cx| {
                            let remaining = studio.pending_clusters(batch).len();
                            if remaining > 0 {
                                studio.open_import_review(
                                    batch,
                                    cluster.min(remaining - 1),
                                    window,
                                    cx,
                                );
                            }
                        })
                        .ok();
                }
                result => {
                    self.error = Some(match result {
                        Ok(Err(e)) => e,
                        Err(e) => e.to_string(),
                        _ => unreachable!(),
                    });
                    self.invalidate_validation();
                    cx.notify();
                }
            }
            return;
        }
        if self.bulk_scope.is_some() {
            let Some(validation) = &self.validation else {
                return;
            };
            let mapping = draft.config().clone();
            let revision = draft.revision;
            let result = self.studio.update(cx, |studio, cx| {
                if let Some(scope) = &self.channel_scope {
                    studio.apply_channels(scope, validation, &mapping, revision, cx)
                } else {
                    studio.apply_repair(validation, &mapping, revision, cx)
                }
            });
            match result {
                Ok(Ok(_)) => self.close(window, cx),
                result => {
                    self.error = Some(match result {
                        Ok(Err(e)) => e,
                        Err(e) => e.to_string(),
                        _ => unreachable!(),
                    });
                    self.invalidate_validation();
                    cx.notify();
                }
            }
            return;
        }
        let mapping = draft.config().clone();
        let target = self.target.clone();
        let result = self.studio.update(cx, |studio, cx| -> Result<(), String> {
            let ix = target
                .group_id
                .as_ref()
                .and_then(|id| studio.menu_index(id))
                .ok_or("The target group was removed or the project changed.")?;
            check_target(
                &target,
                studio.tool_target(ix).as_ref(),
                studio.frozen.contains(&ix),
            )?;
            let before = studio.effective_params(ix).clone();
            let mut after = before.clone();
            after.import = mapping;
            let scope = (ix != NO_ENTRY).then_some(ix);
            if before != after {
                studio.apply_params_to(scope, after.clone());
                studio.record_param_edit(
                    scope,
                    None,
                    before,
                    after,
                    format!("re-map columns: {}", target.label),
                );
                studio.schedule_recompute(cx);
                studio.invalidate_explore_plots(cx);
                studio.sync_param_fields(cx);
                studio.sync_handles(cx);
            }
            studio.status = format!("Updated mapping for {}", target.label).into();
            cx.notify();
            Ok(())
        });
        match result {
            Ok(Ok(())) => self.close(window, cx),
            result => {
                self.error = Some(match result {
                    Ok(Err(error)) => error,
                    Err(error) => error.to_string(),
                    _ => unreachable!(),
                });
                cx.notify();
            }
        }
    }

    fn activate(&mut self, action: Action, window: &mut Window, cx: &mut Context<Self>) {
        match action {
            Action::ReviewLocate => {
                self.locate_pending(cx);
                return;
            }
            Action::ReviewLayout(next) => {
                if let Some(scope) = &self.review_scope {
                    let batch = scope.batch;
                    let cluster = if next {
                        (scope.cluster + 1) % scope.cluster_count
                    } else {
                        (scope.cluster + scope.cluster_count - 1) % scope.cluster_count
                    };
                    let studio = self.studio.clone();
                    self.close(window, cx);
                    studio
                        .update(cx, |studio, cx| {
                            studio.open_import_review(batch, cluster, window, cx)
                        })
                        .ok();
                }
                return;
            }
            Action::ReviewFile(next) => {
                self.save_review_draft();
                if let Some(scope) = &self.review_scope {
                    let len = scope.targets.targets.len();
                    self.review_file = if next {
                        (self.review_file + 1) % len
                    } else {
                        (self.review_file + len - 1) % len
                    };
                    self.target = scope.targets.targets[self.review_file].target.clone();
                    if let Some(draft) = &self.draft {
                        self.params.import = draft.config().clone();
                    }
                    self.reload(cx);
                }
                return;
            }
            Action::ReviewEdit(index) => {
                self.save_review_draft();
                if let Some(config) = self.review_configs.get(index as usize) {
                    self.params.import = config.clone();
                    self.reload(cx);
                }
                return;
            }
            Action::ReviewPrimary(index) => {
                let mode = REVIEW_CHANNELS[index as usize];
                if let Some(scope) = &mut self.review_scope {
                    scope.primary = mode;
                    if !self.review_outputs.contains(&mode) {
                        self.review_outputs.push(mode);
                    }
                }
                self.invalidate_validation();
                cx.notify();
                return;
            }
            Action::ReviewOutput(index) => {
                let mode = REVIEW_CHANNELS[index as usize];
                if self
                    .review_scope
                    .as_ref()
                    .is_some_and(|scope| scope.primary != mode)
                {
                    if self.review_outputs.contains(&mode) {
                        self.review_outputs.retain(|&m| m != mode);
                    } else {
                        self.review_outputs.push(mode);
                    }
                }
                self.invalidate_validation();
                cx.notify();
                return;
            }
            Action::Details => {
                self.show_details = !self.show_details;
                cx.notify();
                return;
            }
            Action::Targets => {
                self.show_targets = !self.show_targets;
                cx.notify();
                return;
            }
            Action::ThisGroup => {
                self.bulk_scope = None;
                self.batch_selected = false;
                if let Some(mode) = self.creation {
                    self.channel_scope = self
                        .studio
                        .read_with(cx, |studio, _| {
                            studio.capture_channel_scope(&self.target, mode, false)
                        })
                        .ok()
                        .flatten();
                    self.bulk_scope = self
                        .channel_scope
                        .as_ref()
                        .map(|scope| scope.targets.clone());
                }
                self.invalidate_validation();
                if self.creation.is_some() {
                    self.validate_targets(cx);
                }
                cx.notify();
                return;
            }
            Action::Batch => {
                if let Some(mode) = self.creation {
                    if !self.batch_selected {
                        self.channel_scope = self
                            .studio
                            .read_with(cx, |studio, _| {
                                studio.capture_channel_scope(&self.target, mode, true)
                            })
                            .ok()
                            .flatten();
                        self.bulk_scope = self
                            .channel_scope
                            .as_ref()
                            .map(|scope| scope.targets.clone());
                        self.batch_selected = true;
                    }
                } else if self.bulk_scope.is_none()
                    && let Some(draft) = &self.draft
                {
                    self.bulk_scope = self
                        .studio
                        .read_with(cx, |studio, _| {
                            studio.capture_repair_batch(&self.target, draft.channel())
                        })
                        .ok()
                        .flatten();
                }
                self.invalidate_validation();
                self.validate_targets(cx);
                return;
            }
            Action::Validate => {
                self.validate_targets(cx);
                return;
            }
            Action::Close | Action::Cancel => {
                self.close(window, cx);
                return;
            }
            Action::Apply => {
                self.apply(window, cx);
                return;
            }
            Action::Reload => {
                if let Some(draft) = &self.draft {
                    self.params.import = draft.config().clone();
                }
                self.reload(cx);
                return;
            }
            Action::Role(role) => {
                self.open_role = (self.open_role != Some(role)).then_some(role);
                cx.notify();
                return;
            }
            _ if self.locked => return,
            _ => {}
        }
        let Some(draft) = &mut self.draft else {
            return;
        };
        match action {
            Action::Reset => draft.use_detected(),
            Action::Axis(unit) => {
                let d_spacing = self.spacing.read(cx).text().parse().unwrap_or(0.);
                draft.set_axis(match unit {
                    0 => AxisConversion::Auto,
                    1 => AxisConversion::EnergyEv,
                    2 => AxisConversion::EnergyKev,
                    3 => AxisConversion::AngleDegrees { d_spacing },
                    _ => AxisConversion::AngleRadians { d_spacing },
                });
            }
            Action::Pick(column) => {
                if let Some(role) = self.open_role.take() {
                    draft.set_column(role, column);
                }
            }
            Action::Roi(column) => draft.toggle_roi(column),
            Action::ReferenceRatio => {
                draft.set_column(ColumnRole::It, draft.config().it_col.unwrap_or(2))
            }
            _ => {}
        }
        self.refresh(cx);
    }

    fn invalidate_validation(&mut self) {
        self.validation_generation += 1;
        self.validation = None;
        self.validating = false;
    }

    fn locate_pending(&mut self, cx: &mut Context<Self>) {
        let Some(scope) = self.review_scope.clone() else {
            return;
        };
        let old = self.target.path.clone();
        let generation = self.generation;
        let pick = cx.prompt_for_paths(gpui::PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Locate source".into()),
        });
        cx.spawn(async move |this, cx| {
            let Ok(Ok(Some(paths))) = pick.await else {
                return;
            };
            let Some(path) = paths.into_iter().next() else {
                return;
            };
            let path = cx
                .background_executor()
                .spawn(async move { path.canonicalize().map_err(|e| e.to_string()) })
                .await;
            this.update(cx, |this, cx| {
                if this.generation != generation {
                    return;
                }
                let result = path.and_then(|path| {
                    this.studio
                        .update(cx, |studio, _| studio.locate_pending(&scope, &old, path))
                        .map_err(|e| e.to_string())
                        .and_then(|r| r)
                });
                match result {
                    Ok(scope) => {
                        this.target = scope.targets.targets[this.review_file].target.clone();
                        this.bulk_scope = Some(scope.targets.clone());
                        this.review_scope = Some(scope);
                        this.reload(cx);
                    }
                    Err(error) => {
                        this.error = Some(error);
                        cx.notify();
                    }
                }
            })
            .ok();
        })
        .detach();
    }

    fn save_review_draft(&mut self) {
        if self.review_scope.is_some()
            && let Some(draft) = &self.draft
            && let Some(config) = self
                .review_configs
                .iter_mut()
                .find(|config| config.mode == draft.channel())
        {
            *config = draft.config().clone();
        }
    }

    fn review_choice(&self) -> Option<ReviewChoice> {
        Some(ReviewChoice {
            primary: self.review_scope.as_ref()?.primary,
            channels: self
                .review_configs
                .iter()
                .filter(|config| self.review_outputs.contains(&config.mode))
                .cloned()
                .collect(),
        })
    }

    fn validate_targets(&mut self, cx: &mut Context<Self>) {
        let Some(draft) = &self.draft else {
            return;
        };
        let Some(table) = &self.table else {
            return;
        };
        let Some(scope) = &self.bulk_scope else {
            return;
        };
        if draft.validate().is_err()
            || !self
                .key(draft.revision)
                .is_some_and(|key| self.preview.ready(&key))
        {
            return;
        }
        let Ok(scope) = self
            .studio
            .read_with(cx, |studio, _| studio.refresh_repair_scope(scope))
        else {
            return;
        };
        let mapping = draft.config().clone();
        let revision = draft.revision;
        let layout = LayoutKey::from_preview(table);
        let channel_scope = self.channel_scope.as_ref().and_then(|scope| {
            self.studio
                .read_with(cx, |studio, _| {
                    studio.refresh_channel_scope(scope, mapping.mode)
                })
                .ok()
        });
        let review = self.review_scope.as_ref().and_then(|scope| {
            self.studio
                .read_with(cx, |studio, _| studio.refresh_review_scope(scope))
                .ok()
                .zip(self.review_choice())
        });
        self.invalidate_validation();
        self.error = None;
        self.validating = true;
        let generation = self.validation_generation;
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    if let Some((scope, choice)) = review {
                        import_review::validate(scope, layout, choice, revision)
                    } else if let Some(scope) = channel_scope {
                        import_channels::validate(scope, layout, mapping, revision)
                    } else {
                        import_repair::validate(scope, layout, mapping, revision)
                    }
                })
                .await;
            this.update(cx, |this, cx| {
                if this.validation_generation == generation {
                    this.validation = Some(result);
                    this.validating = false;
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    fn focus_next(&self, backward: bool, window: &mut Window, cx: &mut Context<Self>) {
        let len = self.visible_focus.len();
        if len == 0 {
            return;
        }
        let index = self.visible_focus.iter().position(|f| f.is_focused(window));
        let next = match index {
            Some(i) if backward => (i + len - 1) % len,
            Some(i) => (i + 1) % len,
            None if backward => len - 1,
            None => 0,
        };
        window.focus(&self.visible_focus[next], cx);
        cx.stop_propagation();
    }

    fn button(
        &mut self,
        action: Action,
        label: impl Into<gpui::SharedString>,
        enabled: bool,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<gpui::Div> {
        let focus = self
            .controls
            .entry(action)
            .or_insert_with(|| cx.focus_handle().tab_stop(true))
            .clone();
        if enabled {
            self.visible_focus.push(focus.clone());
        }
        let theme = self.theme;
        let selected = if let Action::Axis(index) = action {
            self.draft.as_ref().is_some_and(|draft| {
                index
                    == match draft.config().axis {
                        AxisConversion::Auto => 0,
                        AxisConversion::EnergyEv => 1,
                        AxisConversion::EnergyKev => 2,
                        AxisConversion::AngleDegrees { .. } => 3,
                        AxisConversion::AngleRadians { .. } => 4,
                    }
            })
        } else if let Action::ReviewPrimary(index) = action {
            self.review_scope
                .as_ref()
                .is_some_and(|scope| scope.primary == REVIEW_CHANNELS[index as usize])
        } else if let Action::ReviewEdit(index) = action {
            self.params.import.mode == REVIEW_CHANNELS[index as usize]
        } else {
            false
        };
        div()
            .id(gpui::SharedString::from(format!("mapping-{action:?}")))
            .px_2()
            .py_1()
            .rounded_sm()
            .border_1()
            .border_color(theme.border)
            .bg(theme.surface)
            .when(selected, |d| d.bg(theme.accent).text_color(theme.bg))
            .child(label.into())
            .when(!enabled, |d| d.opacity(0.45))
            .when(enabled, |d| {
                d.cursor_pointer()
                    .track_focus(&focus)
                    .focus(|s| s.border_color(theme.accent))
                    .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                        this.activate(action, window, cx)
                    }))
                    .on_key_down(cx.listener(
                        move |this, event: &gpui::KeyDownEvent, window, cx| {
                            if focus.is_focused(window)
                                && matches!(event.keystroke.key.as_str(), "enter" | "space")
                            {
                                window.prevent_default();
                                cx.stop_propagation();
                                this.activate(action, window, cx);
                            }
                        },
                    ))
            })
    }
}

impl Render for ImportEditor {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.visible_focus.clear();
        let t = self.theme;
        let mut panel = div()
            .id("mapping-editor-panel")
            .overflow_y_scroll()
            .w_full()
            .max_w(px(1120.))
            .max_h_full()
            .min_h_0()
            .flex()
            .flex_col()
            .gap_3()
            .p_4()
            .rounded_lg()
            .border_1()
            .border_color(t.border)
            .bg(t.bg)
            .child(
                div()
                    .flex()
                    .justify_between()
                    .items_center()
                    .child(div().flex_1().child(format!(
                            "{} · {}",
                            self.creation
                                .map(|mode| format!("Add {}", mode.label()))
                                .unwrap_or_else(|| if self.review_scope.is_some() {
                                    "Review import"
                                } else {
                                    "Re-map columns"
                                }
                                .into()),
                            self.target.label
                        )))
                    .child(self.button(Action::Close, "Close", true, cx)),
            )
            .child(
                div()
                    .text_color(t.text_muted)
                    .child(self.target.path.display().to_string()),
            );
        if let Some(scope) = self.review_scope.clone() {
            panel = panel.child(scope.reason.clone()).child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_2()
                    .items_center()
                    .child(format!(
                        "Layout {} of {} · representative {} of {}",
                        scope.cluster + 1,
                        scope.cluster_count,
                        self.review_file + 1,
                        scope.targets.targets.len()
                    ))
                    .child(self.button(
                        Action::ReviewLayout(false),
                        "Previous layout",
                        scope.cluster_count > 1,
                        cx,
                    ))
                    .child(self.button(
                        Action::ReviewLayout(true),
                        "Next layout",
                        scope.cluster_count > 1,
                        cx,
                    ))
                    .child(self.button(
                        Action::ReviewFile(false),
                        "Previous file",
                        scope.targets.targets.len() > 1,
                        cx,
                    ))
                    .child(self.button(
                        Action::ReviewFile(true),
                        "Next file",
                        scope.targets.targets.len() > 1,
                        cx,
                    )),
            );
            let mut primary = div()
                .flex()
                .flex_wrap()
                .gap_2()
                .items_center()
                .child("Primary channel:");
            let mut outputs = div()
                .flex()
                .flex_wrap()
                .gap_2()
                .items_center()
                .child("Create:");
            let mut editing = div()
                .flex()
                .flex_wrap()
                .gap_2()
                .items_center()
                .child("Editing channel:");
            for (index, mode) in REVIEW_CHANNELS.into_iter().enumerate() {
                primary = primary.child(self.button(
                    Action::ReviewPrimary(index as u8),
                    mode.label(),
                    true,
                    cx,
                ));
                outputs = outputs.child(self.button(
                    Action::ReviewOutput(index as u8),
                    format!(
                        "{} {}",
                        if self.review_outputs.contains(&mode) {
                            "☑"
                        } else {
                            "☐"
                        },
                        mode.label()
                    ),
                    mode != scope.primary,
                    cx,
                ));
                editing = editing.child(self.button(
                    Action::ReviewEdit(index as u8),
                    mode.label(),
                    true,
                    cx,
                ));
            }
            panel = panel
                .child(primary)
                .child(outputs)
                .child(editing)
                .child(self.button(Action::ReviewLocate, "Locate source…", true, cx));
        }
        if self.locked {
            panel = panel.child("Processing locked · mapping is read-only");
        }
        if let Some(draft) = self.draft.clone() {
            panel = panel.child(div().text_size(px(16.)).child(draft.formula()));
            let mut axes = div()
                .flex()
                .flex_wrap()
                .gap_2()
                .items_center()
                .child("Axis units:");
            for (i, label) in [
                "Detected",
                "eV",
                "keV",
                "Angle · degrees",
                "Angle · radians",
            ]
            .into_iter()
            .enumerate()
            {
                axes = axes.child(self.button(Action::Axis(i as u8), label, !self.locked, cx));
            }
            if matches!(
                draft.config().axis,
                AxisConversion::AngleDegrees { .. } | AxisConversion::AngleRadians { .. }
            ) {
                self.visible_focus
                    .push(self.spacing.read(cx).focus_handle(cx));
                axes = axes
                    .child("d spacing (Å):")
                    .child(div().w(px(110.)).child(self.spacing.clone()))
                    .child("First-order Bragg conversion");
            }
            panel = panel.child(axes);
            let mut roles = div().flex().flex_wrap().gap_2();
            for (role, col) in draft.roles().into_iter().filter(|(role, _)| role != "ROI") {
                let key = match role.as_str() {
                    "Energy" => ColumnRole::Energy,
                    "I0" => ColumnRole::I0,
                    "It" => ColumnRole::It,
                    "Ir" => ColumnRole::Ir,
                    _ => ColumnRole::Mu,
                };
                let label = format!(
                    "{role}: {} ▾",
                    col.map(|c| draft.column_label(c))
                        .unwrap_or("Choose…".into())
                );
                roles = roles.child(self.button(Action::Role(key), label, !self.locked, cx));
            }
            if draft.channel() == DetectionMode::Reference {
                if draft.config().reference_mu_col.is_none() {
                    roles = roles.child(self.button(
                        Action::Role(ColumnRole::Mu),
                        "Reference μ column…",
                        !self.locked,
                        cx,
                    ));
                }
                roles = roles.child(self.button(
                    Action::ReferenceRatio,
                    "Use It / Ir",
                    !self.locked,
                    cx,
                ));
            }
            panel = panel.child(roles);
            if draft.channel() == DetectionMode::Fluorescence {
                let mut rois = div()
                    .id("mapping-rois")
                    .max_h(px(90.))
                    .overflow_y_scroll()
                    .flex()
                    .flex_wrap()
                    .gap_1()
                    .child("ROIs:");
                for column in 0..draft.column_count {
                    let checked = draft
                        .config()
                        .fluor_cols
                        .as_ref()
                        .is_some_and(|c| c.contains(&column));
                    rois = rois.child(self.button(
                        Action::Roi(column),
                        format!(
                            "{} {}",
                            if checked { "☑" } else { "☐" },
                            draft.column_label(column)
                        ),
                        !self.locked,
                        cx,
                    ));
                }
                panel = panel
                    .child(rois)
                    .child("Detector correction: none applied by rexafs");
            }
            if self.open_role.is_some() {
                let mut choices = div()
                    .id("mapping-column-picker")
                    .max_h(px(120.))
                    .overflow_y_scroll()
                    .flex()
                    .flex_wrap()
                    .gap_1();
                for column in 0..draft.column_count {
                    choices = choices.child(self.button(
                        Action::Pick(column),
                        draft.column_label(column),
                        !self.locked,
                        cx,
                    ));
                }
                panel = panel.child(choices);
            }
            let mut table = div()
                .id("mapping-original-table")
                .flex_1()
                .min_w_0()
                .overflow_x_scroll()
                .font_family(super::MONO)
                .text_size(px(11.))
                .flex()
                .flex_col();
            if let Some(original) = &self.table {
                let row = |values: Vec<String>| {
                    div().flex().children(
                        values
                            .into_iter()
                            .map(|value| div().w(px(115.)).flex_shrink_0().p_1().child(value)),
                    )
                };
                table = table.child(row((0..draft.column_count)
                    .map(|c| draft.column_label(c))
                    .collect()));
                table = table.child(row((0..draft.column_count)
                    .map(|c| {
                        draft
                            .xdi
                            .as_ref()
                            .and_then(|h| h.columns.get(c))
                            .and_then(|c| c.units.clone())
                            .unwrap_or("—".into())
                    })
                    .collect()));
                table = table.child(row((0..draft.column_count)
                    .map(|col| {
                        draft
                            .roles()
                            .into_iter()
                            .filter(|(_, index)| *index == Some(col))
                            .map(|(role, _)| role)
                            .collect::<Vec<_>>()
                            .join(", ")
                    })
                    .collect()));
                for values in &original.rows {
                    table = table.child(row(values.iter().map(|v| format!("{v:.6}")).collect()));
                }
            }
            if let Some(Ok(result)) = &self.preview.result {
                let summary = result.table.diagnostics.summary();
                let warnings = result.table.diagnostics.warnings();
                panel = panel.child(
                    div()
                        .flex()
                        .gap_2()
                        .items_center()
                        .text_color(t.text_muted)
                        .child(format!("Full source · {summary}"))
                        .child(self.button(Action::Details, "Details", !warnings.is_empty(), cx)),
                );
                if self.show_details {
                    panel =
                        panel.children(warnings.into_iter().map(|warning| div().child(warning)));
                }
            }
            let mut plot = div().w(px(420.)).h(px(280.)).min_w_0();
            if let Some(entity) = &self.plot {
                plot = plot.child(entity.clone());
            } else {
                plot = plot
                    .child("Raw μ(E) preview unavailable while the draft is invalid or updating.");
            }
            panel = panel.child(div().flex().gap_3().min_h_0().child(table).child(plot));
        } else {
            panel = panel.child(if self.error.is_some() {
                "Source preview unavailable."
            } else {
                "Reading source columns and full raw signal…"
            });
        }
        if let Some(error) = &self.error {
            panel = panel.child(div().text_color(t.accent).child(error.clone()));
        }
        let raw_ready = self
            .draft
            .as_ref()
            .and_then(|d| self.key(d.revision))
            .is_some_and(|key| self.preview.ready(&key))
            && !self.locked;
        let ready = raw_ready
            && self.error.is_none()
            && (self.creation.is_none() || self.channel_scope.is_some())
            && self.bulk_scope.as_ref().is_none_or(|_| {
                self.validation
                    .as_ref()
                    .is_some_and(|v| v.ready_count() > 0)
                    && !self.validating
            });
        if self.batch_available && self.review_scope.is_none() {
            panel = panel.child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_2()
                    .items_center()
                    .child("Target:")
                    .child(self.button(
                        Action::ThisGroup,
                        if self.creation.is_some() {
                            "This file"
                        } else {
                            "This group"
                        },
                        true,
                        cx,
                    ))
                    .child(self.button(
                        Action::Batch,
                        if self.creation.is_some() {
                            "All files in this import batch…"
                        } else {
                            "This channel in its import batch…"
                        },
                        self.draft.is_some() && !self.locked,
                        cx,
                    )),
            );
        }
        if let Some(scope) = &self.bulk_scope {
            let label = format!(
                "{} · {} captured {}",
                scope.label,
                scope.targets.len(),
                if self.creation.is_some() || self.review_scope.is_some() {
                    "files"
                } else {
                    "groups"
                }
            );
            panel = panel.child(label);
            let summary = self
                .validation
                .as_ref()
                .map(|v| {
                    if self.review_scope.is_some() {
                        format!(
                            "{} compatible files → {} groups · {} need separate review",
                            v.file_count(),
                            v.file_count() * self.review_outputs.len(),
                            v.results.len() - v.ready_count()
                        )
                    } else {
                        v.summary()
                    }
                })
                .unwrap_or_else(|| {
                    if self.validating {
                        "Validating every captured target…"
                    } else {
                        "Validate this draft to see compatible, locked and changed counts."
                    }
                    .into()
                });
            panel = panel.child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap_2()
                    .child(summary)
                    .child(self.button(
                        Action::Validate,
                        "Revalidate",
                        raw_ready && !self.validating,
                        cx,
                    ))
                    .child(self.button(Action::Targets, "View target files…", true, cx)),
            );
            if self.show_targets {
                let scope = self.bulk_scope.as_ref().unwrap();
                let mut list = div()
                    .id("mapping-target-files")
                    .max_h(px(150.))
                    .overflow_y_scroll();
                for (index, target) in scope.targets.iter().enumerate() {
                    let status = self
                        .validation
                        .as_ref()
                        .and_then(|v| v.results.get(index))
                        .map(|v| v.description())
                        .unwrap_or("Not validated for this draft".into());
                    list = list.child(div().py_1().child(format!(
                        "{} · {} · {status}",
                        target.target.label,
                        target.target.path.display()
                    )));
                }
                panel = panel.child(list);
            }
        }
        let mut apply_label = self
            .validation
            .as_ref()
            .map(|v| format!("Apply to {} files", v.file_count()))
            .unwrap_or_else(|| {
                if self.bulk_scope.is_some() {
                    "Apply to validated files".into()
                } else {
                    "Apply to 1 file".into()
                }
            });
        if let Some(mode) = self.creation {
            apply_label = self
                .validation
                .as_ref()
                .map(|v| format!("Add {} {} groups", v.ready_count(), mode.label()))
                .unwrap_or("Add validated channels".into());
        }
        if self.review_scope.is_some() {
            apply_label = self
                .validation
                .as_ref()
                .map(|v| {
                    format!(
                        "Add {} files → {} groups",
                        v.file_count(),
                        v.file_count() * self.review_outputs.len()
                    )
                })
                .unwrap_or("Add validated files".into());
        }
        let scope_label = if self.review_scope.is_some() {
            "Only accepted files are added; other sources stay pending"
        } else if self.creation.is_some() {
            "New groups · independent processing"
        } else if self.bulk_scope.is_some() {
            "Mapping only · captured groups"
        } else {
            "Target: this group · 1 file"
        };
        panel = panel.child(
            div()
                .flex()
                .flex_wrap()
                .gap_2()
                .items_center()
                .child(self.button(
                    Action::Reset,
                    "Use detected columns",
                    self.draft.is_some() && !self.locked,
                    cx,
                ))
                .child(self.button(Action::Reload, "Reload source", true, cx))
                .child(div().flex_1().child(scope_label))
                .child(self.button(
                    Action::Cancel,
                    if self.review_scope.is_some() {
                        "Review later"
                    } else {
                        "Cancel"
                    },
                    true,
                    cx,
                ))
                .child(self.button(Action::Apply, apply_label, ready, cx)),
        );
        div()
            .id("import-mapping-modal")
            .key_context("ImportEditor")
            .track_focus(&self.focus)
            .absolute()
            .inset_0()
            .occlude()
            .bg(gpui::rgba(0x00000099))
            .flex()
            .items_center()
            .justify_center()
            .p_4()
            .text_size(px(12.))
            .text_color(t.text)
            .capture_action(
                cx.listener(|this, _: &NextField, window, cx| this.focus_next(false, window, cx)),
            )
            .capture_action(
                cx.listener(|this, _: &PrevField, window, cx| this.focus_next(true, window, cx)),
            )
            .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, window, cx| {
                if event.keystroke.key == "escape" {
                    this.close(window, cx);
                    cx.stop_propagation();
                } else if !this.spacing.read(cx).focus_handle(cx).is_focused(window) {
                    // Unhandled keys must reach the native input handler while a
                    // text field has focus. The ImportEditor context already
                    // excludes the surrounding Studio shortcuts.
                    cx.stop_propagation();
                }
            }))
            .child(panel)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn apply_refuses_removed_replaced_edited_and_newly_locked_targets() {
        let target = ToolTarget::standalone(
            Some(crate::group_identity::GroupId::new_result()),
            "/source.dat".into(),
            "Source".into(),
            11,
            2,
            3,
        );
        assert!(check_target(&target, Some(&target), false).is_ok());
        assert!(check_target(&target, None, false).is_err());
        assert!(check_target(&target, Some(&target), true).is_err());
        let mut changed = target.clone();
        changed.fingerprint += 1;
        assert!(check_target(&target, Some(&changed), false).is_err());
        changed = target.clone();
        changed.project_generation += 1;
        assert!(check_target(&target, Some(&changed), false).is_err());
        changed = target.clone();
        changed.group_id = Some(crate::group_identity::GroupId::new_result());
        assert!(check_target(&target, Some(&changed), false).is_err());
    }
}
