//! Center for the four processing stages: a plot bar, the stage's main
//! plot(s) with drag handles, the shared legend strip, and the four-thumbnail
//! ripple strip.

use gpui::{
    ClickEvent, Context, IntoElement, ParentElement, SharedString, Styled, div, prelude::*, px,
};

use super::{
    BkgView, EQuantity, MONO, PlotScope, Stage, StageStatus, TfView, chip, segment, segmented,
};
use crate::app::{ParamKey, StudioApp};

/// Quadrant slots in `StudioApp::quadrants` (built by `build_quadrant_specs`).
pub const PLOT_MU: usize = 0;
pub const PLOT_NORM: usize = 1;
pub const PLOT_CHIK: usize = 2;
pub const PLOT_CHIR: usize = 3;
pub const PLOT_CHIQ: usize = 4;

fn current_label(
    selected: Option<usize>,
    entry_label: impl FnOnce(usize) -> String,
) -> SharedString {
    entry_label(selected.unwrap_or(crate::app::NO_ENTRY)).into()
}

impl StudioApp {
    pub(crate) fn stage_center(&mut self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let t = self.theme;
        let ready = self.quadrants.len() > PLOT_CHIQ;
        if !ready && self.current_path.as_os_str().is_empty() && !self.catalog.scanning {
            return div().flex_1().flex().child(self.empty_drop_target(cx));
        }
        let mut column = div()
            .flex_1()
            .min_h_0()
            .min_w_0()
            .flex()
            .flex_col()
            .child(self.plot_bar(cx));
        if let Some(weight) = self.mixed_overlay_weight {
            column = column.child(div().px_3().py_1().text_size(px(11.5)).text_color(t.warn)
                .child(format!("Mixed FT weights: χ(k) uses k^{weight} for all curves. R/q curves retain each group's weight (shown in the legend).")));
        }
        if !ready {
            return column.child(
                div()
                    .flex_1()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(t.text_muted)
                    .child(self.status.clone()),
            );
        }
        let tool_preview = (self.stage == Stage::Data && self.tool_preview_current(cx))
            .then(|| self.tools.preview_plot.clone())
            .flatten();
        let plots: Vec<(usize, SharedString)> = if tool_preview.is_some() {
            Vec::new()
        } else {
            self.stage_plots()
        };
        let mut area = div()
            .flex_1()
            .min_h_0()
            .min_w_0()
            .flex()
            .flex_col()
            .gap_2()
            .px_3()
            .pt_2();
        if let Some(plot) = tool_preview {
            area = area.child(
                div()
                    .flex_1()
                    .min_h_0()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .text_size(px(11.5))
                            .text_color(t.text_muted)
                            .child("Preview · original / result / standard"),
                    )
                    .child(div().flex_1().min_h_0().min_w_0().child(plot)),
            );
        }
        for (index, title) in plots {
            area = area.child(self.plot_card(index, title, cx));
        }
        if self.stage == Stage::Data
            && let Some(plot) = self.analysis.plot.clone()
            && let Some(tool) = self.analysis.shown
        {
            let title: SharedString = match tool {
                super::tools::Tool::Lcf => {
                    "linear combination fit · data / fit / components / residual".into()
                }
                _ => "PCA target transform · data / reconstruction / residual".into(),
            };
            area = area.child(
                div()
                    .flex_none()
                    .h(px(300.))
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .rounded_lg()
                    .bg(t.raised)
                    .border_1()
                    .border_color(t.border)
                    .child(
                        div()
                            .flex_none()
                            .px_3()
                            .pt_2()
                            .text_size(px(11.5))
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .child(title),
                    )
                    .child(div().flex_1().min_h_0().min_w_0().p_1().child(plot)),
            );
        }
        column = column.child(area);
        if self.view.legend && !self.legend_entries.is_empty() {
            column = column.child(self.legend_strip());
        }
        column.when(self.ui.overview, |d| d.child(self.thumbnail_strip(cx)))
    }

    /// Which quadrant plots the current stage shows, top to bottom.
    pub(crate) fn stage_plots(&self) -> Vec<(usize, SharedString)> {
        stage_plot_selection(
            self.stage,
            self.stage_view,
            self.fft_summary().3,
            self.mixed_overlay_weight.is_some(),
            self.spectrum_quantity,
        )
    }

    fn plot_bar(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        use super::controls::{Menu, icon_button};
        use crate::icons::Icon;
        let t = self.theme;
        let v = self.stage_view;
        let mut bar = div()
            .min_h(px(36.))
            .min_w_0()
            .w_full()
            .flex_none()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_1()
            .px_2()
            .py_1()
            .bg(t.surface)
            .border_b_1()
            .border_color(t.border)
            .child(
                icon_button(
                    &t,
                    "plot-compare",
                    Icon::Layers,
                    format!(
                        "Compare current + {} marked · {} spectra",
                        self.selection.len(),
                        self.compare_count()
                    ),
                    v.scope == PlotScope::Marked,
                )
                .w_auto()
                .px_2()
                .gap_1()
                .child(format!("Compare {}", self.compare_count()))
                .on_click(cx.listener(|app, _, _, cx| {
                    app.stage_view.scope = if app.stage_view.scope == PlotScope::Current {
                        PlotScope::Marked
                    } else {
                        PlotScope::Current
                    };
                    app.stage_view_changed(cx);
                })),
            );
        if self.stage.is_processing() && !self.spectrum_quantity.is_absorption() {
            return bar.child(self.spectrum_quantity.label());
        }
        let mut choices = segmented(&t).flex_none();
        match self.stage {
            Stage::Data | Stage::Normalize => {
                for (index, (quantity, label)) in [
                    (EQuantity::Mu, "μ(E)"),
                    (EQuantity::Norm, "norm"),
                    (EQuantity::Flat, "flat"),
                ]
                .into_iter()
                .enumerate()
                {
                    choices = choices.child(
                        segment(
                            &t,
                            ("energy-view", index),
                            label,
                            v.e_quantity == quantity,
                            index == 0,
                        )
                        .on_click(cx.listener(move |app, _, _, cx| {
                            app.stage_view.e_quantity = quantity;
                            app.stage_view_changed(cx);
                        })),
                    );
                }
            }
            Stage::Background => {
                for (index, (view, label)) in [(BkgView::Energy, "μ(E)"), (BkgView::K, "χ(k)")]
                    .into_iter()
                    .enumerate()
                {
                    choices = choices.child(
                        segment(
                            &t,
                            ("background-view", index),
                            label,
                            v.bkg_view == view,
                            index == 0,
                        )
                        .on_click(cx.listener(move |app, _, _, cx| {
                            app.stage_view.bkg_view = view;
                            app.stage_view_changed(cx);
                        })),
                    );
                }
            }
            Stage::Transform => {
                for (index, (view, label)) in [
                    (TfView::K, "k"),
                    (TfView::R, "R"),
                    (TfView::Both, "k + R"),
                    (TfView::Q, "q"),
                ]
                .into_iter()
                .enumerate()
                {
                    choices = choices.child(
                        segment(
                            &t,
                            ("transform-view", index),
                            label,
                            v.tf_view == view,
                            index == 0,
                        )
                        .on_click(cx.listener(move |app, _, _, cx| {
                            app.stage_view.tf_view = view;
                            if view == TfView::Q {
                                app.ui.sections.insert("back-transform");
                            }
                            app.stage_view_changed(cx);
                        })),
                    );
                }
            }
            _ => {}
        }
        bar = bar.child(choices);
        match self.stage {
            Stage::Data | Stage::Normalize => {
                bar =
                    bar.child(chip(&t, "common-e0", "E₀", self.view.show_e0).on_click(
                        cx.listener(|a, _, _, c| {
                            a.view.show_e0 = !a.view.show_e0;
                            a.stage_view_changed(c);
                        }),
                    ))
                    .child(
                        chip(&t, "common-derivative", "dμ/dE", self.view.show_deriv).on_click(
                            cx.listener(|a, _, _, c| {
                                a.view.show_deriv = !a.view.show_deriv;
                                a.stage_view_changed(c);
                            }),
                        ),
                    );
                if self.stage == Stage::Normalize {
                    bar = bar.child(
                        chip(
                            &t,
                            "common-pre-post",
                            "Pre/post",
                            self.view.show_pre && self.view.show_post,
                        )
                        .on_click(cx.listener(|a, _, _, c| {
                            let on = !(a.view.show_pre && a.view.show_post);
                            a.view.show_pre = on;
                            a.view.show_post = on;
                            a.stage_view_changed(c);
                        })),
                    );
                }
            }
            Stage::Background => {
                bar = bar.child(self.kweight_buttons(cx)).child(
                    chip(&t, "common-spline", "Spline", v.show_bkg).on_click(cx.listener(
                        |a, _, _, c| {
                            a.stage_view.show_bkg = !a.stage_view.show_bkg;
                            a.stage_view_changed(c);
                        },
                    )),
                );
            }
            Stage::Transform => {
                bar = bar
                    .child(self.kweight_buttons(cx))
                    .child(
                        chip(&t, "common-real", "Re", v.show_re).on_click(cx.listener(
                            |a, _, _, c| {
                                a.stage_view.show_re = !a.stage_view.show_re;
                                a.stage_view_changed(c);
                            },
                        )),
                    )
                    .child(
                        chip(&t, "common-window", "Window", self.view.show_kwin).on_click(
                            cx.listener(|a, _, _, c| {
                                a.view.show_kwin = !a.view.show_kwin;
                                a.invalidate_explore_plots(c);
                                c.notify();
                            }),
                        ),
                    );
            }
            _ => {}
        }
        bar = bar
            .child(div().flex_1())
            .child(
                chip(
                    &t,
                    "spectrum-colors",
                    "Colors",
                    self.ui.menu == Some(Menu::Colors),
                )
                .on_click(cx.listener(|app, event, window, cx| {
                    app.open_chrome_menu(Menu::Colors, event, window, cx);
                })),
            )
            .child(
                icon_button(
                    &t,
                    "plot-overview",
                    Icon::Grid,
                    "Overview plots",
                    self.ui.overview,
                )
                .on_click(cx.listener(|app, _, _, cx| {
                    app.ui.overview = !app.ui.overview;
                    cx.notify();
                })),
            )
            .child(
                icon_button(
                    &t,
                    "plot-options",
                    Icon::Sliders,
                    "Plot options",
                    self.ui.menu == Some(Menu::Plot),
                )
                .on_click(cx.listener(|app, event, window, cx| {
                    app.open_chrome_menu(Menu::Plot, event, window, cx)
                })),
            );
        bar
    }

    pub(crate) fn plot_options(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let panel = div().flex().flex_col().gap_1();
        panel
            .child(self.plot_option(
                "view-offset",
                "Stack curves",
                self.view.layout == crate::plotting::TraceLayout::Waterfall,
                |a, c| {
                    a.view.layout = if a.view.layout == crate::plotting::TraceLayout::Waterfall {
                        crate::plotting::TraceLayout::Overlay
                    } else {
                        crate::plotting::TraceLayout::Waterfall
                    };
                    a.invalidate_explore_plots(c);
                    c.notify();
                },
                cx,
            ))
            .child(self.plot_option(
                "view-legend",
                "Legend",
                self.view.legend,
                |a, c| {
                    a.view.legend = !a.view.legend;
                    a.invalidate_explore_plots(c);
                    c.notify();
                },
                cx,
            ))
            .child(self.plot_option(
                "view-grid",
                "Grid",
                self.view.grid,
                |a, c| {
                    a.view.grid = !a.view.grid;
                    a.invalidate_explore_plots(c);
                    c.notify();
                },
                cx,
            ))
            .child(self.plot_option(
                "view-overview",
                "Overview plots",
                self.ui.overview,
                |a, c| {
                    a.ui.overview = !a.ui.overview;
                    c.notify();
                },
                cx,
            ))
    }

    /// k-weight 0/1/2/3 (plotting + forward FT; AUTOBK's k-weight is its own).
    fn kweight_buttons(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let t = self.theme;
        let current = self.fft_summary().3.round() as i32;
        let mut row = div().flex().items_center().gap_1().child(
            div()
                .text_size(px(11.))
                .text_color(t.text_muted)
                .child("k-weight"),
        );
        for kw in 0..=3 {
            let on = kw == current;
            row = row.child(
                crate::accessibility::Control::new(
                    div().id(SharedString::from(format!("kw-{kw}"))),
                    format!("k-weight {kw}"),
                    accesskit::Role::Tab,
                )
                .selected(on)
                .w(px(24.))
                .h(px(22.))
                .flex()
                .items_center()
                .justify_center()
                .rounded_md()
                .border_1()
                .font_family(MONO)
                .text_size(px(11.5))
                .cursor_pointer()
                .when(on, |d| {
                    d.bg(gpui::Rgba {
                        a: 0.16,
                        ..t.accent
                    })
                    .border_color(t.accent)
                    .text_color(t.text)
                })
                .when(!on, |d| {
                    d.border_color(t.border)
                        .text_color(t.text_muted)
                        .hover(|d| d.bg(t.raised))
                })
                .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                    this.apply_param(ParamKey::FftKweight, Some(kw as f64), cx);
                    this.sync_param_fields(cx);
                }))
                .child(kw.to_string()),
            );
        }
        row
    }

    /// One main plot with its title, the handle overlay, and a hover hint.
    fn plot_card(
        &mut self,
        index: usize,
        title: SharedString,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let t = self.theme;
        let plot = self.quadrants[index].1.clone();
        let label = self.current_group_label();
        let overlay = self.handle_overlay(index, cx);
        div()
            .id(SharedString::from(format!("plot-card-{index}")))
            .flex_1()
            .min_h_0()
            .min_w_0()
            .relative()
            .flex()
            .flex_col()
            .rounded_lg()
            .bg(t.raised)
            .border_1()
            .border_color(t.border)
            .on_mouse_move(cx.listener(move |this, ev: &gpui::MouseMoveEvent, _w, cx| {
                this.plot_pointer_move(index, ev.position, cx);
            }))
            .capture_any_mouse_down(cx.listener(move |this, ev: &gpui::MouseDownEvent, _, cx| {
                this.capture_handle_press(index, ev, cx);
            }))
            .child(
                div()
                    .flex_none()
                    .px_3()
                    .pt_2()
                    .flex()
                    .items_center()
                    .gap_2()
                    .text_size(px(11.5))
                    .child(div().font_weight(gpui::FontWeight::MEDIUM).child(title))
                    .child(div().text_color(t.text_muted).child(label)),
            )
            .when(
                self.stage_view.scope == PlotScope::Marked && self.stage != Stage::Fit,
                |d| {
                    d.children(
                        self.plot_coverage
                            .get(index)
                            .and_then(|coverage| coverage.disclosure())
                            .map(|text| {
                                div()
                                    .px_3()
                                    .pt_1()
                                    .text_size(px(11.5))
                                    .text_color(t.warn)
                                    .child(text)
                            }),
                    )
                },
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .min_w_0()
                    .p_1()
                    .relative()
                    .child(plot)
                    .child(self.measure_card(index, cx))
                    .children(self.handle_layer(index, cx)),
            )
            .children(overlay)
    }

    pub(crate) fn current_group_label(&self) -> SharedString {
        current_label(self.selected, |ix| self.entry_label(ix))
    }

    /// Clicking a thumbnail opens exactly its current-data quantity.
    fn thumbnail_strip(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let t = self.theme;
        let kw = self.fft_summary().3;
        let captions: [(Stage, String); 4] = [
            (Stage::Normalize, "norm μ(E)".into()),
            (Stage::Background, crate::plotting::chik_label(kw)),
            (
                Stage::Transform,
                format!("{} · window", crate::plotting::chik_label(kw)),
            ),
            (Stage::Transform, "|χ(R)|".into()),
        ];
        let mut strip = div()
            .h(px(118.))
            .flex_none()
            .flex()
            .gap_2()
            .px_3()
            .pt_2()
            .pb_3();
        for (i, (stage, caption)) in captions.into_iter().enumerate() {
            let active = self.stage_view.thumbnail_focus == Some(i);
            let (status, _) = self.stage_summary(stage);
            let dot = if status == StageStatus::Idle {
                t.text_muted
            } else {
                status.color(&t)
            };
            let data = self.thumbs.clone();
            strip = strip.child(
                div()
                    .id(SharedString::from(format!("thumb-{i}")))
                    .flex_1()
                    .min_w_0()
                    .relative()
                    .rounded_md()
                    .bg(t.raised)
                    .border_1()
                    .border_color(if active { t.accent } else { t.border })
                    .cursor_pointer()
                    .hover(|d| d.border_color(t.accent))
                    .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                        this.set_stage(stage, cx);
                        this.stage_view.scope = PlotScope::Current;
                        this.stage_view.e_quantity = EQuantity::Norm;
                        this.stage_view.bkg_view = BkgView::K;
                        this.stage_view.tf_view = if i == 3 { TfView::R } else { TfView::K };
                        this.stage_view.show_re = false;
                        this.view.show_deriv = false;
                        this.view.show_kwin = i == 2;
                        this.stage_view_changed(cx);
                        this.stage_view.thumbnail_focus = Some(i);
                        this.invalidate_explore_plots(cx);
                        cx.notify();
                    }))
                    .child(
                        div()
                            .absolute()
                            .top_1()
                            .left_2()
                            .flex()
                            .items_center()
                            .gap_1p5()
                            .text_size(px(10.5))
                            .text_color(t.text_muted)
                            .child(div().w(px(6.)).h(px(6.)).rounded_full().bg(dot))
                            .child(caption),
                    )
                    .children(data.map(|d| {
                        div()
                            .size_full()
                            .pt_4()
                            .child(super::thumbnails::sparkline(d, i, t))
                    })),
            );
        }
        strip
    }
}

