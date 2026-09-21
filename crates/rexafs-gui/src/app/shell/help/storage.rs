//! User-reviewed cleanup of cached installers and inactive update app copies.
use super::*;
use crate::storage_cleanup::{self, Report};

#[derive(Default)]
pub(super) struct StorageView {
    report: Option<Report>,
    busy: bool,
    message: Option<String>,
}

impl StudioApp {
    pub(crate) fn open_storage(&mut self, cx: &mut Context<Self>) {
        self.help.menu = false;
        self.help.reader = None;
        self.help.storage = Some(StorageView::default());
        self.focus_help(cx);
        self.refresh_storage(cx);
    }

    fn refresh_storage(&mut self, cx: &mut Context<Self>) {
        let Some(view) = &mut self.help.storage else {
            return;
        };
        if view.busy {
            return;
        }
        view.busy = true;
        self.help.storage_generation += 1;
        let generation = self.help.storage_generation;
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async { storage_cleanup::scan() })
                .await;
            this.update(cx, |app, cx| {
                if app.help.storage_generation != generation {
                    return;
                }
                if let Some(view) = &mut app.help.storage {
                    view.busy = false;
                    match result {
                        Ok(report) => view.report = Some(report),
                        Err(error) => view.message = Some(error),
                    }
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    fn clean_storage(&mut self, cx: &mut Context<Self>) {
        if self.updates.is_busy() {
            return;
        }
        let Some(view) = &mut self.help.storage else {
            return;
        };
        if view.busy {
            return;
        }
        let Some(report) = view.report.clone() else {
            return;
        };
        view.busy = true;
        view.message = Some("Deleting the listed installers and inactive app copies…".into());
        let generation = self.help.storage_generation;
        cx.spawn(async move |this, cx| {
            let (result, refreshed) = cx
                .background_executor()
                .spawn(async move { (storage_cleanup::clean(&report), storage_cleanup::scan()) })
                .await;
            this.update(cx, |app, cx| {
                if app.help.storage_generation != generation {
                    return;
                }
                if let Some(view) = &mut app.help.storage {
                    view.busy = false;
                    view.message = Some(match result {
                        Ok((bytes, errors)) => format!(
                            "Freed {}.{}",
                            storage_cleanup::size(bytes),
                            if errors.is_empty() {
                                String::new()
                            } else {
                                format!(" Some items were kept: {}", errors.join("; "))
                            }
                        ),
                        Err(error) => error,
                    });
                    if let Ok(report) = refreshed {
                        view.report = Some(report);
                    }
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    pub(super) fn storage_panel(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let t = self.theme;
        let view = self.help.storage.as_ref().expect("storage view is open");
        let bytes = view.report.as_ref().map_or(0, Report::bytes);
        let mut panel = div().w(px(720.)).max_w_full().max_h_full().flex().flex_col()
            .gap_3().p_4().bg(t.surface).border_1().border_color(t.border).rounded_lg().shadow_lg()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(div().flex().items_center().justify_between()
                .child(div().text_size(px(20.)).child("Storage"))
                .child(button(&t, "close-storage", "Close", false)
                    .on_click(cx.listener(|app, _, _, cx| app.close_help(cx)))))
            .child("Remove downloaded installers and inactive updater app copies.")
            .child(div().text_color(t.text_muted).child(
                "Your installed app, recovery projects, saved analyses, settings and calculation files are kept. Deletion is permanent; installers can be downloaded again."));
        if let Some(report) = &view.report {
            panel = panel.child(format!(
                "{} available to clean · {} items",
                storage_cleanup::size(bytes),
                report.candidates.len()
            ));
            let mut files = div()
                .id("storage-items")
                .max_h(px(190.))
                .overflow_y_scroll()
                .flex()
                .flex_col()
                .gap_2();
            for item in &report.candidates {
                let path = item.path.clone();
                files = files.child(div().text_size(px(12.)).text_color(t.text_muted).child(
                    format!("{} · {}", storage_cleanup::size(item.bytes), path.display()),
                ));
            }
            panel = panel.child(files);
            let mut folders = div().flex().flex_wrap().gap_2();
            for (i, path) in report.folders.iter().enumerate() {
                let path = path.clone();
                let label = if i == 0 {
                    "Open storage folder".to_owned()
                } else if path.file_name().is_some_and(|n| n == "updates") {
                    "Open installer downloads".to_owned()
                } else {
                    format!(
                        "Open {}",
                        path.file_name().unwrap_or_default().to_string_lossy()
                    )
                };
                folders = folders.child(
                    button(&t, ("storage-folder", i), label, false)
                        .on_click(cx.listener(move |_, _, _, cx| cx.open_with_system(&path))),
                );
            }
            panel = panel.child(folders);
            for warning in &report.warnings {
                panel = panel.child(div().text_color(t.warn).child(warning.clone()));
            }
        }
        if let Some(message) = &view.message {
            panel = panel.child(message.clone());
        }
        if view.busy {
            panel = panel.child("Working…");
        } else {
            panel = panel.child(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        button(&t, "refresh-storage", "Refresh", false)
                            .on_click(cx.listener(|app, _, _, cx| app.refresh_storage(cx))),
                    )
                    .when(bytes > 0 && !self.updates.is_busy(), |row| {
                        row.child(
                            button(
                                &t,
                                "clean-storage",
                                format!("Delete listed files · {}", storage_cleanup::size(bytes)),
                                true,
                            )
                            .on_click(cx.listener(|app, _, _, cx| app.clean_storage(cx))),
                        )
                    }),
            );
        }
        if self.updates.is_busy() {
            panel =
                panel.child("Cleanup is unavailable while an update is downloading or installing.");
        }
        panel.into_any_element()
    }
}
