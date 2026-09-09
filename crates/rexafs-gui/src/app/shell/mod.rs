//! Stage-based shell (doc/gui-ux-design-v2.md): the pipeline is the
//! navigation. A stage strip across the top selects *the* plot and *the*
//! parameters; groups live on the left, the inspector on the right.
//!
//! These are child modules of `app` so they can render straight off the
//! private `StudioApp` state without a pub(crate) field explosion; `app.rs`
//! keeps state and jobs, the shell keeps presentation.

pub(crate) mod assistant;
pub(crate) mod assistant_actions;
mod assistant_receipts;
pub(crate) mod assistant_shell;
mod assistant_state;
mod bond_geometry;
pub mod center;
mod chrome;
pub(crate) mod controls;
mod depth_controls;
pub mod fit;
pub(crate) mod fit_preview;
pub mod fit_workspace;
pub(crate) mod group_menu;
pub mod groups_panel;
pub mod handles;
pub(crate) mod help;
pub(crate) mod import_editor;
mod import_receipt;
mod import_review;
mod import_summary;
pub mod inspector;
mod joint_browser;
pub(crate) mod joint_fit;
pub mod journal;
mod marked_removal;
mod molecular_geometry;
pub mod molecule_view;
pub mod palette;
pub(crate) mod parameter_actions;
mod path_diagnostics;
pub mod path_picker;
pub(crate) mod path_routing;
pub(crate) mod publish;
pub mod series;
mod spectrum_colors;
pub mod stage_strip;
mod structure_depth;
pub mod structure_view;
pub mod thumbnails;
pub mod tools;
pub(crate) mod updates_view;

use gpui::{
    ClickEvent, Context, IntoElement, ParentElement, SharedString, Styled, div, prelude::*, px,
};

use super::{StudioApp, Workspace};
use crate::theme::Theme;

/// Pipeline stages, in pipeline order. The number is the ⌘-shortcut.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Stage {
    Data,
    Normalize,
    Background,
    Transform,
    Fit,
    Series,
    Publish,
}

impl Stage {
    pub const ALL: [Stage; 7] = [
        Stage::Data,
        Stage::Normalize,
        Stage::Background,
        Stage::Transform,
        Stage::Fit,
        Stage::Series,
        Stage::Publish,
    ];

    pub fn number(self) -> usize {
        Self::ALL.iter().position(|s| *s == self).unwrap_or(0) + 1
    }

    pub fn name(self) -> &'static str {
        match self {
            Stage::Data => "Data",
            Stage::Normalize => "Normalize",
            Stage::Background => "Background",
            Stage::Transform => "Transform",
            Stage::Fit => "Fit",
            Stage::Series => "Series",
            Stage::Publish => "Publish",
        }
    }

    /// Legacy workspace the stage maps onto: the four processing stages share
    /// the explore plot machinery, Fit and Series keep their centers.
    pub fn workspace(self) -> Workspace {
        match self {
            Stage::Fit => Workspace::Fit,
            Stage::Series => Workspace::Operando,
            _ => Workspace::Explore,
        }
    }

    pub fn is_processing(self) -> bool {
        matches!(
            self,
            Stage::Data | Stage::Normalize | Stage::Background | Stage::Transform
        )
    }
}

/// Which groups the stage plots show (Athena: current vs marked).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PlotScope {
    Current,
    Marked,
}

/// μ(E)-family quantity for the Data / Normalize plot.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EQuantity {
    Mu,
    Norm,
    Flat,
}

/// Background stage main view.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BkgView {
    /// μ(E) with the AUTOBK spline over |χ(R)| with the R < Rbkg region.
    Energy,
    /// k-weighted χ(k) over |χ(R)|.
    K,
}

/// Transform stage main view.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TfView {
    K,
    R,
    Q,
    Both,
}

/// Fit stage main view.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FitView {
    Both,
    K,
    R,
    Q,
}

/// Per-stage view state that is presentation only (never persisted).
#[derive(Clone, Copy, Debug)]
pub struct StageView {
    pub scope: PlotScope,
    pub e_quantity: EQuantity,
    pub thumbnail_focus: Option<usize>,
    pub bkg_view: BkgView,
    pub tf_view: TfView,
    pub show_bkg: bool,
    pub show_re: bool,
    pub fit_view: FitView,
    pub fit_step: fit_workspace::FitStep,
    pub fit_model_tab: usize,
    pub fit_result_tab: usize,
    pub fit_show_paths: bool,
    pub fit_show_re: bool,
    pub fit_show_im: bool,
    pub fit_show_batch: bool,
    pub series_space: crate::app::SeriesSpace,
}

