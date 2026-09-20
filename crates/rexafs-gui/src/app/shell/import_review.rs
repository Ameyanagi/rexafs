//! Pending sources have review and skip actions, never spectrum marks.
use super::*;
use crate::app::import_state::PendingTarget;
use crate::icons::Icon;

impl StudioApp {
    fn skip_import_targets(&mut self, targets: &[PendingTarget], cx: &mut Context<Self>) {
        let count = self.intake.skip_pending(targets);
        self.status = format!(
            "Skipped {count} pending {}",
            crate::text::noun_for(count, "file")
        )
        .into();
        cx.notify();
    }

    pub(crate) fn pending_imports(
        &self,
        cx: &mut Context<Self>,
    ) -> Option<impl IntoElement + use<>> {
        let pending = self.intake.pending_targets();
        let skipped = self.intake.last_skip.len();
        if pending.is_empty() && skipped == 0 {
            return None;
        }
        let t = self.theme;
        let mut header = div()
            .flex()
            .items_center()
            .gap_1()
            .child(div().flex_1().child(if pending.is_empty() {
                format!("Skipped {skipped}")
            } else {
                format!("Pending {}", pending.len())
            }));
        if skipped > 0 {
            header = header.child(button(&t, "undo-import-skip", "Undo skip", false).on_click(
                cx.listener(|app, _, _, cx| {
                    let restored = app.intake.undo_pending_skip();
                    app.status = format!(
                        "Restored {restored} pending {}",
                        crate::text::noun_for(restored, "file")
                    )
                    .into();
                    cx.notify();
                }),
            ));
        }
        if !pending.is_empty() {
            header = header.child(button(&t, "skip-import-menu", "Skip ▾", false).on_click(
                cx.listener(|app, event, window, cx| {
                    app.open_chrome_menu(controls::Menu::SkipImports, event, window, cx);
                }),
            ));
        } else {
            header = header.child(
                controls::icon_button(
                    &t,
                    "dismiss-import-skip",
                    Icon::Close,
                    "Dismiss skipped-import notice",
                    false,
                )
                .on_click(cx.listener(|app, _, _, cx| {
                    app.intake.last_skip.clear();
                    cx.notify();
                })),
            );
        }
        let mut list = div()
            .id("pending-import-sources")
            .max_h(px(180.))
            .overflow_y_scroll()
            .px_2()
            .py_2()
            .text_size(px(11.))
            .border_b_1()
            .border_color(t.border)
            .child(header);
        for target in pending.into_iter().take(24) {
            let filename = target
                .path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            let reason = self.intake.history[target.batch].sources[&target.path]
                .pending
                .as_ref()
                .unwrap()
                .reason
                .clone();
            let tip = controls::Tooltip {
                label: format!("{filename}\n{reason}").into(),
                theme: t,
            };
            let review_target = target.clone();
            let skip_label = format!("Skip {filename}");
            let id = format!("{}-{}", target.batch, target.path.display());
            let review = crate::accessibility::Control::new(
                div().id(SharedString::from(format!("pending-{id}"))),
                format!("Review {filename}: {reason}"),
                accesskit::Role::Button,
            )
            .tab_index(0)
            .key_context("Control")
            .cursor_pointer()
            .flex_1()
            .min_w_0()
            .py_1()
            .text_color(t.warn)
            .overflow_hidden()
            .whitespace_nowrap()
            .text_ellipsis()
            .tooltip(move |_, cx| cx.new(|_| tip.clone()).into())
            .child(filename)
            .on_click(cx.listener(move |app, _, window, cx| {
                let Some(index) = app
                    .pending_clusters(review_target.batch)
                    .iter()
                    .position(|c| c.files.contains(&review_target.path))
                else {
                    return;
                };
                app.open_import_review(review_target.batch, index, window, cx);
            }));
            list = list.child(
                div().flex().items_center().gap_1().child(review).child(
                    controls::icon_button(
                        &t,
                        SharedString::from(format!("skip-{id}")),
                        Icon::Close,
                        skip_label,
                        false,
                    )
                    .on_click(cx.listener(move |app, _, _, cx| {
                        app.skip_import_targets(std::slice::from_ref(&target), cx);
                    })),
                ),
            );
        }
        Some(list)
    }

    pub(crate) fn pending_skip_menu(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let t = self.theme;
        let current = self.intake.pending_targets();
        // Opening the menu captures its scope. New imports never join it implicitly.
        let targets: Vec<_> = self
            .ui
            .pending_skip
            .iter()
            .filter(|target| current.contains(target))
            .cloned()
            .collect();
        let mut types = std::collections::BTreeMap::<Option<String>, Vec<PendingTarget>>::new();
        for target in &targets {
            types
                .entry(target.extension())
                .or_default()
                .push(target.clone());
        }
        let mut menu = div().flex().flex_col().gap_1();
        for (index, (extension, files)) in types.into_iter().enumerate() {
            let label = match extension {
                Some(extension) => format!("Skip .{extension} ({})", files.len()),
                None => format!("Skip files without extension ({})", files.len()),
            };
            menu = menu.child(
                button(&t, ("skip-pending-type", index), label, false)
                    .w_full()
                    .h(px(30.))
                    .on_click(cx.listener(move |app, _, window, cx| {
                        app.close_chrome_menu(window, cx);
                        app.skip_import_targets(&files, cx);
                    })),
            );
        }
        if !targets.is_empty() {
            menu = menu.child(div().h(px(1.)).my_1().bg(t.border)).child(
                button(
                    &t,
                    "skip-all-pending",
                    format!("Skip all pending ({})", targets.len()),
                    false,
                )
                .w_full()
                .h(px(30.))
                .on_click(cx.listener(move |app, _: &ClickEvent, window, cx| {
                    app.close_chrome_menu(window, cx);
                    app.skip_import_targets(&targets, cx);
                })),
            );
        }
        menu.child(
            div()
                .px_2()
                .py_1()
                .text_size(px(11.))
                .text_color(t.text_muted)
                .child("Current pending files only. Undo is available."),
        )
    }
}
