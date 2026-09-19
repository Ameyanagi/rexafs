//! Compact MBACK setup and comparison of independently retained normalization results.
use super::*;
use crate::{
    normalization_history::{self as history, NormalizationHistory, NormalizationRecord},
    params::{PipelineParams, RequiredStage, prepare_arrays},
    widgets::text_input::{InputEvent, TextInput},
};
use gpui::{AppContext, Entity};
use rexafs::prelude::{MbackErfc, MbackOptions, NormalizationMethod};
use ruviz::prelude::{LegendPosition, Plot};
use ruviz_gpui::{RuvizPlot, plot_builder};

/// A single edited bound keeps the other bound visible in the existing plot.
/// Entirely automatic intervals still use the core's MBACK suggestion.
fn inherit_visible_bounds(options: &mut MbackOptions, p: &PipelineParams, visible: [f64; 4]) {
    if options.pre_edge.is_none() && (p.pre_edge_start.is_some() != p.pre_edge_end.is_some()) {
        options.pre_edge = Some([visible[0], visible[1]]);
    }
    if options.post_edge.is_none() && (p.norm_start.is_some() != p.norm_end.is_some()) {
        options.post_edge = Some([visible[2], visible[3]]);
    }
}

#[derive(Default)]
pub(crate) struct NormalizationState {
    pub history: NormalizationHistory,
    pub open: bool,
    group: Option<crate::group_identity::GroupId>,
    editor_group: Option<crate::group_identity::GroupId>,
    editor_model: Option<MbackOptions>,
    pub(crate) show_mback: bool,
    fields: Vec<Entity<TextInput>>,
    erfc: bool,
    busy: bool,
    generation: u64,
    view: usize,
    records: Vec<NormalizationRecord>,
    plot: Option<Entity<RuvizPlot>>,
    message: String,
    identity_hint: String,
}
impl NormalizationState {
    fn sync_editor(
        &mut self,
        group: Option<crate::group_identity::GroupId>,
        model: Option<MbackOptions>,
    ) -> bool {
        if self.editor_group == group && self.editor_model == model {
            return false;
        }
        self.open = false;
        self.generation += 1;
        self.busy = false;
        self.group = group.clone();
        self.editor_group = group;
        self.show_mback = model.is_some();
        self.editor_model = model;
        self.records.clear();
        self.plot = None;
        self.view = 0;
        self.message.clear();
        true
    }
}
impl StudioApp {
    pub(crate) fn normalization_matches_current(&self) -> bool {
        self.normalization.group == self.current_group_index().and_then(|i| self.group_id(i))
    }
    fn current_normalization_record(&self) -> Result<NormalizationRecord, String> {
        if self.stale_plots.is_some()
            || self.active_fingerprint() != self.spectrum_fingerprint
            || self.current_tool_target() != self.spectrum_group
        {
            return Err("Wait for the current spectrum to finish processing".into());
        }
        let group = self
            .current_group_index()
            .and_then(|i| self.group_id(i))
            .ok_or("Select a spectrum")?;
        NormalizationRecord::new(
            group,
            self.spectrum_label.to_string(),
            self.ui_params().clone(),
            self.spectrum.as_ref().ok_or("Select a spectrum")?,
        )
    }
    /// Reload the editor after group changes, undo, reset or settings replacement.
    /// Unapplied edits remain local to their original group.
    pub(crate) fn ensure_normalization_editor(&mut self, cx: &mut Context<Self>) {
        let group = self.current_group_index().and_then(|i| self.group_id(i));
        let model = self.ui_params().mback.clone();
        if !self.normalization.sync_editor(group, model) {
            return;
        }
        let mut model = self.ui_params().mback.clone().unwrap_or_default();
        self.normalization.identity_hint = "Chantler f₂".into();
        if model.element.is_empty() {
            let declared = self.current_group_index().and_then(|ix| {
                self.raw_cache
                    .peek(&(ix, self.effective_params(ix).raw_fingerprint()))
                    .and_then(|r| r.declared_edge.clone())
                    .or_else(|| {
                        self.group_id(ix)
                            .and_then(|id| self.parser_evidence.get(&id))
                            .and_then(|p| p.declared_edge.clone())
                    })
                    .or_else(|| {
                        ix.checked_sub(crate::app::DERIVED_BASE)
                            .and_then(|i| self.derived.get(i))
                            .and_then(|d| {
                                d.declared_edge.clone().or_else(|| {
                                    let op = d.operation.as_ref()?;
                                    if op.tool != "Measurement import"
                                        || op.parameters["format"] != "xdi"
                                    {
                                        return None;
                                    }
                                    crate::source_evidence::DeclaredEdge::from_xdi_header_text(
                                        op.parameters["source_record"]["header"].as_str()?,
                                    )
                                })
                            })
                    })
            });
            if let Some(declared) = declared {
                model.element = declared.element;
                model.edge = declared.edge;
                self.normalization.identity_hint = "Suggested by source header · editable".into();
            } else if let Some(guess) = self.spectrum_interest() {
                // Nearest tabulated absorption edge to the spectrum's E₀ (xraydb
                // guess_edge, within 100 eV); the user can still edit both fields.
                model.element = guess.element;
                model.edge = guess.edge.unwrap_or_else(|| "K".into());
                self.normalization.identity_hint = if guess.estimated {
                    "Estimated from E₀ · editable".into()
                } else {
                    "Suggested by source header · editable".into()
                };
            }
        }
        self.normalization.erfc = model.erfc.is_some();
        let line = model
            .erfc
            .as_ref()
            .map(|e| match &e.emission {
                rexafs::atomic::EmissionSelection::Line(s) => s.clone(),
                rexafs::atomic::EmissionSelection::Family(s) => format!("family:{s}"),
            })
            .unwrap_or_default();
        let values = [
            model.element,
            if model.edge.is_empty() {
                "K".into()
            } else {
                model.edge
            },
            line,
            model
                .erfc
                .as_ref()
                .map(|e| e.width_ev[0].to_string())
                .unwrap_or_default(),
            model
                .erfc
                .as_ref()
                .map(|e| e.width_ev[1].to_string())
                .unwrap_or_default(),
            model
                .erfc
                .as_ref()
                .map(|e| e.amplitude[0].to_string())
                .unwrap_or_default(),
            model
                .erfc
                .as_ref()
                .map(|e| e.amplitude[1].to_string())
                .unwrap_or_default(),
        ];
        self.normalization.fields.clear();
        for (label, value) in [
            "Absorber",
            "Edge",
            "Emission line",
            "Width min (eV)",
            "Width max (eV)",
            "Amplitude min (f₂)",
            "Amplitude max (f₂)",
        ]
        .into_iter()
        .zip(values)
        {
            let field = cx.new(|cx| {
                let mut field = TextInput::new(label, value, self.theme, cx);
                field.set_accessible_name(label);
                field
            });
            cx.subscribe(&field, |app, _, event, cx| {
                if matches!(event, InputEvent::Edited(_)) {
                    app.normalization.identity_hint = "Chantler f₂ · edited settings".into();
                    app.normalization.generation += 1;
                    app.normalization.busy = false;
                    app.normalization.records.clear();
                    app.normalization.plot = None;
                    app.normalization.message = "Settings changed · Apply to update".into();
                    cx.notify();
                }
            })
            .detach();
            self.normalization.fields.push(field);
        }
    }
    fn select_normalization_method(&mut self, mback: bool, cx: &mut Context<Self>) {
        if self.refuse_frozen_edit(cx) {
            return;
        }
        self.ensure_normalization_editor(cx);
        self.normalization.generation += 1;
        self.normalization.busy = false;
        self.normalization.open = false;
        self.normalization.show_mback = mback;
        self.normalization.message.clear();
        if mback != self.ui_params().mback.is_some() {
            // A declared absorber lets method selection behave like the other
            // normalization settings. Missing metadata stays an explicit choice.
            self.calculate_normalization(mback, true, cx);
        }
        cx.notify();
    }
    fn normalization_options(&self, cx: &Context<Self>) -> Result<MbackOptions, String> {
        let value = |i: usize| {
            self.normalization.fields[i]
                .read(cx)
                .text()
                .trim()
                .to_owned()
        };
        if self.normalization.fields.len() != 7 || value(0).is_empty() || value(1).is_empty() {
            return Err("Choose the absorber and edge".into());
        }
        let mut options = self.ui_params().mback.clone().unwrap_or_default();
        // A changed identity explicitly selects current data; an unchanged model
        // retains its original data identity and fails if that version is unavailable.
        if options.element != value(0) || options.edge != value(1) {
            options.reference = None;
        }
        options.element = value(0);
        options.edge = value(1);
        options.erfc = if self.normalization.erfc {
            let number = |i: usize| {
                value(i)
                    .parse::<f64>()
                    .ok()
                    .filter(|v| v.is_finite())
                    .ok_or("Enter finite erfc bounds".to_owned())
            };
            let line = value(2);
            if line.is_empty() {
                return Err("Choose an emission line, such as Ka1".into());
            }
            let mut term = MbackErfc::new(
                line.clone(),
                number(3)?..=number(4)?,
                number(5)?..=number(6)?,
            );
            if let Some(family) = line.strip_prefix("family:") {
                term.emission = rexafs::atomic::EmissionSelection::Family(family.into());
            }
            Some(term)
        } else {
            None
        };
        Ok(options)
    }
    /// Calculation, retention and settings replacement are ordered: a failed
    /// preview/storage operation never destroys the previous result or recipe.
    pub(crate) fn calculate_normalization(
        &mut self,
        mback: bool,
        apply: bool,
        cx: &mut Context<Self>,
    ) {
        if self.normalization.busy || (apply && self.refuse_frozen_edit(cx)) {
            return;
        }
        let Some(ix) = self.current_group_index() else {
            return;
        };
        let Some(group) = self.group_id(ix) else {
            return;
        };
        let input = self.measurement_input(&group, self.entry_label(ix));
        let before = self.ui_params().clone();
        let mut settings = before.clone();
        settings.mback = if mback {
            match self.normalization_options(cx) {
                Ok(o) => Some(o),
                Err(e) => {
                    self.normalization.message = e;
                    cx.notify();
                    return;
                }
            }
        } else {
            None
        };
        let old = self.current_normalization_record().ok();
        if let (Some(options), Some(previous)) = (settings.mback.as_mut(), old.as_ref())
            && let Ok(sp) = previous.spectrum()
            && let Some(ranges) = crate::params::normalization_ranges(&sp)
        {
            inherit_visible_bounds(options, &before, ranges);
        }
        let generation = self.normalization.generation + 1;
        self.normalization.generation = generation;
        self.normalization.busy = true;
        self.normalization.message = "Calculating…".into();
        let project = self.project_generation;
        cx.spawn(async move |this, cx| {
            let worker_group = group.clone();
            let result = cx.background_spawn(async move {
                // Shared preparation preserves typed recovered spectra and all
                // energy preparation. No XANES-only fit invokes AUTOBK.
                let sp = match &input.derived {
                    Some(d) => d.prepare(&settings, RequiredStage::Normalized)?,
                    None => {
                        let (e, mu) = crate::params::load_raw(&input.path, &settings)?;
                        prepare_arrays(e, mu, &settings, RequiredStage::Normalized)?
                    }
                };
                if mback {
                    let Some(NormalizationMethod::MBack(n)) = &sp.normalization else {
                        return Err("Enable baseline refitting before applying MBACK to recovered arrays".into());
                    };
                    let r = n.result.as_ref().ok_or("Missing MBACK result")?;
                    let options = settings.mback.as_mut().unwrap();
                    options.reference = Some(r.reference.clone());
                    options.pre_edge = Some(r.pre_edge);
                    options.post_edge = Some(r.post_edge);
                }
                let record = NormalizationRecord::new(worker_group, input.label, settings.clone(), &sp)?;
                let mut receipts = Vec::new();
                if apply {
                    let root = history::root()?;
                    if let Some(old) = old { receipts.push(history::retain(&root, &old)?); }
                    receipts.push(history::retain(&root, &record)?);
                }
                Ok::<_, String>((settings, record, receipts))
            }).await;
            this.update(cx, |app, cx| {
                if app.project_generation != project || app.normalization.generation != generation { return; }
                app.normalization.busy = false;
                if app.current_group_index().and_then(|i| app.group_id(i)).as_ref() != Some(&group) || app.ui_params() != &before {
                    app.normalization.message = "The spectrum or settings changed. Preview again.".into();
                    cx.notify(); return;
                }
                match result {
                    Err(e) => { app.normalization.message = e; app.normalization.records.clear(); app.normalization.plot = None; }
                    Ok((settings, record, receipts)) => {
                        let details = match &record.normalization {
                            NormalizationMethod::MBack(n) => n.result.as_ref().map(|r| format!(
                                "{} {} · table edge {:.1} eV · scale {:.5} · condition {:.2e}{}",
                                r.requested.element, r.requested.edge, r.tabulated_edge_ev, r.scale, r.condition,
                                if r.warnings.is_empty() { String::new() } else { format!(" · {}", r.warnings.join("; ")) }
                            )).unwrap_or_default(),
                            _ => "Polynomial normalization".into(),
                        };
                        app.normalization.message = if apply {
                            let mut message = format!("{} applied", record.method());
                            if let NormalizationMethod::MBack(n) = &record.normalization
                                && let Some(r) = &n.result
                                && !r.warnings.is_empty()
                            {
                                message.push_str(&format!(" · {}", r.warnings.join("; ")));
                            }
                            message
                        } else { format!("Preview · {details}") };
                        app.normalization.records = vec![record];
                        if !mback { app.normalization.view = 0; }
                        if apply {
                            for receipt in receipts { app.normalization.history.insert(receipt); }
                            app.record("Retained independent normalization results", None);
                            app.normalization.editor_model = settings.mback.clone();
                            app.normalization.show_mback = mback;
                            app.normalization.open = false;
                            app.edit_parameters("Change normalization method; preserve independent results".into(), cx, |p| { *p = settings; Ok(()) });
                        }
                        app.rebuild_normalization_plot(false, cx);
                    }
                }
                cx.notify();
            }).ok();
        }).detach();
        cx.notify();
    }
    pub(crate) fn compare_normalizations(&mut self, cx: &mut Context<Self>) {
        if self.normalization.busy {
            return;
        }
        let current = match self.current_normalization_record() {
            Ok(r) => r,
            Err(e) => {
                self.status = e.into();
                cx.notify();
                return;
            }
        };
        let input = current.input_digest();
        let candidates: Vec<_> = self
            .normalization
            .history
            .entries
            .iter()
            .rev()
            .filter(|r| {
                r.group == current.group && r.input_digest == input && r.method != current.method()
            })
            .take(1)
            .cloned()
            .collect();
        self.normalization.open = true;
        self.normalization.group = self.current_group_index().and_then(|i| self.group_id(i));
        self.normalization.view = 0;
        self.normalization.busy = true;
        self.normalization.generation += 1;
        let generation = self.normalization.generation;
        let project = self.project_generation;
        let group = current.group.clone();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move {
                    let receipt = history::retain(&history::root()?, &current)?;
                    let mut records = vec![current];
                    for receipt in candidates {
                        records.push(history::read(&receipt)?);
                    }
                    Ok::<_, String>((receipt, records))
                })
                .await;
            this.update(cx, |app, cx| {
                if project != app.project_generation || generation != app.normalization.generation {
                    return;
                }
                app.normalization.busy = false;
                if app
                    .current_group_index()
                    .and_then(|i| app.group_id(i))
                    .as_ref()
                    != Some(&group)
                {
                    app.normalization.message =
                        "Select the original spectrum to compare its results".into();
                    cx.notify();
                    return;
                }
                match result {
                    Ok((receipt, records)) => {
                        app.normalization.history.insert(receipt);
                        app.record("Retained normalization comparison", None);
                        app.normalization.message = if records.len() == 1 {
                            "Current result saved. Apply the other method, then compare.".into()
                        } else {
                            "Independent saved results · identical input arrays".into()
                        };
                        app.normalization.records = records;
                        app.rebuild_normalization_plot(false, cx);
                    }
                    Err(e) => app.normalization.message = e,
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }
    fn export_normalizations(&mut self, cx: &mut Context<Self>) {
        if self.normalization.records.is_empty() {
            return;
        }
        let records = self.normalization.records.clone();
        let request =
            cx.prompt_for_new_path(&std::env::temp_dir(), Some("normalization-comparison.json"));
        let project = self.project_generation;
        cx.spawn(async move |this, cx| {
            let Ok(Ok(Some(path))) = request.await else {
                return;
            };
            let result = cx
                .background_spawn(async move {
                    let parent = path.parent().ok_or("Export folder unavailable")?;
                    let mut file =
                        tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
                    serde_json::to_writer_pretty(&mut file, &records).map_err(|e| e.to_string())?;
                    file.as_file().sync_all().map_err(|e| e.to_string())?;
                    file.persist(&path).map_err(|e| e.to_string())?;
                    Ok::<_, String>(
                        "Exported results, original arrays and processing provenance".to_owned(),
                    )
                })
                .await;
            this.update(cx, |app, cx| {
                if project == app.project_generation {
                    app.normalization.message = result.unwrap_or_else(|e| e);
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
    }
    pub(crate) fn rebuild_normalization_plot(
        &mut self,
        preserve_view: bool,
        cx: &mut Context<Self>,
    ) {
        let view = self.normalization.view;
        let mut plot = Plot::new().size(9., 5.).theme(self.theme.plot_theme());
        let mut count = 0;
        for (i, record) in self.normalization.records.iter().enumerate() {
            let color = crate::plotting::trace_color(&self.theme, i);
            if view < 2 {
                if let Ok(sp) = record.spectrum() {
                    let values = if view == 0 { sp.norm() } else { sp.flat() };
                    if let Some(y) = values {
                        let y: Vec<_> = y.iter().copied().collect();
                        plot = plot
                            .line(&record.energy, &y)
                            .label(format!("{} · {}", record.method(), record.label))
                            .color(color)
                            .into();
                        count += 1;
                    }
                }
            } else if let NormalizationMethod::MBack(n) = &record.normalization
                && let Some(r) = &n.result
            {
                if view == 2 {
                    let scaled: Vec<_> = record.mu.iter().map(|v| r.scale * v).collect();
                    plot = plot.line(&r.energy, &scaled).label("Scaled μ").into();
                    plot = plot.line(&r.energy, &r.f2).label("Atomic f₂").into();
                    plot = plot
                        .line(&r.energy, &r.background)
                        .label("Background")
                        .into();
                    plot = plot.line(&r.energy, &r.fpp).label("Matched fpp").into();
                } else {
                    plot = plot
                        .line(&r.energy, &r.residual)
                        .label("f₂ + background − scaled μ")
                        .into();
                }
                if !self.handles.hidden {
                    for range in [r.pre_edge, r.post_edge] {
                        plot = plot.axvspan(r.e0 + range[0], r.e0 + range[1]);
                    }
                }
                count += 1;
            }
        }
        self.normalization.plot = (count > 0).then(|| {
            let plot = plot
                .xlabel("Energy (eV)")
                .ylabel(match view {
                    0 => "Normalized μ(E)",
                    1 => "Flattened μ(E)",
                    2 => "f₂ units",
                    _ => "Residual (f₂ units)",
                })
                .legend_position(LegendPosition::UpperRight);
            if preserve_view && let Some(entity) = &self.normalization.plot {
                entity.update(cx, |view, cx| view.set_plot_keep_view(plot, cx));
                entity.clone()
            } else {
                plot_builder(plot).interactive().build(cx)
            }
        });
    }
    pub(crate) fn normalization_controls(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let t = self.theme;
        let mback = self.normalization.show_mback;
        let mut controls = div().p_3().flex().flex_col().gap_2().child(
            div()
                .flex()
                .gap_1()
                .child(
                    button(&t, "normalization-polynomial", "Polynomial", !mback).on_click(
                        cx.listener(|app, _, _, cx| app.select_normalization_method(false, cx)),
                    ),
                )
                .child(button(&t, "normalization-mback", "MBACK", mback).on_click(
                    cx.listener(|app, _, _, cx| app.select_normalization_method(true, cx)),
                )),
        );
        if mback {
            let mut identity = div().flex().gap_2();
            for (i, label) in ["Absorber", "Edge"].into_iter().enumerate() {
                if let Some(field) = self.normalization.fields.get(i) {
                    identity = identity.child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(label)
                            .child(field.clone()),
                    );
                }
            }
            controls = controls
                .child(identity)
                .child(
                    div()
                        .text_size(px(11.))
                        .text_color(t.text_muted)
                        .child(self.normalization.identity_hint.clone()),
                )
                .child(
                    button(&t, "mback-erfc", "Erfc background", self.normalization.erfc).on_click(
                        cx.listener(|app, _, _, cx| {
                            app.normalization.erfc = !app.normalization.erfc;
                            app.normalization.generation += 1;
                            app.normalization.busy = false;
                            app.normalization.records.clear();
                            app.normalization.plot = None;
                            app.normalization.message = "Settings changed · Apply to update".into();
                            cx.notify();
                        }),
                    ),
                );
            if self.normalization.erfc {
                for (i, label) in [
                    "Emission line",
                    "Width min (eV)",
                    "Width max (eV)",
                    "Amplitude min (f₂)",
                    "Amplitude max (f₂)",
                ]
                .into_iter()
                .enumerate()
                {
                    if let Some(field) = self.normalization.fields.get(i + 2) {
                        controls = controls.child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(div().flex_1().child(label))
                                .child(div().w(px(115.)).child(field.clone())),
                        );
                    }
                }
            }
            controls = controls.child(
                div()
                    .flex()
                    .gap_2()
                    .child(button(&t, "mback-apply", "Apply", true).on_click(
                        cx.listener(|app, _, _, cx| app.calculate_normalization(true, true, cx)),
                    ))
                    .child(
                        button(&t, "mback-preview", "Atomic match…", false).on_click(cx.listener(
                            |app, _, _, cx| {
                                app.normalization.open = true;
                                app.normalization.view = 2;
                                app.calculate_normalization(true, false, cx);
                            },
                        )),
                    ),
            );
        }
        if !self.normalization.open && !self.normalization.message.is_empty() {
            controls = controls.child(
                div()
                    .text_size(px(12.))
                    .text_color(t.text_muted)
                    .child(self.normalization.message.clone()),
            );
        }
        controls
            .child(
                button(&t, "normalization-compare", "Compare methods…", false)
                    .on_click(cx.listener(|app, _, _, cx| app.compare_normalizations(cx))),
            )
            .into_any_element()
    }
    pub(crate) fn normalization_center(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let t = self.theme;
        let mut header = div()
            .flex()
            .flex_wrap()
            .gap_2()
            .items_center()
            .child(
                button(&t, "normalization-back", "← Normalize", false).on_click(cx.listener(
                    |app, _, _, cx| {
                        app.normalization.open = false;
                        cx.notify();
                    },
                )),
            )
            .child(div().flex_1().child("Normalization comparison"));
        for (i, label) in ["Norm", "Flat", "Atomic match", "Residual"]
            .into_iter()
            .enumerate()
        {
            header = header.child(
                button(
                    &t,
                    SharedString::from(format!("normalization-view-{i}")),
                    label,
                    self.normalization.view == i,
                )
                .on_click(cx.listener(move |app, _, _, cx| {
                    app.normalization.view = i;
                    app.rebuild_normalization_plot(false, cx);
                    cx.notify();
                })),
            );
        }
        header = header.child(self.plot_ranges_button(cx)).child(
            button(&t, "normalization-export", "Export JSON…", false)
                .on_click(cx.listener(|app, _, _, cx| app.export_normalizations(cx))),
        );
        div()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .p_3()
            .flex()
            .flex_col()
            .gap_3()
            .child(header)
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(t.text_muted)
                    .child(self.normalization.message.clone()),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .min_w_0()
                    .children(self.normalization.plot.clone()),
            )
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn method_editor_keeps_drafts_local_and_invalidates_work_after_undo_or_group_change() {
        let first = Some(crate::group_identity::GroupId::new_result());
        let second = Some(crate::group_identity::GroupId::new_result());
        let model = Some(rexafs::MBack::for_edge("Cu", "K").options);
        let mut state = NormalizationState::default();
        assert!(state.sync_editor(first.clone(), None));
        // Ordinary redraws keep a not-yet-applied MBACK draft and its worker.
        state.show_mback = true;
        state.busy = true;
        let generation = state.generation;
        assert!(!state.sync_editor(first.clone(), None));
        assert!(state.show_mback && state.busy);
        assert_eq!(state.generation, generation);
        // A different spectrum must not inherit the draft or accept its result.
        state.open = true;
        assert!(state.sync_editor(second, None));
        assert!(!state.show_mback && !state.busy && !state.open);
        assert!(state.generation > generation);
        assert!(state.sync_editor(first.clone(), model.clone()));
        assert!(state.show_mback);
        // Undo and redo synchronize both the selected method and its controls.
        let generation = state.generation;
        assert!(state.sync_editor(first.clone(), None));
        assert!(!state.show_mback);
        assert!(state.generation > generation);
        assert!(state.sync_editor(first, model));
        assert!(state.show_mback);
    }
    #[test]
    fn method_switch_keeps_the_other_visible_bound_after_a_partial_edit() {
        let mut options = MbackOptions::default();
        let mut p = PipelineParams::default();
        inherit_visible_bounds(&mut options, &p, [-200., -30., 150., 1000.]);
        assert!(options.pre_edge.is_none() && options.post_edge.is_none());
        p.pre_edge_end = Some(-61.);
        p.norm_start = Some(180.);
        inherit_visible_bounds(&mut options, &p, [-200., -61., 180., 1000.]);
        assert_eq!(options.pre_edge, Some([-200., -61.]));
        assert_eq!(options.post_edge, Some([180., 1000.]));
        inherit_visible_bounds(&mut options, &p, [-300., -61., 180., 900.]);
        assert_eq!(options.pre_edge, Some([-200., -61.]));
        assert_eq!(options.post_edge, Some([180., 1000.]));
    }
}
