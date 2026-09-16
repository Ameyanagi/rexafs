use super::*;
use crate::app::{
    FrameFirst, FrameJumpBack, FrameJumpFwd, FrameLast, FrameNext, FramePrev, shell::button,
};
use gpui::{IntoElement, ParentElement, Styled, div, prelude::*, px};

impl StudioApp {
    pub(crate) fn series_stage_center(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        if self.live.open {
            return self.live_center(cx);
        }
        self.monitor_measurements(cx);
        if !self.measurements.initialized {
            self.measurements.initialized = true;
            if self.overview_source().is_none() {
                if let Some(index) = self.measurements.selected_series {
                    self.choose_overview_series(index, cx);
                }
            }
        }
        if self.measurements.overview {
            let overview = self.series_overview_center(cx).into_any_element();
            if self.measurements.recovery_entries.is_empty() {
                return overview;
            }
            let t = self.theme;
            return div()
                .flex_1()
                .min_h_0()
                .flex()
                .flex_col()
                .child(overview)
                .child(div().px_3().py_1().child(
                    button(&t, "series-recovery", "Recovery…", false).on_click(cx.listener(
                        |app, _, _, cx| {
                            app.series_results(cx);
                            app.measurements.advanced = true;
                        },
                    )),
                ))
                .into_any_element();
        }
        let t = self.theme;
        let title = if self.measurements.results {
            "Results"
        } else {
            "Add trend"
        };
        let source = self
            .measurements
            .selected_series
            .and_then(|i| self.measurements.archive.series.get(i))
            .map(|s| format!("{} · {} frames", s.name, s.frames.len()))
            .unwrap_or_default();
        let bar = div()
            .flex_none()
            .p_2()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_2()
            .border_b_1()
            .border_color(t.border)
            .child(
                button(&t, "trend-back", "← Series", false)
                    .on_click(cx.listener(|app, _, _, cx| app.close_series_trend(cx))),
            )
            .child(div().text_size(px(15.)).child(title))
            .child(div().text_color(t.text_muted).child(source))
            .child(div().flex_1())
            .child(
                button(
                    &t,
                    "trend-advanced",
                    if self.measurements.advanced {
                        "Advanced ▴"
                    } else {
                        "Advanced ▾"
                    },
                    false,
                )
                .on_click(cx.listener(|app, _, _, cx| {
                    app.measurements.advanced = !app.measurements.advanced;
                    cx.notify();
                })),
            );
        let content = self.measurement_workspace(cx);
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
        self.ensure_measurement_fields(cx);
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
        if self.measurements.advanced {
            body = body
                .child(self.measurement_recovery_panel(cx))
                .child(self.measurement_management(cx));
            if !self.measurements.results {
                body = body
                    .child(self.measurement_presets(cx))
                    .child(self.analysis_recipe_controls(cx));
            }
        }
        let Some(series) = self
            .measurements
            .selected_series
            .and_then(|i| self.measurements.archive.series.get(i))
        else {
            return body.child("Select a series first.").into_any_element();
        };
        let count = series.frames.len();
        let series_id = series.id.clone();
        if self.measurements.results {
            if !running && self.measurements.plot.is_none() {
                self.rebuild_measurement_plot(cx);
            }
            if !self.measurements.message.is_empty() {
                body = body.child(self.measurements.message.clone());
            }
            if let Some(plot) = &self.measurements.preview {
                body = body
                    .child(self.measurements.preview_label.clone())
                    .child(div().h(px(300.)).min_h(px(300.)).child(plot.clone()));
            }

            if self.measurements.selected_run.is_none() {
                body = body.child(
                    div()
                        .text_color(t.text_muted)
                        .child("No saved trends. Choose Add trend to start."),
                );
            }
            let mut history = div().flex().flex_wrap().gap_2();
            for (index, run) in self
                .measurements
                .archive
                .runs
                .iter()
                .enumerate()
                .filter(|(_, r)| r.series_id == series_id)
            {
                let label = format!("{} · #{}", run.definition.name, index + 1);
                history = history.child(
                    button(
                        &t,
                        ("measurement-history", index),
                        label,
                        self.measurements.selected_run == Some(index),
                    )
                    .on_click(cx.listener(move |app, _, _, cx| {
                        app.measurements.selected_run = Some(index);
                        app.measurements.preview = None;
                        app.measurements.preview_label.clear();
                        app.measurements.preview_generation += 1;
                        app.measurements.page = 0;
                        app.measurements.stale.clear();
                        app.rebuild_measurement_plot(cx);
                        cx.notify();
                    })),
                );
            }
            body = body.child(history);
            let mut axes = div().flex().flex_wrap().gap_2();
            for (index, (axis, label)) in [
                (TrendAxis::Sequence, "Frame"),
                (TrendAxis::Coordinate, "Coordinate"),
                (TrendAxis::ElapsedAcquisition, "Time"),
            ]
            .into_iter()
            .enumerate()
            {
                axes = axes.child(
                    button(
                        &t,
                        ("measurement-trend-axis", index),
                        label,
                        self.measurements.trend_axis == axis,
                    )
                    .on_click(cx.listener(move |app, _, _, cx| {
                        app.measurements.trend_axis = axis;
                        app.rebuild_measurement_plot(cx);
                        cx.notify();
                    })),
                );
            }
            body = body.child(axes);
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
                let export = div()
                    .flex()
                    .flex_wrap()
                    .gap_2()
                    .child(
                        button(&t, "measurement-csv", "Export CSV…", false).on_click(
                            cx.listener(|app, _, _, cx| app.export_measurements(false, cx)),
                        ),
                    )
                    .child(
                        button(&t, "measurement-json", "Export JSON…", false).on_click(
                            cx.listener(|app, _, _, cx| app.export_measurements(true, cx)),
                        ),
                    )
                    .child(
                        button(
                            &t,
                            "measurement-check",
                            if self.measurements.checking {
                                "Checking…"
                            } else {
                                "Check inputs"
                            },
                            false,
                        )
                        .on_click(cx.listener(|app, _, _, cx| app.check_measurement_inputs(cx))),
                    );
                body = body.child(export).child(
                div()
                    .text_size(px(11.))
                    .text_color(t.text_muted)
                    .child(format!(
                        "Retained run · series revision {} · {:?} · {:?} · no inferred uncertainty",
                        run.series_revision,
                        run.definition.measurement.space,
                        run.definition.measurement.origin
                    )),
            );
                let result_count = run.rows.len();
                let begin =
                    (self.measurements.page * 50).min(result_count.saturating_sub(1) / 50 * 50);
                let end = (begin + 50).min(run.rows.len());
                let mut table = div().flex().flex_col().gap_1();
                for (index, row) in run.rows.iter().enumerate().skip(begin).take(50) {
                    let value = row
                        .result
                        .as_ref()
                        .map(|r| {
                            if r.unit == "1" {
                                format!("{:.6}", r.value)
                            } else {
                                format!("{:.6} {}", r.value, r.unit)
                            }
                        })
                        .unwrap_or_else(|| "—".into());
                    let reason = if self.measurements.stale.get(index) == Some(&true) {
                        " · Inputs changed".into()
                    } else {
                        row.reason
                            .as_ref()
                            .map(|s| format!(" · {s}"))
                            .unwrap_or_default()
                    };
                    let coordinate = match self.measurements.trend_axis {
                        TrendAxis::Sequence => String::new(),
                        TrendAxis::Coordinate => row
                            .frame
                            .coordinate
                            .map(|x| format!(" · {x} {}", run.coordinate.unit))
                            .unwrap_or_else(|| " · coordinate missing".into()),
                        TrendAxis::ElapsedAcquisition => row
                            .frame
                            .acquired_at
                            .as_ref()
                            .map(|t| format!(" · {t}"))
                            .unwrap_or_else(|| " · timestamp missing".into()),
                    };
                    let status = if row.status == FrameStatus::Succeeded {
                        String::new()
                    } else {
                        format!(" · {:?}", row.status)
                    };
                    let label = format!(
                        "{} · {} · {value}{status}{coordinate}{reason}",
                        row.frame.sequence, row.frame.label
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
                                button(&t, "measurement-page-prev", "Previous rows", false)
                                    .on_click(cx.listener(|app, _, _, cx| {
                                        app.measurements.page =
                                            app.measurements.page.saturating_sub(1);
                                        cx.notify();
                                    })),
                            )
                            .child(format!("{}–{end} / {} rows", begin + 1, run.rows.len()))
                            .child(
                                button(&t, "measurement-page-next", "Next rows", false).on_click(
                                    cx.listener(move |app, _, _, cx| {
                                        if end < result_count {
                                            app.measurements.page += 1;
                                        }
                                        cx.notify();
                                    }),
                                ),
                            ),
                    )
                    .child(table);
            }

            let mut actions = div().flex().gap_2().child(
                button(&t, "results-add-trend", "Add trend…", true).on_click(cx.listener(
                    |app, _, window, cx| {
                        app.begin_series_trend(cx);
                        app.operando_focus.focus(window, cx);
                    },
                )),
            );
            if let Some(index) = self.measurements.selected_run {
                if self.measurement_trend_matches(index) {
                    actions = actions.child(
                        button(&t, "results-show-trend", "Show in Series", false).on_click(
                            cx.listener(move |app, _, _, cx| app.show_measurement_trend(index, cx)),
                        ),
                    );
                }
                if !running && self.measurements.archive.runs[index].cancelled {
                    actions =
                        actions.child(button(&t, "results-resume", "Resume", false).on_click(
                            cx.listener(|app, _, _, cx| app.start_measurements(true, cx)),
                        ));
                }
            }
            return div()
                .flex_1()
                .min_h_0()
                .flex()
                .flex_col()
                .child(body)
                .child(
                    div()
                        .p_3()
                        .border_t_1()
                        .border_color(t.border)
                        .child(actions),
                )
                .into_any_element();
        }
        let mut kinds = div().flex().flex_wrap().gap_2();
        for (index, label) in ["Point", "Maximum", "Integral", "Mean", "Edge energy"]
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
                    app.clear_measurement_handles();
                    app.preview_measurement(cx);
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
                        app.clear_measurement_handles();
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
                        app.preview_measurement(cx);
                    })),
                );
            }
            body = body.child(spaces);
            let mut range = div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap_2()
                .child(div().w(px(240.)).child(self.measurements.fields[0].clone()));
            if self.measurements.kind != 0 {
                range = range.child(div().w(px(240.)).child(self.measurements.fields[1].clone()));
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
                            "eV from E₀"
                        } else {
                            "eV absolute"
                        },
                        false,
                    )
                    .on_click(cx.listener(|app, _, _, cx| {
                        app.measurements.relative = !app.measurements.relative;
                        app.preview_measurement(cx);
                    })),
                );
            } else {
                range = range.child(axis_label(self.measurements.space));
            }
            body = body.child(range);
        }
        let mut preview_controls = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_2()
            .child(
                button(&t, "measurement-prev-frame", "←", false)
                    .on_click(cx.listener(|app, _, _, cx| app.step_measurement_frame(-1, cx))),
            )
            .child(format!("{} / {count}", self.measurements.preview_index + 1))
            .child(
                button(&t, "measurement-next-frame", "→", false)
                    .on_click(cx.listener(|app, _, _, cx| app.step_measurement_frame(1, cx))),
            )
            .child(
                button(&t, "measurement-preview", "Refresh", false)
                    .on_click(cx.listener(|app, _, _, cx| app.preview_measurement(cx))),
            );
        preview_controls = preview_controls.child(
            button(
                &t,
                "measurement-full-preview",
                "Full spectrum",
                self.measurements.preview_full,
            )
            .on_click(cx.listener(|app, _, _, cx| {
                app.measurements.preview_full = !app.measurements.preview_full;
                app.preview_measurement(cx);
            })),
        );
        body = body.child(preview_controls);
        let mut controls = div().flex().items_center().gap_2();
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
                .when(self.measurement_definition(cx).is_err(), |d| {
                    d.disabled(true).opacity(0.45)
                })
                .on_click(cx.listener(|app, _, _, cx| app.start_measurements(false, cx))),
            );
        }
        if !running
            && self
                .measurements
                .selected_run
                .and_then(|i| self.measurements.archive.runs.get(i))
                .is_some_and(|run| run.cancelled)
        {
            controls = controls.child(
                button(&t, "measurement-resume", "Resume unfinished", false)
                    .on_click(cx.listener(|app, _, _, cx| app.start_measurements(true, cx))),
            );
        }

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
                    .id("measurement-preview-card")
                    .h(px(340.))
                    .min_h(px(340.))
                    .w_full()
                    .relative()
                    .on_mouse_move(cx.listener(|app, ev: &gpui::MouseMoveEvent, _, cx| {
                        app.plot_pointer_move(super::drag::PLOT_MEASUREMENT, ev.position, cx);
                    }))
                    .capture_any_mouse_down(cx.listener(
                        |app, ev: &gpui::MouseDownEvent, window, cx| {
                            app.operando_focus.focus(window, cx);
                            app.capture_handle_press(super::drag::PLOT_MEASUREMENT, ev, cx);
                        },
                    ))
                    .child(plot.clone())
                    .children(self.handle_layer(super::drag::PLOT_MEASUREMENT, cx))
                    .children(self.handle_overlay(super::drag::PLOT_MEASUREMENT, cx)),
            );
        }

        div()
            .id("series-trend-editor")
            .key_context("Operando")
            .track_focus(&self.operando_focus)
            .on_action(cx.listener(|app, _: &FramePrev, _, cx| app.step_measurement_frame(-1, cx)))
            .on_action(cx.listener(|app, _: &FrameNext, _, cx| app.step_measurement_frame(1, cx)))
            .on_action(cx.listener(move |app, _: &FrameJumpBack, _, cx| {
                app.step_measurement_frame(-(count.div_ceil(100).max(1) as isize), cx)
            }))
            .on_action(cx.listener(move |app, _: &FrameJumpFwd, _, cx| {
                app.step_measurement_frame(count.div_ceil(100).max(1) as isize, cx)
            }))
            .on_action(
                cx.listener(|app, _: &FrameFirst, _, cx| {
                    app.step_measurement_frame(isize::MIN, cx)
                }),
            )
            .on_action(
                cx.listener(|app, _: &FrameLast, _, cx| app.step_measurement_frame(isize::MAX, cx)),
            )
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .child(body)
            .child(
                div()
                    .flex_none()
                    .p_3()
                    .border_t_1()
                    .border_color(t.border)
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .flex_1()
                            .text_color(t.text_muted)
                            .child("One value per frame"),
                    )
                    .child(controls),
            )
            .into_any_element()
    }
}
