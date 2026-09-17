//! Frozen processing and measurement choices for replay on compatible inputs.
use super::*;
use crate::{import_mapping::LayoutKey, params::Quantity};

/// Compatibility concerns scientific interpretation, not filenames or values.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum InputContract {
    Table(LayoutKey),
    Materialized(Quantity),
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AnalysisRecipe {
    pub schema: u32,
    pub id: GroupId,
    pub revision: u64,
    pub name: String,
    pub definition: MetricDefinition,
    pub settings: PipelineParams,
    pub input: InputContract,
}

impl AnalysisRecipe {
    /// Capture the preview's interpretation and requested processing settings.
    /// Automatic fields stay automatic; resolved values belong to each result.
    pub fn capture(
        name: String,
        definition: MetricDefinition,
        input: &FrameInput,
    ) -> Result<Self, String> {
        let bytes = input.source_bytes()?;
        let recipe = Self {
            schema: 1,
            id: GroupId::new_result(),
            revision: 1,
            name,
            definition,
            settings: input.settings.clone(),
            input: input.contract(&bytes)?,
        };
        recipe.validate()?;
        // Capture only a definition that can actually run on the representative input.
        let (_, revision) = input.revision_of(&bytes);
        let (spectrum, _, _) = input.prepare(&recipe.definition, Some(&revision))?;
        if let Some(w) = &recipe.definition.wavelet {
            let map = spectrum.wavelet(&w.transform).map_err(|e| e.to_string())?;
            w.measure(&map)?;
        } else if recipe.definition.edge_energy {
            spectrum.e0().ok_or("Edge energy unavailable")?;
        } else {
            spectrum
                .measure(&recipe.definition.measurement)
                .map_err(|e| e.to_string())?;
        }
        Ok(recipe)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema != 1 {
            return Err("Unsupported analysis recipe schema".into());
        }
        if self.name.trim().is_empty() || self.name.chars().count() > 200 {
            return Err("Recipe names need 1–200 characters".into());
        }
        if let Some(w) = &self.definition.wavelet {
            w.validate()?;
        }
        let bytes = serde_json::to_vec(&self.settings).map_err(|e| e.to_string())?;
        let restored: PipelineParams = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        if restored != self.settings {
            return Err("Recipe settings must contain finite, serializable values".into());
        }
        if matches!(self.input, InputContract::Materialized(_)) && self.settings.align_to_ref {
            return Err(
                "Reference-channel alignment needs a file with a retained reference channel".into(),
            );
        }
        Ok(())
    }

    pub fn same_choices(&self, other: &Self) -> bool {
        self.settings == other.settings
            && self.input == other.input
            && self.definition.measurement == other.definition.measurement
            && self.definition.edge_energy == other.definition.edge_energy
            && self.definition.wavelet == other.definition.wavelet
    }
}

impl FrameInput {
    fn contract(&self, bytes: &[u8]) -> Result<InputContract, String> {
        if let Some(group) = &self.derived
            && group.source.is_none()
        {
            if group.quantity_unconfirmed {
                return Err("Confirm the source quantity before using an analysis recipe".into());
            }
            return Ok(InputContract::Materialized(group.quantity));
        }
        let path = self
            .derived
            .as_ref()
            .and_then(|g| g.source.as_ref())
            .unwrap_or(&self.path);
        let detection = params::detect_import_reader(bytes, path, &self.settings.import)?
            .ok_or("Input layout is not available in the detection prefix; review the source")?;
        if let Some(error) = detection.mapping_error {
            return Err(error);
        }
        Ok(InputContract::Table(LayoutKey::from_detection(&detection)))
    }

    pub fn with_recipe(mut self, recipe: Option<Arc<AnalysisRecipe>>) -> Self {
        if let Some(recipe) = &recipe {
            self.settings = recipe.settings.clone();
        }
        self.recipe = recipe;
        self
    }

    pub(super) fn validate_recipe(&self, bytes: &[u8]) -> Result<(), String> {
        if let Some(recipe) = &self.recipe {
            if self.contract(bytes)? != recipe.input {
                return Err("Recipe input is incompatible: column layout, units, conversion metadata or source quantity changed. Review the input and save a new recipe.".into());
            }
        }
        Ok(())
    }
}

impl SeriesArchive {
    /// Append immutable versions, including after returning to an earlier choice.
    pub fn save_recipe(&mut self, mut recipe: AnalysisRecipe) -> GroupId {
        if let Some(old) = self.recipes.iter().rev().find(|r| r.name == recipe.name) {
            recipe.id = old.id.clone();
            if old.same_choices(&recipe) {
                return old.id.clone();
            }
            recipe.revision = old.revision + 1;
        }
        let id = recipe.id.clone();
        self.recipes.push(Arc::new(recipe));
        id
    }
}
