//! Project and view menus. Secondary commands keep their names inside menus.
use super::{
    button,
    controls::{Menu, icon, icon_button},
};
use crate::{app::StudioApp, icons::Icon, project::DataStorage};
use gpui::{ClickEvent, Context, IntoElement, SharedString, Window, div, prelude::*, px};

impl StudioApp {
    pub(crate) fn open_chrome_menu(
        &mut self,
        menu: Menu,
        event: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.ui.menu == Some(menu) {
            self.close_chrome_menu(window, cx);
            return;
        }
        if menu == Menu::Colors {
            self.sync_spectrum_colors_menu();
        }
        self.ui.return_focus = window.focused(cx);
        self.ui.menu_position = event.position();
        self.ui.menu = Some(menu);
        self.ui
            .menu_focus
            .get_or_insert_with(|| cx.focus_handle())
            .focus(window, cx);
        cx.notify();
    }

    pub(crate) fn close_chrome_menu(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.ui.menu = None;
        let focus = self
            .ui
            .return_focus
            .take()
            .unwrap_or_else(|| self.root_focus.clone());
        focus.focus(window, cx);
        cx.notify();
    }

    /// Save options are local to this dialog until Save is chosen.
    pub(crate) fn save_project(&mut self, cx: &mut Context<Self>) {
        if self.project_saving {
            return;
        }
        self.ui.save_storage = self.project_storage;
        self.ui.menu = Some(Menu::Save);
        let focus = self
            .ui
            .menu_focus
            .get_or_insert_with(|| cx.focus_handle())
            .clone();
        let view = cx.weak_entity();
        let main = self.main_window;
        cx.defer(move |cx| {
            let _ = main.update(cx, |_, window, cx| {
                let _ = view.update(cx, |app, cx| {
                    if app.ui.menu == Some(Menu::Save) {
                        app.ui.return_focus = window.focused(cx);
                        focus.focus(window, cx);
                    }
                });
            });
        });
        cx.notify();
    }

    fn menu_entry(
        &self,
        id: &'static str,
        glyph: Icon,
        label: impl Into<SharedString>,
        keys: &'static str,
        action: fn(&mut Self, &mut Window, &mut Context<Self>),
        cx: &mut Context<Self>,
    ) -> crate::accessibility::Control {
        let t = self.theme;
        let label = label.into();
        crate::accessibility::Control::new(div().id(id), label.clone(), accesskit::Role::MenuItem)
            .tab_index(0)
            .key_context("Control")
            .h(px(32.))
            .px_2()
            .flex()
            .items_center()
            .gap_2()
            .rounded_md()
            .cursor_pointer()
            .border_1()
            .border_color(gpui::transparent_black())
            .hover(|d| d.bg(t.raised))
            .focus(|d| d.border_color(t.accent))
            .child(icon(&t, glyph))
            .child(label)
            .child(div().flex_1())
            .child(
                div()
                    .text_size(px(10.5))
                    .text_color(t.text_muted)
                    .child(keys),
            )
            .on_click(cx.listener(move |app, _, window, cx| {
                app.close_chrome_menu(window, cx);
                action(app, window, cx);
            }))
    }

