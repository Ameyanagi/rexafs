//! Compact trend setup shares the retained full-frame calculation backend.
use super::*;
use crate::app::{OverviewSource, TrendDomain, TrendSnapshot, TrendSource};

impl StudioApp {
    /// Keyboard and button navigation share the same complete, ordered series.
    pub(super) fn step_measurement_frame(&mut self, delta: isize, cx: &mut Context<Self>) {
        if self.measurements.overview || self.measurements.results {
            return;
        }
        let Some(series) = self
            .measurements
            .selected_series
            .and_then(|i| self.measurements.archive.series.get(i))
        else {
            return;
        };
        if series.frames.is_empty() {
            return;
        }
        let next = self
            .measurements
            .preview_index
            .saturating_add_signed(delta)
            .min(series.frames.len() - 1);
        if next != self.measurements.preview_index {
            self.measurements.preview_index = next;
            self.preview_measurement(cx);
        }
    }

    pub(super) fn ensure_measurement_fields(&mut self, cx: &mut Context<Self>) {
        if !self.measurements.fields.is_empty() {
            return;
        }
        let t = self.theme;
        for (label, value) in [
            ("From", self.measurements.initial_range.0),
            ("To", self.measurements.initial_range.1),
        ] {
            let field = cx.new(|cx| {
                NumericField::new(label, "required", Some(value), FieldKind::Float, t, cx)
            });
            cx.subscribe(
                &field,
                |app, _, _: &crate::widgets::numeric_field::FieldEvent, cx| {
                    app.schedule_measurement_preview(cx)
                },
            )
            .detach();
            self.measurements.fields.push(field);
        }
    }

