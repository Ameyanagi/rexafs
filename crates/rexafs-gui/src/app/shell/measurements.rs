//! Full-frame measurements use retained identities, not overview sample indices.
use crate::{
    app::{DERIVED_BASE, NO_ENTRY, StudioApp},
    series_measurements::*,
    widgets::numeric_field::{FieldKind, NumericField},
};
use gpui::{AppContext, Context, Entity};
use rexafs::prelude::{AxisOrigin, Measurement, MeasurementSpace, Metric};
use ruviz::prelude::Plot;
use ruviz_gpui::{RuvizPlot, plot_builder};
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

mod manage;
mod presets;
mod recipes;
mod reliability;
mod view;

pub(crate) struct MeasurementState {
    pub archive: SeriesArchive,
    pub overview: bool,
    pub selected_series: Option<usize>,
    pub selected_run: Option<usize>,
    kind: usize,
    initial_range: (f64, f64),
    space: MeasurementSpace,
    relative: bool,
    fields: Vec<Entity<NumericField>>,
    message: String,
    preview_index: usize,
    page: usize,
    scroll: gpui::ScrollHandle,
    preview: Option<Entity<RuvizPlot>>,
    preview_label: String,
    plot: Option<Entity<RuvizPlot>>,
    generation: u64,
    preview_generation: u64,
    cancel: Option<Arc<AtomicBool>>,
    progress: (usize, usize),
    checking: bool,
    stale: Vec<bool>,
    editor: Option<manage::SeriesEditor>,
    trend_axis: TrendAxis,
    preset_name: Option<Entity<crate::widgets::text_input::TextInput>>,
    selected_preset: Option<crate::group_identity::GroupId>,
    selected_recipe: Option<(crate::group_identity::GroupId, u64)>,
    pick_range: Option<Vec<f64>>,
    monitoring: bool,
    recovery_entries: Vec<recovery::RecoveryEntry>,
    recovery_busy: bool,
}

impl Default for MeasurementState {
    fn default() -> Self {
        Self::from_archive(SeriesArchive::default())
    }
}
impl MeasurementState {
    pub fn acknowledge_saved(
        &mut self,
        ids: &std::collections::BTreeSet<crate::group_identity::GroupId>,
    ) {
        self.recovery_entries
            .retain(|entry| !ids.contains(&entry.run));
    }

    pub fn from_archive(archive: SeriesArchive) -> Self {
        let selected_series = archive.series.len().checked_sub(1);
        let selected_run = selected_series.and_then(|index| {
            let series = &archive.series[index];
            archive
                .runs
                .iter()
                .rposition(|run| run.series_id == series.id)
        });
        let definition = selected_run.map(|i| &archive.runs[i].definition);
        let kind = definition
            .map(|d| {
                if d.edge_energy {
                    4
                } else {
                    match d.measurement.metric {
                        Metric::Point { .. } => 0,
                        Metric::Maximum { .. } => 1,
                        Metric::Integral { .. } => 2,
                        _ => 3,
                    }
                }
            })
            .unwrap_or(1);
        let space = definition
            .map(|d| d.measurement.space)
            .unwrap_or(MeasurementSpace::Norm);
        let relative = definition
            .map(|d| d.measurement.origin == AxisOrigin::E0)
            .unwrap_or(true);
        let initial_range = definition
            .map(|d| d.measurement.metric.bounds())
            .unwrap_or((-20., 50.));
        let selected_preset = definition.and_then(|d| {
            archive
                .presets
                .iter()
                .find(|p| {
                    p.id == d.id && p.measurement == d.measurement && p.edge_energy == d.edge_energy
                })
                .map(|p| p.id.clone())
        });
        let selected_recipe = selected_run
            .and_then(|i| archive.runs[i].recipe.as_ref())
            .map(|r| (r.id.clone(), r.revision));
        Self {
            archive,
            overview: false,
            selected_series,
            selected_run,
            kind,
            initial_range,
            space,
            relative,
            fields: vec![],
            message: String::new(),
            preview_index: 0,
            page: 0,
            scroll: gpui::ScrollHandle::new(),
            preview: None,
            preview_label: String::new(),
            plot: None,
            generation: 0,
            preview_generation: 0,
            cancel: None,
            progress: (0, 0),
            checking: false,
            stale: vec![],
            editor: None,
            trend_axis: TrendAxis::Sequence,
            preset_name: None,
            selected_preset,
            selected_recipe,
            pick_range: None,
            monitoring: false,
            recovery_entries: vec![],
            recovery_busy: false,
        }
    }
    pub fn stop(&mut self) {
        if let Some(cancel) = self.cancel.take() {
            cancel.store(true, Ordering::Relaxed);
        }
        self.generation += 1;
        self.preview_generation += 1;
    }
}

