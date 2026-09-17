//! Compact wavelet controls; display choices never change retained numerical maps.
use super::*;
use crate::app::series_display::HeatmapPalette;
impl StudioApp {
    pub(crate) fn wavelet_center(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let t = self.theme;
        let header = div()
            .flex()
            .items_center()
            .gap_2()
            .child(
                button(&t, "wavelet-back", "← Data", false).on_click(cx.listener(
                    |app, _, _, cx| {
                        app.wavelet.cancel();
                        app.wavelet.open = false;
                        cx.notify();
                    },
                )),
            )
            .child(div().flex_1().text_size(px(16.)).child("Wavelet"))
            .child(
                button(&t, "wavelet-history", "History ▾", false).on_click(cx.listener(
                    |app, event: &gpui::ClickEvent, window, cx| {
                        app.wavelet.history_open = !app.wavelet.history_open;
                        app.wavelet.colors_open = false;
                        app.wavelet.menu_position = event.position();
                        let focus = cx.focus_handle();
                        focus.focus(window, cx);
                        app.wavelet.menu_focus = Some(focus);
                        cx.notify();
                    },
                )),
            )
            .child(
                button(&t, "wavelet-export", "Export map…", false)
                    .opacity(if self.wavelet.record.is_some() {
                        1.
                    } else {
                        0.4
                    })
                    .on_click(cx.listener(|app, _, _, cx| app.export_wavelet(cx))),
            );
        let mut views = div().flex().items_center().gap_1();
        for (i, label) in ["Magnitude", "Real", "Imaginary", "Phase"]
            .into_iter()
            .enumerate()
        {
            views = views.child(
                button(
                    &t,
                    format!("wavelet-view-{i}"),
                    label,
                    self.wavelet.view == i,
                )
                .w(px(84.))
                .justify_center()
                .on_click(cx.listener(move |app, _, _, cx| {
                    app.wavelet.view = i;
                    app.wavelet.lock_scale = None;
                    app.wavelet.lock_basis = None;
                    app.rebuild_wavelet_plots(cx);
                    cx.notify();
                })),
            );
        }
        views = views
            .child(
                button(&t, "wavelet-colors", "Colors ▾", self.wavelet.colors_open)
                    .w(px(78.))
                    .justify_center()
                    .on_click(cx.listener(|app, event: &gpui::ClickEvent, window, cx| {
                        app.wavelet.colors_open = !app.wavelet.colors_open;
                        app.wavelet.history_open = false;
                        app.wavelet.menu_position = event.position();
                        let focus = cx.focus_handle();
                        focus.focus(window, cx);
                        app.wavelet.menu_focus = Some(focus);
                        cx.notify();
                    })),
            )
            .child(
                button(
                    &t,
                    "wavelet-lock",
                    "Lock scale",
                    self.wavelet.lock_scale.is_some(),
                )
                .w(px(90.))
                .justify_center()
                .on_click(cx.listener(|app, _, _, cx| {
                    app.wavelet.lock_scale = if app.wavelet.lock_scale.is_some() {
                        None
                    } else {
                        app.wavelet
                            .record
                            .as_ref()
                            .map(|r| plots::scale(&r.map, app.wavelet.view))
                    };
                    app.wavelet.lock_basis = if app.wavelet.lock_scale.is_some() {
                        app.wavelet
                            .record
                            .as_ref()
                            .map(|r| (r.settings.clone(), r.map.settings().clone()))
                    } else {
                        None
                    };
                    app.rebuild_wavelet_plots(cx);
                    cx.notify();
                })),
            );
        let mut body = div()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .p_3()
            .flex()
            .flex_col()
            .gap_2()
            .child(header)
            .child(views)
            .child(
                div()
                    .id("wavelet-status")
                    .tooltip({
                        let label: SharedString = self.wavelet.message.clone().into();
                        move |_, cx| {
                            cx.new(|_| super::super::controls::Tooltip {
                                label: label.clone(),
                                theme: t,
                            })
                            .into()
                        }
                    })
                    .h(px(20.))
                    .overflow_hidden()
                    .text_size(px(12.))
                    .text_color(t.text_muted)
                    .child(self.wavelet.message.clone()),
            );
        body = body.child(
            div()
                .flex_1()
                .min_h(px(190.))
                .min_w_0()
                .relative()
                .children(self.wavelet.plot_map.clone())
                .children(self.wavelet_map_overlay()),
        );
        let mut linked = div().flex().items_center().gap_1();
        for (slices, label) in [(false, "Spectra"), (true, "Slices")] {
            linked = linked.child(
                button(
                    &t,
                    format!("wavelet-linked-{slices}"),
                    label,
                    self.wavelet.slices == slices,
                )
                .w(px(78.))
                .justify_center()
                .on_click(cx.listener(move |app, _, _, cx| {
                    if app.wavelet.slices != slices {
                        app.wavelet.plot_k = None;
                        app.wavelet.plot_r = None;
                    }
                    app.wavelet.slices = slices;
                    app.rebuild_wavelet_slices(cx);
                    cx.notify();
                })),
            );
        }
        let caption = if self.wavelet.slices {
            format!(
                "R = {:.2} Å · k = {:.2} Å⁻¹ · |W| = {:.4}",
                self.wavelet.cursor[1],
                self.wavelet.cursor[0],
                self.wavelet_cursor_value().unwrap_or(0.)
            )
        } else {
            self.wavelet
                .record
                .as_ref()
                .and_then(|r| r.fourier.as_ref())
                .map(|f| {
                    format!(
                        "Original χ · Fourier {} window",
                        f.window
                            .as_ref()
                            .map(|w| format!("{w:?}"))
                            .unwrap_or_else(|| "default".into())
                    )
                })
                .unwrap_or_else(|| "Click the map to inspect slices".into())
        };
        linked = linked.child(
            div()
                .text_size(px(12.))
                .text_color(t.text_muted)
                .child(caption),
        );
        body = body.child(linked).child(
            div()
                .flex()
                .gap_2()
                .h(px(245.))
                .min_h(px(245.))
                .child(self.wavelet_plot_card(PLOT_WAVELET_K, cx))
                .child(self.wavelet_plot_card(PLOT_WAVELET_R, cx)),
        );
        body.into_any_element()
    }
    fn wavelet_plot_card(&mut self, plot: usize, cx: &mut Context<Self>) -> gpui::AnyElement {
        div()
            .id(format!("wavelet-linked-plot-{plot}"))
            .relative()
            .flex_1()
            .min_w_0()
            .h_full()
            .on_mouse_move(
                cx.listener(move |app, event: &gpui::MouseMoveEvent, _, cx| {
                    app.plot_pointer_move(plot, event.position, cx)
                }),
            )
            .capture_any_mouse_down(
                cx.listener(move |app, event: &gpui::MouseDownEvent, _, cx| {
                    app.capture_handle_press(plot, event, cx)
                }),
            )
            .children(self.plot_entity(plot))
            .children(self.handle_layer(plot, cx))
            .children(self.handle_overlay(plot, cx))
            .into_any_element()
    }
    pub(crate) fn wavelet_inspector(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let t = self.theme;
        let mut body = div()
            .p_3()
            .flex()
            .flex_col()
            .gap_2()
            .child(section_label(&t, "WAVELET SETTINGS"));
        for (i, label) in [
            (0, "k from (Å⁻¹)"),
            (1, "k to (Å⁻¹)"),
            (2, "k weight"),
            (3, "R maximum (Å)"),
        ] {
            if let Some(field) = self.wavelet.fields.get(i) {
                body = body.child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(div().w(px(124.)).text_size(px(12.)).child(label))
                        .child(div().flex_1().min_w_0().child(field.clone())),
                );
            }
        }
        body = body
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        button(&t, "wavelet-calculate", "Calculate", true)
                            .on_click(cx.listener(|app, _, _, cx| app.calculate_wavelet(cx))),
                    )
                    .child(
                        button(&t, "wavelet-cancel", "Cancel", false)
                            .opacity(if self.wavelet.busy { 1. } else { 0.4 })
                            .on_click(cx.listener(|app, _, _, cx| {
                                if app.wavelet.busy {
                                    app.wavelet.cancel();
                                    app.wavelet.message = "Cancelled".into();
                                    cx.notify();
                                }
                            })),
                    ),
            )
            .child(
                button(&t, "wavelet-advanced", "Advanced ▾", self.wavelet.advanced).on_click(
                    cx.listener(|app, _, _, cx| {
                        app.wavelet.advanced = !app.wavelet.advanced;
                        cx.notify();
                    }),
                ),
            );
        if self.wavelet.advanced {
            for (i, label) in [
                (4, "Cauchy order"),
                (5, "k step (Å⁻¹)"),
                (6, "R step (Å)"),
                (7, "Taper (Å⁻¹)"),
            ] {
                if let Some(field) = self.wavelet.fields.get(i) {
                    body = body.child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(div().w(px(124.)).text_size(px(12.)).child(label))
                            .child(div().flex_1().min_w_0().child(field.clone())),
                    );
                }
            }
            body=body.child(div().text_size(px(11.)).text_color(t.text_muted).child("R step: blank = auto. Taper: 0 = none. Larger order narrows frequency response and broadens localization in k."));
        }
        if self.wavelet.record.is_some() {
            body = body.child(section_label(&t, "REGION INTEGRAL")).child(
                div()
                    .text_size(px(12.))
                    .text_color(t.text_muted)
                    .child("Drag the bounds in the k and R plots."),
            );
            for (i, label) in ["k from (Å⁻¹)", "k to (Å⁻¹)", "R from (Å)", "R to (Å)"]
                .into_iter()
                .enumerate()
            {
                if let Some(field) = self.wavelet.region_fields.get(i) {
                    body = body.child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(div().w(px(124.)).text_size(px(12.)).child(label))
                            .child(div().flex_1().min_w_0().child(field.clone())),
                    );
                }
            }
            body = body
                .child(
                    div()
                        .text_size(px(12.))
                        .child(self.wavelet.region_message.clone()),
                )
                .child(
                    button(&t, "wavelet-save-region", "Save region", false)
                        .opacity(if self.wavelet.region_value.is_some() {
                            1.
                        } else {
                            0.4
                        })
                        .on_click(cx.listener(|app, _, _, cx| app.save_wavelet_region(cx))),
                );
        }
        if let Some(record) = &self.wavelet.record {
            body = body.child(
                div()
                    .text_size(px(11.))
                    .text_color(t.text_muted)
                    .child("R is not phase-corrected. Color changes affect only the display."),
            );
            if self.wavelet.view == 3 {
                body = body.child(
                    div()
                        .text_size(px(11.))
                        .text_color(t.text_muted)
                        .child("Phase: radians; values below 1% of peak magnitude are hidden."),
                );
            }
            for warning in record
                .map
                .warnings()
                .iter()
                .chain(record.fourier_error.iter())
            {
                if self.wavelet.advanced || record.fourier_error.as_ref() == Some(warning) {
                    body = body.child(
                        div()
                            .text_size(px(11.))
                            .text_color(t.warn)
                            .child(warning.clone()),
                    );
                }
            }
            let count = self
                .wavelet
                .archive
                .regions
                .iter()
                .filter(|r| {
                    self.wavelet
                        .receipt
                        .as_ref()
                        .is_some_and(|p| p.digest == r.map_digest)
                })
                .count();
            body = body.child(
                div()
                    .text_size(px(11.))
                    .text_color(t.text_muted)
                    .child(format!("{count} saved regions · full map retained")),
            );
        }
        for (i, region) in self
            .wavelet
            .archive
            .regions
            .iter()
            .enumerate()
            .filter(|(_, r)| {
                self.wavelet
                    .receipt
                    .as_ref()
                    .is_some_and(|p| p.digest == r.map_digest)
            })
        {
            let bounds = [
                region.measurement.k_range[0],
                region.measurement.k_range[1],
                region.measurement.r_range[0],
                region.measurement.r_range[1],
            ];
            body = body.child(
                button(
                    &t,
                    format!("wavelet-saved-region-{i}"),
                    format!(
                        "Region {} · k {:.2}–{:.2}, R {:.2}–{:.2}",
                        i + 1,
                        bounds[0],
                        bounds[1],
                        bounds[2],
                        bounds[3]
                    ),
                    false,
                )
                .on_click(cx.listener(move |app, _, _, cx| {
                    for (f, value) in app.wavelet.region_fields.iter().zip(bounds) {
                        f.update(cx, |f, cx| f.set_text(value.to_string(), cx));
                    }
                    app.read_wavelet_region(cx);
                })),
            );
        }
        body.into_any_element()
    }
    pub(crate) fn wavelet_appearance_overlay(
        &self,
        cx: &mut Context<Self>,
    ) -> Option<gpui::AnyElement> {
        if (!self.wavelet.colors_open && !self.wavelet.history_open)
            || !self.wavelet.open
            || self.stage != Stage::Data
        {
            return None;
        }
        let focus = self.wavelet.menu_focus.as_ref()?;
        let t = self.theme;
        let mut menu = div()
            .id("wavelet-color-menu")
            .w(px(if self.wavelet.history_open {
                340.
            } else {
                190.
            }))
            .max_h(px(430.))
            .overflow_y_scroll()
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
        if self.wavelet.history_open {
            menu = menu.child(
                div()
                    .p_2()
                    .text_size(px(12.))
                    .child("Saved maps · current spectrum and processing"),
            );
            for (i, receipt) in self
                .wavelet
                .archive
                .entries
                .iter()
                .enumerate()
                .rev()
                .filter(|(_, r)| {
                    Some(&r.group) == self.wavelet.group.as_ref() && r.settings == *self.ui_params()
                })
            {
                let receipt = receipt.clone();
                let label = format!(
                    "{} · k {:.1}–{:.1} · R ≤ {:.1} · weight {} · order {}",
                    i + 1,
                    receipt.definition.k_range[0],
                    receipt.definition.k_range[1],
                    receipt
                        .definition
                        .radii
                        .as_ref()
                        .and_then(|r| r.last())
                        .copied()
                        .unwrap_or(receipt.definition.rmax),
                    receipt.definition.kweight,
                    receipt.definition.order
                );
                menu = menu.child(
                    button(&t, format!("wavelet-history-{i}"), label, false)
                        .w_full()
                        .on_click(cx.listener(move |app, _, _, cx| {
                            app.wavelet.history_open = false;
                            app.wavelet.cancel();
                            app.set_wavelet_fields(&receipt.definition, cx);
                            app.wavelet.plot_map = None;
                            app.wavelet.plot_k = None;
                            app.wavelet.plot_r = None;
                            app.load_wavelet(receipt.clone(), cx);
                            cx.notify();
                        })),
                );
            }
        } else {
            menu = menu.child(
                button(
                    &t,
                    "wavelet-color-auto",
                    "Auto",
                    self.wavelet.palette.is_none(),
                )
                .w_full()
                .on_click(cx.listener(|app, _, _, cx| {
                    app.wavelet.palette = None;
                    app.wavelet.colors_open = false;
                    app.rebuild_wavelet_plots(cx);
                    cx.notify();
                })),
            );
            for palette in HeatmapPalette::ALL {
                menu = menu.child(
                    button(
                        &t,
                        format!("wavelet-color-{}", palette.label()),
                        palette.label(),
                        self.wavelet.palette == Some(palette),
                    )
                    .w_full()
                    .on_click(cx.listener(move |app, _, _, cx| {
                        app.wavelet.palette = Some(palette);
                        app.wavelet.colors_open = false;
                        app.rebuild_wavelet_plots(cx);
                        cx.notify();
                    })),
                );
            }
            menu = menu.child(
                button(
                    &t,
                    "wavelet-color-reverse",
                    "Reverse",
                    self.wavelet.reversed,
                )
                .w_full()
                .on_click(cx.listener(|app, _, _, cx| {
                    app.wavelet.reversed = !app.wavelet.reversed;
                    app.rebuild_wavelet_plots(cx);
                    cx.notify();
                })),
            );
        }
        Some(
            div()
                .id("wavelet-color-dismiss")
                .absolute()
                .inset_0()
                .occlude()
                .track_focus(focus)
                .on_key_down(cx.listener(|app, event: &gpui::KeyDownEvent, _, cx| {
                    if event.keystroke.key == "escape" {
                        app.wavelet.colors_open = false;
                        app.wavelet.history_open = false;
                        cx.stop_propagation();
                        cx.notify();
                    }
                }))
                .on_any_mouse_down(cx.listener(|app, _, _, cx| {
                    app.wavelet.colors_open = false;
                    app.wavelet.history_open = false;
                    cx.stop_propagation();
                    cx.notify();
                }))
                .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
                .child(
                    gpui::anchored()
                        .position(self.wavelet.menu_position)
                        .offset(gpui::point(px(0.), px(15.)))
                        .snap_to_window()
                        .child(menu),
                )
                .into_any_element(),
        )
    }
}
