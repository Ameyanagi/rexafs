//! FLUO-style correction of fluorescence over-absorption (since 0.2.10).
//!
//! This homogeneous, optically thick model is intended for XANES. It is not
//! qualified here for EXAFS or finite-thickness samples. All geometry is explicit;
//! angles are measured from the sample surface. Internal conventional normalization
//! is calculated automatically and retained separately from final normalization.
//! See [Larch's applicability note](https://xraypy.github.io/xraylarch/xafs_preedge.html#over-absorption-corrections).
mod calculate;
#[cfg(test)]
mod tests;
use crate::atomic::{
    AtomicData, AtomicDataIdentity, AtomicEdge, AtomicEmission, CompoundAttenuation,
    EmissionSelection,
};
use serde::{Deserialize, Serialize};
use std::ops::RangeInclusive;

/// Input-signal interpretation. A detector ratio alone does not establish fluorescence.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum AbsorptionMode {
    /// No reliable acquisition-mode evidence is attached.
    #[default]
    Unknown,
    /// Logarithm of incident/transmitted intensities; cannot use this correction.
    Transmission,
    /// Fluorescence absorption, established by source evidence or explicit interpretation.
    Fluorescence,
}

impl AbsorptionMode {
    pub(crate) fn is_unknown(&self) -> bool {
        *self == Self::Unknown
    }
}

/// A failed correction has no corrected output and never clips a denominator.
#[derive(Debug, thiserror::Error)]
#[error("Fluorescence correction: {0}")]
pub struct FluorescenceError(pub String);
type Result<T> = std::result::Result<T, FluorescenceError>;
fn invalid(value: impl std::fmt::Display) -> FluorescenceError {
    FluorescenceError(value.to_string())
}

/// Thick-sample XANES correction settings. Prefer [`Self::new`], [`Self::line`]
/// and [`Self::angles`]. Construction does not calculate or alter any spectrum.
///
/// ```no_run
/// # use rexafs::{Spectrum, FluorescenceCorrection};
/// # fn example(spectrum: &Spectrum) -> Result<(), Box<dyn std::error::Error>> {
/// let correction = FluorescenceCorrection::new("CuO", "Cu", "K")
///     .line("Ka1").angles(45.0, 45.0); // supply measured angles, in degrees
/// let mut corrected = spectrum.correct_fluorescence(&correction)?;
/// corrected.normalize()?; // final normalization is a separate operation
/// # Ok(()) }
/// ```
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FluorescenceCorrection {
    /// Complete homogeneous sample stoichiometry, including diluent/matrix elements.
    pub formula: String,
    /// Absorbing element symbol; the formula must contain it.
    pub element: String,
    /// IUPAC absorption shell, such as K or L3.
    pub edge: String,
    /// Explicit detected line or within-shell family. No emission is guessed.
    pub emission: Option<EmissionSelection>,
    /// Incident angle from the sample surface in degrees, in (0, 90].
    pub incidence_deg: Option<f64>,
    /// Exit angle from the sample surface in degrees, in (0, 90].
    pub exit_deg: Option<f64>,
    /// Fixed measured E₀ in eV; None detects the edge before the internal fit.
    pub e0: Option<f64>,
    /// Internal pre-edge interval, in eV from measured E₀. None uses the available
    /// low-energy endpoint to −30 eV; insufficient coverage fails.
    pub pre_edge: Option<[f64; 2]>,
    /// Internal post-edge interval, in eV from measured E₀. None uses +100 eV to
    /// the available endpoint. Explicit intervals require complete coverage.
    pub post_edge: Option<[f64; 2]>,
    /// Internal conventional post-edge polynomial degree, 0–5, default 1.
    /// No Victoreen term or imposed edge step is used in this named profile.
    pub degree: usize,
    /// Historical offline data identity required for exact recomputation.
    /// None selects the packaged data; [`FluorescenceCorrectionResult::definition`]
    /// pins it together with resolved internal intervals and E₀.
    pub reference: Option<AtomicDataIdentity>,
}
impl FluorescenceCorrection {
    /// Select sample stoichiometry, absorber and edge. Line and measured angles
    /// must be supplied before calculation; no geometry or density is invented.
    /// Internal normalization defaults to a linear pre/post fit and measured E₀.
    pub fn new(
        formula: impl Into<String>,
        element: impl Into<String>,
        edge: impl Into<String>,
    ) -> Self {
        Self {
            formula: formula.into(),
            element: element.into(),
            edge: edge.into(),
            emission: None,
            incidence_deg: None,
            exit_deg: None,
            e0: None,
            pre_edge: None,
            post_edge: None,
            degree: 1,
            reference: None,
        }
    }
    /// Select one exact emission line (for example Ka1). Use [`Self::line_family`]
    /// explicitly for an unresolved family, whose energy is intensity-weighted.
    pub fn line(mut self, line: impl Into<String>) -> Self {
        self.emission = Some(EmissionSelection::Line(line.into()));
        self
    }
    /// Select an unresolved within-shell family (for example Ka), not an individual line.
    pub fn line_family(mut self, family: impl Into<String>) -> Self {
        self.emission = Some(EmissionSelection::Family(family.into()));
        self
    }
    /// Set measured incident and exit angles, in that order, from the surface.
    /// Normal incidence is 90°. Both must be finite, greater than 0° and at most 90°.
    pub fn angles(mut self, incidence_deg: f64, exit_deg: f64) -> Self {
        self.incidence_deg = Some(incidence_deg);
        self.exit_deg = Some(exit_deg);
        self
    }
    /// Fix measured E₀ (eV); does not shift the tabulated atomic edge.
    pub fn e0(mut self, value: f64) -> Self {
        self.e0 = Some(value);
        self
    }
    /// Set the internal pre-edge fit interval in eV from E₀. No silent clipping.
    pub fn pre_edge(mut self, range: RangeInclusive<f64>) -> Self {
        self.pre_edge = Some([*range.start(), *range.end()]);
        self
    }
    /// Set the internal post-edge fit interval in eV from E₀. No silent clipping.
    pub fn post_edge(mut self, range: RangeInclusive<f64>) -> Self {
        self.post_edge = Some([*range.start(), *range.end()]);
        self
    }
    /// Set the internal polynomial degree, 0–5; higher flexibility changes the correction.
    pub fn degree(mut self, degree: usize) -> Self {
        self.degree = degree;
        self
    }
    /// Correct explicitly interpreted fluorescence arrays on their original grid.
    ///
    /// Copies finite, strictly increasing positive energy (eV) and matching μ arrays.
    /// Calculates missing conventional normalization automatically; never uses a
    /// prior MBACK/flat curve. Returns the original inputs, corrected μ, factor,
    /// internal fit and atomic provenance. Does not perform final normalization.
    /// Rejects invalid geometry/composition, nonpositive fitted step, incomplete
    /// intervals and nonpositive or numerically singular correction denominators.
    /// Array uncertainties are unavailable: the internal fit depends on the input.
    pub fn apply(&self, energy: &[f64], mu: &[f64]) -> Result<FluorescenceCorrectionResult> {
        calculate::apply(self, energy, mu, &AtomicData::new().map_err(invalid)?)
    }
}

