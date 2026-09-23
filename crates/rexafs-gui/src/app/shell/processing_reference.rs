//! Choose an explicit processing template independently of the selected group.
use crate::{
    app::{DERIVED_BASE, NO_ENTRY, StudioApp},
    group_identity::GroupId,
    params::PipelineParams,
};

impl StudioApp {
    pub(crate) fn processing_reference_choices(&self) -> Vec<(usize, String)> {
        let hidden = self.live_hidden_sources();
        (0..self.catalog.len())
            .chain((0..self.derived.len()).map(|i| DERIVED_BASE + i))
            .chain(self.standalone_source.as_ref().map(|_| NO_ENTRY))
            .filter(|&i| {
                self.valid_group_index(i)
                    && !self.group_registry.index_excluded(i)
                    && !hidden.contains(&i)
            })
            .map(|i| (i, self.entry_label(i).to_string()))
            .collect()
    }

    /// Copy requested settings, preserving Auto values for per-spectrum
    /// resolution. A removed reference is an error, never an implicit fallback.
    pub(crate) fn processing_reference_settings(
        &self,
        reference: Option<&GroupId>,
    ) -> Result<PipelineParams, String> {
        let index = if let Some(id) = reference {
            self.group_registry
                .index(id)
                .or_else(|| {
                    self.standalone_source
                        .as_ref()
                        .filter(|(_, _, g)| g == id)
                        .map(|_| NO_ENTRY)
                })
                .filter(|&i| self.valid_group_index(i) && !self.group_registry.index_excluded(i))
                .ok_or("The processing reference is unavailable. Choose another spectrum.")?
        } else {
            self.selected.unwrap_or(NO_ENTRY)
        };
        Ok(self.effective_params(index).clone())
    }

    pub(crate) fn processing_reference_name(&self, reference: &GroupId) -> String {
        self.group_registry
            .index(reference)
            .map(|i| self.entry_label(i).to_string())
            .unwrap_or_else(|| "Unavailable reference".into())
    }
}
