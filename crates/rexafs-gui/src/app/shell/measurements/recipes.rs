use super::*;
use crate::app::shell::button;
use gpui::{IntoElement, ParentElement, Styled, div};

impl StudioApp {
    pub(super) fn selected_analysis_recipe(&self) -> Option<Arc<AnalysisRecipe>> {
        self.measurements
            .archive
            .recipes
            .iter()
            .rev()
            .find(|r| {
                self.measurements
                    .selected_recipe
                    .as_ref()
                    .is_some_and(|(id, version)| *id == r.id && *version == r.revision)
            })
            .cloned()
    }

    fn apply_analysis_recipe(&mut self, recipe: Arc<AnalysisRecipe>, cx: &mut Context<Self>) {
        self.measurements.selected_recipe = Some((recipe.id.clone(), recipe.revision));
        self.measurements.selected_preset = None;
        self.load_measurement_definition(recipe.definition.clone(), cx);
        if let Some(name) = &self.measurements.preset_name {
            name.update(cx, |f, cx| f.set_text(recipe.name.clone(), cx));
        }
        self.measurements.message = format!(
            "{} · v{}: replay uses frozen processing and input interpretation.",
            recipe.name, recipe.revision
        );
        cx.notify();
    }

    fn save_analysis_recipe(&mut self, cx: &mut Context<Self>) {
        let Some(frame) = self
            .measurements
            .selected_series
            .and_then(|i| self.measurements.archive.series.get(i))
            .and_then(|s| {
                s.frames.get(
                    self.measurements
                        .preview_index
                        .min(s.frames.len().saturating_sub(1)),
                )
            })
        else {
            return;
        };
        let input = self
            .measurement_input(&frame.group, frame.label.clone())
            .with_recipe(self.selected_analysis_recipe());
        let definition = match self.measurement_definition(cx) {
            Ok(d) => d,
            Err(e) => {
                self.measurements.message = e;
                cx.notify();
                return;
            }
        };
        let name = self
            .measurements
            .preset_name
            .as_ref()
            .map(|f| f.read(cx).text().trim().to_string())
            .unwrap_or_default();
        let generation = self.project_generation;
        self.measurements.message = "Checking recipe on the preview frame…".into();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move { AnalysisRecipe::capture(name, definition, &input) })
                .await;
            this.update(cx, |app, cx| {
                if app.project_generation != generation {
                    return;
                }
                match result {
                    Ok(recipe) => {
                        let id = app.measurements.archive.save_recipe(recipe);
                        let version = app
                            .measurements
                            .archive
                            .recipes
                            .iter()
                            .rev()
                            .find(|r| r.id == id)
                            .unwrap()
                            .revision;
                        app.measurements.selected_recipe = Some((id, version));
                        let recipe = app.selected_analysis_recipe().unwrap();
                        app.apply_analysis_recipe(recipe, cx);
                    }
                    Err(error) => app.measurements.message = error,
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    fn analysis_recipe_file(&mut self, import: bool, cx: &mut Context<Self>) {
        let generation = self.project_generation;
        if import {
            let picker = cx.prompt_for_paths(gpui::PathPromptOptions {
                files: true,
                directories: false,
                multiple: false,
                prompt: Some("Import analysis recipe".into()),
            });
            cx.spawn(async move |this, cx| {
                let Ok(Ok(Some(paths))) = picker.await else {
                    return;
                };
                let Some(path) = paths.into_iter().next() else {
                    return;
                };
                let result = cx
                    .background_spawn(async move {
                        use std::io::Read;
                        let mut bytes = Vec::new();
                        std::fs::File::open(path)
                            .map_err(|e| e.to_string())?
                            .take(1024 * 1024 + 1)
                            .read_to_end(&mut bytes)
                            .map_err(|e| e.to_string())?;
                        if bytes.len() > 1024 * 1024 {
                            return Err("Recipe exceeds 1 MiB".into());
                        }
                        let mut recipe: AnalysisRecipe =
                            serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
                        recipe.validate()?;
                        presets::PresetFile {
                            schema: 1,
                            definition: recipe.definition.clone(),
                        }
                        .validate()?;
                        recipe.id = crate::group_identity::GroupId::new_result();
                        recipe.revision = 1;
                        recipe.definition.id = crate::group_identity::GroupId::new_result();
                        recipe.definition.revision = 1;
                        Ok::<_, String>(recipe)
                    })
                    .await;
                this.update(cx, |app, cx| {
                    if app.project_generation != generation {
                        return;
                    }
                    match result {
                        Ok(mut recipe) => {
                            let base = recipe.name.clone();
                            let mut suffix = 2;
                            while app
                                .measurements
                                .archive
                                .recipes
                                .iter()
                                .any(|r| r.name == recipe.name)
                            {
                                recipe.name = format!("{base} ({suffix})");
                                suffix += 1;
                            }
                            let recipe = Arc::new(recipe);
                            app.measurements.archive.recipes.push(recipe.clone());
                            app.apply_analysis_recipe(recipe, cx);
                        }
                        Err(error) => app.measurements.message = error,
                    }
                    cx.notify();
                })
                .ok();
            })
            .detach();
        } else {
            let Some(recipe) = self.selected_analysis_recipe() else {
                self.measurements.message = "Select a saved analysis recipe first.".into();
                cx.notify();
                return;
            };
            let picker =
                cx.prompt_for_new_path(&std::env::temp_dir(), Some("analysis-recipe.json"));
            cx.spawn(async move |this, cx| {
                let Ok(Ok(Some(path))) = picker.await else {
                    return;
                };
                let result = cx
                    .background_spawn(async move {
                        let bytes =
                            serde_json::to_vec_pretty(&recipe).map_err(|e| e.to_string())?;
                        let mut file = tempfile::NamedTempFile::new_in(
                            path.parent().ok_or("Recipe destination has no parent")?,
                        )
                        .map_err(|e| e.to_string())?;
                        use std::io::Write;
                        file.write_all(&bytes).map_err(|e| e.to_string())?;
                        file.as_file().sync_all().map_err(|e| e.to_string())?;
                        file.persist(path).map_err(|e| e.to_string())?;
                        Ok::<_, String>(())
                    })
                    .await;
                this.update(cx, |app, cx| {
                    if app.project_generation != generation {
                        return;
                    }
                    app.measurements.message = result
                        .map(|_| "Analysis recipe exported.".into())
                        .unwrap_or_else(|e| e);
                    cx.notify();
                })
                .ok();
            })
            .detach();
        }
    }

    pub(super) fn analysis_recipe_controls(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let t = self.theme;
        let busy = self.measurements.cancel.is_some();
        let mut choices = div().flex().flex_wrap().gap_2().items_center().child(
            button(
                &t,
                "measurement-group-settings",
                "Group settings",
                self.measurements.selected_recipe.is_none(),
            )
            .disabled(busy)
            .on_click(cx.listener(|app, _, _, cx| {
                app.measurements.selected_recipe = None;
                app.measurements.message =
                    "Measurements use each group's own processing settings.".into();
                app.preview_measurement(cx);
                cx.notify();
            })),
        );
        for (index, recipe) in self.measurements.archive.recipes.iter().enumerate() {
            let recipe = recipe.clone();
            choices = choices.child(
                button(
                    &t,
                    ("analysis-recipe", index),
                    format!("{} · v{}", recipe.name, recipe.revision),
                    self.measurements
                        .selected_recipe
                        .as_ref()
                        .is_some_and(|(id, version)| {
                            *id == recipe.id && *version == recipe.revision
                        }),
                )
                .disabled(busy)
                .on_click(
                    cx.listener(move |app, _, _, cx| app.apply_analysis_recipe(recipe.clone(), cx)),
                ),
            );
        }
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(choices)
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_2()
                    .child(
                        button(
                            &t,
                            "analysis-recipe-save",
                            "Save recipe from preview",
                            false,
                        )
                        .disabled(busy)
                        .on_click(cx.listener(|app, _, _, cx| app.save_analysis_recipe(cx))),
                    )
                    .child(
                        button(&t, "analysis-recipe-import", "Import recipe…", false)
                            .disabled(busy)
                            .on_click(
                                cx.listener(|app, _, _, cx| app.analysis_recipe_file(true, cx)),
                            ),
                    )
                    .child(
                        button(&t, "analysis-recipe-export", "Export recipe…", false)
                            .disabled(busy)
                            .on_click(
                                cx.listener(|app, _, _, cx| app.analysis_recipe_file(false, cx)),
                            ),
                    ),
            )
            .into_any_element()
    }
}
