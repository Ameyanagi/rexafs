//! Batch color presets for the groups in the current plot scope.
use gpui::{Context, IntoElement, SharedString, div, prelude::*, px};

use super::{PlotScope, Stage, chip, journal::UndoOp};
use crate::{
    app::StudioApp,
    group_identity::GroupId,
    spectrum_colors::{Assignment, Palette, rgba},
};

impl StudioApp {
    fn color_targets(&self) -> Vec<GroupId> {
        let indices = if self.stage == Stage::Fit || self.stage_view.scope == PlotScope::Current {
            self.current_group_index().into_iter().collect()
        } else {
            self.compare_groups()
        };
        indices
            .into_iter()
            .filter_map(|ix| self.group_id(ix))
            .collect()
    }

    pub(crate) fn sync_spectrum_colors_menu(&mut self) {
        if let Some(assignment) = self
            .color_targets()
            .first()
            .and_then(|id| self.group_state.plot_colors.get(id))
        {
            self.ui.reverse_colors = assignment.reversed;
        }
    }

    fn apply_spectrum_palette(&mut self, palette: Option<Palette>, cx: &mut Context<Self>) {
        let ids = self.color_targets();
        let count = ids.len();
        let mut changes = Vec::new();
        for (index, id) in ids.into_iter().enumerate() {
            let after = palette.map(|palette| Assignment {
                palette,
                index,
                count,
                reversed: self.ui.reverse_colors && palette.gradient(),
            });
            let before = self.group_state.plot_colors.remove(&id);
            if let Some(assignment) = after {
                self.group_state.plot_colors.insert(id.clone(), assignment);
            }
            if before != after {
                changes.push((id, before, after));
            }
        }
        if !changes.is_empty() {
            self.record(
                format!(
                    "{} colors · {count} groups",
                    palette.map_or("reset", Palette::label)
                ),
                Some(UndoOp::SpectrumColors { changes }),
            );
            self.invalidate_explore_plots(cx);
        }
        cx.notify();
    }

    pub(crate) fn spectrum_colors_menu(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let t = self.theme;
        let ids = self.color_targets();
        let count = ids.len();
        let current = ids
            .first()
            .and_then(|id| self.group_state.plot_colors.get(id))
            .map(|a| a.palette)
            .filter(|palette| {
                ids.iter().all(|id| {
                    self.group_state
                        .plot_colors
                        .get(id)
                        .is_some_and(|a| a.palette == *palette)
                })
            });
        let mut body = div()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .px_2()
                    .py_1()
                    .text_size(px(13.))
                    .child("Spectrum colors"),
            )
            .child(
                div()
                    .px_2()
                    .pb_2()
                    .text_size(px(11.))
                    .text_color(t.text_muted)
                    .child(format!(
                        "Apply to {count} {}",
                        if count == 1 {
                            "current group"
                        } else {
                            "marked + current groups"
                        }
                    )),
            );
        for (i, palette) in Palette::ALL.into_iter().enumerate() {
            if i == 0 || i == 3 {
                body = body.child(
                    div()
                        .px_2()
                        .pt_2()
                        .pb_1()
                        .text_size(px(10.5))
                        .text_color(t.text_muted)
                        .child(if i == 0 { "Color cycles" } else { "Gradients" }),
                );
            }
            let mut swatches = div().flex().items_center().gap(px(2.));
            for index in 0..6 {
                swatches = swatches.child(
                    div().w(px(10.)).h(px(14.)).rounded_sm().bg(rgba(
                        Assignment {
                            palette,
                            index,
                            count: 6,
                            reversed: self.ui.reverse_colors,
                        }
                        .color(&t),
                    )),
                );
            }
            body = body.child(
                crate::accessibility::Control::new(
                    div().id(SharedString::from(format!("spectrum-palette-{i}"))),
                    format!("Apply {} to {count} groups", palette.label()),
                    accesskit::Role::Button,
                )
                .selected(current == Some(palette))
                .tab_index(0)
                .key_context("Control")
                .flex()
                .items_center()
                .gap_2()
                .h(px(32.))
                .px_2()
                .rounded_md()
                .border_1()
                .border_color(if current == Some(palette) {
                    t.accent
                } else {
                    gpui::Rgba { a: 0., ..t.border }
                })
                .hover(|d| d.bg(t.raised))
                .focus(|d| d.border_color(t.accent))
                .cursor_pointer()
                .disabled(count == 0)
                .when(count == 0, |d| d.opacity(0.4))
                .child(div().flex_1().text_size(px(12.)).child(palette.label()))
                .child(swatches)
                .on_click(
                    cx.listener(move |app, _, _, cx| app.apply_spectrum_palette(Some(palette), cx)),
                ),
            );
        }
        body.child(div().mt_2().px_2().child(
            chip(&t, "reverse-spectrum-colors", "Reverse gradient", self.ui.reverse_colors)
                .on_click(cx.listener(move |app, _, _, cx| {
                    app.ui.reverse_colors = !app.ui.reverse_colors;
                    if let Some(palette) = current.filter(|p| p.gradient()) {
                        app.apply_spectrum_palette(Some(palette), cx);
                    } else { cx.notify(); }
                }))))
            .child(div().px_2().py_2().text_size(px(10.5)).text_color(t.text_muted)
                .child("Assigned in group order. Colors stay with each group. Use Marked to color an overlay."))
            .child(chip(&t, "reset-spectrum-colors", "Reset group colors", false)
                .on_click(cx.listener(|app, _, _, cx| app.apply_spectrum_palette(None, cx))))
    }
}
