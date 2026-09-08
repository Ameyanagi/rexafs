//! Project conversation controls and completed-turn snapshots.
use super::*;
use crate::project::assistant::{Conversation, ConversationMode};

impl AssistantWindow {
    pub(super) fn checkpoint_conversation(&mut self, cx: &mut Context<Self>) {
        if self.history_read_only
            || self.transcript.busy
            || self.transcript.stop_pending
            || self.history_revision == Some(self.transcript.revision())
        {
            return;
        }
        let Some(mut conversation) = self.conversation.clone() else {
            return;
        };
        let entries = self.transcript.persisted_entries();
        if !entries
            .iter()
            .any(|e| matches!(e, crate::project::assistant::ConversationEntry::User { .. }))
        {
            return;
        }
        conversation.entries = entries;
        conversation.updated_at = chrono::Utc::now().to_rfc3339();
        conversation.thread_id = self.thread.clone().or(conversation.thread_id);
        conversation.model = self
            .model()
            .map(|m| m.model.clone())
            .or_else(|| self.preferred_model.clone());
        conversation.effort = self.effort().map(str::to_owned);
        conversation.mode = if self.allow_changes {
            ConversationMode::Edit
        } else {
            ConversationMode::Review
        };
        self.history_revision = Some(self.transcript.revision());
        self.conversation = Some(conversation.clone());
        let generation = self.history_project_generation;
        self.studio
            .update(cx, |app, cx| {
                if app.project_generation == generation {
                    app.assistant_history.upsert(conversation);
                    app.assistant_history_revision += 1;
                    cx.notify();
                }
            })
            .ok();
    }

    pub(crate) fn replace_project(&mut self, generation: u64, cx: &mut Context<Self>) {
        self.disconnected(String::new());
        self.history_project_generation = generation;
        self.clear_conversation(cx);
        self.connect(cx);
    }

    fn clear_conversation(&mut self, cx: &mut Context<Self>) {
        self.run_generation += 1;
        self.thread = None;
        self.conversation = None;
        self.transcript = Transcript::default();
        self.history_revision = None;
        self.history_read_only = false;
        self.history_open = false;
        self.resume_context = None;
        self.prepared = None;
        self.unfolded.clear();
        self.copied.clear();
        self.processing_checks.clear();
        self.tool_calls.clear();
        self.follow = true;
        self.last_rendered_revision = 0;
        self.focus_composer = true;
        self.error = None;
        self.input.update(cx, |input, cx| input.set_text("", cx));
        cx.notify();
    }

    pub(super) fn new_conversation(&mut self, cx: &mut Context<Self>) {
        if self.transcript.busy || self.transcript.stop_pending || pending_blocks_run(&self.pending)
        {
            return;
        }
        self.checkpoint_conversation(cx);
        self.clear_conversation(cx);
    }

    fn choose_conversation(&mut self, conversation: Conversation, cx: &mut Context<Self>) {
        if self.transcript.busy || self.transcript.stop_pending || pending_blocks_run(&self.pending)
        {
            return;
        }
        self.checkpoint_conversation(cx);
        self.clear_conversation(cx);
        self.transcript = Transcript::restore(&conversation.entries);
        self.preferred_model = conversation.model.clone();
        self.preferred_effort = conversation.effort.clone();
        self.allow_changes = conversation.mode == ConversationMode::Edit;
        self.history_read_only = true;
        self.history_revision = Some(self.transcript.revision());
        self.conversation = Some(conversation);
        self.focus_composer = false;
        cx.notify();
    }

    pub(super) fn resume_fallback(&mut self, cx: &mut Context<Self>) {
        self.pending.retain(|_, method| method != "thread/resume");
        self.thread = None;
        self.resume_context = self
            .conversation
            .as_ref()
            .map(Conversation::previous_context);
        if let Some(conversation) = &mut self.conversation {
            conversation.thread_id = None;
        }
        self.history_read_only = false;
        self.transcript.note("Starting a new server thread with the last 10 entries as previous-conversation context.");
        self.focus_composer = true;
        cx.notify();
    }