impl StudioApp {
    fn measurement_input(
        &self,
        group: &crate::group_identity::GroupId,
        label: String,
    ) -> FrameInput {
        let index = self.group_registry.index(group).or_else(|| {
            self.standalone_source
                .as_ref()
                .filter(|(_, _, id)| id == group)
                .map(|_| NO_ENTRY)
        });
        let Some(ix) = index.filter(|ix| self.valid_group_index(*ix)) else {
            return FrameInput {
                group: group.clone(),
                label,
                path: PathBuf::new(),
                derived: None,
                settings: Default::default(),
                recipe: None,
            };
        };
        let derived = (ix >= DERIVED_BASE && ix != NO_ENTRY)
            .then(|| self.derived.get(ix - DERIVED_BASE).cloned())
            .flatten()
            .map(Arc::new);
        let path = if ix < self.catalog.len() {
            self.catalog.path(ix)
        } else {
            self.current_path.clone()
        };
        FrameInput {
            group: group.clone(),
            label,
            path,
            derived,
            settings: self.effective_params(ix).clone(),
            recipe: None,
        }
    }

    fn create_measurement_series(&mut self, source: usize, cx: &mut Context<Self>) {
        if self.measurements.cancel.is_some() {
            return;
        }
        let indices: Vec<usize> = match source {
            0 => self.selection.iter().copied().collect(),
            1 => self
                .active_scan
                .and_then(|i| self.catalog.scans.get(i))
                .map(|s| (s.start..s.start + s.len).collect())
                .unwrap_or_default(),
            _ => (0..self.catalog.len())
                .chain((0..self.derived.len()).map(|i| DERIVED_BASE + i))
                .chain(
                    self.standalone_source
                        .as_ref()
                        .filter(|_| self.catalog.is_empty())
                        .map(|_| NO_ENTRY),
                )
                .collect(),
        };
        let mut seen = std::collections::BTreeSet::new();
        let frames: Vec<_> = indices
            .into_iter()
            .filter_map(|ix| {
                let group = self.group_id(ix)?;
                if !seen.insert(group.clone()) {
                    return None;
                }
                Some(SeriesFrame {
                    id: crate::group_identity::GroupId::new_result(),
                    group,
                    label: self.entry_label(ix),
                    sequence: 0,
                    coordinate: None,
                    acquired_at: None,
                })
            })
            .enumerate()
            .map(|(i, mut frame)| {
                frame.sequence = i + 1;
                frame
            })
            .collect();
        if frames.is_empty() {
            self.measurements.message = "Choose at least one available group.".into();
            cx.notify();
            return;
        }
        let name = format!("Series {}", self.measurements.archive.series.len() + 1);
        self.measurements.archive.series.push(SeriesDefinition {
            id: crate::group_identity::GroupId::new_result(),
            revision: 1,
            name,
            ordering: "Current group order (explicit sequence)".into(),
            coordinate: Default::default(),
            frames,
        });
        self.measurements.selected_series = Some(self.measurements.archive.series.len() - 1);
        self.measurements.editor = None;
        self.measurements.selected_run = None;
        self.measurements.plot = None;
        self.measurements.page = 0;
        self.measurements.preview_index = 0;
        self.measurements.message =
            "Membership saved. Review a measurement, then calculate every frame.".into();
        self.preview_measurement(cx);
        cx.notify();
    }

