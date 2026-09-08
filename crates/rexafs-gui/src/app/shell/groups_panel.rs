//! Groups panel (Athena's group list, modernized): every catalog file,
//! scan, and derived spectrum is a *group*. The current group (highlighted)
//! fills the inspector; marked groups (checkbox) are what overlays and bulk
//! actions act on. Source stacks and Results share one virtualized list.

use gpui::{
    ClickEvent, Context, IntoElement, ParentElement, SharedString, Styled, div, prelude::*, px,
    uniform_list,
};

use super::{MONO, PlotScope, button};
use crate::app::group_rows::{self, Row};
use crate::app::{
    DERIVED_BASE, DataTab, NavDown, NavExtendDown, NavExtendUp, NavUp, ScanListRow, StudioApp,
    scan_list_row,
};
use crate::plotting::{middle_truncate, trace_rgba};

/// Stable group colour, also used by its plot trace and legend.
fn swatch(color: gpui::Rgba) -> impl IntoElement {
    div()
        .w(px(10.))
        .h(px(10.))
        .rounded_xs()
        .flex_none()
        .bg(color)
}

/// Mark checkbox.
fn checkbox(t: &crate::theme::Theme, on: bool) -> impl IntoElement {
    div()
        .w(px(14.))
        .h(px(14.))
        .flex_none()
        .rounded_sm()
        .border_1()
        .flex()
        .items_center()
        .justify_center()
        .when(on, |d| d.bg(t.accent).border_color(t.accent))
        .when(!on, |d| d.bg(t.raised).border_color(t.border))
        .child(
            div()
                .text_size(px(10.))
                .text_color(t.bg)
                .child(if on { "✓" } else { "" }),
        )
}

