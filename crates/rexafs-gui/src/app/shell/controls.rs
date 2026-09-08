//! Shared compact controls and transient disclosure state.
use gpui::{Context, FocusHandle, IntoElement, SharedString, Window, div, prelude::*, px};
use std::collections::BTreeSet;

use crate::{icons::Icon, theme::Theme};

#[derive(Default)]
pub(crate) struct Presentation {
    pub sections: BTreeSet<&'static str>,
    pub overview: bool,
    pub native_menu_blocked: Option<bool>,
    pub marked_removal: Option<super::marked_removal::MarkedRemoval>,
    pub merge_review: Option<crate::app::merge::MergeReview>,
    pub scan_picker: bool,
    pub menu: Option<Menu>,
    pub reverse_colors: bool,
    pub menu_focus: Option<FocusHandle>,
    pub return_focus: Option<FocusHandle>,
    pub menu_position: gpui::Point<gpui::Pixels>,
    pub save_storage: crate::project::DataStorage,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Menu {
    Project,
    Save,
    Plot,
    Colors,
    Groups,
    Structure,
    Merge,
    RemoveMarked,
}

pub(crate) fn icon(t: &Theme, icon: Icon) -> gpui::Svg {
    gpui::svg()
        .path(icon.path())
        .size(px(16.))
        .flex_none()
        .text_color(t.text_muted)
}

#[derive(Clone)]
pub(crate) struct Tooltip {
    pub label: SharedString,
    pub theme: Theme,
}

impl gpui::Render for Tooltip {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .px_2()
            .py_1()
            .max_w(px(340.))
            .rounded_md()
            .bg(self.theme.raised)
            .text_color(self.theme.text)
            .border_1()
            .border_color(self.theme.border)
            .shadow_md()
            .text_size(px(12.))
            .child(self.label.clone())
    }
}

/// Icon actions always retain a readable name on hover and keyboard focus.
pub(crate) fn icon_button(
    t: &Theme,
    id: impl Into<gpui::ElementId>,
    glyph: Icon,
    label: impl Into<SharedString>,
    selected: bool,
) -> crate::accessibility::Control {
    let label = label.into();
    let tip = Tooltip {
        label: label.clone(),
        theme: *t,
    };
    crate::accessibility::Control::new(div().id(id), label.clone(), accesskit::Role::Button)
        .tab_index(0)
        .key_context("Control")
        .size(px(28.))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .rounded_md()
        .border_1()
        .border_color(if selected {
            t.accent
        } else {
            gpui::Rgba { a: 0., ..t.accent }
        })
        .text_color(if selected { t.accent } else { t.text_muted })
        .when(selected, |d| d.bg(t.raised))
        .cursor_pointer()
        .hover(|d| d.bg(t.raised).text_color(t.text))
        .focus(|d| d.border_color(t.accent).bg(t.raised))
        .tooltip(move |_, cx| cx.new(|_| tip.clone()).into())
        .child(icon(t, glyph).text_color(if selected { t.accent } else { t.text_muted }))
}

pub(crate) fn disclosure(
    t: &Theme,
    id: impl Into<gpui::ElementId>,
    label: impl Into<SharedString>,
    open: bool,
    active: bool,
) -> crate::accessibility::Control {
    let label = label.into();
    crate::accessibility::Control::new(
        div().id(id),
        label.clone(),
        accesskit::Role::DisclosureTriangle,
    )
    .expanded(open)
    .tab_index(0)
    .key_context("Control")
    .min_h(px(30.))
    .px_2()
    .flex()
    .items_center()
    .gap_2()
    .rounded_md()
    .border_1()
    .border_color(gpui::transparent_black())
    .text_size(px(11.5))
    .text_color(t.text_muted)
    .cursor_pointer()
    .hover(|d| d.bg(t.raised))
    .focus(|d| d.border_color(t.accent))
    .child(icon(
        t,
        if open {
            Icon::ChevronDown
        } else {
            Icon::ChevronRight
        },
    ))
    .child(label)
    .child(div().flex_1())
    .when(active && !open, |d| {
        d.child(
            div()
                .text_size(px(10.5))
                .text_color(t.accent)
                .child("Modified"),
        )
    })
}

/// Let Tab reach controls in dialogs as well as the main workspace.
pub(crate) fn navigate(
    event: &gpui::KeyDownEvent,
    window: &mut Window,
    cx: &mut gpui::App,
) -> bool {
    if event.keystroke.key == "tab"
        && !event.keystroke.modifiers.control
        && !event.keystroke.modifiers.platform
        && !event.keystroke.modifiers.alt
    {
        if event.keystroke.modifiers.shift {
            window.focus_prev(cx);
        } else {
            window.focus_next(cx);
        }
        cx.stop_propagation();
        true
    } else {
        false
    }
}
