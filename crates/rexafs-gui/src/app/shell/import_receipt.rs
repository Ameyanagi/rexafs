//! Intake presentation; the ledger and receipt wording remain window-independent.
use super::*;
use crate::app::{DataTab, import_state::BatchId};

impl StudioApp {
    pub(crate) fn import_receipt(
        &self,
        cx: &mut Context<Self>,
    ) -> Option<impl IntoElement + use<>> {
        if self.intake.history.is_empty() {
            return None;
        }
        let t = self.theme;
        let mut card = div()
            .flex_none()
            .px_3()
            .py_1()
            .bg(t.surface)
            .border_b_1()
            .border_color(t.border)
            .text_xs();
        if let Some(id) = self.intake.receipt {
            let batch = &self.intake.history[id];
            let mut line = div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap_2()
                .child(batch.receipt());
            let pending = batch
                .sources
                .values()
                .filter(|source| source.pending.is_some())
                .count();
            if pending > 0 {
                line = line.child(
                    button(&t, "resolve-import", &format!("Resolve {pending}…"), false).on_click(
                        cx.listener(move |app, _, window, cx| {
                            app.open_import_review(id, 0, window, cx)
                        }),
                    ),
                );
            }
            if batch.sources.iter().any(|(path, outcome)| {
                outcome
                    .created
                    .iter()
                    .any(|group| self.intake_group_index(path, group).is_some())
            }) {
                line = line.child(
                    button(&t, "show-imported", "Show imported", false).on_click(cx.listener(
                        move |app, _, window, cx| {
                            let indices: Vec<_> = app.intake.history[id]
                                .sources
                                .iter()
                                .flat_map(|(path, outcome)| {
                                    outcome
                                        .created
                                        .iter()
                                        .filter_map(|group| app.intake_group_index(path, group))
                                })
                                .collect();
                            // Only explicitly revealed groups need registry locators.
                            app.intake.reveal =
                                indices.iter().filter_map(|&ix| app.group_id(ix)).collect();
                            app.filter_reveal.get_or_insert_with(|| {
                                (app.expanded_sources.clone(), app.data_tab)
                            });
                            app.data_tab = DataTab::Files;
                            app.data_panel_open = true;
                            for &ix in &indices {
                                app.expand_group_stack(ix);
                            }
                            if let Some(&ix) = indices.first() {
                                app.focus_group = Some(ix);
                                app.scroll_group_row(ix);
                            }
                            let focus = app.data_focus.clone();
                            cx.defer_in(window, move |_, window, cx| focus.focus(window, cx));
                            cx.notify();
                        },
                    )),
                );
            }
            line = line
                .child(
                    button(&t, "import-details", "Details", false).on_click(
                        cx.listener(move |app, _, _, cx| app.show_intake_details(id, cx)),
                    ),
                )
                .child(
                    button(&t, "dismiss-import", "Dismiss", false).on_click(cx.listener(
                        |app, _, _, cx| {
                            app.intake.receipt = None;
                            cx.notify();
                        },
                    )),
                );
            card = card.child(line);
            let mut names = self
                .imports
                .applications
                .iter()
                .filter(|a| a.batch == id)
                .filter_map(|a| self.imports.recipes.get(&a.recipe))
                .map(|r| r.label())
                .collect::<Vec<_>>();
            names.sort();
            names.dedup();
            if !names.is_empty() {
                card = card.child(format!("Recipe: {}", names.join(" · ")));
            }
        }
        if let Some(text) = self.intake.queued_text() {
            card = card.child(text);
        }
        card = card.child(
            button(
                &t,
                "import-history",
                "Import history",
                self.intake.history_open,
            )
            .on_click(cx.listener(|app, _, _, cx| {
                app.intake.history_open = !app.intake.history_open;
                cx.notify();
            })),
        );
        if self.intake.history_open {
            let mut history = div()
                .id("intake-history-list")
                .max_h(px(140.))
                .overflow_y_scroll();
            for (id, batch) in self.intake.history.iter().enumerate().rev() {
                history = history.child(
                    div()
                        .id(SharedString::from(format!("intake-{id}")))
                        .cursor_pointer()
                        .py_1()
                        .hover(|d| d.bg(t.raised))
                        .child(format!(
                            "#{} · {} · {}",
                            id + 1,
                            batch
                                .paths
                                .first()
                                .map(|p| p.display().to_string())
                                .unwrap_or_default(),
                            batch.receipt()
                        ))
                        .on_click(cx.listener(move |app, _, _, cx| {
                            app.intake.receipt = Some(id);
                            app.show_intake_details(id, cx);
                        })),
                );
            }
            card = card.child(history);
        }
        Some(card)
    }

    fn show_intake_details(&mut self, id: BatchId, cx: &mut Context<Self>) {
        self.problems_batch = Some(id);
        self.problems_page = 0;
        self.problems_open = true;
        cx.notify();
    }
}