    fn measurement_definition(&self, cx: &Context<Self>) -> Result<MetricDefinition, String> {
        let kind = self.measurements.kind;
        let start = if kind == 4 {
            0.
        } else {
            self.measurements
                .fields
                .first()
                .and_then(|f| f.read(cx).value())
                .ok_or("Enter a point or range start")?
        };
        let end = if kind == 0 || kind == 4 {
            start + 1.
        } else {
            self.measurements
                .fields
                .get(1)
                .and_then(|f| f.read(cx).value())
                .ok_or("Enter a range end")?
        };
        if kind != 0 && kind != 4 && end <= start {
            return Err("Range end must be greater than start".into());
        }
        let metric = match kind {
            0 => Metric::Point { x: start },
            1 => Metric::Maximum { start, end },
            2 => Metric::Integral { start, end },
            _ => Metric::Mean { start, end },
        };
        let name = [
            "Point",
            "Region maximum",
            "Region integral",
            "Region mean",
            "Absolute E₀",
        ][kind]
            .to_string();
        let mut definition = MetricDefinition {
            id: crate::group_identity::GroupId::new_result(),
            revision: 1,
            name,
            measurement: Measurement {
                metric,
                space: self.measurements.space,
                origin: if self.measurements.relative {
                    AxisOrigin::E0
                } else {
                    AxisOrigin::Absolute
                },
            },
            edge_energy: kind == 4,
        };
        let recipe = self.selected_analysis_recipe();
        if let Some(old) = recipe
            .as_ref()
            .map(|r| &r.definition)
            .or_else(|| {
                self.measurements
                    .archive
                    .presets
                    .iter()
                    .find(|p| Some(&p.id) == self.measurements.selected_preset.as_ref())
            })
            .or_else(|| {
                self.measurements
                    .selected_run
                    .and_then(|i| self.measurements.archive.runs.get(i))
                    .map(|r| &r.definition)
            })
        {
            definition.id = old.id.clone();
            if self.measurements.selected_preset.as_ref() == Some(&old.id) {
                definition.name = old.name.clone();
            }
            definition.revision = self.measurements.archive.definition_revision(&definition);
        }
        Ok(definition)
    }

    fn preview_measurement(&mut self, cx: &mut Context<Self>) {
        let Some(series) = self
            .measurements
            .selected_series
            .and_then(|i| self.measurements.archive.series.get(i))
        else {
            return;
        };
        let Some(frame) = series.frames.get(
            self.measurements
                .preview_index
                .min(series.frames.len().saturating_sub(1)),
        ) else {
            return;
        };
        let input = self
            .measurement_input(&frame.group, frame.label.clone())
            .with_recipe(self.selected_analysis_recipe());
        let definition = match self.measurement_definition(cx) {
            Ok(d) => d,
            Err(e) => {
                self.measurements.message = e;
                return;
            }
        };
        self.queue_measurement_preview(input, definition, None, cx);
    }

    fn preview_retained_measurement(&mut self, row_index: usize, cx: &mut Context<Self>) {
        self.measurements
            .scroll
            .set_offset(gpui::point(gpui::px(0.), gpui::px(0.)));
        let Some(run) = self
            .measurements
            .selected_run
            .and_then(|i| self.measurements.archive.runs.get(i))
        else {
            return;
        };
        let Some(row) = run.rows.get(row_index) else {
            return;
        };
        let mut input = self.measurement_input(&row.frame.group, row.frame.label.clone());
        input = input.with_recipe(run.recipe.clone());
        input.settings = (*row.settings).clone();
        let definition = run.definition.clone();
        let revision = row.input_revision.clone();
        self.queue_measurement_preview(input, definition, revision, cx);
    }

