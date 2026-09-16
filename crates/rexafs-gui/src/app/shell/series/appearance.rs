//! Heatmap controls keep fixed positions; their menus overlay the workspace.
use super::*;
use crate::app::series_display::HeatmapPalette;
use crate::widgets::numeric_field::{FieldKind, NumericField};
use gpui::{AppContext, Window};

impl StudioApp {
    fn close_series_appearance(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.series_display.reference_open = false;
        self.series_display.colors_open = false;
        self.operando_focus.focus(window, cx);
        cx.notify();
    }

    fn open_series_appearance(
        &mut self,
        reference: bool,
        event: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.series_display.reference_open = reference;
        self.series_display.colors_open = !reference;
        self.series_display.menu_position = event.position();
        self.series_display
            .menu_focus
            .get_or_insert_with(|| cx.focus_handle())
            .focus(window, cx);
        if reference {
            let frame = self
                .active_series_reference()
                .map_or(self.time_pos, |r| r.frame);
            let t = self.theme;
            self.series_display.reference_field = Some(cx.new(|cx| {
                NumericField::new(
                    "Frame",
                    "required",
                    Some((frame + 1) as f64),
                    FieldKind::Integer { min: Some(1) },
                    t,
                    cx,
                )
            }));
        }
        cx.notify();
    }

    pub(super) fn series_appearance_buttons(
        &self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let t = self.theme;
        let reference = self
            .active_series_reference()
            .map(|r| r.frame)
            .or(self.series_display.pending_frame);
        div()
            .flex()
            .flex_none()
            .items_center()
            .gap_1()
            .child(
                button(
                    &t,
                    "series-difference",
                    "Difference",
                    self.series_display.difference,
                )
                .w(px(86.))
                .justify_center()
                .on_click(cx.listener(|app, _, _, cx| {
                    if app.series_display.difference {
                        app.clear_series_difference(cx);
                    } else {
                        app.set_series_reference(app.time_pos, cx);
                    }
                })),
            )
            .child(
                button(
                    &t,
                    "series-reference",
                    reference.map_or("Reference…".into(), |i| format!("Ref: {} ▾", i + 1)),
                    self.series_display.reference_open,
                )
                .w(px(106.))
                .justify_center()
                .on_click(cx.listener(|app, event, window, cx| {
                    app.open_series_appearance(true, event, window, cx);
                })),
            )
            .child(
                button(
                    &t,
                    "series-heatmap-colors",
                    "Colors ▾",
                    self.series_display.colors_open,
                )
                .w(px(76.))
                .justify_center()
                .on_click(cx.listener(|app, event, window, cx| {
                    app.open_series_appearance(false, event, window, cx);
                })),
            )
    }

    pub(crate) fn series_appearance_overlay(
        &self,
        cx: &mut Context<Self>,
    ) -> Option<gpui::AnyElement> {
        if self.stage != super::super::Stage::Series
            || !self.measurements.overview
            || (!self.series_display.colors_open && !self.series_display.reference_open)
        {
            return None;
        }
        let focus = self.series_display.menu_focus.as_ref()?;
        let t = self.theme;
        let mut menu = div()
            .id("series-appearance-popup")
            .w(px(if self.series_display.reference_open {
                260.
            } else {
                218.
            }))
            .p_1()
            .flex()
            .flex_col()
            .gap_1()
            .rounded_lg()
            .bg(t.surface)
            .border_1()
            .border_color(t.border)
            .shadow_lg()
            .on_any_mouse_down(|_, _, cx| cx.stop_propagation());
        if self.series_display.reference_open {
            let field = self.series_display.reference_field.as_ref()?;
            menu = menu
                .child(
                    div()
                        .px_2()
                        .py_1()
                        .text_color(t.text_muted)
                        .child("Subtract reference frame"),
                )
                .child(field.clone())
                .child(
                    button(&t, "series-set-reference", "Set reference", true)
                        .w_full()
                        .on_click(cx.listener(|app, _, window, cx| {
                            let value = app
                                .series_display
                                .reference_field
                                .as_ref()
                                .and_then(|f| f.read(cx).pending_value(cx).ok().flatten());
                            let count = app.operando_scan_len().unwrap_or(0);
                            match value {
                                Some(value) if value >= 1. && value <= count as f64 => {
                                    app.close_series_appearance(window, cx);
                                    app.set_series_reference(value as usize - 1, cx);
                                }
                                _ => {
                                    app.status =
                                        format!("Choose a reference frame from 1 to {count}.")
                                            .into();
                                    cx.notify();
                                }
                            }
                        })),
                )
                .child(
                    button(
                        &t,
                        "series-current-reference",
                        format!("Use current ({})", self.time_pos + 1),
                        false,
                    )
                    .w_full()
                    .on_click(cx.listener(|app, _, window, cx| {
                        app.close_series_appearance(window, cx);
                        app.set_series_reference(app.time_pos, cx);
                    })),
                );
        } else {
            menu = menu.child(
                button(
                    &t,
                    "heatmap-auto-colors",
                    "Auto",
                    self.series_display.palette.is_none(),
                )
                .w_full()
                .h(px(30.))
                .on_click(cx.listener(|app, _, window, cx| {
                    app.series_display.palette = None;
                    app.series_display.reversed = false;
                    app.close_series_appearance(window, cx);
                    app.rebuild_operando_plots(cx);
                })),
            );
            for (index, palette) in HeatmapPalette::ALL.into_iter().enumerate() {
                let map = palette.map(self.series_display.reversed);
                let mut swatch = div().flex().w(px(62.)).h(px(10.));
                for i in 0..24 {
                    let color = map.sample(i as f64 / 23.);
                    swatch = swatch.child(div().flex_1().h_full().bg(gpui::Rgba {
                        r: color.r as f32 / 255.,
                        g: color.g as f32 / 255.,
                        b: color.b as f32 / 255.,
                        a: 1.,
                    }));
                }
                menu = menu.child(
                    button(
                        &t,
                        ("heatmap-palette", index),
                        palette.label(),
                        self.series_display.palette == Some(palette),
                    )
                    .w_full()
                    .h(px(30.))
                    .child(div().flex_1())
                    .child(swatch)
                    .on_click(cx.listener(move |app, _, window, cx| {
                        app.series_display.palette = Some(palette);
                        app.close_series_appearance(window, cx);
                        app.rebuild_operando_plots(cx);
                    })),
                );
            }
            menu = menu.child(div().h(px(1.)).my_1().bg(t.border)).child(
                button(
                    &t,
                    "heatmap-reverse",
                    "Reverse",
                    self.series_display.reversed,
                )
                .w_full()
                .h(px(30.))
                .on_click(cx.listener(|app, _, _, cx| {
                    app.series_display.reversed = !app.series_display.reversed;
                    app.rebuild_operando_plots(cx);
                    cx.notify();
                })),
            );
        }
        Some(
            div()
                .id("series-appearance-dismiss")
                .absolute()
                .inset_0()
                .occlude()
                .track_focus(focus)
                .on_key_down(cx.listener(|app, event: &gpui::KeyDownEvent, window, cx| {
                    if event.keystroke.key == "escape" {
                        app.close_series_appearance(window, cx);
                        cx.stop_propagation();
                    }
                }))
                .on_any_mouse_down(cx.listener(|app, _, window, cx| {
                    cx.stop_propagation();
                    app.close_series_appearance(window, cx);
                }))
                .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
                .child(
                    gpui::anchored()
                        .position(self.series_display.menu_position)
                        .offset(gpui::point(px(0.), px(15.)))
                        .snap_to_window()
                        .child(menu),
                )
                .into_any_element(),
        )
    }
}
