//! One identity-bound removal and one undo entry for all marked groups.
use std::collections::{BTreeMap, BTreeSet};

use gpui::{Context, IntoElement, Window, div, prelude::*, px};

use super::{button, controls::Menu};
use crate::{app::StudioApp, group_identity::GroupId};

pub(crate) struct MarkedRemoval {
    generation: u64,
    groups: BTreeMap<GroupId, String>,
    hidden: usize,
    error: Option<String>,
}

fn capture(
    marks: &BTreeSet<usize>,
    mut resolve: impl FnMut(usize) -> Option<(GroupId, String)>,
) -> Result<BTreeMap<GroupId, String>, String> {
    if marks.is_empty() {
        return Err("Mark the groups to remove.".into());
    }
    marks
        .iter()
        .map(|&ix| {
            resolve(ix).ok_or_else(|| {
                "A marked group is no longer available. Review the marks again.".into()
            })
        })
        .collect()
}

impl StudioApp {
    pub(crate) fn open_remove_marked(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let groups = match capture(&self.selection, |ix| {
            Some((self.group_id(ix)?, self.entry_label(ix).to_string()))
        }) {
            Ok(groups) => groups,
            Err(error) => {
                self.status = error.into();
                cx.notify();
                return;
            }
        };
        let (_, hidden, collapsed) = self.interaction_rows().mark_counts(&self.selection);
        self.ui.marked_removal = Some(MarkedRemoval {
            generation: self.project_generation,
            groups,
            hidden: hidden + collapsed,
            error: None,
        });
        self.ui.return_focus = window.focused(cx);
        self.ui.menu = Some(Menu::RemoveMarked);
        self.ui
            .menu_focus
            .get_or_insert_with(|| cx.focus_handle())
            .focus(window, cx);
        cx.notify();
    }

    fn confirm_marked_removal(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(review) = self.ui.marked_removal.as_ref() else {
            return;
        };
        let current = capture(&self.selection, |ix| {
            Some((self.group_id(ix)?, self.entry_label(ix).to_string()))
        });
        if self.project_generation != review.generation
            || !current
                .as_ref()
                .is_ok_and(|current| current.keys().eq(review.groups.keys()))
        {
            self.ui.marked_removal.as_mut().unwrap().error =
                Some("The marked groups changed. Close this dialog and review them again.".into());
            cx.notify();
            return;
        }
        let review = self.ui.marked_removal.take().unwrap();
        let count = review.groups.len();
        self.close_chrome_menu(window, cx);
        self.remove_groups(
            review.groups.into_keys().collect(),
            format!("Remove {count} marked groups"),
            cx,
        );
    }

    pub(crate) fn marked_removal_panel(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let mut panel = div().p_2().flex().flex_col().gap_2();
        let Some(review) = &self.ui.marked_removal else {
            return panel;
        };
        let count = review.groups.len();
        let t = self.theme;
        panel = panel
            .child(format!("Remove {count} marked groups?"))
            .when(review.hidden > 0, |d| {
                d.child(div().text_color(t.warn).child(format!(
                    "Includes {} hidden or collapsed marks",
                    review.hidden
                )))
            })
            .child(
                div()
                    .text_size(px(11.))
                    .text_color(t.text_muted)
                    .child("Source files stay on disk. Undo restores the groups."),
            )
            .child(
                div()
                    .id("marked-removal-list")
                    .max_h(px(220.))
                    .overflow_y_scroll()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .text_size(px(11.))
                    .children(review.groups.values().map(|name| div().child(name.clone()))),
            )
            .when_some(review.error.clone(), |d, error| {
                d.child(div().text_color(t.error).child(error))
            })
            .child(
                div()
                    .flex()
                    .justify_end()
                    .gap_2()
                    .child(
                        button(&t, "cancel-marked-removal", "Cancel", false)
                            .on_click(cx.listener(|a, _, w, c| a.close_chrome_menu(w, c))),
                    )
                    .child(
                        button(
                            &t,
                            "confirm-marked-removal",
                            format!("Remove {count}"),
                            true,
                        )
                        .on_click(cx.listener(|a, _, w, c| a.confirm_marked_removal(w, c))),
                    ),
            );
        panel
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn marked_removal_captures_every_mark_by_identity_and_refuses_partial_resolution() {
        let ids = [
            GroupId::new_result(),
            GroupId::new_result(),
            GroupId::new_result(),
        ];
        let marks = BTreeSet::from([0, 2]);
        let groups = capture(&marks, |ix| Some((ids[ix].clone(), "Same label".into()))).unwrap();
        assert_eq!(
            groups.keys().cloned().collect::<BTreeSet<_>>(),
            BTreeSet::from([ids[0].clone(), ids[2].clone()])
        );
        assert!(
            capture(&marks, |ix| (ix == 0)
                .then(|| (ids[0].clone(), "First".into())))
            .is_err()
        );
        assert!(capture(&BTreeSet::new(), |_| None).is_err());
    }
}