impl Default for StageView {
    fn default() -> Self {
        Self {
            scope: PlotScope::Current,
            e_quantity: EQuantity::Norm,
            thumbnail_focus: None,
            bkg_view: BkgView::Energy,
            tf_view: TfView::Both,
            show_bkg: true,
            show_re: false,
            fit_view: FitView::Both,
            fit_step: fit_workspace::FitStep::Structure,
            fit_model_tab: 0,
            fit_result_tab: 0,
            fit_show_paths: true,
            fit_show_re: false,
            fit_show_im: false,
            fit_show_batch: false,
            series_space: crate::app::SeriesSpace::Energy,
        }
    }
}

/// Status dot semantics shared by the stage strip and thumbnails.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StageStatus {
    Ok,
    Auto,
    Attention,
    Idle,
}

impl StageStatus {
    pub fn color(self, t: &Theme) -> gpui::Rgba {
        match self {
            StageStatus::Ok => t.success,
            StageStatus::Auto => t.accent,
            StageStatus::Attention => t.warn,
            StageStatus::Idle => t.text_muted,
        }
    }
}

/// Monospace face for numbers (tabular) in the chrome.
pub use crate::theme::MONO;

/// Uppercase, letter-spaced section label used by every panel.
pub fn section_label(t: &Theme, text: impl Into<SharedString>) -> impl IntoElement {
    div()
        .text_size(px(10.5))
        .text_color(t.text_muted)
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .child(text.into().to_uppercase())
}

/// Bordered pill toggle. Filled with the soft accent when on.
pub fn chip(
    t: &Theme,
    id: impl Into<gpui::ElementId>,
    label: impl Into<SharedString>,
    on: bool,
) -> crate::accessibility::Control {
    let label = label.into();
    crate::accessibility::Control::new(div().id(id), label.clone(), accesskit::Role::Button)
        .selected(on)
        .tab_index(0)
        .key_context("Control")
        .h(px(22.))
        .px_2()
        .flex()
        .items_center()
        .rounded_full()
        .text_size(px(11.5))
        .cursor_pointer()
        .border_1()
        .whitespace_nowrap()
        .when(on, |d| {
            d.bg(gpui::Rgba {
                a: 0.16,
                ..t.accent
            })
            .border_color(t.accent)
            .text_color(t.text)
        })
        .when(!on, |d| d.border_color(t.border).text_color(t.text_muted))
        .hover(|d| d.bg(t.raised))
        .focus(|d| d.border_color(t.accent))
        .child(label)
}

/// One button of a segmented control.
pub fn segment(
    t: &Theme,
    id: impl Into<gpui::ElementId>,
    label: impl Into<SharedString>,
    on: bool,
    first: bool,
) -> crate::accessibility::Control {
    let label = label.into();
    crate::accessibility::Control::new(div().id(id), label.clone(), accesskit::Role::Tab)
        .selected(on)
        .tab_index(0)
        .key_context("Control")
        .h(px(24.))
        .px_2()
        .flex()
        .items_center()
        .text_size(px(11.5))
        .cursor_pointer()
        .whitespace_nowrap()
        .when(!first, |d| d.border_l_1().border_color(t.border))
        .when(on, |d| d.bg(t.accent).text_color(t.bg))
        .when(!on, |d| {
            d.text_color(t.text_muted).hover(|d| d.bg(t.raised))
        })
        .focus(|d| d.bg(t.raised).text_color(t.accent))
        .child(label)
}

/// Container for a row of [`segment`]s.
pub fn segmented(t: &Theme) -> gpui::Div {
    div()
        .flex()
        .items_center()
        .rounded_md()
        .border_1()
        .border_color(t.border)
        .bg(t.raised)
        .overflow_hidden()
}

/// Small bordered button.
pub fn button(
    t: &Theme,
    id: impl Into<gpui::ElementId>,
    label: impl Into<SharedString>,
    primary: bool,
) -> crate::accessibility::Control {
    let label = label.into();
    crate::accessibility::Control::new(div().id(id), label.clone(), accesskit::Role::Button)
        .tab_index(0)
        .key_context("Control")
        .h(px(24.))
        .px_2()
        .flex()
        .items_center()
        .gap_1()
        .rounded_md()
        .text_size(px(11.5))
        .font_weight(gpui::FontWeight::MEDIUM)
        .cursor_pointer()
        .whitespace_nowrap()
        .border_1()
        .when(primary, |d| {
            d.bg(t.accent).border_color(t.accent).text_color(t.bg)
        })
        .when(!primary, |d| {
            d.bg(t.raised)
                .border_color(t.border)
                .text_color(t.text)
                .hover(|d| d.border_color(t.accent))
        })
        .focus(|d| d.border_color(t.accent))
        .child(label)
}

