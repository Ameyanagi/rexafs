//! Series stage (operando / time-resolved): the scan as a time-ordered
//! matrix. Heatmap with a cursor row on the left, the cursor frame and a
//! trend on the right; the inspector carries the cursor readout, the trend
//! table and the batch / LCF-trend cards.

use gpui::{
    ClickEvent, Context, IntoElement, ParentElement, SharedString, Styled, div, prelude::*, px,
    relative,
};

use super::controls::{Tooltip, icon, icon_button};
use super::{MONO, button, chip, section_label, segment, segmented};
use crate::app::{
    FrameFirst, FrameJumpBack, FrameJumpFwd, FrameLast, FrameNext, FramePrev, SeriesSpace,
    StudioApp, TrendSource,
};
use crate::icons::Icon;

impl StudioApp {
    pub(crate) fn series_ready(&self) -> bool {
        self.active_scan.is_some_and(|scan| {
            self.catalog.scans.get(scan).is_some_and(|s| s.len > 0)
                && self
                    .operando
                    .as_ref()
                    .is_some_and(|data| data.scan == scan && data.scan_len > 0)
        })
    }

    fn choose_series_scan(&mut self, scan: usize, cx: &mut Context<Self>) {
        let Some(source) = self.catalog.scans.get(scan) else {
            return;
        };
        if crate::app::active_scan_indices(&self.group_registry, source.start, source.len, 1)
            .is_empty()
        {
            return;
        }
        if self.active_scan != Some(scan) {
            self.operando = None;
            self.operando_plots = None;
            self.time_pos = 0;
            self.pending_time_pos = None;
        }
        self.active_scan = Some(scan);
        self.ui.scan_picker = false;
        self.ensure_operando(cx);
        cx.notify();
    }

    fn series_scan_picker(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let t = self.theme;
        let mut list = div()
            .id("series-scan-list")
            .w_full()
            .max_w(px(560.))
            .max_h(px(400.))
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap_2();
        for (index, scan) in self.catalog.scans.iter().enumerate() {
            let available =
                !crate::app::active_scan_indices(&self.group_registry, scan.start, scan.len, 1)
                    .is_empty();
            let label = scan.label.clone();
            list = list.child(
                button(&t, ("select-series-scan", index), label, false)
                    .h(px(40.))
                    .w_full()
                    .gap_2()
                    .child(div().flex_1())
                    .child(
                        div()
                            .text_color(t.text_muted)
                            .child(format!("{} frames", scan.len)),
                    )
                    .when(!available, |d| {
                        d.disabled(true)
                            .opacity(0.45)
                            .tab_stop(false)
                            .cursor_default()
                    })
                    .when(available, |d| {
                        d.on_click(
                            cx.listener(move |app, _, _, cx| app.choose_series_scan(index, cx)),
                        )
                    }),
            );
        }
        div()
            .flex_1()
            .min_h_0()
            .min_w_0()
            .p_4()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_3()
            .child(div().text_size(px(16.)).child("Select scan"))
            .child(list)
            .child(
                button(&t, "series-import-scan", "Import…", false)
                    .on_click(cx.listener(|app, _, _, cx| app.open_folder(cx))),
            )
            .when(self.series_ready(), |d| {
                d.child(
                    button(&t, "series-cancel-picker", "Cancel", false).on_click(cx.listener(
                        |app, _, _, cx| {
                            app.ui.scan_picker = false;
                            cx.notify();
                        },
                    )),
                )
            })
    }

