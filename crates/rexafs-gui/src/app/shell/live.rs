//! Live acquisition lives inside Series; opening a saved session never starts it.
use super::*;
use crate::plot_ranges::plot_builder;
use crate::{
    live::*,
    live_intake::{CompletionPolicy, CompletionTracker, Observation},
    series_measurements::{MetricDefinition, SeriesDefinition, SeriesRun},
    widgets::text_input::TextInput,
};
use gpui::{AppContext, Entity};
use rexafs::prelude::Measurement;
use ruviz_gpui::RuvizPlot;
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
mod fit_trends;
mod monitor;
pub(crate) mod ui;
use monitor::LiveDisplay;
use ui::LiveMenu;

pub(crate) struct LiveState {
    pub open: bool,
    fields: Vec<Entity<TextInput>>,
    include_existing: bool,
    recursive: bool,
    recipe: Option<usize>,
    processing_reference: Option<crate::group_identity::GroupId>,
    peak_model: Option<usize>,
    exafs_model: Option<usize>,
    exafs_error: Option<String>,
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
    accepted_sources: std::collections::BTreeSet<PathBuf>,
    recoveries: Vec<(PathBuf, LiveConfig)>,
    menu: Option<LiveMenu>,
    menu_cursor: Option<usize>,
    menu_floating: bool,
    return_focus: Option<gpui::FocusHandle>,
    advanced: bool,
    show_recoveries: bool,
    menu_position: gpui::Point<gpui::Pixels>,
    menu_focus: Option<gpui::FocusHandle>,
    plotted_group: Option<crate::group_identity::GroupId>,
    monitor_docked: bool,
    monitor_window: Option<gpui::AnyWindowHandle>,
    monitor_plot: Option<Entity<RuvizPlot>>,
    held_frame: Option<crate::group_identity::GroupId>,
    held_average: Option<crate::params::DerivedSpectrum>,
    channel: Option<String>,
    display: LiveDisplay,
    plot_key: String,
    plot_generation: u64,
    show_issues: bool,
    channel_choices: Vec<String>,
    selected_channels: std::collections::BTreeSet<String>,
    merge: LiveMerge,
    average_errors: Vec<String>,
    average_dirty: bool,
    show_sources: bool,
    plot_error: Option<String>,
    plotted_label: String,
    plot_updating: bool,
    plot_value: Option<f64>,
    plotted_fit_status: Option<String>,
    fit_parameters: Vec<(String, f64, Option<f64>)>,
    fit_details: bool,
    fit_trend: usize,
    fit_trend_fields: Vec<Entity<TextInput>>,
    fit_trend_edit: bool,
    fit_trend_path: usize,
    fit_trend_error: String,
    fit_trend_errors: bool,
    fit_trend_cache: BTreeMap<String, Result<crate::series_fits::FitSummary, String>>,
    monitor_trend: Option<Entity<RuvizPlot>>,
    monitor_fit_curve: bool,
}
impl Default for LiveState {
    fn default() -> Self {
        Self {
            open: false,
            fields: Vec::new(),
            include_existing: false,
            recursive: false,
            recipe: None,
            processing_reference: None,
            peak_model: None,
            exafs_model: None,
            exafs_error: None,
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
            accepted_sources: Default::default(),
            recoveries: Vec::new(),
            plotted_group: None,
            menu: None,
            menu_cursor: None,
            menu_floating: false,
            return_focus: None,
            advanced: false,
            show_recoveries: false,
            menu_position: Default::default(),
            menu_focus: None,
            monitor_docked: false,
            monitor_window: None,
            monitor_plot: None,
            held_frame: None,
            held_average: None,
            channel: None,
            display: LiveDisplay::Latest,
            plot_key: String::new(),
            plot_generation: 0,
            show_issues: false,
            channel_choices: Vec::new(),
            selected_channels: Default::default(),
            merge: LiveMerge::Individual,
            average_errors: Vec::new(),
            average_dirty: false,
            show_sources: false,
            plot_error: None,
            plotted_label: String::new(),
            plot_updating: false,
            plot_value: None,
            plotted_fit_status: None,
            fit_parameters: Vec::new(),
            fit_details: false,
            fit_trend: 0,
            fit_trend_fields: Vec::new(),
            fit_trend_edit: false,
            fit_trend_path: 0,
            fit_trend_error: String::new(),
            fit_trend_errors: true,
            fit_trend_cache: BTreeMap::new(),
            monitor_trend: None,
            monitor_fit_curve: false,
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
                ("Scans per average", "10".into()),
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
            "{:?}|{}|{}|{:?}|{}|{:?}|{:?}|{:?}|{:?}|{:?}",
            self.live
                .fields
                .iter()
                .map(|f| f.read(cx).text())
                .collect::<Vec<_>>(),
            self.live.include_existing,
            self.live.recursive,
            self.live.recipe,
            self.processing_reference_settings(self.live.processing_reference.as_ref())
                .map(|p| p.fingerprint())
                .unwrap_or(0),
            (&self.live.processing_reference, self.selected),
            self.live.peak_model,
            self.live.selected_channels,
            self.live.merge,
            self.live.exafs_model
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
        let settings = if let Some(recipe) = recipe {
            recipe.settings.clone()
        } else {
            self.processing_reference_settings(self.live.processing_reference.as_ref())?
        };
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
            schema: 3,
            id: crate::group_identity::GroupId::new_result(),
            name: "Live acquisition".into(),
            folder: PathBuf::from(text(0)),
            pattern: text(1).into(),
            recursive: self.live.recursive,
            policy,
            settings,
            processing_reference: if recipe.is_some() {
                None
            } else {
                self.live
                    .processing_reference
                    .clone()
                    .or_else(|| self.selected.and_then(|i| self.group_id(i)))
                    .map(|id| {
                        let label = self.processing_reference_name(&id);
                        (id, label)
                    })
            },
            definition,
            layouts: Vec::new(),
            exafs: self
                .live
                .exafs_model
                .and_then(|i| self.fit_history.get(i))
                .map(ExafsRecipe::capture)
                .transpose()?,
            recipe: recipe.cloned(),
            peak_model: self
                .live
                .peak_model
                .and_then(|i| self.peaks.archive.models.get(i))
                .cloned(),
            merge: match self.live.merge {
                LiveMerge::Batches { .. } => LiveMerge::Batches {
                    scans: text(4)
                        .parse()
                        .map_err(|_| "Scans per average must be a whole number")?,
                },
                other => other,
            },
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
        if !self.live.channel_choices.is_empty() && self.live.selected_channels.is_empty() {
            self.live.message = "Choose at least one signal to preview".into();
            cx.notify();
            return;
        }
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
        let selected_channels = self.live.selected_channels.clone();
        self.live.generation += 1;
        let request = self.live.generation;
        let generation = self.project_generation;
        self.live.cancel = Arc::new(AtomicBool::new(false));
        let cancel = self.live.cancel.clone();
        self.live.preview = None;
        self.live.busy = true;
        self.live.message = "Checking completed files…".into();
        cx.spawn(async move |this, cx| {
            let (result, choices) = cx.background_spawn(async move {
                let mut choices = Vec::new();
                let result = (|| {
                if let Some(recipe)=&mut config.exafs {recipe.freeze_paths()?;}
                let mut exafs_preview=None;
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
                choices = first.signals.iter().map(|s| s.name.clone()).collect();
                for scan in &snapshot.measurement.scans {
                    for signal in preview_channels(scan, &selected_channels, reviewed.as_ref())? {
                        let layout = ScanLayout::capture_channel(&snapshot.measurement, scan, &signal)?;
                        if !config.layouts.contains(&layout) { config.layouts.push(layout); }
                        let sp = prepare_preview(scan, &signal.mapping, &config.settings, crate::series_measurements::required_stage(&config.definition))?;
                        if !config.definition.edge_energy { sp.measure(&config.definition.measurement).map_err(|e| format!("{}: {e}", signal.name))?; }
                        if let Some(recipe) = &config.exafs {
                            let spectrum=prepare_preview(scan,&signal.mapping,&config.settings,crate::params::RequiredStage::Background)?;
                            let directory=tempfile::tempdir().map_err(|e|e.to_string())?;
                            let result=recipe.evaluate(&spectrum,directory.path())?;
                            if result.solver_report.as_ref().is_some_and(|r|!r.converged) {return Err(format!("{}: EXAFS preview did not converge",signal.name));}
                            if exafs_preview.is_none(){exafs_preview=Some(result);}
                        }
                        if let Some(saved) = &config.peak_model {
                            let definition = crate::peak_fits::preparation_definition(&saved.model);
                            let spectrum = prepare_preview(scan, &signal.mapping, &config.settings, crate::series_measurements::required_stage(&definition))?;
                            let fit = saved.model.fit_with_progress(&spectrum, |_, _| !cancel.load(Ordering::Relaxed)).map_err(|e| e.to_string())?;
                            if !matches!(fit.termination, rexafs::prelude::PeakTermination::Converged | rexafs::prelude::PeakTermination::FixedModel) {
                                return Err(format!("{} peak preview: {}", signal.name, fit.termination_detail));
                            }
                        }
                    }
                }
                config.validate()?;
                let sp = prepare_preview(first, &config.layouts[0].mapping, &config.settings, crate::series_measurements::required_stage(&config.definition))?;
                let value = if config.definition.edge_energy { sp.e0().ok_or("Edge energy unavailable")? }
                    else { sp.measure(&config.definition.measurement).map_err(|e| e.to_string())?.value };
                let peak = if let Some(saved) = &config.peak_model {
                    let definition = crate::peak_fits::preparation_definition(&saved.model);
                    let spectrum = prepare_preview(first, &config.layouts[0].mapping, &config.settings, crate::series_measurements::required_stage(&definition))?;
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
                Ok::<_, String>((PreparedLive { config, baseline, signature, count: files.len() }, arrays.axis, arrays.signal, value, peak, exafs_preview))
                })();
                (result, choices)
            }).await;
            this.update(cx, |app, cx| {
                if app.project_generation != generation || app.live.generation != request { return; }
                app.live.busy = false;
                app.live.channel_choices = choices;
                match result {
                    Ok((mut preview, x, y, value, peak, exafs_preview)) => {
                        if app.live.selected_channels.is_empty() && app.live_signature(cx)==preview.signature {
                            app.live.selected_channels=preview.config.layouts.iter().map(|l|l.channel_name().to_owned()).collect();
                            preview.signature=app.live_signature(cx);
                        }
                        let (axis, signal) = live_axes(&preview.config.definition.measurement);
                        let base = ruviz::prelude::Plot::new().size(11., 3.3).theme(app.theme.plot_theme());
                        let plot: ruviz::prelude::Plot = if let Some(r) = &exafs_preview {
                            let indices:Vec<_>=r.k.iter().enumerate().filter(|(_,k)|**k>=r.kmin.unwrap_or(0.) && **k<=r.kmax.unwrap_or(f64::INFINITY)).map(|(i,_)|i).collect();
                            let axis:Vec<_>=indices.iter().map(|&i|r.k[i]).collect();
                            let data:Vec<_>=indices.iter().map(|&i|r.data_chi[i]*r.k[i].powf(r.kweight)).collect();
                            let model:Vec<_>=indices.iter().map(|&i|r.model_chi[i]*r.k[i].powf(r.kweight)).collect();
                            base.line(&axis,&data).label("Data").line(&axis,&model).label("EXAFS fit")
                                .xlabel("k (Å⁻¹)").ylabel(crate::plotting::chik_label(r.kweight)).legend_position(ruviz::prelude::LegendPosition::UpperRight).into()
                        } else if let Some(r) = &peak {
                            let data = super::peaks::peak_curve(base, &r.energy, &r.data, &r.source_indices, "Data", crate::plotting::trace_color(&app.theme, 0));
                            super::peaks::peak_curve(data, &r.energy, &r.model, &r.source_indices, "Peak model", crate::plotting::trace_color(&app.theme, 1))
                                .xlabel("Energy (eV)").ylabel(format!("{:?} μ(E)", r.definition.space))
                                .legend_position(ruviz::prelude::LegendPosition::UpperRight)
                        } else { base.line(&x, &y).xlabel(axis).ylabel(signal).into() };
                        app.live.plot = Some(plot_builder(plot).interactive().build(cx));
                        app.live.message = format!("{} existing files · {} · {} output(s) · {} preview {value:.5}", preview.count,
                            if app.live.include_existing { "included" } else { "new or changed files only" }, preview.config.layouts.iter().map(|l| l.channel_name()).collect::<std::collections::BTreeSet<_>>().len(), preview.config.layouts[0].channel_name());
                        if let Some(r)=exafs_preview {app.live.message.push_str(&format!(" · EXAFS R-factor {:.5}",r.r_factor));}
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
        self.live.plot_key.clear();
        self.live.channel = None;
        self.live.held_frame = None;
        self.live.held_average = None;
        self.live.monitor_plot = None;
        self.live.plot_error = None;
        self.live.plotted_label.clear();
        self.live.plot_value = None;
        self.live.plot_updating = false;
        self.live.plot_generation += 1;
        self.live.average_dirty = engine.store.config.merge != LiveMerge::Individual;
        self.live.display = if self.live.average_dirty {
            LiveDisplay::Average
        } else {
            LiveDisplay::Latest
        };
        self.live.progress = engine.progress();
        self.live.accepted_sources.clear();
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

    fn resume_live(&mut self, cx: &mut Context<Self>) {
        if self.live.engine.is_none() || self.live.busy {
            return;
        }
        self.live.cancel = Arc::new(AtomicBool::new(false));
        self.live.running = true;
        if self.live.session.is_some_and(|i| {
            self.measurements.archive.live_sessions[i]
                .config
                .exafs
                .is_some()
        }) {
            self.live.average_dirty = true;
        }
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
        let average_dirty = self.live.average_dirty;
        self.live.busy = true;
        cx.spawn(async move |this, cx| {
            let (engine, result, progress, committed_before_error, averages, exafs) = cx
                .background_spawn(async move {
                    let result = engine.poll(Instant::now(), 16, &cancel);
                    let progress = engine.progress();
                    let committed_before_error = if result.is_err() {
                        engine.store.records().unwrap_or_default()
                    } else {
                        Vec::new()
                    };
                    let averages = (average_dirty
                        || result.as_ref().is_ok_and(|r| !r.is_empty())
                        || !committed_before_error.is_empty())
                    .then(|| build_averages(&engine.store, &cancel));
                    let exafs = if averages.is_some() || engine.store.config.exafs.is_some() {
                        Some(build_exafs(
                            &engine.store,
                            averages
                                .as_ref()
                                .and_then(|a| a.as_ref().ok())
                                .map(Vec::as_slice)
                                .unwrap_or(&[]),
                            true,
                            &cancel,
                        ))
                    } else {
                        None
                    };
                    (
                        engine,
                        result,
                        progress,
                        committed_before_error,
                        averages,
                        exafs,
                    )
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
                    if let Some(averages) = averages {
                        app.publish_live_averages(averages, cx);
                    }
                    if let Some(exafs) = exafs {
                        app.publish_live_exafs(exafs, cx);
                    }
                    app.refresh_live_spectrum(cx);
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
            self.live.accepted_sources.insert(record.source.clone());
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
        }
        if changed {
            self.refresh_live_spectrum(cx);
        }
    }
    fn publish_live_averages(
        &mut self,
        averages: Result<Vec<LiveAverage>, String>,
        cx: &mut Context<Self>,
    ) {
        let averages = match averages {
            Ok(outputs) => outputs,
            Err(error) => {
                self.live.average_dirty = true;
                self.live.average_errors = vec![error];
                return;
            }
        };
        self.live.average_dirty = false;
        self.live.average_errors.clear();
        let mut changed = false;
        for average in averages {
            let mut group = match average.group {
                Ok(group) => group,
                Err(error) => {
                    self.live.average_errors.push(format!(
                        "{} · set {} · {} scans: {error}",
                        average.channel,
                        average.batch + 1,
                        average.count
                    ));
                    continue;
                }
            };
            if let Some(index) = self
                .derived
                .iter()
                .position(|g| g.group_id == group.group_id)
            {
                if self.derived[index].source == group.source {
                    continue;
                }
                group.id = self.derived[index].id;
                // An arriving scan must not overwrite the analyst's processing edits.
                group.params = self.derived[index].params.clone();
                self.derived[index] = group;
                let indices = std::collections::BTreeSet::from([crate::app::DERIVED_BASE + index]);
                crate::app::evict_group_keys(&mut self.cache, &indices, false);
                crate::app::evict_group_keys(&mut self.raw_cache, &indices, false);
            } else {
                group.id = self.next_group_id();
                self.derived.push(group);
            }
            changed = true;
        }
        if changed {
            self.rekey_after_catalog_change();
            self.refresh_live_spectrum(cx);
        }
    }
    fn publish_live_exafs(&mut self, rows: Result<Vec<ExafsRow>, String>, cx: &mut Context<Self>) {
        let Some(index) = self.live.session else {
            return;
        };
        match rows {
            Ok(rows) => {
                self.live.exafs_error = None;
                let session = &mut self.measurements.archive.live_sessions[index];
                let mut changed = false;
                for mut row in rows {
                    // Embedded projects relocate retained arrays. Match the
                    // displayed output's locator while preserving fit identity.
                    if let Some(source) = self.derived.iter().find_map(|g| {
                        (g.group_id.as_ref() == Some(&row.group))
                            .then_some(g.source.as_ref())
                            .flatten()
                    }) {
                        row.source = source.clone();
                    }
                    if let Some(saved) = session.exafs.iter_mut().find(|r| r.key == row.key) {
                        if saved.source != row.source || saved.artifact != row.artifact {
                            *saved = row;
                            changed = true;
                        }
                    } else {
                        session.exafs.push(row);
                        changed = true;
                    }
                }
                if changed {
                    self.live.plot_key.clear();
                    self.refresh_live_spectrum(cx);
                }
            }
            Err(error) => self.live.exafs_error = Some(error),
        }
        cx.notify();
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
                    let averages = build_averages(&engine.store, &AtomicBool::new(false));
                    let exafs = build_exafs(
                        &engine.store,
                        averages.as_ref().map(Vec::as_slice).unwrap_or(&[]),
                        false,
                        &AtomicBool::new(false),
                    );
                    Ok::<_, String>((engine, records, averages, exafs))
                })
                .await;
            this.update(cx, |app, cx| {
                if app.project_generation != generation {
                    return;
                }
                app.live.busy = false;
                match result {
                    Ok((engine, records, averages, exafs)) => {
                        app.attach_live(engine, cx);
                        app.publish_live(records, cx);
                        app.publish_live_averages(averages, cx);
                        app.publish_live_exafs(exafs, cx);
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
