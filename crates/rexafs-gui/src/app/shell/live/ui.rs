//! Live controls distinguish actions, persistent options, and display choices.
use super::super::controls::{disclosure, icon, icon_button};
use super::*;
use crate::{accessibility::Control, icons::Icon};
use gpui::{AnyElement, Window};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum LiveMenu {
    Channel,
    View,
    Output,
    Processing,
    Peak,
    Exafs,
    FitTrend,
    FitTrendPath,
    Actions,
}

pub(super) fn channel_label(channel: &str) -> String {
    match channel {
        "transmission" => "Transmission",
        "fluorescence" => "Fluorescence",
        "reference" => "Reference",
        "mu" => "μ(E)",
        _ => channel,
    }
    .into()
}

pub(crate) fn check(
    t: &Theme,
    id: impl Into<gpui::ElementId>,
    label: impl Into<SharedString>,
    checked: bool,
) -> Control {
    let label = label.into();
    Control::new(div().id(id), label.clone(), accesskit::Role::CheckBox)
        .selected(checked)
        .tab_index(0)
        .key_context("Control")
        .min_h(px(28.))
        .px_1()
        .flex()
        .items_center()
        .gap_2()
        .rounded_sm()
        .border_1()
        .border_color(gpui::transparent_black())
        .cursor_pointer()
        .text_size(px(12.))
        .text_color(t.text_muted)
        .hover(|d| d.text_color(t.text))
        .focus(|d| d.border_color(t.accent))
        .child(
            div()
                .size(px(14.))
                .flex_none()
                .rounded_sm()
                .border_1()
                .border_color(if checked { t.accent } else { t.border })
                .when(checked, |d| {
                    d.bg(t.accent)
                        .child(icon(t, Icon::Check).size(px(12.)).text_color(t.bg))
                }),
        )
        .child(label)
}

fn field(
    t: &Theme,
    id: &'static str,
    name: &str,
    value: impl Into<SharedString>,
    expanded: bool,
) -> Control {
    let value = value.into();
    Control::new(
        div().id(id),
        format!("{name}: {value}"),
        accesskit::Role::ComboBox,
    )
    .expanded(expanded)
    .tab_index(0)
    .key_context("Control")
    .min_w_0()
    .h(px(30.))
    .px_2()
    .flex()
    .items_center()
    .gap_2()
    .rounded_sm()
    .border_1()
    .border_color(t.border)
    .bg(t.surface)
    .text_color(t.text)
    .text_size(px(12.))
    .cursor_pointer()
    .hover(|d| d.bg(t.raised))
    .focus(|d| d.border_color(t.accent))
    .child(
        div()
            .flex_1()
            .min_w_0()
            .overflow_hidden()
            .whitespace_nowrap()
            .text_ellipsis()
            .child(value),
    )
    .child(icon(t, Icon::ChevronDown).size(px(12.)))
}

fn row(t: &Theme, label: &str, content: impl IntoElement) -> gpui::Div {
    div()
        .flex()
        .items_center()
        .gap_3()
        .child(
            div()
                .w(px(82.))
                .flex_none()
                .text_color(t.text_muted)
                .child(label.to_owned()),
        )
        .child(div().min_w_0().flex_1().child(content))
}

