//! Compact Assistant composer. Menus share keyboard navigation and keep the
//! existing model defaults, analysis permissions and command approval policy.
use super::*;
use crate::app::shell::controls::{Tooltip, icon};
use crate::icons::Icon;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ComposerMenu {
    Model,
    Reasoning,
    Access,
}
impl ComposerMenu {
    fn index(self) -> usize {
        match self {
            Self::Model => 0,
            Self::Reasoning => 1,
            Self::Access => 2,
        }
    }
    fn id(self) -> &'static str {
        match self {
            Self::Model => "assistant-model",
            Self::Reasoning => "assistant-reasoning",
            Self::Access => "assistant-access",
        }
    }
    fn title(self) -> &'static str {
        match self {
            Self::Model => "Model",
            Self::Reasoning => "Reasoning",
            Self::Access => "Access",
        }
    }
    fn row_height(self) -> f32 {
        if self == Self::Access {
            76.
        } else {
            MODEL_ROW_HEIGHT
        }
    }
}

struct Choice {
    label: String,
    detail: Option<&'static str>,
    selected: bool,
    enabled: bool,
}

impl AssistantWindow {
    fn composer_choices(&self, menu: ComposerMenu) -> Vec<Choice> {
        match menu {
            ComposerMenu::Model => std::iter::once(Choice {
                label: codex_client::resolved_model_label(&self.models, None).0,
                detail: None,
                selected: self.preferred_model.is_none(),
                enabled: true,
            })
            .chain(self.models.iter().map(|model| Choice {
                label: model.display_name.clone(),
                detail: None,
                selected: self.preferred_model.as_deref() == Some(model.model.as_str()),
                enabled: true,
            }))
            .collect(),
            ComposerMenu::Reasoning => {
                codex_client::effort_choices(self.model(), self.preferred_effort.as_deref())
                    .into_iter()
                    .map(|(_, label, selected)| Choice {
                        label,
                        selected,
                        detail: None,
                        enabled: true,
                    })
                    .collect()
            }
            ComposerMenu::Access => vec![
                Choice {
                    label: "Review".into(),
                    detail: Some("Inspect spectra and navigate the workspace."),
                    selected: !self.allow_changes,
                    enabled: true,
                },
                Choice {
                    label: "Edit analysis".into(),
                    detail: Some("Also change parameters and run calculations."),
                    selected: self.allow_changes,
                    enabled: true,
                },
                Choice {
                    label: "Workspace commands".into(),
                    detail: Some(
                        "Allow commands in the assistant sandbox, with approval when required.",
                    ),
                    selected: self.extended_access,
                    enabled: !self.connecting,
                },
            ],
        }
    }

    pub(super) fn close_model_picker(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let menu = self.composer_menu.take();
        if let Some(focus) = menu.and_then(|menu| self.controls_focus.get(&menu.id().into())) {
            focus.focus(window, cx);
        } else if self.controls().composer {
            self.input.read(cx).focus_handle(cx).focus(window, cx);
        }
        cx.notify();
    }

    fn open_composer_menu(
        &mut self,
        menu: ComposerMenu,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.controls().preferences {
            return;
        }
        self.settings_open = false;
        self.history_open = false;
        self.account_expanded = false;
        self.composer_menu = Some(menu);
        self.model_highlight = self
            .composer_choices(menu)
            .iter()
            .position(|choice| choice.selected)
            .unwrap_or(0);
        if let Some(focus) = self.controls_focus.get(&menu.id().into()) {
            focus.focus(window, cx);
        }
        self.model_scroll.scroll_to_item(self.model_highlight);
        self.model_scroll_pending.set(true);
        cx.notify();
    }

