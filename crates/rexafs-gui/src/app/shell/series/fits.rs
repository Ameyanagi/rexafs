//! Compact Series fit setup, retained frame curves, and parameter expressions.
use super::*;
use crate::plot_ranges::plot_builder;
use crate::{
    series_fits::{self, FitTrend, SeriesFitRun},
    widgets::text_input::TextInput,
};
use gpui::{AnyElement, Entity};
use ruviz_gpui::RuvizPlot;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

pub(crate) struct SeriesFitState {
    pub view: bool,
    pub edit: bool,
    model: Option<usize>,
    run: Option<usize>,
    trend: usize,
    path: Option<usize>,
    current_processing: bool,
    errors: bool,
    coordinate: bool,
    r_space: bool,
    picker: Option<&'static str>,
    custom: bool,
    fields: Vec<Entity<TextInput>>,
    cancel: Option<Arc<AtomicBool>>,
    message: String,
    plot_key: String,
    curve: Option<Entity<RuvizPlot>>,
    trend_plot: Option<Entity<RuvizPlot>>,
    readout: String,
    estimate: String,
    progress: (usize, usize),
    plot_generation: u64,
}
impl Default for SeriesFitState {
    fn default() -> Self {
        Self {
            view: false,
            edit: false,
            model: None,
            run: None,
            trend: 0,
            path: None,
            current_processing: false,
            errors: true,
            coordinate: false,
            r_space: true,
            picker: None,
            custom: false,
            fields: Vec::new(),
            cancel: None,
            message: String::new(),
            plot_key: String::new(),
            curve: None,
            trend_plot: None,
            readout: String::new(),
            estimate: String::new(),
            progress: (0, 0),
            plot_generation: 0,
        }
    }
}
impl Drop for SeriesFitState {
    fn drop(&mut self) {
        if let Some(stop) = &self.cancel {
            stop.store(true, Ordering::Relaxed);
        }
    }
}

impl StudioApp {
    pub(crate) fn begin_series_fit(&mut self, cx: &mut Context<Self>) {
        if !self.select_trend_series(cx) {
            return;
        }
        self.series_fits.edit = true;
        self.series_fits.picker = None;
        if self.series_fits.model.is_none() {
            self.series_fits.model = self.fit_history.iter().rposition(|h| h.joint.is_none());
        }
        cx.notify();
    }

    fn fit_run_matches(&self, run: &SeriesFitRun) -> bool {
        if let Some(id) = &self.overview_series {
            return self
                .measurements
                .archive
                .series
                .iter()
                .find(|s| &s.id == id)
                .is_some_and(|s| s.id == run.series.id && s.revision == run.series.revision);
        }
        self.overview_source().is_some_and(|s| {
            s.len() == run.rows.len()
                && run.rows.iter().enumerate().all(|(i, r)| {
                    s.entry(i).and_then(|ix| self.group_id(ix)).as_ref() == Some(&r.frame.group)
                })
        })
    }

    fn fit_choice(
        &self,
        id: &'static str,
        label: String,
        options: Vec<(usize, String)>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let t = self.theme;
        let mut body = div().min_w_0().flex().flex_col().gap_1().child(
            button(&t, id, label, false)
                .child(icon(&t, Icon::ChevronDown))
                .on_click(cx.listener(move |app, _, _, cx| {
                    app.series_fits.picker = if app.series_fits.picker == Some(id) {
                        None
                    } else {
                        Some(id)
                    };
                    cx.notify();
                })),
        );
        if self.series_fits.picker == Some(id) {
            let mut list = div()
                .id((id, 1usize))
                .max_h(px(180.))
                .overflow_y_scroll()
                .flex()
                .flex_col()
                .bg(t.surface)
                .border_1()
                .border_color(t.border)
                .p_1();
            for (index, name) in options {
                list = list.child(button(&t, (id, index + 2), name, false).w_full().on_click(
                    cx.listener(move |app, _, _, cx| {
                        match id {
                            "series-fit-model" => app.series_fits.model = Some(index),
                            "series-fit-run" => {
                                app.series_fits.run = Some(index);
                                app.series_fits.trend = 0;
                            }
                            "series-fit-trend" => app.series_fits.trend = index,
                            "series-fit-path" => app.series_fits.path = index.checked_sub(1),
                            _ => {}
                        }
                        app.series_fits.picker = None;
                        app.series_fits.plot_key.clear();
                        cx.notify();
                    }),
                ));
            }
            body = body.child(list);
        }
        body.into_any_element()
    }

