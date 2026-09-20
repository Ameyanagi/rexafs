//! One export menu for processing, comparison, collection and wavelet plots.
use super::*;
use crate::plot_export::{self as data_export, Curve};
use gpui::{Entity, Window};
use ruviz_gpui::RuvizPlot;
use std::io::Write;
mod analysis;

#[derive(Clone, Copy)]
pub(crate) enum Target {
    Quadrant(usize),
    Wavelet,
    SeriesMap,
    SeriesFrame,
    SeriesTrend,
    Analysis,
}

pub(crate) struct Labels {
    pub template: crate::plotting::QuadrantSpec,
    pub x: String,
    pub y: String,
    pub series: Vec<String>,
}

#[derive(Clone, Copy)]
enum Format {
    Csv,
    Png,
    Svg,
}
impl Format {
    fn extension(self) -> &'static str {
        match self {
            Self::Csv => "csv",
            Self::Png => "png",
            Self::Svg => "svg",
        }
    }
}

impl StudioApp {
    pub(crate) fn plot_export_button(
        &self,
        id: impl Into<gpui::ElementId>,
        target: Target,
        cx: &mut Context<Self>,
    ) -> crate::accessibility::Control {
        button(&self.theme, id, "Export ▾", false)
            .flex_none()
            .on_click(cx.listener(move |app, event, window, cx| {
                app.ui.plot_export = Some(target);
                app.open_chrome_menu(super::controls::Menu::ExportPlot, event, window, cx);
            }))
    }

    pub(crate) fn plot_export_menu(&self, cx: &mut Context<Self>) -> gpui::Div {
        let mut menu = div().flex().flex_col().gap_1();
        for (format, label) in [
            (Format::Csv, "Data · CSV…"),
            (Format::Png, "Image · PNG…"),
            (Format::Svg, "Vector image · SVG…"),
        ] {
            menu = menu.child(
                button(
                    &self.theme,
                    format!("export-{}", format.extension()),
                    label,
                    false,
                )
                .w_full()
                .on_click(cx.listener(move |app, _, window, cx| {
                    let target = app.ui.plot_export;
                    app.close_chrome_menu(window, cx);
                    if let Some(target) = target {
                        app.export_plot(target, format, window, cx);
                    }
                })),
            );
        }
        if matches!(self.ui.plot_export, Some(Target::Wavelet)) {
            menu = menu.child(
                button(
                    &self.theme,
                    "export-wavelet-json",
                    "Full map and provenance · JSON…",
                    false,
                )
                .w_full()
                .on_click(cx.listener(|app, _, window, cx| {
                    app.close_chrome_menu(window, cx);
                    app.export_wavelet(cx);
                })),
            );
        }
        menu.child(
            div()
                .text_size(px(11.))
                .text_color(self.theme.text_muted)
                .child(if matches!(self.ui.plot_export, Some(Target::Wavelet)) {
                    "CSV exports the full native k–R grid."
                } else {
                    "CSV includes all plotted curves and labels. Zoom does not trim data."
                }),
        )
    }

    fn export_entity(&self, target: Target) -> Option<Entity<RuvizPlot>> {
        match target {
            Target::Quadrant(i) => self.quadrants.get(i).map(|(_, p)| p.clone()),
            Target::Wavelet => self.wavelet.plot_map.clone(),
            Target::SeriesMap => self.operando_plots.as_ref().map(|p| p.heatmap.clone()),
            Target::SeriesFrame => self.operando_plots.as_ref().map(|p| p.chik.clone()),
            Target::SeriesTrend => self.operando_plots.as_ref().map(|p| p.trend.clone()),
            Target::Analysis => self.analysis.plot.clone(),
        }
    }

