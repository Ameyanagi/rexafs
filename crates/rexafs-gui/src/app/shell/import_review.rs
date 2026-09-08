//! Pending sources have review actions, never spectrum marks or group counts.
use super::*;

impl StudioApp {
    pub(crate) fn pending_imports(
        &self,
        cx: &mut Context<Self>,
    ) -> Option<impl IntoElement + use<>> {
        let pending = self
            .intake
            .history
            .iter()
            .enumerate()
            .flat_map(|(batch, state)| {
                state
                    .sources
                    .iter()
                    .filter(|(path, source)| {
                        source.pending.is_some() && !self.intake.approved.contains_key(*path)
                    })
                    .map(move |(path, source)| (batch, path, source.pending.as_ref().unwrap()))
            })
            .collect::<Vec<_>>();
        if pending.is_empty() {
            return None;
        }
        let mut list = div()
            .id("pending-import-sources")
            .max_h(px(180.))
            .overflow_y_scroll()
            .px_2()
            .py_2()
            .text_size(px(11.))
            .border_b_1()
            .border_color(self.theme.border)
            .child(format!("Pending import · {} sources", pending.len()));
        for (batch, path, pending) in pending.into_iter().take(24) {
            let path = path.clone();
            list = list.child(
                div()
                    .id(SharedString::from(format!(
                        "pending-{batch}-{}",
                        path.display()
                    )))
                    .cursor_pointer()
                    .py_1()
                    .text_color(self.theme.warn)
                    .child(format!(
                        "{} · {}",
                        path.file_name().unwrap_or_default().to_string_lossy(),
                        pending.reason
                    ))
                    .on_click(cx.listener(move |app, _: &ClickEvent, window, cx| {
                        let index = app
                            .pending_clusters(batch)
                            .iter()
                            .position(|c| c.files.contains(&path))
                            .unwrap_or(0);
                        app.open_import_review(batch, index, window, cx);
                    })),
            );
        }
        Some(list)
    }
}