    pub(crate) fn series_fit_editor(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let t = self.theme;
        let series = self
            .measurements
            .selected_series
            .and_then(|i| self.measurements.archive.series.get(i));
        let count = series.map_or(0, |s| s.frames.len());
        let title = series.map(|s| s.name.clone()).unwrap_or_default();
        let selected = self.series_fits.model.and_then(|i| self.fit_history.get(i));
        let label = selected
            .map(|h| format!("{} · fit #{}", h.group, h.id))
            .unwrap_or_else(|| "Choose saved fit".into());
        let details = selected
            .map(|h| {
                format!(
                    "{} paths · {} variables · k {:.1}–{:.1} Å⁻¹ · R {:.1}–{:.1} Å",
                    h.paths.iter().filter(|p| p.enabled).count(),
                    h.vars.iter().filter(|v| v.vary && v.expr.is_none()).count(),
                    h.ranges.kmin,
                    h.ranges.kmax,
                    h.ranges.rmin,
                    h.ranges.rmax
                )
            })
            .unwrap_or_default();
        let options = self
            .fit_history
            .iter()
            .enumerate()
            .filter(|(_, h)| h.joint.is_none())
            .map(|(i, h)| (i, format!("{} · fit #{}", h.group, h.id)))
            .collect();
        let picker = self.fit_choice("series-fit-model", label, options, cx);
        let busy = self.series_fits.cancel.is_some();
        div().flex_1().min_h_0().flex().flex_col()
            .child(div().p_2().flex().items_center().gap_3().border_b_1().border_color(t.border)
                .child(button(&t, "fit-back", "← Series", false).on_click(cx.listener(|app,_,_,cx| {app.series_fits.edit=false; app.series_fits.view=true; app.open_series_overview(cx);})))
                .child("Add trend").child(div().flex_1())
                .child(button(&t,"fit-scalar","Spectral metric…",false).on_click(cx.listener(|app,_,_,cx|app.begin_series_trend(cx)))))
            .child(div().p_4().max_w(px(720.)).flex().flex_col().gap_3()
                .child(div().text_size(px(18.)).child("EXAFS fit"))
                .child(format!("{title} · {count} frames"))
                .child(picker).child(div().text_color(t.text_muted).child(details))
                .child(div().flex().items_center().gap_2().child("Processing").child(segmented(&t)
                    .child(segment(&t,"fit-processing-each","Each spectrum",!self.series_fits.current_processing,true).on_click(cx.listener(|app,_,_,cx|{app.series_fits.current_processing=false;cx.notify();})))
                    .child(segment(&t,"fit-processing-current","Copy selected settings",self.series_fits.current_processing,false).on_click(cx.listener(|app,_,_,cx|{app.series_fits.current_processing=true;cx.notify();})))))
                .child(div().text_color(t.text_muted).child("Fits every frame independently using the saved model and starting values. Parameters, path distances, and uncertainties are retained."))
                .child(div().flex().gap_2()
                    .child(button(&t,"series-fit-start", if busy {"Fitting…"}else{"Fit all frames"},true).on_click(cx.listener(|app,_,_,cx|app.start_series_fits(cx))))
                    .child(button(&t,"series-fit-model-edit","Review model…",false).on_click(cx.listener(|app,_,_,cx|{app.series_fits.edit=false;app.stage=crate::app::shell::Stage::Fit;app.workspace=crate::app::Workspace::Fit;cx.notify();}))))
                .child(self.series_fits.message.clone()))
            .into_any_element()
    }