    fn choose_composer_option(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(menu) = self.composer_menu else {
            return;
        };
        if !self.controls().preferences
            || !self
                .composer_choices(menu)
                .get(index)
                .is_some_and(|choice| choice.enabled)
        {
            return;
        }
        match menu {
            ComposerMenu::Model => {
                let model = index
                    .checked_sub(1)
                    .and_then(|index| self.models.get(index))
                    .map(|model| model.model.clone());
                self.save_preferences(model, self.preferred_effort.clone(), cx);
            }
            ComposerMenu::Reasoning => {
                let choices =
                    codex_client::effort_choices(self.model(), self.preferred_effort.as_deref());
                self.save_preferences(self.preferred_model.clone(), choices[index].0.clone(), cx);
            }
            ComposerMenu::Access => match index {
                0 => {
                    self.allow_changes = false;
                    self.turn_edit = false;
                    self.deny_all_access("Edit analysis permission revoked");
                }
                1 => self.allow_changes = true,
                2 => self.access_preferences(true, cx),
                _ => return,
            },
        }
        self.close_model_picker(window, cx);
        if self.controls().composer {
            self.input.read(cx).focus_handle(cx).focus(window, cx);
        }
    }

    fn composer_menu_key(
        &mut self,
        menu: ComposerMenu,
        event: &gpui::KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let key = event.keystroke.key.as_str();
        if !model_picker_handles_key(self.composer_menu.is_some(), key) {
            return;
        }
        window.prevent_default();
        cx.stop_propagation();
        if !self.controls().preferences {
            return;
        }
        match key {
            "escape" => self.close_model_picker(window, cx),
            "enter" | "space" if self.composer_menu == Some(menu) => {
                self.choose_composer_option(self.model_highlight, window, cx)
            }
            "enter" | "space" => self.open_composer_menu(menu, window, cx),
            "up" | "down" => {
                if self.composer_menu != Some(menu) {
                    self.open_composer_menu(menu, window, cx);
                }
                let count = self.composer_choices(menu).len();
                self.model_highlight = if key == "up" {
                    self.model_highlight.saturating_sub(1)
                } else {
                    (self.model_highlight + 1).min(count.saturating_sub(1))
                };
                self.model_scroll.scroll_to_item(self.model_highlight);
                self.model_scroll_pending.set(true);
                cx.notify();
            }
            _ => {}
        }
    }

    fn composer_trigger(
        &self,
        menu: ComposerMenu,
        label: String,
        glyph: Icon,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        let t = self.theme;
        let open = self.composer_menu == Some(menu);
        let busy = !self.controls().preferences;
        let bounds = self.composer_trigger_bounds[menu.index()].clone();
        let entity = cx.entity().downgrade();
        let tip = Tooltip {
            label: if busy {
                "Available after this response".into()
            } else {
                match menu {
                    ComposerMenu::Model => format!(
                        "Model: {}",
                        codex_client::resolved_model_label(
                            &self.models,
                            self.preferred_model.as_deref()
                        )
                        .0
                    )
                    .into(),
                    ComposerMenu::Reasoning => format!(
                        "Reasoning: {}",
                        codex_client::effort_choices(
                            self.model(),
                            self.preferred_effort.as_deref()
                        )
                        .into_iter()
                        .find(|choice| choice.2)
                        .map(|choice| choice.1)
                        .unwrap_or_else(|| "Model default".into())
                    )
                    .into(),
                    ComposerMenu::Access => format!(
                        "Access: {label}{}",
                        if self.extended_access {
                            " · workspace commands on"
                        } else {
                            ""
                        }
                    )
                    .into(),
                }
            },
            theme: t,
        };
        let trigger = self
            .control(
                menu.id(),
                crate::accessibility::Control::new(
                    div().id(menu.id()),
                    format!("{}: {label}", menu.title()),
                    accesskit::Role::Button,
                ),
                !busy,
                cx.listener(move |this, _: &ClickEvent, window, cx| {
                    if this.composer_menu == Some(menu) {
                        this.close_model_picker(window, cx);
                    } else {
                        this.open_composer_menu(menu, window, cx);
                    }
                }),
            )
            .expanded(open)
            .h(px(30.))
            .text_size(px(13.))
            .px(px(7.))
            .min_w_0()
            .max_w_full()
            .flex()
            .items_center()
            .gap(px(5.))
            .rounded_lg()
            .border_1()
            .border_color(gpui::transparent_black())
            .text_color(if busy { t.text } else { t.text_muted })
            .when(busy, |d| d.opacity(0.7))
            .when(!busy, |d| d.cursor_pointer())
            .when(open, |d| d.bg(t.raised).text_color(t.text))
            .when(!busy, |d| d.hover(|d| d.bg(t.raised).text_color(t.text)))
            .tooltip(move |_, cx| cx.new(|_| tip.clone()).into())
            .on_key_down(cx.listener(move |this, event, window, cx| {
                this.composer_menu_key(menu, event, window, cx)
            }))
            .child(icon(&t, glyph).size(px(14.)))
            .child(
                div()
                    .min_w_0()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .child(label),
            )
            .when(menu == ComposerMenu::Access && self.extended_access, |d| {
                d.child(div().size(px(4.)).rounded_full().bg(t.warn).flex_none())
            })
            .child(icon(&t, Icon::ChevronDown).size(px(11.)));
        div()
            .min_w_0()
            .flex_shrink_0()
            .max_w(px(if menu == ComposerMenu::Model {
                190.
            } else {
                150.
            }))
            .child(trigger)
            .on_children_prepainted(move |children, window, _| {
                if let Some(trigger) = children.first() {
                    let previous = bounds.replace(*trigger);
                    if open && previous != *trigger {
                        let entity = entity.clone();
                        window.on_next_frame(move |_, cx| {
                            entity.update(cx, |_, cx| cx.notify()).ok();
                        });
                    }
                }
            })
    }