    fn resume_conversation(&mut self, cx: &mut Context<Self>) {
        if !self.history_read_only || pending_blocks_run(&self.pending) {
            return;
        }
        let thread = self.conversation.as_ref().and_then(|c| c.thread_id.clone());
        let ready = self.client.is_some() && !self.connecting && self.account;
        if let Some(thread) = thread.filter(|_| ready) {
            let directory = self
                .client
                .as_ref()
                .expect("ready client")
                .directory
                .clone();
            let mut params =
                codex_client::resume_thread_params(&directory, self.extended_access, &thread);
            params["developerInstructions"] = json!(include_str!("assistant_workflow.md"));
            if let Some(model) = &self.preferred_model {
                params["model"] = model.clone().into();
            }
            if self.request("thread/resume", params).is_ok() {
                self.status = "Resuming conversation…".into();
                let generation = self.run_generation;
                cx.spawn(async move |this, cx| {
                    cx.background_executor()
                        .timer(Duration::from_secs(30))
                        .await;
                    this.update(cx, |app, cx| {
                        if generation == app.run_generation
                            && app.pending.values().any(|m| m == "thread/resume")
                        {
                            app.resume_fallback(cx);
                        }
                    })
                    .ok();
                })
                .detach();
                cx.notify();
                return;
            }
        }
        self.resume_fallback(cx);
    }

    pub(super) fn history_controls(&self, cx: &mut Context<Self>) -> gpui::Div {
        let t = self.theme;
        let bounds = self.history_trigger_bounds.clone();
        div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_2()
            .min_w_0()
            .child(
                div()
                    .child(self.button(
                        &t,
                        "assistant-conversations",
                        "Conversations ▾",
                        self.history_open,
                        cx.listener(|this, _, _, cx| {
                            this.history_open = !this.history_open;
                            this.model_picker_open = false;
                            this.account_expanded = false;
                            cx.notify();
                        }),
                    ))
                    .on_children_prepainted(move |children, _, _| {
                        if let Some(trigger) = children.first() {
                            bounds.set(*trigger);
                        }
                    }),
            )
            .child(self.button(
                &t,
                "assistant-new-conversation",
                "New",
                false,
                cx.listener(|this, _, _, cx| this.new_conversation(cx)),
            ))
            .when(self.history_read_only, |row| {
                row.child(self.button(
                    &t,
                    "assistant-resume-conversation",
                    "Resume",
                    true,
                    cx.listener(|this, _, _, cx| this.resume_conversation(cx)),
                ))
            })
    }

    pub(super) fn history_overlay(&self, cx: &mut Context<Self>) -> Option<gpui::AnyElement> {
        if !self.history_open {
            return None;
        }
        let t = self.theme;
        let mut conversations = self
            .studio
            .read_with(cx, |app, _| app.assistant_history.conversations.clone())
            .unwrap_or_default();
        if let Some(current) = &self.conversation
            && !conversations.iter().any(|saved| saved.id == current.id)
        {
            let mut current = current.clone();
            current.entries = self.transcript.persisted_entries();
            conversations.insert(0, current);
        }
        let mut list = div()
            .id("assistant-history-list")
            .max_h(px(230.))
            .overflow_y_scroll();
        if conversations.is_empty() {
            list = list.child(div().p_2().child("Completed conversations appear here."));
        }
        for (index, conversation) in conversations.into_iter().enumerate() {
            let label = format!(
                "{} · {} turns · {}",
                conversation.title,
                conversation.turn_count(),
                conversation.relative_time()
            );
            list = list.child(
                self.button(
                    &t,
                    ("assistant-conversation", index),
                    "",
                    false,
                    cx.listener(move |this, _, _, cx| {
                        this.choose_conversation(conversation.clone(), cx)
                    }),
                )
                .w_full()
                .min_w_0()
                .child(
                    div()
                        .min_w_0()
                        .overflow_hidden()
                        .text_ellipsis()
                        .whitespace_nowrap()
                        .child(label),
                ),
            );
        }
        let popup = div()
            .w(px(300.))
            .max_w_full()
            .p_2()
            .flex()
            .flex_col()
            .gap_2()
            .bg(t.surface)
            .border_1()
            .border_color(t.border)
            .rounded_md()
            .shadow_lg()
            .occlude()
            .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(list)
            .child(self.history_limit.clone())
            .child(
                div()
                    .text_size(px(11.))
                    .text_color(t.text_muted)
                    .child("0 disables saving conversations in projects."),
            );
        let bounds = self.history_trigger_bounds.get();
        Some(
            div()
                .absolute()
                .inset_0()
                .on_mouse_down(
                    gpui::MouseButton::Left,
                    cx.listener(|this, _, _, cx| {
                        this.history_open = false;
                        cx.notify();
                    }),
                )
                .child(
                    gpui::anchored()
                        .anchor(gpui::Corner::TopLeft)
                        .position(gpui::point(
                            bounds.origin.x,
                            bounds.origin.y + bounds.size.height,
                        ))
                        .snap_to_window()
                        .child(popup),
                )
                .into_any_element(),
        )
    }
}