/// Shared main-view selection for freshly created and reopened groups.
pub(crate) fn stage_plot_selection(
    stage: Stage,
    v: super::StageView,
    kw: f64,
    mixed_weights: bool,
    quantity: crate::params::Quantity,
) -> Vec<(usize, SharedString)> {
    if stage.is_processing() && !quantity.is_absorption() {
        return vec![if quantity == crate::params::Quantity::ChiK {
            (PLOT_CHIK, crate::plotting::chik_label(kw).into())
        } else {
            (PLOT_MU, quantity.label().into())
        }];
    }
    let chik: SharedString = crate::plotting::chik_label(kw).into();
    let chir: SharedString = if mixed_weights {
        "|χ(R)| · mixed k weights".into()
    } else {
        crate::plotting::chir_label(kw).into()
    };
    if let Some(i) = v.thumbnail_focus {
        return vec![match i {
            0 => (PLOT_NORM, "normalized μ(E)".into()),
            1 => (PLOT_CHIK, chik),
            2 => (PLOT_CHIK, format!("{} · window", chik).into()),
            _ => (PLOT_CHIR, chir),
        }];
    }
    match stage {
        Stage::Data | Stage::Normalize => vec![match v.e_quantity {
            EQuantity::Mu => (PLOT_MU, "μ(E)".into()),
            EQuantity::Norm => (PLOT_NORM, "normalized μ(E)".into()),
            EQuantity::Flat => (PLOT_NORM, "flattened μ(E)".into()),
        }],
        Stage::Background => match v.bkg_view {
            BkgView::Energy => vec![
                (PLOT_MU, "μ(E) with AUTOBK spline".into()),
                (
                    PLOT_CHIR,
                    "|χ(R)| · R < Rbkg is the background region".into(),
                ),
            ],
            BkgView::K => vec![(PLOT_CHIK, chik), (PLOT_CHIR, chir)],
        },
        Stage::Transform => match v.tf_view {
            TfView::K => vec![(PLOT_CHIK, chik)],
            TfView::R => vec![(PLOT_CHIR, chir)],
            TfView::Q => vec![(PLOT_CHIQ, "χ(q) back-transform".into())],
            TfView::Both => vec![(PLOT_CHIK, chik), (PLOT_CHIR, chir)],
        },
        Stage::Fit | Stage::Series | Stage::Publish => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn current_header_resolves_standalone_and_catalog_display_labels() {
        use crate::app::NO_ENTRY;
        use crate::group_identity::{GroupId, GroupState};
        let standalone = GroupId::new_result();
        let catalog = GroupId::new_result();
        let mut state = GroupState::default();
        state.labels.insert(standalone.clone(), "Cu foil".into());
        state.labels.insert(catalog.clone(), "Cu standard".into());
        for (selected, expected) in [
            (None, "Cu foil"),
            (Some(NO_ENTRY), "Cu foil"),
            (Some(0), "Cu standard"),
        ] {
            let label = super::current_label(selected, |ix| {
                let id = if ix == NO_ENTRY {
                    &standalone
                } else {
                    &catalog
                };
                state.display_label(Some(id), || "load-time label".into())
            });
            assert_eq!(label.as_ref(), expected);
        }
    }

    use super::*;
    use crate::app::shell::StageView;

    #[test]
    fn difference_default_create_reopen_plot_view_uses_quantity_axes() {
        use crate::project::{DataStorage, ProjectFile, load, save_with_storage};
        use crate::{
            params::{DerivedSpectrum, Quantity},
            plotting::{QuadTrace, ViewOptions, quantity_quadrant_specs},
            theme::Theme,
        };
        let temp =
            std::env::temp_dir().join(format!("rexafs-difference-view-{}", std::process::id()));
        std::fs::create_dir_all(&temp).unwrap();
        let mut project = ProjectFile::default();
        let difference = DerivedSpectrum {
            id: 99,
            label: "renamed output".into(),
            quantity: Quantity::NormalizedDifference,
            energy: vec![100., 101., 102.],
            mu: vec![-0.2, 0., 0.3],
            ..Default::default()
        };
        project.derived = vec![difference.clone()];
        project.active_derived = Some(99);
        let assert_default_view = |group: &DerivedSpectrum| {
            let spectrum = std::sync::Arc::new(group.for_display(&project.params).unwrap());
            assert!(spectrum.norm().is_none() && spectrum.flat().is_none());
            let specs = quantity_quadrant_specs(
                &[QuadTrace {
                    color_index: 0,
                    color: None,
                    label: group.display_label(),
                    sp: spectrum,
                    active: true,
                }],
                &ViewOptions::default(),
                &Theme::dark(),
                false,
                group.quantity,
            );
            for stage in [Stage::Data, Stage::Normalize] {
                let view = StageView::default();
                let selected = stage_plot_selection(stage, view, 2., false, group.quantity);
                assert_eq!(selected.len(), 1);
                assert_eq!(selected[0].1.as_ref(), "Δμnorm");
                let plot = &specs[selected[0].0];
                assert_eq!(plot.xlabel, "Energy (eV)");
                assert_eq!(plot.ylabel, "Δμnorm (dimensionless)");
                assert_eq!(plot.series[0].x, group.energy);
                assert_eq!(plot.series[0].y, group.mu);
            }
        };
        assert_default_view(&difference);
        for storage in [DataStorage::Paths, DataStorage::Embedded] {
            let path = temp.join(format!("difference-view-{storage:?}.rxs"));
            save_with_storage(&path, &project, storage).unwrap();
            let reopened = load(&path).unwrap();
            assert_eq!(reopened.active_derived, Some(99));
            assert_default_view(&reopened.derived[0]);
        }
        std::fs::remove_dir_all(temp).unwrap();
    }
}
