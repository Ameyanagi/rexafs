use super::*;
use crate::{app::shell::button, widgets::text_input::TextInput};
use gpui::{IntoElement, ParentElement, Styled, div, px};

#[derive(serde::Serialize, serde::Deserialize)]
pub(super) struct PresetFile {
    pub schema: u32,
    pub definition: MetricDefinition,
}

impl PresetFile {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != 1 {
            return Err("Unsupported measurement preset schema".into());
        }
        let d = &self.definition;
        if d.name.trim().is_empty() || d.name.len() > 200 {
            return Err("Preset names need 1–200 characters".into());
        }
        let (lo, hi) = d.measurement.metric.bounds();
        if !lo.is_finite()
            || !hi.is_finite()
            || (!matches!(d.measurement.metric, Metric::Point { .. }) && hi <= lo)
        {
            return Err("Preset needs finite coordinates and a positive region width".into());
        }
        if matches!(d.measurement.metric, Metric::Centroid { .. }) {
            return Err("Centroid presets are not yet editable in the desktop".into());
        }
        if matches!(d.measurement.origin, AxisOrigin::Reference { .. }) {
            return Err("Fixed-reference origins are not yet editable in the desktop".into());
        }
        if matches!(
            d.measurement.space,
            MeasurementSpace::Chi { .. } | MeasurementSpace::Fourier
        ) && d.measurement.origin != AxisOrigin::Absolute
        {
            return Err("k and R coordinates must be absolute".into());
        }
        Ok(())
    }
}

impl StudioApp {
    fn save_measurement_preset(&mut self, cx: &mut Context<Self>) {
        let name = self
            .measurements
            .preset_name
            .as_ref()
            .map(|f| f.read(cx).text().trim().to_string())
            .unwrap_or_default();
        let result = (|| {
            let mut definition = self.measurement_definition(cx)?;
            definition.name = name;
            let old = self
                .measurements
                .archive
                .presets
                .iter()
                .position(|p| p.name == definition.name);
            if let Some(i) = old {
                let previous = &self.measurements.archive.presets[i];
                definition.id = previous.id.clone();
                definition.revision = previous.revision
                    + u64::from(
                        previous.measurement != definition.measurement
                            || previous.edge_energy != definition.edge_energy,
                    );
            } else {
                definition.id = crate::group_identity::GroupId::new_result();
                definition.revision = 1;
            }
            definition.revision = self.measurements.archive.definition_revision(&definition);
            PresetFile {
                schema: 1,
                definition: definition.clone(),
            }
            .validate()?;
            self.measurements.selected_preset = Some(definition.id.clone());
            if let Some(i) = old {
                self.measurements.archive.presets[i] = definition;
            } else {
                self.measurements.archive.presets.push(definition);
            }
            Ok::<_, String>(())
        })();
        self.measurements.message = result
            .map(|_| "Preset saved in this project. Export it to reuse in another project.".into())
            .unwrap_or_else(|e| e);
        cx.notify();
    }

    fn apply_measurement_preset(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(definition) = self.measurements.archive.presets.get(index).cloned() else {
            return;
        };
        self.measurements.selected_preset = Some(definition.id.clone());
        self.measurements.kind = if definition.edge_energy {
            4
        } else {
            match definition.measurement.metric {
                Metric::Point { .. } => 0,
                Metric::Maximum { .. } => 1,
                Metric::Integral { .. } => 2,
                _ => 3,
            }
        };
        self.measurements.space = definition.measurement.space;
        self.measurements.relative = definition.measurement.origin == AxisOrigin::E0;
        let (lo, hi) = definition.measurement.metric.bounds();
        for (field, value) in self.measurements.fields.iter().zip([lo, hi]) {
            field.update(cx, |f, cx| f.set_value(Some(value), cx));
        }
        if let Some(name) = &self.measurements.preset_name {
            name.update(cx, |f, cx| f.set_text(definition.name, cx));
        }
        self.measurements.pick_range = None;
        self.preview_measurement(cx);
        cx.notify();
    }

