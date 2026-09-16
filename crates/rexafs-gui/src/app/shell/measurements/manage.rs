use super::*;
use crate::{app::shell::button, widgets::text_input::TextInput};
use gpui::{IntoElement, ParentElement, Styled, div, prelude::*};

pub(super) struct SeriesEditor {
    fields: Vec<Entity<TextInput>>,
    page: usize,
    selected: usize,
}

impl StudioApp {
    fn open_series_editor(&mut self, cx: &mut Context<Self>) {
        let Some(series) = self
            .measurements
            .selected_series
            .and_then(|i| self.measurements.archive.series.get(i))
        else {
            return;
        };
        let values = [
            ("Series name", series.name.clone()),
            ("Coordinate name", series.coordinate.label.clone()),
            ("Coordinate unit", series.coordinate.unit.clone()),
            ("Coordinate source", series.coordinate.source.clone()),
            (
                "Timestamp meaning (start/midpoint/end/unspecified)",
                series.coordinate.timestamp_meaning.clone(),
            ),
            (
                "Frame coordinate (blank = missing)",
                series
                    .frames
                    .first()
                    .and_then(|f| f.coordinate)
                    .map(|v| v.to_string())
                    .unwrap_or_default(),
            ),
            (
                "Acquired at (RFC 3339 with timezone)",
                series
                    .frames
                    .first()
                    .and_then(|f| f.acquired_at.clone())
                    .unwrap_or_default(),
            ),
        ];
        let fields = values
            .into_iter()
            .map(|(label, value)| cx.new(|cx| TextInput::new(label, value, self.theme, cx)))
            .collect();
        self.measurements.editor = Some(SeriesEditor {
            fields,
            page: 0,
            selected: 0,
        });
        cx.notify();
    }

    fn select_series_member(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(series) = self
            .measurements
            .selected_series
            .and_then(|i| self.measurements.archive.series.get(i))
        else {
            return;
        };
        let Some(frame) = series.frames.get(index) else {
            return;
        };
        if let Some(editor) = &mut self.measurements.editor {
            editor.selected = index;
            editor.fields[5].update(cx, |f, cx| {
                f.set_text(
                    frame.coordinate.map(|v| v.to_string()).unwrap_or_default(),
                    cx,
                )
            });
            editor.fields[6].update(cx, |f, cx| {
                f.set_text(frame.acquired_at.clone().unwrap_or_default(), cx)
            });
        }
        self.measurements.preview_index = index;
        self.preview_measurement(cx);
        cx.notify();
    }

    fn series_edited(&mut self, cx: &mut Context<Self>) {
        self.measurements.message =
            "Series revision saved. Earlier runs retain their original membership and coordinates."
                .into();
        self.measurements.preview_index = 0;
        self.measurements.preview = None;
        self.measurements.preview_generation += 1;
        cx.notify();
    }

    fn save_series_metadata(&mut self, cx: &mut Context<Self>) {
        let Some(editor) = &self.measurements.editor else {
            return;
        };
        let fields: Vec<_> = editor
            .fields
            .iter()
            .map(|f| f.read(cx).text().trim().to_string())
            .collect();
        let result = (|| {
            if fields[0].is_empty() {
                return Err("Enter a series name".to_string());
            }
            let coordinate = if fields[5].is_empty() {
                None
            } else {
                Some(
                    fields[5]
                        .parse::<f64>()
                        .ok()
                        .filter(|v| v.is_finite())
                        .ok_or("Enter a finite coordinate or leave it blank")?,
                )
            };
            if coordinate.is_some()
                && (fields[1].is_empty() || fields[2].is_empty() || fields[3].is_empty())
            {
                return Err("Declare the coordinate name, unit and source".into());
            }
            let acquired_at = if fields[6].is_empty() {
                None
            } else {
                chrono::DateTime::parse_from_rfc3339(&fields[6]).map_err(|_|"Acquisition timestamps need a UTC offset, for example 2026-09-16T12:00:00+09:00")?;
                Some(fields[6].clone())
            };
            let series =
                &mut self.measurements.archive.series[self.measurements.selected_series.unwrap()];
            series.name = fields[0].clone();
            series.coordinate = CoordinateDefinition {
                label: fields[1].clone(),
                unit: fields[2].clone(),
                source: fields[3].clone(),
                timestamp_meaning: fields[4].clone(),
            };
            if let Some(frame) = series.frames.get_mut(editor.selected) {
                frame.coordinate = coordinate;
                frame.acquired_at = acquired_at;
            }
            series.revise();
            Ok::<_, String>(())
        })();
        match result {
            Ok(()) => self.series_edited(cx),
            Err(e) => self.measurements.message = e,
        }
        cx.notify();
    }