    fn export_plot(
        &mut self,
        target: Target,
        format: Format,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(entity) = self.export_entity(target) else {
            return;
        };
        // Capture the chosen plot before opening a native file dialog. The plot
        // owns the comparison curves; export never falls back to the active group.
        let session = entity.read(cx).interactive_session();
        let mut plot = if let Target::Quadrant(index) = target {
            // Freeze reactive arrays at the time Export is chosen and include the
            // spectrum labels normally drawn in the GUI's separate legend strip.
            let labels = &self.quad_export_labels[index];
            let mut spec = labels.template.clone();
            let (_, sources) = &self.quad_bindings[index];
            for (i, (series, source)) in spec.series.iter_mut().zip(sources).enumerate() {
                series.x = source.x.read().clone();
                series.y = source.y.read().clone();
                series.label = Some(labels.series[i].clone());
            }
            spec.series = spec
                .series
                .into_iter()
                .enumerate()
                .filter(|(i, _)| session.series_visible(*i))
                .map(|(_, s)| s)
                .collect();
            if (2..=24).contains(&spec.series.len()) {
                spec.legend_columns = Some(spec.series.len().min(3));
            }
            let (width, height) = self.card_px.get(&index).copied().unwrap_or((820, 580));
            spec.to_plot(&self.theme).0.size_px(width, height)
        } else {
            session.prepared_plot().plot().clone()
        };
        let bounds = session.view_bounds_snapshot().visible_bounds;
        plot = plot
            .xlim(bounds.min.x, bounds.max.x)
            .ylim(bounds.min.y, bounds.max.y);
        let wavelet = matches!(target, Target::Wavelet)
            .then(|| self.wavelet.record.clone())
            .flatten();
        let curves = if matches!(format, Format::Csv) && wavelet.is_none() {
            match self.export_curves(target, cx) {
                Ok(curves) => curves,
                Err(error) => {
                    self.record_job_error("Plot export", error);
                    cx.notify();
                    return;
                }
            }
        } else {
            Vec::new()
        };
        let name = format!("rexafs-plot.{}", format.extension());
        let request = cx.prompt_for_new_path(&std::env::temp_dir(), Some(&name));
        let project = self.project_generation;
        cx.spawn(async move |this, cx| {
            let Ok(Ok(Some(path))) = request.await else {
                return;
            };
            let destination = path.clone();
            let result = cx
                .background_spawn(async move {
                    let mut temp = tempfile::NamedTempFile::new_in(
                        path.parent().ok_or("Export directory unavailable")?,
                    )
                    .map_err(|e| e.to_string())?;
                    match format {
                        Format::Csv => {
                            if let Some(record) = wavelet {
                                data_export::wavelet_csv(&record.map, &mut temp)?;
                            } else {
                                data_export::curves_csv(&curves, &mut temp)?;
                            }
                        }
                        Format::Png => temp
                            .write_all(&plot.render_png_bytes().map_err(|e| e.to_string())?)
                            .map_err(|e| e.to_string())?,
                        Format::Svg => plot.export_svg(temp.path()).map_err(|e| e.to_string())?,
                    }
                    temp.as_file().sync_all().map_err(|e| e.to_string())?;
                    temp.persist(&path).map_err(|e| e.to_string())?;
                    Ok::<_, String>(())
                })
                .await;
            this.update(cx, |app, cx| {
                if app.project_generation != project {
                    return;
                }
                match result {
                    Ok(()) => app.status = format!("Exported {}", destination.display()).into(),
                    Err(error) => app.record_job_error("Plot export", error),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn export_curves(&self, target: Target, cx: &Context<Self>) -> Result<Vec<Curve>, String> {
        let missing = || "No plotted data to export".to_string();
        match target {
            Target::Quadrant(i) => {
                let (_, sources) = self.quad_bindings.get(i).ok_or_else(missing)?;
                let labels = self.quad_export_labels.get(i).ok_or_else(missing)?;
                let plot = self.quadrants[i].1.read(cx).interactive_session();
                Ok(sources
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| plot.series_visible(*i))
                    .map(|(i, source)| Curve {
                        label: labels.series[i].clone(),
                        x_label: labels.x.clone(),
                        y_label: labels.y.clone(),
                        x: source.x.read().clone(),
                        y: source.y.read().clone(),
                    })
                    .collect())
            }
            Target::SeriesMap | Target::SeriesFrame => {
                let data = self.operando.as_ref().ok_or_else(missing)?;
                let (grid, matrix) = data.space(self.stage_view.series_space);
                let (x_label, mut y_label) = match self.stage_view.series_space {
                    crate::app::SeriesSpace::Energy => {
                        ("Energy (eV)", "normalized μ(E)".to_string())
                    }
                    crate::app::SeriesSpace::Flat => ("Energy (eV)", "flattened μ(E)".to_string()),
                    crate::app::SeriesSpace::K => (
                        crate::plotting::K_AXIS,
                        crate::plotting::chik_label(self.series_display_kweight()),
                    ),
                    crate::app::SeriesSpace::R => (
                        crate::plotting::R_AXIS,
                        crate::plotting::chir_label(self.series_display_kweight()),
                    ),
                };
                if self.series_display.difference {
                    y_label = format!(
                        "Δ {y_label} · reference frame {}",
                        self.active_series_reference()
                            .map(|r| (r.frame + 1).to_string())
                            .unwrap_or_else(|| "unavailable".into())
                    );
                }
                let rows = if matches!(target, Target::SeriesFrame) {
                    vec![(
                        self.time_pos,
                        self.operando_plots
                            .as_ref()
                            .ok_or_else(missing)?
                            .chik_series
                            .values
                            .read()
                            .clone(),
                    )]
                } else {
                    data.sample_frames
                        .iter()
                        .copied()
                        .zip(self.series_difference_matrix(matrix))
                        .collect()
                };
                Ok(rows
                    .into_iter()
                    .map(|(frame, y)| Curve {
                        label: format!("Frame {}", frame + 1),
                        x_label: x_label.into(),
                        y_label: y_label.clone(),
                        x: grid.to_vec(),
                        y,
                    })
                    .collect())
            }
            Target::SeriesTrend => {
                let trend = self.trend_snapshot();
                Ok(vec![Curve {
                    label: trend.name.clone(),
                    x_label: "Frame (one-based)".into(),
                    y_label: trend.name,
                    x: trend.frames.into_iter().map(|frame| frame + 1.0).collect(),
                    y: trend.values,
                }])
            }
            Target::Analysis => self.analysis_export_curves(),
            Target::Wavelet => Err(missing()),
        }
    }
}
