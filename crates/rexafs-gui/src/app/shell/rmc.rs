//! RMC mode in the existing Fitting workspace.
use super::{
    button, chip,
    controls::Menu,
    fit_workspace::FitStep,
    molecule_view::{MoleculeScene, SceneAtom},
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
    plot_tab: usize,
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
        let mut bar = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_2()
            .px_3()
            .py_2()
            .bg(self.theme.raised)
            .child(div().flex_1().child(if let Some(p) = &self.rmc.live {
                format!(
                    "RMC · {} · {}/{} attempts · best {:.6} · {:?}",
                    self.rmc.phase, p.completed, p.limit, p.best.evaluation.score, p.trend.status
                )
            } else {
                format!("RMC · {}", self.rmc.phase)
            }));
        if paused {
            bar = bar.child(
                button(&self.theme, "rmc-resume-live", "Resume", true)
                    .on_click(cx.listener(|app, _, _, cx| app.resume_rmc(cx))),
            );
        } else {
            bar = bar.child(
                button(&self.theme, "rmc-pause", "Pause", true).on_click(cx.listener(
                    |app, _, _, cx| {
                        if let Some(c) = &app.rmc.control {
                            c.pause();
                            app.rmc.phase = "Pausing after current move".into();
                        }
                        cx.notify();
                    },
                )),
            );
        }
        bar.child(
            button(&self.theme, "rmc-stop", "Stop and save", true).on_click(cx.listener(
                |app, _, _, cx| {
                    if let Some(c) = &app.rmc.control {
                        c.stop();
                        app.rmc.phase = "Stopping and saving".into();
                    }
                    cx.notify();
                },
            )),
        )
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
        if !(1..=1_000_000_000).contains(&self.rmc.project.draft.steps) {
            self.rmc.error = Some("Set 1…1,000,000,000 attempts for a continuation.".into());
            cx.notify();
            return;
        }
        if let Some(c) = &self.rmc.control {
            let total = self
                .rmc
                .live
                .as_ref()
                .map_or(self.rmc.project.draft.steps.max(1), |p| {
                    if p.completed >= p.limit {
                        p.completed
                            .saturating_add(self.rmc.project.draft.steps.max(1))
                    } else {
                        p.limit
                    }
                });
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
        if saved.progress.completed >= saved.request.settings.moves.steps {
            saved.request.settings.moves.steps = saved
                .progress
                .completed
                .saturating_add(self.rmc.project.draft.steps.max(1));
        }
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
            (FitStep::Structure, "Structure"),
            (FitStep::Calculate, "Calculate"),
            (FitStep::Model, "Model"),
            (FitStep::Results, "Results"),
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
            out = out.child(div().px_3().py_2().text_color(t.warn).child(error.clone()));
        }
        if self.rmc.project.saved.is_some() || self.rmc.control.is_some() {
            out = out.child(div().px_3().text_size(px(11.)).text_color(t.text_muted)
                .child("Resume uses the saved spectrum, structure and settings. Edits apply to a new run. Pause before exporting the latest result."));
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
                                    "Use structure →",
                                    self.structure.summary.is_some(),
                                )
                                .disabled(
                                    self.structure.summary.is_none()
                                        || self.structure.fetch_running,
                                )
                                .on_click(cx.listener(|app, _, _, cx| app.use_rmc_structure(cx))),
                            )
                            .child(
                                button(&t, "rmc-load-checkpoint", "Open RMC checkpoint…", true)
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
            let fields = if step == FitStep::Calculate {
                0..8
            } else {
                8..self.rmc.fields.len()
            };
            let mut panel = div()
                .id("rmc-settings")
                .w(px(340.))
                .flex_none()
                .min_h_0()
                .overflow_y_scroll()
                .p_3()
                .flex()
                .flex_col()
                .gap_2();
            panel = panel.child(div().child(if step == FitStep::Calculate {
                "Supercell and exact ReFEFF"
            } else {
                "Coordinate refinement · R real + imaginary"
            }));
            for i in fields {
                panel = panel.child(self.rmc.fields[i].clone());
            }
            if step == FitStep::Calculate {
                panel = panel.child(
                    button(&t, "rmc-supercell", "Use suggested size", false)
                        .on_click(cx.listener(|app, _, _, cx| app.suggest_rmc_supercell(cx))),
                );
                panel = panel.child(div().text_size(px(11.5)).text_color(t.text_muted)
                    .child("Live preview while typing · no scattering calculation. Suggested size spans twice the cluster radius plus the displacement margin; larger cells cost more."));
                if self.rmc.builder_busy {
                    panel = panel.child("Updating preview…");
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
                    let absorbers = absorbers.unwrap_or(0);
                    panel = panel.child(format!(
                        "{} atoms · {absorbers} absorbing sites · periodic cell",
                        c.atoms.len()
                    ));
                    let elements: std::collections::BTreeSet<_> =
                        c.atoms.iter().map(|a| a.atomic_number).collect();
                    let mut row = div().flex().flex_wrap().gap_1().child("Absorber");
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
                let mut edges = div().flex().gap_1().child("Edge");
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
                panel = panel.child(edges);
                panel = panel
                    .child("Absorbing atom indices (blank = all of element)")
                    .child(self.rmc.texts[0].clone())
                    .child("Fixed atom indices (zero based)")
                    .child(self.rmc.texts[1].clone());
            } else {
                panel = panel.child(button(&t, "rmc-spectrum-ranges", "Use spectrum ranges", false)
                    .on_click(cx.listener(|app, _, _, cx| app.use_rmc_spectrum_ranges(cx))))
                    .child(div().text_size(px(11.5)).text_color(t.text_muted)
                        .child("Defaults: 10,000 attempts, 0.03 Å moves, numerical tolerance 0.001, exact paths. Spectrum ranges start 0.15 Å above saved Rbkg. Check calibration and convergence for your sample."));
                panel=panel.child("Pair minimum distances (Å)").child(self.rmc.texts[2].clone())
                    .child(div().text_size(px(11.5)).text_color(t.text_muted).child("Exact cached paths; fixed reference potentials. Adaptive scattering remains experimental and is not enabled here. R min must be at least the spectrum's Rbkg."));
            }
            panel = panel
                .child(
                    button(
                        &t,
                        "rmc-prepare",
                        "Prepare initial fit",
                        self.rmc.control.is_none(),
                    )
                    .disabled(
                        self.rmc.control.is_some()
                            || self.rmc.builder_busy
                            || self.rmc.builder_error.is_some(),
                    )
                    .on_click(cx.listener(|app, _, _, cx| app.begin_rmc(true, None, cx))),
                )
                .child(
                    button(&t, "rmc-run", "Run RMC", true)
                        .disabled(
                            self.rmc.control.is_some()
                                || self.rmc.builder_busy
                                || self.rmc.builder_error.is_some(),
                        )
                        .on_click(cx.listener(|app, _, _, cx| app.begin_rmc(false, None, cx))),
                );
            let scene = self.rmc_structure_panel(false, cx);
            return out.child(
                div()
                    .flex_1()
                    .min_h_0()
                    .min_w_0()
                    .flex()
                    .child(panel)
                    .child(scene),
            );
        }
        out.child(self.rmc_results(cx))
    }
}

