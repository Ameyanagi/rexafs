//! Offline atomic references for absorption normalization and correction (unreleased).
//!
//! [`AtomicData::new`] opens the pinned xraydb 0.4.1 data without network access.
//! Every output carries a checksum of the actual loaded dataset and the named
//! interpolation profile. Energies are eV; mass attenuation is cm²/g. Requests
//! outside table support fail, instead of accepting the dependency's clamping.
//! Values around an edge follow that table's grid/interpolation; no experimental
//! broadening or chemical shift is applied. See [`AtomicTable`] for sources.
mod formula;
#[cfg(test)]
mod tests;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, io::Write, sync::OnceLock};
use xraydb::{CrossSectionKind, XrayDb};

/// Invalid scientific requests or unavailable atomic resources.
#[derive(Debug, thiserror::Error)]
pub enum AtomicDataError {
    /// Unknown identity, unsupported energy/formula, or invalid numerical result.
    #[error("Atomic reference: {0}")]
    Invalid(String),
    /// Loading or fingerprinting the offline resource failed.
    #[error("Atomic data unavailable: {0}")]
    Unavailable(String),
    /// Exact recomputation requires the recorded data and interpolation version.
    #[error("The recorded atomic reference is not available in this build")]
    VersionMismatch,
}
type Result<T> = std::result::Result<T, AtomicDataError>;
fn invalid(e: impl std::fmt::Display) -> AtomicDataError {
    AtomicDataError::Invalid(e.to_string())
}

/// Dataset identity retained in scientific results; not just the crate version.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AtomicDataIdentity {
    /// Rust provider/profile version, currently xraydb-rs/0.4.1/rexafs-atomic-v1.
    pub provider: String,
    /// Upstream database version reported by the loaded resource.
    pub data_version: String,
    /// SHA-256 of the actual xraydb-data 0.4.1 record in compact serde JSON order.
    /// This also identifies custom data loaded through xraydb before this provider.
    pub data_sha256: String,
}

/// Named table/interpolation contract, distinct from the containing dataset.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AtomicTable {
    /// Chantler f₂ with linear interpolation in log energy/log f₂, including
    /// the tabulated edge grid; no shifted edge or extrapolation.
    /// [Chantler (2000)](https://doi.org/10.1063/1.1321055).
    ChantlerF2LogLogV1,
    /// Total mass attenuation: photoelectric + coherent + incoherent scattering.
    /// Uses the Elam log-energy/log-coefficient splines and their edge convention.
    /// [Elam, Ravel and Sieber (2002)](https://doi.org/10.1016/S0969-806X(01)00227-4).
    ElamTotalV1,
    /// Tabulated Elam edge and emission-line records, without interpolation.
    ElamTransitionsV1,
}

/// Exact provider and table used by one operation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AtomicReference {
    /// Actual loaded dataset identity.
    pub data: AtomicDataIdentity,
    /// Table and interpolation/contribution profile.
    pub table: AtomicTable,
}

/// Values on exactly the requested energy points; no resampling or extrapolation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AtomicCurve {
    /// Original query energies in eV, in caller order.
    pub energy_ev: Vec<f64>,
    /// f₂ (dimensionless electron units) or mass attenuation (cm²/g), per `unit`.
    pub values: Vec<f64>,
    /// Explicit output unit, retained with the curve.
    pub unit: String,
    /// Dataset and algorithm identity for recomputation.
    pub reference: AtomicReference,
}

/// One absorption edge; identity is explicit, never inferred from sample formula.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AtomicEdge {
    /// Canonical element symbol.
    pub element: String,
    /// IUPAC shell label, such as K or L3.
    pub edge: String,
    /// Tabulated energy in eV, not a measured or chemically shifted E₀.
    pub energy_ev: f64,
    /// Source of this tabulated transition.
    pub reference: AtomicReference,
}

/// Individual emission line and its within-shell relative intensity.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AtomicLine {
    /// Siegbahn line label, such as Ka1.
    pub name: String,
    /// Tabulated emission energy in eV.
    pub energy_ev: f64,
    /// Relative intensity within the initial shell; not across different shells.
    pub intensity: f64,
    /// Initial core-hole shell, such as K.
    pub initial_level: String,
    /// Final shell label, such as L3.
    pub final_level: String,
}

/// A single line and a family are different scientific choices.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EmissionSelection {
    /// One exact Siegbahn label, for example Ka1.
    Line(String),
    /// A family prefix, for example Ka; energy is its intensity-weighted mean.
    /// A family spanning different initial shells is rejected: their intensities
    /// are not on one common scale.
    Family(String),
}

