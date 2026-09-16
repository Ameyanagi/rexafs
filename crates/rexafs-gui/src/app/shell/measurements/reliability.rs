use super::*;
use crate::app::shell::button;
use gpui::{IntoElement, ParentElement, Styled, div, prelude::*};

impl StudioApp {
    pub(super) fn monitor_measurements(&mut self, cx: &mut Context<Self>) {
        if self.measurements.monitoring {
            return;
        }
        self.measurements.monitoring = true;
        let generation = self.project_generation;
        cx.spawn(async move |this, cx| {
            let entries = cx
                .background_spawn(async move {
                    recovery::recovery_root().and_then(|root| recovery::discover(&root))
                })
                .await;
            this.update(cx, |app, cx| {
                if app.project_generation != generation {
                    return;
                }
                match entries {
                    Ok(entries) => app.measurements.recovery_entries = entries,
                    Err(e) => app.measurements.message = format!("Could not inspect recovery: {e}"),
                }
                cx.notify();
            })
            .ok();
            loop {
                cx.background_executor()
                    .timer(std::time::Duration::from_secs(10))
                    .await;
                if this
                    .update(cx, |app, cx| {
                        if app.project_generation != generation {
                            return false;
                        }
                        if app.measurements.cancel.is_none() && !app.measurements.recovery_busy {
                            app.check_measurement_inputs_internal(false, cx);
                        }
                        true
                    })
                    .ok()
                    != Some(true)
                {
                    break;
                }
            }
        })
        .detach();
    }

    fn use_measurement_recovery(&mut self, index: usize, discard: bool, cx: &mut Context<Self>) {
        if self.measurements.cancel.is_some() || self.measurements.recovery_busy {
            return;
        }
        let Some(entry) = self.measurements.recovery_entries.get(index).cloned() else {
            return;
        };
        let generation = self.project_generation;
        self.measurements.recovery_busy = true;
        cx.spawn(async move|this,cx|{
            let result=cx.background_spawn(async move{
                if discard {recovery::discard(&entry).map(|_|None)}else{recovery::recover(&entry).map(Some)}
            }).await;
            this.update(cx,|app,cx|{
                if app.project_generation!=generation{return;}
                app.measurements.recovery_busy=false;
                match result {
                    Ok(Some((mut project,committed)))=>{
                        let next=project.assign_group_ids();
                        let registry=crate::group_identity::GroupRegistry::from_sources(std::mem::take(&mut project.source_groups));
                        app.apply_project(project,next,registry,cx);
                        app.measurements.overview=false;
                        app.measurements.results=true;
                        app.measurements.advanced=true;
                        app.measurements.message=format!("Recovered {committed} committed rows into an unsaved copy. Review inputs, then resume unfinished frames or save.");
                    }
                    Ok(None)=>{app.measurements.recovery_entries.remove(index);app.measurements.message="Recovery copy discarded.".into();}
                    Err(error)=>app.measurements.message=format!("Recovery failed: {error}"),
                }
                cx.notify();
            }).ok();
        }).detach();
        cx.notify();
    }

    pub(super) fn measurement_recovery_panel(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let t = self.theme;
        let mut view = div().flex().flex_col().gap_2();
        if self.measurements.recovery_entries.is_empty() {
            return view.into_any_element();
        }
        view = view.child("Measurement recovery copies");
        view = view.child(div().text_color(t.text_muted).child(
            "Open a recovery copy to replace this workspace. Saved project files are unchanged.",
        ));
        for (index, entry) in self.measurements.recovery_entries.iter().enumerate() {
            view = view.child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_2()
                    .items_center()
                    .child(format!("{} · {}", entry.title, entry.created))
                    .child(
                        button(
                            &t,
                            ("measurement-recover", index),
                            "Open recovery copy",
                            true,
                        )
                        .when(
                            self.measurements.recovery_busy || self.measurements.cancel.is_some(),
                            |d| d.disabled(true),
                        )
                        .on_click(cx.listener(move |app, _, _, cx| {
                            app.use_measurement_recovery(index, false, cx)
                        })),
                    )
                    .child(
                        button(
                            &t,
                            ("measurement-discard", index),
                            "Discard recovery copy",
                            false,
                        )
                        .when(
                            self.measurements.recovery_busy || self.measurements.cancel.is_some(),
                            |d| d.disabled(true),
                        )
                        .on_click(cx.listener(move |app, _, _, cx| {
                            app.use_measurement_recovery(index, true, cx)
                        })),
                    ),
            );
        }
        view.into_any_element()
    }
}
