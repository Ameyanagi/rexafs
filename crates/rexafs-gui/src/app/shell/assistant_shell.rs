//! State decisions for the Assistant shell, independent of GPUI.
pub(crate) const DEFAULT_ASSISTANT_WIDTH: f32 = 380.;
pub(crate) fn clamp_assistant_width(width: f32) -> f32 {
    if width.is_finite() {
        width.clamp(320., 640.)
    } else {
        DEFAULT_ASSISTANT_WIDTH
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum AssistantHost {
    #[default]
    Closed,
    Docked,
    PoppedOut,
}
#[derive(Clone, Copy)]
pub(crate) enum HostAction {
    Toggle { prefer_docked: bool },
    PopOut,
    Dock,
    Close,
}
impl AssistantHost {
    pub(crate) fn transition(self, action: HostAction) -> Self {
        match action {
            HostAction::Toggle { prefer_docked } => match self {
                Self::Closed if !prefer_docked => Self::PoppedOut,
                Self::Closed | Self::PoppedOut => Self::Docked,
                Self::Docked => Self::Closed,
            },
            HostAction::PopOut => Self::PoppedOut,
            HostAction::Dock => Self::Docked,
            HostAction::Close => match self {
                Self::PoppedOut => Self::Docked,
                _ => Self::Closed,
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SidePanel {
    Groups,
    Inspector,
}

/// Fixed width of the Parameters panel, shared with its renderer.
pub(super) const PARAMETERS_WIDTH: f32 = 312.;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct AssistantLayout {
    pub panels: PanelMemory,
    /// Width for this frame; the user's preferred dock width is not changed.
    pub width: f32,
}

/// Keep the most recently opened panel visible. Collapse older panels first,
/// then narrow the Assistant if needed to leave 360 px for the plot. At the
/// Assistant's 320 px minimum, a smaller plot is preferable to hiding the panel
/// the user just opened. Use the actual Groups width, including user resizing.
pub(super) fn fit_assistant_panels(
    available: f32,
    width: f32,
    mut panels: PanelMemory,
    just_opened: Option<SidePanel>,
    groups_width: f32,
) -> AssistantLayout {
    let mut width = clamp_assistant_width(width);
    let panel_width = |panels: PanelMemory| {
        (if panels.file_browser {
            groups_width
        } else {
            0.
        }) + if panels.inspector {
            PARAMETERS_WIDTH
        } else {
            0.
        }
    };
    let order = if just_opened == Some(SidePanel::Inspector) {
        [SidePanel::Groups, SidePanel::Inspector]
    } else {
        [SidePanel::Inspector, SidePanel::Groups]
    };
    for panel in order {
        if available - width - panel_width(panels) >= 360. {
            break;
        }
        if Some(panel) == just_opened {
            continue;
        }
        match panel {
            SidePanel::Groups => panels.file_browser = false,
            SidePanel::Inspector => panels.inspector = false,
        }
    }
    width = width.min(clamp_assistant_width(
        available - panel_width(panels) - 360.,
    ));
    AssistantLayout { panels, width }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct PanelMemory {
    pub file_browser: bool,
    pub inspector: bool,
}
impl PanelMemory {
    pub fn toggle(&mut self, current: Self) -> Self {
        if !current.file_browser && !current.inspector {
            if !self.file_browser && !self.inspector {
                Self {
                    file_browser: true,
                    inspector: true,
                }
            } else {
                *self
            }
        } else {
            *self = current;
            Self::default()
        }
    }
}

pub(super) fn control_key_activates(key: &str, modified: bool) -> bool {
    !modified && matches!(key, "enter" | "space")
}

pub(super) fn model_picker_handles_key(open: bool, key: &str) -> bool {
    open || key != "escape"
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct ControlState {
    pub send: bool,
    pub composer: bool,
    pub preferences: bool,
    pub starters: bool,
    pub navigation: bool,
    pub stop: bool,
    pub copy: bool,
    pub close: bool,
}
impl ControlState {
    pub fn derive(analysis_open: bool, connected: bool, signed_in: bool, busy: bool) -> Self {
        let ready = analysis_open && connected && signed_in;
        Self {
            send: ready && !busy,
            composer: analysis_open,
            preferences: analysis_open && !busy,
            starters: ready && !busy,
            navigation: analysis_open,
            stop: ready && busy,
            copy: true,
            close: !analysis_open,
        }
    }
}

const STARTERS: [(&str, &str); 5] = [
    (
        "Check processing",
        "Check the current spectrum's processing. Review normalization, background subtraction, and transform settings; explain any concerns before proposing changes.",
    ),
    (
        "Review fit setup",
        "Review the current spectrum's fit setup, including paths, parameter constraints, and fit ranges. Explain any concerns before proposing changes.",
    ),
    (
        "Summarize latest fit",
        "Summarize the latest recorded fit for the current spectrum, including fit quality, important estimates, uncertainties, and limitations.",
    ),
    (
        "Explain this spectrum",
        "Explain the current spectrum and the active processing stage. Describe what the data can tell us and any limitations.",
    ),
    (
        "Suggest a fit model",
        "Suggest a fit model using the available paths for the current spectrum. Explain path choices and parameter constraints before proposing changes.",
    ),
];

pub(super) fn task_starters(
    spectrum: bool,
    paths: bool,
    fit: bool,
) -> Vec<(&'static str, &'static str)> {
    if !spectrum {
        return Vec::new();
    }
    let mut starters = vec![STARTERS[0]];
    if paths {
        starters.push(STARTERS[if fit { 1 } else { 4 }]);
    } else {
        starters.push(STARTERS[3]);
    }
    if fit {
        starters.push(STARTERS[2]);
    }
    starters
}

/// Compact connection status, omitting the email retained in the disclosure.
pub(super) fn account_status(label: &str) -> String {
    let parts: Vec<_> = label.split(" · ").collect();
    if parts.first() == Some(&"ChatGPT") && parts.len() >= 3 {
        format!("ChatGPT · {}", parts[parts.len() - 1])
    } else {
        parts.first().copied().unwrap_or_default().to_owned()
    }
}

pub(super) const ANALYSIS_CLOSED: &str = "Analysis closed — conversation kept for copying";

pub(super) fn empty_state_message(
    open: bool,
    connected: bool,
    signed_in: bool,
    spectrum: bool,
) -> &'static str {
    match (open, connected, signed_in, spectrum) {
        (false, _, _, _) => ANALYSIS_CLOSED,
        (_, false, _, _) => "Retry connection",
        (_, _, false, _) => "Sign in with Device login",
        (_, _, _, false) => "Open a spectrum in the analysis window",
        _ => "Ask about the current spectrum or review its processing.",
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum EscapeTarget {
    CloseMenu,
    Stop,
    FocusComposer,
}
pub(super) fn escape_target(menu_open: bool, busy: bool) -> EscapeTarget {
    match (menu_open, busy) {
        (true, _) => EscapeTarget::CloseMenu,
        (_, true) => EscapeTarget::Stop,
        _ => EscapeTarget::FocusComposer,
    }
}

pub(super) fn account_disclosure(label: &str, expanded: bool) -> &str {
    if expanded {
        label
    } else {
        label.split(" · ").next().unwrap_or(label)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn assistant_panels_restore_every_combination_and_manual_changes() {
        for file_browser in [false, true] {
            for inspector in [false, true] {
                let state = PanelMemory {
                    file_browser,
                    inspector,
                };
                let mut memory = state;
                let restored = if state == PanelMemory::default() {
                    PanelMemory {
                        file_browser: true,
                        inspector: true,
                    }
                } else {
                    state
                };
                assert_eq!(memory.toggle(PanelMemory::default()), restored);
                assert_eq!(memory.toggle(restored), PanelMemory::default());
                assert_eq!(memory.toggle(PanelMemory::default()), restored);
            }
        }
        let mut memory = PanelMemory {
            file_browser: true,
            inspector: false,
        };
        let manual = PanelMemory {
            file_browser: false,
            inspector: true,
        };
        assert_eq!(memory.toggle(manual), PanelMemory::default());
        assert_eq!(memory.toggle(PanelMemory::default()), manual);
    }
    #[test]
    fn assistant_control_activation_leaves_shortcuts_and_other_keys_alone() {
        for key in ["enter", "space"] {
            assert!(control_key_activates(key, false));
            assert!(!control_key_activates(key, true));
        }
        for key in ["escape", "tab", "up", "down", ".", "a"] {
            for modified in [false, true] {
                assert!(!control_key_activates(key, modified));
            }
        }
    }
    #[test]
    fn assistant_model_trigger_keeps_activation_and_escape_fallthrough() {
        for key in ["enter", "space", "up", "down"] {
            assert!(model_picker_handles_key(false, key));
            assert!(model_picker_handles_key(true, key));
        }
        assert!(!model_picker_handles_key(false, "escape"));
        assert!(model_picker_handles_key(true, "escape"));
    }
    #[test]
    fn assistant_controls_cover_all_states() {
        for open in [false, true] {
            for connected in [false, true] {
                for signed_in in [false, true] {
                    for busy in [false, true] {
                        let c = ControlState::derive(open, connected, signed_in, busy);
                        assert_eq!(c.send, open && connected && signed_in && !busy);
                        assert_eq!(c.starters, c.send);
                        assert_eq!(c.composer, open);
                        assert_eq!(c.preferences, open && !busy);
                        assert_eq!(c.stop, open && connected && signed_in && busy);
                        assert_eq!(c.navigation, open);
                        assert!(c.copy);
                        assert_eq!(c.close, !open);
                    }
                }
            }
        }
    }
    #[test]
    fn assistant_starters_are_distinct_editable_prompts() {
        for (i, (label, prompt)) in STARTERS.iter().enumerate() {
            assert!(!label.trim().is_empty() && !prompt.trim().is_empty());
            for (other_label, other_prompt) in &STARTERS[..i] {
                assert_ne!(label, other_label);
                assert_ne!(prompt, other_prompt);
            }
        }
    }
    #[test]
    fn assistant_starters_match_available_data() {
        for spectrum in [false, true] {
            for paths in [false, true] {
                for fit in [false, true] {
                    let starters = task_starters(spectrum, paths, fit);
                    assert!(starters.len() <= 3);
                    let labels: Vec<_> = starters.iter().map(|s| s.0).collect();
                    assert_eq!(labels.contains(&"Check processing"), spectrum);
                    assert_eq!(
                        labels.contains(&"Explain this spectrum"),
                        spectrum && !paths
                    );
                    assert_eq!(
                        labels.contains(&"Suggest a fit model"),
                        spectrum && paths && !fit
                    );
                    assert_eq!(
                        labels.contains(&"Review fit setup"),
                        spectrum && paths && fit
                    );
                    assert_eq!(labels.contains(&"Summarize latest fit"), spectrum && fit);
                }
            }
        }
    }
    #[test]
    fn assistant_account_status_keeps_plan_without_email() {
        assert_eq!(
            account_status("ChatGPT · user@example.com · pro"),
            "ChatGPT · pro"
        );
        assert_eq!(account_status("API key"), "API key");
        assert_eq!(account_status("ChatGPT"), "ChatGPT");
    }
    #[test]
    fn assistant_empty_state_priority() {
        assert_eq!(
            empty_state_message(false, false, false, false),
            ANALYSIS_CLOSED
        );
        assert_eq!(
            empty_state_message(true, false, false, false),
            "Retry connection"
        );
        assert_eq!(
            empty_state_message(true, true, false, false),
            "Sign in with Device login"
        );
        assert_eq!(
            empty_state_message(true, true, true, false),
            "Open a spectrum in the analysis window"
        );
        assert_eq!(
            empty_state_message(true, true, true, true),
            "Ask about the current spectrum or review its processing."
        );
    }
    #[test]
    fn assistant_escape_priority() {
        assert_eq!(escape_target(true, true), EscapeTarget::CloseMenu);
        assert_eq!(escape_target(true, false), EscapeTarget::CloseMenu);
        assert_eq!(escape_target(false, true), EscapeTarget::Stop);
        assert_eq!(escape_target(false, false), EscapeTarget::FocusComposer);
    }
    #[test]
    fn assistant_account_disclosure_preserves_details() {
        for label in ["ChatGPT · a@example.com · plus", "API key", "Signed in", ""] {
            assert_eq!(account_disclosure(label, true), label);
        }
        assert_eq!(
            account_disclosure("ChatGPT · a@example.com · plus", false),
            "ChatGPT"
        );
        assert_eq!(account_disclosure("API key", false), "API key");
    }
}

#[cfg(test)]
mod host_tests {
    use super::*;
    #[test]
    fn assistant_width_clamps() {
        for (input, expected) in [
            (0., 320.),
            (320., 320.),
            (380., 380.),
            (640., 640.),
            (900., 640.),
            (f32::NAN, 380.),
            (f32::INFINITY, 380.),
        ] {
            assert_eq!(clamp_assistant_width(input), expected);
        }
    }
    #[test]
    fn assistant_host_transitions() {
        use AssistantHost::*;
        assert_eq!(
            Closed.transition(HostAction::Toggle {
                prefer_docked: true
            }),
            Docked
        );
        assert_eq!(
            Closed.transition(HostAction::Toggle {
                prefer_docked: false
            }),
            PoppedOut
        );
        assert_eq!(Docked.transition(HostAction::PopOut), PoppedOut);
        assert_eq!(PoppedOut.transition(HostAction::Dock), Docked);
        assert_eq!(PoppedOut.transition(HostAction::Close), Docked);
        assert_eq!(Docked.transition(HostAction::Close), Closed);
        assert_eq!(
            Docked.transition(HostAction::Toggle {
                prefer_docked: true
            }),
            Closed
        );
        assert_eq!(
            PoppedOut.transition(HostAction::Toggle {
                prefer_docked: false
            }),
            Docked
        );
        assert_eq!(Closed.transition(HostAction::Close), Closed);
    }
    #[test]
    fn assistant_auto_collapse_preserves_recent_panel_when_possible() {
        let both = PanelMemory {
            file_browser: true,
            inspector: true,
        };
        assert_eq!(
            fit_assistant_panels(1400., 380., both, None, 248.).panels,
            both
        );
        assert_eq!(
            fit_assistant_panels(1000., 380., both, Some(SidePanel::Groups), 248.).panels,
            PanelMemory {
                file_browser: true,
                inspector: false
            }
        );
        assert_eq!(
            fit_assistant_panels(1100., 380., both, Some(SidePanel::Inspector), 248.).panels,
            PanelMemory {
                file_browser: false,
                inspector: true
            }
        );
        // Narrow the Assistant to 328 px instead of undoing Open Parameters.
        assert_eq!(
            fit_assistant_panels(1000., 380., both, Some(SidePanel::Inspector), 248.),
            AssistantLayout {
                panels: PanelMemory {
                    file_browser: false,
                    inspector: true
                },
                width: 328.,
            }
        );
        assert_eq!(
            fit_assistant_panels(1000., 640., both, None, 248.).panels,
            PanelMemory::default()
        );
        assert_eq!(
            fit_assistant_panels(1000., 320., both, Some(SidePanel::Inspector), 248.).panels,
            PanelMemory {
                file_browser: false,
                inspector: true
            }
        );
    }
    #[test]
    fn assistant_open_parameters_reclaims_space_and_keeps_the_preference() {
        let both = PanelMemory {
            file_browser: true,
            inspector: true,
        };
        let layout = fit_assistant_panels(1440., 580., both, Some(SidePanel::Inspector), 280.);
        assert_eq!(
            layout.panels,
            PanelMemory {
                file_browser: false,
                inspector: true
            }
        );
        assert_eq!(layout.width, 580.);

        // A wide Assistant used to prevent Parameters from opening at all.
        let narrow = fit_assistant_panels(1000., 640., both, Some(SidePanel::Inspector), 280.);
        assert_eq!(narrow.panels, layout.panels);
        assert_eq!(narrow.width, 328.);
        assert_eq!(1000. - narrow.width - PARAMETERS_WIDTH, 360.);
        let wider =
            fit_assistant_panels(1440., 640., narrow.panels, Some(SidePanel::Inspector), 280.);
        assert_eq!(wider.width, 640.);
        assert!(wider.panels.inspector);

        // The real, resizable Groups panel can be wider than its old estimate.
        let groups = fit_assistant_panels(1360., 640., both, Some(SidePanel::Groups), 400.);
        assert_eq!(
            groups.panels,
            PanelMemory {
                file_browser: true,
                inspector: false
            }
        );
        assert_eq!(groups.width, 600.);
    }

    #[test]
    fn assistant_requested_panel_survives_resizing_and_repeated_layout() {
        for available in [960., 1000., 1100., 1440., 1920.] {
            for requested in [320., 380., 580., 640.] {
                for groups_width in [240., 280., 400.] {
                    for panel in [SidePanel::Groups, SidePanel::Inspector] {
                        let layout = fit_assistant_panels(
                            available,
                            requested,
                            PanelMemory {
                                file_browser: true,
                                inspector: true,
                            },
                            Some(panel),
                            groups_width,
                        );
                        assert!(match panel {
                            SidePanel::Groups => layout.panels.file_browser,
                            SidePanel::Inspector => layout.panels.inspector,
                        });
                        assert!((320. ..=requested).contains(&layout.width));
                        let center = available
                            - layout.width
                            - if layout.panels.file_browser {
                                groups_width
                            } else {
                                0.
                            }
                            - if layout.panels.inspector {
                                PARAMETERS_WIDTH
                            } else {
                                0.
                            };
                        assert!(center >= if available >= 1100. { 360. } else { 240. });
                        // Studio and Assistant both resolve layout: the second pass must not hide anything.
                        assert_eq!(
                            fit_assistant_panels(
                                available,
                                requested,
                                layout.panels,
                                Some(panel),
                                groups_width
                            ),
                            layout
                        );
                    }
                }
            }
        }
    }
}
