//! Live acquisition lives inside Series; opening a saved session never starts it.
use super::*;
use crate::{
    live::*,
    live_intake::{CompletionPolicy, CompletionTracker, Observation},
    series_measurements::{MetricDefinition, SeriesDefinition, SeriesRun},
    widgets::text_input::TextInput,
};
use gpui::{AppContext, Entity};
use rexafs::prelude::Measurement;
use ruviz_gpui::{RuvizPlot, plot_builder};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

pub(crate) struct LiveState {
    pub open: bool,
    fields: Vec<Entity<TextInput>>,
    include_existing: bool,
    recursive: bool,
    recipe: Option<usize>,
    peak_model: Option<usize>,
    peak_menu: bool,
    preview: Option<PreparedLive>,
    plot: Option<Entity<RuvizPlot>>,
    trend: Option<Entity<RuvizPlot>>,
    message: String,
    busy: bool,
    running: bool,
    follow: bool,
    cancel: Arc<AtomicBool>,
    generation: u64,
    engine: Option<LiveEngine>,
    session: Option<usize>,
    progress: LiveProgress,
    recoveries: Vec<(PathBuf, LiveConfig)>,
    menu_open: bool,
    menu_position: gpui::Point<gpui::Pixels>,
    menu_focus: Option<gpui::FocusHandle>,
    plotted_group: Option<crate::group_identity::GroupId>,
}
impl Default for LiveState {
    fn default() -> Self {
        Self {
            open: false,
            fields: Vec::new(),
            include_existing: false,
            recursive: false,
            recipe: None,
            peak_model: None,
            peak_menu: false,
            preview: None,
            plot: None,
            trend: None,
            message: String::new(),
            busy: false,
            running: false,
            follow: true,
            cancel: Arc::new(AtomicBool::new(false)),
            generation: 0,
            engine: None,
            session: None,
            progress: LiveProgress::default(),
            recoveries: Vec::new(),
            plotted_group: None,
            menu_open: false,
            menu_position: Default::default(),
            menu_focus: None,
        }
    }
}
impl LiveState {
    pub fn stop(&mut self) {
        self.running = false;
        self.cancel.store(true, Ordering::Relaxed);
    }
}
impl Drop for LiveState {
    fn drop(&mut self) {
        self.stop();
    }
}

struct PreparedLive {
    config: LiveConfig,
    baseline: BTreeMap<PathBuf, String>,
    signature: String,
    count: usize,
}