    fn queue_measurement_preview(
        &mut self,
        input: FrameInput,
        definition: MetricDefinition,
        expected: Option<String>,
        cx: &mut Context<Self>,
    ) {
        self.measurements.preview_generation += 1;
        let request = self.measurements.preview_generation;
        let generation = self.project_generation;
        self.measurements.preview = None;
        self.measurements.preview_label = format!("Preparing {}…", input.label);
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move {
                    let (sp, _, _) = input.prepare(&definition, expected.as_deref())?;
                    let mut config = definition.measurement.clone();
                    if definition.edge_energy {
                        config = Measurement::point(0.);
                    }
                    let rexafs::prelude::MeasurementArrays {
                        axis: x,
                        signal: y,
                        e0_ev: e0,
                    } = config.arrays(&sp).map_err(|e| e.to_string())?;
                    let metric = config.resolved_metric(e0).map_err(|e| e.to_string())?;
                    let measured = rexafs::xafs::analysis::metrics::measure(&x, &y, metric)
                        .map_err(|e| e.to_string())?;
                    Ok::<_, String>((
                        input.label,
                        x,
                        y,
                        metric,
                        if definition.edge_energy {
                            e0.ok_or("E₀ unavailable")?
                        } else {
                            measured.value
                        },
                        config.space,
                        metric.bounds().0 - config.metric.bounds().0,
                    ))
                })
                .await;
            this.update(cx, |app, cx| {
                if app.project_generation != generation
                    || app.measurements.preview_generation != request
                {
                    return;
                }
                match result {
                    Ok((label, x, y, metric, value, space, origin)) => {
                        let (lo, hi) = metric.bounds();
                        let mut plot: Plot = Plot::new()
                            .theme(app.theme.plot_theme())
                            .line(&x, &y)
                            .into();
                        let color = ruviz::render::Color::from_gray(170);
                        plot = plot.vline_styled(lo, color, 1.3, ruviz::render::LineStyle::Dashed);
                        if hi != lo {
                            plot =
                                plot.vline_styled(hi, color, 1.3, ruviz::render::LineStyle::Dashed);
                        }
                        plot = plot
                            .xlabel(axis_label(space))
                            .ylabel(space_label(space))
                            .size_px(720, 300)
                            .major_ticks_x(5);
                        let preview = plot_builder(plot).interactive().build(cx);
                        cx.subscribe(&preview, move |app: &mut Self, _, event: &ruviz_gpui::PlotPointerEvent, cx| {
                            if event.kind != ruviz_gpui::PlotPointerEventKind::Click || event.mouse_button != Some(gpui::MouseButton::Left) || app.measurements.preview_generation != request { return; }
                            let Some(position)=event.data_position else {return};
                            let Some(picks)=&mut app.measurements.pick_range else {return};
                            picks.push(position.x-origin);
                            if app.measurements.kind==0 || picks.len()==2 {
                                let lo=picks.iter().copied().fold(f64::INFINITY,f64::min);
                                let hi=picks.iter().copied().fold(f64::NEG_INFINITY,f64::max);
                                if app.measurements.kind!=0 && hi<=lo {app.measurements.message="Choose two distinct positions.".into();app.measurements.pick_range=Some(vec![]);cx.notify();return;}
                                app.measurements.fields[0].update(cx,|f,cx|f.set_value(Some(lo),cx));
                                app.measurements.fields[1].update(cx,|f,cx|f.set_value(Some(hi),cx));
                                app.measurements.pick_range=None;
                                app.measurements.message="Range selected from the plot; review the displayed coordinates.".into();
                                app.preview_measurement(cx);
                            } else { app.measurements.message="Now click the other boundary.".into(); }
                            cx.notify();
                        }).detach();
                        app.measurements.preview = Some(preview);
                        app.measurements.preview_label =
                            format!("{label} · {value:.6} · [{lo:.3}, {hi:.3}]");
                    }
                    Err(e) => {
                        app.measurements.preview_label = e;
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    fn start_measurements(&mut self, resume: bool, cx: &mut Context<Self>) {
        if self.measurements.cancel.is_some() {
            return;
        }
        if self.catalog.scanning {
            self.measurements.message = "Wait for the group catalogue to finish loading.".into();
            cx.notify();
            return;
        }
        let Some(series) = self
            .measurements
            .selected_series
            .and_then(|i| self.measurements.archive.series.get(i))
            .cloned()
        else {
            return;
        };
        let recipe = if resume {
            self.measurements
                .selected_run
                .and_then(|i| self.measurements.archive.runs.get(i))
                .and_then(|r| r.recipe.clone())
        } else {
            self.selected_analysis_recipe()
        };
        let inputs: Vec<_> = series
            .frames
            .iter()
            .map(|f| {
                self.measurement_input(&f.group, f.label.clone())
                    .with_recipe(recipe.clone())
            })
            .collect();
        let run = if resume {
            let Some(saved) = self
                .measurements
                .selected_run
                .and_then(|i| self.measurements.archive.runs.get(i))
                .cloned()
            else {
                return;
            };
            if saved.series_id != series.id || saved.series_revision != series.revision {
                self.measurements.message = "Series membership changed; start a new run.".into();
                return;
            }
            (*saved).clone()
        } else {
            let definition = match self.measurement_definition(cx) {
                Ok(d) => d,
                Err(e) => {
                    self.measurements.message = e;
                    cx.notify();
                    return;
                }
            };
            if recipe.as_ref().is_some_and(|r| {
                r.definition.measurement != definition.measurement
                    || r.definition.edge_energy != definition.edge_energy
            }) {
                self.measurements.message = "Recipe choices are frozen. Save a new recipe to use the edited measurement, or choose Group settings.".into();
                cx.notify();
                return;
            }
            SeriesRun::new(&series, definition, &inputs)
        };
        let index = if resume {
            self.measurements.selected_run.unwrap()
        } else {
            self.measurements.archive.runs.push(Arc::new(run.clone()));
            self.measurements.archive.runs.len() - 1
        };
        self.measurements.selected_run = Some(index);
        self.measurements.plot = None;
        self.measurements.page = 0;
        self.measurements.stale.clear();
        self.measurements.generation += 1;
        let generation = self.measurements.generation;
        let project = self.project_generation;
        let cancel = Arc::new(AtomicBool::new(false));
        self.measurements.cancel = Some(cancel.clone());
        self.measurements.progress = (0, inputs.len());
        self.measurements.message = if resume {
            "Checking unfinished inputs…"
        } else {
            "Snapshotting input revisions…"
        }
        .into();
        let recovery_project = self.project_file();
        let recovery_root = recovery::recovery_root();
        let snapshot_progress = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let snapshot_phase = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let monitor_progress = snapshot_progress.clone();
        let monitor_phase = snapshot_phase.clone();
        let count = inputs.len();
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(200))
                    .await;
                let phase = monitor_phase.load(Ordering::Relaxed);
                if phase == 2 {
                    break;
                }
                let progress = monitor_progress.load(Ordering::Relaxed);
                if this
                    .update(cx, |app, cx| {
                        if app.project_generation != project
                            || app.measurements.generation != generation
                        {
                            return false;
                        }
                        app.measurements.progress = (progress, count);
                        app.measurements.message = if phase == 0 {
                            format!("Checking input {progress} / {count}…")
                        } else {
                            "Saving recovery checkpoint…".into()
                        };
                        cx.notify();
                        true
                    })
                    .ok()
                    != Some(true)
                {
                    break;
                }
            }
        })
        .detach();
        cx.spawn(async move |this, cx| {
            let stop = cancel.clone();
            let sources = Arc::new(inputs);
            let sources_job = sources.clone();
            let worker_phase=snapshot_phase.clone();
            let initialized = cx
                .background_spawn(async move {
                    let mut run = run;
                    run.freeze_with_progress(&sources_job, || stop.load(Ordering::Relaxed), |count|snapshot_progress.store(count,Ordering::Relaxed));
                    worker_phase.store(1,Ordering::Relaxed);
                    let root = recovery_root?;
                    let journal=recovery::RecoveryWriter::begin(&root,recovery_project,&run)?;
                    Ok::<_,String>((run,journal))
                }).await;
            snapshot_phase.store(2,Ordering::Relaxed);
            let (mut run, mut journal) = match initialized {
                Ok(value)=>value,
                Err(error)=>{
                    this.update(cx,|app,cx|{if app.project_generation!=project||app.measurements.generation!=generation{return;}
                        app.measurements.cancel=None; Arc::make_mut(&mut app.measurements.archive.runs[index]).finish(true);
                        app.measurements.message=format!("Could not create recovery checkpoint: {error}. Free disk space and resume.");cx.notify();
                    }).ok();return;
                }
            };
            let frozen = run.clone();
            if this
                .update(cx, |app, cx| {
                    if app.project_generation != project
                        || app.measurements.generation != generation
                    {
                        return false;
                    }
                    app.measurements.archive.runs[index] = Arc::new(frozen);
                    app.measurements.message = "Calculating every frame…".into();
                    app.measurements.progress=(0,sources.len());
                    cx.notify();
                    true
                })
                .ok()
                != Some(true)
            {
                return;
            }
            // One bounded chunk at a time. No full processed-spectrum collection
            // or nested BLAS pool is retained by the coordinator.
            let mut checkpoint_error=None;
            let chunk_size = if sources.len() > 10_000 { 128 } else { 16 };
            for begin in (0..sources.len()).step_by(chunk_size) {
                if cancel.load(Ordering::Relaxed) {
                    break;
                }
                let end = (begin + chunk_size).min(sources.len());
                let rows = run.rows[begin..end].to_vec();
                let definition = run.definition.clone();
                let worker_sources = sources.clone();
                let stop = cancel.clone();
                let (values, returned_journal, error) = cx
                    .background_spawn(async move {
                        let values=rows.into_iter()
                            .enumerate()
                            .map(|(offset, row)| {
                                if stop.load(Ordering::Relaxed)
                                    || matches!(
                                        row.status,
                                        FrameStatus::Succeeded
                                            | FrameStatus::Failed
                                            | FrameStatus::Unavailable
                                    )
                                {
                                    row
                                } else {
                                    calculate_row(
                                        &worker_sources[begin + offset],
                                        &row,
                                        &definition,
                                    )
                                }
                            })
                            .collect::<Vec<_>>();
                        let error=journal.rows(begin,&values).err();
                        (values,journal,error)
                    }).await;
                journal=returned_journal;
                if let Some(error)=error {checkpoint_error=Some(error);cancel.store(true,Ordering::Relaxed);break;}
                run.rows[begin..end].clone_from_slice(&values);
                if this
                    .update(cx, |app, cx| {
                        if app.project_generation != project
                            || app.measurements.generation != generation
                        {
                            return false;
                        }
                        Arc::make_mut(&mut app.measurements.archive.runs[index]).rows[begin..end]
                            .clone_from_slice(&values);
                        app.measurements.progress = (end, sources.len());
                        cx.notify();
                        true
                    })
                    .ok()
                    != Some(true)
                {
                    return;
                }
            }
            run.finish(cancel.load(Ordering::Relaxed));
            let cancelled=run.cancelled;
            // A failed append can leave a torn final line. Do not append a
            // footer after it: recovery deliberately ignores only that tail.
            let final_error=if checkpoint_error.is_none() {
                cx.background_spawn(async move{journal.finish(cancelled).err()}).await
            } else { None };
            checkpoint_error=checkpoint_error.or(final_error);
            this.update(cx, |app, cx| {
                if app.project_generation != project || app.measurements.generation != generation {
                    return;
                }
                let good = run
                    .rows
                    .iter()
                    .filter(|r| r.status == FrameStatus::Succeeded)
                    .count();
                app.measurements.message = format!(
                    "{good} / {} succeeded{}",
                    run.rows.len(),
                    if run.cancelled { " · cancelled" } else { "" }
                );
                if let Some(error)=checkpoint_error {app.measurements.message=format!("Checkpoint failed: {error}. Completed rows are retained; save the project before continuing.");}
                app.measurements.archive.runs[index] = Arc::new(run);
                app.measurements.cancel = None;
                app.rebuild_measurement_plot(cx);
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    fn rebuild_measurement_plot(&mut self, cx: &mut Context<Self>) {
        let Some(run) = self
            .measurements
            .selected_run
            .and_then(|i| self.measurements.archive.runs.get(i))
        else {
            return;
        };
        if !run.rows.iter().any(|row| row.result.is_some()) {
            self.measurements.plot = None;
            return;
        }
        let mut plot = Plot::new()
            .theme(self.theme.plot_theme())
            .xlabel(run.coordinate_label(self.measurements.trend_axis));
        let coordinates = run.plot_coordinates(self.measurements.trend_axis);
        let mut xs = Vec::new();
        let mut ys = Vec::new();
        // Separate segments preserve missing outcomes as visible gaps.
        for row in run
            .rows
            .iter()
            .zip(coordinates)
            .map(Some)
            .chain(std::iter::once(None))
        {
            if let Some((row, Some(coordinate))) = row
                && let Some(value) = &row.result
            {
                xs.push(coordinate);
                ys.push(value.value);
            } else if !xs.is_empty() {
                plot = if xs.len() == 1 {
                    plot.scatter(&xs, &ys)
                        .color(crate::plotting::trace_color(&self.theme, 0))
                        .into()
                } else {
                    plot.line(&xs, &ys)
                        .color(crate::plotting::trace_color(&self.theme, 0))
                        .into()
                };
                xs.clear();
                ys.clear();
            }
        }
        let unit = run
            .rows
            .iter()
            .find_map(|r| r.result.as_ref())
            .map(|r| r.unit.as_str())
            .unwrap_or("unavailable");
        plot = plot
            .ylabel(format!("{} ({unit})", run.definition.name))
            .size_px(720, 300)
            .major_ticks_x(5);
        self.measurements.plot = Some(plot_builder(plot).interactive().build(cx));
    }

    fn check_measurement_inputs(&mut self, cx: &mut Context<Self>) {
        self.check_measurement_inputs_internal(true, cx);
    }

    fn check_measurement_inputs_internal(&mut self, feedback: bool, cx: &mut Context<Self>) {
        if self.measurements.checking {
            return;
        }
        let Some(index) = self.measurements.selected_run else {
            return;
        };
        let Some(run) = self.measurements.archive.runs.get(index) else {
            return;
        };
        let inputs: Vec<_> = run
            .rows
            .iter()
            .map(|r| {
                (
                    self.measurement_input(&r.frame.group, r.frame.label.clone())
                        .with_recipe(run.recipe.clone()),
                    r.input_revision.clone(),
                )
            })
            .collect();
        let generation = self.project_generation;
        self.measurements.checking = true;
        cx.spawn(async move |this, cx| {
            let stale = cx
                .background_spawn(async move {
                    inputs
                        .into_iter()
                        .map(|(input, old)| {
                            input
                                .revision()
                                .map(|(_, now)| Some(now) != old)
                                .unwrap_or(true)
                        })
                        .collect::<Vec<_>>()
                })
                .await;
            this.update(cx, |app, cx| {
                if app.project_generation != generation {
                    return;
                }
                app.measurements.checking = false;
                if app.measurements.selected_run == Some(index) {
                    let count = stale.iter().filter(|s| **s).count();
                    if feedback || (count > 0 && app.measurements.stale != stale) {
                        app.measurements.message = format!(
                            "{count} inputs changed or unavailable; retained values are unchanged."
                        );
                    }
                    app.measurements.stale = stale;
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    fn export_measurements(&mut self, json: bool, cx: &mut Context<Self>) {
        let Some(run) = self
            .measurements
            .selected_run
            .and_then(|i| self.measurements.archive.runs.get(i))
            .cloned()
        else {
            return;
        };
        let folder = self
            .project_path
            .as_ref()
            .and_then(|p| p.parent())
            .map(|p| p.to_path_buf())
            .unwrap_or_else(std::env::temp_dir);
        let picker = cx.prompt_for_new_path(
            &folder,
            Some(if json {
                "series-measurements.json"
            } else {
                "series-measurements.csv"
            }),
        );
        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(path))) = picker.await {
                let outcome = cx
                    .background_spawn(async move {
                        use std::io::Write;
                        let file = tempfile::NamedTempFile::new_in(
                            path.parent().ok_or("Export directory unavailable")?,
                        )
                        .map_err(|e| e.to_string())?;
                        {
                            let mut writer = std::io::BufWriter::new(file.as_file());
                            if json {
                                serde_json::to_writer_pretty(&mut writer, &run)
                                    .map_err(|e| e.to_string())?;
                            } else {
                                run.write_csv(&mut writer).map_err(|e| e.to_string())?;
                            }
                            writer.flush().map_err(|e| e.to_string())?;
                        }
                        file.as_file().sync_all().map_err(|e| e.to_string())?;
                        file.persist(&path).map_err(|e| e.to_string())?;
                        Ok::<_, String>(path)
                    })
                    .await;
                this.update(cx, |app, cx| {
                    app.measurements.message = match outcome {
                        Ok(path) => format!("Exported {}", path.display()),
                        Err(e) => format!("Export failed: {e}"),
                    };
                    cx.notify();
                })
                .ok();
            }
        })
        .detach();
    }
}

fn space_label(space: MeasurementSpace) -> &'static str {
    match space {
        MeasurementSpace::Mu => "μ(E)",
        MeasurementSpace::Norm => "Normalized μ(E)",
        MeasurementSpace::Flat => "Flattened μ(E)",
        MeasurementSpace::Chi { .. } => "Weighted χ(k)",
        MeasurementSpace::Fourier => "Fourier magnitude",
    }
}
fn axis_label(space: MeasurementSpace) -> &'static str {
    match space {
        MeasurementSpace::Chi { .. } => "k (Å⁻¹)",
        MeasurementSpace::Fourier => "R (Å; uncorrected)",
        _ => "Energy (eV)",
    }
}
