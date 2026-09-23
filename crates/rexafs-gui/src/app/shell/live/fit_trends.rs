//! Live fit expressions share Series covariance propagation and retained fits.
use super::*;
use crate::series_fits::{FitSummary, FitTrend};
use gpui::AnyElement;

pub(super) fn estimates(
    history: &[ExafsRow],
    current: &ExafsRow,
    model: &ExafsRecipe,
    directory: &std::path::Path,
    trend: &FitTrend,
    cache: &mut BTreeMap<String, Result<FitSummary, String>>,
) -> Vec<(Option<f64>, Option<crate::fit_details::Estimate>)> {
    let paths = model.materialize_paths(directory);
    let mut values = Vec::new();
    for row in history.iter().filter(|r| r.channel == current.channel) {
        let key = row.artifact_digest.clone();
        if !cache.contains_key(&key) {
            let summary = (|| {
                let r = read_exafs(row)?.result?;
                let paths = paths.as_ref().map_err(Clone::clone)?;
                Ok(FitSummary {
                    variables: r.variables.clone(),
                    varying_names: r.varying_names.clone(),
                    covariance: r.covariance.clone(),
                    paths: crate::fit_details::snapshot(paths, &r),
                    r_factor: r.r_factor,
                    converged: row.converged,
                    notices: row.notices.clone(),
                })
            })();
            cache.insert(key.clone(), summary);
        }
        let estimate = cache
            .get(&key)
            .and_then(|s| s.as_ref().ok())
            .filter(|s| s.converged)
            .and_then(|s| s.estimate(trend));
        values.push((Some((values.len() + 1) as f64), estimate));
        if row.key == current.key {
            break;
        }
    }
    values
}