    fn edit_series_members(&mut self, action: usize, cx: &mut Context<Self>) {
        if self.measurements.cancel.is_some() {
            return;
        }
        let Some(index) = self.measurements.selected_series else {
            return;
        };
        let selected = self
            .measurements
            .editor
            .as_ref()
            .map(|e| e.selected)
            .unwrap_or(0);
        let additions = if action == 4 {
            self.selection
                .iter()
                .filter_map(|&ix| self.group_id(ix).map(|group| (group, self.entry_label(ix))))
                .collect::<Vec<_>>()
        } else {
            vec![]
        };
        let series = &mut self.measurements.archive.series[index];
        match action {
            0 => series.sort_naturally(),
            1 => {
                if let Err(e) = series.sort_acquisition() {
                    self.measurements.message = e;
                    cx.notify();
                    return;
                }
            }
            2 | 3 => {
                let other = if action == 2 {
                    selected.saturating_sub(1)
                } else {
                    (selected + 1).min(series.frames.len().saturating_sub(1))
                };
                if selected < series.frames.len() {
                    series.frames.swap(selected, other);
                    series.ordering = "Manual sequence".into();
                    series.revise();
                }
            }
            4 => {
                for (group, label) in additions {
                    if !series.frames.iter().any(|f| f.group == group) {
                        series.frames.push(SeriesFrame {
                            id: crate::group_identity::GroupId::new_result(),
                            group,
                            label,
                            sequence: 0,
                            coordinate: None,
                            acquired_at: None,
                        });
                    }
                }
                series.revise();
            }
            5 => {
                if series.frames.len() <= 1 {
                    self.measurements.message = "A series needs at least one frame.".into();
                    cx.notify();
                    return;
                }
                if selected < series.frames.len() {
                    series.frames.remove(selected);
                    series.revise();
                }
            }
            _ => return,
        }
        self.open_series_editor(cx);
        self.series_edited(cx);
    }

    fn coordinate_file(&mut self, import: bool, cx: &mut Context<Self>) {
        let Some(index) = self.measurements.selected_series else {
            return;
        };
        let series = self.measurements.archive.series[index].clone();
        let generation = self.project_generation;
        if import {
            let picker = cx.prompt_for_paths(gpui::PathPromptOptions {
                files: true,
                directories: false,
                multiple: false,
                prompt: Some("Import coordinates CSV".into()),
            });
            cx.spawn(async move |this, cx| {
                if let Ok(Ok(Some(paths))) = picker.await {
                    let Some(path) = paths.first() else { return };
                    let path = path.clone();
                    let original_revision = series.revision;
                    let result = cx
                        .background_spawn(async move {
                            let mut series = series;
                            use std::io::Read;
                            let mut bytes = Vec::new();
                            std::fs::File::open(path)
                                .map_err(|e| e.to_string())?
                                .take(16 * 1024 * 1024 + 1)
                                .read_to_end(&mut bytes)
                                .map_err(|e| e.to_string())?;
                            if bytes.len() > 16 * 1024 * 1024 {
                                return Err("Coordinate CSV exceeds 16 MiB".into());
                            }
                            let count = series.import_coordinates(&bytes)?;
                            Ok::<_, String>((series, count))
                        })
                        .await;
                    this.update(cx, |app, cx| {
                        if app.project_generation != generation {
                            return;
                        }
                        match result {
                            Ok((series, count))
                                if app.measurements.archive.series.get(index).is_some_and(
                                    |old| old.id == series.id && old.revision == original_revision,
                                ) =>
                            {
                                app.measurements.archive.series[index] = series;
                                app.open_series_editor(cx);
                                app.series_edited(cx);
                                app.measurements.message =
                                    format!("Imported coordinates for {count} frames.");
                            }
                            Ok(_) => {
                                app.measurements.message =
                                    "Series changed while importing; import the coordinates again."
                                        .into()
                            }
                            Err(e) => app.measurements.message = e,
                        }
                        cx.notify();
                    })
                    .ok();
                }
            })
            .detach();
        } else {
            let picker =
                cx.prompt_for_new_path(&std::env::temp_dir(), Some("series-coordinates.csv"));
            cx.spawn(async move |this, cx| {
                if let Ok(Ok(Some(path))) = picker.await {
                    let outcome = cx
                        .background_spawn(async move {
                            std::fs::write(path, series.coordinates_csv()?)
                                .map_err(|e| e.to_string())
                        })
                        .await;
                    this.update(cx, |app, cx| {
                        if app.project_generation != generation {
                            return;
                        }
                        app.measurements.message = outcome
                            .map(|_| {
                                "Coordinate CSV exported. Keep frame/group IDs when editing.".into()
                            })
                            .unwrap_or_else(|e| e);
                        cx.notify();
                    })
                    .ok();
                }
            })
            .detach();
        }
    }