/// Internal conventional fit, distinct from normalization of the corrected output.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FluorescenceInternalNormalization {
    /// Fixed measured E₀ in eV (selected or automatically detected).
    pub e0: f64,
    /// Resolved pre-edge offsets from E₀ in eV.
    pub pre_edge: [f64; 2],
    /// Resolved post-edge offsets from E₀ in eV.
    pub post_edge: [f64; 2],
    /// Post-edge polynomial degree; pre-edge degree is one, Victoreen exponent zero.
    pub degree: usize,
    /// Fitted positive edge jump in original μ units, before any numerical floor.
    pub edge_step: f64,
    /// Pre-edge line in original μ units, on the input energy grid.
    pub pre_curve: Vec<f64>,
    /// Pre-edge line plus post-edge polynomial, in original μ units.
    pub post_curve: Vec<f64>,
    /// Dimensionless internal n₀ used in the correction denominator.
    pub norm: Vec<f64>,
}

/// Independent correction record; original arrays and assumptions remain available.
/// No covariance is claimed for the corrected arrays.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FluorescenceCorrectionResult {
    /// Named profile and normalization convention, currently `fluo_elam_v1`.
    pub method: String,
    /// Requested settings, including any automatic normalization choices.
    pub requested: FluorescenceCorrection,
    /// Original acquisition evidence; array calls explicitly interpret fluorescence.
    pub input_mode: AbsorptionMode,
    /// Original energies (eV), unchanged in order or values.
    pub energy: Vec<f64>,
    /// Original uncorrected fluorescence μ, retained unchanged.
    pub original_mu: Vec<f64>,
    /// Corrected μ on the same grid and in the same units.
    pub corrected_mu: Vec<f64>,
    /// Atomic edge, distinct from measured E₀.
    pub edge: AtomicEdge,
    /// Exact emission selection, energy and constituent lines.
    pub emission: AtomicEmission,
    /// Formula mass fractions and total attenuation (cm²/g) at emission, edge−10,
    /// edge+10 eV, in that order, with provider/table identities.
    pub attenuation: CompoundAttenuation,
    /// sin(incident angle) / sin(exit angle), dimensionless.
    pub geometry_ratio: f64,
    /// Constant α of the named FLUO profile, dimensionless.
    pub alpha: f64,
    /// Internal fit of the uncorrected μ, including its resolved settings.
    pub internal: FluorescenceInternalNormalization,
    /// d(E) = α + 1 − n₀(E), dimensionless; never clipped.
    pub denominator: Vec<f64>,
    /// Multiplicative α/d(E), dimensionless; never clipped.
    pub factor: Vec<f64>,
    /// Smallest denominator on the whole input grid.
    pub minimum_denominator: f64,
    /// Largest multiplicative factor on the whole input grid.
    pub maximum_amplification: f64,
    /// Numerical rejection threshold: 64*machine epsilon*max(1, α+1).
    pub singularity_threshold: f64,
    /// Domain and numerical diagnostics. Thresholds are engineering diagnostics,
    /// not experimental confidence limits.
    pub warnings: Vec<String>,
}
impl FluorescenceCorrectionResult {
    /// Reusable exact-replay settings: freeze resolved normalization and atomic data.
    pub fn definition(&self) -> FluorescenceCorrection {
        let mut model = self.requested.clone();
        model.e0 = Some(self.internal.e0);
        model.pre_edge = Some(self.internal.pre_edge);
        model.post_edge = Some(self.internal.post_edge);
        model.reference = Some(self.edge.reference.data.clone());
        model
    }
}