impl StudioApp {
    fn open_live_menu(
        &mut self,
        menu: LiveMenu,
        floating: bool,
        event: &gpui::ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.live.menu = Some(menu);
        self.live.menu_cursor = None;
        self.live.menu_floating = floating;
        self.live.menu_position = event.position();
        self.live.return_focus = window.focused(cx);
        self.live
            .menu_focus
            .get_or_insert_with(|| cx.focus_handle())
            .focus(window, cx);
        cx.notify();
    }
    fn close_live_menu(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.live.menu = None;
        if let Some(focus) = self.live.return_focus.take() {
            focus.focus(window, cx);
        }
        cx.notify();
    }
    pub(super) fn live_selector(
        &self,
        id: &'static str,
        name: &str,
        value: impl Into<SharedString>,
        menu: LiveMenu,
        floating: bool,
        cx: &mut Context<Self>,
    ) -> Control {
        field(
            &self.theme,
            id,
            name,
            value,
            self.live.menu == Some(menu) && self.live.menu_floating == floating,
        )
        .on_click(cx.listener(move |app, event, window, cx| {
            app.open_live_menu(menu, floating, event, window, cx)
        }))
    }
    pub(super) fn live_actions(&self, floating: bool, cx: &mut Context<Self>) -> Control {
        icon_button(
            &self.theme,
            "live-actions",
            Icon::More,
            "Live options",
            false,
        )
        .on_click(cx.listener(move |app, event, window, cx| {
            app.open_live_menu(LiveMenu::Actions, floating, event, window, cx)
        }))
    }
    pub(super) fn live_pause_control(&self, cx: &mut Context<Self>) -> Control {
        let running = self.live.running;
        icon_button(
            &self.theme,
            "live-pause",
            if running { Icon::Pause } else { Icon::Play },
            if running {
                "Pause acquisition"
            } else {
                "Resume acquisition"
            },
            false,
        )
        .w_auto()
        .px_2()
        .gap_1()
        .rounded_sm()
        .border_color(self.theme.border)
        .child(if running { "Pause" } else { "Resume" })
        .disabled(!running && self.live.busy)
        .on_click(cx.listener(|app, _, _, cx| app.toggle_live_pause(cx)))
    }
    pub(super) fn live_follow_control(&self, cx: &mut Context<Self>) -> AnyElement {
        div()
            .flex()
            .items_center()
            .gap_1()
            .child(
                check(
                    &self.theme,
                    "live-follow",
                    "Follow latest",
                    self.live.follow,
                )
                .on_click(cx.listener(|app, _, _, cx| {
                    app.toggle_live_follow(cx);
                    cx.notify();
                })),
            )
            .when(!self.live.follow, |d| {
                d.child(
                    icon_button(
                        &self.theme,
                        "live-previous",
                        Icon::ChevronLeft,
                        "Previous spectrum",
                        false,
                    )
                    .on_click(cx.listener(|app, _, _, cx| app.live_step(-1, cx))),
                )
                .child(
                    icon_button(
                        &self.theme,
                        "live-next",
                        Icon::ChevronRight,
                        "Next spectrum",
                        false,
                    )
                    .on_click(cx.listener(|app, _, _, cx| app.live_step(1, cx))),
                )
            })
            .into_any_element()
    }
    pub(crate) fn live_menu_overlay(
        &self,
        floating: bool,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let kind = self.live.menu?;
        if self.live.menu_floating != floating {
            return None;
        }
        let focus = self.live.menu_focus.as_ref()?;
        // Menu labels and selected values come from the same session as the plot.
        let options: Vec<(String, bool, bool)> = match kind {
            LiveMenu::FitTrend => self
                .live_fit_trend_choices()
                .iter()
                .enumerate()
                .map(|(i, t)| (t.label(), i == self.live.fit_trend, true))
                .collect(),
            LiveMenu::FitTrendPath => self
                .live
                .session
                .and_then(|i| self.measurements.archive.live_sessions.get(i))
                .and_then(|s| s.config.exafs.as_ref())
                .map(|m| {
                    m.paths
                        .iter()
                        .enumerate()
                        .map(|(i, (p, _))| (p.label.clone(), self.live.fit_trend_path == i, true))
                        .collect()
                })
                .unwrap_or_default(),
            LiveMenu::Channel => self
                .live
                .session
                .map(|i| {
                    self.measurements.archive.live_sessions[i]
                        .config
                        .layouts
                        .iter()
                        .map(|l| l.channel_name().to_owned())
                        .collect::<std::collections::BTreeSet<_>>()
                })
                .unwrap_or_default()
                .into_iter()
                .map(|c| {
                    (
                        channel_label(&c),
                        self.live.channel.as_ref() == Some(&c),
                        true,
                    )
                })
                .collect(),
            LiveMenu::View => self
                .live_display_choices()
                .into_iter()
                .map(|mode| {
                    (
                        match mode {
                            LiveDisplay::Latest => "Latest scan",
                            LiveDisplay::Recent => "Last 5 scans",
                            LiveDisplay::Average => "Average",
                            LiveDisplay::PeakFit => "XANES peaks",
                            LiveDisplay::ExafsK => "EXAFS · k",
                            LiveDisplay::ExafsR => "EXAFS · R",
                        }
                        .into(),
                        self.live.display == mode,
                        true,
                    )
                })
                .collect(),
            LiveMenu::Output => vec![
                (
                    "Individual scans".into(),
                    self.live.merge == LiveMerge::Individual,
                    true,
                ),
                (
                    "Running averages".into(),
                    self.live.merge == LiveMerge::Running,
                    true,
                ),
                (
                    "Average every N scans".into(),
                    matches!(self.live.merge, LiveMerge::Batches { .. }),
                    true,
                ),
            ],
            LiveMenu::Processing => std::iter::once((
                "Copy selected spectrum’s settings".into(),
                self.live.recipe.is_none() && self.live.processing_reference.is_none(),
                true,
            ))
            .chain(
                self.measurements
                    .archive
                    .recipes
                    .iter()
                    .enumerate()
                    .map(|(i, r)| {
                        (
                            format!("Recipe: {} · v{}", r.name, r.revision),
                            self.live.recipe == Some(i),
                            true,
                        )
                    }),
            )
            .chain(
                self.processing_reference_choices()
                    .into_iter()
                    .map(|(i, name)| {
                        (
                            format!("Reference: {name}"),
                            self.group_id(i) == self.live.processing_reference,
                            true,
                        )
                    }),
            )
            .collect(),
            LiveMenu::Exafs => {
                std::iter::once(("Off".into(), self.live.exafs_model.is_none(), true))
                    .chain(self.fit_history.iter().enumerate().map(|(i, h)| {
                        (
                            format!("{} · fit #{}", h.group, h.id),
                            self.live.exafs_model == Some(i),
                            h.joint.is_none(),
                        )
                    }))
                    .collect()
            }
            LiveMenu::Peak => std::iter::once(("Off".into(), self.live.peak_model.is_none(), true))
                .chain(self.peaks.archive.models.iter().enumerate().map(|(i, m)| {
                    (
                        format!("{} · v{}", m.name, m.revision),
                        self.live.peak_model == Some(i),
                        true,
                    )
                }))
                .collect(),
            LiveMenu::Actions => vec![
                ("Acquisition details".into(), false, true),
                (
                    if floating {
                        "Dock in sidebar"
                    } else {
                        "Open separate monitor"
                    }
                    .into(),
                    false,
                    true,
                ),
                ("Show source scans".into(), self.live.show_sources, true),
                (
                    "Retry unavailable files".into(),
                    false,
                    !self.live.busy && !self.live.progress.failed.is_empty(),
                ),
                (
                    "Finish acquisition".into(),
                    false,
                    self.live.running
                        || self
                            .live
                            .session
                            .is_some_and(|i| !self.measurements.archive.live_sessions[i].stopped),
                ),
                (
                    "New acquisition…".into(),
                    false,
                    !self.live.running && !self.live.busy,
                ),
            ],
        };
        let mut menu = div()
            .id("live-choice-menu")
            .w(px(265.))
            .max_h(px(360.))
            .overflow_y_scroll()
            .p_1()
            .flex()
            .flex_col()
            .rounded_md()
            .bg(self.theme.raised)
            .border_1()
            .border_color(self.theme.border)
            .shadow_lg()
            .on_any_mouse_down(|_, _, cx| cx.stop_propagation());
        let enabled_items: Vec<usize> = options
            .iter()
            .enumerate()
            .filter(|(_, o)| o.2)
            .map(|(i, _)| i)
            .collect();
        let cursor = self
            .live
            .menu_cursor
            .or_else(|| options.iter().position(|o| o.1 && o.2));
        for (i, (label, selected, enabled)) in options.into_iter().enumerate() {
            let t = self.theme;
            menu = menu.child(
                Control::new(
                    div().id(("live-choice", i)),
                    label.clone(),
                    accesskit::Role::MenuItem,
                )
                .tab_index(0)
                .key_context("Control")
                .disabled(!enabled)
                .h(px(32.))
                .px_2()
                .flex()
                .items_center()
                .gap_2()
                .rounded_sm()
                .text_size(px(12.))
                .text_color(t.text)
                .cursor_pointer()
                .selected(selected)
                .when(cursor == Some(i), |d| d.bg(t.surface))
                .when(!enabled, |d| d.opacity(0.4))
                .hover(|d| d.bg(t.surface))
                .focus(|d| d.bg(t.surface))
                .child(div().size(px(14.)).when(selected, |d| {
                    d.child(icon(&t, Icon::Check).size(px(14.)).text_color(t.accent))
                }))
                .child(label)
                .on_click(cx.listener(move |app, _, window, cx| {
                    app.close_live_menu(window, cx);
                    app.choose_live_option(kind, i, floating, cx);
                })),
            );
        }
        Some(
            div()
                .id("live-menu-dismiss")
                .absolute()
                .inset_0()
                .occlude()
                .track_focus(focus)
                .on_key_down(
                    cx.listener(move |app, event: &gpui::KeyDownEvent, window, cx| {
                        if matches!(
                            event.keystroke.key.as_str(),
                            "up" | "down" | "enter" | "space" | "escape"
                        ) {
                            window.prevent_default();
                        }
                        if matches!(event.keystroke.key.as_str(), "up" | "down")
                            && !enabled_items.is_empty()
                        {
                            let old =
                                cursor.and_then(|i| enabled_items.iter().position(|v| *v == i));
                            let next = if event.keystroke.key == "down" {
                                old.map_or(0, |i| (i + 1) % enabled_items.len())
                            } else {
                                old.map_or(enabled_items.len() - 1, |i| {
                                    (i + enabled_items.len() - 1) % enabled_items.len()
                                })
                            };
                            app.live.menu_cursor = Some(enabled_items[next]);
                            if let Some(focus) = &app.live.menu_focus {
                                focus.focus(window, cx);
                            }
                            cx.notify();
                            cx.stop_propagation();
                        } else if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                            cx.stop_propagation();
                        } else if event.keystroke.key == "escape" {
                            app.close_live_menu(window, cx);
                            cx.stop_propagation();
                        }
                    }),
                )
                .on_key_up(
                    cx.listener(move |app, event: &gpui::KeyUpEvent, window, cx| {
                        // GPUI activates a focused control on key release. Keep the
                        // menu focused until then so returning focus cannot reopen
                        // its trigger with the same Enter/Space press.
                        if matches!(event.keystroke.key.as_str(), "enter" | "space")
                            && app
                                .live
                                .menu_focus
                                .as_ref()
                                .is_some_and(|f| f.is_focused(window))
                        {
                            window.prevent_default();
                            cx.stop_propagation();
                            if let Some(i) = cursor {
                                app.close_live_menu(window, cx);
                                app.choose_live_option(kind, i, floating, cx);
                            }
                        }
                    }),
                )
                .on_any_mouse_down(cx.listener(|app, _, window, cx| {
                    app.close_live_menu(window, cx);
                    cx.stop_propagation();
                }))
                .child(
                    gpui::anchored()
                        .position(self.live.menu_position)
                        .offset(gpui::point(px(0.), px(16.)))
                        .snap_to_window()
                        .child(menu),
                )
                .into_any_element(),
        )
    }
    fn choose_live_option(
        &mut self,
        menu: LiveMenu,
        index: usize,
        floating: bool,
        cx: &mut Context<Self>,
    ) {
        match menu {
            LiveMenu::FitTrend => {
                self.live.fit_trend = index;
                self.live.plot_key.clear();
                self.refresh_live_spectrum(cx);
            }
            LiveMenu::FitTrendPath => self.live.fit_trend_path = index,
            LiveMenu::Channel => {
                self.live.channel = self.live.session.and_then(|i| {
                    self.measurements.archive.live_sessions[i]
                        .config
                        .layouts
                        .iter()
                        .map(|l| l.channel_name().to_owned())
                        .collect::<std::collections::BTreeSet<_>>()
                        .into_iter()
                        .nth(index)
                });
            }
            LiveMenu::View => {
                if let Some(mode) = self.live_display_choices().get(index) {
                    self.live.display = *mode;
                }
            }
            LiveMenu::Output => {
                self.live.merge = [
                    LiveMerge::Individual,
                    LiveMerge::Running,
                    LiveMerge::Batches { scans: 10 },
                ][index]
            }
            LiveMenu::Processing => {
                self.live.recipe = None;
                self.live.processing_reference = None;
                if index > 0 && index <= self.measurements.archive.recipes.len() {
                    self.live.recipe = Some(index - 1);
                } else if index > self.measurements.archive.recipes.len() {
                    self.live.processing_reference = self
                        .processing_reference_choices()
                        .get(index - self.measurements.archive.recipes.len() - 1)
                        .and_then(|(i, _)| self.group_id(*i));
                }
            }
            LiveMenu::Exafs => self.live.exafs_model = index.checked_sub(1),
            LiveMenu::Peak => self.live.peak_model = index.checked_sub(1),
            LiveMenu::Actions => match index {
                0 => {
                    self.stage = Stage::Series;
                    self.open_live(cx);
                    let handle = self.main_window;
                    cx.defer(move |cx| {
                        handle
                            .update(cx, |_, window, _| window.activate_window())
                            .ok();
                    });
                }
                1 => {
                    if self.live.open && self.stage == Stage::Series {
                        self.set_stage(Stage::Data, cx);
                    }
                    self.set_live_host(floating, !floating, cx);
                }
                2 => self.live.show_sources = !self.live.show_sources,
                3 => {
                    if let Some(engine) = &mut self.live.engine {
                        engine.retry();
                    }
                }
                4 => {
                    self.live.stop();
                    if let Some(i) = self.live.session {
                        self.measurements.archive.live_sessions[i].stopped = true;
                    }
                    if !self.live.busy {
                        self.complete_live_run();
                    }
                    self.live.message = "Finished · results retained".into();
                }
                5 => {
                    self.set_live_host(false, false, cx);
                    self.live.engine = None;
                    self.live.session = None;
                    self.live.preview = None;
                    self.live.plot = None;
                    self.live.plotted_group = None;
                    self.live.monitor_plot = None;
                    self.live.plot_generation += 1;
                    self.live.trend = None;
                    self.live.message.clear();
                    self.stage = Stage::Series;
                    self.open_live(cx);
                }
                _ => {}
            },
        }
        if matches!(menu, LiveMenu::Channel | LiveMenu::View) {
            self.live.held_frame = None;
            self.live.held_average = None;
            if !self.live.follow {
                self.hold_live_latest();
            }
            self.refresh_live_spectrum(cx);
        }
        if matches!(
            menu,
            LiveMenu::Output | LiveMenu::Processing | LiveMenu::Peak | LiveMenu::Exafs
        ) {
            self.live.preview = None;
        }
        cx.notify();
    }

    pub(crate) fn live_center(&mut self, cx: &mut Context<Self>) -> AnyElement {
        self.refresh_live_spectrum(cx);
        let t = self.theme;
        let active = self.live.session.is_some();
        let mut view = div()
            .id("live-workspace")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap_3()
            .p_4()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        icon_button(&t, "live-back", Icon::ChevronLeft, "Back to Series", false)
                            .on_click(cx.listener(|app, _, _, cx| {
                                app.live.open = false;
                                if let Some(i) = app.measurements.selected_series {
                                    app.choose_overview_series(i, cx);
                                }
                                cx.notify();
                            })),
                    )
                    .child(
                        div()
                            .flex_1()
                            .text_size(px(17.))
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .child("Live acquisition"),
                    )
                    .when(active, |d| {
                        d.child(self.live_pause_control(cx))
                            .child(
                                icon_button(
                                    &t,
                                    "live-monitor-open",
                                    Icon::External,
                                    "Open separate monitor",
                                    false,
                                )
                                .on_click(cx.listener(
                                    |app, _, _, cx| {
                                        app.set_stage(Stage::Data, cx);
                                        app.set_live_host(false, true, cx);
                                    },
                                )),
                            )
                            .child(self.live_actions(false, cx))
                    }),
            );
        if active {
            view = view
                .child(
                    div()
                        .text_color(t.text_muted)
                        .child(self.live_status_text()),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_3()
                        .child(div().w(px(360.)).child(self.live_view_controls(false, cx)))
                        .child(self.live_follow_control(cx)),
                );
        } else {
            view = view.child(self.live_setup(cx));
        }
        if !self.live.message.is_empty() && !active {
            view = view.child(
                div()
                    .text_color(t.text_muted)
                    .child(self.live.message.clone()),
            );
        }
        if let Some(error) = &self.live.plot_error {
            view = view.child(div().text_color(t.warn).child(error.clone()));
        }
        if let Some(plot) = &self.live.plot {
            view = view.child(div().w_full().h(px(310.)).flex_none().child(plot.clone()));
        }
        if active {
            if let Some(status) = &self.live.plotted_fit_status {
                view = view.child(div().text_color(t.text_muted).child(status.clone()));
            }
            view = view.child(
                div()
                    .text_color(t.text_muted)
                    .child(self.live.plotted_label.clone()),
            );
            view = view.child(self.live_fit_trend_controls(false, cx));
            if let Some(plot) = &self.live.trend {
                view = view.child(div().w_full().h(px(200.)).flex_none().child(plot.clone()));
            }
            view = view
                .child(self.live_fit_parameters(cx))
                .child(self.live_issues(cx));
            if let Some(i) = self.live.session {
                let session = &self.measurements.archive.live_sessions[i];
                let summary = match session.config.merge {
                    LiveMerge::Individual => "Individual scans".to_owned(),
                    LiveMerge::Running => "Running averages".into(),
                    LiveMerge::Batches { scans } => format!("Average every {scans} scans"),
                };
                view = view.child(
                    div()
                        .text_size(px(11.))
                        .text_color(t.text_muted)
                        .child(format!("{summary} · {}", session.config.folder.display())),
                );
                if let Some((index, run)) = self
                    .peaks
                    .archive
                    .runs
                    .iter()
                    .enumerate()
                    .find(|(_, r)| r.id == session.peak_run_id())
                {
                    view = view.child(
                        button(
                            &t,
                            "live-peak-results",
                            format!("Inspect {} peak fits…", run.rows.len()),
                            false,
                        )
                        .on_click(cx.listener(move |app, _, _, cx| app.open_peak_run(index, cx))),
                    );
                }
            }
        } else if !self.live.recoveries.is_empty() {
            view = view.child(
                disclosure(
                    &t,
                    "live-recoveries",
                    "Previous acquisitions",
                    self.live.show_recoveries,
                    false,
                )
                .on_click(cx.listener(|app, _, _, cx| {
                    app.live.show_recoveries = !app.live.show_recoveries;
                    cx.notify();
                })),
            );
            if self.live.show_recoveries {
                for (i, (directory, config)) in self.live.recoveries.iter().take(8).enumerate() {
                    let directory = directory.clone();
                    let date = chrono::DateTime::parse_from_rfc3339(&config.created)
                        .map(|d| {
                            d.with_timezone(&chrono::Local)
                                .format("%b %-d · %H:%M")
                                .to_string()
                        })
                        .unwrap_or_else(|_| config.created.clone());
                    let label = format!(
                        "{} · {date}",
                        config
                            .folder
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                    );
                    view =
                        view.child(
                            Control::new(
                                div().id(("live-recover", i)),
                                format!("Open paused acquisition: {label}"),
                                accesskit::Role::Button,
                            )
                            .tab_index(0)
                            .key_context("Control")
                            .h(px(34.))
                            .max_w(px(760.))
                            .px_2()
                            .flex()
                            .items_center()
                            .gap_2()
                            .border_b_1()
                            .border_color(t.border)
                            .text_color(t.text_muted)
                            .cursor_pointer()
                            .hover(|d| d.bg(t.surface))
                            .focus(|d| d.bg(t.surface))
                            .child(icon(&t, Icon::History))
                            .child(label)
                            .disabled(self.live.busy)
                            .on_click(cx.listener(
                                move |app, _, _, cx| app.recover_live(directory.clone(), cx),
                            )),
                        );
                }
            }
        }
        view.into_any_element()
    }

    fn live_setup(&self, cx: &mut Context<Self>) -> AnyElement {
        let t = self.theme;
        let mut form = div()
            .max_w(px(760.))
            .flex()
            .flex_col()
            .gap_3()
            .child(row(
                &t,
                "Folder",
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(div().flex_1().min_w_0().child(self.live.fields[0].clone()))
                    .child(
                        icon_button(
                            &t,
                            "live-folder",
                            Icon::Folder,
                            "Choose acquisition folder",
                            false,
                        )
                        .on_click(cx.listener(|app, _, _, cx| app.choose_live_folder(cx))),
                    ),
            ))
            .child(row(
                &t,
                "Files",
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(div().w(px(145.)).child(self.live.fields[1].clone()))
                    .child(
                        check(
                            &t,
                            "live-existing",
                            "Include existing files",
                            self.live.include_existing,
                        )
                        .on_click(cx.listener(|app, _, _, cx| {
                            app.live.include_existing = !app.live.include_existing;
                            app.live.preview = None;
                            cx.notify();
                        })),
                    ),
            ));
        let mut signals = div().flex().flex_wrap().items_center().gap_3();
        if self.live.channel_choices.is_empty() {
            signals = signals
                .text_color(t.text_muted)
                .child("Preview a sample to choose signals");
        }
        for (i, channel) in self.live.channel_choices.iter().cloned().enumerate() {
            signals = signals.child(
                check(
                    &t,
                    ("live-signal", i),
                    channel_label(&channel),
                    self.live.selected_channels.contains(&channel),
                )
                .disabled(self.live.busy)
                .on_click(cx.listener(move |app, _, _, cx| {
                    if !app.live.selected_channels.remove(&channel) {
                        app.live.selected_channels.insert(channel.clone());
                    }
                    app.live.preview = None;
                    cx.notify();
                })),
            );
        }
        let output = match self.live.merge {
            LiveMerge::Individual => "Individual scans",
            LiveMerge::Running => "Running averages",
            LiveMerge::Batches { .. } => "Average every N scans",
        };
        form = form
            .child(row(&t, "Signals", signals))
            .child(row(
                &t,
                "Output",
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        self.live_selector(
                            "live-output",
                            "Output",
                            output,
                            LiveMenu::Output,
                            false,
                            cx,
                        )
                        .w(px(215.))
                        .disabled(self.live.busy),
                    )
                    .when(matches!(self.live.merge, LiveMerge::Batches { .. }), |d| {
                        d.child(div().w(px(65.)).child(self.live.fields[4].clone()))
                            .child("scans")
                    }),
            ))
            .child(row(
                &t,
                "Processing",
                self.live_selector(
                    "live-recipe",
                    "Processing",
                    self.live
                        .recipe
                        .and_then(|i| self.measurements.archive.recipes.get(i))
                        .map(|r| format!("{} · v{}", r.name, r.revision))
                        .unwrap_or_else(|| {
                            self.live
                                .processing_reference
                                .as_ref()
                                .map(|id| {
                                    format!("Reference: {}", self.processing_reference_name(id))
                                })
                                .unwrap_or_else(|| "Copy selected spectrum’s settings".into())
                        }),
                    LiveMenu::Processing,
                    false,
                    cx,
                )
                .disabled(self.live.busy),
            ))
            .child(row(
                &t,
                "EXAFS fit",
                self.live_selector(
                    "live-exafs-model",
                    "EXAFS fit",
                    self.live
                        .exafs_model
                        .and_then(|i| self.fit_history.get(i))
                        .map(|h| format!("{} · fit #{}", h.group, h.id))
                        .unwrap_or_else(|| "Off".into()),
                    LiveMenu::Exafs,
                    false,
                    cx,
                )
                .disabled(self.live.busy),
            ))
            .child(row(
                &t,
                "XANES peaks",
                self.live_selector(
                    "live-peak-model",
                    "XANES peaks",
                    self.live
                        .peak_model
                        .and_then(|i| self.peaks.archive.models.get(i))
                        .map(|m| format!("{} · v{}", m.name, m.revision))
                        .unwrap_or_else(|| "Off".into()),
                    LiveMenu::Peak,
                    false,
                    cx,
                )
                .disabled(self.live.busy),
            ))
            .child(
                disclosure(
                    &t,
                    "live-file-handling",
                    "File handling",
                    self.live.advanced,
                    false,
                )
                .on_click(cx.listener(|app, _, _, cx| {
                    app.live.advanced = !app.live.advanced;
                    cx.notify();
                })),
            );
        if self.live.advanced {
            form = form
                .child(row(
                    &t,
                    "Wait until",
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(div().w(px(60.)).child(self.live.fields[2].clone()))
                        .child("unchanged checks, every")
                        .child(div().w(px(60.)).child(self.live.fields[3].clone()))
                        .child("s"),
                ))
                .child(row(
                    &t,
                    "",
                    check(
                        &t,
                        "live-recursive",
                        "Include subfolders",
                        self.live.recursive,
                    )
                    .on_click(cx.listener(|app, _, _, cx| {
                        app.live.recursive = !app.live.recursive;
                        app.live.preview = None;
                        cx.notify();
                    })),
                ));
        }
        let ready = self
            .live
            .preview
            .as_ref()
            .is_some_and(|p| p.signature == self.live_signature(cx));
        form = form.child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .pt_1()
                .child(
                    button(
                        &t,
                        "live-preview",
                        if self.live.busy {
                            "Checking…"
                        } else {
                            "Preview sample"
                        },
                        !ready && !self.live.busy,
                    )
                    .h(px(30.))
                    .rounded_sm()
                    .disabled(self.live.busy)
                    .on_click(cx.listener(|app, _, _, cx| app.preview_live(cx))),
                )
                .when(ready, |d| {
                    d.child(
                        button(&t, "live-start", "Start acquisition", true)
                            .h(px(30.))
                            .rounded_sm()
                            .disabled(self.live.busy)
                            .on_click(cx.listener(|app, _, _, cx| app.start_live(cx))),
                    )
                }),
        );
        form.into_any_element()
    }

    pub(super) fn live_fit_parameters(&self, cx: &mut Context<Self>) -> AnyElement {
        let mut view = div().flex().flex_col().gap_1();
        if !self.live.fit_parameters.is_empty() {
            view = view.child(
                disclosure(
                    &self.theme,
                    "live-fit-parameters",
                    "Fit parameters",
                    self.live.fit_details,
                    false,
                )
                .on_click(cx.listener(|app, _, _, cx| {
                    app.live.fit_details = !app.live.fit_details;
                    cx.notify();
                })),
            );
            if self.live.fit_details {
                for (name, value, error) in &self.live.fit_parameters {
                    view = view.child(
                        div()
                            .flex()
                            .gap_2()
                            .text_size(px(11.))
                            .child(div().flex_1().child(name.clone()))
                            .child(div().font_family(crate::theme::MONO).child(format!(
                                "{value:.5}{}",
                                error.map(|e| format!(" ± {e:.5}")).unwrap_or_default()
                            ))),
                    );
                }
                view = view.child(
                    div()
                        .text_size(px(10.))
                        .text_color(self.theme.text_muted)
                        .child("Units follow the saved model. Errors are local estimates."),
                );
            }
        }
        view.into_any_element()
    }
    pub(super) fn live_issues(&self, cx: &mut Context<Self>) -> AnyElement {
        let issues = self.live_issue_messages();
        let mut view = div().flex().flex_col().gap_1();
        if !issues.is_empty() {
            view = view.child(
                disclosure(
                    &self.theme,
                    "live-issues",
                    format!("{} need attention", issues.len()),
                    self.live.show_issues,
                    false,
                )
                .text_color(self.theme.warn)
                .on_click(cx.listener(|app, _, _, cx| {
                    app.live.show_issues = !app.live.show_issues;
                    cx.notify();
                })),
            );
            if self.live.show_issues {
                for issue in issues.into_iter().take(20) {
                    view = view.child(div().text_color(self.theme.warn).child(issue));
                }
                for (i, (path, _)) in self
                    .live
                    .progress
                    .review
                    .iter()
                    .chain(&self.live.progress.failed)
                    .take(20)
                    .enumerate()
                {
                    let path = path.clone();
                    view = view.child(
                        button(
                            &self.theme,
                            ("live-review", i),
                            format!(
                                "Review {}…",
                                path.file_name().unwrap_or_default().to_string_lossy()
                            ),
                            false,
                        )
                        .on_click(cx.listener(move |app, _, _, cx| {
                            app.open_measurement_path(path.clone(), false, cx)
                        })),
                    );
                }
            }
        }
        view.into_any_element()
    }
}