    /// "<scan> · N frames" · Refresh overview.
    pub(crate) fn series_inspector_header(
        &self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let t = self.theme;
        let label: SharedString = match self.active_scan.and_then(|ix| self.catalog.scans.get(ix)) {
            Some(scan) => format!("{} · {} frames", scan.label, scan.len).into(),
            None => "no scan selected".into(),
        };
        div()
            .flex_none()
            .px_3()
            .py_2()
            .flex()
            .items_center()
            .gap_2()
            .border_b_1()
            .border_color(t.border)
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .text_size(px(11.5))
                    .text_color(t.text)
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .child(label),
            )
            .child(
                icon_button(
                    &t,
                    "series-refresh",
                    Icon::Refresh,
                    "Refresh overview",
                    false,
                )
                .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                    this.operando = None;
                    this.ensure_operando(cx);
                    cx.notify();
                })),
            )
    }

    pub(crate) fn series_stage_center(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let t = self.theme;
        if self.ui.scan_picker {
            return self.series_scan_picker(cx).into_any_element();
        }
        let bar = self.series_plot_bar(cx);
        let Some((heatmap, chik, trend)) = self
            .operando_plots
            .as_ref()
            .filter(|_| self.series_ready())
            .map(|p| (p.heatmap.clone(), p.chik.clone(), p.trend.clone()))
        else {
            let running = self.operando_running;
            let has_scan = self.active_scan.is_some();
            let can_select = !self.catalog.scans.is_empty();
            return div()
                .flex_1()
                .min_h_0()
                .min_w_0()
                .flex()
                .flex_col()
                .when(has_scan, |d| d.child(bar))
                .child(
                    div()
                        .flex_1()
                        .flex()
                        .flex_col()
                        .items_center()
                        .justify_center()
                        .gap_3()
                        .when(running, |d| d.child("Building overview…"))
                        .when(!running, |d| {
                            d.child(
                                button(
                                    &t,
                                    "series-select-scan",
                                    if can_select {
                                        "Select scan"
                                    } else {
                                        "Import frames…"
                                    },
                                    true,
                                )
                                .on_click(cx.listener(
                                    move |app, _, _, cx| {
                                        if can_select {
                                            app.ui.scan_picker = true;
                                            cx.notify();
                                        } else {
                                            app.open_folder(cx);
                                        }
                                    },
                                )),
                            )
                        })
                        .when(has_scan && !running, |d| {
                            d.child(
                                div()
                                    .max_w(px(540.))
                                    .text_size(px(11.))
                                    .text_color(t.warn)
                                    .child(self.status.clone()),
                            )
                        }),
                )
                .into_any_element();
        };
        let frames = self.operando_scan_len().unwrap_or(0);
        let frame_label: SharedString = format!("frame {} / {frames}", self.time_pos + 1).into();
        let space_label: SharedString = match self.stage_view.series_space {
            SeriesSpace::Energy => "normalized μ(E)".into(),
            SeriesSpace::K => crate::plotting::chik_label(
                self.operando.as_ref().map(|d| d.kweight).unwrap_or(2.0),
            )
            .into(),
            SeriesSpace::R => "|χ(R)|".into(),
        };
        let trend_name: SharedString = self.trend_snapshot_name().into();
        let card = |title: SharedString| {
            div()
                .min_h_0()
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
        };
        div()
            .id("operando-center")
            .key_context("Operando")
            .track_focus(&self.operando_focus)
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(|this, _ev, window, cx| {
                    let handle = this.operando_focus.clone();
                    window.focus(&handle, cx);
                }),
            )
            .on_action(cx.listener(|this: &mut Self, _: &FramePrev, _w, cx| {
                this.step_time(-1, cx);
            }))
            .on_action(cx.listener(|this: &mut Self, _: &FrameNext, _w, cx| {
                this.step_time(1, cx);
            }))
            .on_action(cx.listener(|this: &mut Self, _: &FrameJumpBack, _w, cx| {
                this.step_time_percent(-1, cx);
            }))
            .on_action(cx.listener(|this: &mut Self, _: &FrameJumpFwd, _w, cx| {
                this.step_time_percent(1, cx);
            }))
            .on_action(cx.listener(|this: &mut Self, _: &FrameFirst, _w, cx| {
                this.set_time_pos(0, cx);
            }))
            .on_action(cx.listener(|this: &mut Self, _: &FrameLast, _w, cx| {
                let last = this
                    .operando_scan_len()
                    .map(|len| len.saturating_sub(1))
                    .unwrap_or(0);
                this.set_time_pos(last, cx);
            }))
            .flex_1()
            .min_h_0()
            .min_w_0()
            .flex()
            .flex_col()
            .child(bar)
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .min_w_0()
                    .flex()
                    .gap_2()
                    .px_3()
                    .pt_2()
                    .pb_3()
                    .child(
                        card(format!("{space_label} · heatmap").into())
                            .flex_1()
                            .child(div().flex_1().min_h_0().min_w_0().p_1().child(heatmap))
                            .child(self.time_scrubber(cx))
                            .child(
                                div()
                                    .px_3()
                                    .pb_1()
                                    .font_family(MONO)
                                    .text_size(px(10.5))
                                    .text_color(t.text_muted)
                                    .child(frame_label),
                            ),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_h_0()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(
                                card(format!("frame {} · {space_label}", self.time_pos + 1).into())
                                    .flex_1()
                                    .child(div().flex_1().min_h_0().min_w_0().p_1().child(chik)),
                            )
                            .child(
                                card(format!("trend · {trend_name}").into())
                                    .flex_1()
                                    .child(div().flex_1().min_h_0().min_w_0().p_1().child(trend)),
                            ),
                    ),
            )
            .into_any_element()
    }

    fn series_plot_bar(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let t = self.theme;
        let space = self.stage_view.series_space;
        let kw = self.operando.as_ref().map(|d| d.kweight).unwrap_or(2.0);
        let options: [(SeriesSpace, String); 3] = [
            (SeriesSpace::Energy, "norm μ(E)".into()),
            (SeriesSpace::K, crate::plotting::chik_label(kw)),
            (SeriesSpace::R, "|χ(R)|".into()),
        ];
        let mut seg = segmented(&t);
        for (i, (sp, label)) in options.into_iter().enumerate() {
            seg = seg.child(
                segment(
                    &t,
                    SharedString::from(format!("series-space-{i}")),
                    label,
                    space == sp,
                    i == 0,
                )
                .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                    this.stage_view.series_space = sp;
                    this.rebuild_operando_plots(cx);
                    cx.notify();
                })),
            );
        }
        div()
            .min_h(px(36.))
            .w_full()
            .min_w_0()
            .flex_none()
            .flex()
            .items_center()
            .gap_2()
            .px_3()
            .bg(t.surface)
            .border_b_1()
            .border_color(t.border)
            .flex_wrap()
            .child(
                button(&t, "series-change-scan", self.active_scan
                    .and_then(|ix| self.catalog.scans.get(ix)).map(|s| s.label.clone())
                    .unwrap_or_else(|| "Select scan".into()), false)
                    .max_w(px(180.)).overflow_hidden().text_ellipsis()
                    .child(icon(&t, Icon::ChevronDown))
                    .on_click(cx.listener(|app, _, _, cx| { app.ui.scan_picker = true; cx.notify(); })),
            )
            .child(seg)
            .child(div().w(px(1.)).h(px(18.)).bg(t.border))
            .child(div().flex_1())
            .child(
                chip(
                    &t,
                    "series-preview",
                    if self.batch_preview { "Sampled frames" } else { "All frames" },
                    self.batch_preview,
                )
                .on_click(
                    cx.listener(|this, _: &ClickEvent, _w, cx| this.toggle_batch_preview(cx)),
                ).tooltip(move |_, cx| cx.new(|_| Tooltip { theme: t, label: "Calculation scope. The overview is always sampled; All frames calculates every surviving frame.".into() }).into()),
            )
            .child(icon_button(&t, "series-shortcuts", Icon::Help, "Click a heatmap row to jump. ←/→: frame; Shift+←/→: 1%; Home/End: first/last.", false))
    }

    fn time_scrubber(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        const SEGMENTS: usize = 96;
        let t = self.theme;
        let frames = self.operando_scan_len().unwrap_or(0).max(1);
        let active_seg = if frames > 1 {
            self.time_pos * (SEGMENTS - 1) / (frames - 1)
        } else {
            0
        };
        let mut strip = div().flex().w_full().h(px(14.)).gap(px(1.)).px_3();
        for seg in 0..SEGMENTS {
            let frame = if SEGMENTS > 1 {
                seg * (frames - 1) / (SEGMENTS - 1)
            } else {
                0
            };
            strip = strip.child(
                div()
                    .id(("scrub", seg))
                    .flex_1()
                    .h_full()
                    .rounded_xs()
                    .bg(if seg == active_seg {
                        t.accent
                    } else {
                        t.border
                    })
                    .hover(|d| d.bg(t.text_muted))
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                        this.set_time_pos(frame, cx);
                    })),
            );
        }
        strip
    }

    fn trend_snapshot_name(&self) -> String {
        match &self.series_trend {
            TrendSource::E0 => "E₀ (eV)".into(),
            TrendSource::WhiteLine => "white line".into(),
            TrendSource::FitVar(name) => format!("fit · {name}"),
            TrendSource::Lcf(i) => self
                .series_lcf
                .as_ref()
                .and_then(|l| l.names.get(*i))
                .map(|n| format!("LCF · {n}"))
                .unwrap_or_else(|| "LCF".into()),
        }
    }

    // ---- inspector ----------------------------------------------------------

    pub(crate) fn series_inspector(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let t = self.theme;
        div()
            .flex()
            .flex_col()
            .child(self.series_cursor_section(cx))
            .child(self.series_trends_section(cx))
            .child(self.series_lcf_section(cx))
            .child(self.series_batch_section(cx))
            .child(div().h(px(12.)).bg(t.surface))
    }

    fn series_cursor_section(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let t = self.theme;
        let frames = self.operando_scan_len().unwrap_or(0);
        let pos = self.time_pos;
        let data = self.operando.as_ref();
        let sample = data.map(|d| crate::app::nearest_sample_pos(pos, d.scan_len, d.e0s.len()));
        let e0 = self
            .spectrum
            .as_ref()
            .and_then(|s| s.e0())
            .or_else(|| data.zip(sample).and_then(|(d, i)| d.e0s.get(i).copied()));
        let whiteline = data
            .zip(sample)
            .and_then(|(d, i)| d.whitelines.get(i).copied());
        let fmt = |v: Option<f64>, d: usize, unit: &str| {
            v.filter(|v| v.is_finite())
                .map(|v| format!("{v:.d$}{unit}"))
                .unwrap_or("—".into())
        };
        let mut rows: Vec<gpui::AnyElement> = Vec::new();
        if frames > 0 {
            let frac = if frames > 1 {
                pos as f32 / (frames - 1) as f32
            } else {
                0.0
            };
            rows.push(
                div()
                    .mx_3()
                    .mt_1()
                    .h(px(4.))
                    .rounded_full()
                    .bg(t.border)
                    .child(div().h_full().w(relative(frac)).rounded_full().bg(t.accent))
                    .into_any_element(),
            );
        }
        rows.push(
            self.result_card(vec![
                ("Frame".into(), format!("{} / {frames}", pos + 1)),
                ("File".into(), self.current_group_label().to_string()),
                ("E₀".into(), fmt(e0, 1, " eV")),
                ("White line".into(), fmt(whiteline, 3, "")),
            ])
            .into_any_element(),
        );
        rows.push(
            div()
                .px_3()
                .pt_1()
                .flex()
                .gap_1()
                .child(
                    button(&t, "open-frame", "Open frame in Normalize", false).on_click(
                        cx.listener(|this, _: &ClickEvent, _w, cx| {
                            this.open_frame_in_normalize(cx)
                        }),
                    ),
                )
                .into_any_element(),
        );
        self.section("Cursor", None, rows, cx)
    }

    fn series_trends_section(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let t = self.theme;
        let mut entries: Vec<(TrendSource, String, String, bool)> = vec![
            (TrendSource::E0, "E₀ shift".into(), "normalize".into(), true),
            (
                TrendSource::WhiteLine,
                "white line".into(),
                "normalize".into(),
                true,
            ),
        ];
        let batch_ok = self.active_batch_trend().is_some();
        if let Some(bf) = &self.batch_fit {
            for name in &bf.varying_names {
                entries.push((
                    TrendSource::FitVar(name.clone()),
                    name.clone(),
                    if batch_ok {
                        "batch fit".into()
                    } else {
                        "batch fit · stale".into()
                    },
                    batch_ok,
                ));
            }
        }
        let lcf_ok = self.active_series_lcf().is_some();
        if let Some(lcf) = &self.series_lcf {
            for (i, name) in lcf.names.iter().enumerate() {
                entries.push((
                    TrendSource::Lcf(i),
                    format!("LCF · {name}"),
                    if lcf_ok {
                        "LCF trend".into()
                    } else {
                        "LCF · stale".into()
                    },
                    lcf_ok,
                ));
            }
        }
        let mut rows: Vec<gpui::AnyElement> = Vec::new();
        for (i, (source, name, origin, ready)) in entries.into_iter().enumerate() {
            let on = self.series_trend == source;
            rows.push(
                div()
                    .id(("trend-row", i))
                    .mx_2()
                    .h(px(24.))
                    .px_1p5()
                    .flex()
                    .items_center()
                    .gap_2()
                    .rounded_md()
                    .cursor_pointer()
                    .when(on, |d| {
                        d.bg(gpui::Rgba {
                            a: 0.14,
                            ..t.accent
                        })
                    })
                    .hover(|d| d.bg(t.raised))
                    .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                        this.series_trend = source.clone();
                        this.rebuild_operando_trend(cx);
                        cx.notify();
                    }))
                    .child(div().w(px(8.)).h(px(8.)).rounded_full().bg(if on {
                        t.accent
                    } else {
                        t.border
                    }))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .text_ellipsis()
                            .text_color(if ready { t.text } else { t.text_muted })
                            .child(name),
                    )
                    .child(
                        div()
                            .text_size(px(10.5))
                            .text_color(if ready { t.text_muted } else { t.warn })
                            .child(origin),
                    )
                    .into_any_element(),
            );
        }
        let head = div()
            .px_3()
            .pt_3()
            .pb_1()
            .flex()
            .items_center()
            .gap_2()
            .child(section_label(&t, "Trends"))
            .child(div().flex_1())
            .child(
                div()
                    .text_size(px(10.5))
                    .text_color(t.text_muted)
                    .child("click to plot"),
            );
        div()
            .flex()
            .flex_col()
            .border_b_1()
            .border_color(t.border)
            .pb_2()
            .child(head)
            .children(rows)
    }

    fn series_lcf_section(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let t = self.theme;
        let standards = self.lcf_standards();
        let mut rows: Vec<gpui::AnyElement> = Vec::new();
        let names: String = if standards.is_empty() {
            "mark the standards in the Groups panel".into()
        } else {
            standards
                .iter()
                .map(|(n, _)| n.as_str())
                .collect::<Vec<_>>()
                .join(" · ")
        };
        rows.push(
            div()
                .px_3()
                .py_0p5()
                .text_size(px(11.))
                .text_color(t.text_muted)
                .overflow_hidden()
                .whitespace_nowrap()
                .text_ellipsis()
                .child(format!("standards: {names}"))
                .into_any_element(),
        );
        let label: SharedString = if self.lcf_running {
            let (done, total) = self.lcf_progress;
            format!("Cancel ({done}/{total})").into()
        } else {
            "Run LCF trend".into()
        };
        rows.push(
            div()
                .px_3()
                .py_1()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .flex_1()
                        .text_size(px(11.))
                        .text_color(t.text_muted)
                        .child(format!(
                            "space {} · sum to one",
                            self.tools.lcf_space_label()
                        )),
                )
                .child(
                    button(
                        &t,
                        "lcf-run",
                        label,
                        !self.lcf_running && standards.len() >= 2,
                    )
                    .when(self.lcf_running, |d| {
                        d.border_color(t.error).text_color(t.error)
                    })
                    .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                        if this.lcf_running {
                            this.cancel_series_lcf(cx);
                        } else {
                            this.run_series_lcf(cx);
                        }
                    })),
                )
                .into_any_element(),
        );
        if self.lcf_running {
            let (done, total) = self.lcf_progress;
            let frac = if total > 0 {
                done as f32 / total as f32
            } else {
                0.0
            };
            rows.push(
                div()
                    .mx_3()
                    .h(px(4.))
                    .rounded_full()
                    .bg(t.border)
                    .child(div().h_full().w(relative(frac)).rounded_full().bg(t.accent))
                    .into_any_element(),
            );
        }
        if let Some(lcf) = &self.series_lcf {
            rows.push(
                div()
                    .px_3()
                    .py_0p5()
                    .text_size(px(11.))
                    .text_color(t.text_muted)
                    .child(format!(
                        "{} / {} frames{}",
                        lcf.rows.len(),
                        lcf.total,
                        if lcf.cancelled { " · cancelled" } else { "" }
                    ))
                    .into_any_element(),
            );
        }
        self.section("LCF trend", None, rows, cx)
    }

    fn series_batch_section(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let t = self.theme;
        let mut rows: Vec<gpui::AnyElement> = Vec::new();
        rows.push(
            div()
                .px_3()
                .py_0p5()
                .text_size(px(11.))
                .text_color(t.text_muted)
                .child(self.batch_scope_line())
                .into_any_element(),
        );
        let can = self.fit_paths.iter().any(|p| p.spec.enabled);
        let label: SharedString = if self.batch_running {
            let (done, total) = self.batch_progress;
            format!("Cancel ({done}/{total})").into()
        } else {
            "Run batch fit".into()
        };
        rows.push(
            div()
                .px_3()
                .py_1()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .flex_1()
                        .text_size(px(11.))
                        .text_color(t.text_muted)
                        .child(if can {
                            format!(
                                "{} paths · model from the Fit stage",
                                self.fit_paths.iter().filter(|p| p.spec.enabled).count()
                            )
                        } else {
                            "set up a model in the Fit stage first".into()
                        }),
                )
                .child(
                    button(&t, "series-batch", label, !self.batch_running && can)
                        .when(self.batch_running, |d| {
                            d.border_color(t.error).text_color(t.error)
                        })
                        .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                            if this.batch_running {
                                this.cancel_batch_fit(cx);
                            } else if can {
                                this.run_batch_fit(cx);
                            }
                        })),
                )
                .into_any_element(),
        );
        if self.batch_running {
            let (done, total) = self.batch_progress;
            let frac = if total > 0 {
                done as f32 / total as f32
            } else {
                0.0
            };
            rows.push(
                div()
                    .mx_3()
                    .h(px(4.))
                    .rounded_full()
                    .bg(t.border)
                    .child(div().h_full().w(relative(frac)).rounded_full().bg(t.accent))
                    .into_any_element(),
            );
        }
        if let Some(bf) = &self.batch_fit {
            rows.push(
                div()
                    .px_3()
                    .py_0p5()
                    .flex()
                    .items_center()
                    .gap_2()
                    .text_size(px(11.))
                    .text_color(t.text_muted)
                    .child(format!(
                        "{} / {} fitted · {} problems{}",
                        bf.rows.len(),
                        bf.total,
                        bf.problems.len(),
                        if bf.cancelled { " · cancelled" } else { "" }
                    ))
                    .child(div().flex_1())
                    .child(
                        div()
                            .id("series-show-fit")
                            .text_color(t.accent)
                            .cursor_pointer()
                            .hover(|d| d.text_color(t.text))
                            .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                                this.stage_view.fit_show_batch = true;
                                this.set_stage(super::Stage::Fit, cx);
                            }))
                            .child("results table →"),
                    )
                    .into_any_element(),
            );
        }
        self.section("Batch fit", None, rows, cx)
    }
}
