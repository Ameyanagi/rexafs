use super::*;
use crate::app::shell::button;
use gpui::{IntoElement, ParentElement, Styled, div, prelude::*, px};

impl StudioApp {
    pub(crate) fn series_stage_center(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let t = self.theme;
        let bar = div()
            .p_2()
            .flex()
            .gap_2()
            .border_b_1()
            .border_color(t.border)
            .child(
                button(
                    &t,
                    "series-measurements",
                    "Measurements",
                    !self.measurements.overview,
                )
                .on_click(cx.listener(|app, _, _, cx| {
                    app.measurements.overview = false;
                    app.rebuild_measurement_plot(cx);
                    cx.notify();
                })),
            )
            .child(
                button(
                    &t,
                    "series-overview",
                    "Scan overview",
                    self.measurements.overview,
                )
                .on_click(cx.listener(|app, _, _, cx| {
                    app.measurements.overview = true;
                    cx.notify();
                })),
            );
        let content = if self.measurements.overview {
            self.series_overview_center(cx).into_any_element()
        } else {
            self.measurement_workspace(cx)
        };
        div()
            .flex_1()
            .min_h_0()
            .min_w_0()
            .flex()
            .flex_col()
            .child(bar)
            .child(content)
            .into_any_element()
    }

    fn measurement_workspace(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let t = self.theme;
        if self.measurements.fields.is_empty() {
            for (label, value) in [("Start / point", -20.), ("End", 50.)] {
                let field = cx.new(|cx| {
                    NumericField::new(label, "required", Some(value), FieldKind::Float, t, cx)
                });
                cx.subscribe(&field, |app, _, _, cx| {
                    app.measurements.preview = None;
                    app.measurements.preview_generation += 1;
                    app.measurements.preview_label = "Definition changed · preview again".into();
                    cx.notify();
                })
                .detach();
                self.measurements.fields.push(field);
            }
        }
        if self.measurements.plot.is_none() && self.measurements.selected_run.is_some() {
            self.rebuild_measurement_plot(cx);
        }
        let running = self.measurements.cancel.is_some();
        let mut body = div()
            .id("full-frame-measurements")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .track_scroll(&self.measurements.scroll)
            .p_3()
            .flex()
            .flex_col()
            .gap_3();
        let mut create = div()
            .flex()
            .flex_wrap()
            .gap_2()
            .items_center()
            .child(div().text_size(px(15.)).child("Create series"));
        for (id, label) in [(0, "Marked groups"), (1, "Current scan"), (2, "All groups")] {
            create = create.child(
                button(&t, ("measurement-create", id), label, false)
                    .when(running, |d| d.disabled(true).opacity(0.45))
                    .on_click(
                        cx.listener(move |app, _, _, cx| app.create_measurement_series(id, cx)),
                    ),
            );
        }
        body = body.child(create);
        if !self.measurements.archive.series.is_empty() {
            let mut choices = div().flex().flex_wrap().gap_2();
            for (index, series) in self.measurements.archive.series.iter().enumerate() {
                let label = format!("{} · {} frames", series.name, series.frames.len());
                choices = choices.child(
                    button(
                        &t,
                        ("measurement-series", index),
                        label,
                        self.measurements.selected_series == Some(index),
                    )
                    .on_click(cx.listener(move |app, _, _, cx| {
                        if app.measurements.cancel.is_some() {
                            return;
                        }
                        app.measurements.selected_series = Some(index);
                        app.measurements.preview_index = 0;
                        app.measurements.page = 0;
                        let id = app.measurements.archive.series[index].id.clone();
                        app.measurements.selected_run = app
                            .measurements
                            .archive
                            .runs
                            .iter()
                            .rposition(|r| r.series_id == id);
                        app.measurements.plot = None;
                        app.measurements.stale.clear();
                        app.preview_measurement(cx);
                        cx.notify();
                    })),
                );
            }
            body = body.child(choices);
        }
        let Some(series) = self
            .measurements
            .selected_series
            .and_then(|i| self.measurements.archive.series.get(i))
        else {
            return body.child(div().text_color(t.text_muted).child("Choose groups or a scan. Every frame receives a result or an explicit failure; plots do not sample the calculation.")).into_any_element();
        };
        let count = series.frames.len();
        body = body.child(
            div()
                .text_size(px(11.))
                .text_color(t.text_muted)
                .child(format!(
                    "{} · membership frozen · coordinates: frame sequence",
                    series.ordering
                )),
        );
        let mut kinds = div().flex().flex_wrap().gap_2();
        for (index, label) in ["Point", "Maximum", "Integral", "Mean", "E₀"]
            .into_iter()
            .enumerate()
        {
            kinds = kinds.child(
                button(
                    &t,
                    ("measurement-kind", index),
                    label,
                    self.measurements.kind == index,
                )
                .on_click(cx.listener(move |app, _, _, cx| {
                    app.measurements.kind = index;
                    app.measurements.preview = None;
                    app.measurements.preview_generation += 1;
                    cx.notify();
                })),
            );
        }
        body = body.child(kinds);
        if self.measurements.kind != 4 {
            let mut spaces = div().flex().flex_wrap().gap_2();
            for (index, (space, label)) in [
                (MeasurementSpace::Mu, "μ(E)"),
                (MeasurementSpace::Norm, "Norm"),
                (MeasurementSpace::Flat, "Flat"),
                (MeasurementSpace::Chi { kweight: 0 }, "χ(k)"),
                (MeasurementSpace::Chi { kweight: 2 }, "k²χ(k)"),
                (MeasurementSpace::Fourier, "|χ(R)|"),
            ]
            .into_iter()
            .enumerate()
            {
                spaces = spaces.child(
                    button(
                        &t,
                        ("measurement-space", index),
                        label,
                        self.measurements.space == space,
                    )
                    .on_click(cx.listener(move |app, _, _, cx| {
                        let was_energy = matches!(
                            app.measurements.space,
                            MeasurementSpace::Mu | MeasurementSpace::Norm | MeasurementSpace::Flat
                        );
                        let energy = matches!(
                            space,
                            MeasurementSpace::Mu | MeasurementSpace::Norm | MeasurementSpace::Flat
                        );
                        app.measurements.space = space;
                        app.measurements.relative = energy;
                        if was_energy != energy || !energy {
                            let (lo, hi) = match space {
                                MeasurementSpace::Chi { .. } => (3., 10.),
                                MeasurementSpace::Fourier => (1., 3.),
                                _ => (-20., 50.),
                            };
                            app.measurements.fields[0]
                                .update(cx, |field, cx| field.set_value(Some(lo), cx));
                            app.measurements.fields[1]
                                .update(cx, |field, cx| field.set_value(Some(hi), cx));
                        }
                        app.measurements.preview = None;
                        app.measurements.preview_generation += 1;
                        cx.notify();
                    })),
                );
            }
            body = body.child(spaces);
            let mut range = div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap_2()
                .child(div().w(px(185.)).child(self.measurements.fields[0].clone()));
            if self.measurements.kind != 0 {
                range = range.child(div().w(px(185.)).child(self.measurements.fields[1].clone()));
            }
            let energy = matches!(
                self.measurements.space,
                MeasurementSpace::Mu | MeasurementSpace::Norm | MeasurementSpace::Flat
            );
            if energy {
                range = range.child(
                    button(
                        &t,
                        "measurement-origin",
                        if self.measurements.relative {
                            "Relative to E₀ (eV)"
                        } else {
                            "Absolute energy (eV)"
                        },
                        false,
                    )
                    .on_click(cx.listener(|app, _, _, cx| {
                        app.measurements.relative = !app.measurements.relative;
                        app.measurements.preview = None;
                        app.measurements.preview_generation += 1;
                        cx.notify();
                    })),
                );
            } else {
                range = range.child(axis_label(self.measurements.space));
            }
            body = body.child(range);
        }
        let mut controls = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_2()
            .child(
                button(&t, "measurement-prev-frame", "Previous frame", false).on_click(
                    cx.listener(|app, _, _, cx| {
                        app.measurements.preview_index =
                            app.measurements.preview_index.saturating_sub(1);
                        app.preview_measurement(cx);
                    }),
                ),
            )
            .child(format!("{} / {count}", self.measurements.preview_index + 1))
            .child(
                button(&t, "measurement-next-frame", "Next frame", false).on_click(cx.listener(
                    move |app, _, _, cx| {
                        app.measurements.preview_index =
                            (app.measurements.preview_index + 1).min(count - 1);
                        app.preview_measurement(cx);
                    },
                )),
            )
            .child(
                button(&t, "measurement-preview", "Preview", false)
                    .on_click(cx.listener(|app, _, _, cx| app.preview_measurement(cx))),
            );
        if running {
            controls = controls.child(
                button(
                    &t,
                    "measurement-cancel",
                    format!(
                        "Cancel · {} / {}",
                        self.measurements.progress.0, self.measurements.progress.1
                    ),
                    false,
                )
                .on_click(cx.listener(|app, _, _, cx| {
                    if let Some(cancel) = &app.measurements.cancel {
                        cancel.store(true, Ordering::Relaxed);
                    }
                    app.measurements.message = "Cancelling after the current frame…".into();
                    cx.notify();
                })),
            );
        } else {
            controls = controls.child(
                button(
                    &t,
                    "measurement-run",
                    format!("Calculate all {count} frames"),
                    true,
                )
                .on_click(cx.listener(|app, _, _, cx| app.start_measurements(false, cx))),
            );
        }
        body = body.child(controls);
        if !self.measurements.message.is_empty() {
            body = body.child(
                div()
                    .text_size(px(12.))
                    .text_color(t.text_muted)
                    .child(self.measurements.message.clone()),
            );
        }
        body = body.child(
            div()
                .text_size(px(12.))
                .child(self.measurements.preview_label.clone()),
        );
        if let Some(plot) = &self.measurements.preview {
            body = body.child(
                div()
                    .h(px(300.))
                    .min_h(px(300.))
                    .w_full()
                    .child(plot.clone()),
            );
        }
        let series_id = series.id.clone();
        let mut history = div().flex().flex_wrap().gap_2();
        for (index, run) in self
            .measurements
            .archive
            .runs
            .iter()
            .enumerate()
            .filter(|(_, r)| r.series_id == series_id)
        {
            let label = format!("Run {} · {}", index + 1, run.definition.name);
            history = history.child(
                button(
                    &t,
                    ("measurement-history", index),
                    label,
                    self.measurements.selected_run == Some(index),
                )
                .on_click(cx.listener(move |app, _, _, cx| {
                    app.measurements.selected_run = Some(index);
                    app.measurements.page = 0;
                    app.measurements.stale.clear();
                    app.rebuild_measurement_plot(cx);
                    cx.notify();
                })),
            );
        }
        body = body.child(history);
        if let Some(plot) = &self.measurements.plot {
            body = body.child(
                div()
                    .h(px(300.))
                    .min_h(px(300.))
                    .w_full()
                    .child(plot.clone()),
            );
        }
        if let Some(run) = self
            .measurements
            .selected_run
            .and_then(|i| self.measurements.archive.runs.get(i))
        {
            let mut export = div()
                .flex()
                .flex_wrap()
                .gap_2()
                .child(
                    button(&t, "measurement-csv", "Export CSV…", false)
                        .on_click(cx.listener(|app, _, _, cx| app.export_measurements(false, cx))),
                )
                .child(
                    button(
                        &t,
                        "measurement-json",
                        "Export definition and results…",
                        false,
                    )
                    .on_click(cx.listener(|app, _, _, cx| app.export_measurements(true, cx))),
                )
                .child(
                    button(
                        &t,
                        "measurement-check",
                        if self.measurements.checking {
                            "Checking…"
                        } else {
                            "Check input revisions"
                        },
                        false,
                    )
                    .on_click(cx.listener(|app, _, _, cx| app.check_measurement_inputs(cx))),
                );
            if run.cancelled && !running {
                export = export.child(
                    button(&t, "measurement-resume", "Resume unfinished", false)
                        .on_click(cx.listener(|app, _, _, cx| app.start_measurements(true, cx))),
                );
            }
            body =
                body.child(export)
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(t.text_muted)
                            .child(format!(
                                "Retained run · {:?} · {:?} · no inferred uncertainty",
                                run.definition.measurement.space, run.definition.measurement.origin
                            )),
                    );
            let begin = self.measurements.page * 50;
            let end = (begin + 50).min(run.rows.len());
            let mut table = div().flex().flex_col().gap_1();
            for (index, row) in run.rows.iter().enumerate().skip(begin).take(50) {
                let value = row
                    .result
                    .as_ref()
                    .map(|r| format!("{:.6} {}", r.value, r.unit))
                    .unwrap_or_else(|| "—".into());
                let reason = if self.measurements.stale.get(index) == Some(&true) {
                    " · Inputs changed".into()
                } else {
                    row.reason
                        .as_ref()
                        .map(|s| format!(" · {s}"))
                        .unwrap_or_default()
                };
                let label = format!(
                    "{} · {} · {value} · {:?}{reason}",
                    row.frame.sequence, row.frame.label, row.status
                );
                let group = row.frame.group.clone();
                table = table.child(
                    button(&t, ("measurement-row", index), label, false)
                        .justify_start()
                        .w_full()
                        .on_click(cx.listener(move |app, _, _, cx| {
                            app.measurements.preview_index = index;
                            app.preview_retained_measurement(index, cx);
                            if let Some(ix) = app.group_registry.index(&group) {
                                app.select_entry(ix, cx);
                            }
                            cx.notify();
                        })),
                );
            }
            body = body
                .child(
                    div()
                        .flex()
                        .gap_2()
                        .items_center()
                        .child(
                            button(&t, "measurement-page-prev", "Previous rows", false).on_click(
                                cx.listener(|app, _, _, cx| {
                                    app.measurements.page = app.measurements.page.saturating_sub(1);
                                    cx.notify();
                                }),
                            ),
                        )
                        .child(format!("{}–{end} / {} rows", begin + 1, run.rows.len()))
                        .child(
                            button(&t, "measurement-page-next", "Next rows", false).on_click(
                                cx.listener(move |app, _, _, cx| {
                                    if end < count {
                                        app.measurements.page += 1;
                                    }
                                    cx.notify();
                                }),
                            ),
                        ),
                )
                .child(table);
        }
        body.into_any_element()
    }
}