#[derive(Clone)]
struct RowTooltip {
    text: String,
    theme: crate::theme::Theme,
}
impl gpui::Render for RowTooltip {
    fn render(&mut self, _: &mut gpui::Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .max_w(px(500.))
            .p_2()
            .bg(self.theme.raised)
            .text_color(self.theme.text)
            .text_size(px(12.))
            .child(self.text.clone())
    }
}
struct SidebarResize;
impl gpui::Render for SidebarResize {
    fn render(&mut self, _: &mut gpui::Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

pub(crate) fn processing_locked(
    current: Option<usize>,
    frozen: &std::collections::BTreeSet<usize>,
) -> bool {
    current.is_some_and(|ix| frozen.contains(&ix))
}

pub(crate) fn restore_standalone_lock(
    frozen: &mut std::collections::BTreeSet<usize>,
    state: &crate::group_identity::GroupState,
    id: &crate::group_identity::GroupId,
) {
    if state.frozen.contains(id) {
        frozen.insert(crate::app::NO_ENTRY);
    } else {
        frozen.remove(&crate::app::NO_ENTRY);
    }
}

pub(crate) fn migrate_standalone_lock(
    frozen: &mut std::collections::BTreeSet<usize>,
    index: usize,
) {
    if frozen.remove(&crate::app::NO_ENTRY) {
        frozen.insert(index);
    }
}

impl StudioApp {
    pub(crate) fn group_row_mode(&self, ix: usize) -> crate::params::DetectionMode {
        let mode = self.effective_params(ix).import.mode;
        if mode == crate::params::DetectionMode::Auto && self.current_group_index() == Some(ix) {
            self.import_preview
                .as_ref()
                .map_or(mode, |p| p.resolved.mode)
        } else {
            mode
        }
    }

    pub(crate) fn default_group_row_label(&self, row: Row) -> String {
        let ix = row.group().unwrap_or(crate::app::NO_ENTRY);
        if matches!(row, Row::Child { .. }) {
            self.group_row_mode(ix).label().to_string()
        } else {
            ix.checked_sub(DERIVED_BASE)
                .and_then(|i| self.derived.get(i))
                .map(|d| d.display_label())
                .unwrap_or_else(|| self.default_entry_label(ix))
        }
    }

    pub(crate) fn group_row_label(&self, row: Row) -> String {
        let ix = row.group().unwrap_or(crate::app::NO_ENTRY);
        self.group_state
            .display_label(self.peek_group_id(ix).as_ref(), || {
                self.default_group_row_label(row)
            })
    }

    pub(crate) fn invert_group_marks(&mut self, cx: &mut Context<Self>) {
        self.interaction_rows().invert_shown(&mut self.selection);
        self.ensure_compare_loaded(cx);
        self.sync_param_fields(cx);
        cx.notify();
    }

    /// Lock / unlock processing for the current group.
    pub(crate) fn toggle_frozen(&mut self, cx: &mut Context<Self>) {
        let Some(ix) = self.current_group_index() else {
            return;
        };
        self.toggle_group_lock(ix, cx);
    }

    pub(crate) fn toggle_group_lock(&mut self, ix: usize, cx: &mut Context<Self>) {
        let Some(id) = self.group_id(ix) else { return };
        let label = self.entry_label(ix);
        if !self.frozen.remove(&ix) {
            self.frozen.insert(ix);
            self.group_state.frozen.insert(id);
            self.record(format!("lock processing {label}"), None);
        } else {
            self.group_state.frozen.remove(&id);
            self.record(format!("unlock processing {label}"), None);
        }
        cx.notify();
    }

    pub(crate) fn group_color_index(&self, ix: usize) -> usize {
        self.peek_group_id(ix)
            .as_ref()
            .map(|id| {
                self.group_state
                    .colors
                    .get(id)
                    .map(|c| usize::from(*c % 8))
                    .unwrap_or_else(|| group_rows::color_index(id))
            })
            .unwrap_or(0)
    }

    fn stack_id(&self, ix: usize) -> Option<crate::group_identity::GroupId> {
        let path = if ix < self.catalog.len() {
            Some(self.catalog.path(ix))
        } else {
            ix.checked_sub(DERIVED_BASE)
                .and_then(|i| self.derived.get(i))
                .and_then(|d| d.source.clone())
        };
        if let Some(path) = path {
            if let Some(primary) = self.catalog.find_by_canonical_path(&path) {
                return Some(self.group_registry.register_source(
                    Some(primary),
                    path,
                    self.effective_params(primary).import.mode,
                    &self.project_source_origins,
                ));
            }
            if let Some((_, _, id)) = self
                .standalone_source
                .as_ref()
                .filter(|(p, _, _)| p == &path)
            {
                return Some(id.clone());
            }
            // Orphan stacks also retain their disclosure when their first channel changes.
            return Some(crate::group_identity::GroupId::source(
                &path,
                crate::params::DetectionMode::Auto,
            ));
        }
        self.group_id(ix)
    }

    pub(crate) fn interaction_rows(&self) -> group_rows::Rows {
        if self.data_tab == DataTab::Scans {
            let scan = self.expanded_scan.and_then(|i| self.catalog.scans.get(i));
            return self
                .group_rows()
                .in_catalog_range(scan.map_or(0, |s| s.start), scan.map_or(0, |s| s.len));
        }
        self.group_rows()
    }

    pub(crate) fn group_rows(&self) -> group_rows::Rows {
        self.rows_for_filter(self.filter_reveal.is_some())
    }

    fn rows_for_filter(&self, reveal: bool) -> group_rows::Rows {
        let mut marks = std::collections::BTreeSet::new();
        if reveal {
            marks.clone_from(&self.selection);
            marks.extend(self.reveal_current);
            marks.extend(
                self.intake
                    .reveal
                    .iter()
                    .filter_map(|id| self.group_registry.index(id)),
            );
        }
        group_rows::build_rows_active(
            &self.catalog,
            &self.derived,
            |g| {
                self.stack_id(g)
                    .and_then(|id| self.expanded_sources.get(&id).copied())
            },
            self.filtered.clone(),
            &self.filter_text,
            self.standalone_path(),
            &marks,
            &self.group_registry.excluded_indices(),
        )
    }

    fn toggle_filter_reveal(&mut self, cx: &mut Context<Self>) {
        if let Some((expanded, tab)) = self.filter_reveal.take() {
            self.expanded_sources = expanded;
            self.data_tab = tab;
            self.reveal_current = None;
            self.intake.reveal.clear();
        } else {
            self.filter_reveal = Some((self.expanded_sources.clone(), self.data_tab));
            self.data_tab = DataTab::Files;
        }
        cx.notify();
    }

    fn clear_hidden_marks(&mut self, cx: &mut Context<Self>) {
        let rows = self.interaction_rows();
        self.selection.retain(|&g| !rows.hidden_by_filter(g));
        self.ensure_compare_loaded(cx);
        self.sync_param_fields(cx);
        cx.notify();
    }

    fn focus_stack(&mut self, expand: bool, cx: &mut Context<Self>) {
        let rows = self.group_rows();
        let Some(row) = self
            .focus_group
            .and_then(|g| rows.row_index(g))
            .and_then(|i| rows.row_at(i))
        else {
            return;
        };
        match row {
            Row::Primary {
                group,
                extra_channels,
                ..
            } if extra_channels > 0 => {
                if let Some(id) = self.stack_id(group) {
                    self.expanded_sources.insert(id, expand);
                }
            }
            Row::Child { parent, .. } if !expand => self.focus_group = Some(parent),
            _ => {}
        }
        cx.notify();
    }

    fn leave_filter(&mut self, window: &mut gpui::Window, cx: &mut Context<Self>) {
        window.focus(&self.data_focus, cx);
        let rows = self.interaction_rows();
        if let Some(group) = self
            .current_group_index()
            .filter(|&g| rows.row_index(g).is_some())
            .or_else(|| rows.shown().next())
        {
            self.click_group(group, Default::default(), cx);
        }
    }

    pub(crate) fn reveal_group_row(&mut self, ix: usize) {
        self.expand_group_stack(ix);
        self.scroll_group_row(ix);
    }

    pub(crate) fn expand_group_stack(&mut self, ix: usize) {
        if let Some(path) = ix
            .checked_sub(DERIVED_BASE)
            .and_then(|i| self.derived.get(i))
            .and_then(|d| d.source.as_ref())
        {
            let parent = self
                .standalone_path()
                .filter(|p| *p == path)
                .map(|_| crate::app::NO_ENTRY)
                .or_else(|| self.catalog.find_by_canonical_path(path))
                .or_else(|| {
                    self.derived
                        .iter()
                        .position(|d| d.source.as_ref() == Some(path))
                        .map(|i| DERIVED_BASE + i)
                });
            if let Some(parent) = parent.filter(|&p| p != ix)
                && let Some(id) = self.stack_id(parent)
            {
                self.expanded_sources.insert(id, true);
            }
        }
    }

    pub(crate) fn scroll_group_row(&mut self, ix: usize) {
        if let Some(row) = self.interaction_rows().scroll_row(ix, self.expanded_scan) {
            let scroll = if self.data_tab == DataTab::Scans {
                &self.scan_scroll
            } else {
                &self.file_scroll
            };
            scroll.scroll_to_item(row, gpui::ScrollStrategy::Nearest);
        }
    }

    fn finish_groups_resize(&mut self, cx: &mut Context<Self>) {
        if self.groups_resize.take().is_some() {
            if let Err(error) = self.structure.settings.save() {
                self.record_job_error("sidebar width", error);
            }
            cx.notify();
        }
    }

    pub(crate) fn click_group(
        &mut self,
        ix: usize,
        modifiers: gpui::Modifiers,
        cx: &mut Context<Self>,
    ) {
        self.click_entry(ix, modifiers, cx);
    }

    pub(crate) fn toggle_mark(&mut self, ix: usize, cx: &mut Context<Self>) {
        let modifiers = gpui::Modifiers {
            platform: true,
            ..Default::default()
        };
        self.click_entry(ix, modifiers, cx);
    }

    pub(crate) fn groups_panel(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let t = self.theme;
        let (marked, hidden, collapsed) = self.interaction_rows().mark_counts(&self.selection);
        let footer = format!(
            "{marked} marked · {hidden} hidden by filter · {collapsed} marked in collapsed rows"
        );
        let scope_on = self.stage_view.scope == PlotScope::Marked;
        div()
            .id("groups-panel")
            .key_context("DataPanel")
            .track_focus(&self.data_focus)
            .on_action(
                cx.listener(|this, _: &crate::app::RenameGroup, window, cx| {
                    if let Some(ix) = this
                        .focus_group
                        .filter(|&ix| this.interaction_rows().row_index(ix).is_some())
                    {
                        this.start_group_rename(ix, window, cx);
                    } else if this.data_tab == DataTab::Scans
                        && let Some(scan) = this.active_scan
                    {
                        this.expanded_scan = (this.expanded_scan != Some(scan)).then_some(scan);
                        cx.notify();
                    }
                }),
            )
            .on_action(
                cx.listener(|this: &mut Self, _: &crate::app::MarkAllGroups, _, cx| {
                    this.mark_all(true, cx)
                }),
            )
            .on_action(
                cx.listener(|this: &mut Self, _: &crate::app::InvertGroupMarks, _, cx| {
                    this.invert_group_marks(cx)
                }),
            )
            .on_action(cx.listener(|this: &mut Self, _: &NavUp, _window, cx| {
                this.nav_move(-1, false, cx);
            }))
            .on_action(cx.listener(|this: &mut Self, _: &NavDown, _window, cx| {
                this.nav_move(1, false, cx);
            }))
            .on_action(
                cx.listener(|this: &mut Self, _: &NavExtendUp, _window, cx| {
                    this.nav_move(-1, true, cx);
                }),
            )
            .on_action(
                cx.listener(|this: &mut Self, _: &NavExtendDown, _window, cx| {
                    this.nav_move(1, true, cx);
                }),
            )
            .on_action(cx.listener(
                |this: &mut Self, _: &crate::app::ClearCompare, _window, cx| {
                    this.clear_selection(cx);
                },
            ))
            .on_action(
                cx.listener(|this, _: &crate::app::ToggleFocusedMark, _, cx| {
                    if let Some(ix) = this
                        .focus_group
                        .filter(|&g| this.interaction_rows().row_index(g).is_some())
                    {
                        this.toggle_mark(ix, cx);
                    }
                }),
            )
            .on_action(
                cx.listener(|this, _: &crate::app::CollapseFocusedStack, _, cx| {
                    this.focus_stack(false, cx)
                }),
            )
            .on_action(
                cx.listener(|this, _: &crate::app::ExpandFocusedStack, _, cx| {
                    this.focus_stack(true, cx)
                }),
            )
            .relative()
            .w(px(self.structure.settings.groups_panel_width()))
            .h_full()
            .min_h_0()
            .min_w_0()
            .flex_none()
            .flex()
            .flex_col()
            .bg(t.surface)
            .border_r_1()
            .border_color(t.border)
            .child(
                div()
                    .px_3()
                    .pt_2()
                    .pb_1()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(super::section_label(&t, "Groups"))
                    .child(
                        div()
                            .font_family(MONO)
                            .text_size(px(11.))
                            .text_color(t.text_muted)
                            .child(format!(
                                "{}",
                                self.catalog.len()
                                    - self
                                        .group_registry
                                        .excluded_indices()
                                        .iter()
                                        .filter(|&&g| g < DERIVED_BASE)
                                        .count()
                                    + self.derived.len()
                                    + usize::from(self.standalone_path().is_some())
                            )),
                    )
                    .child(div().flex_1())
                    .child(
                        div()
                            .id("import-files")
                            .px_1p5()
                            .rounded_sm()
                            .text_size(px(11.5))
                            .text_color(t.accent)
                            .cursor_pointer()
                            .hover(|d| d.bg(t.raised))
                            .on_click(cx.listener(|this, _: &ClickEvent, _window, cx| {
                                this.open_folder(cx);
                            }))
                            .child("+ Import"),
                    ),
            )
            .child(
                div()
                    .key_context("GroupFilter")
                    .px_2()
                    .pb_1()
                    .on_action(
                        cx.listener(|this, _: &crate::app::LeaveFilter, window, cx| {
                            this.leave_filter(window, cx)
                        }),
                    )
                    .on_action(
                        cx.listener(|this, _: &crate::app::EscapeFilter, window, cx| {
                            if this.filter_text.is_empty() {
                                window.focus(&this.data_focus, cx);
                            } else {
                                this.filter_text.clear();
                                if let Some(input) = &this.filter_input {
                                    input.update(cx, |input, cx| input.set_text("", cx));
                                }
                                this.apply_filter(cx);
                            }
                        }),
                    )
                    .children(self.filter_input.clone()),
            )
            .child(
                div()
                    .px_2()
                    .pb_1()
                    .flex()
                    .flex_wrap()
                    .gap_1()
                    .child(
                        button(&t, "mark-all-groups", "Mark shown", false)
                            .on_click(cx.listener(|this, _, _, cx| this.mark_all(true, cx))),
                    )
                    .child(
                        button(&t, "unmark-all-groups", "Clear marks", false)
                            .on_click(cx.listener(|this, _, _, cx| this.clear_selection(cx))),
                    )
                    .child(
                        button(&t, "invert-all-groups", "Invert shown", false)
                            .on_click(cx.listener(|this, _, _, cx| this.invert_group_marks(cx))),
                    ),
            )
            .child(
                div()
                    .flex()
                    .border_b_1()
                    .border_color(t.border)
                    .child(self.data_tab_button("tab-files", "Files", DataTab::Files, cx))
                    .child(self.data_tab_button("tab-scans", "Scans", DataTab::Scans, cx)),
            )
            .child(match self.data_tab {
                DataTab::Files => self.file_list(cx).into_any_element(),
                DataTab::Scans => self.scan_list(cx).into_any_element(),
            })
            .child(
                div()
                    .px_3()
                    .pt_2()
                    .text_size(px(11.))
                    .text_color(t.text_muted)
                    .child(footer),
            )
            .child(
                div()
                    .px_2()
                    .py_1()
                    .flex()
                    .flex_wrap()
                    .gap_1()
                    .child(
                        button(
                            &t,
                            "show-marked",
                            if self.filter_reveal.is_some() {
                                "Back to filter"
                            } else {
                                "Show marked"
                            },
                            false,
                        )
                        .on_click(cx.listener(|this, _, _, cx| this.toggle_filter_reveal(cx))),
                    )
                    .child(
                        button(&t, "clear-hidden", "Clear hidden marks", false)
                            .on_click(cx.listener(|this, _, _, cx| this.clear_hidden_marks(cx))),
                    )
                    .when(
                        self.current_group_index()
                            .is_some_and(|g| self.interaction_rows().row_index(g).is_none()),
                        |d| {
                            d.child(
                                button(&t, "reveal-current", "Reveal current", false).on_click(
                                    cx.listener(|this, _, _, cx| {
                                        this.filter_reveal.get_or_insert_with(|| {
                                            (this.expanded_sources.clone(), this.data_tab)
                                        });
                                        this.data_tab = DataTab::Files;
                                        this.reveal_current = this.current_group_index();
                                        if let Some(ix) = this.reveal_current {
                                            this.reveal_group_row(ix);
                                        }
                                        cx.notify();
                                    }),
                                ),
                            )
                        },
                    ),
            )
            .child(
                div()
                    .px_2()
                    .py_2()
                    .flex()
                    .flex_wrap()
                    .gap_1p5()
                    .border_t_1()
                    .border_color(t.border)
                    .child(
                        button(&t, "merge-marked", format!("Merge {marked}…"), false).on_click(
                            cx.listener(|this, _: &ClickEvent, _window, cx| {
                                this.merge_selection(cx);
                            }),
                        ),
                    )
                    .child(
                        button(&t, "align-marked", "Align…", false).on_click(cx.listener(
                            |this, _: &ClickEvent, _window, cx| {
                                this.open_tool(super::tools::Tool::Align, cx);
                            },
                        )),
                    )
                    .child(
                        button(&t, "compare-scope", "Compare", scope_on).on_click(cx.listener(
                            |this, _: &ClickEvent, _window, cx| {
                                this.stage_view.scope = PlotScope::Marked;
                                this.stage_view_changed(cx);
                            },
                        )),
                    ),
            )
            .child(if self.catalog.is_empty() {
                div().into_any_element()
            } else {
                let cmd = |id: &'static str,
                           label: &'static str,
                           enabled: bool,
                           action: fn(&mut Self, &mut Context<Self>)| {
                    div()
                        .id(id)
                        .px_1()
                        .rounded_sm()
                        .text_size(px(11.))
                        .text_color(if enabled { t.accent } else { t.text_muted })
                        .when(enabled, |d| d.cursor_pointer().hover(|d| d.bg(t.raised)))
                        .on_click(cx.listener(move |this, _: &ClickEvent, _window, cx| {
                            if enabled {
                                action(this, cx);
                            }
                        }))
                        .child(label)
                };
                div()
                    .px_2()
                    .pb_1()
                    .flex()
                    .items_center()
                    .gap_1()
                    .text_size(px(11.))
                    .text_color(t.text_muted)
                    .child("mark:")
                    .flex_wrap()
                    .child(cmd(
                        "sel-scan",
                        "scan",
                        self.selected.is_some(),
                        |this, cx| this.select_active_scan(cx),
                    ))
                    .child(cmd(
                        "sel-tenth",
                        "every 10th",
                        self.selection.len() > 1,
                        |this, cx| this.thin_selection(cx),
                    ))
                    .child(cmd(
                        "sel-filter",
                        "filter",
                        self.filtered.is_some(),
                        |this, cx| this.select_filter_results(cx),
                    ))
                    .child(cmd(
                        "sel-freeze",
                        if processing_locked(self.current_group_index(), &self.frozen) {
                            "Unlock processing"
                        } else {
                            "Lock processing"
                        },
                        self.current_group_index().is_some(),
                        |this, cx| this.toggle_frozen(cx),
                    ))
                    .child(div().flex_1())
                    .child(cmd(
                        "sel-clear",
                        "clear",
                        !self.selection.is_empty(),
                        |this, cx| this.clear_selection(cx),
                    ))
                    .into_any_element()
            })
            .child(
                div()
                    .id("groups-width-handle")
                    .absolute()
                    .right_0()
                    .top_0()
                    .bottom_0()
                    .w(px(5.))
                    .cursor(gpui::CursorStyle::ResizeLeftRight)
                    .hover(|d| d.bg(t.accent))
                    .on_mouse_down(
                        gpui::MouseButton::Left,
                        cx.listener(|this, ev: &gpui::MouseDownEvent, _, cx| {
                            this.groups_resize =
                                Some((ev.position.x, this.structure.settings.groups_panel_width()));
                            cx.stop_propagation();
                        }),
                    )
                    .on_drag(SidebarResize, |_, _, _, cx| cx.new(|_| SidebarResize))
                    .on_drag_move::<SidebarResize>(cx.listener(
                        |this, ev: &gpui::DragMoveEvent<SidebarResize>, _, cx| {
                            if let Some((start, width)) = this.groups_resize {
                                this.structure.settings.groups_panel_width =
                                    Some(crate::settings::clamp_groups_panel_width(
                                        width + f32::from(ev.event.position.x - start),
                                    ));
                                cx.notify();
                            }
                        },
                    ))
                    .on_mouse_up(
                        gpui::MouseButton::Left,
                        cx.listener(|this, _, _, cx| this.finish_groups_resize(cx)),
                    )
                    .on_mouse_up_out(
                        gpui::MouseButton::Left,
                        cx.listener(|this, _, _, cx| this.finish_groups_resize(cx)),
                    ),
            )
    }

    /// One virtualized scroll surface for every source, channel and result.
    pub(crate) fn file_list(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let entity = cx.entity();
        let rows = self.group_rows();
        uniform_list("catalog-groups", rows.row_count(), move |range, _, app| {
            entity.update(app, |this, cx| {
                range
                    .filter_map(|i| rows.row_at(i))
                    .map(|row| {
                        if let Row::Header(_) = row {
                            div()
                                .h(px(27.))
                                .px_3()
                                .flex()
                                .items_center()
                                .text_color(this.theme.text_muted)
                                .child("Results")
                                .into_any_element()
                        } else {
                            this.group_row(row, cx).into_any_element()
                        }
                    })
                    .collect()
            })
        })
        .track_scroll(&self.file_scroll)
        .flex_1()
        .min_h_0()
    }

    fn group_row(&self, row: Row, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let t = self.theme;
        let ix = row.group().unwrap_or(crate::app::NO_ENTRY);
        let real = ix != crate::app::NO_ENTRY;
        let active = self.current_group_index() == Some(ix);
        let child = matches!(row, Row::Child { .. });
        let (expanded, extra) = match row {
            Row::Primary {
                expanded,
                extra_channels,
                ..
            } => (expanded, extra_channels),
            _ => (false, 0),
        };
        let derived = ix
            .checked_sub(DERIVED_BASE)
            .and_then(|i| self.derived.get(i));
        let path = derived
            .and_then(|d| d.source.clone())
            .or_else(|| (ix < self.catalog.len()).then(|| self.catalog.path(ix)))
            .or_else(|| {
                (!real)
                    .then(|| self.standalone_path().map(ToOwned::to_owned))
                    .flatten()
            });
        let mode = self.group_row_mode(ix);
        let tag = group_rows::kind(
            mode,
            if matches!(row, Row::Result { .. }) {
                derived
            } else {
                None
            },
        );
        let full_label = self.entry_label(ix);
        let label = self.group_row_label(row);
        let path_label = path
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_default();
        let id = self.peek_group_id(ix);
        let problems = id
            .as_ref()
            .map(|id| {
                self.group_diagnostics
                    .get(id, self.effective_fingerprint(ix))
            })
            .unwrap_or_default();
        let missing = derived
            .and_then(|d| group_rows::input_missing(d, |id| self.group_registry.is_excluded(id)));
        let changed = derived.and_then(|d| self.inputs_changed(d));
        let has_problems = !problems.is_empty() || missing.is_some() || changed.is_some();
        let error = problems
            .iter()
            .any(|p| p.severity == crate::app::ProblemSeverity::Error);
        let locked = self.frozen.contains(&ix);
        let mut detail = format!("{full_label}\n{path_label}\n{}", mode.label());
        for problem in problems {
            detail.push_str(&format!("\n{}", problem.message));
        }
        if let Some(missing) = &missing {
            detail.push_str(&format!("\n{missing}"));
        }
        if let Some(changed) = &changed {
            detail.push_str(&format!("\n{changed}"));
        }
        if locked {
            detail.push_str("\nProcessing locked");
        }
        if let Some(op) = derived.and_then(|d| d.operation.as_ref()) {
            detail.push_str(&format!("\n{} inputs", op.inputs.len()));
        }
        let tip = RowTooltip {
            text: detail,
            theme: t,
        };
        let tag = if child { "" } else { tag };
        let marked_children = if extra > 0 && !expanded {
            self.derived
                .iter()
                .enumerate()
                .filter(|(i, d)| {
                    DERIVED_BASE + i != ix
                        && d.source.is_some()
                        && d.source == path
                        && self.selection.contains(&(DERIVED_BASE + i))
                })
                .count()
        } else {
            0
        };
        let suffix = if marked_children > 0 {
            format!("+{extra} · {marked_children}✓")
        } else if extra > 0 && !expanded {
            format!("+{extra}")
        } else {
            String::new()
        };
        // Monospace labels let us budget for the fixed slots and ASCII tags and
        // middle-truncate names while retaining their identifying suffix.
        let reserved = 100.
            + if child { 12. } else { 0. }
            + (tag.chars().count() + suffix.chars().count()) as f32 * 6.5
            + if has_problems { 16. } else { 0. }
            + if locked { 16. } else { 0. };
        let label = middle_truncate(
            &label,
            ((self.structure.settings.groups_panel_width() - reserved) / 7.2).max(2.) as usize,
        );
        div()
            .id(("group", ix))
            .h(px(27.))
            .w_full()
            .min_w_0()
            .px_1p5()
            .when(child, |d| d.pl(px(18.)))
            .flex()
            .items_center()
            .gap_1()
            .rounded_md()
            .border_1()
            .border_color(if self.focus_group == Some(ix) {
                t.accent
            } else {
                gpui::Rgba { a: 0., ..t.border }
            })
            .cursor_pointer()
            .when(active, |d| {
                d.bg(gpui::Rgba {
                    a: 0.16,
                    ..t.accent
                })
            })
            .when(!active, |d| d.hover(|d| d.bg(t.raised)))
            .tooltip(move |_, cx| cx.new(|_| tip.clone()).into())
            .on_mouse_down(
                gpui::MouseButton::Right,
                cx.listener(move |this, ev: &gpui::MouseDownEvent, window, cx| {
                    cx.stop_propagation();
                    this.open_group_menu(ix, ev.position, window, cx);
                }),
            )
            .on_click(cx.listener(move |this, ev: &ClickEvent, window, cx| {
                window.focus(&this.data_focus, cx);
                this.click_group(ix, ev.modifiers(), cx);
            }))
            .child(
                div()
                    .id("disclosure")
                    .w(px(12.))
                    .flex_none()
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        cx.stop_propagation();
                        if extra > 0
                            && let Some(id) = this.stack_id(ix)
                        {
                            this.expanded_sources.insert(id, !expanded);
                            cx.notify();
                        }
                    }))
                    .child(if extra == 0 {
                        ""
                    } else if expanded {
                        "▾"
                    } else {
                        "▸"
                    }),
            )
            .child(
                div()
                    .id("mark")
                    .flex_none()
                    .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                        cx.stop_propagation();
                        window.focus(&this.data_focus, cx);
                        this.toggle_mark(ix, cx);
                    }))
                    .child(checkbox(&t, self.selection.contains(&ix))),
            )
            .child(swatch(trace_rgba(&t, self.group_color_index(ix))))
            .child(self.group_label_editor(ix, label, cx))
            .child(
                div()
                    .flex_none()
                    .text_size(px(10.5))
                    .text_color(t.text_muted)
                    .child(suffix),
            )
            .child(
                div()
                    .flex_none()
                    .text_size(px(10.5))
                    .text_color(t.text_muted)
                    .child(tag),
            )
            .when(has_problems, |d| {
                d.child(
                    div()
                        .flex_none()
                        .text_color(if error { t.error } else { t.warn })
                        .child(if error { "!" } else { "⚠" }),
                )
            })
            .when(locked, |d| {
                d.child(div().flex_none().text_size(px(11.)).child("🔒"))
            })
            .child(
                div()
                    .id("group-more")
                    .w(px(12.))
                    .flex_none()
                    .child("⋯")
                    .on_mouse_down(
                        gpui::MouseButton::Left,
                        cx.listener(move |this, ev: &gpui::MouseDownEvent, window, cx| {
                            cx.stop_propagation();
                            this.open_group_menu(ix, ev.position, window, cx);
                        }),
                    )
                    .on_click(|_, _, cx| cx.stop_propagation()),
            )
    }

    pub(crate) fn scan_list(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let t = self.theme;
        let entity = cx.entity();
        let active = self.active_scan;
        let expanded_scan = self.expanded_scan;
        let members = self.interaction_rows();
        let expanded = expanded_scan.and_then(|scan_ix| {
            self.catalog
                .scans
                .get(scan_ix)
                .map(|_| (scan_ix, members.row_count()))
        });
        let count = self.catalog.scans.len() + expanded.map(|(_, len)| len).unwrap_or(0);
        uniform_list("catalog-scans", count, move |range, _window, app| {
            let mut rows = Vec::with_capacity(range.len());
            for row in range {
                let Some(item) = scan_list_row(row, entity.read(app).catalog.scans.len(), expanded)
                else {
                    continue;
                };
                match item {
                    ScanListRow::Header(scan_ix) => {
                        let (label, meta): (SharedString, SharedString) = {
                            let scan = &entity.read(app).catalog.scans[scan_ix];
                            (
                                scan.label.clone().into(),
                                format!("folder run · {} files", scan.len).into(),
                            )
                        };
                        let is_active = active == Some(scan_ix);
                        let is_expanded = expanded_scan == Some(scan_ix);
                        let row_entity = entity.clone();
                        let button_entity = entity.clone();
                        rows.push(
                            div()
                                .id(("scan-header", scan_ix))
                                .h(px(27.))
                                .mx_1p5()
                                .px_1p5()
                                .gap_1p5()
                                .flex()
                                .items_center()
                                .rounded_md()
                                .overflow_hidden()
                                .when(is_active, |d| {
                                    d.bg(gpui::Rgba {
                                        a: 0.16,
                                        ..t.accent
                                    })
                                })
                                .when(!is_active, |d| d.hover(|d| d.bg(t.raised)))
                                .cursor_pointer()
                                .on_click(move |ev: &ClickEvent, window, app| {
                                    let modifiers = ev.modifiers();
                                    let double = ev.click_count() >= 2;
                                    row_entity.update(app, |this, cx| {
                                        window.focus(&this.data_focus, cx);
                                        this.focus_group = None;
                                        if modifiers.shift || modifiers.platform {
                                            this.select_scan_range(scan_ix, cx);
                                        } else if double {
                                            this.open_scan(scan_ix, cx);
                                        } else {
                                            this.active_scan = Some(scan_ix);
                                            this.expanded_scan = (this.expanded_scan
                                                != Some(scan_ix))
                                            .then_some(scan_ix);
                                            cx.notify();
                                        }
                                    });
                                })
                                .child(div().text_color(t.text_muted).child(if is_expanded {
                                    "▾"
                                } else {
                                    "▸"
                                }))
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .overflow_hidden()
                                        .whitespace_nowrap()
                                        .text_ellipsis()
                                        .child(label),
                                )
                                .child(
                                    div()
                                        .flex_none()
                                        .font_family(MONO)
                                        .text_size(px(10.5))
                                        .text_color(t.accent)
                                        .child(meta),
                                )
                                .child(
                                    div()
                                        .id(("scan-series", scan_ix))
                                        .flex_none()
                                        .px_1()
                                        .rounded_sm()
                                        .text_size(px(10.5))
                                        .text_color(t.accent)
                                        .border_1()
                                        .border_color(t.border)
                                        .hover(|d| d.bg(t.surface))
                                        .cursor_pointer()
                                        .on_click(move |_: &ClickEvent, _window, app| {
                                            app.stop_propagation();
                                            button_entity.update(app, |this, cx| {
                                                this.open_scan(scan_ix, cx)
                                            });
                                        })
                                        .child("open"),
                                )
                                .into_any_element(),
                        );
                    }
                    ScanListRow::Member { offset, .. } => {
                        if let Some(row) = members.row_at(offset) {
                            rows.push(entity.update(app, |this, cx| {
                                this.group_row(row, cx).into_any_element()
                            }));
                        }
                    }
                }
            }
            rows
        })
        .track_scroll(&self.scan_scroll)
        .flex_1()
        .min_h_0()
    }

    pub(crate) fn data_tab_button(
        &self,
        id: &'static str,
        label: &'static str,
        tab: DataTab,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let t = self.theme;
        let active = self.data_tab == tab;
        div()
            .id(id)
            .flex_1()
            .py_1()
            .flex()
            .justify_center()
            .text_size(px(11.5))
            .cursor_pointer()
            .when(active, |d| {
                d.text_color(t.accent).border_b_2().border_color(t.accent)
            })
            .when(!active, |d| d.text_color(t.text_muted))
            .hover(|d| d.bg(t.raised))
            .on_click(cx.listener(move |this, _: &ClickEvent, _window, cx| {
                this.data_tab = tab;
                cx.notify();
            }))
            .child(label)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::group_identity::{GroupId, GroupRegistry};
    use crate::params::DetectionMode;
    use std::{collections::BTreeMap, path::PathBuf};

    #[test]
    fn standalone_lock_survives_capture_restore_migration_and_unlock() {
        use crate::app::NO_ENTRY;
        use crate::group_identity::GroupState;
        use std::collections::BTreeSet;
        let registry = GroupRegistry::default();
        let id = registry.register_source(
            None,
            "/standalone.dat".into(),
            DetectionMode::Auto,
            &BTreeMap::new(),
        );
        let mut state = GroupState::default();
        let mut frozen = BTreeSet::from([NO_ENTRY]);
        assert!(processing_locked(Some(NO_ENTRY), &frozen));
        assert!(!processing_locked(None, &frozen));
        state.capture(&registry, &BTreeSet::new(), &frozen, &BTreeMap::new(), None);
        state.capture_standalone_lock(&registry, &id, frozen.contains(&NO_ENTRY));
        let mut restored: GroupState =
            serde_json::from_str(&serde_json::to_string(&state).unwrap()).unwrap();
        assert!(restored.frozen.contains(&id));
        frozen.clear();
        restore_standalone_lock(&mut frozen, &restored, &id);
        assert!(processing_locked(Some(NO_ENTRY), &frozen));
        restore_standalone_lock(&mut frozen, &restored, &GroupId::new_result());
        assert!(!processing_locked(Some(NO_ENTRY), &frozen));
        restore_standalone_lock(&mut frozen, &restored, &id);
        // An unlock must remove the otherwise retained unresolved identity.
        restored.capture_standalone_lock(&registry, &id, false);
        assert!(restored.frozen.is_empty());
        registry.register_source(
            Some(3),
            "/standalone.dat".into(),
            DetectionMode::Auto,
            &BTreeMap::new(),
        );
        migrate_standalone_lock(&mut frozen, 3);
        assert_eq!(frozen, BTreeSet::from([3]));
        assert!(processing_locked(Some(3), &frozen));
        assert_eq!(registry.indices(&state.frozen), frozen);
        state.capture(&registry, &BTreeSet::new(), &frozen, &BTreeMap::new(), None);
        state.capture_standalone_lock(&registry, &id, false);
        assert!(
            state.frozen.contains(&id),
            "catalog lock survives retiring standalone adapter"
        );
    }

    #[test]
    fn standalone_and_child_labels_use_durable_display_metadata() {
        let id = GroupId::source(std::path::Path::new("/standalone.dat"), DetectionMode::Auto);
        let mut state = crate::group_identity::GroupState::default();
        for default in ["standalone.dat", "Fluorescence"] {
            assert_eq!(state.display_label(Some(&id), || default.into()), default);
        }
        state.labels.insert(id.clone(), "Cu foil".into());
        for default in ["standalone.dat", "Fluorescence", "Fluorescence · file.dat"] {
            assert_eq!(state.display_label(Some(&id), || default.into()), "Cu foil");
        }
        assert_eq!(
            state.display_label(Some(&GroupId::new_result()), || "other".into()),
            "other"
        );
    }

    #[test]
    fn standalone_swatch_matches_the_plotted_durable_identity() {
        let registry = GroupRegistry::default();
        let origins = BTreeMap::new();
        let path = (0..100)
            .map(|i| PathBuf::from(format!("/standalone/{i}.dat")))
            .find(|path| group_rows::color_index(&GroupId::source(path, DetectionMode::Auto)) != 0)
            .expect("fixture must exercise a nonzero swatch");
        let plot_id = registry.register_source(None, path.clone(), DetectionMode::Auto, &origins);
        assert!(
            registry.id(crate::app::NO_ENTRY).is_none(),
            "standalone has no catalog adapter index"
        );
        let row_id = registry.peek_source(&path, DetectionMode::Auto, &origins);
        assert_eq!(
            group_rows::color_index(&row_id),
            group_rows::color_index(&plot_id)
        );
        assert_ne!(group_rows::color_index(&row_id), 0);
        // Editing the mode must keep the original durable colour.
        assert_eq!(
            registry.peek_source(&path, DetectionMode::Reference, &origins),
            plot_id
        );
    }
}