    pub(crate) fn chrome_menu_overlay(&self, cx: &mut Context<Self>) -> Option<gpui::AnyElement> {
        let menu = self.ui.menu?;
        let t = self.theme;
        let separator = || div().my_1().h(px(1.)).bg(t.border);
        let mut body = crate::accessibility::Control::new(
            div().id("chrome-menu-body"),
            match menu {
                Menu::Project => "Project",
                Menu::Save => "Save project",
                Menu::Plot => "Plot options",
                Menu::Colors => "Spectrum colors",
                Menu::Groups => "Groups",
                Menu::Structure => "Structure appearance",
                Menu::Merge => "Merge preview",
                Menu::RemoveMarked => "Remove marked groups",
            },
            accesskit::Role::Dialog,
        )
        .modal()
        .flex()
        .flex_col()
        .gap_1()
        .p_2()
        .bg(t.surface)
        .border_1()
        .border_color(t.border)
        .rounded_lg()
        .shadow_lg()
        .max_h_full()
        .overflow_y_scroll()
        .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation());
        body = match menu {
            Menu::Project => body
                .child(self.menu_entry(
                    "menu-import",
                    Icon::Import,
                    "Import…",
                    "⇧⌘O",
                    |a, _, c| a.open_folder(c),
                    cx,
                ))
                .child(self.menu_entry(
                    "menu-open",
                    Icon::Folder,
                    "Open project…",
                    "⌘O",
                    |a, _, c| a.open_project(c),
                    cx,
                ))
                .child(self.menu_entry(
                    "menu-save",
                    Icon::Save,
                    "Save project…",
                    "⌘S",
                    |a, _, c| a.save_project(c),
                    cx,
                ))
                .child(separator())
                .child(self.menu_entry(
                    "menu-undo",
                    Icon::Undo,
                    "Undo",
                    "⌘Z",
                    |a, _, c| a.undo(c),
                    cx,
                ))
                .child(self.menu_entry(
                    "menu-redo",
                    Icon::Redo,
                    "Redo",
                    "⇧⌘Z",
                    |a, _, c| a.redo(c),
                    cx,
                ))
                .child(self.menu_entry(
                    "menu-history",
                    Icon::History,
                    "Import history",
                    "",
                    |a, _, c| {
                        a.intake.history_open = !a.intake.history_open;
                        c.notify();
                    },
                    cx,
                ))
                .child(self.menu_entry(
                    "menu-journal",
                    Icon::History,
                    "Analysis history",
                    "⇧⌘J",
                    |a, _, c| {
                        a.journal.open = !a.journal.open;
                        c.notify();
                    },
                    cx,
                ))
                .child(separator())
                .child(self.menu_entry(
                    "menu-theme",
                    if t.mode == crate::theme::ThemeMode::Dark {
                        Icon::Sun
                    } else {
                        Icon::Moon
                    },
                    "Switch theme",
                    "",
                    |a, _, c| a.toggle_theme(c),
                    cx,
                ))
                .child(self.menu_entry(
                    "menu-help",
                    Icon::Help,
                    "Help",
                    "",
                    |a, _, c| a.open_help(c),
                    cx,
                )),
            Menu::Save => {
                let selected = self.ui.save_storage;
                body = body
                    .p_4()
                    .gap_3()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(div().flex_1().text_size(px(17.)).child("Save project"))
                            .child(
                                icon_button(
                                    &t,
                                    "close-save-options",
                                    Icon::Close,
                                    "Close save options",
                                    false,
                                )
                                .on_click(cx.listener(
                                    |app, _, window, cx| app.close_chrome_menu(window, cx),
                                )),
                            ),
                    )
                    .child(
                        button(
                            &t,
                            "save-linked",
                            "Link source files",
                            selected == DataStorage::Paths,
                        )
                        .on_click(cx.listener(|app, _, _, cx| {
                            app.ui.save_storage = DataStorage::Paths;
                            cx.notify();
                        })),
                    )
                    .child(
                        button(
                            &t,
                            "save-portable",
                            "Include source files",
                            selected == DataStorage::Embedded,
                        )
                        .on_click(cx.listener(|app, _, _, cx| {
                            app.ui.save_storage = DataStorage::Embedded;
                            cx.notify();
                        })),
                    )
                    .child(div().text_size(px(11.5)).text_color(t.text_muted).child(
                        if selected == DataStorage::Embedded {
                            "Portable project · larger file"
                        } else {
                            "Smaller project · keep source files available"
                        },
                    ))
                    .child(
                        button(&t, "confirm-save-options", "Choose location…", true).on_click(
                            cx.listener(|app, _, window, cx| {
                                app.project_storage = app.ui.save_storage;
                                app.close_chrome_menu(window, cx);
                                app.save_project_confirmed(cx);
                            }),
                        ),
                    );
                body
            }
            Menu::Plot => body.child(self.plot_options(cx)),
            Menu::Colors => body.child(self.spectrum_colors_menu(cx)),
            Menu::Groups => body
                .child(
                    self.menu_entry(
                        "menu-remove-marked",
                        Icon::Trash,
                        format!("Remove {} marked…", self.selection.len()),
                        "",
                        |a, w, c| a.open_remove_marked(w, c),
                        cx,
                    )
                    .when(self.selection.is_empty(), |d| {
                        d.disabled(true).opacity(0.4).tab_stop(false)
                    }),
                )
                .child(self.menu_entry(
                    "menu-mark-all",
                    Icon::Check,
                    "Mark shown",
                    "⌘A",
                    |a, _, c| a.mark_all(true, c),
                    cx,
                ))
                .child(self.menu_entry(
                    "menu-mark-none",
                    Icon::Close,
                    "Clear marks",
                    "⇧⌘A",
                    |a, _, c| a.clear_selection(c),
                    cx,
                ))
                .child(self.menu_entry(
                    "menu-mark-invert",
                    Icon::Refresh,
                    "Invert shown",
                    "⌘I",
                    |a, _, c| a.invert_group_marks(c),
                    cx,
                ))
                .child(self.menu_entry(
                    "menu-mark-scan",
                    Icon::Layers,
                    "Mark scan",
                    "",
                    |a, _, c| a.select_active_scan(c),
                    cx,
                ))
                .child(self.menu_entry(
                    "menu-mark-filter",
                    Icon::Search,
                    "Mark filter results",
                    "",
                    |a, _, c| a.select_filter_results(c),
                    cx,
                ))
                .child(self.menu_entry(
                    "menu-mark-tenth",
                    Icon::Layers,
                    "Keep every 10th mark",
                    "",
                    |a, _, c| a.thin_selection(c),
                    cx,
                ))
                .child(separator())
                .child(self.menu_entry(
                    "menu-show-marked",
                    Icon::Eye,
                    if self.filter_reveal.is_some() {
                        "Back to filter"
                    } else {
                        "Show marked"
                    },
                    "",
                    |a, _, c| a.toggle_filter_reveal(c),
                    cx,
                ))
                .child(self.menu_entry(
                    "menu-clear-hidden",
                    Icon::Close,
                    "Clear hidden marks",
                    "",
                    |a, _, c| a.clear_hidden_marks(c),
                    cx,
                ))
                .child(
                    self.menu_entry(
                        "menu-lock-current",
                        Icon::Lock,
                        if self
                            .current_group_index()
                            .is_some_and(|i| self.frozen.contains(&i))
                        {
                            "Unlock current"
                        } else {
                            "Lock current"
                        },
                        "",
                        |a, _, c| a.toggle_frozen(c),
                        cx,
                    ),
                ),
            Menu::Structure => body.child(self.structure_display_menu(cx)),
            Menu::Merge => body.child(self.merge_review_panel(cx)),
            Menu::RemoveMarked => body.child(self.marked_removal_panel(cx)),
        };
        let center = matches!(menu, Menu::Save | Menu::Merge | Menu::RemoveMarked);
        let width = if menu == Menu::Merge {
            680.
        } else if center {
            380.
        } else if menu == Menu::Structure {
            420.
        } else {
            280.
        };
        let position = self.ui.menu_position;
        let overlay = div()
            .id("chrome-menu-overlay")
            .absolute()
            .inset_0()
            .occlude()
            .tab_group()
            .track_focus(self.ui.menu_focus.as_ref().expect("open menu has focus"))
            .key_context("StudioPopover")
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(|app, _, window, cx| app.close_chrome_menu(window, cx)),
            )
            .on_key_down(cx.listener(|app, event: &gpui::KeyDownEvent, window, cx| {
                match event.keystroke.key.as_str() {
                    "escape" => app.close_chrome_menu(window, cx),
                    "down" => window.focus_next(cx),
                    "up" => window.focus_prev(cx),
                    _ => {
                        super::controls::navigate(event, window, cx);
                    }
                }
                cx.stop_propagation();
            }))
            .when(center, |d| {
                d.flex()
                    .items_center()
                    .justify_center()
                    .p_4()
                    .bg(gpui::rgba(0x00000088))
            })
            .child(body.w(px(width)).max_w_full().when(!center, |d| {
                d.absolute()
                    .left(px(f32::from(position.x)
                        .min((self.viewport_w - width - 12.).max(8.))
                        .max(8.)))
                    .top(px(f32::from(position.y)
                        .min((self.viewport_h - 320.).max(38.))
                        .max(38.)))
                    .max_h(px((self.viewport_h
                        - f32::from(position.y)
                            .min((self.viewport_h - 320.).max(38.))
                            .max(38.)
                        - 12.)
                        .max(100.)))
            }));
        Some(overlay.into_any_element())
    }

    pub(crate) fn plot_option(
        &self,
        id: &'static str,
        label: &'static str,
        on: bool,
        action: fn(&mut Self, &mut Context<Self>),
        cx: &mut Context<Self>,
    ) -> crate::accessibility::Control {
        let t = self.theme;
        button(&t, id, label, false)
            .h(px(30.))
            .w_full()
            .child(div().flex_1())
            .when(on, |d| d.child(icon(&t, Icon::Check)))
            .on_click(cx.listener(move |app, _, _, cx| action(app, cx)))
    }
}