/// Resolved emission selection with all contributing lines retained.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AtomicEmission {
    /// Canonical emitting element.
    pub element: String,
    /// Exact requested line or family.
    pub selection: EmissionSelection,
    /// Individual energy, or within-shell intensity-weighted family mean, in eV.
    pub energy_ev: f64,
    /// Contributing original records, sorted by name.
    pub lines: Vec<AtomicLine>,
    /// Transition-table identity.
    pub reference: AtomicReference,
}

/// Stoichiometry and explicit mass fraction of one element.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ElementFraction {
    /// Canonical element symbol.
    pub element: String,
    /// Number of atoms per supplied formula unit; may be fractional.
    pub atoms: f64,
    /// Molar mass in g/mol used to compute the fraction.
    pub molar_mass: f64,
    /// atoms*molar_mass divided by total formula mass; dimensionless.
    pub mass_fraction: f64,
}

/// Compound attenuation obtained by the independent-atom mass-fraction sum.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CompoundAttenuation {
    /// Original stoichiometric expression; no density or material-name lookup.
    pub formula: String,
    /// Elemental composition and arithmetic, sorted by symbol.
    pub composition: Vec<ElementFraction>,
    /// Total mass attenuation in cm²/g on the requested energy grid.
    pub curve: AtomicCurve,
}

/// Pure Rust provider boundary for qualified alternatives or numerical references.
/// Implementations must reject unsupported energies and identify every output.
/// Normal callers use [`AtomicData`] directly; processing calls obtain it automatically.
pub trait AtomicDataProvider: Send + Sync {
    /// Exact actual dataset identity, including content checksum.
    fn identity(&self) -> &AtomicDataIdentity;
    /// Edge energy and provenance for an explicit absorber/shell.
    fn edge(&self, element: &str, edge: &str) -> Result<AtomicEdge>;
    /// All supported edges for an element, ordered by IUPAC label.
    fn edges(&self, element: &str) -> Result<Vec<AtomicEdge>>;
    /// Explicit individual line or family, retaining original member records.
    fn emission(&self, element: &str, selection: &EmissionSelection) -> Result<AtomicEmission>;
    /// Chantler f₂ at positive, supported energies in eV; no extrapolation.
    fn f2(&self, element: &str, energy_ev: &[f64]) -> Result<AtomicCurve>;
    /// Elam total elemental mass attenuation in cm²/g.
    fn attenuation(&self, element: &str, energy_ev: &[f64]) -> Result<AtomicCurve>;
    /// Elam compound mass attenuation via explicit stoichiometric mass fractions.
    fn compound_attenuation(&self, formula: &str, energy_ev: &[f64])
        -> Result<CompoundAttenuation>;
}

