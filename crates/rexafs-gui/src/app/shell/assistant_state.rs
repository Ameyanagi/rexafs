//! Pure transcript reducer. Protocol IDs are scoped to the current turn.
use super::assistant_receipts::Receipt;
use std::time::{Duration, Instant};

#[derive(Clone, Debug, PartialEq)]
pub(super) enum ActivityState {
    Running,
    Done,
    Stopped,
    Failed(String),
}
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Status {
    Preparing,
    Waiting { started: Instant },
    Stopped,
    Error(String),
    Reconnected,
}
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Entry {
    User(String, bool),
    Assistant {
        id: String,
        text: String,
    },
    Thinking {
        id: String,
        text: String,
    },
    Activity {
        id: String,
        label: String,
        tool: String,
        state: ActivityState,
    },
    Receipt(Receipt),
    Status(Status),
}
#[derive(Clone, Copy, Debug)]
pub(super) enum ItemKind {
    Assistant,
    Thinking,
}
#[derive(Debug)]
pub(super) enum Update {
    Started,
    Delta(String),
    Completed(Option<String>),
}
#[derive(Debug)]
pub(super) enum Event {
    Send(String, bool),
    TurnStarted(String),
    TurnCompleted {
        turn: String,
        error: Option<String>,
    },
    Item {
        turn: String,
        id: String,
        kind: ItemKind,
        update: Update,
    },
    ToolStarted {
        turn: String,
        id: String,
        label: String,
        tool: String,
    },
    ToolFinished {
        turn: String,
        id: String,
        error: Option<String>,
    },
    Receipt(Receipt),
    Failed(String),
    StopRequested,
    StopTimeout,
    AnalysisClosed,
    Disconnected,
    Reconnected,
}
#[derive(Default, Debug)]
pub(super) struct Transcript {
    pub entries: Vec<Entry>,
    pub turn: Option<String>,
    pub busy: bool,
    pub stop_pending: bool,
    start: usize,
    awaiting_ack: bool,
    disconnected: bool,
    revision: u64,
}
impl Transcript {
    pub fn accepts(&self, turn: &str) -> bool {
        self.busy && self.turn.as_deref() == Some(turn)
    }
    fn clear_waiting(&mut self) -> bool {
        let before = self.entries.len();
        self.entries.retain(|entry| {
            !matches!(
                entry,
                Entry::Status(Status::Preparing | Status::Waiting { .. })
            )
        });
        before != self.entries.len()
    }
    fn finish(&mut self, status: Option<Status>) {
        self.clear_waiting();
        self.busy = false;
        self.stop_pending = false;
        for entry in &mut self.entries[self.start..] {
            if let Entry::Activity { state, .. } = entry
                && *state == ActivityState::Running
            {
                *state = ActivityState::Failed("Turn ended before the tool responded".into());
            }
        }
        if let Some(status) = status {
            self.entries.push(Entry::Status(status));
        }
    }
    pub fn apply(&mut self, event: Event, now: Instant) -> bool {
        let changed = self.apply_event(event, now);
        if changed {
            self.revision = self.revision.wrapping_add(1);
        }
        changed
    }
    /// Advances on reducer changes, including replacements and waiting removal.
    /// Renders (including the waiting timer) and ignored events leave it unchanged.
    pub fn revision(&self) -> u64 {
        self.revision
    }
    fn apply_event(&mut self, event: Event, now: Instant) -> bool {
        match event {
            Event::Receipt(receipt) => self.entries.push(Entry::Receipt(receipt)),
            Event::Send(text, edit) => {
                if self.busy || self.stop_pending {
                    return false;
                }
                self.start = self.entries.len();
                self.turn = None;
                self.busy = true;
                self.awaiting_ack = true;
                self.entries
                    .extend([Entry::User(text, edit), Entry::Status(Status::Preparing)]);
            }
            Event::TurnStarted(turn) => {
                if !self.awaiting_ack {
                    return false;
                }
                self.awaiting_ack = false;
                self.turn = Some(turn);
                if self.busy {
                    self.clear_waiting();
                    self.entries
                        .push(Entry::Status(Status::Waiting { started: now }));
                }
            }
            Event::TurnCompleted { turn, error } => {
                if self.turn.as_deref() != Some(&turn) || !(self.busy || self.stop_pending) {
                    return false;
                }
                let status = error
                    .map(Status::Error)
                    .or_else(|| self.stop_pending.then_some(Status::Stopped));
                self.finish(status);
            }
            Event::Item {
                turn,
                id,
                kind,
                update,
            } => {
                if !self.accepts(&turn) {
                    return false;
                }
                let cleared = (matches!(&update, Update::Delta(text) if !text.is_empty())
                    || matches!(&update, Update::Completed(_)))
                    && self.clear_waiting();
                let entries = &mut self.entries[self.start..];
                let existing = entries.iter().position(|e| match (kind, e) {
                    (ItemKind::Assistant, Entry::Assistant { id: key, .. })
                    | (ItemKind::Thinking, Entry::Thinking { id: key, .. }) => key == &id,
                    _ => false,
                });
                let index = if let Some(index) = existing {
                    self.start + index
                } else {
                    self.entries.push(match kind {
                        ItemKind::Assistant => Entry::Assistant {
                            id,
                            text: String::new(),
                        },
                        ItemKind::Thinking => Entry::Thinking {
                            id,
                            text: String::new(),
                        },
                    });
                    self.entries.len() - 1
                };
                if let Entry::Assistant { text, .. } | Entry::Thinking { text, .. } =
                    &mut self.entries[index]
                {
                    match update {
                        Update::Delta(delta) => {
                            if existing.is_some() && delta.is_empty() {
                                return cleared;
                            }
                            text.push_str(&delta);
                        }
                        Update::Completed(Some(value)) => {
                            if existing.is_some() && *text == value {
                                return cleared;
                            }
                            *text = value;
                        }
                        _ if existing.is_some() => return cleared,
                        _ => {}
                    }
                }
            }
            Event::ToolStarted {
                turn,
                id,
                label,
                tool,
            } => {
                if !self.accepts(&turn)
                    || self.entries[self.start..]
                        .iter()
                        .any(|e| matches!(e, Entry::Activity { id: key, .. } if key == &id))
                {
                    return false;
                }
                self.clear_waiting();
                self.entries.push(Entry::Activity {
                    id,
                    label,
                    tool,
                    state: ActivityState::Running,
                });
            }
            Event::ToolFinished { turn, id, error } => {
                if !self.accepts(&turn) {
                    return false;
                }
                let Some(Entry::Activity { state, .. }) = self.entries[self.start..]
                    .iter_mut()
                    .find(|e| matches!(e, Entry::Activity { id: key, .. } if key == &id))
                else {
                    return false;
                };
                if *state != ActivityState::Running {
                    return false;
                }
                *state = error.map_or(ActivityState::Done, ActivityState::Failed);
            }
            Event::StopRequested => {
                if !self.busy {
                    return false;
                }
                self.busy = false;
                self.stop_pending = true;
                self.clear_waiting();
            }
            Event::StopTimeout => {
                if !self.stop_pending {
                    return false;
                }
                self.finish(Some(Status::Stopped));
            }
            Event::Failed(error) => {
                self.awaiting_ack = false;
                self.finish(Some(Status::Error(error)));
            }
            Event::AnalysisClosed | Event::Disconnected => {
                let intentional = matches!(event, Event::AnalysisClosed);
                let changed = !self.disconnected
                    || self.busy
                    || self.stop_pending
                    || self.awaiting_ack
                    || self.turn.is_some();
                self.disconnected = true;
                self.awaiting_ack = false;
                if intentional {
                    for entry in &mut self.entries[self.start..] {
                        if let Entry::Activity { state, .. } = entry
                            && *state == ActivityState::Running
                        {
                            *state = ActivityState::Stopped;
                        }
                    }
                }
                self.finish((self.busy || self.stop_pending).then(|| {
                    if intentional {
                        Status::Stopped
                    } else {
                        Status::Error("Connection lost".into())
                    }
                }));
                self.turn = None;
                return changed;
            }
            Event::Reconnected => {
                if !self.disconnected {
                    return false;
                }
                self.disconnected = false;
                self.entries.push(Entry::Status(Status::Reconnected));
            }
        }
        true
    }
    pub fn update_receipts(&mut self, mut update: impl FnMut(&mut Receipt)) {
        for entry in &mut self.entries {
            if let Entry::Receipt(receipt) = entry {
                let before = receipt.clone();
                update(receipt);
                if *receipt != before {
                    self.revision = self.revision.wrapping_add(1);
                }
            }
        }
    }
    pub fn conversation(&self, now: Instant) -> String {
        self.entries
            .iter()
            .map(|entry| {
                let role = match entry {
                    Entry::User(_, true) => "You · Edit",
                    Entry::User(_, false) => "You · Review",
                    Entry::Receipt(_) => "Receipt",
                    Entry::Assistant { .. } => "Assistant",
                    Entry::Thinking { .. } => "Thinking",
                    Entry::Activity { .. } => "Activity",
                    Entry::Status(_) => "Status",
                };
                format!("## {role}\n\n{}\n", entry.text(now))
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}
impl Entry {
    pub fn text(&self, now: Instant) -> String {
        match self {
            Self::User(text, _) | Self::Assistant { text, .. } | Self::Thinking { text, .. } => {
                text.clone()
            }
            Self::Receipt(receipt) => receipt.text(),
            Self::Activity { label, state, .. } => format!(
                "{label}{}",
                match state {
                    ActivityState::Running => String::new(),
                    ActivityState::Done => " · done".into(),
                    ActivityState::Stopped => " · stopped".into(),
                    ActivityState::Failed(error) => format!(" · failed: {error}"),
                }
            ),
            Self::Status(status) => match status {
                Status::Preparing => "Preparing…".into(),
                Status::Waiting { started } => {
                    waiting_label(now.saturating_duration_since(*started))
                }
                Status::Stopped => "Stopped".into(),
                Status::Error(error) => format!("Error: {error}"),
                Status::Reconnected => {
                    "Connection restored. The next message starts a new conversation.".into()
                }
            },
        }
    }
}
pub(super) fn waiting_label(elapsed: Duration) -> String {
    format!(
        "{} · {} s",
        if elapsed.as_secs() >= 15 {
            "Still waiting"
        } else {
            "Waiting for response"
        },
        elapsed.as_secs()
    )
}
/// GPUI adds the delta to its negative offset before our scroll listener runs.
/// Any upward motion releases follow, even inside the bottom threshold. Only
/// downward motion can resume it; horizontal/zero-delta events preserve intent.
pub(super) fn follow_after_scroll(
    prev_follow: bool,
    delta_y: f32,
    offset: f32,
    max_offset: f32,
) -> bool {
    if delta_y > 0. {
        false
    } else if delta_y < 0. {
        max_offset + offset <= 32.
    } else {
        prev_follow
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn active(now: Instant) -> Transcript {
        let mut t = Transcript::default();
        assert!(t.apply(Event::Send("hello".into(), false), now));
        assert_eq!(t.entries[1].text(now), "Preparing…");
        assert!(t.apply(Event::TurnStarted("turn".into()), now));
        t
    }
    fn item(id: &str, update: Update) -> Event {
        Event::Item {
            turn: "turn".into(),
            id: id.into(),
            kind: ItemKind::Assistant,
            update,
        }
    }
    fn tool(id: &str) -> Event {
        Event::ToolStarted {
            turn: "turn".into(),
            id: id.into(),
            label: "Inspecting…".into(),
            tool: "xray_get_plots".into(),
        }
    }
    fn finished(id: &str, error: Option<&str>) -> Event {
        Event::ToolFinished {
            turn: "turn".into(),
            id: id.into(),
            error: error.map(str::to_owned),
        }
    }
    fn completed(turn: &str, error: Option<&str>) -> Event {
        Event::TurnCompleted {
            turn: turn.into(),
            error: error.map(str::to_owned),
        }
    }
    #[test]
    fn mode_and_app_receipts_survive_turn_completion() {
        let now = Instant::now();
        let mut t = Transcript::default();
        t.apply(Event::Send("edit".into(), true), now);
        t.apply(Event::TurnStarted("turn".into()), now);
        assert_eq!(t.entries[0], Entry::User("edit".into(), true));
        t.apply(
            Event::Receipt(Receipt::change(
                "Cu",
                super::super::Stage::Normalize,
                vec!["E₀: Auto → 8979 eV".into()],
                1,
            )),
            now,
        );
        t.apply(completed("turn", None), now);
        let revision = t.revision();
        t.update_receipts(|r| r.state = "Recalculation complete".into());
        assert!(t.revision() > revision);
        let revision = t.revision();
        t.update_receipts(|_| {});
        assert_eq!(t.revision(), revision);
        assert!(t.conversation(now).contains("You · Edit"));
        assert!(t.conversation(now).contains("Recalculation complete"));
        t.apply(Event::Send("review".into(), false), now);
        assert!(t.conversation(now).contains("You · Review"));
    }
    #[test]
    fn small_upward_scrolls_release_follow_and_accumulate() {
        let mut follow = true;
        let mut offset = -800.;
        for _ in 0..20 {
            offset += 2.;
            follow = follow_after_scroll(follow, 2., offset, 800.);
            assert!(!follow);
            // Gesture-end and horizontal events must not re-enable follow.
            assert!(!follow_after_scroll(follow, 0., offset, 800.));
        }
        assert_eq!(offset, -760.);
    }
    #[test]
    fn downward_scroll_resumes_follow_only_near_bottom() {
        assert!(!follow_after_scroll(false, -1., -767., 800.));
        assert!(follow_after_scroll(false, -1., -768., 800.));
        assert!(follow_after_scroll(false, -32., -800., 800.));
        assert!(follow_after_scroll(false, -1., -801., 800.));
        assert!(follow_after_scroll(false, -1., 0., 0.));
        assert!(follow_after_scroll(true, 0., -800., 800.));
        assert!(!follow_after_scroll(true, 0.5, -799.5, 800.));
    }
    #[test]
    fn revisions_track_updates_to_earlier_entries_and_ignore_duplicates() {
        let now = Instant::now();
        let mut t = active(now);
        t.apply(item("A", Update::Delta("first".into())), now);
        t.apply(tool("X"), now);
        t.apply(item("B", Update::Delta("last".into())), now);
        let revision = t.revision();
        let len = t.entries.len();
        // A same-length replacement in an earlier row still changes content.
        assert!(t.apply(item("A", Update::Completed(Some("other".into()))), now));
        assert_eq!(t.revision(), revision + 1);
        assert_eq!(t.entries.len(), len);
        assert!(t.apply(finished("X", None), now));
        assert_eq!(t.revision(), revision + 2);
        assert!(!t.apply(item("A", Update::Completed(Some("other".into()))), now));
        assert!(!t.apply(item("B", Update::Delta(String::new())), now));
        assert!(!t.apply(finished("X", None), now));
        assert!(!t.apply(completed("stale", None), now));
        let _ = t.conversation(now + Duration::from_secs(1));
        assert_eq!(t.revision(), revision + 2);
        assert!(t.apply(completed("turn", None), now));
        assert_eq!(t.revision(), revision + 3);
    }
    #[test]
    fn revisions_track_waiting_removal_and_disconnection_early_returns() {
        let now = Instant::now();
        let mut t = Transcript::default();
        assert_eq!(t.revision(), 0);
        assert!(!t.apply(Event::Reconnected, now));
        assert_eq!(t.revision(), 0);
        t.apply(Event::Send("hello".into(), false), now);
        t.apply(Event::TurnStarted("turn".into()), now);
        assert_eq!(t.revision(), 2);
        let _ = t.conversation(now + Duration::from_secs(20));
        assert_eq!(t.revision(), 2);
        t.apply(item("A", Update::Started), now);
        let revision = t.revision();
        assert!(t.apply(item("A", Update::Completed(None)), now));
        assert_eq!(t.revision(), revision + 1);
        assert!(!t.apply(item("A", Update::Completed(None)), now));
        assert_eq!(t.revision(), revision + 1);
        assert!(t.apply(Event::Disconnected, now));
        assert_eq!(t.revision(), revision + 2);
        assert!(!t.apply(Event::Disconnected, now));
        assert_eq!(t.revision(), revision + 2);
        assert!(t.apply(Event::Reconnected, now));
        assert_eq!(t.revision(), revision + 3);
    }
    #[test]
    fn interleaved_calls_close_only_matching_id() {
        let now = Instant::now();
        let mut t = active(now);
        for id in ["X", "Y"] {
            assert!(t.apply(tool(id), now));
        }
        assert!(!t.apply(tool("X"), now));
        assert!(!t.apply(finished("unknown", None), now));
        assert!(t.apply(finished("X", None), now));
        assert!(t.entries[1].text(now).ends_with("· done"));
        assert_eq!(t.entries[2].text(now), "Inspecting…");
        assert!(t.apply(finished("Y", Some("oops")), now));
        assert!(!t.apply(finished("Y", None), now));
        assert!(t.entries[2].text(now).ends_with("· failed: oops"));
    }
    #[test]
    fn items_preserve_order_replace_and_deduplicate() {
        let now = Instant::now();
        let mut t = active(now);
        t.apply(item("A", Update::Started), now);
        t.apply(item("B", Update::Started), now);
        t.apply(item("B", Update::Delta("second".into())), now);
        t.apply(item("A", Update::Delta("fir".into())), now);
        t.apply(item("A", Update::Delta("st".into())), now);
        assert!(t.conversation(now).contains("first"));
        assert!(t.apply(item("A", Update::Completed(Some("final".into()))), now));
        assert!(!t.apply(item("A", Update::Completed(Some("final".into()))), now));
        assert_eq!(t.entries.len(), 3);
        assert_eq!(t.entries[1].text(now), "final");
        assert_eq!(t.entries[2].text(now), "second");
        t.apply(item("empty", Update::Started), now);
        assert!(!t.apply(item("empty", Update::Delta(String::new())), now));
        assert!(!t.apply(item("empty", Update::Completed(None)), now));
        t.apply(
            Event::Item {
                turn: "turn".into(),
                id: "R".into(),
                kind: ItemKind::Thinking,
                update: Update::Delta("reason".into()),
            },
            now,
        );
        assert!(matches!(&t.entries[4], Entry::Thinking { text, .. } if text == "reason"));
    }
    #[test]
    fn empty_completion_reports_waiting_removal_once() {
        let now = Instant::now();
        for value in [None, Some(String::new())] {
            let mut t = active(now);
            assert!(t.apply(item("A", Update::Started), now));
            assert!(!t.apply(item("A", Update::Delta(String::new())), now));
            assert_eq!(t.entries.len(), 3);
            assert!(t.apply(item("A", Update::Completed(value.clone())), now));
            assert_eq!(t.entries.len(), 2);
            assert!(!t.apply(item("A", Update::Completed(value)), now));
        }
    }
    #[test]
    fn error_and_stale_events() {
        let now = Instant::now();
        let mut t = active(now);
        assert!(!t.accepts("old"));
        assert!(!t.apply(completed("old", None), now));
        assert!(!t.apply(Event::TurnStarted("old".into()), now));
        assert!(!t.apply(
            Event::Item {
                turn: "old".into(),
                id: "A".into(),
                kind: ItemKind::Assistant,
                update: Update::Delta("stale".into())
            },
            now
        ));
        assert!(t.apply(completed("turn", Some("failed")), now));
        assert!(!t.busy);
        assert_eq!(
            t.entries.last(),
            Some(&Entry::Status(Status::Error("failed".into())))
        );
        assert!(!t.apply(item("A", Update::Completed(Some("late".into()))), now));
        t.apply(Event::Send("next".into(), false), now);
        t.apply(Event::TurnStarted("next".into()), now);
        assert!(!t.apply(item("A", Update::Delta("old".into())), now));
        assert!(!t.apply(tool("X"), now));
        assert!(!t.apply(finished("X", None), now));
    }
    #[test]
    fn stop_before_ack_and_timeout_or_completion() {
        let now = Instant::now();
        for timeout_first in [false, true] {
            let mut t = Transcript::default();
            t.apply(Event::Send("hello".into(), false), now);
            t.apply(Event::StopRequested, now);
            assert!(!t.busy && t.stop_pending);
            if timeout_first {
                t.apply(Event::StopTimeout, now);
            }
            t.apply(Event::TurnStarted("turn".into()), now);
            assert!(!t.busy);
            assert!(!t.apply(tool("late"), now));
            t.apply(completed("turn", None), now);
            assert!(!t.apply(Event::StopTimeout, now));
            assert_eq!(
                t.entries,
                vec![
                    Entry::User("hello".into(), false),
                    Entry::Status(Status::Stopped)
                ]
            );
        }
    }
    #[test]
    fn assistant_analysis_close_stops_neutrally_and_ignores_late_events() {
        let now = Instant::now();
        for acknowledged in [false, true] {
            for stop_pending in [false, true] {
                let mut t = Transcript::default();
                t.apply(Event::Send("keep my question".into(), false), now);
                if acknowledged {
                    t.apply(Event::TurnStarted("turn".into()), now);
                    t.apply(item("A", Update::Delta("partial reply".into())), now);
                    t.apply(tool("running"), now);
                }
                if stop_pending {
                    t.apply(Event::StopRequested, now);
                }
                assert!(t.apply(Event::AnalysisClosed, now));
                assert!(!t.busy && !t.stop_pending && !t.awaiting_ack);
                assert!(t.turn.is_none());
                assert_eq!(t.entries.last(), Some(&Entry::Status(Status::Stopped)));
                let copied = t.conversation(now);
                assert!(copied.contains("keep my question"));
                assert!(!copied.contains("Error:") && !copied.contains("failed:"));
                assert!(!copied.contains("Preparing") && !copied.contains("Waiting"));
                if acknowledged {
                    assert!(copied.contains("partial reply"));
                    assert!(copied.contains("Inspecting… · stopped"));
                }
                let revision = t.revision();
                assert!(!t.apply(Event::Disconnected, now));
                assert!(!t.apply(Event::AnalysisClosed, now));
                assert!(!t.apply(Event::TurnStarted("late".into()), now));
                assert!(!t.apply(completed("turn", Some("late error")), now));
                assert!(!t.apply(item("A", Update::Delta("late text".into())), now));
                assert!(!t.apply(finished("running", None), now));
                assert!(!t.apply(Event::StopTimeout, now));
                assert_eq!(t.revision(), revision);
                assert_eq!(t.conversation(now), copied);
            }
        }
    }
    #[test]
    fn assistant_idle_analysis_close_preserves_history_and_real_disconnect_errors() {
        let now = Instant::now();
        let mut idle = Transcript::default();
        assert!(idle.apply(Event::AnalysisClosed, now));
        assert!(idle.entries.is_empty());

        let mut done = active(now);
        done.apply(item("A", Update::Delta("finished reply".into())), now);
        done.apply(completed("turn", None), now);
        let copied = done.conversation(now);
        assert!(done.apply(Event::AnalysisClosed, now));
        assert_eq!(done.conversation(now), copied);

        let mut lost = active(now);
        lost.apply(Event::Disconnected, now);
        assert_eq!(
            lost.entries.last(),
            Some(&Entry::Status(Status::Error("Connection lost".into())))
        );
    }
    #[test]
    fn labels_and_reconnection() {
        assert_eq!(
            waiting_label(Duration::from_secs(3)),
            "Waiting for response · 3 s"
        );
        assert_eq!(
            waiting_label(Duration::from_secs(20)),
            "Still waiting · 20 s"
        );
        assert_eq!(
            waiting_label(Duration::from_secs(15)),
            "Still waiting · 15 s"
        );
        let now = Instant::now();
        let mut t = active(now);
        assert_eq!(
            t.entries[1].text(now + Duration::from_secs(3)),
            "Waiting for response · 3 s"
        );
        assert!(!t.apply(Event::Reconnected, now));
        t.apply(Event::Disconnected, now);
        assert!(!t.apply(Event::Disconnected, now));
        assert!(!t.busy);
        assert!(t.apply(Event::Reconnected, now));
        assert_eq!(
            t.entries.last().map(|e| e.text(now)).as_deref(),
            Some("Connection restored. The next message starts a new conversation.")
        );
        assert!(!t.apply(Event::Reconnected, now));
        t.apply(Event::Send("next".into(), false), now);
        t.apply(Event::Failed("preparing failed".into()), now);
        assert!(!t.busy);
        t.apply(Event::Send("cancel preparation".into(), false), now);
        t.apply(Event::StopRequested, now);
        t.apply(Event::StopTimeout, now);
        assert!(t.apply(Event::Send("try again".into(), false), now));
    }
}
