//! About, update-channel preferences, and verified desktop downloads.
use super::{
    button,
    controls::{disclosure, icon, icon_button},
};
use crate::icons::Icon;
use crate::{
    app::StudioApp,
    updates::{self, UpdateChannel, UpdateCheck},
};
use gpui::{Context, IntoElement, ParentElement, Styled, div, prelude::*, px};
use std::path::PathBuf;

#[derive(Default)]
pub(crate) struct UpdateState {
    pub open: bool,
    checking: bool,
    downloading: bool,
    generation: u64,
    pub result: Option<UpdateCheck>,
    error: Option<String>,
    downloaded: Option<PathBuf>,
    focus: Option<gpui::FocusHandle>,
    return_focus: Option<gpui::FocusHandle>,
    preferences_open: bool,
}

impl StudioApp {
    pub(crate) fn open_updates(&mut self, cx: &mut Context<Self>) {
        self.updates.open = true;
        let focus = self
            .updates
            .focus
            .get_or_insert_with(|| cx.focus_handle())
            .clone();
        let view = cx.weak_entity();
        let handle = self.main_window;
        cx.defer(move |cx| {
            let _ = handle.update(cx, |_, window, cx| {
                let _ = view.update(cx, |app, cx| {
                    if app.updates.open {
                        if app.updates.return_focus.is_none() {
                            app.updates.return_focus = window.focused(cx);
                        }
                        focus.focus(window, cx);
                    }
                });
            });
        });
        if self.updates.result.is_none() && !self.updates.checking {
            self.check_for_updates(cx);
        }
        cx.notify();
    }
    fn close_updates(&mut self, window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.updates.open = false;
        let focus = self
            .updates
            .return_focus
            .take()
            .unwrap_or_else(|| self.root_focus.clone());
        cx.defer_in(window, move |_, window, cx| focus.focus(window, cx));
        cx.notify();
    }
    pub(crate) fn check_for_updates(&mut self, cx: &mut Context<Self>) {
        if self.updates.checking || self.updates.downloading {
            return;
        }
        self.updates.generation += 1;
        let generation = self.updates.generation;
        let channel = self.structure.settings.update_channel;
        self.updates.checking = true;
        self.updates.error = None;
        self.updates.downloaded = None;
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { updates::check(channel) })
                .await;
            this.update(cx, |app, cx| {
                if generation != app.updates.generation {
                    return;
                }
                app.updates.checking = false;
                match result {
                    Ok(result) => {
                        if result.available {
                            app.status = format!(
                                "{} update available — open Updates to review it.",
                                channel.label()
                            )
                            .into();
                        } else if app.updates.open {
                            app.status = if result.release.is_some() {
                                format!("{} release check complete — up to date.", channel.label())
                            } else {
                                format!("No {} releases are published yet.", channel.label())
                            }
                            .into();
                        }
                        app.updates.result = Some(result);
                    }
                    Err(e) => app.updates.error = Some(e),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }
    fn set_update_channel(&mut self, channel: UpdateChannel, cx: &mut Context<Self>) {
        if self.updates.downloading || channel == self.structure.settings.update_channel {
            return;
        }
        let mut settings = self.structure.settings.clone();
        settings.update_channel = channel;
        if let Err(e) = settings.save() {
            self.updates.error = Some(e);
            cx.notify();
            return;
        }
        self.structure.settings = settings;
        self.updates.generation += 1;
        self.updates.checking = false;
        self.updates.result = None;
        self.updates.downloaded = None;
        self.check_for_updates(cx);
    }
    fn toggle_startup_update_check(&mut self, cx: &mut Context<Self>) {
        let mut settings = self.structure.settings.clone();
        settings.check_updates_on_startup =
            Some(!settings.check_updates_on_startup.unwrap_or(true));
        match settings.save() {
            Ok(()) => self.structure.settings = settings,
            Err(e) => self.updates.error = Some(e),
        }
        cx.notify();
    }
    fn download_update(&mut self, cx: &mut Context<Self>) {
        if self.updates.downloading {
            return;
        }
        let Some(release) = self.updates.result.as_ref().and_then(|r| r.release.clone()) else {
            return;
        };
        if release.asset.is_none() {
            return;
        }
        self.updates.downloading = true;
        self.updates.error = None;
        self.updates.downloaded = None;
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { updates::download(&release) })
                .await;
            this.update(cx, |app, cx| {
                app.updates.downloading = false;
                match result {
                    Ok(path) => {
                        app.updates.downloaded = Some(path);
                        app.status =
                            "Update downloaded and SHA-256 verified. Open Updates to reveal it."
                                .into();
                    }
                    Err(e) => app.updates.error = Some(e),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }
    pub(crate) fn updates_overlay(&self, cx: &mut Context<Self>) -> Option<gpui::AnyElement> {
        if !self.updates.open {
            return None;
        }
        let t = self.theme;
        let channel = self.structure.settings.update_channel;
        let auto = self
            .structure
            .settings
            .check_updates_on_startup
            .unwrap_or(true);
        let mut panel = crate::accessibility::Control::new(
            div().id("updates-panel"),
            "Updates",
            accesskit::Role::Dialog,
        )
        .modal()
        .w(px(440.))
        .max_w_full()
        .max_h_full()
        .overflow_y_scroll()
        .p_4()
        .flex()
        .flex_col()
        .gap_3()
        .rounded_lg()
        .bg(t.surface)
        .border_1()
        .border_color(t.border)
        .child(
            div()
                .flex()
                .items_center()
                .gap_3()
                .child(div().flex_1().text_size(px(18.)).child("Updates"))
                .child(
                    icon_button(&t, "close-updates", Icon::Close, "Close updates", false)
                        .on_click(cx.listener(|app, _, window, cx| app.close_updates(window, cx))),
                ),
        )
        .child(div().text_color(t.text_muted).child(format!(
            "{} · {}",
            updates::installed_label(),
            updates::installed_channel().label()
        )));
        if self.updates.checking {
            panel = panel.child("Checking…");
        } else if let Some(result) = &self.updates.result {
            if let Some(release) = &result.release {
                panel = panel.child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(icon(
                            &t,
                            if result.available {
                                Icon::Download
                            } else {
                                Icon::Check
                            },
                        ))
                        .child(if result.available {
                            format!("{} available", release.tag)
                        } else {
                            "Up to date".into()
                        }),
                );
                let url = release.url.clone();
                panel = panel.child(
                    button(&t, "update-notes", "Release notes", false)
                        .child(icon(&t, Icon::External))
                        .on_click(cx.listener(move |_, _, _, cx| cx.open_url(&url))),
                );
                if let Some(asset) = &release.asset {
                    if !self.updates.downloading
                        && self.updates.downloaded.is_none()
                        && (result.available || self.updates.preferences_open)
                    {
                        panel = panel.child(
                            button(
                                &t,
                                "download-update",
                                format!(
                                    "Download {} · {:.1} MB",
                                    release.tag,
                                    asset.size as f64 / 1_000_000.
                                ),
                                result.available,
                            )
                            .on_click(cx.listener(|app, _, _, cx| app.download_update(cx))),
                        );
                    }
                } else if result.available {
                    panel = panel.child(
                        div()
                            .text_color(t.warn)
                            .child("No verified download for this platform."),
                    );
                }
            } else {
                panel = panel.child(format!("No {} releases yet", channel.label()));
            }
        }
        if self.updates.downloading {
            panel = panel.child("Downloading and verifying…");
        }
        if let Some(path) = &self.updates.downloaded {
            let path = path.clone();
            panel = panel
                .child(
                    div()
                        .flex()
                        .gap_2()
                        .child(icon(&t, Icon::Check))
                        .child("Download verified"),
                )
                .child(
                    button(&t, "reveal-update", "Show download", true)
                        .on_click(cx.listener(move |_, _, _, cx| cx.reveal_path(&path))),
                )
                .child(
                    div()
                        .text_color(t.text_muted)
                        .child("Save your project and quit before replacing the app."),
                );
        }
        if let Some(error) = &self.updates.error {
            panel = panel.child(div().text_color(t.error).child(error.clone()));
        }
        panel = panel.child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    disclosure(
                        &t,
                        "update-preferences",
                        "Preferences",
                        self.updates.preferences_open,
                        false,
                    )
                    .flex_1()
                    .on_click(cx.listener(|app, _, _, cx| {
                        app.updates.preferences_open = !app.updates.preferences_open;
                        cx.notify();
                    })),
                )
                .when(!self.updates.checking && !self.updates.downloading, |d| {
                    d.child(
                        icon_button(&t, "check-updates", Icon::Refresh, "Check again", false)
                            .on_click(cx.listener(|app, _, _, cx| app.check_for_updates(cx))),
                    )
                }),
        );
        if self.updates.preferences_open {
            panel = panel
                .child(
                    div()
                        .flex()
                        .gap_2()
                        .child(
                            button(
                                &t,
                                "stable-updates",
                                "Stable",
                                channel == UpdateChannel::Stable,
                            )
                            .on_click(cx.listener(|app, _, _, cx| {
                                app.set_update_channel(UpdateChannel::Stable, cx)
                            })),
                        )
                        .child(
                            button(
                                &t,
                                "nightly-updates",
                                "Nightly",
                                channel == UpdateChannel::Nightly,
                            )
                            .on_click(cx.listener(|app, _, _, cx| {
                                app.set_update_channel(UpdateChannel::Nightly, cx)
                            })),
                        ),
                )
                .when(channel == UpdateChannel::Nightly, |d| {
                    d.child(
                        div()
                            .text_color(t.text_muted)
                            .child("Daily development builds"),
                    )
                })
                .child(
                    button(
                        &t,
                        "startup-updates",
                        if auto {
                            "✓ Check on startup"
                        } else {
                            "Check on startup"
                        },
                        false,
                    )
                    .on_click(cx.listener(|app, _, _, cx| app.toggle_startup_update_check(cx))),
                );
        }
        Some(
            div()
                .id("updates-overlay")
                .occlude()
                .tab_group()
                .track_focus(
                    self.updates
                        .focus
                        .as_ref()
                        .expect("open update dialog has focus"),
                )
                .on_key_down(cx.listener(|app, event: &gpui::KeyDownEvent, window, cx| {
                    if event.keystroke.key == "escape" {
                        app.close_updates(window, cx);
                    }
                    super::controls::navigate(event, window, cx);
                    cx.stop_propagation();
                }))
                .absolute()
                .inset_0()
                .p_4()
                .flex()
                .items_center()
                .justify_center()
                .bg(gpui::rgba(0x00000099))
                .child(panel)
                .into_any_element(),
        )
    }
}
