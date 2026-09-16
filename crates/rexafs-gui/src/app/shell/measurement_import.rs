//! The universal reader supplies the established plotted import editor.
use super::*;
use crate::app::{DERIVED_BASE, import_preview::PreviewResult};
use crate::import_mapping::AxisConversion;
use crate::params::{
    DerivedSpectrum, DetectionMode, ImportConfig, ImportPreview, MeasurementLayout, Operation,
    ParserDiagnostics, RawData, ResolvedImport,
};
use base64::Engine as _;
use rexafs::io::{EnergyConversion, Measurement, SignalConversion, SpectrumMapping};
use std::{io::Read, path::PathBuf, sync::Arc};

mod batch;
mod scan_choices;
use scan_choices::ScanChoice;

/// One immutable source snapshot; editing a mapping never rereads or changes it.
#[derive(Clone)]
pub(crate) struct MeasurementImport {
    pub path: PathBuf,
    pub batch: Option<Arc<batch::BatchReview>>,
    pub apply_to_batch: bool,
    original_bytes: Arc<Vec<u8>>,
    pub document: Arc<Measurement>,
    pub scan: usize,
    pub signal: Option<usize>,
    pub confirmed: bool,
    pub included: Vec<bool>,
    pub configs: Vec<ImportConfig>,
    pub candidate_errors: Vec<Option<String>>,
    pub dataset_paths: Vec<String>,
    pub selected_scans: Vec<bool>,
    choices: Vec<Option<ScanChoice>>,
    custom_config: Option<ImportConfig>,
    custom_error: Option<String>,
}

fn initial_mapping(document: &Measurement, scan: usize) -> SpectrumMapping {
    document
        .scans
        .get(scan)
        .and_then(|s| s.signals.first())
        .map(|s| s.mapping.clone())
        .unwrap_or(SpectrumMapping {
            energy_column: 0,
            energy: EnergyConversion::Ev,
            signal: SignalConversion::Direct { column: 1 },
        })
}

