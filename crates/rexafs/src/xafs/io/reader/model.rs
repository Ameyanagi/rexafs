//! Owned measurement data and explicit, validated spectrum conversions.
use super::signals;
use crate::xafs::xasspectrum::XASSpectrum;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A read or conversion failure with a source location when available.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{message}")]
pub struct ReadError {
    /// Human-readable format, scan, line, column or dataset context.
    pub message: String,
}
impl ReadError {
    pub(super) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

/// An original numeric channel. Values are owned and retain source units.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MeasurementColumn {
    /// Source label, or `column_N` when the file has no labels (N is one-based).
    pub name: String,
    /// Source unit; `None` means that the source did not declare one.
    pub units: Option<String>,
    /// Numeric samples in acquisition order; nonfinite values remain inspectable.
    pub values: Vec<f64>,
}

/// Conversion of the selected axis to electronvolts (eV).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum EnergyConversion {
    /// Energy already in eV.
    Ev,
    /// Energy in kiloelectronvolts; multiply by 1000.
    Kev,
    /// Relative energy in eV: E_absolute = E_source + offset_ev.
    /// FDMNES outputs use their declared E_edge as this offset, following
    /// Larch's read_fdmnes convention. Source arrays remain unchanged.
    OffsetEv {
        /// Finite energy origin in eV; zero leaves an eV axis unchanged.
        offset_ev: f64,
    },
    /// First-order Bragg diffraction, E = hc / (2 d sin(theta)).
    /// The constant hc is 12398.419843320026 eV Å; no offset is inferred.
    Bragg {
        /// Positive lattice-plane spacing d in angstroms (Å).
        d_spacing: f64,
        /// Multiply the source axis by this value to obtain degrees.
        /// Use 1 for degrees, 180/pi for radians, or 1/steps_per_degree.
        degrees_per_unit: f64,
    },
}

/// Detector arithmetic for one unprocessed spectrum.
///
/// Transmission follows the intensity-ratio convention in
/// [Newville (2014)](https://doi.org/10.2138/rmg.2014.78.2). All indices are
/// zero-based. No dark-current, gain, dead-time or self-absorption corrections
/// are inferred. Apply needed corrections before selecting raw channels.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SignalConversion {
    /// Copy an already calculated absorption or yield channel.
    Direct {
        /// Column containing the signal; its original scale is retained.
        column: usize,
    },
    /// Dimensionless optical thickness ln(incident/transmitted).
    /// The channels must have matching units and a positive ratio. Matching
    /// negative electronics polarity is accepted; opposite signs/zero fail.
    Transmission {
        /// Incident intensity column (I0, or It for a reference foil).
        incident: usize,
        /// Transmitted intensity column (It, or Ir for a reference foil).
        transmitted: usize,
    },
    /// Fluorescence or electron yield: sum(selected detectors) / incident.
    /// This is proportional to absorption under the experiment's assumptions;
    /// its scale depends on detector efficiency, geometry and channel units.
    Ratio {
        /// Explicit, nonempty, unique detector column indices.
        detectors: Vec<usize>,
        /// Incident monitor column; must be nonzero.
        incident: usize,
    },
}

/// Explicit axis and signal selection; never modifies the imported scan.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpectrumMapping {
    /// Zero-based axis column.
    pub energy_column: usize,
    /// Source axis units or crystal calibration; no magnitude heuristic is used.
    pub energy: EnergyConversion,
    /// Selected signal arithmetic.
    pub signal: SignalConversion,
}

/// An identified signal choice. Multiple choices require caller selection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SignalCandidate {
    /// Human-readable signal label, also used by interface selectors.
    pub name: String,
    /// Fully specified conversion supported by the header/layout.
    pub mapping: SpectrumMapping,
}

/// One acquisition scan or project group, with its original numeric channels.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MeasurementScan {
    /// Stable source scan identifier, project group key, or HDF5 group path.
    pub id: String,
    /// Display label; no scientific meaning is inferred from filenames.
    pub label: String,
    /// Columns in source order; all have the same number of samples.
    pub columns: Vec<MeasurementColumn>,
    /// Original text header, retained for attribution and manual interpretation.
    pub header: String,
    /// Extracted metadata. Original spelling remains in `header`.
    pub metadata: BTreeMap<String, String>,
    /// Header-supported signal mappings. Empty means manual mapping is needed.
    /// Shared header inference requires unique detector-role matches. Each stored
    /// absorption column remains a separate choice, even when labels repeat
    /// (since 0.2.7).
    pub signals: Vec<SignalCandidate>,
    /// Unit assumptions, ambiguous channels and historical-format observations.
    pub warnings: Vec<String>,
}