    fn start_series_fits(&mut self, cx: &mut Context<Self>) {
        if self.series_fits.cancel.is_some() {
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
        let model = self
            .series_fits
            .model
            .and_then(|i| self.fit_history.get(i))
            .ok_or_else(|| "First run and review a single-spectrum EXAFS fit in Fit.".to_string())
            .and_then(crate::live::ExafsRecipe::capture);
        let model = match model {
            Ok(model) => model,
            Err(e) => {
                self.series_fits.message = e;
                cx.notify();
                return;
            }
        };
        let current = self
            .effective_params(self.selected.unwrap_or(crate::app::NO_ENTRY))
            .clone();
        let inputs: Vec<_> = series
            .frames
            .iter()
            .map(|f| {
                let mut input = self.measurement_input(&f.group, f.label.clone());
                if self.series_fits.current_processing {
                    let import = input.settings.import.clone();
                    input.settings = current.clone();
                    input.settings.import = import;
                }
                input
            })
            .collect();
        let run = SeriesFitRun::new(series, model, &inputs);
        let index = self.measurements.archive.fit_runs.len();
        self.measurements
            .archive
            .fit_runs
            .push(Arc::new(run.clone()));
        self.series_fits.run = Some(index);
        self.series_fits.trend = 0;
        self.series_fits.view = true;
        self.series_fits.edit = false;
        self.series_fits.plot_key.clear();
        self.open_series_overview(cx);
        let project = self.project_generation;
        let stop = Arc::new(AtomicBool::new(false));
        self.series_fits.cancel = Some(stop.clone());
        self.series_fits.progress = (0, inputs.len());
        self.series_fits.message = "Checking inputs and freezing FEFF paths…".into();
        cx.spawn(async move |this, cx| {
            let worker_stop = stop.clone();
            let initialized = cx
                .background_spawn(async move {
                    let mut run = run;
                    let root = series_fits::root()?;
                    run.freeze(&inputs, || worker_stop.load(Ordering::Relaxed))?;
                    Ok::<_, String>((run, inputs, root))
                })
                .await;
            let (mut run, inputs, root) = match initialized {
                Ok(v) => v,
                Err(e) => {
                    this.update(cx, |app, cx| {
                        if app.project_generation != project {
                            return;
                        }
                        app.series_fits.cancel = None;
                        app.series_fits.message = e;
                        Arc::make_mut(&mut app.measurements.archive.fit_runs[index]).finish(true);
                        cx.notify();
                    })
                    .ok();
                    return;
                }
            };
            let frozen = run.clone();
            if this
                .update(cx, |app, cx| {
                    if app.project_generation != project {
                        return false;
                    }
                    app.measurements.archive.fit_runs[index] = Arc::new(frozen);
                    cx.notify();
                    true
                })
                .ok()
                != Some(true)
            {
                return;
            }
            let shared_model = Arc::new(run.model.clone());
            for (i, input) in inputs.into_iter().enumerate() {
                if stop.load(Ordering::Relaxed) {
                    break;
                }
                let row = run.rows[i].clone();
                let model = shared_model.clone();
                let directory = root.clone();
                let row = cx
                    .background_spawn(async move {
                        series_fits::calculate(&input, &row, &model, &directory)
                    })
                    .await;
                run.rows[i] = row.clone();
                if this
                    .update(cx, |app, cx| {
                        if app.project_generation != project {
                            return false;
                        }
                        Arc::make_mut(&mut app.measurements.archive.fit_runs[index]).rows[i] = row;
                        app.series_fits.progress = (i + 1, run.rows.len());
                        app.series_fits.message = format!("Fitting {} / {}", i + 1, run.rows.len());
                        app.series_fits.plot_key.clear();
                        cx.notify();
                        true
                    })
                    .ok()
                    != Some(true)
                {
                    return;
                }
            }
            run.finish(stop.load(Ordering::Relaxed));
            this.update(cx, |app, cx| {
                if app.project_generation != project {
                    return;
                }
                let converged = run
                    .rows
                    .iter()
                    .filter(|r| r.summary.as_ref().is_some_and(|s| s.converged))
                    .count();
                app.series_fits.message = format!(
                    "{} / {} converged{}",
                    converged,
                    run.rows.len(),
                    if run.cancelled { " · stopped" } else { "" }
                );
                app.measurements.archive.fit_runs[index] = Arc::new(run);
                app.series_fits.cancel = None;
                app.series_fits.plot_key.clear();
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    pub(crate) fn series_fit_center(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let t = self.theme;
        let matching: Vec<_> = self
            .measurements
            .archive
            .fit_runs
            .iter()
            .enumerate()
            .filter(|(_, r)| self.fit_run_matches(r))
            .map(|(i, r)| {
                (
                    i,
                    format!(
                        "{} · {}",
                        r.model.name,
                        r.created.get(..19).unwrap_or(&r.created)
                    ),
                )
            })
            .collect();
        if !self
            .series_fits
            .run
            .is_some_and(|i| matching.iter().any(|(j, _)| *j == i))
        {
            self.series_fits.run = matching.last().map(|(i, _)| *i);
            self.series_fits.trend = 0;
            self.series_fits.plot_key.clear();
        }
        let Some(index) = self.series_fits.run else {
            return div().flex_1().flex().flex_col().items_center().justify_center().gap_3()
                .child(div().text_size(px(18.)).child("Fit this series"))
                .child(div().text_color(t.text_muted).child("Reuse a reviewed EXAFS model. Follow parameters and path distances across every frame."))
                .child(button(&t,"series-fit-configure","Set up fits…",true).on_click(cx.listener(|app,_,_,cx|app.begin_series_fit(cx))))
                .into_any_element();
        };
        self.refresh_series_fit(cx);
        let run = self.measurements.archive.fit_runs[index].clone();
        let selected = run.trends.get(self.series_fits.trend);
        let run_choice = self.fit_choice("series-fit-run", run.model.name.clone(), matching, cx);
        let trend_choice = self.fit_choice(
            "series-fit-trend",
            selected
                .map(FitTrend::label)
                .unwrap_or_else(|| "Parameter".into()),
            run.trends
                .iter()
                .enumerate()
                .map(|(i, t)| (i, t.label()))
                .collect(),
            cx,
        );
        let mut body = div()
            .flex_1()
            .min_h_0()
            .min_w_0()
            .flex()
            .flex_col()
            .child(
                div()
                    .px_3()
                    .py_2()
                    .flex()
                    .items_start()
                    .gap_3()
                    .child(run_choice)
                    .child(div().flex_1())
                    .child(div().text_color(t.text_muted).child(
                        if self.series_fits.cancel.is_some() {
                            self.series_fits.message.clone()
                        } else {
                            format!(
                                "{} / {} converged{}",
                                run.rows
                                    .iter()
                                    .filter(|r| r.summary.as_ref().is_some_and(|s| s.converged))
                                    .count(),
                                run.rows.len(),
                                if run.cancelled { " · stopped" } else { "" }
                            )
                        },
                    ))
                    .when(self.series_fits.cancel.is_some(), |d| {
                        d.child(
                            button(&t, "series-fit-stop", "Stop", false).on_click(cx.listener(
                                |app, _, _, cx| {
                                    if let Some(stop) = &app.series_fits.cancel {
                                        stop.store(true, Ordering::Relaxed);
                                    }
                                    app.series_fits.message =
                                        "Stopping after the current fit…".into();
                                    cx.notify();
                                },
                            )),
                        )
                    }),
            )
            .child(
                div()
                    .flex()
                    .px_3()
                    .items_center()
                    .gap_3()
                    .child(
                        segmented(&t)
                            .child(
                                segment(&t, "series-fit-r", "R", self.series_fits.r_space, true)
                                    .on_click(cx.listener(|app, _, _, cx| {
                                        app.series_fits.r_space = true;
                                        app.series_fits.plot_key.clear();
                                        cx.notify();
                                    })),
                            )
                            .child(
                                segment(&t, "series-fit-k", "k", !self.series_fits.r_space, false)
                                    .on_click(cx.listener(|app, _, _, cx| {
                                        app.series_fits.r_space = false;
                                        app.series_fits.plot_key.clear();
                                        cx.notify();
                                    })),
                            ),
                    )
                    .child(
                        div()
                            .text_color(t.text_muted)
                            .child(self.series_fits.readout.clone()),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .min_h(px(130.))
                    .when_some(self.series_fits.curve.clone(), |d, p| d.child(p)),
            )
            .child(self.series_frame_navigation(cx))
            .child(
                div()
                    .px_3()
                    .py_1()
                    .border_t_1()
                    .border_color(t.border)
                    .flex()
                    .items_start()
                    .gap_3()
                    .child(trend_choice)
                    .child(
                        button(&t, "fit-new-expression", "Expression…", false).on_click(
                            cx.listener(|app, _, _, cx| {
                                app.series_fits.custom = !app.series_fits.custom;
                                app.series_fits.message.clear();
                                cx.notify();
                            }),
                        ),
                    )
                    .child(
                        super::super::live::ui::check(
                            &t,
                            "fit-error-bars",
                            "Error bars",
                            self.series_fits.errors,
                        )
                        .on_click(cx.listener(|app, _, _, cx| {
                            app.series_fits.errors = !app.series_fits.errors;
                            app.series_fits.plot_key.clear();
                            cx.notify();
                        })),
                    )
                    .when(
                        run.series.frames.iter().any(|f| f.coordinate.is_some()),
                        |d| {
                            d.child(
                                super::super::live::ui::check(
                                    &t,
                                    "fit-coordinate",
                                    "Series coordinate",
                                    self.series_fits.coordinate,
                                )
                                .on_click(cx.listener(
                                    |app, _, _, cx| {
                                        app.series_fits.coordinate = !app.series_fits.coordinate;
                                        app.series_fits.plot_key.clear();
                                        cx.notify();
                                    },
                                )),
                            )
                        },
                    )
                    .child(div().flex_1())
                    .child(
                        div()
                            .text_color(t.text_muted)
                            .child(self.series_fits.estimate.clone()),
                    ),
            );
        if self.series_fits.custom {
            body = body.child(self.series_fit_expression(cx));
        }
        body.child(div().flex_1().min_h(px(130.)).when_some(self.series_fits.trend_plot.clone(),|d,p|d.child(p)))
            .child(div().px_3().py_1().text_size(px(11.)).text_color(t.text_muted).child("Bars: ±1 standard error · gaps: unavailable or unconverged fits · connecting lines guide the eye"))
            .into_any_element()
    }

    fn series_fit_expression(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let t = self.theme;
        if self.series_fits.fields.is_empty() {
            let default = self
                .series_fits
                .run
                .and_then(|i| self.measurements.archive.fit_runs.get(i))
                .and_then(|r| r.trends.first())
                .map(|t| t.expression.clone())
                .unwrap_or_default();
            for (name, text) in [
                ("Trend name", "Distance".to_string()),
                ("Expression", default),
                ("Unit", "Å".into()),
            ] {
                self.series_fits.fields.push(cx.new(|cx| {
                    let mut f = TextInput::new(name, text, t, cx);
                    f.set_accessible_name(name);
                    f
                }));
            }
            self.series_fits.path = Some(0);
        }
        let run = self
            .series_fits
            .run
            .and_then(|i| self.measurements.archive.fit_runs.get(i));
        let options = std::iter::once((0, "No path geometry".into()))
            .chain(run.into_iter().flat_map(|r| {
                r.model
                    .paths
                    .iter()
                    .enumerate()
                    .map(|(i, (p, _))| (i + 1, p.label.clone()))
            }))
            .collect();
        let label = self
            .series_fits
            .path
            .and_then(|i| run.and_then(|r| r.model.paths.get(i)))
            .map(|(p, _)| p.label.clone())
            .unwrap_or_else(|| "No path geometry".into());
        let picker = self.fit_choice("series-fit-path", label, options, cx);
        let symbols = run
            .map(|r| {
                r.model
                    .variables
                    .iter()
                    .map(|v| v.name.clone())
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .unwrap_or_default();
        div().px_3().py_2().bg(t.surface).flex().flex_col().gap_2()
            .child(div().flex().gap_2()
                .child(div().w(px(160.)).child(self.series_fits.fields[0].clone()))
                .child(div().flex_1().child(self.series_fits.fields[1].clone()))
                .child(div().w(px(65.)).child(self.series_fits.fields[2].clone()))
                .child(picker)
                .child(button(&t,"fit-save-expression","Add trend",true).on_click(cx.listener(|app,_,_,cx|app.add_fit_expression(cx)))))
            .child(div().text_size(px(11.)).text_color(t.text_muted).child(format!("Parameters: {symbols} · reff / degen use the selected path. Example: reff + dr_1. Units are a label.")))
            .when(!self.series_fits.message.is_empty(),|d|d.child(self.series_fits.message.clone()))
            .into_any_element()
    }

    pub(crate) fn series_fit_inspector(&self, cx: &mut Context<Self>) -> AnyElement {
        let t = self.theme;
        let mut panel = div()
            .p_3()
            .flex()
            .flex_col()
            .gap_3()
            .child(section_label(&t, "Fit parameters"));
        if let Some(run) = self
            .series_fits
            .run
            .and_then(|i| self.measurements.archive.fit_runs.get(i))
            .filter(|r| self.fit_run_matches(r))
        {
            if let Some(row) = run.rows.get(self.time_pos) {
                panel = panel.child(row.frame.label.clone());
                if let Some(summary) = &row.summary {
                    for (i, trend) in run.trends.iter().enumerate() {
                        if let Some(estimate) = summary.estimate(trend) {
                            panel = panel
                                .child(
                                    button(
                                        &t,
                                        ("inspect-fit-trend", i),
                                        trend.label(),
                                        self.series_fits.trend == i,
                                    )
                                    .on_click(cx.listener(
                                        move |app, _, _, cx| {
                                            app.series_fits.trend = i;
                                            app.series_fits.plot_key.clear();
                                            cx.notify();
                                        },
                                    )),
                                )
                                .child(
                                    div()
                                        .font_family(MONO)
                                        .text_size(px(12.))
                                        .text_color(t.text_muted)
                                        .child(estimate.label(5)),
                                );
                        }
                    }
                    for notice in &summary.notices {
                        panel = panel.child(
                            div()
                                .text_size(px(11.))
                                .text_color(t.text_muted)
                                .child(notice.clone()),
                        );
                    }
                }
                if let Some(reason) = &row.reason {
                    panel = panel.child(reason.clone());
                }
            }
        }
        panel.into_any_element()
    }

    fn add_fit_expression(&mut self, cx: &mut Context<Self>) {
        let Some(index) = self.series_fits.run else {
            return;
        };
        if self.series_fits.cancel.is_some() {
            self.series_fits.message =
                "Wait for fitting to finish before adding an expression.".into();
            cx.notify();
            return;
        }
        let trend = FitTrend {
            name: self.series_fits.fields[0].read(cx).text().trim().into(),
            expression: self.series_fits.fields[1].read(cx).text().trim().into(),
            unit: self.series_fits.fields[2].read(cx).text().trim().into(),
            path: self.series_fits.path,
        };
        let run = &mut self.measurements.archive.fit_runs[index];
        if trend.name.is_empty()
            || trend.expression.is_empty()
            || !run
                .rows
                .iter()
                .filter_map(|r| r.summary.as_ref())
                .any(|s| s.estimate(&trend).is_some())
        {
            self.series_fits.message="Enter a name and a valid expression using the listed parameters and selected path.".into();
            cx.notify();
            return;
        }
        self.series_fits.trend = run.trends.len();
        Arc::make_mut(run).trends.push(trend);
        self.series_fits.custom = false;
        self.series_fits.plot_key.clear();
        self.series_fits.message.clear();
        cx.notify();
    }

    fn refresh_series_fit(&mut self, cx: &mut Context<Self>) {
        let Some(run) = self
            .series_fits
            .run
            .and_then(|i| self.measurements.archive.fit_runs.get(i))
            .cloned()
        else {
            return;
        };
        let frame = self.time_pos.min(run.rows.len().saturating_sub(1));
        let key = format!(
            "{:?}/{frame}/{}/{}/{}/{}/{:?}",
            run.id,
            self.series_fits.trend,
            self.series_fits.errors,
            self.series_fits.coordinate,
            self.series_fits.r_space,
            self.theme.mode
        );
        if self.series_fits.plot_key == key {
            return;
        }
        self.series_fits.plot_key = key;
        self.series_fits.plot_generation += 1;
        self.series_fits.curve = None;
        self.series_fits.trend_plot = None;
        self.series_fits.readout = "Loading retained fit…".into();
        self.series_fits.estimate.clear();
        let request = self.series_fits.plot_generation;
        let generation = self.project_generation;
        let trend = run.trends.get(self.series_fits.trend).cloned();
        let errors = self.series_fits.errors;
        let coordinate = self.series_fits.coordinate;
        let r_space = self.series_fits.r_space;
        cx.spawn(async move |this, cx| {
            let (result, values) = cx
                .background_spawn(async move {
                    let result = run
                        .rows
                        .get(frame)
                        .ok_or_else(|| "No frame".to_string())
                        .and_then(|row| {
                            series_fits::read(row).map(|r| r.result).map_err(|e| {
                                row.reason
                                    .clone()
                                    .unwrap_or_else(|| format!("{:?}: {e}", row.status))
                            })
                        });
                    let values: Vec<_> = run
                        .rows
                        .iter()
                        .map(|r| {
                            let x = if coordinate {
                                r.frame.coordinate.filter(|x| x.is_finite())
                            } else {
                                Some(r.frame.sequence as f64)
                            };
                            let estimate = r
                                .summary
                                .as_ref()
                                .filter(|s| s.converged)
                                .and_then(|s| trend.as_ref().and_then(|t| s.estimate(t)));
                            (x, estimate)
                        })
                        .collect();
                    ((run, trend, result), values)
                })
                .await;
            this.update(cx, |app, cx| {
                if app.project_generation != generation
                    || app.series_fits.plot_generation != request
                {
                    return;
                }
                let (run, trend, result) = result;
                let theme = app.theme;
                if let Some(row) = run.rows.get(frame) {
                    app.series_fits.readout = format!(
                        "{} · {}",
                        row.frame.label,
                        match &result {
                            Ok(r) => format!(
                                "{} · R-factor {:.5}{}",
                                if row.summary.as_ref().is_some_and(|s| s.converged) {
                                    "converged"
                                } else {
                                    "unconverged"
                                },
                                r.r_factor,
                                if r.warnings.is_empty() {
                                    String::new()
                                } else {
                                    format!(" · {} notices", r.warnings.len())
                                }
                            ),
                            Err(e) => e.clone(),
                        }
                    );
                }
                if let Ok(r) = result {
                    let (x, data, model, xlabel, ylabel) = if r_space {
                        let n =
                            r.r.iter()
                                .take_while(|x| **x <= r.rmax.unwrap_or(3.) + 1.5)
                                .count();
                        (
                            r.r.iter().take(n).copied().collect::<Vec<_>>(),
                            (0..n)
                                .map(|i| r.data_chir_re[i].hypot(r.data_chir_im[i]))
                                .collect::<Vec<_>>(),
                            r.model_chir_mag.iter().take(n).copied().collect::<Vec<_>>(),
                            "R (Å)",
                            crate::plotting::chir_label(r.kweight),
                        )
                    } else {
                        let indices: Vec<_> =
                            r.k.iter()
                                .enumerate()
                                .filter(|(_, k)| {
                                    **k >= r.kmin.unwrap_or(0.)
                                        && **k <= r.kmax.unwrap_or(f64::INFINITY)
                                })
                                .map(|(i, _)| i)
                                .collect();
                        (
                            indices.iter().map(|&i| r.k[i]).collect(),
                            indices
                                .iter()
                                .map(|&i| r.data_chi[i] * r.k[i].powf(r.kweight))
                                .collect(),
                            indices
                                .iter()
                                .map(|&i| r.model_chi[i] * r.k[i].powf(r.kweight))
                                .collect(),
                            "k (Å⁻¹)",
                            crate::plotting::chik_label(r.kweight),
                        )
                    };
                    let plot = ruviz::prelude::Plot::new()
                        .size(10., 3.)
                        .theme(theme.plot_theme())
                        .line(&x, &data)
                        .color(crate::plotting::trace_color(&theme, 0))
                        .label("Data")
                        .line(&x, &model)
                        .color(crate::plotting::trace_color(&theme, 1))
                        .label("Fit")
                        .xlabel(xlabel)
                        .ylabel(&ylabel)
                        .legend_position(ruviz::prelude::LegendPosition::UpperRight);
                    app.series_fits.curve = Some(
                        plot_builder(plot)
                            .range_key(format!("series-fit:{:?}:curve:{r_space}", run.id))
                            .interactive()
                            .build(cx),
                    );
                }
                if let Some(trend) = trend {
                    let x: Vec<_> = values.iter().map(|(x, _)| x.unwrap_or(f64::NAN)).collect();
                    let y: Vec<_> = values
                        .iter()
                        .map(|(_, e)| e.as_ref().map_or(f64::NAN, |e| e.value))
                        .collect();
                    app.series_fits.estimate = values
                        .get(frame)
                        .and_then(|(_, e)| e.as_ref())
                        .map(|e| e.label(5))
                        .unwrap_or_else(|| "Unavailable".into());
                    if y.iter().any(|y| y.is_finite()) {
                        let xlabel = if coordinate {
                            format!(
                                "{} ({})",
                                run.series.coordinate.label, run.series.coordinate.unit
                            )
                        } else {
                            "Frame".into()
                        };
                        let mut plot: ruviz::prelude::Plot = ruviz::prelude::Plot::new()
                            .size(10., 3.)
                            .theme(theme.plot_theme())
                            .line(&x, &y)
                            .color(crate::plotting::trace_color(&theme, 0))
                            .xlabel(&xlabel)
                            .ylabel(trend.label())
                            .into();
                        let valid: Vec<_> = values
                            .iter()
                            .filter_map(|(x, e)| Some(((*x)?, e.as_ref()?.value)))
                            .collect();
                        let xx: Vec<_> = valid.iter().map(|v| v.0).collect();
                        let yy: Vec<_> = valid.iter().map(|v| v.1).collect();
                        plot = plot
                            .scatter(&xx, &yy)
                            .color(crate::plotting::trace_color(&theme, 0))
                            .into();
                        if errors {
                            let bars: Vec<_> = values
                                .iter()
                                .filter_map(|(x, e)| {
                                    let e = e.as_ref()?;
                                    Some((
                                        (*x)?,
                                        e.value,
                                        e.stderr.filter(|v| v.is_finite() && *v > 0.)?,
                                    ))
                                })
                                .collect();
                            if !bars.is_empty() {
                                let xx: Vec<_> = bars.iter().map(|v| v.0).collect();
                                let yy: Vec<_> = bars.iter().map(|v| v.1).collect();
                                let ee: Vec<_> = bars.iter().map(|v| v.2).collect();
                                plot = plot
                                    .error_bars(&xx, &yy, &ee)
                                    .cap_size(5.)
                                    .color(crate::plotting::trace_color(&theme, 0))
                                    .into();
                            }
                        }
                        app.series_fits.trend_plot = Some(
                            plot_builder(plot)
                                .range_key(format!(
                                    "series-fit:{:?}:trend:{}:{}:{:?}:{coordinate}",
                                    run.id, trend.expression, trend.unit, trend.path
                                ))
                                .interactive()
                                .build(cx),
                        );
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }
}