impl MeasurementImport {
    pub fn new(path: PathBuf, document: Measurement, bytes: Vec<u8>) -> Self {
        let count = document.scans.len();
        let mut source = Self {
            path,
            batch: None,
            apply_to_batch: false,
            original_bytes: Arc::new(bytes),
            document: Arc::new(document),
            scan: usize::MAX,
            signal: None,
            confirmed: false,
            included: vec![],
            configs: vec![],
            candidate_errors: vec![],
            dataset_paths: vec![],
            selected_scans: vec![true; count],
            choices: vec![None; count],
            custom_config: None,
            custom_error: None,
        };
        // Validate detected outputs once, rather than on every render. Unknown
        // mappings remain selected and visibly block import until reviewed.
        for index in 0..count {
            source.select_scan(index);
        }
        source.select_scan(0);
        source
    }
    pub fn selected_mapping(&self) -> SpectrumMapping {
        self.document
            .scans
            .get(self.scan)
            .and_then(|s| s.signals.get(self.signal.unwrap_or(0)))
            .map(|s| s.mapping.clone())
            .unwrap_or_else(|| initial_mapping(&self.document, self.scan))
    }
    pub fn config(&self) -> ImportConfig {
        if let Some(config) = self.signal.and_then(|i| self.configs.get(i)) {
            return config.clone();
        }
        if let Some(config) = &self.custom_config {
            return config.clone();
        }
        self.original_config()
    }
    pub fn original_config(&self) -> ImportConfig {
        let mapping = self.selected_mapping();
        let mut config = self.config_for_mapping(&mapping);
        if self
            .document
            .scans
            .get(self.scan)
            .and_then(|scan| self.signal.and_then(|i| scan.signals.get(i)))
            .is_some_and(|signal| signal.name == "reference")
            && let SignalConversion::Transmission {
                incident,
                transmitted,
            } = mapping.signal
        {
            config.mode = DetectionMode::Reference;
            config.it_col = Some(incident);
            config.ir_col = Some(transmitted);
        }
        config
    }
    fn config_for_mapping(&self, mapping: &SpectrumMapping) -> ImportConfig {
        let mapping = mapping.clone();
        let mut config = ImportConfig {
            energy_col: Some(mapping.energy_column),
            ..Default::default()
        };
        match mapping.signal {
            SignalConversion::Direct { column } => {
                config.mode = DetectionMode::MuColumn;
                config.mu_col = Some(column);
            }
            SignalConversion::Transmission {
                incident,
                transmitted,
            } => {
                config.mode = DetectionMode::Transmission;
                config.i0_col = Some(incident);
                config.it_col = Some(transmitted);
            }
            SignalConversion::Ratio {
                detectors,
                incident,
            } => {
                config.mode = DetectionMode::Fluorescence;
                config.i0_col = Some(incident);
                config.fluor_cols = Some(detectors);
            }
        }
        config
    }
    pub fn select_scan(&mut self, index: usize) {
        if index >= self.document.scans.len() {
            return;
        }
        if self.scan < self.choices.len() {
            self.choices[self.scan] = Some(ScanChoice::capture(self));
        }
        self.scan = index;
        if let Some(choice) = self.choices[index].clone() {
            choice.restore(self);
            return;
        }
        self.configs.clear();
        self.candidate_errors.clear();
        self.included.clear();
        self.custom_config = None;
        self.custom_error = None;
        if let Some(scan) = self.document.scans.get(index) {
            for candidate in &scan.signals {
                let mut config = self.config_for_mapping(&candidate.mapping);
                if candidate.name == "reference" {
                    if let SignalConversion::Transmission {
                        incident,
                        transmitted,
                    } = candidate.mapping.signal
                    {
                        config.mode = DetectionMode::Reference;
                        config.it_col = Some(incident);
                        config.ir_col = Some(transmitted);
                    }
                }
                self.configs.push(config);
                let error = scan
                    .arrays(Some(&candidate.mapping))
                    .err()
                    .map(|e| e.to_string());
                self.included.push(error.is_none());
                self.candidate_errors.push(error);
            }
        }
        self.signal = (!self.configs.is_empty()).then(|| {
            self.candidate_errors
                .iter()
                .position(Option::is_none)
                .unwrap_or(0)
        });
        self.confirmed = self.signal.is_some();
    }
    pub fn remember_config(&mut self, config: &ImportConfig) {
        let error = self
            .mapping(config)
            .and_then(|m| {
                self.document.scans[self.scan]
                    .arrays(Some(&m))
                    .map_err(|e| e.to_string())
            })
            .err();
        if let Some(index) = self.signal {
            self.configs[index] = config.clone();
            self.candidate_errors[index] = error;
        } else {
            self.custom_config = Some(config.clone());
            self.custom_error = error;
        }
    }
    pub fn included_count(&self) -> usize {
        if self.signal.is_none() {
            usize::from(self.confirmed)
        } else {
            self.included.iter().filter(|v| **v).count()
        }
    }
    pub fn included_valid(&self) -> bool {
        if self.signal.is_none() {
            return self.confirmed && self.custom_error.is_none();
        }
        self.included_count() > 0
            && self
                .included
                .iter()
                .zip(&self.candidate_errors)
                .all(|(include, error)| !include || error.is_none())
    }
    pub fn preview_included(&self) -> bool {
        self.selected_scans.get(self.scan) == Some(&true)
            && self.signal.is_none_or(|index| self.included[index])
    }
    /// Convert every checked output before adding any group to the project.
    pub fn materialize_selected(
        &self,
        current: &ImportConfig,
    ) -> Result<Vec<DerivedSpectrum>, String> {
        if self.signal.is_none() {
            return self.materialize(current).map(|g| vec![g]);
        }
        if !self.included_valid() {
            return Err("Select at least one valid signal to import.".into());
        }
        let mut groups = Vec::new();
        for (index, include) in self.included.iter().enumerate() {
            if !include {
                continue;
            }
            let mut source = self.clone();
            source.signal = Some(index);
            let config = if self.signal == Some(index) {
                current
            } else {
                &self.configs[index]
            };
            let mut group = source.materialize(config)?;
            let name = &self.document.scans[self.scan].signals[index].name;
            if self.configs.len() > 1 {
                group.label = format!("{} · {name}", group.label);
            }
            if let Some(operation) = &mut group.operation {
                operation.parameters["signal_name"] = name.clone().into();
            }
            groups.push(group);
        }
        Ok(groups)
    }
    pub fn mapping(&self, config: &ImportConfig) -> Result<SpectrumMapping, String> {
        let scan = self
            .document
            .scans
            .get(self.scan)
            .ok_or("Select matching one-dimensional datasets.")?;
        let col = config
            .energy_col
            .ok_or("Choose an energy or angle column.")?;
        let selected = self.selected_mapping();
        let energy = match config.axis {
            AxisConversion::Auto => {
                if scan.signals.get(self.signal.unwrap_or(0)).is_some()
                    && col == selected.energy_column
                {
                    selected.energy
                } else {
                    match scan.columns.get(col).and_then(|c| c.units.as_deref()).map(str::to_ascii_lowercase).as_deref() {
                    Some("ev") => EnergyConversion::Ev, Some("kev") => EnergyConversion::Kev,
                    _ => return Err("Choose the axis units; the source does not specify an energy conversion.".into()),
                }
                }
            }
            AxisConversion::EnergyEv => EnergyConversion::Ev,
            AxisConversion::EnergyKev => EnergyConversion::Kev,
            AxisConversion::AngleDegrees { d_spacing } => EnergyConversion::Bragg {
                d_spacing,
                degrees_per_unit: 1.,
            },
            AxisConversion::AngleRadians { d_spacing } => EnergyConversion::Bragg {
                d_spacing,
                degrees_per_unit: 180. / std::f64::consts::PI,
            },
        };
        let signal = match config.mode {
            DetectionMode::MuColumn => SignalConversion::Direct {
                column: config.mu_col.ok_or("Choose a signal column.")?,
            },
            DetectionMode::Transmission => SignalConversion::Transmission {
                incident: config.i0_col.ok_or("Choose I0.")?,
                transmitted: config.it_col.ok_or("Choose It.")?,
            },
            DetectionMode::Reference => match config.reference_mu_col {
                Some(column) => SignalConversion::Direct { column },
                None => SignalConversion::Transmission {
                    incident: config.it_col.ok_or("Choose It.")?,
                    transmitted: config.ir_col.ok_or("Choose Ir.")?,
                },
            },
            DetectionMode::Fluorescence => SignalConversion::Ratio {
                incident: config.i0_col.ok_or("Choose I0.")?,
                detectors: config.fluor_cols.clone().unwrap_or_default(),
            },
            DetectionMode::Auto => return Err("Choose a signal.".into()),
        };
        Ok(SpectrumMapping {
            energy_column: col,
            energy,
            signal,
        })
    }
    pub fn preview(&self, config: &ImportConfig) -> Result<PreviewResult, String> {
        let scan = self
            .document
            .scans
            .get(self.scan)
            .ok_or("Select matching one-dimensional datasets.")?;
        let columns = scan.columns.len();
        let points = scan.columns.first().map_or(0, |c| c.values.len());
        let roles = |c: &ImportConfig| ResolvedImport {
            mode: c.mode,
            energy_col: c.energy_col.unwrap_or(0),
            i0_col: c.i0_col.unwrap_or(1),
            it_col: c.it_col.unwrap_or(2),
            ir_col: c.ir_col.unwrap_or(3),
            fluor_cols: c.fluor_cols.clone().unwrap_or_default(),
            mu_col: c.mu_col,
        };
        let diagnostics = ParserDiagnostics {
            valid_points: points,
            ..Default::default()
        };
        let raw = self
            .mapping(config)
            .and_then(|m| scan.to_spectrum(Some(&m)).map_err(|e| e.to_string()))
            .map(|s| RawData {
                channel: config.mode,
                declared_edge: None,
                energy: s.energy.unwrap().as_slice().to_vec(),
                mu: s.mu.unwrap().as_slice().to_vec(),
                diagnostics: diagnostics.clone(),
            });
        let table = ImportPreview {
            layout: Some(MeasurementLayout {
                format: self.document.format.clone(),
                units: scan.columns.iter().map(|c| c.units.clone()).collect(),
            }),
            column_count: columns,
            names: Some(scan.columns.iter().map(|c| c.name.clone()).collect()),
            rows: (0..points.min(3))
                .map(|i| scan.columns.iter().map(|c| c.values[i]).collect())
                .collect(),
            detected: roles(&self.config()),
            resolved: roles(config),
            auto_mode: self.config().mode,
            xdi: None,
            diagnostics,
            signal_error: raw.as_ref().err().cloned(),
        };
        Ok(PreviewResult { table, raw })
    }
    pub fn materialize(&self, config: &ImportConfig) -> Result<DerivedSpectrum, String> {
        if !self.confirmed {
            return Err("Choose a signal before importing.".into());
        }
        materialize(
            &self.path,
            &self.original_bytes,
            &self.document,
            self.scan,
            &self.mapping(config)?,
        )
        .map_err(|e| e.to_string())
    }
    pub fn source_details(&self) -> String {
        let mut details = format!("{}\nFormat: {}", self.path.display(), self.document.format);
        if let Some(scan) = self.document.scans.get(self.scan) {
            details.push_str(&format!("\n\n{}", scan.header));
            for warning in &scan.warnings {
                details.push_str(&format!("\n{warning}"));
            }
        }
        for warning in &self.document.warnings {
            details.push_str(&format!("\n{warning}"));
        }
        details
    }
    pub fn select_datasets(&mut self) -> Result<(), String> {
        let scan = self
            .document
            .dataset_scan(&self.dataset_paths)
            .map_err(|e| e.to_string())?;
        let document = Arc::make_mut(&mut self.document);
        document.scans.push(scan);
        self.selected_scans.push(true);
        self.choices.push(None);
        self.select_scan(self.document.scans.len() - 1);
        Ok(())
    }
}
impl StudioApp {
    pub(crate) fn open_measurement(&mut self, cx: &mut Context<Self>) {
        self.measurement_request += 1;
        let request = self.measurement_request;
        let generation = self.project_generation;
        let pick = cx.prompt_for_paths(gpui::PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Read measurement".into()),
        });
        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(paths))) = pick.await
                && let Some(path) = paths.into_iter().next()
            {
                this.update(cx, |app, cx| {
                    if app.measurement_request == request && app.project_generation == generation {
                        app.open_measurement_path(path, true, cx);
                    }
                })
                .ok();
            }
        })
        .detach();
    }

    pub(crate) fn open_measurement_path(
        &mut self,
        path: PathBuf,
        remember: bool,
        cx: &mut Context<Self>,
    ) {
        self.open_measurement_batch(path, remember, None, cx);
    }

    pub(crate) fn open_measurement_batch(
        &mut self,
        path: PathBuf,
        remember: bool,
        batch: Option<usize>,
        cx: &mut Context<Self>,
    ) {
        let peers = batch
            .and_then(|id| self.intake.history.get(id))
            .map(|batch| {
                batch
                    .sources
                    .iter()
                    .filter(|(_, outcome)| {
                        outcome
                            .pending
                            .as_ref()
                            .is_some_and(|pending| pending.measurement_reader)
                    })
                    .map(|(path, _)| path.clone())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        self.measurement_request += 1;
        let request = self.measurement_request;
        let generation = self.project_generation;
        self.status = "Reading measurement…".into();
        self.measurement_import = None;
        cx.spawn(async move |this, cx| {
            let source = path.clone();
            let result = cx
                .background_spawn(async move {
                    let (document, bytes) = read_source(&source)?;
                    let mut source = MeasurementImport::new(source, document, bytes);
                    if let Some(id) = batch {
                        source.review_batch(id, peers);
                    }
                    Ok::<_, String>(source)
                })
                .await;
            this.update(cx, |app, cx| {
                if app.measurement_request != request || app.project_generation != generation {
                    return;
                }
                match result {
                    Ok(source) => {
                        if remember
                            && let Ok(absolute) = path.canonicalize()
                            && let Some(parent) = absolute.parent()
                        {
                            crate::settings::push_recent(
                                &mut app.structure.settings.recent_import_folders,
                                parent.to_path_buf(),
                                8,
                            );
                            app.persist_recent_locations();
                        }
                        app.measurement_import = Some(source);
                        app.status = "Review import".into();
                    }
                    Err(error) => {
                        app.record_job_error("Measurement import", error.clone());
                        app.status = error.into();
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    pub(crate) fn accept_measurements(
        &mut self,
        groups: Vec<DerivedSpectrum>,
        batch: Option<usize>,
        cx: &mut Context<Self>,
    ) {
        let count = groups.len();
        if count == 0 {
            return;
        }
        let first = self.derived.len();
        let mut ids = std::collections::BTreeSet::new();
        let mut imported_sources =
            std::collections::BTreeMap::<PathBuf, Vec<crate::group_identity::GroupId>>::new();
        for mut group in groups {
            group.id = self.next_group_id();
            if let Some(path) = group
                .operation
                .as_ref()
                .and_then(|operation| operation.parameters.get("source_path"))
                .and_then(|value| serde_json::from_value::<PathBuf>(value.clone()).ok())
                && let Some(id) = &group.group_id
            {
                imported_sources.entry(path).or_default().push(id.clone());
            }
            ids.insert(
                group
                    .group_id
                    .clone()
                    .expect("Materialized measurement has an identity"),
            );
            self.derived.push(group);
        }
        if let Some(batch) = batch.and_then(|id| self.intake.history.get_mut(id)) {
            for (path, created) in &imported_sources {
                if let Some(source) = batch.sources.get_mut(path)
                    && source
                        .pending
                        .as_ref()
                        .is_some_and(|pending| pending.measurement_reader)
                {
                    source.pending = None;
                    source.failed = None;
                    source.created.extend(created.iter().cloned());
                }
            }
        }
        self.rekey_after_catalog_change();
        self.record_created_groups(ids, format!("Import {count} measurement spectra"));
        // Show the same raw quantity that the user reviewed before importing.
        self.stage_view.e_quantity = EQuantity::Mu;
        self.stage_view.scope = PlotScope::Current;
        self.set_stage(Stage::Data, cx);
        self.select_entry(DERIVED_BASE + first, cx);
        self.status = format!(
            "Imported {count} {}",
            if count == 1 { "spectrum" } else { "spectra" }
        )
        .into();
        cx.notify();
    }
}

fn read_source(path: &std::path::Path) -> Result<(Measurement, Vec<u8>), String> {
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut bytes = Vec::new();
    file.take(256 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 256 * 1024 * 1024 {
        return Err("Desktop measurement review accepts files up to 256 MiB.".into());
    }
    let document = rexafs::io::parse_measurement(&bytes).map_err(|e| e.to_string())?;
    Ok((document, bytes))
}

/// Materialize one selected record with portable source evidence; saved stages
/// remain provenance and never become active rexafs processing caches.
fn materialize(
    path: &std::path::Path,
    bytes: &[u8],
    document: &Measurement,
    index: usize,
    mapping: &SpectrumMapping,
) -> Result<DerivedSpectrum, rexafs::io::ReadError> {
    let scan = document
        .scans
        .get(index)
        .ok_or_else(|| rexafs::io::ReadError {
            message: "Select an available scan".into(),
        })?;
    let spectrum = scan.to_spectrum(Some(mapping))?;
    let provenance = serde_json::json!({
        "format":document.format,"source_path":path,"record_id":scan.id,"mapping":mapping,
        "original_bytes_base64":base64::engine::general_purpose::STANDARD.encode(bytes),
        "source_record":scan,
        "container_metadata":document.metadata,
        "archived_tables":document.datasets.iter().filter(|d|d.path.starts_with(&format!("/{}/",scan.id))).collect::<Vec<_>>(),
        "ordering":"Energy and signal sorted together; duplicates retained."
    });
    Ok(DerivedSpectrum {
        label: format!(
            "{} · {}",
            path.file_name().unwrap_or_default().to_string_lossy(),
            scan.label
        ),
        energy: spectrum.energy.unwrap().as_slice().to_vec(),
        mu: spectrum.mu.unwrap().as_slice().to_vec(),
        group_id: Some(crate::group_identity::GroupId::new_result()),
        operation: Some(Operation {
            tool: "Measurement import".into(),
            parameters: provenance,
            inputs: vec![],
            applied_energy_shift_ev: 0.,
        }),
        ..Default::default()
    })
}

#[cfg(test)]
mod tests;
