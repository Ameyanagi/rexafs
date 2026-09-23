//! Live owns its display cursor; incoming scans never select the analyst's group.
use super::super::controls::{Tooltip, icon_button};
use super::ui::{channel_label, check};
use super::*;
use crate::icons::Icon;
use crate::{group_identity::GroupId, params::DerivedSpectrum};
use gpui::{AnyElement, Render, WeakEntity, Window};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum LiveDisplay {
    Latest,
    Recent,
    Average,
    PeakFit,
    ExafsK,
    ExafsR,
}

struct LiveMonitorWindow {
    studio: WeakEntity<StudioApp>,
    session: GroupId,
}
impl Render for LiveMonitorWindow {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let session = self.session.clone();
        let content = self
            .studio
            .update(cx, |app, cx| {
                if app
                    .live
                    .session
                    .and_then(|i| app.measurements.archive.live_sessions.get(i))
                    .is_some_and(|s| s.config.id == session)
                {
                    div()
                        .relative()
                        .size_full()
                        .child(app.live_monitor_content(true, cx))
                        .children(app.live_menu_overlay(true, cx))
                        .into_any_element()
                } else {
                    div()
                        .p_3()
                        .child("This Live session is closed.")
                        .into_any_element()
                }
            })
            .unwrap_or_else(|_| {
                div()
                    .p_3()
                    .child("Analysis window closed.")
                    .into_any_element()
            });
        crate::accessibility::root(content, "rexafs Live monitor")
    }
}

/// Choose within Live's channel, independently of the main analysis selection.
fn cursor_index(ids: &[GroupId], follow: bool, held: Option<&GroupId>) -> Option<usize> {
    if ids.is_empty() {
        return None;
    }
    if !follow && let Some(index) = held.and_then(|id| ids.iter().position(|g| g == id)) {
        return Some(index);
    }
    Some(ids.len() - 1)
}

impl StudioApp {
    fn live_axis_key(&self, quantity: &str) -> String {
        let session = self
            .live
            .session
            .and_then(|i| self.measurements.archive.live_sessions.get(i))
            .map(|s| &s.config.id);
        format!(
            "live:{session:?}:{:?}:{:?}:{quantity}",
            self.live.display, self.live.channel
        )
    }

