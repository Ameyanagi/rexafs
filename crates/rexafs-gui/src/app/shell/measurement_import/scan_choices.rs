//! Per-scan import drafts and atomic multi-scan conversion.
use super::{DerivedSpectrum, ImportConfig, MeasurementImport};

#[derive(Clone)]
pub(super) struct ScanChoice {
    signal: Option<usize>,
    confirmed: bool,
    included: Vec<bool>,
    configs: Vec<ImportConfig>,
    errors: Vec<Option<String>>,
    custom: Option<ImportConfig>,
    custom_error: Option<String>,
}

impl ScanChoice {
    pub(super) fn capture(source: &MeasurementImport) -> Self {
        Self {
            signal: source.signal,
            confirmed: source.confirmed,
            included: source.included.clone(),
            configs: source.configs.clone(),
            errors: source.candidate_errors.clone(),
            custom: source.custom_config.clone(),
            custom_error: source.custom_error.clone(),
        }
    }

    pub(super) fn restore(self, source: &mut MeasurementImport) {
        source.signal = self.signal;
        source.confirmed = self.confirmed;
        source.included = self.included;
        source.configs = self.configs;
        source.candidate_errors = self.errors;
        source.custom_config = self.custom;
        source.custom_error = self.custom_error;
    }

    fn count(&self) -> usize {
        if self.signal.is_none() {
            usize::from(self.confirmed)
        } else {
            self.included.iter().filter(|&&v| v).count()
        }
    }

    fn valid(&self) -> bool {
        if self.signal.is_none() {
            self.confirmed && self.custom_error.is_none()
        } else {
            self.count() > 0
                && self
                    .included
                    .iter()
                    .zip(&self.errors)
                    .all(|(include, error)| !include || error.is_none())
        }
    }
}

impl MeasurementImport {
    /// Use the retained record identity when display labels repeat, as in HDF5
    /// entry and instrument tables with the same sample name.
    pub fn scan_label(&self, index: usize) -> &str {
        let scan = &self.document.scans[index];
        if self
            .document
            .scans
            .iter()
            .enumerate()
            .any(|(i, other)| i != index && other.label == scan.label)
        {
            &scan.id
        } else {
            &scan.label
        }
    }

    /// Number of selected output spectra and readiness for one source scan.
    /// Readiness never infers a mapping for an undeclared axis or signal.
    pub fn scan_status(&self, index: usize) -> (usize, bool) {
        if index == self.scan {
            (self.included_count(), self.included_valid())
        } else {
            self.choices
                .get(index)
                .and_then(Option::as_ref)
                .map_or((0, false), |choice| (choice.count(), choice.valid()))
        }
    }

    pub fn import_count(&self) -> usize {
        self.selected_scans
            .iter()
            .enumerate()
            .filter(|(_, selected)| **selected)
            .map(|(index, _)| self.scan_status(index).0)
            .sum()
    }

    pub fn import_ready(&self) -> bool {
        self.selected_scans.iter().any(|&selected| selected)
            && self
                .selected_scans
                .iter()
                .enumerate()
                .all(|(index, selected)| !selected || self.scan_status(index).1)
    }

    /// Convert every selected scan before appending any group. A failure returns
    /// its scan label and leaves the project unchanged; no scan is silently lost.
    /// Each scan retains its own checked signals, mappings and original evidence.
    pub fn materialize_import(
        &self,
        current: &ImportConfig,
    ) -> Result<Vec<DerivedSpectrum>, String> {
        let mut source = self.clone();
        source.remember_config(current);
        if !source.import_ready() {
            return Err("Select scans and resolve their signal mappings before importing.".into());
        }
        let selected = source.selected_scans.clone();
        let mut groups = Vec::new();
        for (index, include) in selected.into_iter().enumerate() {
            if !include {
                continue;
            }
            source.select_scan(index);
            let converted = source
                .materialize_selected(&source.config())
                .map_err(|error| format!("{}: {error}", source.document.scans[index].label))?;
            groups.extend(converted);
        }
        Ok(groups)
    }
}