impl StudioApp {
    /// Read the visible input, including uncommitted edits, before submission.
    fn commit_rmc_fields(&mut self, cx: &mut Context<Self>) -> Result<(), String> {
        if self.rmc.fields.is_empty() {
            return Ok(());
        }
        let values: Vec<f64> = self
            .rmc
            .fields
            .iter()
            .map(|field| {
                field
                    .read(cx)
                    .pending_value(cx)
                    .map_err(|_| "Invalid numeric RMC setting.".to_string())?
                    .ok_or_else(|| "All RMC settings require explicit values.".to_string())
            })
            .collect::<Result<_, _>>()?;
        let mut draft = self.rmc.project.draft.clone();
        for (index, value) in values.into_iter().enumerate() {
            set_draft_field(&mut draft, index, value);
        }
        if self.rmc.texts.len() == 3 {
            draft.absorber_atoms = self.rmc.texts[0].read(cx).text().into();
            draft.fixed_atoms = self.rmc.texts[1].read(cx).text().into();
            draft.constraints.pairs = engine::pair_distances(self.rmc.texts[2].read(cx).text())?;
        }
        if draft.repeats != self.rmc.project.draft.repeats {
            self.rmc.project.configuration = None;
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
            ("Attempts / continuation", d.steps as f64),
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
            let field = cx
                .new(|cx| NumericField::new(label, "required", Some(value), kind, self.theme, cx));
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
        self.rebuild_rmc_plots(cx);
    }
    fn rebuild_rmc_plots(&mut self, cx: &mut Context<Self>) {
        let (Some(progress), Some(request)) = (&self.rmc.live, &self.rmc.request) else {
            return;
        };
        match result_plots(progress, request, self.theme) {
            Ok(plots) => {
                for (i, plot) in plots.into_iter().enumerate() {
                    if let Some(entity) = self.rmc.plots.get(i) {
                        entity.update(cx, |view, cx| view.set_plot_keep_view(plot, cx));
                    } else {
                        self.rmc
                            .plots
                            .push(plot_builder(plot).interactive().build(cx));
                    }
                }
            }
            Err(e) => self.rmc.error = Some(e),
        }
        self.rmc.scene_key = None;
    }
    fn rmc_results(&mut self, cx: &mut Context<Self>) -> gpui::Div {
        let t = self.theme;
        let mut out = div().flex_1().min_h_0().min_w_0().flex().flex_col();
        let mut actions = div().flex().flex_wrap().gap_2().px_3().py_2();
        for (i, label) in ["Fit plots", "Structure", "Convergence"]
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
        actions = actions.child(div().flex_1());
        if self.rmc.control.is_none() && self.rmc.project.saved.is_some() {
            actions = actions.child(
                button(&t, "rmc-resume-saved", "Resume / continue", true)
                    .on_click(cx.listener(|app, _, _, cx| app.resume_rmc(cx))),
            );
        }
        actions = actions
            .child(
                button(&t, "rmc-open-run", "Open checkpoint…", true)
                    .on_click(cx.listener(|app, _, _, cx| app.rmc_checkpoint_dialog(cx))),
            )
            .child(
                button(
                    &t,
                    "rmc-export",
                    "Export result…",
                    self.rmc.project.saved.is_some(),
                )
                .disabled(
                    self.rmc.project.saved.is_none()
                        || (self.rmc.control.is_some() && self.rmc.phase != "Paused"),
                )
                .on_click(cx.listener(|app, _, _, cx| app.export_rmc(cx))),
            );
        out = out.child(actions);
        let Some(p) = self.rmc.live.clone() else {
            return out.child(
                div()
                    .p_3()
                    .child("Prepare a structure and run RMC, or open a saved checkpoint."),
            );
        };
        let initial = p.initial.evaluation.score;
        let best = p.best.evaluation.score;
        let improvement = if initial > 0. {
            format!("{:.2}%", 100. * (initial - best) / initial)
        } else {
            "—".into()
        };
        let trend = match p.trend.status {
            ResidualTrendStatus::InsufficientHistory => "Insufficient history",
            ResidualTrendStatus::StillChanging => "Still changing",
            ResidualTrendStatus::ResidualPlateau => "Residual plateau",
        };
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
            p.elapsed_seconds / p.completed as f64
        } else {
            0.
        };
        let mut summary=div().px_3().py_2().flex().flex_col().gap_1().bg(t.surface)
            .child(format!("Initial {initial:.7}  →  best {best:.7}  ·  improvement {improvement}  ·  current {:.7}",p.current_score))
            .child(format!("{} · {trend} · {}/{} attempts · accepted {acceptance:.1}% · constraint rejected {}",self.rmc.phase,p.completed,p.limit,p.constraint_rejected))
            .child(format!("Active time {:.1}s · setup this session {:.1}s · inclusive {:.3}s/attempt · active path reuse {reuse}",p.elapsed_seconds,p.setup_seconds,sec))
            .child(div().text_size(px(11.5)).text_color(t.text_muted).child("Normalized R real + imaginary residual plus structural penalty. Reuse = reused active paths / (reused active + exact path calculations), current calculator only."));
        for fit in &p.best.evaluation.datasets {
            summary = summary.child(format!(
                "{}: spectral residual {:.7} · structural penalty {:.7}",
                fit.name, fit.score, p.best.penalty
            ));
        }
        if let Some(request) = &self.rmc.request
            && let rexafs::rmc::Objective::R(transform) = &request.problem.datasets[0].objective
        {
            summary = summary.child(format!("Objective window: k {:.2}–{:.2} Å⁻¹ · R {:.2}–{:.2} Å · k weight {}. Curves may extend outside the fitted region.",
                    transform.kmin, transform.kmax, transform.rmin, transform.rmax,
                    request.problem.datasets[0].exafs.kweight));
        }
        if let Some(window) = p.trend.windows.last() {
            let best_change = window
                .best_improvement
                .map_or_else(|| "—".into(), |v| format!("{v:.3e}"));
            let mean_change = window
                .mean_change
                .map_or_else(|| "—".into(), |v| format!("{v:.3e}"));
            summary = summary.child(format!("Recent {}–{}: best change {best_change}, mean change {mean_change} · plateau requires 3 passing 500-attempt comparisons after 3,000 attempts", window.first_step, window.last_step));
        }
        if let Some(path) = &self.rmc.recovery {
            summary = summary.child(div().text_size(px(11.5)).text_color(t.text_muted).child(
                format!(
                    "Recovery checkpoint: {} · saved at move boundaries every 30 seconds and on pause/stop",
                    path.display()
                ),
            ));
        }
        out = out.child(summary);
        if self.rmc.plot_tab == 1 {
            return out.child(self.rmc_structure_panel(true, cx));
        }
        if self.rmc.plot_tab == 2 {
            if let Some(plot) = self.rmc.plots.get(4) {
                out = out.child(div().flex_1().min_h_0().p_3().child(plot.clone()));
            }
            return out;
        }
        let mut grid = div().flex_1().min_h_0().flex().flex_col().gap_2().p_3();
        for pair in self.rmc.plots.iter().take(4).collect::<Vec<_>>().chunks(2) {
            let mut row = div().flex_1().min_h_0().min_w_0().flex().gap_2();
            for plot in pair {
                row = row.child(div().flex_1().min_w_0().min_h_0().child((*plot).clone()));
            }
            grid = grid.child(row);
        }
        out.child(grid)
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
        out = out
            .child(bar)
            .child(div().flex_1().min_h_0().min_w_0().child(canvas));
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
    let center = std::array::from_fn(|axis| {
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
    });
    let extent = c
        .atoms
        .iter()
        .map(|a| length(std::array::from_fn(|i| a.position[i] - center[i])))
        .fold(1.5, f64::max);
    let mut scene = MoleculeScene {
        center,
        extent,
        radius: extent,
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
) -> Result<Vec<Plot>, String> {
    use nalgebra::DVector;
    use rexafs::{fitting::transform::apply_kweight_transform, rmc::Objective};
    engine::validate_progress(p, request)?;
    let dataset = request
        .problem
        .datasets
        .first()
        .ok_or("Missing saved dataset")?;
    let Objective::R(transform) = &dataset.objective else {
        return Err("The desktop result viewer currently requires R space.".into());
    };
    let k = DVector::from_column_slice(&dataset.exafs.k);
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
        let a = apply_kweight_transform(&k, &DVector::from_column_slice(chi), transform, weight)
            .map_err(|e| e.to_string())?;
        kp = kp
            .line(k.as_slice(), a.chik.as_slice())
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
    let mut x: Vec<_> = p.history.iter().map(|s| s.step as f64).collect();
    let mut current: Vec<_> = p.history.iter().map(|s| s.score).collect();
    let mut best: Vec<_> = p.history.iter().map(|s| s.best_score).collect();
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
    let mut out = vec![kp];
    out.extend(rs);
    out.push(trend);
    Ok(out)
}
