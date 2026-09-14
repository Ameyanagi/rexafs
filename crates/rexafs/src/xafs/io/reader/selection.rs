//! Resolve user-selected names without changing the original indexed mapping API.
use super::{EnergyConversion, MeasurementScan, ReadError, SignalConversion, SpectrumMapping};
use crate::xafs::xasspectrum::XASSpectrum;
use serde::{Deserialize, Serialize};

/// A zero-based index or exact source column name (unreleased).
/// Names are case-sensitive and must identify exactly one column in the selected
/// scan. Use an index for duplicate labels; numeric strings remain names.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ColumnSelector {
    /// Zero-based column index.
    Index(usize),
    /// Exact, case-sensitive column label, without trimming or alias expansion.
    Name(String),
}
impl From<usize> for ColumnSelector {
    fn from(value: usize) -> Self {
        Self::Index(value)
    }
}
impl From<&str> for ColumnSelector {
    fn from(value: &str) -> Self {
        Self::Name(value.into())
    }
}
impl From<String> for ColumnSelector {
    fn from(value: String) -> Self {
        Self::Name(value)
    }
}

/// Detector arithmetic using names, indices, or both (unreleased).
/// Resolution delegates to [`SignalConversion`]; no extra detector corrections,
/// normalization, or changes to the stored columns occur.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SignalSelection {
    /// Copy the stored signal without changing its scale.
    Direct {
        /// Stored absorption or yield column.
        column: ColumnSelector,
    },
    /// Calculate dimensionless optical thickness ln(incident / transmitted).
    /// Uses the same finite, nonzero, same-sign requirements as [`SignalConversion`].
    Transmission {
        /// Incident intensity, normally I0.
        incident: ColumnSelector,
        /// Transmitted intensity, normally It.
        transmitted: ColumnSelector,
    },
    /// Divide the sum of explicitly selected detector channels by a monitor.
    Ratio {
        /// Nonempty list of distinct detector columns; no automatic detector sum.
        detectors: Vec<ColumnSelector>,
        /// Incident monitor column, which must be nonzero during conversion.
        incident: ColumnSelector,
    },
}

/// Explicit column selection by name or index (unreleased).
///
/// Constructors use the selected axis's detected calibration or declared eV/keV
/// units. Missing or conflicting calibration requires [`Self::with_energy`].
/// Resolve against each scan separately; indices may differ between files.
/// Existing [`SpectrumMapping`] struct literals and numeric APIs remain valid.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpectrumSelection {
    /// Axis column, selected by exact name or zero-based index.
    pub energy_column: ColumnSelector,
    /// Explicit conversion to eV. None uses the selected scan's axis calibration.
    #[serde(default)]
    pub energy: Option<EnergyConversion>,
    /// Stored signal or detector arithmetic with explicit column roles.
    pub signal: SignalSelection,
}
impl SpectrumSelection {
    /// Select stored absorption with names or indices; infer axis units from metadata.
    pub fn direct(energy: impl Into<ColumnSelector>, mu: impl Into<ColumnSelector>) -> Self {
        Self {
            energy_column: energy.into(),
            energy: None,
            signal: SignalSelection::Direct { column: mu.into() },
        }
    }
    /// Select transmission ln(I0 / It); infer the axis calibration from metadata.
    pub fn transmission(
        energy: impl Into<ColumnSelector>,
        i0: impl Into<ColumnSelector>,
        it: impl Into<ColumnSelector>,
    ) -> Self {
        Self {
            energy_column: energy.into(),
            energy: None,
            signal: SignalSelection::Transmission {
                incident: i0.into(),
                transmitted: it.into(),
            },
        }
    }
    /// Select a fluorescence/yield detector sum divided by I0.
    /// Each detector is explicit; resolution/conversion rejects missing or repeated roles.
    pub fn fluorescence(
        energy: impl Into<ColumnSelector>,
        i0: impl Into<ColumnSelector>,
        detectors: impl IntoIterator<Item = impl Into<ColumnSelector>>,
    ) -> Self {
        Self {
            energy_column: energy.into(),
            energy: None,
            signal: SignalSelection::Ratio {
                incident: i0.into(),
                detectors: detectors.into_iter().map(Into::into).collect(),
            },
        }
    }
    /// Override metadata with an explicit eV/keV, relative-energy or Bragg conversion.
    /// This changes the selection only; input data and saved calibrations are unchanged.
    pub fn with_energy(mut self, energy: EnergyConversion) -> Self {
        self.energy = Some(energy);
        self
    }
    /// Resolve names and calibration to an indexed mapping for inspection or conversion.
    /// Missing/duplicate names, invalid indices, and unknown/conflicting calibration
    /// return [`ReadError`]. Numerical and distinct-role checks run during `arrays()`.
    pub fn resolve(&self, scan: &MeasurementScan) -> Result<SpectrumMapping, ReadError> {
        let energy_column = scan.column_index(&self.energy_column)?;
        let energy = match &self.energy {
            Some(energy) => energy.clone(),
            None => scan.selected_energy(energy_column)?,
        };
        let signal = match &self.signal {
            SignalSelection::Direct { column } => SignalConversion::Direct {
                column: scan.column_index(column)?,
            },
            SignalSelection::Transmission {
                incident,
                transmitted,
            } => SignalConversion::Transmission {
                incident: scan.column_index(incident)?,
                transmitted: scan.column_index(transmitted)?,
            },
            SignalSelection::Ratio {
                detectors,
                incident,
            } => SignalConversion::Ratio {
                detectors: detectors
                    .iter()
                    .map(|c| scan.column_index(c))
                    .collect::<Result<_, _>>()?,
                incident: scan.column_index(incident)?,
            },
        };
        Ok(SpectrumMapping {
            energy_column,
            energy,
            signal,
        })
    }
}

