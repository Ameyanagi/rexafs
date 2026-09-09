//! One merge path for marked files, lazy channels, and materialized outputs.

use super::shell::tools::{ToolTarget, marked_group_indices};
use super::{DERIVED_BASE, NO_ENTRY, StudioApp, shell};
use crate::params::{
    DerivedSpectrum, DetectionMode, Operation, PipelineParams, Quantity, StreamingAverage,
    load_group_raw_with_diagnostics, preview_import,
};
use gpui::{Context, IntoElement, div, prelude::*, px};
use rexafs::prelude::XASSpectrum;
use std::{
    collections::BTreeSet,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

/// Canonical group list order (independent of filtering/collapse).
/// Resolve every mark; an invalid index is an error, never an omitted input.
fn collect_inputs(
    marks: &BTreeSet<usize>,
    mut resolve: impl FnMut(usize) -> Option<ToolTarget>,
) -> Result<Vec<ToolTarget>, String> {
    let unavailable = |ix| format!("merge refused: marked group {ix} is no longer available");
    let inputs = marked_group_indices(marks)
        .chain(marks.contains(&NO_ENTRY).then_some(NO_ENTRY))
        .map(|ix| resolve(ix).ok_or_else(|| unavailable(ix)))
        .collect::<Result<Vec<_>, _>>()?;
    if inputs.len() < 2 {
        return Err("mark at least 2 spectra to merge".into());
    }
    Ok(inputs)
}

fn template_first(inputs: &mut [ToolTarget], current: Option<&ToolTarget>) {
    if let Some(pos) = inputs.iter().position(|input| Some(input) == current) {
        // Keep all other operands in their original stable order.
        inputs[..=pos].rotate_right(1);
    }
}

#[derive(Clone)]
struct MergeInput {
    target: ToolTarget,
    params: PipelineParams,
    derived: Option<DerivedSpectrum>,
    // Auto on a materialized output must be resolved from its target lineage,
    // never treated as a sample merely because its arrays contain mu(E).
    mode_source: PathBuf,
}

impl MergeInput {
    fn quantity(&self) -> Quantity {
        self.derived
            .as_ref()
            .map_or(Quantity::RawMu, |d| d.quantity)
    }

    fn validate_quantity(&self) -> Result<(), String> {
        if let Some(reason) = self
            .derived
            .as_ref()
            .and_then(DerivedSpectrum::processing_block_reason)
        {
            return Err(format!("merge refused: {:?}: {reason}", self.target.label));
        }
        Ok(())
    }

    fn mode(&self) -> Result<DetectionMode, String> {
        if self.params.import.mode != DetectionMode::Auto {
            return Ok(self.params.import.mode);
        }
        if self.mode_source.as_os_str().is_empty() {
            return Err("cannot resolve Auto channel kind: source identity is unavailable".into());
        }
        preview_import(&self.mode_source, &self.params.import).map(|p| p.resolved.mode)
    }
}

struct Compatibility {
    label: String,
    quantity: Quantity,
    mode: DetectionMode,
    lo: f64,
    hi: f64,
    e0: f64,
    declared_edge: Option<crate::source_evidence::DeclaredEdge>,
}

impl Compatibility {
    fn from_raw(input: &MergeInput, energy: &[f64], mu: &[f64]) -> Result<Self, String> {
        if energy.len() < 2
            || energy.len() != mu.len()
            || energy.iter().chain(mu).any(|v| !v.is_finite())
            || energy.windows(2).any(|e| e[0] > e[1])
        {
            return Err("need matching finite arrays on a non-decreasing energy grid".into());
        }
        let mut sp = XASSpectrum::new();
        sp.set_spectrum(energy.to_vec(), mu.to_vec());
        sp.find_e0()
            .map_err(|e| format!("E0 detection failed: {e}"))?;
        let e0 = sp.e0().ok_or("E0 not found")?;
        if !e0.is_finite() {
            return Err("E0 must be finite".into());
        }
        Ok(Self {
            label: input.target.label.clone(),
            quantity: input.quantity(),
            mode: input.mode()?,
            lo: energy[0],
            hi: energy[energy.len() - 1],
            e0,
            declared_edge: input.derived.as_ref().and_then(|d| d.declared_edge.clone()),
        })
    }

    fn validate_explicit_e0(&self, explicit: Option<f64>) -> Result<(), String> {
        if let Some(e0) = explicit
            && (!e0.is_finite() || e0 < self.lo || e0 > self.hi || (e0 - self.e0).abs() > 50.0)
        {
            return Err(format!(
                "merge refused: {:?}: explicit E0 {e0} eV must be within the data range \
                 and within 50 eV of detected E0 {:.1} eV",
                self.label, self.e0
            ));
        }
        Ok(())
    }
}

/// Pairwise checks include non-template pairs (the total E0 spread is <= 50 eV).
fn compatible(a: &Compatibility, b: &Compatibility) -> Result<(), String> {
    let reason = if a.quantity != b.quantity || !a.quantity.is_absorption() {
        format!(
            "incompatible quantities: {} and {}",
            a.quantity.label(),
            b.quantity.label()
        )
    } else if a.mode == DetectionMode::Auto || b.mode == DetectionMode::Auto {
        "unresolved channel kind".into()
    } else if (a.mode == DetectionMode::Reference) != (b.mode == DetectionMode::Reference) {
        format!(
            "different channel kinds: {} and {}",
            a.mode.label(),
            b.mode.label()
        )
    } else if a.lo.max(b.lo) >= a.hi.min(b.hi) {
        "no overlapping energy range".into()
    } else if let (Some(left), Some(right)) = (&a.declared_edge, &b.declared_edge) {
        if left == right {
            return Ok(());
        }
        format!(
            "different declared edges: {} and {}",
            left.label(),
            right.label()
        )
    } else if (a.e0 - b.e0).abs() > 50.0 {
        format!(
            "different edges (E0 {:.1} eV and {:.1} eV; tolerance 50 eV)",
            a.e0, b.e0
        )
    } else {
        return Ok(());
    };
    Err(format!(
        "merge refused: {:?} and {:?}: {reason}",
        a.label, b.label
    ))
}

fn run_merge(inputs: Vec<MergeInput>, cancel: &AtomicBool) -> Result<DerivedSpectrum, String> {
    let template = inputs.first().ok_or("mark at least 2 spectra to merge")?;
    let mut params = template.params.for_materialized(0.0);
    let label = format!("{} · merge {}", template.target.label, inputs.len());
    let operation = Operation {
        tool: "merge".into(),
        parameters: serde_json::json!({ "template": template.target.label, "count": inputs.len() }),
        inputs: inputs.iter().map(|i| i.target.operation_input()).collect(),
        applied_energy_shift_ev: 0.0,
    };
    let mut seen = Vec::new();
    let mut acc: Option<StreamingAverage> = None;
    // Keep only the accumulator, one raw input, and scalar compatibility data.
    // Consume snapshots so materialized clones are freed as they are folded in.
    for input in inputs {
        if cancel.load(Ordering::Relaxed) {
            return Err("merge cancelled".into());
        }
        input.validate_quantity()?;
        let raw = load_group_raw_with_diagnostics(
            &input.target.path,
            &input.params,
            input.derived.as_ref(),
        )
        .map_err(|e| format!("merge refused: {:?}: {e}", input.target.label))?;
        let (energy, mu) = (raw.energy, raw.mu);
        let mut info = Compatibility::from_raw(&input, &energy, &mu)
            .map_err(|e| format!("merge refused: {:?}: {e}", input.target.label))?;
        info.declared_edge = raw.declared_edge;
        for previous in &seen {
            compatible(previous, &info)?;
        }
        info.validate_explicit_e0(input.params.e0)?;
        match &mut acc {
            Some(acc) => acc.add(&energy, &mu),
            None => {
                params.import.mode = info.mode;
                acc = Some(StreamingAverage::new(energy, mu));
            }
        }
        seen.push(info);
    }
    if cancel.load(Ordering::Relaxed) {
        return Err("merge cancelled".into());
    }
    let (energy, mu) = acc
        .ok_or("mark at least 2 spectra to merge")?
        .finish()
        .map_err(|e| format!("merge refused ({label}): {e}"))?;
    Ok(DerivedSpectrum {
        declared_edge: seen.first().and_then(|info| info.declared_edge.clone()),
        label,
        energy,
        mu,
        params: Some(params),
        quantity: Quantity::RawMu,
        operation: Some(operation),
        ..Default::default()
    })
}

fn keep_current(
    started: (Option<usize>, Option<&ToolTarget>, u64),
    now: (Option<usize>, Option<&ToolTarget>, u64),
) -> bool {
    started != now
}

pub(crate) struct MergeReview {
    targets: Vec<ToolTarget>,
    plot: Option<gpui::Entity<ruviz_gpui::RuvizPlot>>,
    error: Option<String>,
    ready: bool,
}

impl StudioApp {
    /// Check facts already available to the UI; disk reads and full validation
    /// stay in the merge worker. Unknown inputs can still be checked on click.
    pub(crate) fn merge_disabled_reason(&self) -> Option<String> {
        if self.merge_running {
            return Some("A merge is already running.".into());
        }
        let targets = match collect_inputs(&self.selection, |ix| self.tool_target(ix)) {
            Ok(targets) => targets,
            Err(reason) => return Some(reason),
        };
        let mut known = Vec::new();
        let mut modes: Vec<(String, DetectionMode)> = Vec::new();
        let mut edges: Vec<(String, crate::source_evidence::DeclaredEdge)> = Vec::new();
        for target in targets {
            let params = self.effective_params(target.ix);
            let derived = target
                .ix
                .checked_sub(DERIVED_BASE)
                .and_then(|ix| self.derived.get(ix));
            let quantity = derived.map_or(Quantity::RawMu, |d| d.quantity);
            if let Some(reason) = derived.and_then(DerivedSpectrum::processing_block_reason) {
                return Some(format!("{}: {reason}", target.label));
            }
            if !quantity.is_absorption() {
                return Some(format!(
                    "{} has quantity {}; Merge needs μ(E).",
                    target.label,
                    quantity.label()
                ));
            }
            let mode = self
                .raw_cache
                .peek(&(target.ix, params.raw_fingerprint()))
                .map(|raw| raw.channel)
                .or_else(|| {
                    self.saved_parser_record(target.ix)
                        .map(|record| record.channel)
                })
                .unwrap_or(params.import.mode);
            if mode != DetectionMode::Auto {
                if let Some((label, other)) = modes.iter().find(|(_, other)| {
                    (*other == DetectionMode::Reference) != (mode == DetectionMode::Reference)
                }) {
                    return Some(format!(
                        "Different channel kinds: {label} ({}) and {} ({}).",
                        other.label(),
                        target.label,
                        mode.label()
                    ));
                }
                modes.push((target.label.clone(), mode));
            }
            let edge = self.group_declared_edge(target.ix);
            if let Some(edge) = &edge {
                if let Some((label, other)) = edges.iter().find(|(_, other)| other != edge) {
                    return Some(format!(
                        "Different declared edges: {label} ({}) and {} ({}).",
                        other.label(),
                        target.label,
                        edge.label()
                    ));
                }
                edges.push((target.label.clone(), edge.clone()));
            }
            let spectrum = self
                .cache
                .peek(&(target.ix, target.fingerprint))
                .or_else(|| {
                    (self.spectrum_group.as_ref() == Some(&target)
                        && self.spectrum_fingerprint == target.fingerprint
                        && !self.load_running
                        && self.stale_plots.is_none())
                    .then_some(self.spectrum.as_ref())
                    .flatten()
                });
            if mode != DetectionMode::Auto
                && let Some(spectrum) = spectrum
                && let (Some(energy), Some(e0)) = (&spectrum.energy, spectrum.e0())
                && energy.len() >= 2
            {
                let info = Compatibility {
                    label: target.label,
                    quantity,
                    mode,
                    lo: energy[0],
                    hi: energy[energy.len() - 1],
                    e0,
                    declared_edge: edge,
                };
                for previous in &known {
                    if let Err(reason) = compatible(previous, &info) {
                        return Some(reason);
                    }
                }
                known.push(info);
            }
        }
        None
    }

    /// Find the original target source for Auto on a materialized tool result.
    /// Bound traversal also makes malformed/cyclic imported provenance harmless.
    fn merge_mode_source(&self, target: &ToolTarget) -> PathBuf {
        let mut path = target.path.clone();
        let mut id = target.derived_id;
        for _ in 0..=self.derived.len() {
            if !path.as_os_str().is_empty() {
                return path;
            }
            let Some(input) = id
                .and_then(|id| self.derived.iter().find(|d| d.id == id))
                .and_then(|d| d.operation.as_ref())
                .and_then(|op| op.inputs.first())
            else {
                break;
            };
            path = input.path.clone();
            id = input.derived_id;
        }
        path
    }

    pub(crate) fn open_merge_review(
        &mut self,
        event: &gpui::ClickEvent,
        window: &mut gpui::Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(reason) = self.merge_disabled_reason() {
            self.status = reason.into();
            cx.notify();
            return;
        }
        let mut targets = match collect_inputs(&self.selection, |ix| self.tool_target(ix)) {
            Ok(targets) => targets,
            Err(error) => {
                self.status = error.into();
                cx.notify();
                return;
            }
        };
        template_first(&mut targets, self.current_tool_target().as_ref());
        let inputs = self.merge_inputs(targets.clone());
        self.ui.merge_review = Some(MergeReview {
            targets: targets.clone(),
            plot: None,
            error: None,
            ready: false,
        });
        self.open_chrome_menu(shell::controls::Menu::Merge, event, window, cx);
        let theme = self.theme;
        let job = cx.background_executor().spawn(async move {
            let template = inputs.first().ok_or("No merge inputs")?;
            let raw = load_group_raw_with_diagnostics(
                &template.target.path,
                &template.params,
                template.derived.as_ref(),
            )?;
            let mut before = XASSpectrum::new();
            before.set_spectrum(raw.energy, raw.mu);
            let merged = run_merge(inputs, &AtomicBool::new(false))?;
            let mut after = XASSpectrum::new();
            after.set_spectrum(merged.energy, merged.mu);
            crate::plotting::build_tool_preview(&before, &after, None, false, &theme)
        });
        cx.spawn(async move |this, cx| {
            let result = job.await;
            this.update(cx, |app, cx| {
                if !app
                    .ui
                    .merge_review
                    .as_ref()
                    .is_some_and(|r| r.targets == targets)
                {
                    return;
                }
                let review = app.ui.merge_review.as_mut().unwrap();
                match result {
                    Ok(plot) => {
                        review.plot = Some(
                            ruviz_gpui::plot_builder(plot.size_px(780, 400))
                                .interactive()
                                .build(cx),
                        );
                        review.ready = true;
                    }
                    Err(error) => review.error = Some(error),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn merge_inputs(&self, targets: Vec<ToolTarget>) -> Vec<MergeInput> {
        targets
            .into_iter()
            .map(|target| MergeInput {
                params: self.effective_params(target.ix).clone(),
                derived: target
                    .ix
                    .checked_sub(DERIVED_BASE)
                    .and_then(|i| self.derived.get(i))
                    .cloned(),
                mode_source: self.merge_mode_source(&target),
                target,
            })
            .collect()
    }

    pub(crate) fn merge_review_panel(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let t = self.theme;
        let Some(review) = &self.ui.merge_review else {
            return div().into_any_element();
        };
        let valid = review
            .targets
            .iter()
            .all(|target| self.tool_target(target.ix).as_ref() == Some(target))
            && self.selection == review.targets.iter().map(|t| t.ix).collect();
        let ready = valid && review.ready && self.merge_disabled_reason().is_none();
        let mut inputs = div().id("merge-inputs").max_h(px(112.)).overflow_y_scroll();
        for (i, target) in review.targets.iter().enumerate() {
            inputs = inputs.child(div().text_size(px(11.5)).text_color(t.text_muted).child(
                format!(
                    "{}{}",
                    target.label,
                    if i == 0 {
                        " · grid & settings template"
                    } else {
                        ""
                    }
                ),
            ));
        }
        div()
            .flex()
            .flex_col()
            .gap_2()
            .p_2()
            .child(
                div()
                    .text_size(px(16.))
                    .child(format!("Merge {} → 1 new group", review.targets.len())),
            )
            .child(inputs)
            .when(valid, |d| {
                d.when_some(review.plot.clone(), |d, plot| {
                    d.child(div().h(px(280.)).min_w_0().child(plot))
                })
            })
            .when(!valid, |d| {
                d.child(
                    div()
                        .text_color(t.warn)
                        .child("Inputs changed. Close and reopen Merge."),
                )
            })
            .when(valid && !review.ready && review.error.is_none(), |d| {
                d.child("Preparing preview…")
            })
            .when_some(review.error.clone(), |d, error| {
                d.child(div().text_size(px(12.)).text_color(t.warn).child(error))
            })
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        shell::button(&t, "merge-review-apply", "Merge", ready)
                            .when(!ready, |d| d.opacity(0.45).cursor_default())
                            .on_click(cx.listener(move |app, _, window, cx| {
                                if ready {
                                    app.close_chrome_menu(window, cx);
                                    app.merge_selection(cx);
                                }
                            })),
                    )
                    .child(
                        shell::controls::icon_button(
                            &t,
                            "merge-review-close",
                            crate::icons::Icon::Close,
                            "Close merge preview",
                            false,
                        )
                        .on_click(
                            cx.listener(|app, _, window, cx| app.close_chrome_menu(window, cx)),
                        ),
                    ),
            )
            .into_any_element()
    }

    pub(super) fn merge_selection(&mut self, cx: &mut Context<Self>) {
        if let Some(reason) = self.merge_disabled_reason() {
            self.status = reason.into();
            cx.notify();
            return;
        }
        let current = self.current_tool_target();
        let mut targets = match collect_inputs(&self.selection, |ix| self.tool_target(ix)) {
            Ok(inputs) => inputs,
            Err(e) => {
                self.status = e.into();
                cx.notify();
                return;
            }
        };
        template_first(&mut targets, current.as_ref());
        let inputs = self.merge_inputs(targets);
        for input in &inputs {
            if let Err(e) = input.validate_quantity() {
                self.status = e.into();
                cx.notify();
                return;
            }
        }
        let selected = self.selected;
        let current_generation = self.generation;
        let catalog_gen = self.catalog_gen;
        if let Some(cancel) = self.merge_cancel.take() {
            cancel.store(true, Ordering::Relaxed);
        }
        self.merge_gen += 1;
        let generation = self.merge_gen;
        let cancel = Arc::new(AtomicBool::new(false));
        self.merge_cancel = Some(cancel.clone());
        self.job_inputs[2] = inputs
            .iter()
            .filter_map(|input| input.target.group_id.clone())
            .collect();
        self.merge_running = true;
        self.status = format!(
            "merging {} spectra · Grid and settings from {:?}",
            inputs.len(),
            inputs[0].target.label
        )
        .into();
        cx.notify();
        let job_cancel = cancel.clone();
        let job = cx
            .background_executor()
            .spawn(async move { run_merge(inputs, &job_cancel) });
        cx.spawn(async move |this, cx| {
            let result = job.await;
            this.update(cx, |app, cx| {
                if app.catalog_gen != catalog_gen || app.merge_gen != generation {
                    return;
                }
                app.merge_running = false;
                app.merge_cancel = None;
                match result {
                    Ok(mut merged) if !cancel.load(Ordering::Relaxed) => {
                        let base = merged.label.clone();
                        let mut suffix = 2;
                        while app.derived.iter().any(|d| d.label == merged.label) {
                            merged.label = format!("{base} ({suffix})");
                            suffix += 1;
                        }
                        merged.id = app.next_group_id();
                        merged.group_id = Some(crate::group_identity::GroupId::new_result());
                        let label = merged.label.clone();
                        app.record(
                            format!("merge → {label}"),
                            Some(shell::journal::UndoOp::DerivedAdd {
                                index: app.derived.len(),
                                spectrum: merged.clone(),
                            }),
                        );
                        app.derived.push(merged);
                        app.rekey_after_catalog_change();
                        if keep_current(
                            (selected, current.as_ref(), current_generation),
                            (
                                app.selected,
                                app.current_tool_target().as_ref(),
                                app.generation,
                            ),
                        ) {
                            app.status = format!("merged → {label} (in Results)").into();
                        } else {
                            app.select_entry(DERIVED_BASE + app.derived.len() - 1, cx);
                            app.status = format!("merged → {label}").into();
                        }
                    }
                    _ if cancel.load(Ordering::Relaxed) => app.status = "merge cancelled".into(),
                    Err(e) => {
                        app.status = e.clone().into();
                        app.record_job_error("merge", e);
                    }
                    Ok(_) => unreachable!("successful uncancelled merge handled above"),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target(ix: usize, label: &str) -> ToolTarget {
        ToolTarget {
            group_id: Some(crate::group_identity::GroupId::legacy_result(ix as u64)),
            ix,
            label: label.into(),
            path: PathBuf::new(),
            derived_id: ix.checked_sub(DERIVED_BASE).map(|i| i as u64 + 1),
            fingerprint: ix as u64,
            project_generation: 1,
            catalog_generation: 1,
            size: None,
        }
    }

    fn input(ix: usize, label: &str, value: f64) -> MergeInput {
        let mut params = PipelineParams {
            e0: Some(9000.0),
            ..Default::default()
        };
        params.import.mode = DetectionMode::Fluorescence;
        MergeInput {
            target: target(ix, label),
            params,
            derived: Some(DerivedSpectrum {
                label: label.into(),
                energy: (8800..=9200).map(f64::from).collect(),
                mu: (8800..=9200)
                    .map(|e| value + edge_mu(f64::from(e), 9000.))
                    .collect(),
                ..Default::default()
            }),
            mode_source: PathBuf::new(),
        }
    }

    fn edge_mu(energy: f64, e0: f64) -> f64 {
        1. / (1. + (-(energy - e0) / 5.).exp())
    }

    fn info(label: &str, e0: f64) -> Compatibility {
        Compatibility {
            label: label.into(),
            quantity: Quantity::RawMu,
            mode: DetectionMode::Fluorescence,
            lo: 8000.,
            hi: 9500.,
            e0,
            declared_edge: None,
        }
    }

    #[test]
    fn declared_edges_take_precedence_with_e0_fallback_when_missing() {
        use crate::source_evidence::DeclaredEdge;
        let edge = |element: &str| DeclaredEdge {
            element: element.into(),
            edge: "K".into(),
        };
        let mut a = info("first", 8979.);
        let mut b = info("second", 8980.);
        a.declared_edge = Some(edge("Cu"));
        b.declared_edge = Some(edge("Ni"));
        assert!(
            compatible(&a, &b)
                .unwrap_err()
                .contains("different declared edges")
        );
        b.declared_edge = Some(edge("Cu"));
        b.e0 = 9090.;
        compatible(&a, &b).unwrap();
        b.declared_edge = None;
        assert!(compatible(&a, &b).unwrap_err().contains("tolerance 50 eV"));
        b.e0 = a.e0;
        compatible(&a, &b).unwrap();
        b.quantity = Quantity::NormalizedDifference;
        assert!(compatible(&a, &b).is_err());
    }

    #[test]
    fn raw_xdi_reads_retain_declared_edge_for_merge() {
        let path = std::env::temp_dir()
            .join(crate::import_recipes::new_id("edge").replace(':', "-"))
            .with_extension("xdi");
        std::fs::write(&path, "# XDI/1.0\n# Element.symbol: cU\n# Element.edge: k\n# Column.1: energy eV\n# Column.2: mutrans\n# ---\n8900 1\n9000 2\n9100 3\n").unwrap();
        let raw = crate::params::load_mu_with_diagnostics(&path, &Default::default()).unwrap();
        assert_eq!(raw.declared_edge.unwrap().label(), "Cu K");
        assert_eq!(raw.channel, DetectionMode::MuColumn);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn materialized_merge_preserves_and_checks_declared_edges() {
        use crate::source_evidence::DeclaredEdge;
        for element in ["Ni", "Cu"] {
            let mut a = input(DERIVED_BASE, "declared Cu", 0.);
            let mut b = input(DERIVED_BASE + 1, "second", 0.1);
            a.derived.as_mut().unwrap().declared_edge = Some(DeclaredEdge {
                element: "Cu".into(),
                edge: "K".into(),
            });
            b.derived.as_mut().unwrap().declared_edge = Some(DeclaredEdge {
                element: element.into(),
                edge: "K".into(),
            });
            let result = run_merge(vec![a, b], &AtomicBool::new(false));
            if element == "Ni" {
                assert!(result.err().unwrap().contains("different declared edges"));
            } else {
                assert_eq!(result.unwrap().declared_edge.unwrap().label(), "Cu K");
            }
        }
    }

    #[test]
    fn merge_collection_includes_every_kind_and_preserves_marks() {
        let marks = BTreeSet::from([DERIVED_BASE + 2, 2, DERIVED_BASE, 0]);
        let before = marks.clone();
        let inputs = collect_inputs(&marks, |ix| Some(target(ix, "group"))).unwrap();
        assert_eq!(
            inputs.iter().map(|t| t.ix).collect::<Vec<_>>(),
            vec![DERIVED_BASE, DERIVED_BASE + 2, 0, 2]
        );
        assert_eq!(marks, before);
        assert!(
            collect_inputs(&marks, |ix| (ix != DERIVED_BASE)
                .then(|| target(ix, "group")))
            .unwrap_err()
            .contains(&DERIVED_BASE.to_string())
        );
        assert!(collect_inputs(&BTreeSet::from([0]), |ix| Some(target(ix, "one"))).is_err());
        assert!(
            collect_inputs(&BTreeSet::from([0, NO_ENTRY]), |ix| (ix != NO_ENTRY)
                .then(|| target(ix, "invalid")))
            .is_err()
        );
    }

    #[test]
    fn merge_uses_marks_independently_of_current() {
        let marks = BTreeSet::from([0, DERIVED_BASE]);
        for current in [0, 1] {
            let inputs = collect_inputs(&marks, |ix| Some(target(ix, "input"))).unwrap();
            assert_eq!(inputs.iter().map(|t| t.ix).collect::<BTreeSet<_>>(), marks);
            let compare = crate::app::group_rows::compare_set(Some(current), &marks);
            assert_eq!(compare.len(), if current == 0 { 2 } else { 3 });
        }
    }

    #[test]
    fn merge_template_is_bound_current_or_stable_first() {
        let mut inputs = vec![
            target(0, "first"),
            target(1, "second"),
            target(DERIVED_BASE, "aligned"),
        ];
        let current = inputs[2].clone();
        template_first(&mut inputs, Some(&current));
        assert_eq!(
            inputs.iter().map(|t| t.ix).collect::<Vec<_>>(),
            vec![DERIVED_BASE, 0, 1]
        );
        let before = inputs.clone();
        template_first(&mut inputs, Some(&target(99, "outside")));
        assert_eq!(inputs, before);
        let mut changed_identity = current.clone();
        changed_identity.derived_id = Some(99);
        template_first(&mut inputs, Some(&changed_identity));
        assert_eq!(inputs, before);
    }

    #[test]
    fn merge_mixed_input_fallback_uses_first_panel_group_grid_and_settings() {
        let path =
            std::env::temp_dir().join(format!("rexafs-merge-order-{}.dat", std::process::id()));
        let text: String = (8800..=9200)
            .map(|e| format!("{e} {}\n", edge_mu(f64::from(e), 9000.)))
            .collect();
        std::fs::write(&path, text).unwrap();
        let mut catalog = input(0, "catalog", 0.);
        catalog.derived = None;
        catalog.target.path = path.clone();
        catalog.params.import.mode = DetectionMode::MuColumn;
        catalog.params.import.mu_col = Some(1);
        catalog.params.pre_edge_start = Some(-100.);
        let mut derived = input(DERIVED_BASE, "aligned", 2.);
        derived.params.pre_edge_start = Some(-123.);
        let data = derived.derived.as_mut().unwrap();
        data.energy = (8850..=9150).step_by(2).map(f64::from).collect();
        data.mu = data
            .energy
            .iter()
            .map(|&e| 2. + edge_mu(e, 9000.))
            .collect();
        let expected_grid = data.energy.clone();
        let marks = BTreeSet::from([0, DERIVED_BASE]);
        let mut targets = collect_inputs(&marks, |ix| {
            Some(if ix == 0 {
                catalog.target.clone()
            } else {
                derived.target.clone()
            })
        })
        .unwrap();
        template_first(&mut targets, Some(&target(1, "outside")));
        assert_eq!(targets[0], derived.target);
        let mut sources = vec![catalog, derived];
        let inputs = targets
            .into_iter()
            .map(|target| {
                let pos = sources
                    .iter()
                    .position(|input| input.target == target)
                    .unwrap();
                sources.remove(pos)
            })
            .collect();
        let output = run_merge(inputs, &AtomicBool::new(false)).unwrap();
        assert_eq!(output.energy, expected_grid);
        assert_eq!(output.params.as_ref().unwrap().pre_edge_start, Some(-123.));
        assert_eq!(
            output.operation.as_ref().unwrap().parameters["template"],
            "aligned"
        );
        assert_eq!(marks, BTreeSet::from([0, DERIVED_BASE]));
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn merge_shared_explicit_e0_cannot_hide_different_detected_edges() {
        let mut cu = input(DERIVED_BASE, "cu_150k.xdi", 0.);
        let mut ni = input(DERIVED_BASE + 1, "ni_metal_rt.xdi", 0.);
        for (input, lo, hi, edge) in [(&mut cu, 8800, 9800, 8979.), (&mut ni, 8200, 9200, 8333.)] {
            input.params.e0 = Some(8979.);
            let data = input.derived.as_mut().unwrap();
            data.energy = (lo..=hi).map(f64::from).collect();
            data.mu = data.energy.iter().map(|&e| edge_mu(e, edge)).collect();
            let info = Compatibility::from_raw(
                input,
                &input.derived.as_ref().unwrap().energy,
                &input.derived.as_ref().unwrap().mu,
            )
            .unwrap();
            assert!((info.e0 - edge).abs() < 5.);
        }
        let error = run_merge(vec![cu, ni], &AtomicBool::new(false))
            .err()
            .unwrap();
        assert!(
            error.contains("cu_150k.xdi")
                && error.contains("ni_metal_rt.xdi")
                && error.contains("different edges"),
            "{error}"
        );
    }

    #[test]
    fn merge_explicit_e0_must_be_finite_in_range_and_near_detected_edge() {
        for e0 in [f64::NAN, 9300., 9060.] {
            let mut bad = input(DERIVED_BASE, "bad E0", 0.);
            bad.params.e0 = Some(e0);
            let error = run_merge(
                vec![bad, input(DERIVED_BASE + 1, "other", 0.)],
                &AtomicBool::new(false),
            )
            .err()
            .unwrap();
            assert!(
                error.contains("bad E0") && error.contains("explicit E0"),
                "{error}"
            );
        }
        let mut outside = info("outside", 9000.);
        outside.lo = 8990.;
        assert!(outside.validate_explicit_e0(Some(8980.)).is_err());
        assert!(outside.validate_explicit_e0(Some(9050.)).is_ok());
    }

    #[test]
    fn merge_accepts_repeated_energy_points_in_template_and_other_input() {
        let mut a = input(DERIVED_BASE, "repeated A", 2.);
        let mut b = input(DERIVED_BASE + 1, "repeated B", 4.);
        for input in [&mut a, &mut b] {
            let data = input.derived.as_mut().unwrap();
            data.energy.insert(100, data.energy[100]);
            data.mu.insert(100, data.mu[100]);
        }
        let expected_grid = a.derived.as_ref().unwrap().energy.clone();
        let output = run_merge(vec![a, b], &AtomicBool::new(false)).unwrap();
        assert_eq!(output.energy, expected_grid);
        for (&e, &mu) in output.energy.iter().zip(&output.mu) {
            assert!((mu - (3. + edge_mu(e, 9000.))).abs() < 1e-12);
        }
        let mut descending = input(DERIVED_BASE, "descending", 0.);
        let data = descending.derived.as_mut().unwrap();
        data.energy.swap(100, 101);
        assert!(
            Compatibility::from_raw(
                &descending,
                &descending.derived.as_ref().unwrap().energy,
                &descending.derived.as_ref().unwrap().mu
            )
            .is_err()
        );
    }

    #[test]
    fn merge_compatibility_names_both_edges_and_checks_tolerance() {
        let a = info("cu_150k.xdi", 8979.);
        let b = info("ni_metal_rt.xdi", 8333.);
        let error = compatible(&a, &b).unwrap_err();
        for name in ["cu_150k.xdi", "ni_metal_rt.xdi", "8979", "8333"] {
            assert!(error.contains(name));
        }
        assert!(compatible(&a, &info("boundary", 9029.)).is_ok());
        assert!(compatible(&a, &info("outside", 9029.01)).is_err());
    }

    #[test]
    fn merge_compatibility_requires_overlap_quantity_and_channel_kind() {
        let a = info("sample", 9000.);
        let mut b = info("other", 9000.);
        for mode in [
            DetectionMode::Transmission,
            DetectionMode::Fluorescence,
            DetectionMode::MuColumn,
        ] {
            b.mode = mode;
            assert!(compatible(&a, &b).is_ok());
        }
        b.mode = DetectionMode::Reference;
        assert!(compatible(&a, &b).unwrap_err().contains("channel kinds"));
        assert!(compatible(&b, &b).is_ok());
        b.mode = DetectionMode::Auto;
        assert!(compatible(&a, &b).is_err());
        b.mode = a.mode;
        for quantity in [
            Quantity::NormalizedMu,
            Quantity::NormalizedDifference,
            Quantity::ChiK,
        ] {
            b.quantity = quantity;
            assert!(compatible(&a, &b).is_err());
        }
        b.quantity = a.quantity;
        b.lo = a.hi;
        assert!(compatible(&a, &b).unwrap_err().contains("overlapping"));
    }

    #[test]
    fn merge_materialized_mean_uses_template_grid_settings_and_provenance() {
        let mut a = input(DERIVED_BASE, "aligned A", 2.);
        a.params.align_to_ref = true;
        a.params.align_target = Some(9001.);
        a.params.pre_edge_start = Some(-123.);
        let mut b = input(DERIVED_BASE + 1, "aligned B", 4.);
        let data = b.derived.as_mut().unwrap();
        data.energy = (8750..=9250).map(f64::from).collect();
        data.mu = data
            .energy
            .iter()
            .map(|&e| 4. + edge_mu(e, 9000.))
            .collect();
        let output = run_merge(vec![a, b], &AtomicBool::new(false)).unwrap();
        assert_eq!(
            output.energy,
            (8800..=9200).map(f64::from).collect::<Vec<_>>()
        );
        for (&e, &mu) in output.energy.iter().zip(&output.mu) {
            assert!((mu - (3. + edge_mu(e, 9000.))).abs() < 1e-12);
        }
        assert_eq!(output.label, "aligned A · merge 2");
        let params = output.params.as_ref().unwrap();
        assert_eq!(params.pre_edge_start, Some(-123.));
        assert_eq!(params.e0, Some(9000.));
        assert!(!params.align_to_ref);
        assert_eq!(params.align_target, None);
        let op = output.operation.as_ref().unwrap();
        assert_eq!(op.tool, "merge");
        assert_eq!(
            op.inputs.iter().map(|i| i.derived_id).collect::<Vec<_>>(),
            vec![Some(1), Some(2)]
        );
        assert_eq!(
            op.parameters,
            serde_json::json!({"template": "aligned A", "count": 2})
        );
        assert_eq!(op.applied_energy_shift_ev, 0.);
        assert_eq!(output.quantity, Quantity::RawMu);
        let restored: DerivedSpectrum =
            serde_json::from_value(serde_json::to_value(&output).unwrap()).unwrap();
        assert_eq!(restored.operation, output.operation);
        assert_eq!(
            restored.raw(restored.params.as_ref().unwrap()).unwrap(),
            (output.energy, output.mu)
        );
    }

    #[test]
    fn merge_refuses_non_template_edge_pair_and_blocked_or_invalid_inputs() {
        let a = input(DERIVED_BASE, "middle", 1.);
        let mut b = input(DERIVED_BASE + 1, "low", 2.);
        let mut c = input(DERIVED_BASE + 2, "high", 3.);
        b.params.e0 = Some(8970.);
        c.params.e0 = Some(9030.);
        for (input, shift) in [(&mut b, -30.), (&mut c, 30.)] {
            for e in &mut input.derived.as_mut().unwrap().energy {
                *e += shift;
            }
        }
        let error = run_merge(vec![a, b, c], &AtomicBool::new(false))
            .err()
            .unwrap();
        assert!(error.contains("low") && error.contains("high"));
        for case in 0..5 {
            let mut bad = input(DERIVED_BASE + 1, "bad", 2.);
            let d = bad.derived.as_mut().unwrap();
            match case {
                0 => d.quantity_unconfirmed = true,
                1 => d.quantity = Quantity::NormalizedDifference,
                2 => d.mu.clear(),
                3 => d.energy[1] = f64::NAN,
                _ => bad.params.e0 = Some(f64::NAN),
            }
            let error = run_merge(
                vec![input(DERIVED_BASE, "good", 1.), bad],
                &AtomicBool::new(false),
            )
            .err()
            .unwrap();
            assert!(error.contains("bad"), "{error}");
        }
        assert!(
            run_merge(
                vec![input(DERIVED_BASE, "cancelled", 1.)],
                &AtomicBool::new(true)
            )
            .err()
            .unwrap()
            .contains("cancelled")
        );
    }

    #[test]
    fn merge_uncached_catalog_and_channel_resolve_auto_and_detect_e0() {
        let path = std::env::temp_dir().join(format!("rexafs-merge-{}.dat", std::process::id()));
        let mut text = "# energy i0 roi1 it ir\n".to_string();
        for e in 8800..=9200 {
            let mu = 1. / (1. + (-((e as f64) - 9000.) / 5.).exp());
            text.push_str(&format!(
                "{e} 1 {mu} {} {}\n",
                (-mu).exp(),
                (-2. * mu).exp()
            ));
        }
        std::fs::write(&path, text).unwrap();
        let mut a = input(0, "catalog", 0.);
        a.derived = None;
        a.target.path = path.clone();
        a.mode_source = path.clone();
        a.params.e0 = None;
        a.params.import.mode = DetectionMode::Auto;
        let mut b = input(DERIVED_BASE, "fluorescence channel", 0.);
        b.params.e0 = None;
        b.derived.as_mut().unwrap().source = Some(path.clone());
        let output = run_merge(vec![a, b], &AtomicBool::new(false)).unwrap();
        assert_eq!(output.energy.len(), 401);
        assert!((output.mu[200] - 0.5).abs() < 1e-12);
        assert_eq!(
            output.params.as_ref().unwrap().import.mode,
            DetectionMode::Transmission
        );
        assert_eq!(output.quantity, Quantity::RawMu);
        let mut sample = input(DERIVED_BASE, "sample", 1.);
        sample.params.import.mode = DetectionMode::Auto;
        sample.mode_source = path.clone();
        let mut reference = input(DERIVED_BASE + 1, "reference", 2.);
        reference.params.import.mode = DetectionMode::Reference;
        let error = run_merge(vec![sample, reference], &AtomicBool::new(false))
            .err()
            .unwrap();
        assert!(
            error.contains("sample")
                && error.contains("reference")
                && error.contains("channel kinds")
        );
        std::fs::remove_file(&path).unwrap();
        let mut missing = input(0, "missing file", 1.);
        missing.derived = None;
        missing.target.path = path;
        assert!(
            run_merge(
                vec![missing, input(DERIVED_BASE, "other", 1.)],
                &AtomicBool::new(false)
            )
            .err()
            .unwrap()
            .contains("missing file")
        );
    }

    #[test]
    fn merge_current_guard_detects_navigation_revision_and_return_navigation() {
        let a = target(0, "A");
        let b = target(1, "B");
        assert!(!keep_current(
            (Some(0), Some(&a), 1),
            (Some(0), Some(&a), 1)
        ));
        assert!(keep_current((Some(0), Some(&a), 1), (Some(1), Some(&b), 2)));
        assert!(keep_current((Some(0), Some(&a), 1), (Some(0), Some(&a), 3)));
        let mut revised = a.clone();
        revised.fingerprint += 1;
        assert!(keep_current(
            (Some(0), Some(&a), 1),
            (Some(0), Some(&revised), 1)
        ));
    }
}
