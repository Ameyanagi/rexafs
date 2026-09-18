//! Compact correction controls and historical-result views.
use super::*;

impl StudioApp {
    pub(crate) fn fluorescence_center(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        if !self.fluorescence.historical
            && self.fluorescence.record.is_none()
            && self.fluorescence.plot.is_none()
            && let Some(sp) = &self.spectrum
            && let (Some(e), Some(mu)) = (&sp.energy, &sp.mu)
        {
            self.fluorescence.plot = Some(
                plot_builder(
                    Plot::new()
                        .theme(self.theme.plot_theme())
                        .line(e.as_slice(), mu.as_slice())
                        .label("Original")
                        .xlabel("Energy (eV)")
                        .ylabel("μ(E)")
                        .legend_position(LegendPosition::UpperRight),
                )
                .interactive()
                .build(cx),
            );
        }
        let t = self.theme;
        let historical = self.fluorescence.historical;
        let header = div()
            .flex()
            .items_center()
            .gap_2()
            .child(
                button(&t, "fluo-back", "← Data", false).on_click(cx.listener(|a, _, _, cx| {
                    a.fluorescence.close();
                    cx.notify();
                })),
            )
            .child(div().flex_1().child("Fluorescence correction"))
            .child(
                button(&t, "fluo-mu", "μ(E)", !self.fluorescence.factor).on_click(cx.listener(
                    |a, _, _, cx| {
                        a.fluorescence.factor = false;
                        a.rebuild_fluorescence_plot(cx);
                        cx.notify();
                    },
                )),
            )
            .child(
                button(&t, "fluo-factor", "Factor", self.fluorescence.factor).on_click(
                    cx.listener(|a, _, _, cx| {
                        if a.fluorescence.record.is_some() {
                            a.fluorescence.factor = true;
                        }
                        a.rebuild_fluorescence_plot(cx);
                        cx.notify();
                    }),
                ),
            );
        let mut column = div()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .flex()
            .flex_col()
            .p_3()
            .gap_3()
            .child(header);
        if historical {
            let count = self
                .current_group_index()
                .map(|i| self.correction_sources(i).len())
                .unwrap_or(0);
            if count > 1 {
                column =
                    column.child(
                        div()
                            .flex()
                            .gap_2()
                            .child(button(&t, "fluo-history-prev", "←", false).on_click(
                                cx.listener(|a, _, _, cx| {
                                    a.load_fluorescence_history(
                                        a.fluorescence.history_index.saturating_sub(1),
                                        cx,
                                    )
                                }),
                            ))
                            .child(format!(
                                "Correction {} / {}",
                                self.fluorescence.history_index + 1,
                                count
                            ))
                            .child(button(&t, "fluo-history-next", "→", false).on_click(
                                cx.listener(|a, _, _, cx| {
                                    a.load_fluorescence_history(
                                        a.fluorescence.history_index + 1,
                                        cx,
                                    )
                                }),
                            )),
                    );
            }
            if let Some(ix) = self
                .fluorescence
                .record
                .as_ref()
                .and_then(|r| r.input.group_id.as_ref())
                .and_then(|id| self.group_registry.index(id))
            {
                column = column.child(div().flex().child(
                    button(&t, "fluo-original", "Open original spectrum", false).on_click(
                        cx.listener(move |a, _, _, cx| {
                            a.fluorescence.close();
                            a.select_entry(ix, cx);
                            cx.notify();
                        }),
                    ),
                ));
            }
        }
        if !historical {
            let mut form = div().flex().gap_2();
            for (i, label) in [
                "Composition",
                "Absorber",
                "Edge",
                "Emission",
                "Incident (°)",
                "Exit (°)",
            ]
            .into_iter()
            .enumerate()
            {
                if let Some(field) = self.fluorescence.fields.get(i) {
                    form = form.child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .text_size(px(12.))
                            .child(label)
                            .child(field.clone()),
                    );
                }
            }
            column = column
                .child(form)
                .child(div().text_size(px(12.)).text_color(t.text_muted).child(
                    "Angles from the sample surface · thick, homogeneous sample · XANES only",
                ))
                .child(
                    div()
                        .flex()
                        .gap_2()
                        .items_center()
                        .child(
                            button(
                                &t,
                                "fluo-input",
                                "Uncorrected fluorescence μ(E)",
                                self.fluorescence.assumed,
                            )
                            .on_click(cx.listener(|a, _, _, cx| {
                                a.fluorescence.assumed = !a.fluorescence.assumed;
                                a.fluorescence.invalidate();
                                cx.notify();
                            })),
                        )
                        .child(
                            button(
                                &t,
                                "fluo-preview",
                                if self.fluorescence.busy {
                                    "Calculating…"
                                } else {
                                    "Preview"
                                },
                                true,
                            )
                            .on_click(cx.listener(|a, _, _, cx| a.preview_fluorescence(cx))),
                        ),
                );
        }
        column = column
            .child(
                div()
                    .text_size(px(12.))
                    .child(self.fluorescence.message.clone()),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .min_w_0()
                    .children(self.fluorescence.plot.clone()),
            );
        if !historical {
            column = column.child(
                div().flex().justify_end().child(
                    button(&t, "fluo-add", "Add corrected spectrum →", true)
                        .opacity(
                            if self.fluorescence.record.is_some() && !self.fluorescence.busy {
                                1.
                            } else {
                                0.4
                            },
                        )
                        .on_click(cx.listener(|a, _, _, cx| a.apply_fluorescence(cx))),
                ),
            );
        }
        column.into_any_element()
    }
    pub(crate) fn fluorescence_inspector(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let t = self.theme;
        let mut out = div().flex().flex_col().p_3().gap_2().text_size(px(12.));
        if !self.fluorescence.historical {
            out = out.child(
                button(
                    &t,
                    "fluo-advanced",
                    "Internal normalization ▾",
                    self.fluorescence.advanced,
                )
                .on_click(cx.listener(|a, _, _, cx| {
                    a.fluorescence.advanced = !a.fluorescence.advanced;
                    cx.notify();
                })),
            );
            if self.fluorescence.advanced {
                out=out.child("Used only to calculate the correction. Blank bounds use core defaults; intervals are eV from E₀.");
                for (i, label) in [
                    (6, "E₀ (eV)"),
                    (7, "Pre from"),
                    (8, "Pre to"),
                    (9, "Post from"),
                    (10, "Post to"),
                    (11, "Degree"),
                ] {
                    if let Some(f) = self.fluorescence.fields.get(i) {
                        out = out.child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(div().w(px(85.)).child(label))
                                .child(div().flex_1().child(f.clone())),
                        );
                    }
                }
            }
        }
        if let Some(record) = &self.fluorescence.record {
            let r = &record.result;
            out = out
                .child(format!(
                    "{} · {} {}",
                    r.requested.formula, r.requested.element, r.requested.edge
                ))
                .child(format!(
                    "{} · {:.1} eV",
                    match &r.emission.selection {
                        rexafs::atomic::EmissionSelection::Line(s) => s.clone(),
                        rexafs::atomic::EmissionSelection::Family(s) => format!("{s} family"),
                    },
                    r.emission.energy_ev
                ))
                .child(format!(
                    "Angles {:.1}° / {:.1}°",
                    r.requested.incidence_deg.unwrap_or_default(),
                    r.requested.exit_deg.unwrap_or_default()
                ))
                .child(format!("Internal E₀ {:.1} eV", r.internal.e0))
                .child(format!(
                    "Pre {:.1} … {:.1} eV",
                    r.internal.pre_edge[0], r.internal.pre_edge[1]
                ))
                .child(format!(
                    "Post {:.1} … {:.1} eV",
                    r.internal.post_edge[0], r.internal.post_edge[1]
                ))
                .child(
                    button(
                        &t,
                        "fluo-notes",
                        SharedString::from(format!("Calculation notes ({}) ▾", r.warnings.len())),
                        self.fluorescence.notes,
                    )
                    .on_click(cx.listener(|a, _, _, cx| {
                        a.fluorescence.notes = !a.fluorescence.notes;
                        cx.notify();
                    })),
                )
                .children(self.fluorescence.notes.then(|| {
                    div().flex().flex_col().gap_2().children(
                        r.warnings
                            .iter()
                            .map(|w| div().text_color(t.warn).child(w.clone())),
                    )
                }))
                .child(
                    button(&t, "fluo-export", "Export correction…", false)
                        .on_click(cx.listener(|a, _, _, cx| a.export_fluorescence(cx))),
                );
        }
        out.into_any_element()
    }
}
