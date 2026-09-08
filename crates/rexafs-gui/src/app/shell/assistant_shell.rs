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

/// Collapse older panels first, and both when one is insufficient. The dock's
/// requested width remains intact; the centre retains at least 360 px at 1000 px.
pub(super) fn fit_assistant_panels(
    available: f32,
    width: f32,
    mut panels: PanelMemory,
    just_opened: Option<SidePanel>,
) -> PanelMemory {
    let order = if just_opened == Some(SidePanel::Inspector) {
        [SidePanel::Groups, SidePanel::Inspector]
    } else {
        [SidePanel::Inspector, SidePanel::Groups]
    };
    for panel in order {
        let center = available
            - clamp_assistant_width(width)
            - if panels.file_browser { 248. } else { 0. }
            - if panels.inspector { 312. } else { 0. };
        if center >= 360. {
            break;
        }
        match panel {
            SidePanel::Groups => panels.file_browser = false,
            SidePanel::Inspector => panels.inspector = false,
        }
    }
    panels
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
        assert_eq!(fit_assistant_panels(1400., 380., both, None), both);
        assert_eq!(
            fit_assistant_panels(1000., 380., both, Some(SidePanel::Groups)),
            PanelMemory {
                file_browser: true,
                inspector: false
            }
        );
        assert_eq!(
            fit_assistant_panels(1100., 380., both, Some(SidePanel::Inspector)),
            PanelMemory {
                file_browser: false,
                inspector: true
            }
        );
        // At 1000 even the inspector alone leaves only 308 px: collapse both.
        assert_eq!(
            fit_assistant_panels(1000., 380., both, Some(SidePanel::Inspector)),
            PanelMemory::default()
        );
        assert_eq!(
            fit_assistant_panels(1000., 640., both, None),
            PanelMemory::default()
        );
        assert_eq!(
            fit_assistant_panels(1000., 320., both, Some(SidePanel::Inspector)),
            PanelMemory {
                file_browser: false,
                inspector: true
            }
        );
    }
}
