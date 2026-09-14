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

/// One immutable source snapshot; editing a mapping never rereads or changes it.
#[derive(Clone)]
pub(crate) struct MeasurementImport {
    pub path: PathBuf,
    original_bytes: Arc<Vec<u8>>,
    pub document: Arc<Measurement>,
    pub scan: usize,
    pub signal: Option<usize>,
    pub confirmed: bool,
    pub dataset_paths: Vec<String>,
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
        let confirmed = document.scans.first().is_some_and(|s| s.signals.len() == 1);
        Self {
            path,
            original_bytes: Arc::new(bytes),
            document: Arc::new(document),
            scan: 0,
            signal: confirmed.then_some(0),
            confirmed,
            dataset_paths: vec![],
        }
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
        let mapping = self.selected_mapping();
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
        self.scan = index;
        self.confirmed = self
            .document
            .scans
            .get(index)
            .is_some_and(|s| s.signals.len() == 1);
        self.signal = self.confirmed.then_some(0);
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
        self.measurement_request += 1;
        let request = self.measurement_request;
        let generation = self.project_generation;
        self.status = "Reading measurement…".into();
        self.measurement_import = None;
        cx.spawn(async move |this, cx| {
            let source = path.clone();
            let result = cx
                .background_spawn(async move { read_source(&source) })
                .await;
            this.update(cx, |app, cx| {
                if app.measurement_request != request || app.project_generation != generation {
                    return;
                }
                match result {
                    Ok((document, original_bytes)) => {
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
                        app.measurement_import =
                            Some(MeasurementImport::new(path, document, original_bytes));
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

    pub(crate) fn accept_measurement(
        &mut self,
        mut group: DerivedSpectrum,
        cx: &mut Context<Self>,
    ) {
        group.id = self.next_group_id();
        self.record(
            "Import measurement",
            Some(journal::UndoOp::DerivedAdd {
                index: self.derived.len(),
                spectrum: group.clone(),
            }),
        );
        self.derived.push(group);
        self.rekey_after_catalog_change();
        self.select_entry(DERIVED_BASE + self.derived.len() - 1, cx);
        self.status = "Spectrum imported".into();
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