impl StudioApp {
    pub(super) fn live_fit_trend_choices(&self) -> Vec<FitTrend> {
        let Some(session) = self
            .live
            .session
            .and_then(|i| self.measurements.archive.live_sessions.get(i))
        else {
            return Vec::new();
        };
        session
            .config
            .exafs
            .as_ref()
            .map(crate::series_fits::default_trends)
            .unwrap_or_default()
            .into_iter()
            .chain(session.exafs_trends.clone())
            .collect()
    }
    pub(super) fn live_fit_trend_controls(
        &mut self,
        floating: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        if !matches!(self.live.display, LiveDisplay::ExafsK | LiveDisplay::ExafsR) {
            return div().into_any_element();
        }
        let t = self.theme;
        let choices = self.live_fit_trend_choices();
        let label = choices
            .get(self.live.fit_trend)
            .map(FitTrend::label)
            .unwrap_or_else(|| "Fit parameter".into());
        let mut view = div().flex().flex_col().gap_1().child(
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap_2()
                .child(
                    self.live_selector(
                        "live-fit-trend",
                        "Parameter",
                        label,
                        LiveMenu::FitTrend,
                        floating,
                        cx,
                    )
                    .flex_1(),
                )
                .child(
                    ui::check(
                        &t,
                        "live-fit-errors",
                        "Error bars",
                        self.live.fit_trend_errors,
                    )
                    .on_click(cx.listener(|app, _, _, cx| {
                        app.live.fit_trend_errors = !app.live.fit_trend_errors;
                        app.live.plot_key.clear();
                        app.refresh_live_spectrum(cx);
                        cx.notify();
                    })),
                )
                .child(
                    button(&t, "live-fit-expression", "Expression…", false).on_click(cx.listener(
                        |app, _, _, cx| {
                            app.live.fit_trend_edit = !app.live.fit_trend_edit;
                            cx.notify();
                        },
                    )),
                ),
        );
        if self.live.fit_trend_edit {
            if self.live.fit_trend_fields.is_empty() {
                let selected = choices.get(self.live.fit_trend).or_else(|| choices.first());
                self.live.fit_trend_path = selected.and_then(|trend| trend.path).unwrap_or(0);
                for (name, value) in [
                    ("Live trend name", String::new()),
                    (
                        "Live expression",
                        selected.map(|t| t.expression.clone()).unwrap_or_default(),
                    ),
                    (
                        "Live trend unit",
                        selected.map(|t| t.unit.clone()).unwrap_or_default(),
                    ),
                ] {
                    self.live.fit_trend_fields.push(cx.new(|cx| {
                        let mut field = TextInput::new(name, value, t, cx);
                        field.set_accessible_name(name);
                        field
                    }));
                }
            }
            let path = self
                .live
                .session
                .and_then(|i| self.measurements.archive.live_sessions.get(i))
                .and_then(|s| s.config.exafs.as_ref())
                .and_then(|m| m.paths.get(self.live.fit_trend_path))
                .map(|(p, _)| p.label.clone())
                .unwrap_or_else(|| "Choose path".into());
            let field = |label, input: Entity<TextInput>| {
                div()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(t.text_muted)
                            .child(label),
                    )
                    .child(input)
            };
            view = view.child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .mt_1()
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .child(field("Name", self.live.fit_trend_fields[0].clone()).flex_1())
                            .child(field("Unit", self.live.fit_trend_fields[2].clone()).w(px(64.))),
                    )
                    .child(field("Expression", self.live.fit_trend_fields[1].clone()))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                self.live_selector(
                                    "live-fit-trend-path",
                                    "Path geometry",
                                    path,
                                    LiveMenu::FitTrendPath,
                                    floating,
                                    cx,
                                )
                                .min_w_0()
                                .flex_1(),
                            )
                            .child(button(&t, "live-cancel-fit-trend", "Cancel", false).on_click(
                                cx.listener(|app, _, _, cx| {
                                    app.live.fit_trend_edit = false;
                                    app.live.fit_trend_error.clear();
                                    cx.notify();
                                }),
                            ))
                            .child(button(&t, "live-add-fit-trend", "Add", true).on_click(
                                cx.listener(|app, _, _, cx| app.add_live_fit_trend(cx)),
                            )),
                    )
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(t.text_muted)
                            .child("Use model variables; reff and degen use this path. Unit labels do not convert values."),
                    )
                    .when(!self.live.fit_trend_error.is_empty(), |d| {
                        d.child(div().text_color(t.warn).child(self.live.fit_trend_error.clone()))
                    }),
            );
        }
        view.into_any_element()
    }
    fn add_live_fit_trend(&mut self, cx: &mut Context<Self>) {
        let Some(index) = self.live.session else {
            return;
        };
        let trend = FitTrend {
            name: self.live.fit_trend_fields[0].read(cx).text().trim().into(),
            expression: self.live.fit_trend_fields[1].read(cx).text().trim().into(),
            unit: self.live.fit_trend_fields[2].read(cx).text().trim().into(),
            path: Some(self.live.fit_trend_path),
        };
        if trend.name.is_empty()
            || trend.expression.is_empty()
            || !self
                .live
                .fit_trend_cache
                .values()
                .filter_map(|s| s.as_ref().ok())
                .any(|s| s.estimate(&trend).is_some())
        {
            self.live.fit_trend_error =
                "Enter a name and a valid expression for the selected path.".into();
            cx.notify();
            return;
        }
        self.live.fit_trend = self.live_fit_trend_choices().len();
        self.measurements.archive.live_sessions[index]
            .exafs_trends
            .push(trend);
        self.live.fit_trend_edit = false;
        self.live.fit_trend_error.clear();
        self.live.plot_key.clear();
        self.refresh_live_spectrum(cx);
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{fitting::FitRanges, group_identity::GroupId};
    use rexafs::prelude::{FeffFitResult, FitVariable};
    #[test]
    fn live_trends_hold_the_cursor_keep_gaps_and_reuse_verified_summaries() {
        let tmp = tempfile::tempdir().unwrap();
        let model = ExafsRecipe {
            name: "test".into(),
            paths: vec![],
            variables: vec![],
            ranges: FitRanges::default(),
        };
        let trend = FitTrend {
            name: "twice a".into(),
            expression: "2*a".into(),
            unit: String::new(),
            path: None,
        };
        let rows: Vec<_> = (0..4)
            .map(|i| {
                let mut result = FeffFitResult::default();
                result.variables.insert(
                    "a",
                    FitVariable {
                        value: i as f64,
                        vary: true,
                        ..Default::default()
                    },
                );
                result.varying_names = vec!["a".into()];
                result.covariance = Some(vec![vec![0.04]]);
                let group = GroupId::new_result();
                let key = format!("{i}");
                let record = crate::live::exafs::ExafsRecord {
                    key: key.clone(),
                    group: group.clone(),
                    label: key.clone(),
                    result: Ok(result),
                };
                let (artifact, artifact_digest) =
                    crate::analysis_store::retain(tmp.path(), &record).unwrap();
                ExafsRow {
                    key,
                    group,
                    source: Default::default(),
                    channel: "mu".into(),
                    label: i.to_string(),
                    artifact,
                    artifact_digest,
                    r_factor: Some(0.),
                    converged: i != 1,
                    notices: vec![],
                    error: None,
                }
            })
            .collect();
        let mut cache = Default::default();
        let values = estimates(&rows, &rows[2], &model, tmp.path(), &trend, &mut cache);
        assert_eq!(values.len(), 3);
        assert!(values[1].1.is_none());
        assert!((values[2].1.as_ref().unwrap().stderr.unwrap() - 0.4).abs() < 1e-9);
        std::fs::remove_file(&rows[0].artifact).unwrap();
        let held = estimates(&rows, &rows[0], &model, tmp.path(), &trend, &mut cache);
        assert_eq!(held.len(), 1);
        assert_eq!(held[0].1.as_ref().unwrap().value, 0.);
    }
}
