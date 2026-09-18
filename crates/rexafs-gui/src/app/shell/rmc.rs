//! RMC mode in the existing Fitting workspace.
use super::{
    button, chip,
    controls::Menu,
    fit_workspace::FitStep,
    molecule_view::{MoleculeScene, SceneAtom},
    section_label,
};
use crate::{
    app::StudioApp,
    rmc_fitting::{self as engine, Event, FitMode, Progress, Request, Source},
    widgets::{
        numeric_field::{FieldEvent, FieldKind, FieldPreview, NumericField},
        text_input::{InputEvent, TextInput},
    },
};
use gpui::{Context, Entity, IntoElement, ParentElement, Styled, div, prelude::*, px};
use rexafs::{
    rmc::{Configuration, ResidualTrendStatus},
    structure::{Edge, Element},
};
use ruviz::prelude::Plot;
use ruviz_gpui::{RuvizPlot, plot_builder};
use std::{path::PathBuf, sync::Arc, time::Duration};

#[derive(Default)]
pub(crate) struct RmcState {
    pub project: engine::Project,
    pub control: Option<engine::Control>,
    pub live: Option<Progress>,
    pub request: Option<Request>,
    pub phase: String,
    pub error: Option<String>,
    pub generation: u64,
    pub recovery: Option<PathBuf>,
    builder_generation: u64,
    builder_busy: bool,
    builder_error: Option<String>,
    fields: Vec<Entity<NumericField>>,
    texts: Vec<Entity<TextInput>>,
    plots: Vec<Entity<RuvizPlot>>,
    plotted_best: Option<u64>,
    plot_tab: usize,
    fit_plot_view: usize,
    hide_initial_curve: bool,
    full_history: bool,
    advanced: bool,
    continuation: Option<Entity<NumericField>>,
    show_initial: bool,
    scene_key: Option<(usize, u64, bool)>,
    page: Option<FitStep>,
    path_page: Option<FitStep>,
}
impl RmcState {
    pub(crate) fn from_project(mut project: engine::Project) -> Self {
        let error = project
            .saved
            .as_ref()
            .and_then(|saved| saved.validate().err());
        if error.is_some() {
            project.saved = None;
        }
        let builder_error = if let Some(structure) = &project.structure {
            match engine::build_preview(structure, project.draft.repeats) {
                Ok(configuration) => {
                    project.configuration = Some(configuration);
                    None
                }
                Err(e) => {
                    project.configuration = None;
                    Some(e)
                }
            }
        } else {
            None
        };
        Self {
            project,
            error,
            builder_error,
            ..Default::default()
        }
    }
}
impl StudioApp {
    pub(super) fn fit_mode_picker(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        button(
            &self.theme,
            "fit-mode-picker",
            format!("Fit mode: {} ▾", self.fit_mode.label()),
            true,
        )
        .on_click(cx.listener(|app, event, window, cx| {
            app.open_chrome_menu(Menu::FitMode, event, window, cx)
        }))
    }
    pub(super) fn fit_mode_menu(&self, cx: &mut Context<Self>) -> gpui::Div {
        let mut out = div().flex().flex_col().gap_2().p_2();
        for (i, mode) in FitMode::ALL.into_iter().enumerate() {
            out = out.child(
                chip(
                    &self.theme,
                    ("fit-mode", i),
                    mode.label(),
                    self.fit_mode == mode,
                )
                .on_click(cx.listener(move |app, _, window, cx| {
                    app.set_fit_mode(mode, cx);
                    app.close_chrome_menu(window, cx);
                })),
            );
        }
        out
    }
    pub(crate) fn set_fit_mode(&mut self, mode: FitMode, cx: &mut Context<Self>) {
        if mode == self.fit_mode {
            return;
        }
        self.rmc.scene_key = None;
        if mode == FitMode::Rmc {
            if !self.rmc.project.spectrum_defaults_applied
                && self.rmc.project.saved.is_none()
                && self.spectrum.is_some()
            {
                self.use_rmc_spectrum_ranges(cx);
            }
            self.rmc.path_page = Some(self.stage_view.fit_step);
            self.stage_view.fit_step = self.rmc.page.unwrap_or(FitStep::Structure);
        } else {
            self.rmc.page = Some(self.stage_view.fit_step);
            self.stage_view.fit_step = self.rmc.path_page.unwrap_or(FitStep::Model);
            self.refresh_structure(cx);
            self.rebuild_structure_plot(cx);
        }
        self.fit_mode = mode;
        cx.notify();
    }
    pub(super) fn rmc_job_bar(&self, cx: &mut Context<Self>) -> gpui::Div {
        if self.rmc.control.is_none() {
            return div();
        }
        let paused = self.rmc.phase == "Paused";
        let pending =
            self.rmc.phase.starts_with("Pausing") || self.rmc.phase.starts_with("Stopping");
        let mut bar = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_2()
            .px_3()
            .py_2()
            .bg(self.theme.raised)
            .child(div().flex_1().min_w_0().flex().flex_col().gap_1()
                .child(format!("RMC · {}", self.rmc.phase))
                .child(div().text_size(px(11.)).text_color(self.theme.text_muted).child(
                    if let Some(p) = &self.rmc.live {
                        format!("{} / {} attempts · {:.1}% of budget · {}", count(p.completed), count(p.limit),
                            100. * p.completed as f64 / p.limit.max(1) as f64,
                            if paused { "Calculator retained for a fast resume" } else { "Updates after completed moves" })
                    } else {
                        "Preparing potentials and exact scattering paths. This may take several minutes; you can stop safely.".into()
                    })));
        if paused {
            bar = bar.child(
                button(
                    &self.theme,
                    "rmc-resume-live",
                    if self.rmc.live.as_ref().is_some_and(|p| p.completed == 0) {
                        "Start optimization"
                    } else {
                        "Resume"
                    },
                    true,
                )
                .on_click(cx.listener(|app, _, _, cx| app.resume_rmc(cx))),
            );
        } else {
            bar = bar.child(
                button(&self.theme, "rmc-pause", "Pause", false)
                    .disabled(pending)
                    .when(pending, |d| d.opacity(0.45).cursor_default())
                    .on_click(cx.listener(|app, _, _, cx| {
                        if let Some(c) = &app.rmc.control {
                            c.pause();
                            app.rmc.phase = "Pausing after current move".into();
                        }
                        cx.notify();
                    })),
            );
        }
        bar = bar.child(
            button(&self.theme, "rmc-stop", "Stop and save", false)
                .disabled(self.rmc.phase.starts_with("Stopping"))
                .when(self.rmc.phase.starts_with("Stopping"), |d| {
                    d.opacity(0.45).cursor_default()
                })
                .on_click(cx.listener(|app, _, _, cx| {
                    if let Some(c) = &app.rmc.control {
                        c.stop();
                        app.rmc.phase = "Stopping and saving".into();
                    }
                    cx.notify();
                })),
        );
        let mut out = div().flex().flex_col().child(bar);
        if let Some(p) = &self.rmc.live {
            out = out.child(
                div().h(px(3.)).bg(self.theme.border).child(
                    div()
                        .h_full()
                        .w(gpui::relative(
                            (p.completed as f32 / p.limit.max(1) as f32).clamp(0., 1.),
                        ))
                        .bg(self.theme.accent),
                ),
            );
        }
        out
    }
    fn rmc_source(&self) -> Source {
        Source {
            group_id: self.current_group_index().and_then(|ix| self.group_id(ix)),
            label: self.spectrum_label.to_string(),
            path: self.spectrum_path.clone(),
            fingerprint: self.spectrum_fingerprint,
            recipe: self.ui_params().clone(),
        }
    }
    fn rmc_source_stale(&self) -> bool {
        self.rmc.request.as_ref().is_some_and(|r| {
            r.source.group_id != self.current_group_index().and_then(|ix| self.group_id(ix))
                || r.source.fingerprint != self.spectrum_fingerprint
                || r.source.path != self.spectrum_path
        })
    }
    fn new_rmc_request(&self) -> Result<Request, String> {
        if self.fit_running || self.feff_running || self.batch_running {
            return Err("Wait for the active fit or scattering calculation to finish.".into());
        }
        if self.load_running || self.stale_plots.is_some() {
            return Err("Wait for successful spectrum processing.".into());
        }
        if !cfg!(feature = "refeff-runner") {
            return Err("RMC requires a desktop build with ReFEFF.".into());
        }
        let spectrum = self
            .spectrum
            .as_ref()
            .ok_or("Select a processed spectrum.")?;
        let config = self
            .rmc
            .project
            .configuration
            .clone()
            .ok_or("Pick a structure and build its supercell first.")?;
        Request::new(spectrum, config, &self.rmc.project.draft, self.rmc_source())
    }
    fn begin_rmc(
        &mut self,
        prepare_only: bool,
        resume: Option<Box<engine::SavedRun>>,
        cx: &mut Context<Self>,
    ) {
        if self.fit_running || self.feff_running || self.batch_running {
            self.rmc.error =
                Some("Wait for the active fit or scattering calculation to finish.".into());
            cx.notify();
            return;
        }
        if self.rmc.control.is_some() {
            self.rmc.error =
                Some("Stop and save the current RMC run before starting another.".into());
            return;
        }
        if resume.is_none() {
            if self.rmc.builder_busy || self.rmc.builder_error.is_some() {
                self.rmc.error = Some(
                    "Wait for a valid supercell preview before preparing or running RMC.".into(),
                );
                cx.notify();
                return;
            }
            if let Err(e) = self.commit_rmc_fields(cx) {
                self.rmc.error = Some(e);
                cx.notify();
                return;
            }
        }
        let request = match &resume {
            Some(saved) => saved.validate().map(|()| saved.request.clone()),
            None => self.new_rmc_request(),
        };
        let request = match request {
            Ok(v) => v,
            Err(e) => {
                self.rmc.error = Some(e);
                cx.notify();
                return;
            }
        };
        let Some(root) = crate::settings::app_dir() else {
            self.rmc.error = Some("Cannot create the RMC recovery directory.".into());
            return;
        };
        let dir = match tempfile::Builder::new().prefix("run-").tempdir_in({
            let path = root.join("rmc");
            if let Err(e) = std::fs::create_dir_all(&path) {
                self.rmc.error = Some(e.to_string());
                return;
            }
            path
        }) {
            Ok(dir) => dir.keep(),
            Err(e) => {
                self.rmc.error = Some(e.to_string());
                return;
            }
        };
        let recovery = dir.join("checkpoint.json");
        let (control, rx) = engine::spawn(request.clone(), resume, recovery.clone(), prepare_only);
        self.rmc.generation += 1;
        let generation = self.rmc.generation;
        let project = self.project_generation;
        self.rmc.control = Some(control);
        self.rmc.request = Some(request);
        self.rmc.recovery = Some(recovery);
        self.rmc.error = None;
        self.rmc.live = None;
        self.rmc.project.saved = None;
        self.rmc.continuation = None;
        self.rmc.plots.clear();
        self.rmc.scene_key = None;
        self.rmc.phase = "Preparing exact ReFEFF".into();
        self.stage_view.fit_step = FitStep::Results;
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(250))
                    .await;
                let mut disconnected = false;
                let mut messages = Vec::new();
                loop {
                    match rx.try_recv() {
                        Ok(m) => messages.push(m),
                        Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                            disconnected = true;
                            break;
                        }
                        Err(_) => break,
                    }
                }
                let keep = this
                    .update(cx, |app, cx| {
                        if app.rmc.generation != generation || app.project_generation != project {
                            return false;
                        }
                        for message in messages {
                            match message {
                                Event::Progress(p) => {
                                    app.rmc.live = Some(*p);
                                    if !app.rmc.phase.starts_with("Pausing")
                                        && !app.rmc.phase.starts_with("Stopping")
                                    {
                                        app.rmc.phase = "Running".into();
                                    }
                                    app.rebuild_rmc_plots(cx);
                                }
                                Event::Saved(saved) => {
                                    app.rmc.live = Some(saved.progress.clone());
                                    app.rmc.project.saved = Some(saved);
                                    app.rebuild_rmc_plots(cx);
                                }
                                Event::Paused => {
                                    app.rmc.phase = "Paused".into();
                                    app.rebuild_rmc_plots(cx);
                                }
                                Event::Finished(saved) => {
                                    app.rmc.phase = saved.status.clone();
                                    app.rmc.live = Some(saved.progress.clone());
                                    app.rmc.project.saved = Some(saved);
                                    app.rmc.control = None;
                                    app.rebuild_rmc_plots(cx);
                                    if app.fit_mode == FitMode::Rmc {
                                        app.stage_view.fit_step = FitStep::Results;
                                    }
                                }
                                Event::Error(e) => {
                                    app.rmc.error = Some(e);
                                    app.rmc.phase = "Stopped with error".into();
                                    app.rmc.control = None;
                                }
                            }
                        }
                        if disconnected {
                            if app.rmc.control.is_some() {
                                app.rmc.phase = "Worker stopped".into();
                                app.rmc.error = Some("The RMC worker ended unexpectedly. The latest saved checkpoint remains available for recovery.".into());
                            }
                            app.rmc.control = None;
                        }
                        cx.notify();
                        !disconnected
                    })
                    .unwrap_or(false);
                if !keep {
                    break;
                }
            }
        })
        .detach();
        cx.notify();
    }
    fn resume_rmc(&mut self, cx: &mut Context<Self>) {
        let Some(p) = self.rmc.live.as_ref() else {
            return;
        };
        let additional = self
            .rmc
            .continuation
            .as_ref()
            .map_or(Ok(Some(10_000.)), |field| field.read(cx).pending_value(cx));
        let total = if p.completed < p.limit {
            Ok(p.limit)
        } else {
            additional
                .map_err(|_| "Enter a valid number of additional attempts.".to_string())
                .and_then(|v| {
                    v.ok_or_else(|| "Enter the number of additional attempts.".to_string())
                })
                .and_then(|v| engine::continuation_limit(p.completed, p.limit, v as usize))
        };
        let total = match total {
            Ok(total) => total,
            Err(e) => {
                self.rmc.error = Some(e);
                cx.notify();
                return;
            }
        };
        self.rmc.error = None;
        if let Some(c) = &self.rmc.control {
            c.resume(total);
            self.rmc.phase = "Running".into();
            cx.notify();
            return;
        }
        let Some(mut saved) = self.rmc.project.saved.clone() else {
            self.rmc.error = Some("Load a saved RMC checkpoint first.".into());
            cx.notify();
            return;
        };
        saved.request.settings.moves.steps = total;
        self.begin_rmc(false, Some(saved), cx);
    }
    fn rmc_checkpoint_dialog(&mut self, cx: &mut Context<Self>) {
        let project = self.project_generation;
        let prompt = cx.prompt_for_paths(gpui::PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: None,
        });
        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(paths))) = prompt.await
                && let Some(path) = paths.first()
            {
                this.update(cx, |app, cx| {
                    if app.project_generation == project {
                        app.load_rmc_checkpoint(path.clone(), cx);
                    }
                })
                .ok();
            }
        })
        .detach();
    }
    fn recover_latest_rmc(&mut self, cx: &mut Context<Self>) {
        let Some(root) = crate::settings::app_dir() else {
            return;
        };
        match engine::latest_recovery(&root.join("rmc")) {
            Ok(path) => self.load_rmc_checkpoint(path, cx),
            Err(e) => {
                self.rmc.error = Some(e);
                cx.notify();
            }
        }
    }
    fn load_rmc_checkpoint(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        if self.rmc.control.is_some() {
            self.rmc.error =
                Some("Stop and save the active RMC run before loading a checkpoint.".into());
            cx.notify();
            return;
        }
        self.rmc.generation += 1; // Ignore an older asynchronous file selection.
        let project = self.project_generation;
        let generation = self.rmc.generation;
        cx.spawn(async move |this, cx| {
            let work_path = path.clone();
            let result = cx
                .background_executor()
                .spawn(async move { engine::load_run(&work_path) })
                .await;
            this.update(cx, |app, cx| {
                if app.project_generation != project
                    || app.rmc.generation != generation
                    || app.rmc.control.is_some()
                {
                    return;
                }
                match result {
                    Ok(saved) => {
                        app.rmc.request = Some(saved.request.clone());
                        app.rmc.live = Some(saved.progress.clone());
                        app.rmc.phase = "Saved run · ready to resume".into();
                        app.rmc.project.saved = Some(Box::new(saved));
                        app.rmc.recovery = Some(path);
                        app.rmc.error = None;
                        app.rmc.continuation = None;
                        app.rmc.plots.clear();
                        app.rmc.plotted_best = None;
                        app.rebuild_rmc_plots(cx);
                        app.stage_view.fit_step = FitStep::Results;
                    }
                    Err(e) => app.rmc.error = Some(e),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }
    fn use_rmc_spectrum_ranges(&mut self, cx: &mut Context<Self>) {
        let recipe = self.ui_params().clone();
        let result = self
            .spectrum
            .as_ref()
            .ok_or_else(|| "Prepare a spectrum in Background first.".to_string())
            .and_then(|spectrum| {
                self.rmc
                    .project
                    .draft
                    .use_spectrum_ranges(spectrum, &recipe)
            });
        match result {
            Ok(()) => {
                self.rmc.project.spectrum_defaults_applied = true;
                self.rmc.fields.clear();
                self.rmc.texts.clear();
                self.rmc.error = None;
            }
            Err(e) => self.rmc.error = Some(e),
        }
        cx.notify();
    }
    fn suggest_rmc_supercell(&mut self, cx: &mut Context<Self>) {
        let Some(structure) = &self.rmc.project.structure else {
            return;
        };
        let read = |index: usize, fallback: f64| -> Result<f64, String> {
            self.rmc.fields.get(index).map_or(Ok(fallback), |f| {
                f.read(cx)
                    .pending_value(cx)
                    .map_err(|_| "Enter a valid cluster radius and displacement.".to_string())?
                    .ok_or_else(|| "Enter an explicit cluster radius and displacement.".to_string())
            })
        };
        let values = read(3, self.rmc.project.draft.options.cluster_radius).and_then(|radius| {
            read(6, self.rmc.project.draft.max_displacement)
                .map(|displacement| (radius, displacement))
        });
        let (radius, displacement) = match values {
            Ok((radius, displacement)) if radius > 0.5 && radius < 50. && displacement > 0. => {
                (radius, displacement)
            }
            _ => {
                self.rmc.builder_error = Some(
                    "Set a cluster radius between 0.5 and 50 Å and a positive displacement.".into(),
                );
                cx.notify();
                return;
            }
        };
        self.rmc.project.draft.options.cluster_radius = radius;
        self.rmc.project.draft.max_displacement = displacement;
        let draft = &self.rmc.project.draft;
        match rexafs::rmc::suggested_supercell_repeats(
            structure,
            2. * (draft.options.cluster_radius + draft.max_displacement),
        ) {
            Ok(repeats) => {
                self.rmc.project.draft.repeats = repeats;
                for (i, value) in repeats.into_iter().enumerate() {
                    if let Some(field) = self.rmc.fields.get(i) {
                        field.update(cx, |f, cx| f.set_value(Some(value as f64), cx));
                    }
                }
                self.build_rmc_supercell(cx);
            }
            Err(e) => {
                self.rmc.builder_error = Some(e.to_string());
                cx.notify();
            }
        }
    }
    /// Debounce only the geometry preview. It never prepares scattering, and a
    /// generation token prevents an older build replacing a newer draft/project.
    fn build_rmc_supercell(&mut self, cx: &mut Context<Self>) {
        self.rmc.builder_generation += 1;
        let generation = self.rmc.builder_generation;
        let project = self.project_generation;
        let result = (|| -> Result<_, String> {
            let structure = self
                .rmc
                .project
                .structure
                .clone()
                .ok_or("Choose a structure first.")?;
            let mut repeats = self.rmc.project.draft.repeats;
            for (i, field) in self.rmc.fields.iter().take(3).enumerate() {
                let value = field
                    .read(cx)
                    .pending_value(cx)
                    .map_err(|_| "Enter positive integer repeat counts.")?
                    .ok_or("Enter all three repeat counts.")?;
                if !(1. ..=10_000.).contains(&value) {
                    return Err("Repeat counts must be between 1 and 10,000.".into());
                }
                repeats[i] = value as usize;
            }
            Ok((structure, repeats))
        })();
        let (structure, repeats) = match result {
            Ok(value) => value,
            Err(e) => {
                self.rmc.builder_busy = false;
                self.rmc.builder_error = Some(e);
                cx.notify();
                return;
            }
        };
        self.rmc.builder_busy = true;
        self.rmc.builder_error = None;
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(150))
                .await;
            if !this
                .update(cx, |app, _| {
                    app.project_generation == project && app.rmc.builder_generation == generation
                })
                .unwrap_or(false)
            {
                return;
            }
            let result = cx
                .background_executor()
                .spawn(async move { engine::build_preview(&structure, repeats) })
                .await;
            this.update(cx, |app, cx| {
                if app.project_generation != project || app.rmc.builder_generation != generation {
                    return;
                }
                app.rmc.builder_busy = false;
                match result {
                    Ok(configuration) => {
                        app.rmc.project.configuration = Some(configuration);
                        app.rmc.project.draft.repeats = repeats;
                        app.rmc.scene_key = None;
                    }
                    Err(e) => app.rmc.builder_error = Some(e),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }
    fn use_rmc_structure(&mut self, cx: &mut Context<Self>) {
        let Some(summary) = self.structure.summary.clone() else {
            return;
        };
        if !self.rmc.project.spectrum_defaults_applied && self.spectrum.is_some() {
            self.use_rmc_spectrum_ranges(cx);
        }
        self.rmc.fields.clear();
        self.rmc.texts.clear();
        self.rmc.project.configuration = None;
        self.rmc.project.structure = Some((*summary.structure).clone());
        self.rmc.project.label = summary.structure.formula();
        if let Some(z) = self
            .structure
            .absorber
            .as_ref()
            .and_then(|s| Element::from_symbol(s))
            .map(|e| e.z)
        {
            self.rmc.project.draft.absorber = z;
        }
        self.suggest_rmc_supercell(cx);
        self.stage_view.fit_step = FitStep::Calculate;
    }
    pub(super) fn rmc_workspace(&mut self, cx: &mut Context<Self>) -> gpui::Div {
        let t = self.theme;
        let step = self.stage_view.fit_step;
        let mut nav = div().flex().flex_wrap().gap_2().px_3().py_2();
        for (i, (dest, label)) in [
            (FitStep::Structure, "1 · Structure"),
            (FitStep::Calculate, "2 · Supercell"),
            (FitStep::Model, "3 · Fit settings"),
            (FitStep::Results, "4 · Results"),
        ]
        .into_iter()
        .enumerate()
        {
            nav = nav.child(
                chip(&t, ("rmc-page", i), label, step == dest).on_click(cx.listener(
                    move |app, _, _, cx| {
                        app.stage_view.fit_step = dest;
                        app.rmc.scene_key = None;
                        cx.notify();
                    },
                )),
            );
        }
        nav = nav
            .child(div().flex_1())
            .child(
                button(&t, "rmc-recover-latest", "Recover latest run", false)
                    .disabled(self.rmc.control.is_some())
                    .when(self.rmc.control.is_some(), |d| {
                        d.opacity(0.45).cursor_default()
                    })
                    .on_click(cx.listener(|app, _, _, cx| app.recover_latest_rmc(cx))),
            )
            .child(self.fit_mode_picker(cx));
        let mut out = div()
            .flex_1()
            .min_h_0()
            .min_w_0()
            .flex()
            .flex_col()
            .child(nav)
            .child(self.rmc_job_bar(cx));
        if let Some(error) = &self.rmc.error {
            out = out.child(
                div()
                    .px_3()
                    .py_2()
                    .flex()
                    .items_center()
                    .gap_2()
                    .bg(t.raised)
                    .child(div().flex_1().text_color(t.warn).child(error.clone()))
                    .child(button(&t, "rmc-dismiss-error", "Dismiss", false).on_click(
                        cx.listener(|app, _, _, cx| {
                            app.rmc.error = None;
                            cx.notify();
                        }),
                    )),
            );
        }
        if step != FitStep::Results {
            out = out.child(
                div()
                    .px_3()
                    .py_2()
                    .text_size(px(12.))
                    .text_color(t.text_muted)
                    .child(if self.spectrum.is_some() {
                        format!(
                            "Spectrum: {} · R-space real + imaginary · exact ReFEFF",
                            self.spectrum_label
                        )
                    } else {
                        "Select and process a spectrum in Background to begin.".into()
                    }),
            );
            if self.rmc.control.is_some() {
                out = out.child(div().px_3().py_2().text_color(t.warn)
                    .child("A run is active. These settings are for the next run; use Results to pause, resume or stop the current run."));
            }
        }
        if self.rmc_source_stale() {
            out=out.child(div().px_3().py_2().text_color(t.warn).child("Showing the saved run's inputs. The active spectrum or preprocessing differs; Resume continues the saved problem."));
        }
        if step == FitStep::Structure {
            let library = self.structure_library_panel(cx).into_any_element();
            let viewer = self.structure_center(cx).into_any_element();
            return out.child(
                div()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .gap_2()
                    .child(
                        div()
                            .id("rmc-library")
                            .w(px(350.))
                            .min_h_0()
                            .overflow_y_scroll()
                            .child(library)
                            .child(
                                button(
                                    &t,
                                    "rmc-use-structure",
                                    "Use structure → Supercell",
                                    self.structure.summary.is_some(),
                                )
                                .disabled(
                                    self.structure.summary.is_none()
                                        || self.structure.fetch_running,
                                )
                                .on_click(cx.listener(|app, _, _, cx| app.use_rmc_structure(cx))),
                            )
                            .child(
                                button(&t, "rmc-load-checkpoint", "Open RMC checkpoint…", false)
                                    .on_click(
                                        cx.listener(|app, _, _, cx| app.rmc_checkpoint_dialog(cx)),
                                    ),
                            ),
                    )
                    .child(viewer),
            );
        }
        if matches!(step, FitStep::Calculate | FitStep::Model) {
            self.ensure_rmc_fields(cx);
            let is_cell = step == FitStep::Calculate;
            let mut panel = div()
                .id("rmc-settings")
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .p_3()
                .flex()
                .flex_col()
                .gap_3();
            if is_cell {
                panel = panel.child(section_label(&t, "Periodic supercell"))
                    .child(hint(&t, "Edit the repeats to update the 3D preview. Scattering starts only when you prepare or run."));
                for i in 0..3 {
                    panel = panel.child(self.rmc.fields[i].clone());
                }
                panel = panel.child(
                    button(&t, "rmc-supercell", "Use suggested size", false)
                        .on_click(cx.listener(|app, _, _, cx| app.suggest_rmc_supercell(cx))),
                );
                if self.rmc.builder_busy {
                    panel = panel.child(hint(&t, "Updating preview…"));
                }
                if let Some(e) = &self.rmc.builder_error {
                    panel = panel.child(
                        div()
                            .text_color(t.warn)
                            .child(format!("{e} Showing the last valid preview.")),
                    );
                }
                if let Some(c) = &self.rmc.project.configuration {
                    let absorbers =
                        engine::selected_absorbers(c, &self.rmc.project.draft).map(|v| v.len());
                    if let Err(e) = &absorbers {
                        panel = panel.child(div().text_color(t.warn).child(e.clone()));
                    }
                    panel = panel.child(div().rounded_md().p_2().bg(t.raised).child(format!(
                        "{} atoms · {} absorbing sites",
                        count(c.atoms.len()),
                        count(absorbers.unwrap_or(0))
                    )));
                    let elements: std::collections::BTreeSet<_> =
                        c.atoms.iter().map(|a| a.atomic_number).collect();
                    let mut row = div()
                        .flex()
                        .flex_wrap()
                        .items_center()
                        .gap_2()
                        .child("Absorber");
                    for z in elements {
                        let label = Element::from_z(z).map_or("?", |e| e.symbol);
                        row = row.child(
                            chip(
                                &t,
                                ("rmc-element", z as usize),
                                label,
                                z == self.rmc.project.draft.absorber,
                            )
                            .on_click(cx.listener(
                                move |app, _, _, cx| {
                                    app.rmc.project.draft.absorber = z;
                                    app.rmc.scene_key = None;
                                    cx.notify();
                                },
                            )),
                        );
                    }
                    panel = panel.child(row);
                }
                let mut edges = div().flex().items_center().gap_2().child("Edge");
                for (i, edge) in [Edge::K, Edge::L1, Edge::L2, Edge::L3]
                    .into_iter()
                    .enumerate()
                {
                    edges = edges.child(
                        chip(
                            &t,
                            ("rmc-edge", i),
                            format!("{edge:?}"),
                            edge == self.rmc.project.draft.edge,
                        )
                        .on_click(cx.listener(move |app, _, _, cx| {
                            app.rmc.project.draft.edge = edge;
                            cx.notify();
                        })),
                    );
                }
                panel = panel.child(edges).child(hint(&t, "All sites of the selected element contribute to the average. Larger cells and more sites cost more."));
            } else {
                panel = panel.child(section_label(&t, "Run budget"))
                    .child(self.rmc.fields[8].clone())
                    .child(self.rmc.fields[9].clone())
                    .child(hint(&t, "10,000 attempts is a starting budget, not a convergence guarantee. Pause and continue whenever needed."))
                    .child(section_label(&t, "R-space objective"));
                for i in 14..19 {
                    panel = panel.child(self.rmc.fields[i].clone());
                }
                panel = panel.child(button(&t, "rmc-spectrum-ranges", "Use spectrum ranges", false)
                    .on_click(cx.listener(|app, _, _, cx| app.use_rmc_spectrum_ranges(cx))))
                    .child(hint(&t, "R min starts 0.15 Å above saved Rbkg to avoid the low-R background region."))
                    .child(section_label(&t, "Fixed calibration"))
                    .child(self.rmc.fields[12].clone()).child(self.rmc.fields[13].clone())
                    .child(hint(&t, "Check S₀² and ΔE₀ before running. RMC moves coordinates; these two values stay fixed."));
            }
            panel = panel.child(
                button(
                    &t,
                    "rmc-advanced",
                    if self.rmc.advanced {
                        "▾ Advanced settings"
                    } else {
                        "▸ Advanced settings"
                    },
                    false,
                )
                .on_click(cx.listener(|app, _, _, cx| {
                    app.rmc.advanced = !app.rmc.advanced;
                    cx.notify();
                })),
            );
            if self.rmc.advanced {
                if is_cell {
                    panel = panel.child(section_label(&t, "Exact scattering"));
                    for i in 3..6 {
                        panel = panel.child(self.rmc.fields[i].clone());
                    }
                    panel = panel.child(hint(&t, "Exact cached paths with fixed reference potentials. Adaptive scattering is experimental and disabled."))
                        .child(section_label(&t, "Physical bounds"));
                    for i in 6..8 {
                        panel = panel.child(self.rmc.fields[i].clone());
                    }
                    panel = panel.child(hint(&t, "Displacement is measured from the starting positions. Minimum distance rejects atomic overlaps."))
                        .child("Absorbing atom indices (blank = all)").child(self.rmc.texts[0].clone())
                        .child("Fixed atom indices").child(self.rmc.texts[1].clone())
                        .child(hint(&t, "Indices start at 0. Atom 0 is fixed by default to anchor the structure."));
                } else {
                    panel = panel.child(self.rmc.fields[10].clone()).child(self.rmc.fields[11].clone())
                        .child(hint(&t, "Tolerance controls acceptance of worse moves; zero accepts improvements only. It is numerical, not a physical temperature."))
                        .child("Pair minimum distances (Å)").child(self.rmc.texts[2].clone())
                        .child(hint(&t, "Optional: Cu-O=1.5, Cu-Cu=2.0. Choose physically justified bounds for your material."));
                }
            }
            let reason = self.rmc_form_blocker(cx);
            let mut footer = div()
                .p_3()
                .flex()
                .flex_col()
                .gap_2()
                .border_t_1()
                .border_color(t.border)
                .bg(t.surface);
            if is_cell {
                footer = footer.child(
                    button(&t, "rmc-next-model", "Next: fit settings →", true)
                        .disabled(
                            self.rmc.project.configuration.is_none()
                                || self.rmc.builder_busy
                                || self.rmc.builder_error.is_some(),
                        )
                        .on_click(cx.listener(|app, _, _, cx| {
                            app.stage_view.fit_step = FitStep::Model;
                            app.rmc.advanced = false;
                            cx.notify();
                        })),
                );
            } else {
                footer = footer
                    .child(
                        div()
                            .text_size(px(11.5))
                            .text_color(if reason.is_some() { t.warn } else { t.success })
                            .child(reason.clone().unwrap_or_else(|| {
                                "Ready · processed spectrum and valid supercell".into()
                            })),
                    )
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .child(
                                button(&t, "rmc-prepare", "Preview initial fit", false)
                                    .disabled(reason.is_some())
                                    .when(reason.is_some(), |d| d.opacity(0.45).cursor_default())
                                    .on_click(
                                        cx.listener(|app, _, _, cx| app.begin_rmc(true, None, cx)),
                                    ),
                            )
                            .child(
                                button(&t, "rmc-run", "Run RMC →", true)
                                    .disabled(reason.is_some())
                                    .when(reason.is_some(), |d| d.opacity(0.45).cursor_default())
                                    .on_click(
                                        cx.listener(|app, _, _, cx| app.begin_rmc(false, None, cx)),
                                    ),
                            ),
                    );
            }
            let scene = self.rmc_structure_panel(false, cx);
            return out.child(
                div()
                    .flex_1()
                    .min_h_0()
                    .min_w_0()
                    .flex()
                    .child(
                        div()
                            .w(px(340.))
                            .flex_none()
                            .min_h_0()
                            .flex()
                            .flex_col()
                            .border_r_1()
                            .border_color(t.border)
                            .child(panel)
                            .child(footer),
                    )
                    .child(scene),
            );
        }
        out.child(self.rmc_results(cx))
    }
}

impl StudioApp {
    /// Read uncommitted controls for both inline feedback and submission.
    fn pending_rmc_draft(&self, cx: &Context<Self>) -> Result<engine::Draft, String> {
        let mut draft = self.rmc.project.draft.clone();
        for (index, field) in self.rmc.fields.iter().enumerate() {
            let value = field
                .read(cx)
                .pending_value(cx)
                .map_err(|_| format!("{}: enter a valid number.", FIELD_LABELS[index]))?
                .ok_or_else(|| format!("{} requires an explicit value.", FIELD_LABELS[index]))?;
            set_draft_field(&mut draft, index, value);
        }
        if self.rmc.texts.len() == 3 {
            draft.absorber_atoms = self.rmc.texts[0].read(cx).text().into();
            draft.fixed_atoms = self.rmc.texts[1].read(cx).text().into();
            draft.constraints.pairs = engine::pair_distances(self.rmc.texts[2].read(cx).text())?;
        }
        Ok(draft)
    }
    fn rmc_form_blocker(&self, cx: &Context<Self>) -> Option<String> {
        (|| -> Result<(), String> {
            if self.rmc.control.is_some() {
                return Err("Stop and save the active run before starting a new one.".into());
            }
            if self.fit_running || self.feff_running || self.batch_running {
                return Err("Wait for the active calculation to finish.".into());
            }
            if self.load_running || self.stale_plots.is_some() {
                return Err("Finish spectrum processing in Background first.".into());
            }
            if !cfg!(feature = "refeff-runner") {
                return Err("This desktop build needs the ReFEFF backend.".into());
            }
            if self.rmc.builder_busy {
                return Err("Updating the supercell preview…".into());
            }
            if let Some(e) = &self.rmc.builder_error {
                return Err(e.clone());
            }
            let spectrum = self
                .spectrum
                .as_ref()
                .ok_or("Select a spectrum in the Groups panel.")?;
            let configuration = self
                .rmc
                .project
                .configuration
                .as_ref()
                .ok_or("Choose a structure and build its supercell first.")?;
            let draft = self.pending_rmc_draft(cx)?;
            draft.validate_form(spectrum, configuration)
        })()
        .err()
    }
    fn commit_rmc_fields(&mut self, cx: &mut Context<Self>) -> Result<(), String> {
        let draft = self.pending_rmc_draft(cx)?;
        if draft.repeats != self.rmc.project.draft.repeats {
            return Err("Wait for the supercell preview to finish updating.".into());
        }
        self.rmc.project.draft = draft;
        Ok(())
    }
    fn ensure_rmc_fields(&mut self, cx: &mut Context<Self>) {
        if !self.rmc.fields.is_empty() {
            return;
        }
        let d = &self.rmc.project.draft;
        let specs = [
            ("Repeat a", d.repeats[0] as f64),
            ("Repeat b", d.repeats[1] as f64),
            ("Repeat c", d.repeats[2] as f64),
            ("Cluster radius (Å)", d.options.cluster_radius),
            ("Path radius (Å)", d.options.path_radius),
            ("Maximum path legs", d.options.max_legs as f64),
            ("Maximum displacement (Å)", d.max_displacement),
            ("Minimum distance (Å)", d.min_distance),
            ("Attempt budget", d.steps as f64),
            ("Move size (Å)", d.step_size),
            ("Metropolis tolerance", d.temperature),
            ("Random seed", d.seed as f64),
            ("S₀²", d.s02),
            ("Fit ΔE₀ (eV)", d.delta_e0),
            ("k min (Å⁻¹)", d.ranges.kmin),
            ("k max (Å⁻¹)", d.ranges.kmax),
            ("R min (Å)", d.ranges.rmin),
            ("R max (Å)", d.ranges.rmax),
            ("k weight", d.ranges.kweight),
        ];
        for (index, (label, value)) in specs.into_iter().enumerate() {
            let kind = if matches!(index, 0..=2 | 5 | 8 | 11 | 18) {
                FieldKind::Integer { min: Some(0) }
            } else {
                FieldKind::Float
            };
            let step = match index {
                8 => 1_000.,
                9 => 0.01,
                10 => 0.0001,
                12 => 0.01,
                3 | 4 | 6 | 7 | 13..=17 => 0.1,
                _ => 1.,
            };
            let field = cx.new(|cx| {
                NumericField::new(label, "required", Some(value), kind, self.theme, cx)
                    .with_step(step)
            });
            cx.subscribe(&field, |_, _, _: &FieldPreview, cx| cx.notify())
                .detach();
            if index <= 2 {
                cx.subscribe(&field, |app, _, _: &FieldPreview, cx| {
                    app.build_rmc_supercell(cx)
                })
                .detach();
            }
            cx.subscribe(&field, move |app, _, event, cx| {
                if let FieldEvent::Changed(value) = event {
                    let Some(v) = value else {
                        if index <= 2 {
                            app.build_rmc_supercell(cx);
                        }
                        app.rmc.error = Some("RMC settings require explicit values.".into());
                        cx.notify();
                        return;
                    };
                    set_draft_field(&mut app.rmc.project.draft, index, *v);
                    if index <= 2 {
                        app.build_rmc_supercell(cx);
                    }
                    app.rmc.error = None;
                    app.rmc.scene_key = None;
                    cx.notify();
                } else if index <= 2 && matches!(event, FieldEvent::Invalid(_)) {
                    // NumericField restored its previous value after rejection.
                    app.build_rmc_supercell(cx);
                }
            })
            .detach();
            self.rmc.fields.push(field);
        }
        let d = &self.rmc.project.draft;
        let pairs = d
            .constraints
            .pairs
            .iter()
            .map(|p| {
                format!(
                    "{}-{}={}",
                    Element::from_z(p.elements[0]).map_or("?", |e| e.symbol),
                    Element::from_z(p.elements[1]).map_or("?", |e| e.symbol),
                    p.minimum
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        for (index, (placeholder, value)) in [
            ("all atoms of selected element", d.absorber_atoms.clone()),
            ("0", d.fixed_atoms.clone()),
            ("Cu-O=1.5, Cu-Cu=2.0", pairs),
        ]
        .into_iter()
        .enumerate()
        {
            let field = cx.new(|cx| TextInput::new(placeholder, value, self.theme, cx));
            cx.subscribe(&field, move |app, _, event, cx| {
                if matches!(event, InputEvent::Edited(_)) {
                    cx.notify();
                }
                if let InputEvent::Edited(value) = event
                    && index == 0
                {
                    app.rmc.project.draft.absorber_atoms = value.to_string();
                    app.rmc.scene_key = None;
                    cx.notify();
                }
                if let InputEvent::Committed(value) = event {
                    match index {
                        0 => app.rmc.project.draft.absorber_atoms = value.to_string(),
                        1 => app.rmc.project.draft.fixed_atoms = value.to_string(),
                        _ => match engine::pair_distances(value) {
                            Ok(pairs) => app.rmc.project.draft.constraints.pairs = pairs,
                            Err(e) => {
                                app.rmc.error = Some(e);
                                cx.notify();
                                return;
                            }
                        },
                    }
                    app.rmc.scene_key = None;
                    app.rmc.error = None;
                    cx.notify();
                }
            })
            .detach();
            self.rmc.texts.push(field);
        }
    }
    pub(crate) fn restore_rmc_plots(&mut self, cx: &mut Context<Self>) {
        self.rmc.plotted_best = None;
        self.rebuild_rmc_plots(cx);
    }
    pub(crate) fn restyle_rmc(&mut self, cx: &mut Context<Self>) {
        for field in self.rmc.fields.iter().chain(self.rmc.continuation.iter()) {
            field.update(cx, |f, cx| f.set_theme(self.theme, cx));
        }
        for field in &self.rmc.texts {
            field.update(cx, |f, cx| f.set_theme(self.theme, cx));
        }
        self.restore_rmc_plots(cx);
    }
    fn rebuild_rmc_plots(&mut self, cx: &mut Context<Self>) {
        let (Some(progress), Some(request)) = (&self.rmc.live, &self.rmc.request) else {
            return;
        };
        let key = progress.best.evaluation.score.to_bits();
        let refresh_curves = self.rmc.plots.len() != 5 || self.rmc.plotted_best != Some(key);
        match result_plots(
            progress,
            request,
            self.theme,
            refresh_curves,
            !self.rmc.hide_initial_curve,
            self.rmc.full_history,
        ) {
            Ok(plots) => {
                for (i, plot) in plots {
                    if let Some(entity) = self.rmc.plots.get(i) {
                        entity.update(cx, |view, cx| view.set_plot_keep_view(plot, cx));
                    } else {
                        self.rmc
                            .plots
                            .push(plot_builder(plot).interactive().build(cx));
                    }
                }
                self.rmc.plotted_best = Some(key);
            }
            Err(e) => self.rmc.error = Some(e),
        }
        if refresh_curves {
            self.rmc.scene_key = None;
        }
    }
    fn rmc_results(&mut self, cx: &mut Context<Self>) -> gpui::Div {
        let t = self.theme;
        let mut out = div().flex_1().min_h_0().min_w_0().flex().flex_col();
        let export_disabled = self.rmc.project.saved.is_none()
            || (self.rmc.control.is_some() && self.rmc.phase != "Paused");
        let mut actions = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_2()
            .px_3()
            .py_2();
        for (i, label) in ["Fit plots", "Structure", "Convergence", "Run details"]
            .into_iter()
            .enumerate()
        {
            actions = actions.child(
                chip(&t, ("rmc-result-tab", i), label, self.rmc.plot_tab == i).on_click(
                    cx.listener(move |app, _, _, cx| {
                        app.rmc.plot_tab = i;
                        cx.notify();
                    }),
                ),
            );
        }
        actions = actions
            .child(div().flex_1())
            .child(
                button(&t, "rmc-open-run", "Open checkpoint…", false)
                    .disabled(self.rmc.control.is_some())
                    .when(self.rmc.control.is_some(), |d| {
                        d.opacity(0.45).cursor_default()
                    })
                    .on_click(cx.listener(|app, _, _, cx| app.rmc_checkpoint_dialog(cx))),
            )
            .child(
                button(&t, "rmc-export", "Export result…", false)
                    .disabled(export_disabled)
                    .when(export_disabled, |d| d.opacity(0.45).cursor_default())
                    .on_click(cx.listener(|app, _, _, cx| app.export_rmc(cx))),
            );
        out = out.child(actions);
        let Some(p) = &self.rmc.live else {
            return out.child(div().flex_1().flex().flex_col().justify_center().items_center().gap_3().p_4()
                .child(div().text_size(px(18.)).child(if self.rmc.control.is_some() { "Preparing your initial fit" } else { "Your RMC results will appear here" }))
                .child(hint(&t, if self.rmc.control.is_some() {
                    "Exact ReFEFF preparation runs in the background. Live curves will appear when the initial calculation finishes."
                } else { "Choose a structure, check the fit settings, then preview the initial fit or start optimization." }))
                .when(self.rmc.control.is_none(), |d| d.child(button(&t, "rmc-back-setup", "Go to fit settings →", true)
                    .on_click(cx.listener(|app, _, _, cx| { app.stage_view.fit_step = FitStep::Model; cx.notify(); })))));
        };
        let initial = p.initial.evaluation.score;
        let best = p.best.evaluation.score;
        let improvement = if initial > 0. {
            format!("{:.2}%", 100. * (initial - best) / initial)
        } else {
            "—".into()
        };
        if self.rmc.control.is_none() && self.rmc.project.saved.is_some() {
            let complete = p.completed >= p.limit;
            let mut resume = div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap_3()
                .px_3()
                .py_2()
                .bg(t.raised)
                .child(div().flex_1().child(format!(
                    "{} · {} completed attempts",
                    self.rmc.phase,
                    count(p.completed)
                )));
            if complete {
                let field = self.rmc.continuation.get_or_insert_with(|| {
                    cx.new(|cx| {
                        NumericField::new(
                            "Additional attempts",
                            "required",
                            Some(10_000.),
                            FieldKind::Integer { min: Some(0) },
                            t,
                            cx,
                        )
                        .with_step(1_000.)
                    })
                });
                resume = resume.child(div().w(px(320.)).flex_none().child(field.clone()));
            }
            resume = resume.child(
                button(
                    &t,
                    "rmc-resume-saved",
                    if complete {
                        "Continue optimization →"
                    } else {
                        "Resume saved run →"
                    },
                    true,
                )
                .on_click(cx.listener(|app, _, _, cx| app.resume_rmc(cx))),
            );
            out = out.child(resume);
        }
        let trend = trend_label(&p.trend.status);
        out = out.child(
            div()
                .flex()
                .gap_2()
                .px_3()
                .py_2()
                .child(metric(
                    &t,
                    "Best objective ↓",
                    format!("{best:.7}"),
                    format!("Initial {initial:.7}"),
                ))
                .child(metric(
                    &t,
                    "Improvement",
                    improvement,
                    format!("Current {:.7}", p.current_score),
                ))
                .child(metric(
                    &t,
                    "Convergence",
                    trend.into(),
                    match p.trend.status {
                        ResidualTrendStatus::InsufficientHistory => format!(
                            "Need at least {} attempts",
                            count(p.trend.settings.minimum_attempts)
                        ),
                        ResidualTrendStatus::StillChanging => {
                            "Continue and monitor the trend".into()
                        }
                        ResidualTrendStatus::ResidualPlateau => {
                            "Numerical plateau; inspect the fit".into()
                        }
                    },
                )),
        );
        if let Some(request) = &self.rmc.request {
            let data = &request.problem.datasets[0];
            if let rexafs::rmc::Objective::R(transform) = &data.objective {
                out = out.child(div().px_3().pb_2().text_size(px(11.5)).text_color(t.text_muted).child(format!(
                    "{} · R {:.2}–{:.2} Å · k {:.2}–{:.2} Å⁻¹ · k weight {} · fitting real + imaginary",
                    request.source.label, transform.rmin, transform.rmax, transform.kmin, transform.kmax, data.exafs.kweight)));
            }
        }
        if self.rmc.plot_tab == 1 {
            return out.child(self.rmc_structure_panel(true, cx));
        }
        if self.rmc.plot_tab == 2 {
            let settings = &p.trend.settings;
            let mut diagnostic = div().px_3().py_2().flex().flex_col().gap_1().bg(t.surface)
                .child(match p.trend.status {
                    ResidualTrendStatus::InsufficientHistory => "Too early to assess convergence. Reaching the attempt budget alone does not establish a plateau.",
                    ResidualTrendStatus::StillChanging => "The recent residual is still changing. Continue the same run to assess whether it settles.",
                    ResidualTrendStatus::ResidualPlateau => "The recent residual has plateaued. This does not establish a unique or physically complete structure.",
                })
                .child(hint(&t, format!("Criterion: {} consecutive comparisons of {}-attempt windows, after {} attempts. This diagnostic does not stop the run.",
                    settings.stable_windows, count(settings.window), count(settings.minimum_attempts))));
            if let Some(w) = p.trend.windows.last() {
                diagnostic = diagnostic.child(format!(
                    "Recent attempts {}–{} · best improvement {} · mean change {}",
                    count(w.first_step),
                    count(w.last_step),
                    w.best_improvement
                        .map_or_else(|| "—".into(), |v| format!("{v:.3e}")),
                    w.mean_change
                        .map_or_else(|| "—".into(), |v| format!("{v:.3e}"))
                ));
            }
            diagnostic = diagnostic.child(
                div()
                    .flex()
                    .gap_3()
                    .child(
                        div()
                            .text_color(crate::plotting::trace_rgba(&t, 0))
                            .child("━ Current objective"),
                    )
                    .child(
                        div()
                            .text_color(crate::plotting::trace_rgba(&t, 2))
                            .child("━ Best objective"),
                    ),
            );
            let mut scope = div().flex().gap_2().items_center();
            for (i, label) in ["Recent trend", "All retained attempts"]
                .into_iter()
                .enumerate()
            {
                scope = scope.child(
                    chip(
                        &t,
                        ("rmc-trend-scope", i),
                        label,
                        self.rmc.full_history == (i == 1),
                    )
                    .on_click(cx.listener(move |app, _, _, cx| {
                        app.rmc.full_history = i == 1;
                        if app.rmc.plots.len() == 5 {
                            app.rmc.plots.pop();
                        }
                        app.rebuild_rmc_plots(cx);
                        cx.notify();
                    })),
                );
            }
            diagnostic = diagnostic.child(scope).child(hint(&t, "Recent view shows the latest four diagnostic windows. Changing this view does not change the convergence criterion."));
            out = out.child(diagnostic);
            if let Some(plot) = self.rmc.plots.get(4) {
                out = out.child(div().flex_1().min_h_0().p_3().child(plot.clone()));
            }
            return out;
        }
        if self.rmc.plot_tab == 3 {
            let acceptance = if p.completed > 0 {
                100. * p.accepted as f64 / p.completed as f64
            } else {
                0.
            };
            let reuse = p
                .cache
                .active_reuse()
                .map(|v| format!("{:.1}%", 100. * v))
                .unwrap_or_else(|| "—".into());
            let sec = if p.completed > 0 {
                format!("{:.3} s/attempt", p.elapsed_seconds / p.completed as f64)
            } else {
                "Awaiting moves".into()
            };
            let mut details = div().id("rmc-run-details").flex_1().min_h_0().overflow_y_scroll().p_3().flex().flex_col().gap_3()
                .child(section_label(&t, "Optimization"))
                .child(format!("{} / {} attempts · accepted {acceptance:.1}% · constraint rejected {}", count(p.completed), count(p.limit), count(p.constraint_rejected)))
                .child(format!("Active time {:.1} s · setup this session {:.1} s · {sec} inclusive", p.elapsed_seconds, p.setup_seconds))
                .child(hint(&t, "Time includes preparation and calculation but excludes pauses. Inclusive time per attempt is not an isolated scattering benchmark."))
                .child(section_label(&t, "Exact scattering cache"))
                .child(format!("Active path reuse {reuse} · {} exact path calculations · {:.1} MiB cached", count(p.cache.exact as usize), p.cache.bytes as f64 / 1048576.))
                .child(hint(&t, "Reuse = reused active paths / (reused active paths + exact calculations). Cache counters restart on a cold resume."))
                .child(section_label(&t, "Objective"))
                .child(hint(&t, "Normalized R-space real + imaginary residual plus a structural penalty. R magnitude and k-space curves are diagnostic views."));
            for fit in &p.best.evaluation.datasets {
                details = details.child(format!(
                    "Spectral residual {:.7} · structural penalty {:.7}",
                    fit.score, p.best.penalty
                ));
            }
            details = details.child(section_label(&t, "Recovery and provenance"));
            if let Some(saved) = &self.rmc.project.saved {
                details = details.child(format!(
                    "Checkpoint state: {} completed attempts",
                    count(saved.progress.completed)
                ));
            }
            if let Some(path) = &self.rmc.recovery {
                details = details.child(format!("Recovery file: {}", path.display()));
            }
            details = details.child(hint(&t, "Saved after preparation, every 30 seconds at move boundaries, and on pause, stop or completion. Resume keeps the saved spectrum, preprocessing, structure, calibration and random sequence. Draft edits apply to a new run."))
                .child(hint(&t, "Pause before exporting the latest completed state. Export includes checkpoint, initial/best structures, k/R curves and a convergence report."));
            return out.child(details);
        }
        let mut views = div().flex().flex_wrap().items_center().gap_2().px_3();
        for (i, label) in ["k + R magnitude", "R real + imaginary", "All four plots"]
            .into_iter()
            .enumerate()
        {
            views = views.child(
                chip(
                    &t,
                    ("rmc-curve-view", i),
                    label,
                    self.rmc.fit_plot_view == i,
                )
                .on_click(cx.listener(move |app, _, _, cx| {
                    app.rmc.fit_plot_view = i;
                    cx.notify();
                })),
            );
        }
        views = views.child(
            button(
                &t,
                "rmc-toggle-initial",
                if self.rmc.hide_initial_curve {
                    "Show initial"
                } else {
                    "Hide initial"
                },
                false,
            )
            .on_click(cx.listener(|app, _, _, cx| {
                app.rmc.hide_initial_curve = !app.rmc.hide_initial_curve;
                app.rmc.plotted_best = None;
                // Recompute automatic limits when toggling traces, while ordinary
                // live updates keep the user's zoom/pan unchanged.
                app.rmc.plots.clear();
                app.rebuild_rmc_plots(cx);
                cx.notify();
            })),
        );
        let mut legend = div().flex().flex_wrap().items_center().gap_3();
        for (i, label) in ["Experimental", "Initial", "Best"].into_iter().enumerate() {
            if i == 1 && self.rmc.hide_initial_curve {
                continue;
            }
            legend = legend.child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(
                        div()
                            .w(px(16.))
                            .h(px(3.))
                            .bg(crate::plotting::trace_rgba(&t, i)),
                    )
                    .child(hint(&t, label)),
            );
        }
        out = out.child(views.child(div().flex_1()).child(legend));
        let indices: &[usize] = match self.rmc.fit_plot_view {
            1 => &[2, 3],
            2 => &[0, 1, 2, 3],
            _ => &[0, 1],
        };
        let mut grid = div().flex_1().min_h_0().flex().flex_col().gap_2().p_3();
        for pair in indices.chunks(2) {
            let mut row = div().flex_1().min_h_0().min_w_0().flex().gap_2();
            for &index in pair {
                if let Some(plot) = self.rmc.plots.get(index) {
                    row = row.child(div().flex_1().min_w_0().min_h_0().child(plot.clone()));
                }
            }
            grid = grid.child(row);
        }
        out.child(grid).child(div().px_3().pb_2().child(hint(&t, "Experimental / initial / best · curves may extend outside the fitted window. Best fit and best structure always refer to the same state.")))
    }
    fn rmc_structure_panel(&mut self, result: bool, cx: &mut Context<Self>) -> gpui::Div {
        let configuration = if result {
            self.rmc
                .live
                .as_ref()
                .and_then(|p| {
                    if self.rmc.show_initial {
                        p.initial.structures.first()
                    } else {
                        p.best.structures.first()
                    }
                })
                .map(|s| s.configuration.clone())
        } else {
            self.rmc.project.configuration.clone()
        };
        let Some(configuration) = configuration else {
            return div()
                .flex_1()
                .p_3()
                .child("Choose a structure and build its supercell.");
        };
        let source = if result {
            self.rmc
                .request
                .as_ref()
                .and_then(|r| r.problem.datasets.first())
                .map(|d| d.exafs.absorbers.clone())
                .unwrap_or_default()
        } else {
            engine::selected_absorbers(&configuration, &self.rmc.project.draft).unwrap_or_default()
        };
        let score = self
            .rmc
            .live
            .as_ref()
            .map_or(0, |p| p.best.evaluation.score.to_bits());
        let key = (configuration.atoms.len(), score, self.rmc.show_initial);
        if self.rmc.scene_key != Some(key) {
            self.structure.scene = Some(Arc::new(configuration_scene(&configuration, &source)));
            self.rmc.scene_key = Some(key);
        }
        let canvas = self.molecule_canvas(cx);
        let mut out = div().flex_1().min_w_0().min_h_0().flex().flex_col();
        let mut bar = div().flex().flex_wrap().gap_2().p_2().child(format!(
            "{} atoms · {}",
            configuration.atoms.len(),
            if configuration.cell.is_some() {
                "Periodic supercell"
            } else {
                "Finite cluster"
            }
        ));
        if result {
            for (i, label) in ["Best structure", "Initial structure"]
                .into_iter()
                .enumerate()
            {
                bar = bar.child(
                    chip(
                        &self.theme,
                        ("rmc-structure-state", i),
                        label,
                        self.rmc.show_initial == (i == 1),
                    )
                    .on_click(cx.listener(move |app, _, _, cx| {
                        app.rmc.show_initial = i == 1;
                        app.rmc.scene_key = None;
                        cx.notify();
                    })),
                );
            }
        }
        if let Some(cell) = configuration.cell {
            bar = bar.child(format!(
                "a {:.3} Å · b {:.3} Å · c {:.3} Å",
                length(cell[0]),
                length(cell[1]),
                length(cell[2])
            ));
        }
        bar = bar.child(div().flex_1()).child(
            button(&self.theme, "rmc-reset-view", "Reset view", false).on_click(cx.listener(
                |app, _, _, cx| {
                    app.structure.camera = Default::default();
                    cx.notify();
                },
            )),
        );
        let elements: std::collections::BTreeSet<_> = configuration
            .atoms
            .iter()
            .map(|a| a.atomic_number)
            .collect();
        let legend = elements
            .into_iter()
            .map(|z| {
                let name = Element::from_z(z).map_or("?", |e| e.symbol);
                if source
                    .iter()
                    .any(|i| configuration.atoms[*i].atomic_number == z)
                {
                    format!("{name}: absorbing sites")
                } else {
                    name.into()
                }
            })
            .collect::<Vec<_>>()
            .join(" · ");
        out = out
            .child(bar)
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .min_w_0()
                    .relative()
                    .overflow_hidden()
                    .child(canvas),
            )
            .child(div().px_3().py_2().child(hint(
                &self.theme,
                format!("{legend} · Drag to rotate · scroll to zoom"),
            )));
        out
    }
    fn export_rmc(&mut self, cx: &mut Context<Self>) {
        if self.rmc.control.is_some() && self.rmc.phase != "Paused" {
            self.rmc.error =
                Some("Pause the run before exporting its latest completed state.".into());
            cx.notify();
            return;
        }
        let Some(saved) = self.rmc.project.saved.clone() else {
            return;
        };
        let prompt = cx.prompt_for_paths(gpui::PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: None,
        });
        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(paths))) = prompt.await
                && let Some(parent) = paths.first()
            {
                let parent = parent.clone();
                let result = cx
                    .background_executor()
                    .spawn(async move { engine::export(&parent, &saved) })
                    .await;
                this.update(cx, |app, cx| {
                    match result {
                        Ok(path) => {
                            app.status =
                                format!("RMC results exported to {}", path.display()).into()
                        }
                        Err(e) => app.rmc.error = Some(e),
                    }
                    cx.notify();
                })
                .ok();
            }
        })
        .detach();
    }
}
const FIELD_LABELS: [&str; 19] = [
    "Repeat a",
    "Repeat b",
    "Repeat c",
    "Cluster radius",
    "Path radius",
    "Maximum path legs",
    "Maximum displacement",
    "Minimum distance",
    "Attempt budget",
    "Move size",
    "Metropolis tolerance",
    "Random seed",
    "S₀²",
    "Fit ΔE₀",
    "k min",
    "k max",
    "R min",
    "R max",
    "k weight",
];
fn hint(t: &crate::theme::Theme, text: impl Into<gpui::SharedString>) -> gpui::Div {
    div()
        .text_size(px(11.5))
        .text_color(t.text_muted)
        .child(text.into())
}
fn metric(t: &crate::theme::Theme, label: &str, value: String, detail: String) -> gpui::Div {
    div()
        .flex_1()
        .min_w_0()
        .p_2()
        .rounded_md()
        .border_1()
        .border_color(t.border)
        .bg(t.surface)
        .child(hint(t, label.to_string()))
        .child(div().text_size(px(17.)).child(value))
        .child(hint(t, detail))
}
fn count(n: usize) -> String {
    let digits = n.to_string();
    digits
        .chars()
        .enumerate()
        .fold(String::new(), |mut out, (i, c)| {
            if i > 0 && (digits.len() - i).is_multiple_of(3) {
                out.push(',');
            }
            out.push(c);
            out
        })
}
fn trend_label(status: &ResidualTrendStatus) -> &'static str {
    match status {
        ResidualTrendStatus::InsufficientHistory => "Not assessed yet",
        ResidualTrendStatus::StillChanging => "Still changing",
        ResidualTrendStatus::ResidualPlateau => "Residual plateau",
    }
}
fn set_draft_field(d: &mut engine::Draft, index: usize, v: f64) {
    match index {
        0..=2 => d.repeats[index] = v as usize,
        3 => d.options.cluster_radius = v,
        4 => d.options.path_radius = v,
        5 => d.options.max_legs = v.min(255.) as u8,
        6 => d.max_displacement = v,
        7 => d.min_distance = v,
        8 => d.steps = v as usize,
        9 => d.step_size = v,
        10 => d.temperature = v,
        11 => d.seed = v as u64,
        12 => d.s02 = v,
        13 => d.delta_e0 = v,
        14 => d.ranges.kmin = v,
        15 => d.ranges.kmax = v,
        16 => d.ranges.rmin = v,
        17 => d.ranges.rmax = v,
        _ => {
            d.ranges.kweight = v;
            d.ranges.kweights = vec![v];
            d.ranges.follow_transform = false;
        }
    }
}
fn length(v: [f64; 3]) -> f64 {
    v.iter().map(|x| x * x).sum::<f64>().sqrt()
}
fn configuration_scene(c: &Configuration, absorbers: &[usize]) -> MoleculeScene {
    let center = c
        .cell
        .map(|cell| std::array::from_fn(|axis| cell.iter().map(|v| v[axis]).sum::<f64>() / 2.))
        .unwrap_or_else(|| {
            std::array::from_fn(|axis| {
                let lo = c
                    .atoms
                    .iter()
                    .map(|a| a.position[axis])
                    .fold(f64::INFINITY, f64::min);
                let hi = c
                    .atoms
                    .iter()
                    .map(|a| a.position[axis])
                    .fold(f64::NEG_INFINITY, f64::max);
                (lo + hi) / 2.
            })
        });
    let mut extent = c
        .atoms
        .iter()
        .map(|a| length(std::array::from_fn(|i| a.position[i] - center[i])))
        .fold(1.5, f64::max);
    if let Some(cell) = c.cell {
        for n in 0..8 {
            let corner = std::array::from_fn(|axis| {
                (0..3)
                    .filter(|i| n & (1 << i) != 0)
                    .map(|i| cell[i][axis])
                    .sum::<f64>()
                    - center[axis]
            });
            extent = extent.max(length(corner));
        }
    }
    let mut scene = MoleculeScene {
        center,
        extent,
        // A periodic configuration has no single absorber-centred radius guide.
        radius: 0.,
        atoms: c
            .atoms
            .iter()
            .enumerate()
            .map(|(i, a)| SceneAtom {
                pos: a.position,
                z: a.atomic_number as u32,
                index: Some(i),
                shell: 0,
                absorber: absorbers.contains(&i),
                faded: false,
                label: format!(
                    "{} {i}",
                    Element::from_z(a.atomic_number).map_or("?", |e| e.symbol)
                ),
            })
            .collect(),
        ..Default::default()
    };
    if let Some(cell) = c.cell {
        let corner = |n: usize| {
            std::array::from_fn(|j| {
                (0..3)
                    .filter(|i| n & (1 << i) != 0)
                    .map(|i| cell[i][j])
                    .sum()
            })
        };
        for n in 0..8 {
            for axis in 0..3 {
                if n & (1 << axis) == 0 {
                    scene.edges.push([corner(n), corner(n | (1 << axis))]);
                }
            }
        }
    }
    scene
}
fn result_plots(
    p: &Progress,
    request: &Request,
    theme: crate::theme::Theme,
    refresh_curves: bool,
    show_initial: bool,
    full_history: bool,
) -> Result<Vec<(usize, Plot)>, String> {
    use rexafs::rmc::{Objective, transform_spectrum_fourier};
    engine::validate_progress(p, request)?;
    let mut out = Vec::new();
    if refresh_curves {
        let dataset = request
            .problem
            .datasets
            .first()
            .ok_or("Missing saved dataset")?;
        let Objective::R(transform) = &dataset.objective else {
            return Err("The desktop result viewer currently requires R space.".into());
        };
        let weight = dataset.exafs.kweight as f64;
        let curves = [
            ("Experimental", &dataset.exafs.chi),
            ("Initial", &p.initial.evaluation.datasets[0].chi),
            ("Best", &p.best.evaluation.datasets[0].chi),
        ];
        let mut kp = Plot::new()
            .theme(theme.plot_theme())
            .xlabel("k (Å⁻¹)")
            .ylabel(crate::plotting::chik_label(weight));
        let mut rs: Vec<_> = ["|χ(R)|", "Re χ(R)", "Im χ(R)"]
            .iter()
            .map(|name| {
                Plot::new()
                    .theme(theme.plot_theme())
                    .xlabel("R (Å)")
                    .ylabel(*name)
                    .xlim(0., transform.rmax + 1.)
            })
            .collect();
        for (i, (label, chi)) in curves.into_iter().enumerate() {
            if i == 1 && !show_initial {
                continue;
            }
            let a =
                transform_spectrum_fourier(&dataset.exafs.k, chi, dataset.exafs.kweight, transform)
                    .map_err(|e| e.to_string())?;
            let weighted: Vec<_> = dataset
                .exafs
                .k
                .iter()
                .zip(chi)
                .map(|(k, x)| x * k.powf(weight))
                .collect();
            kp = kp
                .line(&dataset.exafs.k, &weighted)
                .label(label)
                .color(crate::plotting::trace_color(&theme, i))
                .into();
            for (plot, y) in
                rs.iter_mut()
                    .zip([&a.r_space.chir_mag, &a.r_space.chir_re, &a.r_space.chir_im])
            {
                *plot = plot
                    .clone()
                    .line(a.r_space.r.as_slice(), y.as_slice())
                    .label(label)
                    .color(crate::plotting::trace_color(&theme, i))
                    .into();
            }
        }

        let mut curves = vec![kp];
        curves.extend(rs);
        out.extend(curves.into_iter().enumerate());
    }
    let recent = p
        .trend
        .settings
        .window
        .saturating_mul(p.trend.settings.stable_windows.saturating_add(1));
    let history = if full_history {
        &p.history[..]
    } else {
        &p.history[p.history.len().saturating_sub(recent)..]
    };
    let mut x: Vec<_> = history.iter().map(|s| s.step as f64).collect();
    let mut current: Vec<_> = history.iter().map(|s| s.score).collect();
    let mut best: Vec<_> = history.iter().map(|s| s.best_score).collect();
    if x.is_empty() {
        x.push(p.completed as f64);
        current.push(p.current_score);
        best.push(p.best.evaluation.score);
    }
    let trend = Plot::new()
        .theme(theme.plot_theme())
        .xlabel("Completed attempts")
        .ylabel("Objective")
        .line(&x, &current)
        .label("Current")
        .color(crate::plotting::trace_color(&theme, 0))
        .line(&x, &best)
        .label("Best")
        .color(crate::plotting::trace_color(&theme, 2))
        .into();
    out.push((4, trend));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn periodic_preview_frames_cell_and_preserves_coordinates() {
        let configuration = Configuration {
            atoms: vec![
                rexafs::rmc::Atom {
                    atomic_number: 29,
                    position: [0.; 3],
                },
                rexafs::rmc::Atom {
                    atomic_number: 8,
                    position: [2., 2., 2.],
                },
            ],
            cell: Some([[8., 0., 0.], [2., 7., 0.], [0., 1., 6.]]),
        };
        let scene = configuration_scene(&configuration, &[0]);
        assert_eq!(scene.center, [5., 4., 3.]);
        assert_eq!(scene.atoms[1].pos, configuration.atoms[1].position);
        assert_eq!(scene.radius, 0.); // No absorber-centred guides on a periodic cell.
        for edge in scene.edges {
            for point in edge {
                assert!(
                    length(std::array::from_fn(|i| point[i] - scene.center[i])) <= scene.extent
                );
            }
        }
    }
}