/// Pinned offline provider. Cheap copies share an immutable, lazily decoded dataset.
#[derive(Clone, Copy)]
pub struct AtomicData {
    db: XrayDb,
    identity: &'static AtomicDataIdentity,
}
static IDENTITY: OnceLock<std::result::Result<AtomicDataIdentity, String>> = OnceLock::new();
impl AtomicData {
    /// Open the offline dataset. No network or filesystem lookup is performed.
    /// Fingerprints the actual loaded data once; upstream custom initialization
    /// cannot be mislabeled as the built-in blob. Loading failure returns an error.
    pub fn new() -> Result<Self> {
        let db = XrayDb::try_new().map_err(|e| AtomicDataError::Unavailable(e.to_string()))?;
        let identity = IDENTITY
            .get_or_init(|| {
                struct HashWriter(Sha256);
                impl Write for HashWriter {
                    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
                        self.0.update(bytes);
                        Ok(bytes.len())
                    }
                    fn flush(&mut self) -> std::io::Result<()> {
                        Ok(())
                    }
                }
                let mut writer = HashWriter(Sha256::new());
                serde_json::to_writer(&mut writer, db.raw()).map_err(|e| e.to_string())?;
                Ok(AtomicDataIdentity {
                    provider: "xraydb-rs/0.4.1/rexafs-atomic-v1".into(),
                    data_version: db
                        .raw()
                        .version
                        .iter()
                        .max_by_key(|v| {
                            v.tag
                                .split('.')
                                .map(|n| n.parse::<u32>().unwrap_or(0))
                                .collect::<Vec<_>>()
                        })
                        .map_or("unknown", |v| v.tag.as_str())
                        .into(),
                    data_sha256: writer
                        .0
                        .finalize()
                        .iter()
                        .map(|b| format!("{b:02x}"))
                        .collect(),
                })
            })
            .as_ref()
            .map_err(|e| AtomicDataError::Unavailable(e.clone()))?;
        Ok(Self { db, identity })
    }
    /// Actual data identity; retain it rather than assuming a crate version implies identical data.
    pub fn identity(&self) -> &AtomicDataIdentity {
        self.identity
    }
    /// Require a historical provider/dataset before recomputing an archived result.
    /// The current build never downloads or silently substitutes an older/newer table.
    pub fn require_reference(&self, reference: &AtomicReference) -> Result<()> {
        if &reference.data != self.identity {
            return Err(AtomicDataError::VersionMismatch);
        }
        Ok(())
    }
    fn reference(&self, table: AtomicTable) -> AtomicReference {
        AtomicReference {
            data: self.identity.clone(),
            table,
        }
    }
    /// Resolve an explicit element/shell. Energy is tabulated eV, not fitted E₀.
    pub fn edge(&self, element: &str, edge: &str) -> Result<AtomicEdge> {
        let symbol = self.db.symbol(element).map_err(invalid)?;
        let edge = edge.to_ascii_uppercase();
        let energy_ev = self.db.xray_edge(symbol, &edge).map_err(invalid)?.energy;
        if !energy_ev.is_finite() || energy_ev <= 0. {
            return Err(invalid("invalid tabulated edge"));
        }
        Ok(AtomicEdge {
            element: symbol.into(),
            edge,
            energy_ev,
            reference: self.reference(AtomicTable::ElamTransitionsV1),
        })
    }
    /// Return supported edges in deterministic IUPAC-label order.
    pub fn edges(&self, element: &str) -> Result<Vec<AtomicEdge>> {
        let mut labels = self
            .db
            .xray_edges(element)
            .map_err(invalid)?
            .into_keys()
            .collect::<Vec<_>>();
        labels.sort();
        labels.iter().map(|e| self.edge(element, e)).collect()
    }
    /// Resolve one exact line or an explicit family; no automatic strongest-line choice.
    pub fn emission(&self, element: &str, selection: &EmissionSelection) -> Result<AtomicEmission> {
        let symbol = self.db.symbol(element).map_err(invalid)?;
        let wanted = match selection {
            EmissionSelection::Line(s) | EmissionSelection::Family(s) => s,
        };
        if wanted.is_empty() {
            return Err(invalid("select an emission line or family"));
        }
        let mut lines = self
            .db
            .xray_lines(symbol, None, None)
            .map_err(invalid)?
            .into_iter()
            .filter(|(name, _)| match selection {
                EmissionSelection::Line(_) => name == wanted,
                EmissionSelection::Family(_) => {
                    name.trim_end_matches(|c: char| c.is_ascii_digit()) == wanted
                }
            })
            .map(|(name, line)| AtomicLine {
                name,
                energy_ev: line.energy,
                intensity: line.intensity,
                initial_level: line.initial_level,
                final_level: line.final_level,
            })
            .collect::<Vec<_>>();
        lines.sort_by(|a, b| a.name.cmp(&b.name));
        let first = lines
            .first()
            .ok_or_else(|| invalid(format!("unknown emission {wanted} for {symbol}")))?;
        if lines.iter().any(|l| l.initial_level != first.initial_level) {
            return Err(invalid(
                "emission family spans different initial shells; select one line",
            ));
        }
        let sum = lines.iter().map(|l| l.intensity).sum::<f64>();
        if !sum.is_finite()
            || sum <= 0.
            || lines.iter().any(|l| {
                !l.energy_ev.is_finite()
                    || l.energy_ev <= 0.
                    || !l.intensity.is_finite()
                    || l.intensity < 0.
            })
        {
            return Err(invalid("invalid emission records"));
        }
        let energy_ev = lines
            .iter()
            .map(|l| l.energy_ev * (l.intensity / sum))
            .sum();
        Ok(AtomicEmission {
            element: symbol.into(),
            selection: selection.clone(),
            energy_ev,
            lines,
            reference: self.reference(AtomicTable::ElamTransitionsV1),
        })
    }
    /// Interpolate Chantler f₂ in log energy/log value, retaining the requested grid.
    /// Reject nonfinite/out-of-table energies rather than clamping or shifting edges.
    pub fn f2(&self, element: &str, energy_ev: &[f64]) -> Result<AtomicCurve> {
        checked_energies(
            energy_ev,
            self.db.chantler_energy_range(element).map_err(invalid)?,
        )?;
        self.curve(
            energy_ev,
            self.db.f2_chantler(element, energy_ev).map_err(invalid)?,
            "electron",
            AtomicTable::ChantlerF2LogLogV1,
        )
    }
    /// Total elemental mass attenuation in cm²/g, using the Elam log splines.
    /// Reject unsupported energies; no density is needed for a mass coefficient.
    pub fn attenuation(&self, element: &str, energy_ev: &[f64]) -> Result<AtomicCurve> {
        checked_energies(energy_ev, XrayDb::elam_energy_range())?;
        self.curve(
            energy_ev,
            self.db
                .mu_elam(element, energy_ev, CrossSectionKind::Total)
                .map_err(invalid)?,
            "cm^2/g",
            AtomicTable::ElamTotalV1,
        )
    }
    /// Independent-atom mass-fraction sum for a stoichiometric formula, in cm²/g.
    /// Supports canonical element symbols, positive decimal counts and parentheses.
    /// Rejects isotopes, charge, hydrate dots, named materials and scientific notation
    /// with a byte-position error; use explicit parentheses for hydrated composition.
    /// No density or implicit absorber-only approximation is introduced.
    pub fn compound_attenuation(
        &self,
        formula: &str,
        energy_ev: &[f64],
    ) -> Result<CompoundAttenuation> {
        let atoms = formula::parse(formula, &self.db)?;
        let mut composition = Vec::new();
        let mut total = 0.;
        for (element, atoms) in atoms {
            let molar_mass = self.db.molar_mass(&element).map_err(invalid)?;
            total += atoms * molar_mass;
            composition.push(ElementFraction {
                element,
                atoms,
                molar_mass,
                mass_fraction: 0.,
            });
        }
        if !total.is_finite() || total <= 0. {
            return Err(invalid("formula mass must be finite and positive"));
        }
        let mut values = vec![0.; energy_ev.len()];
        for part in &mut composition {
            part.mass_fraction = part.atoms * part.molar_mass / total;
            let attenuation = self.attenuation(&part.element, energy_ev)?;
            for (sum, value) in values.iter_mut().zip(attenuation.values) {
                *sum += part.mass_fraction * value;
            }
        }
        let curve = self.curve(energy_ev, values, "cm^2/g", AtomicTable::ElamTotalV1)?;
        Ok(CompoundAttenuation {
            formula: formula.into(),
            composition,
            curve,
        })
    }
    fn curve(
        &self,
        energy_ev: &[f64],
        values: Vec<f64>,
        unit: &str,
        table: AtomicTable,
    ) -> Result<AtomicCurve> {
        if values.len() != energy_ev.len() || values.iter().any(|v| !v.is_finite() || *v <= 0.) {
            return Err(invalid("tabulated values must be finite and positive"));
        }
        Ok(AtomicCurve {
            energy_ev: energy_ev.to_vec(),
            values,
            unit: unit.into(),
            reference: self.reference(table),
        })
    }
}
fn checked_energies(energy: &[f64], range: (f64, f64)) -> Result<()> {
    if energy.is_empty() {
        return Err(invalid("provide at least one energy in eV"));
    }
    for (i, &value) in energy.iter().enumerate() {
        if !value.is_finite() || value < range.0 || value > range.1 || value <= 0. {
            return Err(invalid(format!(
                "energy[{i}]={value} eV is outside {}..={} eV",
                range.0, range.1
            )));
        }
    }
    Ok(())
}
impl AtomicDataProvider for AtomicData {
    fn identity(&self) -> &AtomicDataIdentity {
        self.identity()
    }
    fn edge(&self, e: &str, s: &str) -> Result<AtomicEdge> {
        self.edge(e, s)
    }
    fn edges(&self, e: &str) -> Result<Vec<AtomicEdge>> {
        self.edges(e)
    }
    fn emission(&self, e: &str, s: &EmissionSelection) -> Result<AtomicEmission> {
        self.emission(e, s)
    }
    fn f2(&self, e: &str, x: &[f64]) -> Result<AtomicCurve> {
        self.f2(e, x)
    }
    fn attenuation(&self, e: &str, x: &[f64]) -> Result<AtomicCurve> {
        self.attenuation(e, x)
    }
    fn compound_attenuation(&self, f: &str, x: &[f64]) -> Result<CompoundAttenuation> {
        self.compound_attenuation(f, x)
    }
}