    pub(super) fn measurement_management(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let t = self.theme;
        let running = self.measurements.cancel.is_some();
        let mut view = div().flex().flex_col().gap_2();
        if self.measurements.selected_series.is_none() {
            return view.into_any_element();
        }
        view = view.child(
            button(
                &t,
                "series-edit",
                if self.measurements.editor.is_some() {
                    "Close series editor"
                } else {
                    "Edit series…"
                },
                false,
            )
            .when(running, |d| d.disabled(true))
            .on_click(cx.listener(|app, _, _, cx| {
                if app.measurements.editor.is_some() {
                    app.measurements.editor = None;
                    cx.notify();
                } else {
                    app.open_series_editor(cx);
                }
            })),
        );
        let Some(editor) = &self.measurements.editor else {
            return view.into_any_element();
        };
        let series = &self.measurements.archive.series[self.measurements.selected_series.unwrap()];
        let mut panel = div()
            .p_3()
            .border_1()
            .border_color(t.border)
            .rounded_md()
            .flex()
            .flex_col()
            .gap_2();
        for row in editor.fields[..5].chunks(2) {
            let mut line = div().flex().gap_2();
            for input in row {
                line = line.child(div().flex_1().min_w_0().child(input.clone()));
            }
            panel = panel.child(line);
        }
        let mut buttons = div().flex().flex_wrap().gap_2();
        for (id, label) in [
            "Natural order",
            "Acquisition order",
            "Move up",
            "Move down",
            "Add marked",
            "Remove frame",
        ]
        .into_iter()
        .enumerate()
        {
            buttons = buttons.child(
                button(&t, ("series-action", id), label, false)
                    .when(running, |d| d.disabled(true))
                    .on_click(cx.listener(move |app, _, _, cx| app.edit_series_members(id, cx))),
            );
        }
        panel = panel.child(buttons);
        let start = editor.page * 10;
        for (i, frame) in series.frames.iter().enumerate().skip(start).take(10) {
            panel = panel.child(
                button(
                    &t,
                    ("series-member", i),
                    format!("{} · {}", i + 1, frame.label),
                    editor.selected == i,
                )
                .justify_start()
                .on_click(cx.listener(move |app, _, _, cx| app.select_series_member(i, cx))),
            );
        }
        let pages = series.frames.len().div_ceil(10);
        panel = panel.child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    button(&t, "series-members-prev", "Previous members", false).on_click(
                        cx.listener(|app, _, _, cx| {
                            if let Some(e) = &mut app.measurements.editor {
                                e.page = e.page.saturating_sub(1);
                            }
                            cx.notify();
                        }),
                    ),
                )
                .child(format!("{} / {pages}", editor.page + 1))
                .child(
                    button(&t, "series-members-next", "Next members", false).on_click(cx.listener(
                        move |app, _, _, cx| {
                            if let Some(e) = &mut app.measurements.editor {
                                e.page = (e.page + 1).min(pages.saturating_sub(1));
                            }
                            cx.notify();
                        },
                    )),
                ),
        );
        panel = panel.child(format!(
            "Selected frame {}: coordinate and acquisition timestamp",
            editor.selected + 1
        ));
        for input in &editor.fields[5..] {
            panel = panel.child(input.clone());
        }
        panel = panel.child(
            div()
                .flex()
                .flex_wrap()
                .gap_2()
                .child(
                    button(&t, "series-save-metadata", "Save series metadata", true)
                        .when(running, |d| d.disabled(true))
                        .on_click(cx.listener(|app, _, _, cx| app.save_series_metadata(cx))),
                )
                .child(
                    button(
                        &t,
                        "series-coordinate-export",
                        "Export coordinates CSV…",
                        false,
                    )
                    .on_click(cx.listener(|app, _, _, cx| app.coordinate_file(false, cx))),
                )
                .child(
                    button(
                        &t,
                        "series-coordinate-import",
                        "Import coordinates CSV…",
                        false,
                    )
                    .when(running, |d| d.disabled(true))
                    .on_click(cx.listener(|app, _, _, cx| app.coordinate_file(true, cx))),
                ),
        );
        view.child(panel).into_any_element()
    }
}