    pub(super) fn composer(
        &self,
        catalog_settled: bool,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        let t = self.theme;
        let model = self
            .model()
            .map(|model| model.display_name.clone())
            .unwrap_or_else(|| "Automatic".into());
        let reasoning =
            codex_client::effort_choices(self.model(), self.preferred_effort.as_deref())
                .into_iter()
                .find(|choice| choice.2)
                .map(|choice| {
                    if choice.0.is_some() {
                        choice.1
                    } else {
                        choice
                            .1
                            .strip_prefix("Model default (")
                            .and_then(|label| label.strip_suffix(')'))
                            .unwrap_or("Default")
                            .to_owned()
                    }
                })
                .unwrap_or_else(|| "Default".into());
        let access = if self.allow_changes {
            "Edit analysis"
        } else {
            "Review"
        };
        let stopping = self.transcript.busy || self.transcript.stop_pending;
        let enabled = if stopping {
            self.controls().stop
        } else {
            self.controls().send && !self.input.read(cx).text().trim().is_empty()
        };
        let send = self
            .control(
                if stopping {
                    "assistant-stop"
                } else {
                    "assistant-send"
                },
                crate::accessibility::Control::new(
                    div().id("assistant-submit"),
                    if stopping {
                        "Stop response"
                    } else {
                        "Send message"
                    },
                    accesskit::Role::Button,
                ),
                enabled,
                cx.listener(move |this, _: &ClickEvent, _, cx| {
                    if stopping {
                        this.stop(cx);
                    } else {
                        this.run(cx);
                    }
                }),
            )
            .on_key_down(
                cx.listener(move |this, event: &gpui::KeyDownEvent, window, cx| {
                    if enabled && matches!(event.keystroke.key.as_str(), "enter" | "space") {
                        window.prevent_default();
                        cx.stop_propagation();
                        if stopping {
                            this.stop(cx);
                        } else {
                            this.run(cx);
                        }
                    }
                }),
            )
            .size(px(32.))
            .flex_none()
            .rounded_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(if enabled { t.accent } else { t.border })
            .cursor_pointer()
            .hover(|d| d.bg(if enabled { t.accent } else { t.border }))
            .child(
                icon(&t, if stopping { Icon::Stop } else { Icon::ArrowUp })
                    .size(px(17.))
                    .text_color(if enabled {
                        gpui::rgb(0xffffff)
                    } else {
                        t.text_muted
                    }),
            );
        let warning =
            codex_client::resolved_model_label(&self.models, self.preferred_model.as_deref())
                .1
                .filter(|_| catalog_settled);
        div()
            .flex()
            .flex_col()
            .gap_1()
            .min_w_0()
            .flex_shrink_0()
            .when_some(warning, |d, warning| {
                d.child(div().text_size(px(12.)).text_color(t.warn).child(warning))
            })
            .child(
                div()
                    .id("assistant-composer")
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .p(px(10.))
                    .rounded(px(18.))
                    .bg(t.surface)
                    .border_1()
                    .border_color(if self.input.read(cx).focus_handle(cx).is_focused(window) {
                        t.accent
                    } else {
                        t.border
                    })
                    .child(self.input.clone())
                    .child(
                        div()
                            .flex()
                            .items_end()
                            .gap(px(4.))
                            .min_w_0()
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .flex()
                                    .flex_wrap()
                                    .items_center()
                                    .gap(px(2.))
                                    .child(self.composer_trigger(
                                        ComposerMenu::Model,
                                        model,
                                        Icon::Layers,
                                        cx,
                                    ))
                                    .child(self.composer_trigger(
                                        ComposerMenu::Reasoning,
                                        reasoning,
                                        Icon::Sliders,
                                        cx,
                                    ))
                                    .child(self.composer_trigger(
                                        ComposerMenu::Access,
                                        access.into(),
                                        if self.allow_changes {
                                            Icon::Unlock
                                        } else {
                                            Icon::Lock
                                        },
                                        cx,
                                    )),
                            )
                            .child(send),
                    ),
            )
    }

    pub(super) fn model_picker_overlay(&self, cx: &mut Context<Self>) -> Option<gpui::AnyElement> {
        let menu = self.composer_menu?;
        if !self.controls().preferences {
            return None;
        }
        let t = self.theme;
        let choices = self.composer_choices(menu);
        let count = choices.len();
        let row_height = menu.row_height();
        let trigger = self.composer_trigger_bounds[menu.index()].get();
        let available = (f32::from(trigger.top()) - 18.).max(0.);
        let height = (count.min(7) as f32 * row_height).min(available);
        let rendered_offset = self.model_scroll.offset().y;
        let (thumb_height, thumb_offset) =
            model_scrollbar(count, f32::from(rendered_offset), height);
        let scroll = self.model_scroll.clone();
        let pending = self.model_scroll_pending.clone();
        let highlight = self.model_highlight;
        let entity = cx.entity().downgrade();
        let mut list = div()
            .id("assistant-composer-options")
            .h(px(height))
            .flex_1()
            .min_w_0()
            .overflow_y_scroll()
            .track_scroll(&self.model_scroll)
            .on_scroll_wheel(cx.listener(|_, _: &gpui::ScrollWheelEvent, _, cx| {
                cx.stop_propagation();
                cx.notify();
            }))
            .flex()
            .flex_col();
        for (index, choice) in choices.into_iter().enumerate() {
            let id = match menu {
                ComposerMenu::Model => gpui::ElementId::from(("assistant-model-option", index)),
                ComposerMenu::Reasoning => ("assistant-effort", index).into(),
                ComposerMenu::Access => {
                    ["assistant-review", "assistant-edit", "assistant-extended"][index].into()
                }
            };
            let label = choice.label.clone();
            list = list.child(
                self.control(
                    id.clone(),
                    crate::accessibility::Control::new(
                        div().id(id),
                        label.clone(),
                        accesskit::Role::Button,
                    ),
                    choice.enabled,
                    cx.listener(move |this, _: &ClickEvent, window, cx| {
                        this.choose_composer_option(index, window, cx)
                    }),
                )
                .selected(choice.selected)
                .when_some(choice.detail, |d, detail| d.description(detail))
                .when(menu == ComposerMenu::Access && index == 2, |d| {
                    d.role(accesskit::Role::CheckBox)
                })
                .on_key_down(
                    cx.listener(move |this, event: &gpui::KeyDownEvent, window, cx| {
                        if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                            window.prevent_default();
                            cx.stop_propagation();
                            this.choose_composer_option(index, window, cx);
                        }
                    }),
                )
                .h(px(row_height))
                .flex_shrink_0()
                .px(px(10.))
                .rounded_md()
                .flex()
                .items_center()
                .gap_2()
                .cursor_pointer()
                .text_color(t.text)
                .when(index == self.model_highlight, |d| d.bg(t.raised))
                .hover(|d| d.bg(t.raised))
                .when(menu == ComposerMenu::Access && index == 2, |d| {
                    d.border_t_1().border_color(t.border)
                })
                .when(menu == ComposerMenu::Access, |d| {
                    d.child(
                        icon(&t, [Icon::Lock, Icon::Unlock, Icon::Sliders][index]).size(px(16.)),
                    )
                })
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap(px(4.))
                        .child(
                            div()
                                .whitespace_nowrap()
                                .overflow_hidden()
                                .text_ellipsis()
                                .child(label),
                        )
                        .when_some(choice.detail, |d, detail| {
                            d.child(
                                div()
                                    .text_size(px(11.))
                                    .line_height(px(15.))
                                    .text_color(t.text_muted)
                                    .child(detail),
                            )
                        }),
                )
                .child(div().w(px(16.)).flex_none().when(choice.selected, |d| {
                    d.child(icon(&t, Icon::Check).text_color(t.text))
                })),
            );
        }
        let popup = div()
            .on_children_prepainted(move |_, window, _| {
                let retry = pending.replace(false);
                if retry {
                    scroll.scroll_to_item(highlight);
                }
                if retry || scroll.offset().y != rendered_offset {
                    let entity = entity.clone();
                    window.on_next_frame(move |_, cx| {
                        entity.update(cx, |_, cx| cx.notify()).ok();
                    });
                }
            })
            .id("assistant-composer-popup")
            .on_key_down(
                cx.listener(move |this, event: &gpui::KeyDownEvent, window, cx| {
                    if matches!(event.keystroke.key.as_str(), "up" | "down" | "escape") {
                        if let Some(focus) = this.controls_focus.get(&menu.id().into()) {
                            focus.focus(window, cx);
                        }
                        this.composer_menu_key(menu, event, window, cx);
                    }
                }),
            )
            .w(px(if menu == ComposerMenu::Access {
                300.
            } else {
                280.
            }))
            .p(px(5.))
            .rounded_lg()
            .bg(t.surface)
            .border_1()
            .border_color(t.border)
            .shadow_lg()
            .flex()
            .gap(px(4.))
            .on_any_mouse_down(|_, window, cx| {
                window.prevent_default();
                cx.stop_propagation();
            })
            .child(list)
            .when(
                menu != ComposerMenu::Access && count as f32 * row_height > height,
                |d| {
                    d.child(
                        div()
                            .relative()
                            .w(px(4.))
                            .h(px(height))
                            .flex_none()
                            .rounded_full()
                            .bg(t.raised)
                            .child(
                                div()
                                    .absolute()
                                    .top(px(thumb_offset))
                                    .w(px(4.))
                                    .h(px(thumb_height))
                                    .rounded_full()
                                    .bg(t.text_muted),
                            ),
                    )
                },
            );
        Some(
            div()
                .id("assistant-composer-dismiss")
                .absolute()
                .inset_0()
                .occlude()
                .on_any_mouse_down(cx.listener(|this, _: &gpui::MouseDownEvent, window, cx| {
                    window.prevent_default();
                    cx.stop_propagation();
                    this.close_model_picker(window, cx);
                }))
                .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
                .child(
                    gpui::anchored()
                        .anchor(gpui::Corner::BottomLeft)
                        .position(trigger.origin)
                        .offset(gpui::point(px(0.), px(-6.)))
                        .snap_to_window()
                        .child(popup),
                )
                .into_any_element(),
        )
    }
}