impl MeasurementScan {
    /// Find an exact column name or validate a zero-based index without changing data.
    /// Duplicate names require an index; names are never case-folded or interpreted
    /// as numbers. Errors identify the scan and the missing or ambiguous selector.
    pub fn column_index(&self, selector: &ColumnSelector) -> Result<usize, ReadError> {
        match selector {
            ColumnSelector::Index(index) if *index < self.columns.len() => Ok(*index),
            ColumnSelector::Index(index) => Err(ReadError::new(format!(
                "Scan '{}': column {index} is out of range",
                self.id
            ))),
            ColumnSelector::Name(name) => {
                let mut matches = self
                    .columns
                    .iter()
                    .enumerate()
                    .filter(|(_, c)| c.name == *name);
                let Some((index, _)) = matches.next() else {
                    return Err(ReadError::new(format!(
                        "Scan '{}': no column named {name:?}; inspect scan.columns",
                        self.id
                    )));
                };
                if matches.next().is_some() {
                    return Err(ReadError::new(format!(
                        "Scan '{}': column name {name:?} is ambiguous; use a zero-based index",
                        self.id
                    )));
                }
                Ok(index)
            }
        }
    }
    fn selected_energy(&self, column: usize) -> Result<EnergyConversion, ReadError> {
        let mut detected = self
            .signals
            .iter()
            .filter(|s| s.mapping.energy_column == column)
            .map(|s| &s.mapping.energy);
        if let Some(energy) = detected.next() {
            if detected.any(|other| other != energy) {
                return Err(ReadError::new(
                    "Conflicting axis calibrations; specify the energy conversion explicitly",
                ));
            }
            return Ok(energy.clone());
        }
        match self.columns[column].units.as_deref().map(str::to_ascii_lowercase).as_deref() {
            Some("ev") => Ok(EnergyConversion::Ev),
            Some("kev") => Ok(EnergyConversion::Kev),
            _ => Err(ReadError::new(format!("Scan '{}': column {column} has no supported energy calibration; specify energy_unit or an explicit energy conversion", self.id))),
        }
    }
    /// Convert a name/index selection to owned energy (eV) and signal arrays.
    /// Uses [`Self::arrays`] after resolution, retaining order and duplicates;
    /// no processing, correction or input mutation occurs.
    pub fn arrays_with(
        &self,
        selection: &SpectrumSelection,
    ) -> Result<(Vec<f64>, Vec<f64>), ReadError> {
        self.arrays(Some(&selection.resolve(self)?))
    }
    /// Create an unprocessed spectrum using a name/index selection.
    /// Uses [`Self::to_spectrum`]: energy and signal are sorted together, duplicate
    /// energies remain, and no normalization/background/FFT stages run.
    pub fn to_spectrum_with(
        &self,
        selection: &SpectrumSelection,
    ) -> Result<XASSpectrum, ReadError> {
        self.to_spectrum(Some(&selection.resolve(self)?))
    }
}