    fn preset_file(&mut self, import: bool, cx: &mut Context<Self>) {
        let generation = self.project_generation;
        if import {
            let picker = cx.prompt_for_paths(gpui::PathPromptOptions {
                files: true,
                directories: false,
                multiple: false,
                prompt: Some("Import measurement preset".into()),
            });
            cx.spawn(async move|this,cx|{
                if let Ok(Ok(Some(paths)))=picker.await{
                    let Some(path)=paths.into_iter().next() else{return};
                    let result=cx.background_spawn(async move{
                        use std::io::Read;
                        let mut bytes=Vec::new();std::fs::File::open(path).map_err(|e|e.to_string())?.take(1024*1024+1).read_to_end(&mut bytes).map_err(|e|e.to_string())?;
                        if bytes.len()>1024*1024{return Err("Preset exceeds 1 MiB".into());}
                        let mut preset:PresetFile=serde_json::from_slice(&bytes).map_err(|e|e.to_string())?;preset.validate()?;
                        preset.definition.id=crate::group_identity::GroupId::new_result();preset.definition.revision=1;
                        Ok::<_,String>(preset.definition)
                    }).await;
                    this.update(cx,|app,cx|{if app.project_generation!=generation{return;}
                        match result{Ok(mut d)=>{
                            let base=d.name.clone();let mut suffix=2;
                            while app.measurements.archive.presets.iter().any(|p|p.name==d.name){d.name=format!("{base} ({suffix})");suffix+=1;}
                            app.measurements.archive.presets.push(d);let index=app.measurements.archive.presets.len()-1;app.apply_measurement_preset(index,cx);app.measurements.message="Preset imported; review its units and range before calculating.".into();
                        },Err(e)=>app.measurements.message=e}cx.notify();
                    }).ok();
                }
            }).detach();
        } else {
            let Some(definition) = self
                .measurements
                .archive
                .presets
                .iter()
                .find(|p| Some(&p.id) == self.measurements.selected_preset.as_ref())
                .cloned()
            else {
                self.measurements.message = "Save or select a preset first.".into();
                cx.notify();
                return;
            };
            let picker =
                cx.prompt_for_new_path(&std::env::temp_dir(), Some("measurement-preset.json"));
            cx.spawn(async move |this, cx| {
                if let Ok(Ok(Some(path))) = picker.await {
                    let result = cx
                        .background_spawn(async move {
                            let bytes = serde_json::to_vec_pretty(&PresetFile {
                                schema: 1,
                                definition,
                            })
                            .map_err(|e| e.to_string())?;
                            std::fs::write(path, bytes).map_err(|e| e.to_string())
                        })
                        .await;
                    this.update(cx, |app, cx| {
                        if app.project_generation != generation {
                            return;
                        }
                        app.measurements.message = result
                            .map(|_| "Measurement preset exported.".into())
                            .unwrap_or_else(|e| e);
                        cx.notify();
                    })
                    .ok();
                }
            })
            .detach();
        }
    }

    pub(super) fn measurement_presets(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let t = self.theme;
        if self.measurements.preset_name.is_none() {
            self.measurements.preset_name =
                Some(cx.new(|cx| TextInput::new("Measurement preset name", "", t, cx)));
        }
        let mut view = div().flex().flex_col().gap_2();
        view = view.child(
            div()
                .flex()
                .flex_wrap()
                .gap_2()
                .items_center()
                .child(
                    div()
                        .w(px(230.))
                        .child(self.measurements.preset_name.clone().unwrap()),
                )
                .child(
                    button(&t, "measurement-preset-save", "Save preset", false)
                        .on_click(cx.listener(|app, _, _, cx| app.save_measurement_preset(cx))),
                )
                .child(
                    button(&t, "measurement-preset-import", "Import preset…", false)
                        .on_click(cx.listener(|app, _, _, cx| app.preset_file(true, cx))),
                )
                .child(
                    button(&t, "measurement-preset-export", "Export preset…", false)
                        .on_click(cx.listener(|app, _, _, cx| app.preset_file(false, cx))),
                ),
        );
        let mut presets = div().flex().flex_wrap().gap_2();
        for (i, preset) in self.measurements.archive.presets.iter().enumerate() {
            presets = presets.child(
                button(
                    &t,
                    ("measurement-preset", i),
                    format!("{} · v{}", preset.name, preset.revision),
                    self.measurements.selected_preset.as_ref() == Some(&preset.id),
                )
                .on_click(cx.listener(move |app, _, _, cx| app.apply_measurement_preset(i, cx))),
            );
        }
        view.child(presets).into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exported_presets_retain_units_and_reject_uneditable_interpretations() {
        let preset = PresetFile {
            schema: 1,
            definition: MetricDefinition {
                id: crate::group_identity::GroupId::new_result(),
                revision: 3,
                name: "White line".into(),
                measurement: Measurement::maximum(0.0..=30.0).flat(),
                edge_energy: false,
            },
        };
        let encoded = serde_json::to_vec(&preset).unwrap();
        let mut restored: PresetFile = serde_json::from_slice(&encoded).unwrap();
        restored.validate().unwrap();
        assert_eq!(
            restored.definition.measurement,
            preset.definition.measurement
        );
        assert_eq!(restored.definition.revision, 3);
        restored.schema = 2;
        assert!(restored.validate().is_err());
        restored.schema = 1;
        restored.definition.measurement.origin = AxisOrigin::Reference { energy_ev: 9000. };
        assert!(restored.validate().is_err());
        restored.definition.measurement = Measurement::mean(30.0..=0.0);
        assert!(restored.validate().is_err());
        restored.definition.measurement = Measurement::mean(3.0..=10.0).chi(2);
        restored.validate().unwrap();
        restored.definition.measurement.origin = AxisOrigin::E0;
        assert!(restored.validate().is_err());
    }
}