impl StudioApp {
    pub(crate) fn open_live(&mut self, cx: &mut Context<Self>) {
        self.live.open = true;
        if self.live.fields.is_empty() {
            let folder = self
                .source_dir
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_default();
            for (name, text) in [
                ("Live folder", folder),
                ("Filename filter", "*.qd".into()),
                ("Stable observations", "3".into()),
                ("Check interval (seconds)", "1".into()),
            ] {
                self.live.fields.push(cx.new(|cx| {
                    let mut field = TextInput::new(name, text, self.theme, cx);
                    field.set_accessible_name(name);
                    field
                }));
            }
        }
        let generation = self.project_generation;
        cx.spawn(async move |this, cx| {
            let entries = cx
                .background_spawn(async { crate::live::root().and_then(|r| discover(&r)) })
                .await;
            this.update(cx, |app, cx| {
                if app.project_generation == generation {
                    app.live.recoveries = entries.unwrap_or_default();
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
        cx.notify();
    }
    fn live_signature(&self, cx: &Context<Self>) -> String {
        format!(
            "{:?}|{}|{}|{:?}|{}|{:?}|{:?}",
            self.live
                .fields
                .iter()
                .map(|f| f.read(cx).text())
                .collect::<Vec<_>>(),
            self.live.include_existing,
            self.live.recursive,
            self.live.recipe,
            self.selected
                .map(|i| self.effective_params(i))
                .unwrap_or(&self.params)
                .fingerprint(),
            self.selected,
            self.live.peak_model
        )
    }
    fn live_config(&self, cx: &Context<Self>) -> Result<LiveConfig, String> {
        let text = |i: usize| self.live.fields[i].read(cx).text().trim();
        let checks: u16 = text(2)
            .parse()
            .map_err(|_| "Stable observations must be a whole number")?;
        let seconds: f64 = text(3)
            .parse()
            .map_err(|_| "Check interval must be a number of seconds")?;
        if !seconds.is_finite() || !(0.1..=60.).contains(&seconds) {
            return Err("Use a check interval from 0.1 to 60 seconds".into());
        }
        let policy = CompletionPolicy::Quiet {
            checks,
            interval_ms: (seconds * 1000.).round() as u64,
        };
        policy.validate()?;
        let recipe = self
            .live
            .recipe
            .and_then(|i| self.measurements.archive.recipes.get(i));
        let settings = recipe.map(|r| r.settings.clone()).unwrap_or_else(|| {
            self.selected
                .map(|i| self.effective_params(i).clone())
                .unwrap_or_else(|| self.params.clone())
        });
        let definition = recipe
            .map(|r| r.definition.clone())
            .unwrap_or_else(|| MetricDefinition {
                id: crate::group_identity::GroupId::new_result(),
                revision: 1,
                name: "Flat mean −20…30 eV".into(),
                measurement: Measurement::mean(-20.0..=30.0).flat(),
                edge_energy: false,
                wavelet: None,
            });
        Ok(LiveConfig {
            schema: 1,
            id: crate::group_identity::GroupId::new_result(),
            name: "Live acquisition".into(),
            folder: PathBuf::from(text(0)),
            pattern: text(1).into(),
            recursive: self.live.recursive,
            policy,
            settings,
            definition,
            layouts: Vec::new(),
            recipe: recipe.cloned(),
            peak_model: self
                .live
                .peak_model
                .and_then(|i| self.peaks.archive.models.get(i))
                .cloned(),
            created: chrono::Utc::now().to_rfc3339(),
        })
    }
    fn choose_live_folder(&mut self, cx: &mut Context<Self>) {
        let generation = self.project_generation;
        let picker = cx.prompt_for_paths(gpui::PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("Watch folder".into()),
        });
        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(paths))) = picker.await
                && let Some(path) = paths.first()
            {
                this.update(cx, |app, cx| {
                    if app.project_generation == generation {
                        app.live.fields[0].update(cx, |field, cx| {
                            field.set_text(path.display().to_string(), cx)
                        });
                        app.live.preview = None;
                        cx.notify();
                    }
                })
                .ok();
            }
        })
        .detach();
    }
    fn preview_live(&mut self, cx: &mut Context<Self>) {
        let mut config = match self.live_config(cx) {
            Ok(c) => c,
            Err(e) => {
                self.live.message = e;
                cx.notify();
                return;
            }
        };
        let signature = self.live_signature(cx);
        let reviewed = self
            .selected
            .and_then(|i| i.checked_sub(crate::app::DERIVED_BASE))
            .and_then(|i| self.derived.get(i))
            .and_then(|g| g.operation.as_ref())
            .map(|op| op.parameters.clone());
        let include = self.live.include_existing;
        self.live.generation += 1;
        let request = self.live.generation;
        let generation = self.project_generation;
        self.live.cancel = Arc::new(AtomicBool::new(false));
        let cancel = self.live.cancel.clone();
        self.live.preview = None;
        self.live.busy = true;
        self.live.message = "Checking completed files…".into();
        cx.spawn(async move |this, cx| {
            let result = cx.background_spawn(async move {
                config.folder = config.folder.canonicalize().map_err(|e| e.to_string())?;
                let files = matching_files(&config.folder, &config.pattern, config.recursive)?;
                let path = files.first().ok_or("No matching sample yet. Place a completed representative file in this folder, then Preview.")?;
                let mut tracker = CompletionTracker::new(config.policy.clone())?;
                let mut snapshot = None;
                let CompletionPolicy::Quiet { checks, interval_ms } = config.policy else { unreachable!() };
                for observation in 0..checks {
                    if cancel.load(Ordering::Relaxed) { return Err("Preview cancelled".into()); }
                    if observation > 0 {
                        let until = Instant::now() + Duration::from_millis(interval_ms);
                        while Instant::now() < until {
                            if cancel.load(Ordering::Relaxed) { return Err("Preview cancelled".into()); }
                            std::thread::sleep(until.saturating_duration_since(Instant::now()).min(Duration::from_millis(50)));
                        }
                    }
                    match tracker.observe(path, Instant::now()) {
                        Observation::Ready(value) => { snapshot = Some(value); break; }
                        Observation::NeedsReview(e) | Observation::Unavailable(e) => return Err(e),
                        _ => {}
                    }
                }
                let snapshot = snapshot.ok_or("Sample is still changing; preview again after it completes")?;
                validate_recipe(config.recipe.as_deref(), &snapshot.bytes, &snapshot.source)?;
                let first = snapshot.measurement.scans.first().ok_or("No complete scans to preview")?;
                for scan in &snapshot.measurement.scans {
                    let mapping = preview_mapping(scan, reviewed.as_ref())?;
                    let layout = ScanLayout::capture(&snapshot.measurement, scan, mapping)?;
                    if !config.layouts.contains(&layout) { config.layouts.push(layout); }
                }
                config.validate()?;
                let (x, y) = first.arrays(Some(&preview_mapping(first, reviewed.as_ref())?)).map_err(|e| e.to_string())?;
                let sp = crate::params::prepare_arrays(x.clone(), y.clone(), &config.settings, crate::series_measurements::required_stage(&config.definition))?;
                let value = if config.definition.edge_energy { sp.e0().ok_or("Edge energy unavailable")? }
                    else { sp.measure(&config.definition.measurement).map_err(|e| e.to_string())?.value };
                let peak = if let Some(saved) = &config.peak_model {
                    let definition = crate::peak_fits::preparation_definition(&saved.model);
                    let spectrum = crate::params::prepare_arrays(x, y, &config.settings, crate::series_measurements::required_stage(&definition))?;
                    let result = saved.model.fit_with_progress(&spectrum, |_, _| !cancel.load(Ordering::Relaxed)).map_err(|e| e.to_string())?;
                    if !matches!(result.termination, rexafs::prelude::PeakTermination::Converged | rexafs::prelude::PeakTermination::FixedModel) {
                        return Err(format!("Peak preview: {}", result.termination_detail));
                    }
                    Some(result)
                } else { None };
                let mut baseline = BTreeMap::new();
                if !include { for path in &files {
                    if cancel.load(Ordering::Relaxed) { return Err("Preview cancelled".into()); }
                    use std::io::Read; use sha2::{Digest, Sha256};
                    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
                    if file.metadata().map_err(|e| e.to_string())?.len() > 256 * 1024 * 1024 { return Err("An existing file exceeds 256 MiB; narrow the filter".into()); }
                    let mut bytes = Vec::new(); file.take(256 * 1024 * 1024 + 1).read_to_end(&mut bytes).map_err(|e| e.to_string())?;
                    baseline.insert(path.clone(), Sha256::digest(&bytes).iter().map(|b| format!("{b:02x}")).collect());
                }}
                let arrays = config.definition.measurement.arrays(&sp).map_err(|e| e.to_string())?;
                Ok::<_, String>((PreparedLive { config, baseline, signature, count: files.len() }, arrays.axis, arrays.signal, value, peak))
            }).await;
            this.update(cx, |app, cx| {
                if app.project_generation != generation || app.live.generation != request { return; }
                app.live.busy = false;
                match result {
                    Ok((preview, x, y, value, peak)) => {
                        let (axis, signal) = live_axes(&preview.config.definition.measurement);
                        let base = ruviz::prelude::Plot::new().size(11., 3.3).theme(app.theme.plot_theme());
                        let plot: ruviz::prelude::Plot = if let Some(r) = &peak {
                            let data = super::peaks::peak_curve(base, &r.energy, &r.data, &r.source_indices, "Data", crate::plotting::trace_color(&app.theme, 0));
                            super::peaks::peak_curve(data, &r.energy, &r.model, &r.source_indices, "Peak model", crate::plotting::trace_color(&app.theme, 1))
                                .xlabel("Energy (eV)").ylabel(format!("{:?} μ(E)", r.definition.space))
                                .legend_position(ruviz::prelude::LegendPosition::UpperRight)
                        } else { base.line(&x, &y).xlabel(axis).ylabel(signal).into() };
                        app.live.plot = Some(plot_builder(plot).interactive().build(cx));
                        app.live.message = format!("{} existing files · {} · scalar preview {value:.5}", preview.count,
                            if app.live.include_existing { "included" } else { "new or changed files only" });
                        if let Some(r) = peak {
                            app.live.message.push_str(&format!(" · peak fit {:?}{}", r.termination,
                                if r.warnings.is_empty() { String::new() } else { format!(" · {}", r.warnings.join("; ")) }));
                        }
                        app.live.preview = Some(preview);
                    }
                    Err(e) => { app.live.message = e; app.live.plot = None; }
                }
                cx.notify();
            }).ok();
        }).detach();
        cx.notify();
    }
    fn start_live(&mut self, cx: &mut Context<Self>) {
        let signature = self.live_signature(cx);
        let Some(preview) = self.live.preview.take() else {
            return;
        };
        if signature != preview.signature {
            self.live.message = "Settings changed; Preview again before Start".into();
            cx.notify();
            return;
        }
        self.live.busy = true;
        let generation = self.project_generation;
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move {
                    let store =
                        LiveStore::create(&crate::live::root()?, preview.config, preview.baseline)?;
                    LiveEngine::open(store)
                })
                .await;
            this.update(cx, |app, cx| {
                if app.project_generation != generation {
                    return;
                }
                app.live.busy = false;
                match result {
                    Ok(engine) => {
                        app.attach_live(engine, cx);
                        app.resume_live(cx);
                    }
                    Err(e) => app.live.message = e,
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }
    fn attach_live(&mut self, engine: LiveEngine, cx: &mut Context<Self>) {
        let session = engine.store.session();
        let index = self
            .measurements
            .archive
            .live_sessions
            .iter()
            .position(|s| s.config.id == session.config.id)
            .unwrap_or_else(|| {
                self.measurements
                    .archive
                    .live_sessions
                    .push(session.clone());
                self.measurements.archive.live_sessions.len() - 1
            });
        if !self
            .measurements
            .archive
            .series
            .iter()
            .any(|s| s.id == session.series)
        {
            self.measurements.archive.series.push(SeriesDefinition {
                id: session.series.clone(),
                revision: 1,
                name: session.config.name.clone(),
                ordering: "Completed-file arrival (not acquisition time)".into(),
                frames: Vec::new(),
                coordinate: Default::default(),
            });
        }
        if !self
            .measurements
            .archive
            .runs
            .iter()
            .any(|r| r.id == session.run)
        {
            self.measurements.archive.runs.push(Arc::new(SeriesRun {
                id: session.run.clone(),
                series_id: session.series.clone(),
                series_revision: 1,
                definition: session.config.definition.clone(),
                created: session.config.created.clone(),
                software: env!("CARGO_PKG_VERSION").into(),
                recipe: session.config.recipe.clone(),
                rows: Vec::new(),
                complete: false,
                cancelled: false,
                coordinate: Default::default(),
            }));
        }
        if let Some(model) = &session.config.peak_model {
            let id = session.peak_run_id();
            if !self.peaks.archive.runs.iter().any(|r| r.id == id) {
                let mut run = crate::peak_fits::PeakRun::new(
                    format!("Live · {} · v{}", model.name, model.revision),
                    model.model.clone(),
                    &[],
                );
                run.id = id;
                run.created = session.config.created.clone();
                run.series = self
                    .measurements
                    .archive
                    .series
                    .iter()
                    .find(|s| s.id == session.series)
                    .cloned();
                self.peaks.archive.runs.push(Arc::new(run));
            }
        }
        self.live.plot = None;
        self.live.trend = None;
        self.live.plotted_group = None;
        self.live.progress = engine.progress();
        self.live.session = Some(index);
        self.live.engine = Some(engine);
        self.live.message = "Paused".into();
        cx.notify();
    }
    fn complete_live_run(&mut self) {
        if let Some(session) = self
            .live
            .session
            .and_then(|i| self.measurements.archive.live_sessions.get(i))
            && let Some(run) = self
                .peaks
                .archive
                .runs
                .iter_mut()
                .find(|r| r.id == session.peak_run_id())
        {
            Arc::make_mut(run).complete = true;
        }
        if let Some(session) = self
            .live
            .session
            .and_then(|i| self.measurements.archive.live_sessions.get(i))
            && let Some(run) = self
                .measurements
                .archive
                .runs
                .iter_mut()
                .find(|r| r.id == session.run)
        {
            Arc::make_mut(run).complete = true;
        }
    }

    pub(super) fn live_recipe_overlay(&self, cx: &mut Context<Self>) -> Option<gpui::AnyElement> {
        if !self.live.open || !self.live.menu_open {
            return None;
        }
        let focus = self.live.menu_focus.as_ref()?;
        let mut menu = div()
            .id("live-recipes")
            .w(px(360.))
            .max_h(px(420.))
            .overflow_y_scroll()
            .p_1()
            .flex()
            .flex_col()
            .gap_1()
            .rounded_lg()
            .bg(self.theme.surface)
            .border_1()
            .border_color(self.theme.border)
            .shadow_lg()
            .on_any_mouse_down(|_, _, cx| cx.stop_propagation());
        let peak = self.live.peak_menu;
        let options: Vec<_> = if peak {
            std::iter::once((None, "No peak fitting".to_owned()))
                .chain(
                    self.peaks
                        .archive
                        .models
                        .iter()
                        .enumerate()
                        .map(|(i, m)| (Some(i), format!("{} · v{}", m.name, m.revision))),
                )
                .collect()
        } else {
            std::iter::once((None, "Current processing · Flat mean −20…30 eV".to_owned()))
                .chain(
                    self.measurements
                        .archive
                        .recipes
                        .iter()
                        .enumerate()
                        .map(|(i, r)| (Some(i), format!("{} · v{}", r.name, r.revision))),
                )
                .collect()
        };
        for (index, (choice, label)) in options.into_iter().enumerate() {
            menu = menu.child(
                button(
                    &self.theme,
                    ("live-recipe-choice", index),
                    label,
                    if peak {
                        self.live.peak_model == choice
                    } else {
                        self.live.recipe == choice
                    },
                )
                .on_click(cx.listener(move |app, _, window, cx| {
                    if peak {
                        app.live.peak_model = choice;
                    } else {
                        app.live.recipe = choice;
                    }
                    app.live.preview = None;
                    app.live.menu_open = false;
                    app.operando_focus.focus(window, cx);
                    cx.notify();
                })),
            );
        }
        Some(
            div()
                .id("live-recipes-dismiss")
                .absolute()
                .inset_0()
                .occlude()
                .track_focus(focus)
                .on_key_down(cx.listener(|app, event: &gpui::KeyDownEvent, window, cx| {
                    if event.keystroke.key == "escape" {
                        app.live.menu_open = false;
                        app.operando_focus.focus(window, cx);
                        cx.notify();
                        cx.stop_propagation();
                    }
                }))
                .on_any_mouse_down(cx.listener(|app, _, window, cx| {
                    app.live.menu_open = false;
                    app.operando_focus.focus(window, cx);
                    cx.notify();
                    cx.stop_propagation();
                }))
                .child(
                    gpui::anchored()
                        .position(self.live.menu_position)
                        .offset(gpui::point(px(0.), px(15.)))
                        .snap_to_window()
                        .child(menu),
                )
                .into_any_element(),
        )
    }

    fn resume_live(&mut self, cx: &mut Context<Self>) {
        if self.live.engine.is_none() || self.live.busy {
            return;
        }
        self.live.cancel = Arc::new(AtomicBool::new(false));
        self.live.running = true;
        if let Some(i) = self.live.session {
            self.measurements.archive.live_sessions[i].stopped = false;
            let peak_id = self.measurements.archive.live_sessions[i].peak_run_id();
            if let Some(run) = self.peaks.archive.runs.iter_mut().find(|r| r.id == peak_id) {
                Arc::make_mut(run).complete = false;
            }
            let run_id = &self.measurements.archive.live_sessions[i].run;
            if let Some(run) = self
                .measurements
                .archive
                .runs
                .iter_mut()
                .find(|r| &r.id == run_id)
            {
                Arc::make_mut(run).complete = false;
            }
        }
        self.live.message = "Watching · completion inferred from quiet files".into();
        self.poll_live(cx);
    }
    fn poll_live(&mut self, cx: &mut Context<Self>) {
        let Some(mut engine) = self.live.engine.take() else {
            return;
        };
        if !self.live.running {
            self.live.engine = Some(engine);
            return;
        }
        let generation = self.project_generation;
        let request = self.live.generation;
        let cancel = self.live.cancel.clone();
        self.live.busy = true;
        cx.spawn(async move |this, cx| {
            let (engine, result, progress, committed_before_error) = cx
                .background_spawn(async move {
                    let result = engine.poll(Instant::now(), 16, &cancel);
                    let progress = engine.progress();
                    let committed_before_error = if result.is_err() {
                        engine.store.records().unwrap_or_default()
                    } else {
                        Vec::new()
                    };
                    (engine, result, progress, committed_before_error)
                })
                .await;
            let again = this
                .update(cx, |app, cx| {
                    if app.project_generation != generation || app.live.generation != request {
                        return false;
                    }
                    app.live.busy = false;
                    app.live.engine = Some(engine);
                    app.live.progress = progress;
                    match result {
                        Ok(records) => app.publish_live(records, cx),
                        Err(e) => {
                            app.publish_live(committed_before_error, cx);
                            app.live.stop();
                            app.live.message = format!("Paused: {e}");
                        }
                    }
                    if app
                        .live
                        .session
                        .is_some_and(|i| app.measurements.archive.live_sessions[i].stopped)
                    {
                        app.complete_live_run();
                    }
                    cx.notify();
                    app.live.running
                })
                .unwrap_or(false);
            if again {
                cx.background_executor()
                    .timer(Duration::from_millis(500))
                    .await;
                this.update(cx, |app, cx| {
                    if app.project_generation == generation && app.live.generation == request {
                        app.poll_live(cx);
                    }
                })
                .ok();
            }
        })
        .detach();
    }
    fn publish_live(&mut self, records: Vec<LiveRecord>, cx: &mut Context<Self>) {
        let Some(index) = self.live.session else {
            return;
        };
        let session = self.measurements.archive.live_sessions[index].clone();
        let Some(series_index) = self
            .measurements
            .archive
            .series
            .iter()
            .position(|s| s.id == session.series)
        else {
            return;
        };
        let Some(run_index) = self
            .measurements
            .archive
            .runs
            .iter()
            .position(|r| r.id == session.run)
        else {
            return;
        };
        let mut changed = false;
        for record in records {
            if !self.measurements.archive.live_sessions[index]
                .published
                .insert(record.key)
            {
                continue;
            }
            self.measurements.archive.live_sessions[index]
                .snapshots
                .insert(session.directory.join(format!("{}.raw", record.revision)));
            for mut frame in record.frames {
                frame.group.id = self.next_group_id();
                frame.row.frame.sequence =
                    self.measurements.archive.series[series_index].frames.len() + 1;
                if let Some(mut peak) = frame.peak
                    && let Some(run) = self
                        .peaks
                        .archive
                        .runs
                        .iter_mut()
                        .find(|r| r.id == session.peak_run_id())
                {
                    peak.sequence = frame.row.frame.sequence;
                    let run = Arc::make_mut(run);
                    run.complete = false;
                    run.rows.push(peak);
                    if let Some(series) = &mut run.series {
                        series.frames.push(frame.row.frame.clone());
                    }
                }
                self.measurements.archive.series[series_index]
                    .frames
                    .push(frame.row.frame.clone());
                Arc::make_mut(&mut self.measurements.archive.runs[run_index])
                    .rows
                    .push(frame.row);
                self.derived.push(frame.group);
                changed = true;
            }
        }
        if changed {
            self.refresh_live_peak_trend(&session.peak_run_id(), cx);
            self.record("Live acquisition: committed frames added", None);
            self.rekey_after_catalog_change();
            if self.live.follow
                && self.live.open
                && self.stage == Stage::Series
                && !self.derived.is_empty()
            {
                self.select_entry(crate::app::DERIVED_BASE + self.derived.len() - 1, cx);
            }
        }
        self.measurements.selected_series = Some(series_index);
        self.measurements.selected_run = Some(run_index);
        if changed || self.live.trend.is_none() {
            let run = &self.measurements.archive.runs[run_index];
            let values: Vec<_> = run
                .rows
                .iter()
                .map(|r| {
                    (
                        r.frame.sequence as f64,
                        r.result.as_ref().map_or(f64::NAN, |v| v.value),
                    )
                })
                .collect();
            let x: Vec<_> = values.iter().map(|v| v.0).collect();
            let y: Vec<_> = values.iter().map(|v| v.1).collect();
            if !x.is_empty() {
                let plot: ruviz::prelude::Plot = ruviz::prelude::Plot::new()
                    .size(11., 3.3)
                    .theme(self.theme.plot_theme())
                    .line(&x, &y)
                    .xlabel("Completed frame")
                    .ylabel(run.definition.name.clone())
                    .into();
                self.live.trend = Some(plot_builder(plot).interactive().build(cx));
            }
        }
    }
    fn recover_live(&mut self, directory: PathBuf, cx: &mut Context<Self>) {
        if self.live.busy {
            return;
        }
        self.live.stop();
        self.live.engine = None;
        self.live.busy = true;
        let generation = self.project_generation;
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move {
                    let engine = LiveEngine::open(LiveStore::open(&directory)?)?;
                    let records = engine.store.records()?;
                    Ok::<_, String>((engine, records))
                })
                .await;
            this.update(cx, |app, cx| {
                if app.project_generation != generation {
                    return;
                }
                app.live.busy = false;
                match result {
                    Ok((engine, records)) => {
                        app.attach_live(engine, cx);
                        app.publish_live(records, cx);
                        app.live.message = "Recovered · paused until Resume".into();
                    }
                    Err(e) => app.live.message = e,
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }
    fn refresh_live_spectrum(&mut self, cx: &mut Context<Self>) {
        let Some(session) = self
            .live
            .session
            .and_then(|i| self.measurements.archive.live_sessions.get(i))
        else {
            return;
        };
        let Some(index) = self
            .selected
            .and_then(|i| i.checked_sub(crate::app::DERIVED_BASE))
        else {
            return;
        };
        let Some(group) = self.derived.get(index).cloned() else {
            return;
        };
        if group.group_id == self.live.plotted_group {
            return;
        }
        self.live.plotted_group = group.group_id.clone();
        self.live.plot = None;
        let identity = group.group_id.clone();
        let definition = session.config.definition.clone();
        let settings = group
            .params
            .clone()
            .unwrap_or_else(|| session.config.settings.clone());
        let generation = self.project_generation;
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move {
                    let spectrum = group.prepare(
                        &settings,
                        crate::series_measurements::required_stage(&definition),
                    )?;
                    let arrays = definition
                        .measurement
                        .arrays(&spectrum)
                        .map_err(|e| e.to_string())?;
                    Ok::<_, String>((arrays, definition.measurement, group.label))
                })
                .await;
            this.update(cx, |app, cx| {
                if app.project_generation != generation || app.live.plotted_group != identity {
                    return;
                }
                match result {
                    Ok((arrays, measurement, label)) => {
                        let (axis, signal) = live_axes(&measurement);
                        let plot: ruviz::prelude::Plot = ruviz::prelude::Plot::new()
                            .size(11., 3.3)
                            .theme(app.theme.plot_theme())
                            .line(&arrays.axis, &arrays.signal)
                            .title(label)
                            .xlabel(axis)
                            .ylabel(signal)
                            .into();
                        app.live.plot = Some(plot_builder(plot).interactive().build(cx));
                    }
                    Err(e) => app.live.message = format!("Selected spectrum: {e}"),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }
    pub(crate) fn live_center(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        self.refresh_live_spectrum(cx);
        let t = self.theme;
        let running = self.live.running;
        let active = self.live.session.is_some();
        let mut view = div()
            .id("live-workspace")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap_3()
            .p_3()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        button(&t, "live-back", "← Series", false).on_click(cx.listener(
                            |app, _, _, cx| {
                                app.live.open = false;
                                if let Some(index) = app.measurements.selected_series {
                                    app.choose_overview_series(index, cx);
                                }
                                cx.notify();
                            },
                        )),
                    )
                    .child(div().text_size(px(16.)).child("Live acquisition")),
            );
        if !active {
            view = view
                .child(
                    div()
                        .flex()
                        .gap_2()
                        .child(div().flex_1().child(self.live.fields[0].clone()))
                        .child(
                            button(&t, "live-folder", "Choose folder…", false)
                                .on_click(cx.listener(|app, _, _, cx| app.choose_live_folder(cx))),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child("Filter")
                        .child(div().w(px(170.)).child(self.live.fields[1].clone()))
                        .child(
                            button(
                                &t,
                                "live-existing",
                                "Include existing",
                                self.live.include_existing,
                            )
                            .on_click(cx.listener(|app, _, _, cx| {
                                app.live.include_existing = !app.live.include_existing;
                                cx.notify();
                            })),
                        )
                        .child(
                            button(&t, "live-recursive", "Subfolders", self.live.recursive)
                                .on_click(cx.listener(|app, _, _, cx| {
                                    app.live.recursive = !app.live.recursive;
                                    cx.notify();
                                })),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child("Quiet file:")
                        .child(div().w(px(65.)).child(self.live.fields[2].clone()))
                        .child("checks, every")
                        .child(div().w(px(65.)).child(self.live.fields[3].clone()))
                        .child("seconds · inferred completion"),
                );
            let recipe_label = self
                .live
                .recipe
                .and_then(|i| self.measurements.archive.recipes.get(i))
                .map(|r| format!("{} · v{}", r.name, r.revision))
                .unwrap_or_else(|| "Current processing · Flat mean −20…30 eV".into());
            view = view
                .child(
                    button(&t, "live-recipe", format!("{recipe_label} ▾"), false).on_click(
                        cx.listener(|app, event: &gpui::ClickEvent, window, cx| {
                            app.live.peak_menu = false;
                            app.live.menu_open = true;
                            app.live.menu_position = event.position();
                            app.live
                                .menu_focus
                                .get_or_insert_with(|| cx.focus_handle())
                                .focus(window, cx);
                            cx.notify();
                        }),
                    ),
                )
                .child(
                    button(
                        &t,
                        "live-peak-model",
                        format!(
                            "Peak fit: {} ▾",
                            self.live
                                .peak_model
                                .and_then(|i| self.peaks.archive.models.get(i))
                                .map(|m| format!("{} · v{}", m.name, m.revision))
                                .unwrap_or_else(|| "Off".into())
                        ),
                        false,
                    )
                    .on_click(cx.listener(
                        |app, event: &gpui::ClickEvent, window, cx| {
                            app.live.peak_menu = true;
                            app.live.menu_open = true;
                            app.live.menu_position = event.position();
                            app.live
                                .menu_focus
                                .get_or_insert_with(|| cx.focus_handle())
                                .focus(window, cx);
                            cx.notify();
                        },
                    )),
                )
                .child(
                    div()
                        .flex()
                        .gap_2()
                        .child(
                            button(&t, "live-preview", "Preview", false)
                                .when(self.live.busy, |d| d.disabled(true))
                                .on_click(cx.listener(|app, _, _, cx| app.preview_live(cx))),
                        )
                        .child(
                            button(&t, "live-start", "Start", true)
                                .when(self.live.busy || self.live.preview.is_none(), |d| {
                                    d.disabled(true)
                                })
                                .on_click(cx.listener(|app, _, _, cx| app.start_live(cx))),
                        ),
                );
        } else {
            view = view.child(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        button(
                            &t,
                            "live-pause",
                            if running { "Pause" } else { "Resume" },
                            running,
                        )
                        .w(px(85.))
                        .on_click(cx.listener(|app, _, _, cx| {
                            if app.live.running {
                                app.live.stop();
                                app.live.message = "Paused".into();
                            } else {
                                app.resume_live(cx);
                            }
                            cx.notify();
                        })),
                    )
                    .child(button(&t, "live-stop", "Stop", false).on_click(cx.listener(
                        |app, _, _, cx| {
                            app.live.stop();
                            if let Some(i) = app.live.session {
                                app.measurements.archive.live_sessions[i].stopped = true;
                            }
                            if !app.live.busy {
                                app.complete_live_run();
                            }
                            app.live.message = "Stopped · results retained".into();
                            cx.notify();
                        },
                    )))
                    .child(
                        button(&t, "live-retry", "Retry", false)
                            .when(self.live.busy, |d| d.disabled(true))
                            .on_click(cx.listener(|app, _, _, cx| {
                                if let Some(engine) = &mut app.live.engine {
                                    engine.retry();
                                }
                                cx.notify();
                            })),
                    )
                    .child(
                        button(&t, "live-follow", "Follow latest", self.live.follow).on_click(
                            cx.listener(|app, _, _, cx| {
                                app.live.follow = !app.live.follow;
                                cx.notify();
                            }),
                        ),
                    )
                    .child(
                        button(&t, "live-new", "New recipe…", false)
                            .when(running || self.live.busy, |d| d.disabled(true))
                            .on_click(cx.listener(|app, _, _, cx| {
                                app.live.engine = None;
                                app.live.session = None;
                                app.live.preview = None;
                                app.live.plot = None;
                                app.live.plotted_group = None;
                                app.live.trend = None;
                                app.live.message.clear();
                                cx.notify();
                            })),
                    ),
            );
            let p = &self.live.progress;
            view = view.child(format!(
                "{} files · {} completed · {} waiting · {} queued · {} need review · {} unavailable",
                p.discovered, p.completed,
                p.waiting,
                p.queued,
                p.review.len(),
                p.failed.len()
            ));
            if let Some(session) = self
                .live
                .session
                .and_then(|i| self.measurements.archive.live_sessions.get(i))
                && let Some((index, run)) = self
                    .peaks
                    .archive
                    .runs
                    .iter()
                    .enumerate()
                    .find(|(_, r)| r.id == session.peak_run_id())
            {
                let succeeded = run
                    .rows
                    .iter()
                    .filter(|r| r.status == crate::series_measurements::FrameStatus::Succeeded)
                    .count();
                view = view.child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(format!(
                            "Peak fits: {succeeded} / {} succeeded",
                            run.rows.len()
                        ))
                        .child(
                            button(&t, "live-peak-results", "Inspect peak fits…", false)
                                .disabled(run.rows.is_empty())
                                .on_click(
                                    cx.listener(move |app, _, _, cx| app.open_peak_run(index, cx)),
                                ),
                        ),
                );
                for row in run
                    .rows
                    .iter()
                    .filter(|r| r.status != crate::series_measurements::FrameStatus::Succeeded)
                    .take(5)
                {
                    view = view.child(div().text_color(t.warn).child(format!(
                        "{} · {}",
                        row.label,
                        row.reason.as_deref().unwrap_or("Peak fit unavailable")
                    )));
                }
            }
        }
        view = view.child(
            div()
                .text_color(t.text_muted)
                .child(self.live.message.clone()),
        );
        if let Some(plot) = self.live.plot.clone() {
            view = view.child(div().w_full().h(px(310.)).flex_none().child(plot));
        }
        if let Some(plot) = self.live.trend.clone() {
            view = view.child(div().w_full().h(px(280.)).flex_none().child(plot));
        }
        for (index, (path, reason)) in self
            .live
            .progress
            .review
            .iter()
            .chain(&self.live.progress.failed)
            .take(20)
            .enumerate()
        {
            let path = path.clone();
            let label = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            view = view.child(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        button(&t, ("live-review", index), label, false).on_click(cx.listener(
                            move |app, _, _, cx| app.open_measurement_path(path.clone(), false, cx),
                        )),
                    )
                    .child(div().text_color(t.warn).child(reason.clone())),
            );
        }
        if let Some(session) = self
            .live
            .session
            .and_then(|i| self.measurements.archive.live_sessions.get(i))
            && let Some(run) = self
                .measurements
                .archive
                .runs
                .iter()
                .find(|r| r.id == session.run)
        {
            for row in run.rows.iter().filter(|r| r.result.is_none()).take(10) {
                view = view.child(div().text_color(t.warn).child(format!(
                    "{}: {}",
                    row.frame.label,
                    row.reason.as_deref().unwrap_or("Processing unavailable")
                )));
            }
        }
        if !running {
            for (index, (directory, config)) in self.live.recoveries.iter().take(8).enumerate() {
                let directory = directory.clone();
                view = view.child(
                    button(
                        &t,
                        ("live-recover", index),
                        format!("Open paused: {} · {}", config.name, config.created),
                        false,
                    )
                    .when(self.live.busy, |d| d.disabled(true))
                    .on_click(
                        cx.listener(move |app, _, _, cx| app.recover_live(directory.clone(), cx)),
                    ),
                );
            }
        }
        view.into_any_element()
    }
}

fn live_axes(measurement: &Measurement) -> (&'static str, String) {
    use rexafs::prelude::MeasurementSpace;
    match measurement.space {
        MeasurementSpace::Mu => ("Energy (eV)", "μ(E)".into()),
        MeasurementSpace::Norm => ("Energy (eV)", "Normalized μ(E)".into()),
        MeasurementSpace::Flat => ("Energy (eV)", "Flattened μ(E)".into()),
        MeasurementSpace::Chi { kweight } => {
            ("k (Å⁻¹)", crate::plotting::chik_label(kweight as f64))
        }
        MeasurementSpace::Fourier => ("R (Å)", "|χ(R)|".into()),
    }
}