impl MeasurementScan {
    /// Convert to owned energy (eV) and signal arrays in original acquisition order.
    ///
    /// Recommended `None` uses the sole detected signal. Zero or multiple
    /// candidates require `Some(mapping)`. Rejects invalid indices, mismatched
    /// lengths, nonfinite selected values, nonpositive energy, invalid Bragg
    /// geometry, zero monitors and nonpositive transmission ratios. Finite
    /// negative direct/yield values are retained. Duplicate energies are retained.
    /// No input arrays or settings are modified.
    pub fn arrays(
        &self,
        mapping: Option<&SpectrumMapping>,
    ) -> Result<(Vec<f64>, Vec<f64>), ReadError> {
        let mapping = match mapping {
            Some(m) => m,
            None if self.signals.len() == 1 => &self.signals[0].mapping,
            None => {
                return Err(ReadError::new(format!(
                    "Scan '{}': select an explicit energy/signal mapping ({} detected choices)",
                    self.id,
                    self.signals.len()
                )))
            }
        };
        let col = |i: usize| {
            self.columns.get(i).ok_or_else(|| {
                ReadError::new(format!("Scan '{}': column {i} is out of range", self.id))
            })
        };
        let axis = col(mapping.energy_column)?;
        if axis.values.is_empty() {
            return Err(ReadError::new("Selected scan is empty"));
        }
        let mut selected = vec![mapping.energy_column];
        match &mapping.signal {
            SignalConversion::Direct { column } => selected.push(*column),
            SignalConversion::Transmission {
                incident,
                transmitted,
            } => selected.extend([*incident, *transmitted]),
            SignalConversion::Ratio {
                detectors,
                incident,
            } => {
                if detectors.is_empty() {
                    return Err(ReadError::new("Select at least one detector"));
                }
                selected.extend(detectors);
                selected.push(*incident);
            }
        }
        let mut unique = selected.clone();
        unique.sort_unstable();
        unique.dedup();
        if unique.len() != selected.len() {
            return Err(ReadError::new(
                "Axis, detector and monitor roles must use distinct columns",
            ));
        }
        for i in selected {
            if col(i)?.values.len() != axis.values.len() {
                return Err(ReadError::new("Selected column lengths differ"));
            }
        }
        let mut energy = Vec::with_capacity(axis.values.len());
        let mut mu = Vec::with_capacity(axis.values.len());
        for (row, &x) in axis.values.iter().enumerate() {
            let fail = |message: &str| {
                ReadError::new(format!("Scan '{}', row {}: {message}", self.id, row + 1))
            };
            let e = match mapping.energy {
                EnergyConversion::Ev => x,
                EnergyConversion::Kev => x * 1000.,
                EnergyConversion::OffsetEv { offset_ev } => x + offset_ev,
                EnergyConversion::Bragg {
                    d_spacing,
                    degrees_per_unit,
                } => {
                    let degrees = x * degrees_per_unit;
                    if !d_spacing.is_finite()
                        || d_spacing <= 0.
                        || !degrees_per_unit.is_finite()
                        || degrees_per_unit <= 0.
                        || !(0. < degrees && degrees <= 90.)
                    {
                        return Err(fail("invalid crystal spacing or Bragg angle"));
                    }
                    12398.419843320026 / (2. * d_spacing * degrees.to_radians().sin())
                }
            };
            if !e.is_finite() || e <= 0. {
                return Err(fail("energy must be positive and finite"));
            }
            let v = |i: usize| -> Result<f64, ReadError> {
                let value = self.columns[i].values[row];
                if !value.is_finite() {
                    return Err(fail(&format!("column {i} is not finite")));
                }
                Ok(value)
            };
            let y = match &mapping.signal {
                SignalConversion::Direct { column } => v(*column)?,
                SignalConversion::Transmission {
                    incident,
                    transmitted,
                } => {
                    let a = v(*incident)?;
                    let b = v(*transmitted)?;
                    if a == 0. || b == 0. || a.is_sign_positive() != b.is_sign_positive() {
                        return Err(fail(
                            "transmission requires nonzero intensities with the same sign",
                        ));
                    }
                    // Difference of logarithms avoids overflow/underflow in a/b.
                    a.abs().ln() - b.abs().ln()
                }
                SignalConversion::Ratio {
                    detectors,
                    incident,
                } => {
                    let denominator = v(*incident)?;
                    if denominator == 0. {
                        return Err(fail("incident monitor is zero"));
                    }
                    let sum = detectors.iter().try_fold(0., |a, &i| Ok(a + v(i)?))?;
                    sum / denominator
                }
            };
            if !y.is_finite() {
                return Err(fail("calculated signal is not finite"));
            }
            energy.push(e);
            mu.push(y);
        }
        Ok((energy, mu))
    }