    pub(crate) fn restyle_live(&mut self, cx: &mut Context<Self>) {
        self.live.plot_key.clear();
        self.refresh_live_spectrum(cx);
    }
    fn live_sources(&self, averages: bool) -> Vec<DerivedSpectrum> {
        let Some(session) = self
            .live
            .session
            .and_then(|i| self.measurements.archive.live_sessions.get(i))
        else {
            return Vec::new();
        };
        if averages {
            let mut groups: Vec<_> = self
                .derived
                .iter()
                .filter(|g| {
                    g.operation.as_ref().is_some_and(|o| {
                        o.tool == "Live average"
                            && o.parameters["session_id"] == serde_json::json!(session.config.id)
                    })
                })
                .cloned()
                .collect();
            groups.sort_by_key(|g| {
                g.operation
                    .as_ref()
                    .and_then(|o| o.parameters["batch"].as_u64())
                    .unwrap_or(0)
            });
            groups
        } else {
            self.measurements
                .archive
                .series
                .iter()
                .find(|s| s.id == session.series)
                .map(|s| {
                    s.frames
                        .iter()
                        .filter_map(|f| self.group_registry.index(&f.group))
                        .filter_map(|i| {
                            i.checked_sub(crate::app::DERIVED_BASE)
                                .and_then(|i| self.derived.get(i))
                        })
                        .cloned()
                        .collect()
                })
                .unwrap_or_default()
        }
    }
    fn live_channel(group: &DerivedSpectrum) -> &str {
        group
            .operation
            .as_ref()
            .and_then(|o| o.parameters["channel"].as_str())
            .filter(|s| !s.is_empty())
            .unwrap_or("Signal")
    }
    fn live_uses_averages(&self) -> bool {
        self.live.display == LiveDisplay::Average
            || matches!(self.live.display, LiveDisplay::ExafsK | LiveDisplay::ExafsR)
                && self.live.session.is_some_and(|i| {
                    self.measurements.archive.live_sessions[i].config.merge != LiveMerge::Individual
                })
    }
    fn live_cursor_groups(&self) -> Vec<DerivedSpectrum> {
        self.live_sources(self.live_uses_averages())
            .into_iter()
            .map(|group| {
                if self.live_uses_averages()
                    && !self.live.follow
                    && let Some(held) = &self.live.held_average
                    && held.group_id == group.group_id
                {
                    return held.clone();
                }
                group
            })
            .filter(|g| {
                self.live
                    .channel
                    .as_deref()
                    .is_none_or(|c| Self::live_channel(g) == c)
            })
            .collect()
    }
    pub(super) fn hold_live_latest(&mut self) {
        let latest = self.live_cursor_groups().last().cloned();
        self.live.held_frame = latest.as_ref().and_then(|g| g.group_id.clone());
        self.live.held_average = (self.live_uses_averages()).then_some(latest).flatten();
    }
    pub(super) fn toggle_live_follow(&mut self, cx: &mut Context<Self>) {
        if self.live.follow {
            self.hold_live_latest();
        }
        self.live.follow = !self.live.follow;
        self.refresh_live_spectrum(cx);
    }
    pub(super) fn live_step(&mut self, step: isize, cx: &mut Context<Self>) {
        let ids: Vec<_> = self
            .live_cursor_groups()
            .iter()
            .filter_map(|g| g.group_id.clone())
            .collect();
        if let Some(index) = cursor_index(&ids, self.live.follow, self.live.held_frame.as_ref()) {
            let next = (index as isize + step).clamp(0, ids.len() as isize - 1) as usize;
            self.live.held_average = (self.live_uses_averages())
                .then(|| self.live_cursor_groups().get(next).cloned())
                .flatten();
            self.live.held_frame = Some(ids[next].clone());
            self.live.follow = false;
            self.refresh_live_spectrum(cx);
        }
        cx.notify();
    }
    pub(super) fn refresh_live_spectrum(&mut self, cx: &mut Context<Self>) {
        if !(self.live.open && self.stage == Stage::Series
            || self.live.monitor_docked
            || self.live.monitor_window.is_some())
        {
            return;
        }
        let Some(session) = self
            .live
            .session
            .and_then(|i| self.measurements.archive.live_sessions.get(i))
            .cloned()
        else {
            return;
        };
        if self.live.channel.is_none() {
            self.live.channel = session
                .config
                .layouts
                .first()
                .map(|l| l.channel_name().to_owned());
        }
        let groups = self.live_cursor_groups();
        let ids: Vec<_> = groups.iter().filter_map(|g| g.group_id.clone()).collect();
        let Some(index) = cursor_index(&ids, self.live.follow, self.live.held_frame.as_ref())
        else {
            return;
        };
        let start = if self.live.display == LiveDisplay::Recent {
            index.saturating_sub(4)
        } else {
            index
        };
        let groups = groups[start..=index].to_vec();
        let key = format!(
            "{:?}|{:?}|{:?}|{:?}|{}",
            session.config.id,
            self.live.display,
            self.live.channel,
            groups
                .iter()
                .map(|g| (&g.group_id, &g.source))
                .collect::<Vec<_>>(),
            self.live_sources(false).len()
        );
        if key == self.live.plot_key {
            return;
        }
        self.live.plot_key = key;
        self.live.plot_generation += 1;
        let request = self.live.plot_generation;
        let generation = self.project_generation;
        if matches!(self.live.display, LiveDisplay::ExafsK | LiveDisplay::ExafsR) {
            let row = session
                .exafs
                .iter()
                .rev()
                .find(|r| {
                    groups.last().is_some_and(|g| {
                        Some(&r.group) == g.group_id.as_ref()
                            && Some(&r.source) == g.source.as_ref()
                    })
                })
                .cloned();
            self.refresh_live_exafs(row, request, generation, cx);
            return;
        }
        if self.live.display == LiveDisplay::PeakFit {
            let row = self
                .peaks
                .archive
                .runs
                .iter()
                .find(|r| r.id == session.peak_run_id())
                .and_then(|r| {
                    r.rows
                        .iter()
                        .find(|r| Some(&r.group) == groups.last().and_then(|g| g.group_id.as_ref()))
                })
                .cloned();
            self.refresh_live_fit(row, request, generation, cx);
            return;
        }
        self.live.plotted_fit_status = None;
        self.live.fit_parameters.clear();
        let settings = session.config.settings.clone();
        let definition = session.config.definition.clone();
        let channel = self.live.channel.clone();
        let trend_values: Vec<_> = self
            .measurements
            .archive
            .runs
            .iter()
            .find(|r| r.id == session.run)
            .map(|r| {
                r.rows
                    .iter()
                    .filter(|r| {
                        self.group_registry
                            .index(&r.frame.group)
                            .and_then(|i| i.checked_sub(crate::app::DERIVED_BASE))
                            .and_then(|i| self.derived.get(i))
                            .is_some_and(|g| {
                                channel
                                    .as_deref()
                                    .is_none_or(|c| Self::live_channel(g) == c)
                            })
                    })
                    .enumerate()
                    .map(|(i, r)| {
                        (
                            (i + 1) as f64,
                            r.result.as_ref().map_or(f64::NAN, |v| v.value),
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();
        self.live.plot_error = None;
        self.live.plot_updating = true;
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move {
                    let mut curves = Vec::new();
                    for group in groups {
                        let mut params = settings.clone();
                        if let Some(cached) = &group.params {
                            params.import = cached.import.clone();
                        }
                        let spectrum = group.prepare(
                            &params,
                            crate::series_measurements::required_stage(&definition),
                        )?;
                        let arrays = definition
                            .measurement
                            .arrays(&spectrum)
                            .map_err(|e| e.to_string())?;
                        let value = if definition.edge_energy {
                            spectrum.e0()
                        } else {
                            spectrum
                                .measure(&definition.measurement)
                                .ok()
                                .map(|m| m.value)
                        };
                        curves.push((arrays.axis, arrays.signal, group.label, value));
                    }
                    Ok::<_, String>((curves, definition, trend_values))
                })
                .await;
            this.update(cx, |app, cx| {
                if app.project_generation != generation || app.live.plot_generation != request {
                    return;
                }
                app.live.plot_updating = false;
                match result {
                    Ok((curves, definition, trend_values)) => {
                        if let Some((_, _, label, value)) = curves.last() {
                            app.live.plotted_label = label.clone();
                            app.live.plot_value = *value;
                        }
                        let (axis, signal) = live_axes(&definition.measurement);
                        let build = |width, height| {
                            let mut plot = ruviz::prelude::Plot::new()
                                .size(width, height)
                                .theme(app.theme.plot_theme());
                            for (i, (x, y, _, _)) in curves.iter().enumerate() {
                                plot = plot
                                    .line(x, y)
                                    .color(crate::plotting::trace_color(&app.theme, i))
                                    .line_width(if i + 1 == curves.len() { 2.0 } else { 0.8 })
                                    .into();
                            }
                            plot.xlabel(axis).ylabel(&signal)
                        };
                        app.live.plot = Some(
                            plot_builder(build(11., 3.3))
                                .range_key(app.live_axis_key("curve"))
                                .interactive()
                                .build(cx),
                        );
                        app.live.monitor_plot = Some(
                            plot_builder(build(5., 3.4))
                                .range_key(app.live_axis_key("curve"))
                                .interactive()
                                .build(cx),
                        );
                        if !trend_values.is_empty() {
                            let x: Vec<_> = trend_values.iter().map(|v| v.0).collect();
                            let y: Vec<_> = trend_values.iter().map(|v| v.1).collect();
                            let plot: ruviz::prelude::Plot = ruviz::prelude::Plot::new()
                                .size(11., 2.5)
                                .theme(app.theme.plot_theme())
                                .line(&x, &y)
                                .xlabel("Accepted channel frame")
                                .ylabel(definition.name)
                                .into();
                            app.live.trend = Some(
                                plot_builder(plot)
                                    .range_key(app.live_axis_key("measurement"))
                                    .interactive()
                                    .build(cx),
                            );
                        }
                    }
                    Err(error) => app.live.plot_error = Some(error),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }
    fn refresh_live_exafs(
        &mut self,
        row: Option<ExafsRow>,
        request: u64,
        generation: u64,
        cx: &mut Context<Self>,
    ) {
        self.live.plot_updating = true;
        let space = self.live.display;
        let session = self
            .live
            .session
            .and_then(|i| self.measurements.archive.live_sessions.get(i))
            .cloned();
        let trend = self
            .live_fit_trend_choices()
            .get(self.live.fit_trend)
            .cloned();
        let errors = self.live.fit_trend_errors;
        let mut cache = self.live.fit_trend_cache.clone();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move {
                    let row = row.ok_or("Waiting for an EXAFS fit…")?;
                    let record = read_exafs(&row)?;
                    let result = record.result?;
                    let values = if let (Some(session), Some(trend)) = (&session, &trend) {
                        session
                            .config
                            .exafs
                            .as_ref()
                            .map(|model| {
                                super::fit_trends::estimates(
                                    &session.exafs,
                                    &row,
                                    model,
                                    &session.directory,
                                    trend,
                                    &mut cache,
                                )
                            })
                            .unwrap_or_default()
                    } else {
                        Vec::new()
                    };
                    Ok::<_, String>((row, result, values, cache, trend))
                })
                .await;
            this.update(cx, |app, cx| {
                if app.project_generation != generation || app.live.plot_generation != request {
                    return;
                }
                app.live.plot_updating = false;
                app.live.plot_value = None;
                app.live.trend = None;
                app.live.monitor_trend = None;
                match result {
                    Ok((row, r, values, cache, trend)) => {
                        app.live.fit_trend_cache = cache;
                        if let Some(trend) = trend {
                            let label = trend.label();
                            app.live.trend = crate::plotting::parameter_trend(
                                &values,
                                &label,
                                "Fit update",
                                errors,
                                &app.theme,
                                (11., 2.5),
                            )
                            .map(|p| {
                                plot_builder(p)
                                    .range_key(app.live_axis_key(&format!(
                                        "trend:{}:{}:{:?}",
                                        trend.expression, trend.unit, trend.path
                                    )))
                                    .interactive()
                                    .build(cx)
                            });
                            app.live.monitor_trend = crate::plotting::parameter_trend(
                                &values,
                                &label,
                                "Fit update",
                                errors,
                                &app.theme,
                                (5., 2.5),
                            )
                            .map(|p| {
                                plot_builder(p)
                                    .range_key(app.live_axis_key(&format!(
                                        "trend:{}:{}:{:?}",
                                        trend.expression, trend.unit, trend.path
                                    )))
                                    .interactive()
                                    .build(cx)
                            });
                        }
                        app.live.fit_parameters = r
                            .varying_names
                            .iter()
                            .filter_map(|name| {
                                r.variables
                                    .get(name)
                                    .map(|v| (name.clone(), v.value, v.stderr))
                            })
                            .collect();
                        let (x, data, model, axis, signal) = if space == LiveDisplay::ExafsK {
                            let selected: Vec<_> =
                                r.k.iter()
                                    .enumerate()
                                    .filter(|(_, k)| {
                                        **k >= r.kmin.unwrap_or(0.)
                                            && **k <= r.kmax.unwrap_or(f64::INFINITY)
                                    })
                                    .map(|(i, _)| i)
                                    .collect();
                            (
                                selected.iter().map(|&i| r.k[i]).collect::<Vec<_>>(),
                                selected
                                    .iter()
                                    .map(|&i| r.data_chi[i] * r.k[i].powf(r.kweight))
                                    .collect::<Vec<_>>(),
                                selected
                                    .iter()
                                    .map(|&i| r.model_chi[i] * r.k[i].powf(r.kweight))
                                    .collect::<Vec<_>>(),
                                "k (Å⁻¹)",
                                crate::plotting::chik_label(r.kweight),
                            )
                        } else {
                            let n =
                                r.r.iter()
                                    .take_while(|v| **v <= r.rmax.unwrap_or(3.) + 1.5)
                                    .count();
                            (
                                r.r.iter().take(n).copied().collect(),
                                (0..n)
                                    .map(|i| r.data_chir_re[i].hypot(r.data_chir_im[i]))
                                    .collect(),
                                r.model_chir_mag.iter().take(n).copied().collect(),
                                "R (Å)",
                                crate::plotting::chir_label(r.kweight),
                            )
                        };
                        let build = |w, h| -> ruviz::prelude::Plot {
                            ruviz::prelude::Plot::new()
                                .size(w, h)
                                .theme(app.theme.plot_theme())
                                .line(&x, &data)
                                .color(crate::plotting::trace_color(&app.theme, 0))
                                .label("Data")
                                .line(&x, &model)
                                .color(crate::plotting::trace_color(&app.theme, 1))
                                .label("Fit")
                                .xlabel(axis)
                                .ylabel(&signal)
                                .legend_position(ruviz::prelude::LegendPosition::UpperRight)
                                .into()
                        };
                        app.live.plot = Some(
                            plot_builder(build(11., 3.3))
                                .range_key(app.live_axis_key("curve"))
                                .interactive()
                                .build(cx),
                        );
                        app.live.monitor_plot = Some(
                            plot_builder(build(5., 3.4))
                                .range_key(app.live_axis_key("curve"))
                                .interactive()
                                .build(cx),
                        );
                        app.live.plotted_label = row.label;
                        app.live.plotted_fit_status = Some(format!(
                            "EXAFS {} · R-factor {:.5}{}",
                            if row.converged {
                                "converged"
                            } else {
                                "stopped"
                            },
                            r.r_factor,
                            if row.notices.is_empty() {
                                String::new()
                            } else {
                                format!(" · {} notices", row.notices.len())
                            }
                        ));
                        app.live.plot_error = None;
                    }
                    Err(error) => {
                        app.live.plot = None;
                        app.live.monitor_plot = None;
                        app.live.plotted_label.clear();
                        app.live.plotted_fit_status = None;
                        app.live.fit_parameters.clear();
                        app.live.plot_error = Some(error);
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }
    fn refresh_live_fit(
        &mut self,
        row: Option<crate::peak_fits::PeakRow>,
        request: u64,
        generation: u64,
        cx: &mut Context<Self>,
    ) {
        self.live.plot_updating = true;
        self.live.fit_parameters.clear();
        let label = row.as_ref().map(|r| r.label.clone()).unwrap_or_default();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move {
                    let row = row.ok_or("No peak fit for this scan")?;
                    crate::peak_fits::read(&row).map_err(|e| row.reason.unwrap_or(e))
                })
                .await;
            this.update(cx, |app, cx| {
                if app.project_generation != generation || app.live.plot_generation != request {
                    return;
                }
                app.live.plot_updating = false;
                app.live.plotted_label = label;
                app.live.plot_value = None;
                app.live.trend = None;
                match result {
                    Ok(record) => {
                        let r = &record.result;
                        let status = match r.termination {
                            rexafs::prelude::PeakTermination::Converged => "Fit converged",
                            rexafs::prelude::PeakTermination::FixedModel => "Fixed model",
                            rexafs::prelude::PeakTermination::NotConverged => {
                                "Fit did not converge"
                            }
                            rexafs::prelude::PeakTermination::Cancelled => "Fit cancelled",
                        };
                        app.live.plotted_fit_status = Some(if r.warnings.is_empty() {
                            status.to_owned()
                        } else {
                            format!("{status} · {} warnings", r.warnings.len())
                        });
                        app.live.plot_error = None;
                        let build = |width, height| {
                            let base = ruviz::prelude::Plot::new()
                                .size(width, height)
                                .theme(app.theme.plot_theme());
                            let data = super::super::peaks::peak_curve(
                                base,
                                &r.energy,
                                &r.data,
                                &r.source_indices,
                                "Data",
                                crate::plotting::trace_color(&app.theme, 0),
                            );
                            super::super::peaks::peak_curve(
                                data,
                                &r.energy,
                                &r.model,
                                &r.source_indices,
                                "Fit",
                                crate::plotting::trace_color(&app.theme, 1),
                            )
                            .xlabel("Energy (eV)")
                            .ylabel(match r.definition.space {
                                rexafs::prelude::MeasurementSpace::Norm => "Normalized μ(E)",
                                rexafs::prelude::MeasurementSpace::Flat => "Flattened μ(E)",
                                _ => "μ(E)",
                            })
                            .legend_position(ruviz::prelude::LegendPosition::UpperRight)
                        };
                        app.live.plot = Some(
                            plot_builder(build(11., 3.3))
                                .range_key(app.live_axis_key("curve"))
                                .interactive()
                                .build(cx),
                        );
                        app.live.monitor_plot = Some(
                            plot_builder(build(5., 3.4))
                                .range_key(app.live_axis_key("curve"))
                                .interactive()
                                .build(cx),
                        );
                    }
                    Err(error) => {
                        app.live.plot = None;
                        app.live.monitor_plot = None;
                        app.live.plotted_fit_status = None;
                        app.live.fit_parameters.clear();
                        app.live.plot_error = Some(error);
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }
    pub(super) fn live_display_choices(&self) -> Vec<LiveDisplay> {
        let mut choices = vec![LiveDisplay::Latest, LiveDisplay::Recent];
        if let Some(i) = self.live.session {
            let config = &self.measurements.archive.live_sessions[i].config;
            if config.merge != LiveMerge::Individual {
                choices.push(LiveDisplay::Average);
            }
            if config.exafs.is_some() {
                choices.extend([LiveDisplay::ExafsK, LiveDisplay::ExafsR]);
            }
            if config.peak_model.is_some() {
                choices.push(LiveDisplay::PeakFit);
            }
        }
        choices
    }
    pub(super) fn live_issue_messages(&self) -> Vec<String> {
        let mut issues: Vec<_> = self
            .live
            .progress
            .review
            .iter()
            .chain(&self.live.progress.failed)
            .map(|(path, error)| {
                format!(
                    "{}: {error}",
                    path.file_name().unwrap_or_default().to_string_lossy()
                )
            })
            .chain(self.live.average_errors.iter().cloned())
            .collect();
        if let Some(session) = self
            .live
            .session
            .and_then(|i| self.measurements.archive.live_sessions.get(i))
        {
            if let Some(run) = self
                .measurements
                .archive
                .runs
                .iter()
                .find(|r| r.id == session.run)
            {
                issues.extend(run.rows.iter().filter(|r| r.result.is_none()).map(|r| {
                    format!(
                        "{}: {}",
                        r.frame.label,
                        r.reason.as_deref().unwrap_or("Processing unavailable")
                    )
                }));
            }
            if let Some(run) = self
                .peaks
                .archive
                .runs
                .iter()
                .find(|r| r.id == session.peak_run_id())
            {
                issues.extend(
                    run.rows
                        .iter()
                        .filter(|r| r.status != crate::series_measurements::FrameStatus::Succeeded)
                        .map(|r| {
                            format!(
                                "{}: {}",
                                r.label,
                                r.reason.as_deref().unwrap_or("Peak fit unavailable")
                            )
                        }),
                );
            }
        }
        if let Some(error) = &self.live.exafs_error {
            issues.push(error.clone());
        }
        if let Some(i) = self.live.session {
            let mut seen = std::collections::BTreeSet::new();
            for row in self.measurements.archive.live_sessions[i]
                .exafs
                .iter()
                .rev()
                .filter(|r| seen.insert(r.group.clone()))
            {
                if let Some(error) = &row.error {
                    issues.push(format!("{} · EXAFS: {error}", row.label));
                }
                for notice in &row.notices {
                    issues.push(format!("{} · {notice}", row.label));
                }
            }
        }
        issues
    }
    pub(super) fn live_status_text(&self) -> String {
        let status = if self.live.running {
            "Watching"
        } else if self.live.busy {
            "Pausing…"
        } else if self
            .live
            .session
            .is_some_and(|i| self.measurements.archive.live_sessions[i].stopped)
        {
            "Finished"
        } else {
            "Paused"
        };
        let issues = self.live_issue_messages().len();
        format!(
            "{status} · {} {}{}",
            self.live.accepted_sources.len(),
            if self.live.accepted_sources.len() == 1 {
                "file"
            } else {
                "files"
            },
            if issues == 0 {
                String::new()
            } else {
                format!(" · {issues} need attention")
            }
        )
    }
    pub(crate) fn live_status_indicator(&self, cx: &Context<Self>) -> Option<AnyElement> {
        self.live.session?;
        let t = self.theme;
        let tip = Tooltip {
            label: format!("Live · {} · Open monitor", self.live_status_text()).into(),
            theme: t,
        };
        Some(
            crate::accessibility::Control::new(
                div().id("live-status"),
                format!("Live · {} · Open monitor", self.live_status_text()),
                accesskit::Role::Button,
            )
            .tab_index(0)
            .key_context("Control")
            .h(px(28.))
            .px_2()
            .mr_2()
            .flex()
            .items_center()
            .gap_2()
            .rounded_sm()
            .border_1()
            .border_color(gpui::transparent_black())
            .text_size(px(12.))
            .cursor_pointer()
            .hover(|d| d.bg(t.raised))
            .focus(|d| d.border_color(t.accent))
            .tooltip(move |_, cx| cx.new(|_| tip.clone()).into())
            .child(div().size(px(6.)).rounded_full().bg(
                if !self.live_issue_messages().is_empty() {
                    t.warn
                } else if self.live.running {
                    t.success
                } else {
                    t.text_muted
                },
            ))
            .child(format!(
                "Live · {}",
                if self.live.running {
                    self.live.accepted_sources.len().to_string()
                } else {
                    self.live_status_text()
                        .split(" · ")
                        .next()
                        .unwrap_or("Paused")
                        .to_owned()
                }
            ))
            .on_click(cx.listener(|app, _, _, cx| {
                if let Some(handle) = app.live.monitor_window {
                    cx.defer(move |cx| {
                        handle
                            .update(cx, |_, window, _| window.activate_window())
                            .ok();
                    });
                } else {
                    app.set_live_host(false, true, cx);
                }
            }))
            .into_any_element(),
        )
    }
    pub(super) fn toggle_live_pause(&mut self, cx: &mut Context<Self>) {
        if self.live.running {
            self.live.stop();
            self.live.message = "Paused".into();
        } else {
            self.resume_live(cx);
        }
        cx.notify();
    }
    pub(crate) fn live_monitor_panel(&mut self, cx: &mut Context<Self>) -> Option<AnyElement> {
        if !self.live.monitor_docked
            || self.live.session.is_none()
            || self.live.open && self.stage == Stage::Series
        {
            return None;
        }
        Some(
            div()
                .w(px(340.))
                .flex_none()
                .min_h_0()
                .border_l_1()
                .border_color(self.theme.border)
                .child(self.live_monitor_content(false, cx))
                .into_any_element(),
        )
    }
    fn live_monitor_content(&mut self, floating: bool, cx: &mut Context<Self>) -> AnyElement {
        self.refresh_live_spectrum(cx);
        let t = self.theme;
        let mut view = div()
            .id("live-monitor")
            .size_full()
            .min_h_0()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap_3()
            .p_3()
            .bg(t.surface)
            .text_color(t.text)
            .text_size(px(12.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(
                        div()
                            .flex_1()
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .child("Live"),
                    )
                    .child(self.live_pause_control(cx))
                    .child(self.live_actions(floating, cx))
                    .child(
                        icon_button(
                            &t,
                            "live-monitor-hide",
                            Icon::Close,
                            "Hide monitor; acquisition continues",
                            false,
                        )
                        .on_click(cx.listener(|app, _, _, cx| app.set_live_host(false, false, cx))),
                    ),
            )
            .child(
                div()
                    .text_color(t.text_muted)
                    .child(self.live_status_text()),
            )
            .child(self.live_view_controls(floating, cx));
        if let Some(error) = &self.live.plot_error {
            view = view.child(div().text_color(t.warn).child(error.clone()));
        }
        let fit_view = matches!(self.live.display, LiveDisplay::ExafsK | LiveDisplay::ExafsR);
        if fit_view {
            view = view.child(
                super::super::controls::disclosure(
                    &t,
                    "live-fit-comparison",
                    "Fit comparison",
                    self.live.monitor_fit_curve,
                    false,
                )
                .on_click(cx.listener(|app, _, _, cx| {
                    app.live.monitor_fit_curve = !app.live.monitor_fit_curve;
                    cx.notify();
                })),
            );
        }
        if let Some(plot) = &self.live.monitor_plot
            && (!fit_view || self.live.monitor_fit_curve)
        {
            view = view.child(div().h(px(240.)).flex_none().child(plot.clone()));
        } else if self.live.monitor_plot.is_none() {
            view = view.child(
                div()
                    .h(px(240.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(t.text_muted)
                    .child("Waiting for a completed scan…"),
            );
        }
        if !self.live.plotted_label.is_empty() {
            view = view.child(
                div()
                    .text_size(px(11.))
                    .text_color(t.text_muted)
                    .child(self.live.plotted_label.clone()),
            );
        }
        if let Some(value) = self.live.plot_value
            && let Some(session) = self
                .live
                .session
                .and_then(|i| self.measurements.archive.live_sessions.get(i))
        {
            view = view.child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .flex_1()
                            .text_size(px(11.))
                            .text_color(t.text_muted)
                            .child(session.config.definition.name.clone()),
                    )
                    .child(
                        div()
                            .font_family(crate::theme::MONO)
                            .child(format!("{value:.5}")),
                    ),
            );
        }
        if let Some(status) = &self.live.plotted_fit_status {
            view = view.child(div().text_color(t.text_muted).child(status.clone()));
        }
        view = view
            .child(
                div()
                    .border_t_1()
                    .border_color(t.border)
                    .pt_1()
                    .child(self.live_follow_control(cx)),
            )
            .child(self.live_fit_trend_controls(floating, cx))
            .when(
                matches!(self.live.display, LiveDisplay::ExafsK | LiveDisplay::ExafsR),
                |d| {
                    d.when_some(self.live.monitor_trend.clone(), |d, p| {
                        d.child(div().h(px(180.)).flex_none().child(p))
                    })
                },
            )
            .child(self.live_fit_parameters(cx))
            .child(self.live_issues(cx));
        view.into_any_element()
    }
    pub(super) fn live_view_controls(&self, floating: bool, cx: &mut Context<Self>) -> AnyElement {
        let channel = self
            .live
            .channel
            .as_deref()
            .map(channel_label)
            .unwrap_or_else(|| "Signal".into());
        let mode = match self.live.display {
            LiveDisplay::Latest => "Latest scan",
            LiveDisplay::Recent => "Last 5 scans",
            LiveDisplay::Average => "Average",
            LiveDisplay::PeakFit => "XANES peaks",
            LiveDisplay::ExafsK => "EXAFS · k",
            LiveDisplay::ExafsR => "EXAFS · R",
        };
        div()
            .flex()
            .items_center()
            .gap_2()
            .child(
                self.live_selector(
                    "live-channel",
                    "Signal",
                    channel,
                    LiveMenu::Channel,
                    floating,
                    cx,
                )
                .flex_1(),
            )
            .child(
                self.live_selector("live-view", "View", mode, LiveMenu::View, floating, cx)
                    .flex_1(),
            )
            .into_any_element()
    }
    pub(super) fn set_live_host(&mut self, dock: bool, pop: bool, cx: &mut Context<Self>) {
        let studio = cx.weak_entity();
        cx.defer(move |cx| {
            let Some(app) = studio.upgrade() else {
                return;
            };
            let old = app.update(cx, |app, cx| {
                app.live.menu = None;
                app.live.monitor_docked = dock;
                cx.notify();
                app.live.monitor_window.take()
            });
            if let Some(handle) = old {
                handle
                    .update(cx, |_, window, _| window.remove_window())
                    .ok();
            }
            if !pop {
                return;
            }
            let Some(session) = app.read(cx).live.session.map(|i| {
                app.read(cx).measurements.archive.live_sessions[i]
                    .config
                    .id
                    .clone()
            }) else {
                return;
            };
            let close_studio = studio.clone();
            let bounds = gpui::Bounds::centered(None, gpui::size(px(370.), px(580.)), cx);
            let opened = cx.open_window(
                gpui::WindowOptions {
                    show: false,
                    focus: false,
                    window_bounds: Some(gpui::WindowBounds::Windowed(bounds)),
                    titlebar: Some(gpui::TitlebarOptions {
                        title: Some("rexafs Live monitor".into()),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                move |window, cx| {
                    crate::accessibility::install(window, cx);
                    window.on_window_should_close(cx, move |_, cx| {
                        close_studio
                            .update(cx, |app, cx| {
                                app.live.monitor_window = None;
                                cx.notify();
                            })
                            .ok();
                        true
                    });
                    cx.new(|cx| {
                        if let Some(app) = studio.upgrade() {
                            cx.observe(&app, |_, _, cx| cx.notify()).detach();
                            cx.observe_release(&app, |_, _, cx| cx.notify()).detach();
                        }
                        LiveMonitorWindow { studio, session }
                    })
                },
            );
            app.update(cx, |app, cx| {
                match opened {
                    Ok(handle) => {
                        handle
                            .update(cx, |_, window, _| crate::accessibility::show(window))
                            .ok();
                        app.live.monitor_window = Some(handle.into());
                    }
                    Err(error) => {
                        app.live.monitor_docked = true;
                        app.live.message = format!("Monitor docked: {error}");
                    }
                }
                cx.notify();
            });
        });
    }
    /// Hide retained contributors in averaged sessions, while keeping explicitly
    /// marked/current scans visible. This changes presentation, never provenance.
    pub(crate) fn live_average_update_banner(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let index = self.selected?;
        let group = index
            .checked_sub(crate::app::DERIVED_BASE)
            .and_then(|i| self.derived.get(i))?;
        if !group
            .operation
            .as_ref()
            .is_some_and(|o| o.tool == "Live average")
        {
            return None;
        }
        let path = group.source.as_ref()?;
        if self.spectrum_path == *path {
            return None;
        }
        let path = path.clone();
        let label = group.label.clone();
        Some(
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap_2()
                .px_3()
                .py_1()
                .bg(self.theme.raised)
                .child("Live average updated · analysis view held")
                .child(
                    button(&self.theme, "live-refresh-analysis", "Refresh", false).on_click(
                        cx.listener(move |app, _, _, cx| {
                            app.current_path = path.clone();
                            app.load_spectrum(index, path.clone(), label.clone().into(), cx);
                        }),
                    ),
                )
                .into_any_element(),
        )
    }
    pub(crate) fn live_sources_toggle(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        if !self
            .measurements
            .archive
            .live_sessions
            .iter()
            .any(|s| s.config.merge != LiveMerge::Individual)
        {
            return None;
        }
        Some(
            div()
                .px_2()
                .pb_1()
                .child(
                    check(
                        &self.theme,
                        "groups-live-sources",
                        "Show source scans",
                        self.live.show_sources,
                    )
                    .on_click(cx.listener(|app, _, _, cx| {
                        app.live.show_sources = !app.live.show_sources;
                        cx.notify();
                    })),
                )
                .into_any_element(),
        )
    }
    pub(crate) fn live_hidden_sources(&self) -> std::collections::BTreeSet<usize> {
        // Embedded projects also catalog the cached arrays. The named Live
        // result already represents each array, so do not show its storage file.
        let mut hidden: std::collections::BTreeSet<_> = self
            .derived
            .iter()
            .filter(|g| {
                g.operation
                    .as_ref()
                    .is_some_and(|o| matches!(o.tool.as_str(), "Live acquisition" | "Live average"))
            })
            .filter_map(|g| g.source.as_ref())
            .filter_map(|p| self.catalog.find_by_canonical_path(p))
            .collect();
        // A resumed average replaces its portable copy. The embedded catalog
        // may still contain older internal arrays; keep those storage entries
        // hidden after replacement, while retaining the files and provenance.
        hidden.extend(
            self.project_source_origins
                .iter()
                .filter_map(|(cached, original)| {
                    let name = original.file_name()?.to_str()?;
                    let key = name.strip_prefix("average-")?.strip_suffix(".dat")?;
                    (key.len() == 64
                        && key.bytes().all(|c| c.is_ascii_hexdigit())
                        && self
                            .measurements
                            .archive
                            .live_sessions
                            .iter()
                            .any(|s| original.parent() == Some(s.directory.as_path())))
                    .then(|| self.catalog.find_by_canonical_path(cached))
                    .flatten()
                }),
        );
        if self.live.show_sources {
            return hidden;
        }
        let sessions: Vec<_> = self
            .measurements
            .archive
            .live_sessions
            .iter()
            .filter(|s| s.config.merge != LiveMerge::Individual)
            .map(|s| serde_json::json!(s.config.id))
            .collect();
        hidden.extend(
            self.derived
                .iter()
                .enumerate()
                .filter(|(i, g)| {
                    let index = crate::app::DERIVED_BASE + *i;
                    self.selected != Some(index)
                        && !self.selection.contains(&index)
                        && g.operation.as_ref().is_some_and(|o| {
                            o.tool == "Live acquisition"
                                && sessions.contains(&o.parameters["recipe"])
                        })
                })
                .map(|(i, _)| crate::app::DERIVED_BASE + i),
        );
        hidden
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn following_catches_background_arrivals_and_hold_survives_them() {
        let first = GroupId::new_result();
        let second = GroupId::new_result();
        assert_eq!(cursor_index(&[], true, None), None);
        assert_eq!(
            cursor_index(std::slice::from_ref(&first), true, None),
            Some(0)
        );
        let ids = vec![first.clone(), second];
        assert_eq!(cursor_index(&ids, true, Some(&first)), Some(1));
        assert_eq!(cursor_index(&ids, false, Some(&first)), Some(0));
        assert_eq!(cursor_index(&ids, true, Some(&first)), Some(1));
    }
}