    pub(super) fn schedule_measurement_preview(&mut self, cx: &mut Context<Self>) {
        self.clear_measurement_handles();
        self.measurements.preview_data = None;
        self.measurements.preview_timer += 1;
        self.measurements.preview_generation += 1;
        self.measurements.preview = None;
        let request = self.measurements.preview_timer;
        let project = self.project_generation;
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(250))
                .await;
            this.update(cx, |app, cx| {
                if app.project_generation == project
                    && app.measurements.preview_timer == request
                    && !app.measurements.overview
                    && !app.measurements.results
                {
                    app.preview_measurement(cx);
                }
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    /// Resolve the visible source without changing the overview's frame order.
    fn select_trend_series(&mut self, cx: &mut Context<Self>) -> bool {
        if let Some(id) = &self.overview_series {
            self.measurements.selected_series = self
                .measurements
                .archive
                .series
                .iter()
                .position(|s| &s.id == id);
        } else if let Some(source) = self.overview_source() {
            let groups: Vec<_> = (0..source.len())
                .filter_map(|i| source.entry(i).and_then(|ix| self.group_id(ix)))
                .collect();
            let found = self.measurements.archive.series.iter().position(|s| {
                s.frames.len() == groups.len()
                    && s.frames.iter().zip(&groups).all(|(f, g)| &f.group == g)
            });
            if let Some(index) = found {
                self.measurements.selected_series = Some(index);
            } else {
                let name = self.overview_label();
                self.create_measurement_series(1, cx);
                if let Some(index) = self.measurements.selected_series {
                    self.measurements.archive.series[index].name = name;
                }
            }
        }
        self.measurements.selected_series.is_some()
    }

    pub(crate) fn close_series_trend(&mut self, cx: &mut Context<Self>) {
        self.clear_measurement_handles();
        self.measurements.preview_data = None;
        self.measurements.preview_timer += 1;
        self.measurements.preview_generation += 1;
        self.open_series_overview(cx);
    }

    pub(crate) fn begin_series_trend(&mut self, cx: &mut Context<Self>) {
        if self.measurements.cancel.is_some() {
            self.measurements.overview = false;
            self.measurements.results = false;
            cx.notify();
            return;
        }
        if !self.select_trend_series(cx) {
            return;
        }
        self.measurements.overview = false;
        self.measurements.results = false;
        self.measurements.advanced = false;
        self.measurements.editor = None;
        self.measurements.selected_run = None;
        self.measurements.selected_recipe = None;
        self.measurements.selected_preset = None;
        self.measurements.preset_name = None;
        self.measurements.kind = 1;
        self.measurements.preview_full = false;
        self.measurements.space = MeasurementSpace::Flat;
        self.measurements.relative = true;
        self.measurements.preview_index = self.time_pos;
        self.clear_measurement_handles();
        self.measurements.message.clear();
        self.measurements
            .scroll
            .set_offset(gpui::point(gpui::px(0.), gpui::px(0.)));
        self.ensure_measurement_fields(cx);
        for (field, value) in self.measurements.fields.iter().zip([0., 30.]) {
            field.update(cx, |f, cx| f.set_value(Some(value), cx));
        }
        self.preview_measurement(cx);
    }

    pub(crate) fn series_results(&mut self, cx: &mut Context<Self>) {
        if self.measurements.cancel.is_none() {
            self.select_trend_series(cx);
        }
        let id = self
            .measurements
            .selected_series
            .and_then(|i| self.measurements.archive.series.get(i))
            .map(|s| &s.id);
        let selected = match self.series_trend {
            TrendSource::Measurement(i) => Some(i),
            _ => None,
        };
        self.measurements.selected_run = selected
            .filter(|i| {
                self.measurements
                    .archive
                    .runs
                    .get(*i)
                    .is_some_and(|r| Some(&r.series_id) == id)
            })
            .or_else(|| {
                self.measurements
                    .archive
                    .runs
                    .iter()
                    .rposition(|r| Some(&r.series_id) == id)
            });
        self.clear_measurement_handles();
        self.measurements.preview_data = None;
        self.measurements.results = true;
        self.measurements.overview = false;
        self.measurements.advanced = false;
        self.measurements.preview = None;
        self.measurements.preview_generation += 1;
        self.measurements.preview_label.clear();
        self.measurements.page = 0;
        self.measurements
            .scroll
            .set_offset(gpui::point(gpui::px(0.), gpui::px(0.)));
        self.rebuild_measurement_plot(cx);
        cx.notify();
    }

    pub(crate) fn show_measurement_trend(&mut self, index: usize, cx: &mut Context<Self>) {
        if self.measurement_trend_matches(index) {
            self.measurements.overview = true;
            self.series_trend = TrendSource::Measurement(index);
            self.rebuild_operando_trend(cx);
        } else {
            // Edited membership must not move saved results onto different frames.
            self.measurements.results = true;
            self.rebuild_measurement_plot(cx);
        }
        cx.notify();
    }

    pub(crate) fn measurement_trend_matches(&self, index: usize) -> bool {
        let Some(run) = self.measurements.archive.runs.get(index) else {
            return false;
        };
        let Some(data) = &self.operando else {
            return false;
        };
        if let OverviewSource::Series { id, revision, .. } = &data.source {
            if self.overview_series.as_ref() != Some(id)
                || !self
                    .measurements
                    .archive
                    .series
                    .iter()
                    .any(|s| &s.id == id && s.revision == *revision)
            {
                return false;
            }
        } else if self.overview_series.is_some() || data.source.scan_index() != self.active_scan {
            return false;
        }
        run_matches_source(run, &data.source, &self.group_registry)
    }

    pub(crate) fn measurement_trend_name(&self, index: usize) -> String {
        self.measurements
            .archive
            .runs
            .get(index)
            .map(|r| {
                if r.definition.edge_energy {
                    return "Edge energy · saved".into();
                }
                let (lo, hi) = r.definition.measurement.metric.bounds();
                let bounds = if lo == hi {
                    format!("{lo}")
                } else {
                    format!("{lo}…{hi}")
                };
                let origin = if r.definition.measurement.origin == AxisOrigin::E0 {
                    " from E₀"
                } else {
                    ""
                };
                format!(
                    "{} · {} · {bounds} {}{origin} · saved",
                    r.definition.name,
                    space_label(r.definition.measurement.space),
                    match r.definition.measurement.space {
                        MeasurementSpace::Chi { .. } => "Å⁻¹",
                        MeasurementSpace::Fourier => "Å",
                        _ => "eV",
                    }
                )
            })
            .unwrap_or_else(|| "Saved trend unavailable".into())
    }

    pub(in crate::app) fn measurement_trend_snapshot(&self, index: usize) -> TrendSnapshot {
        let len = self.operando.as_ref().map(|d| d.scan_len).unwrap_or(0);
        let values = retained_values(
            self.measurements.archive.runs.get(index).map(AsRef::as_ref),
            len,
            self.measurement_trend_matches(index),
        );
        TrendSnapshot {
            frames: (0..len).map(|i| i as f64).collect(),
            values,
            name: self
                .measurements
                .archive
                .runs
                .get(index)
                .map(|r| {
                    let unit = r
                        .rows
                        .iter()
                        .find_map(|r| r.result.as_ref())
                        .map(|r| r.unit.as_str())
                        .unwrap_or("");
                    format!("{} ({unit})", r.definition.name)
                })
                .unwrap_or_else(|| "Unavailable".into()),
            domain: TrendDomain::FullScan,
        }
    }

    pub(crate) fn create_series_from_selection(&mut self, marked: bool, cx: &mut Context<Self>) {
        if self.measurements.cancel.is_some() || (marked && self.selection.is_empty()) {
            return;
        }
        self.create_measurement_series(if marked { 0 } else { 2 }, cx);
        if let Some(index) = self.measurements.selected_series {
            self.choose_overview_series(index, cx);
        }
    }
}

fn run_matches_source(
    run: &SeriesRun,
    source: &OverviewSource,
    registry: &crate::group_identity::GroupRegistry,
) -> bool {
    if run.rows.len() != source.len() {
        return false;
    }
    match source {
        OverviewSource::Series { id, revision, .. } => {
            &run.series_id == id && run.series_revision == *revision
        }
        OverviewSource::Scan { .. } => run
            .rows
            .iter()
            .enumerate()
            .all(|(i, row)| registry.index(&row.frame.group) == source.entry(i)),
    }
}

/// Preserve every frame position, including failures and unsampled peaks.
fn retained_values(run: Option<&SeriesRun>, len: usize, matches: bool) -> Vec<f64> {
    let mut values = vec![f64::NAN; len];
    if matches && let Some(run) = run {
        for (slot, row) in values.iter_mut().zip(&run.rows) {
            if let Some(result) = &row.result {
                *slot = result.value;
            }
        }
    }
    values
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::group_identity::{GroupId, GroupRegistry};

    fn fixture(count: usize) -> (SeriesDefinition, SeriesRun, GroupRegistry) {
        let registry = GroupRegistry::default();
        let inputs: Vec<_> = (0..count)
            .map(|i| {
                let path = PathBuf::from(format!("/synthetic/frame-{i}.dat"));
                let group = registry.register_source(
                    Some(i),
                    path.clone(),
                    Default::default(),
                    &Default::default(),
                );
                FrameInput {
                    group,
                    path,
                    label: format!("Frame {i}"),
                    derived: None,
                    settings: Default::default(),
                    recipe: None,
                }
            })
            .collect();
        let series = SeriesDefinition {
            id: GroupId::new_result(),
            revision: 2,
            name: "Synthetic".into(),
            ordering: "Explicit".into(),
            coordinate: Default::default(),
            frames: inputs
                .iter()
                .enumerate()
                .map(|(i, input)| SeriesFrame {
                    id: GroupId::new_result(),
                    group: input.group.clone(),
                    label: input.label.clone(),
                    sequence: i + 1,
                    coordinate: None,
                    acquired_at: None,
                })
                .collect(),
        };
        let definition = MetricDefinition {
            id: GroupId::new_result(),
            revision: 1,
            name: "Mean".into(),
            measurement: Measurement::mean(0.0..=1.0).raw_mu().absolute(),
            edge_energy: false,
        };
        let run = SeriesRun::new(&series, definition, &inputs);
        (series, run, registry)
    }

    #[test]
    fn restored_workspace_opens_the_latest_analyzed_series() {
        let (series, run, _) = fixture(3);
        let (unrelated, _, _) = fixture(2);
        let state = MeasurementState::from_archive(SeriesArchive {
            series: vec![series, unrelated],
            runs: vec![Arc::new(run)],
            ..Default::default()
        });
        assert_eq!(state.selected_series, Some(0));
        assert_eq!(state.selected_run, Some(0));
        assert!(state.overview);
    }

    #[test]
    fn saved_trends_never_attach_to_another_order_or_revision() {
        let (mut series, mut run, registry) = fixture(3);
        assert!(run_matches_source(
            &run,
            &OverviewSource::from_series(&series, &registry),
            &registry
        ));
        series.revision += 1;
        series.frames.reverse();
        assert!(!run_matches_source(
            &run,
            &OverviewSource::from_series(&series, &registry),
            &registry
        ));
        series.revision = run.series_revision;
        series.id = GroupId::new_result();
        assert!(!run_matches_source(
            &run,
            &OverviewSource::from_series(&series, &registry),
            &registry
        ));
        let scan = OverviewSource::Scan {
            index: 0,
            start: 0,
            len: 3,
        };
        assert!(run_matches_source(&run, &scan, &registry));
        run.rows.swap(0, 1);
        assert!(!run_matches_source(&run, &scan, &registry));
    }

    #[test]
    fn saved_trend_keeps_unsampled_peak_and_failure_positions() {
        let (_, mut run, _) = fixture(513);
        for (i, row) in run.rows.iter_mut().enumerate() {
            let value = if i == 257 { 19. } else { 1. };
            row.result = Some(rexafs::prelude::MeasurementResult {
                measurement: run.definition.measurement.clone(),
                value,
                position: None,
                range: [0., 1.],
                standard_error: None,
                unit: "μ".into(),
                e0_ev: None,
            });
        }
        run.rows[256].result = None;
        let values = retained_values(Some(&run), 513, true);
        assert_eq!(values.len(), 513);
        assert!(values[256].is_nan());
        assert_eq!(values[257], 19.);
        assert_eq!(values[258], 1.);
        assert!(
            retained_values(Some(&run), 513, false)
                .iter()
                .all(|v| v.is_nan())
        );
    }
}
