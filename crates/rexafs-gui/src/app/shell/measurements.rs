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
    pick_range: Option<Vec<f64>>,
}

impl Default for MeasurementState {
    fn default() -> Self {
        Self::from_archive(SeriesArchive::default())
    }
}
impl MeasurementState {
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
            selected_preset: None,
            pick_range: None,
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
        let start = self
            .measurements
            .fields
            .first()
            .and_then(|f| f.read(cx).value())
            .ok_or("Enter a point or range start")?;
        let end = self
            .measurements
            .fields
            .get(1)
            .and_then(|f| f.read(cx).value())
            .ok_or("Enter a range end")?;
        let kind = self.measurements.kind;
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
        if let Some(old) = self
            .measurements
            .archive
            .presets
            .iter()
            .find(|p| Some(&p.id) == self.measurements.selected_preset.as_ref())
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
        let input = self.measurement_input(&frame.group, frame.label.clone());
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
        input.settings = row.settings.clone();
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
        let Some(series) = self
            .measurements
            .selected_series
            .and_then(|i| self.measurements.archive.series.get(i))
            .cloned()
        else {
            return;
        };
        let inputs: Vec<_> = series
            .frames
            .iter()
            .map(|f| self.measurement_input(&f.group, f.label.clone()))
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
            saved
        } else {
            let definition = match self.measurement_definition(cx) {
                Ok(d) => d,
                Err(e) => {
                    self.measurements.message = e;
                    cx.notify();
                    return;
                }
            };
            SeriesRun::new(&series, definition, &inputs)
        };
        let index = if resume {
            self.measurements.selected_run.unwrap()
        } else {
            self.measurements.archive.runs.push(run.clone());
            self.measurements.archive.runs.len() - 1
        };
        self.measurements.selected_run = Some(index);
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
        cx.spawn(async move |this, cx| {
            let stop = cancel.clone();
            let sources = Arc::new(inputs);
            let sources_job = sources.clone();
            let mut run = cx
                .background_spawn(async move {
                    let mut run = run;
                    run.freeze(&sources_job, || stop.load(Ordering::Relaxed));
                    run
                })
                .await;
            let frozen = run.clone();
            if this
                .update(cx, |app, cx| {
                    if app.project_generation != project
                        || app.measurements.generation != generation
                    {
                        return false;
                    }
                    app.measurements.archive.runs[index] = frozen;
                    app.measurements.message = "Calculating every frame…".into();
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
            for begin in (0..sources.len()).step_by(16) {
                if cancel.load(Ordering::Relaxed) {
                    break;
                }
                let end = (begin + 16).min(sources.len());
                let rows = run.rows[begin..end].to_vec();
                let definition = run.definition.clone();
                let worker_sources = sources.clone();
                let stop = cancel.clone();
                let values = cx
                    .background_spawn(async move {
                        rows.into_iter()
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
                            .collect::<Vec<_>>()
                    })
                    .await;
                run.rows[begin..end].clone_from_slice(&values);
                if this
                    .update(cx, |app, cx| {
                        if app.project_generation != project
                            || app.measurements.generation != generation
                        {
                            return false;
                        }
                        app.measurements.archive.runs[index].rows[begin..end]
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
                app.measurements.archive.runs[index] = run;
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
                    self.measurement_input(&r.frame.group, r.frame.label.clone()),
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
                    app.measurements.message = format!(
                        "{count} inputs changed or unavailable; retained values are unchanged."
                    );
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
                        let text = if json {
                            serde_json::to_string_pretty(&run).map_err(|e| e.to_string())?
                        } else {
                            run.csv()
                        };
                        std::fs::write(&path, text).map_err(|e| e.to_string())?;
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
