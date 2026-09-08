//! Import evidence and explicit, identity-bound repair entry points.
use super::*;
use crate::{
    app::StudioApp,
    import_mapping::{AxisConversion, LayoutKey, MappingDraft},
    params::DetectionMode,
};

fn action(
    t: &Theme,
    id: impl Into<gpui::ElementId>,
    label: impl Into<SharedString>,
) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .flex_none()
        .px_2()
        .py_1()
        .rounded_md()
        .border_1()
        .border_color(t.border)
        .bg(t.raised)
        .text_color(t.text)
        .text_size(px(11.5))
        .cursor_pointer()
        .hover(|d| d.border_color(t.accent))
        .child(label.into())
}

impl StudioApp {
    pub(crate) fn import_summary(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let t = self.theme;
        let mut card = div()
            .flex()
            .flex_col()
            .flex_none()
            .gap_2()
            .p_3()
            .text_size(px(12.));
        let target = self
            .current_group_index()
            .and_then(|ix| self.tool_target(ix));
        let Some(target) = target.filter(|t| !t.path.as_os_str().is_empty()) else {
            card = card.child(section_label(&t, "Inputs and operation"));
            if let Some(result) = self
                .current_group_index()
                .and_then(|ix| ix.checked_sub(crate::app::DERIVED_BASE))
                .and_then(|ix| self.derived.get(ix))
            {
                card = card.child(format!("Quantity: {}", result.quantity.label()));
                if let Some(operation) = &result.operation {
                    card = card.child(format!("Operation: {}", operation.tool));
                    for input in &operation.inputs {
                        card = card.child(format!("Input: {}", input.label));
                    }
                    card = card
                        .child(section_label(&t, "Energy correction"))
                        .child(format!(
                            "{:+.4} eV · applied by {}",
                            operation.applied_energy_shift_ev, operation.tool
                        ));
                } else {
                    card = card.child("No recorded operation for these stored arrays.");
                }
            }
            return card.into_any_element();
        };
        let Some(group) = target.group_id.clone() else {
            return card.into_any_element();
        };
        card = card.child(section_label(&t, "Import")).child(format!(
            "Source: {}",
            target
                .path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
        ));
        let preview = self.import_preview.as_ref();
        let draft = preview.map(|p| MappingDraft::new(p, &self.ui_params().import));
        if let Some(draft) = &draft {
            card = card
                .child(format!("Channel: {}", draft.channel().label()))
                .child(draft.formula());
            let column = draft.config().energy_col.unwrap_or(0);
            let units = preview
                .map(LayoutKey::from_preview)
                .and_then(|key| key.units.get(column).cloned().flatten());
            let axis = match draft.config().axis {
                AxisConversion::EnergyEv => "eV".into(),
                AxisConversion::EnergyKev => "keV → eV".into(),
                AxisConversion::AngleDegrees { d_spacing } => {
                    format!("degrees → eV · d = {d_spacing} Å")
                }
                AxisConversion::AngleRadians { d_spacing } => {
                    format!("radians → eV · d = {d_spacing} Å")
                }
                AxisConversion::Auto => units
                    .map(|u| format!("{u} → eV"))
                    .unwrap_or("eV assumed; units absent".into()),
            };
            card = card.child(format!("Energy: {} · {axis}", draft.column_label(column)));
        } else {
            card = card.child(if self.import_preview_error.is_empty() {
                "Reading source interpretation…".into()
            } else {
                self.import_preview_error.to_string()
            });
        }
        let application = self.imports.application_for(&group);
        if let Some(application) = application {
            let recipe = self.imports.recipes.get(&application.recipe);
            let changed = application
                .members
                .iter()
                .find(|m| m.group == group)
                .is_some_and(|m| {
                    m.mapping_revision
                        != crate::import_recipes::mapping_revision(&self.ui_params().import)
                });
            let label = recipe.map(|r| r.label()).unwrap_or("Saved recipe".into());
            card = card.child(if changed {
                format!("Mapping: manually changed · originally {label}")
            } else {
                format!("Mapping: {label}")
            });
            if recipe.is_some_and(|r| !r.reuse) {
                card = card.child("Automatic recipe reuse is stopped.");
            }
        } else {
            card = card.child("Mapping: detected or manually assigned · no reusable recipe");
        }
        if let Some(preview) = preview {
            card = card.child(if preview.signal_error.is_none() {
                format!(
                    "Spectrum checked · {} points",
                    preview.diagnostics.valid_points
                )
            } else {
                format!(
                    "Spectrum needs repair · {}",
                    preview.signal_error.as_deref().unwrap()
                )
            });
            if !preview.diagnostics.warnings().is_empty() {
                card = card.child(format!(
                    "{} parser warning categories · Source details",
                    preview.diagnostics.warnings().len()
                ));
            }
        }
        card = card.child(section_label(&t, "Energy correction"));
        let params = self.ui_params();
        card = card.child(if params.align_to_ref {
            params
                .align_target
                .map(|target| format!("Saved reference alignment · target {target:.3} eV"))
                .unwrap_or("0 eV · reference alignment has no target".into())
        } else {
            "0 eV · original source axis".into()
        });
        let id = group.clone();
        card = card.child(
            action(&t, "remap-current", "Re-map columns…").on_click(cx.listener(
                move |app, _, window, cx| {
                    if let Some(ix) = app.menu_index(&id) {
                        app.open_import_editor(ix, window, cx);
                    }
                },
            )),
        );
        if application.is_some() {
            let id = group.clone();
            card = card.child(
                action(&t, "repair-import-application", "Review this application…").on_click(
                    cx.listener(move |app, _, window, cx| {
                        if let Some(ix) = app.menu_index(&id) {
                            app.open_application_editor(ix, window, cx);
                        }
                    }),
                ),
            );
        }
        card = card.child(
            action(
                &t,
                "import-source-details",
                if self.adv_open[3] {
                    "Hide source details"
                } else {
                    "Source details…"
                },
            )
            .on_click(cx.listener(|app, _, _, cx| {
                app.adv_open[3] = !app.adv_open[3];
                cx.notify();
            })),
        );
        if self.adv_open[3] {
            card = card.child(target.path.display().to_string());
            if let Some(preview) = preview {
                if let Some(header) = &preview.xdi {
                    for key in [
                        "element.symbol",
                        "element.edge",
                        "sample.name",
                        "sample.temperature",
                        "beamline.name",
                        "facility.name",
                        "scan.start_time",
                    ] {
                        if let Some(value) = header.get(key) {
                            card = card.child(format!("{key}: {value}"));
                        }
                    }
                }
                card = card.children(
                    preview
                        .diagnostics
                        .warnings()
                        .into_iter()
                        .map(|warning| div().text_color(t.warn).child(warning)),
                );
            }
            if let Some(recipe) = application
                .and_then(|a| self.imports.recipes.get(&a.recipe))
                .filter(|r| r.reuse)
            {
                let recipe_id = recipe.reference.id.clone();
                card = card.child(
                    action(&t, "stop-import-recipe", "Stop reusing this recipe").on_click(
                        cx.listener(move |app, _, _, cx| {
                            app.imports.recipes.stop_reusing(&recipe_id);
                            app.structure
                                .settings
                                .import_recipes
                                .stop_reusing(&recipe_id);
                            if let Err(error) = app.structure.settings.save() {
                                app.record_job_error("Stop recipe reuse", error);
                            }
                            cx.notify();
                        }),
                    ),
                );
            }
        }
        card = card.child(section_label(&t, "Channels for this file"));
        let paths = self.import_application_paths(&target);
        let modes = [
            DetectionMode::Transmission,
            DetectionMode::Fluorescence,
            DetectionMode::Reference,
            DetectionMode::MuColumn,
        ];
        for mode in modes {
            let present = draft.as_ref().is_some_and(|d| d.channel() == mode)
                || self.source_has_channel_canonical(&target.path, mode);
            if present {
                card = card.child(format!("{} · present", mode.label()));
            } else {
                let id = group.clone();
                card = card.child(
                    action(
                        &t,
                        SharedString::from(format!("import-add-{mode:?}")),
                        format!("Add {} for this file…", mode.label()),
                    )
                    .on_click(cx.listener(move |app, _, window, cx| {
                        if let Some(ix) = app.menu_index(&id) {
                            app.open_channel_editor(ix, mode, window, cx);
                        }
                    })),
                );
            }
        }
        if paths.len() > 1 {
            card = card
                .child(section_label(&t, "Import application"))
                .child(format!(
                    "{} source files · independent of marks and filters",
                    paths.len()
                ));
            let uncertain = paths.iter().any(|path| {
                self.catalog
                    .find_by_canonical_path(path)
                    .is_some_and(|ix| self.effective_params(ix).import.mode == DetectionMode::Auto)
            });
            for mode in modes {
                let missing = paths
                    .iter()
                    .filter(|path| !self.source_has_channel_canonical(path, mode))
                    .count();
                if missing == 0 {
                    continue;
                }
                let id = group.clone();
                let label = if uncertain {
                    format!("Review {} for all {} files…", mode.label(), paths.len())
                } else {
                    format!(
                        "Add {} for {missing} files… · {} already present",
                        mode.label(),
                        paths.len() - missing
                    )
                };
                card = card.child(
                    action(
                        &t,
                        SharedString::from(format!("import-batch-{mode:?}")),
                        label,
                    )
                    .on_click(cx.listener(move |app, _, window, cx| {
                        if let Some(ix) = app.menu_index(&id) {
                            app.open_batch_channel_editor(ix, mode, window, cx);
                        }
                    })),
                );
            }
        }
        card.into_any_element()
    }
}