impl StudioApp {
    /// Stage shortcuts must leave focus on a surviving element. A focused plot
    /// option can disappear in the next stage, stranding later key bindings.
    pub(crate) fn navigate_stage(
        &mut self,
        stage: Stage,
        window: &mut gpui::Window,
        cx: &mut Context<Self>,
    ) {
        self.root_focus.focus(window, cx);
        self.set_stage(stage, cx);
    }

    pub(crate) fn set_stage(&mut self, stage: Stage, cx: &mut Context<Self>) {
        let previous = self.stage;
        self.stage = stage;
        self.set_workspace(stage.workspace(), cx);
        if previous != stage {
            self.view.show_kwin = stage == Stage::Transform;
            self.stage_view_changed(cx);
        }
    }

    /// Stage view options feed the explore plot builders; changing them
    /// rebuilds the plots and re-syncs the drag handles.
    pub(crate) fn stage_view_changed(&mut self, cx: &mut Context<Self>) {
        self.stage_view.thumbnail_focus = None;
        self.invalidate_explore_plots(cx);
        self.sync_handles(cx);
        cx.notify();
    }

    /// The whole window.
    pub(crate) fn shell_root(&mut self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let t = self.theme;
        let center = match self.stage {
            Stage::Fit => self.fit_workspace(cx).into_any_element(),
            Stage::Series => self.series_stage_center(cx).into_any_element(),
            Stage::Publish => self.publish_panel(cx),
            _ => self.stage_center(cx).into_any_element(),
        };
        let groups = self
            .data_panel_open
            .then(|| self.groups_panel(cx).into_any_element());
        let inspector = (self.context_panel_open
            && (self.stage != Stage::Series || self.series_ready())
            && !matches!(self.stage, Stage::Fit | Stage::Publish))
        .then(|| self.inspector(cx).into_any_element());
        let assistant = self.assistant_panel(cx);
        div()
            .id("studio-shell")
            .on_mouse_move(cx.listener(|this, event: &gpui::MouseMoveEvent, _, cx| {
                this.resize_assistant(event, cx)
            }))
            .on_mouse_up(
                gpui::MouseButton::Left,
                cx.listener(|this, _, _, cx| this.finish_assistant_resize(cx)),
            )
            .on_mouse_up_out(
                gpui::MouseButton::Left,
                cx.listener(|this, _, _, cx| this.finish_assistant_resize(cx)),
            )
            .size_full()
            .min_h_0()
            .min_w_0()
            .relative()
            .flex()
            .flex_col()
            .bg(t.bg)
            .text_color(t.text)
            .text_size(px(12.5))
            .child(self.top_bar(cx))
            .child(self.stage_strip(cx))
            .children(self.import_receipt(cx))
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .min_w_0()
                    .flex()
                    .children(groups)
                    .child(
                        div()
                            .flex_1()
                            .min_h_0()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .children(self.stale_plots_banner(cx))
                            .child(center),
                    )
                    .children(inspector)
                    .children(assistant),
            )
            .children(
                self.problems_open
                    .then(|| self.problems_panel(cx).into_any_element()),
            )
            .children(
                self.journal
                    .open
                    .then(|| self.journal_panel(cx).into_any_element()),
            )
            .child(self.status_bar(cx))
            .children(self.group_menu_overlay(cx))
            .children(self.palette_overlay(cx))
            .children(self.parameter_menu_overlay(cx))
            .children(self.parameter_context_overlay(cx))
            .children(self.updates_overlay(cx))
            .children(self.help_overlay(cx))
            .children(self.chrome_menu_overlay(cx))
            .when(self.assistant_resizing.is_some(), |d| {
                d.child(
                    div()
                        .id("assistant-resize-capture")
                        .absolute()
                        .inset_0()
                        .cursor(gpui::CursorStyle::ResizeLeftRight)
                        .occlude()
                        .on_mouse_move(cx.listener(|this, event: &gpui::MouseMoveEvent, _, cx| {
                            this.resize_assistant(event, cx)
                        }))
                        .on_mouse_up(
                            gpui::MouseButton::Left,
                            cx.listener(|this, _, _, cx| this.finish_assistant_resize(cx)),
                        ),
                )
            })
            .children(self.path_route_overlay(cx))
            .children(self.import_editor.clone())
    }

    /// Project identity and a small set of familiar actions.
    fn top_bar(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        use self::controls::{Menu, icon_button};
        use crate::icons::Icon;
        let t = self.theme;
        let mut project = self
            .project_path
            .as_ref()
            .and_then(|p| p.file_stem())
            .or_else(|| self.source_dir.as_ref().and_then(|p| p.file_name()))
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| self.spectrum_label.to_string());
        if self.assistant_history_revision != self.assistant_history_saved_revision {
            project.push_str(" •");
        }
        let action = |id: &'static str,
                      glyph: Icon,
                      label: &'static str,
                      active: bool,
                      f: fn(&mut Self, &mut Context<Self>)| {
            icon_button(&t, id, glyph, label, active)
                .on_click(cx.listener(move |app, _, _, cx| f(app, cx)))
        };
        div()
            .h(px(38.))
            .min_w_0()
            .w_full()
            .flex_none()
            .flex()
            .items_center()
            .gap_1()
            .px_2()
            .bg(t.surface)
            .border_b_1()
            .border_color(t.border)
            .child(
                icon_button(
                    &t,
                    "project-menu",
                    Icon::Folder,
                    "Project",
                    self.ui.menu == Some(Menu::Project),
                )
                .on_click(cx.listener(|app, event, window, cx| {
                    app.open_chrome_menu(Menu::Project, event, window, cx)
                })),
            )
            .child(
                div()
                    .min_w_0()
                    .flex_1()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .child(if project.is_empty() {
                        "rexafs".into()
                    } else {
                        project
                    }),
            )
            .when(self.viewport_w >= 800., |d| {
                d.child(
                    action("undo", Icon::Undo, "Undo · ⌘Z", false, |a, c| a.undo(c))
                        .when(self.journal.undo.is_empty(), |d| {
                            d.disabled(true).opacity(0.4).tab_stop(false)
                        }),
                )
                .child(
                    action("redo", Icon::Redo, "Redo · ⇧⌘Z", false, |a, c| {
                        a.redo(c)
                    })
                    .when(self.journal.redo.is_empty(), |d| {
                        d.disabled(true).opacity(0.4).tab_stop(false)
                    }),
                )
            })
            .child(
                action(
                    "open-folder",
                    Icon::Import,
                    "Import files or folders · ⇧⌘O",
                    false,
                    |a, c| a.open_folder(c),
                )
                .w_auto()
                .px_2()
                .gap_1()
                .child("Import…"),
            )
            .child(action(
                "save-project",
                Icon::Save,
                "Save project · ⌘S",
                self.project_saving,
                |a, c| a.save_project(c),
            ))
            .child(action(
                "switch-theme",
                if t.mode == crate::theme::ThemeMode::Dark {
                    Icon::Sun
                } else {
                    Icon::Moon
                },
                if t.mode == crate::theme::ThemeMode::Dark {
                    "Switch to light theme"
                } else {
                    "Switch to dark theme"
                },
                false,
                |a, c| a.toggle_theme(c),
            ))
            .child(div().w(px(1.)).h(px(18.)).mx_1().bg(t.border))
            .child(
                icon_button(
                    &t,
                    "cmdk",
                    Icon::Search,
                    "Find actions, tools, or groups · ⌘K",
                    false,
                )
                .on_click(cx.listener(|app, _, window, cx| app.open_palette(window, cx))),
            )
            .child(action(
                "toggle-groups",
                Icon::PanelLeft,
                "Groups · ⌘B",
                self.data_panel_open,
                |a, c| {
                    a.data_panel_open = !a.data_panel_open;
                    a.fit_assistant_layout();
                    c.notify();
                },
            ))
            .when(!matches!(self.stage, Stage::Fit | Stage::Publish), |d| {
                d.child(action(
                    "toggle-inspector",
                    Icon::PanelRight,
                    "Parameters · ⌘J",
                    self.context_panel_open,
                    |a, c| {
                        a.context_panel_open = !a.context_panel_open;
                        a.fit_assistant_layout();
                        c.notify();
                    },
                ))
            })
            .child(action(
                "assistant-window",
                Icon::Chat,
                "Assistant",
                self.assistant_host == assistant_shell::AssistantHost::Docked,
                |a, c| a.open_assistant(c),
            ))
            .when(
                self.updates.result.as_ref().is_some_and(|r| r.available),
                |d| {
                    d.child(action(
                        "updates",
                        Icon::Download,
                        "Update available",
                        true,
                        |a, c| a.open_updates(c),
                    ))
                },
            )
            .child(action("help", Icon::Help, "Help", false, |a, c| {
                a.open_help(c)
            }))
    }
}