    /// Create an owned, unprocessed spectrum using [`Self::arrays`].
    /// Energy and signal are sorted together by the spectrum setter. Repeated
    /// energies remain; downstream numerical stages may require explicit cleanup.
    /// No processing prerequisites run and no source data or caches are changed.
    pub fn to_spectrum(&self, mapping: Option<&SpectrumMapping>) -> Result<XASSpectrum, ReadError> {
        let (energy, mu) = self.arrays(mapping)?;
        let mut spectrum = XASSpectrum::new();
        spectrum.set_spectrum(energy, mu);
        Ok(spectrum)
    }
}

/// A numeric dataset or archived result array, in row-major order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MeasurementDataset {
    /// Absolute HDF5 path, XTUNES table path, or Larix `/symbol/attribute` path.
    pub path: String,
    /// Original dimensions in row-major order; empty means a scalar.
    pub shape: Vec<u64>,
    /// Decoded numeric values, without detector projection or region selection.
    pub values: Vec<f64>,
    /// Imaginary components for a complex Larix array, with the same shape and
    /// length as `values` (its real components). Absent for real arrays.
    /// These are archived values, not an active rexafs Fourier cache.
    #[serde(default)]
    pub imaginary: Option<Vec<f64>>,
    /// Source attributes as text. Larix includes `larix.dtype` and
    /// `larix.bytes_base64`: exact dtype bytes (original byte order, C order),
    /// even when f64 views round integers. Legacy numeric lists are encoded to
    /// their declared dtype. XTUNES includes headings and quantity descriptions.
    /// These are archived results, not rexafs processing caches.
    pub attributes: BTreeMap<String, String>,
}

/// Owned import result; one source may have many scans or only detector arrays.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Measurement {
    /// Content-detected family; never inferred solely from the filename.
    pub format: String,
    /// All recovered scans, including those requiring explicit mapping.
    pub scans: Vec<MeasurementScan>,
    /// Numeric datasets and archived tables, including non-absorption quantities.
    pub datasets: Vec<MeasurementDataset>,
    /// Container provenance as text. Larix retains `larix.session_text`,
    /// `larix.command_history` and ordered `larix.symbol_order` here, including
    /// empty sessions. Commands and saved Python objects are never executed.
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
    /// Container and encoding observations shared by the scans.
    pub warnings: Vec<String>,
}

impl Measurement {
    /// Assemble an explicit real-valued dataset selection into a new owned scan.
    ///
    /// Paths must name at least two distinct one-dimensional datasets with equal
    /// nonzero lengths. Order determines column indices. This supports generic
    /// HDF5 files whose axis and detectors live in different groups. Arrays and
    /// metadata are copied; this document is unchanged. Multidimensional images
    /// require an explicit reduction outside this reader. Missing paths or
    /// incompatible shapes return ReadError. No signal arithmetic is performed.
    pub fn dataset_scan(&self, paths: &[String]) -> Result<MeasurementScan, ReadError> {
        if paths.len() < 2 {
            return Err(ReadError::new("Select at least two dataset paths"));
        }
        let mut unique = std::collections::BTreeSet::new();
        let mut columns = Vec::new();
        let mut length = None;
        for path in paths {
            if !unique.insert(path) {
                return Err(ReadError::new("Dataset paths must be distinct"));
            }
            let ds = self
                .datasets
                .iter()
                .find(|d| &d.path == path)
                .ok_or_else(|| ReadError::new(format!("Unknown dataset: {path}")))?;
            if ds.imaginary.is_some() {
                return Err(ReadError::new(format!(
                    "Dataset {path} is complex; choose a real-valued signal explicitly"
                )));
            }
            if ds.shape.len() != 1
                || ds.shape[0] != ds.values.len() as u64
                || ds.values.is_empty()
                || length.is_some_and(|n| n != ds.values.len())
            {
                return Err(ReadError::new(format!(
                    "Dataset {path} must be a nonempty vector matching the other selected lengths"
                )));
            }
            length = Some(ds.values.len());
            columns.push(MeasurementColumn {
                name: path.rsplit('/').next().unwrap_or(path).into(),
                units: ds.attributes.get("units").cloned(),
                values: ds.values.clone(),
            });
        }
        let mut scan = MeasurementScan {
            id: "selected_datasets".into(),
            label: if self.format == "hdf5" {
                "Selected HDF5 datasets"
            } else {
                "Selected datasets"
            }
            .into(),
            columns,
            header: String::new(),
            metadata: BTreeMap::from([("dataset_paths".into(), paths.join("\n"))]),
            signals: Vec::new(),
            warnings: Vec::new(),
        };
        signals::infer(&mut scan);
        Ok(scan)
    }
}
