//! Full Chantler MBACK normalization (since 0.2.10).
//!
//! [`MBack::for_edge`] selects an explicit absorber and edge. [`MBack::fit`]
//! borrows energy/absorption arrays; using it as a spectrum normalization method
//! stores outputs and invalidates downstream processing through the usual setters.
//! Defaults are a quadratic smooth background and no complementary-error-function
//! term. The atomic table is never energy shifted. See `doc/mback-normalization.md`
//! for the objective, normalization convention and implementation differences from
//! [Weng et al. (2005)](https://doi.org/10.1107/S0909049504034193).
mod solver;
#[cfg(test)]
mod tests;
use super::errors::NormalizationError;
use super::normalization::Normalization;
use crate::atomic::{
    AtomicData, AtomicDataProvider, AtomicEmission, AtomicReference, EmissionSelection,
};
use nalgebra::DVector;
use serde::{Deserialize, Serialize};
use std::ops::RangeInclusive;
#[cfg(not(feature = "ndarray-compat"))]
type Array = DVector<f64>;
#[cfg(feature = "ndarray-compat")]
type Array = ndarray::Array1<f64>;

pub(super) fn invalid(message: impl std::fmt::Display) -> NormalizationError {
    NormalizationError::Other {
        message: format!("MBACK: {message}"),
    }
}

/// Optional fluorescence-background shape. This is not a self-absorption correction.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MbackErfc {
    /// Exact emission line or explicit within-shell family; must match the chosen edge.
    pub emission: EmissionSelection,
    /// Finite, positive lower/upper width bounds in eV. The fit searches log width.
    pub width_ev: [f64; 2],
    /// Finite amplitude bounds in f₂ units. Signed amplitudes are permitted when
    /// explicitly requested. Bound-active results carry a warning.
    pub amplitude: [f64; 2],
}
impl MbackErfc {
    /// Select an individual emission line and explicit width/amplitude bounds.
    /// No geometry or detector interpretation is inferred from this background term.
    pub fn new(
        line: impl Into<String>,
        width_ev: RangeInclusive<f64>,
        amplitude: RangeInclusive<f64>,
    ) -> Self {
        Self {
            emission: EmissionSelection::Line(line.into()),
            width_ev: [*width_ev.start(), *width_ev.end()],
            amplitude: [*amplitude.start(), *amplitude.end()],
        }
    }
}

/// Requested MBACK settings. Prefer [`MBack::for_edge`] and its range builders.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MbackOptions {
    /// Explicit absorber symbol; empty historical settings cannot calculate a result.
    pub element: String,
    /// Explicit IUPAC edge label, such as K or L3.
    pub edge: String,
    /// Pre-edge fitting interval in eV from E₀. None suggests a scan-dependent interval.
    pub pre_edge: Option<[f64; 2]>,
    /// Post-edge fitting interval in eV from E₀. None suggests a scan-dependent interval.
    pub post_edge: Option<[f64; 2]>,
    /// Smooth-background polynomial degree, 0–5. Default 2; excessive degree can
    /// absorb real structure. Rank-deficient requests fail instead of reducing degree.
    pub degree: usize,
    /// Optional bounded complementary-error-function background; default disabled.
    pub erfc: Option<MbackErfc>,
    /// An archived reference required for exact recomputation; default current
    /// offline data. A different identity or table fails rather than substitutes.
    pub reference: Option<AtomicReference>,
}
impl Default for MbackOptions {
    fn default() -> Self {
        Self {
            element: String::new(),
            edge: String::new(),
            pre_edge: None,
            post_edge: None,
            degree: 2,
            erfc: None,
            reference: None,
        }
    }
}

/// Full MBACK result, retained independently of the input. All curves use the
/// original energy grid. Region-balancing weights are not measurement errors;
/// no statistical covariance or confidence interval is inferred from them.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MbackResult {
    /// Named numerical convention, currently `mback_chantler_v1`.
    pub method: String,
    /// Requested settings, before automatic ranges resolve.
    pub requested: MbackOptions,
    /// Requested fixed E₀, or None if the derivative detector was used.
    pub requested_e0: Option<f64>,
    /// Actual table and interpolation identity.
    pub reference: AtomicReference,
    /// Absorber's tabulated edge energy, in eV; distinct from measured E₀.
    pub tabulated_edge_ev: f64,
    /// Fixed resolved E₀ in eV; does not shift reference data.
    pub e0: f64,
    /// Resolved inclusive E₀-relative fit intervals, in eV.
    pub pre_edge: [f64; 2],
    /// Resolved inclusive E₀-relative post-edge interval, in eV.
    pub post_edge: [f64; 2],
    /// Original energy grid, in eV.
    pub energy: Vec<f64>,
    /// Original indices included in the balanced objective.
    pub fit_indices: Vec<usize>,
    /// Multipliers 1/sqrt(region point count), in the order of fit_indices.
    pub weights: Vec<f64>,
    /// Positive scale converting input absorption to f₂ units.
    pub scale: f64,
    /// E_scale in t=(E-E₀)/E_scale, in eV; conditions the polynomial basis.
    pub energy_scale: f64,
    /// Increasing powers of t, in f₂ units.
    pub coefficients: Vec<f64>,
    /// Resolved optional emission reference; none when erfc is disabled.
    pub emission: Option<AtomicEmission>,
    /// Fitted erfc width in eV; none when disabled.
    pub erfc_width: Option<f64>,
    /// Fitted erfc amplitude in f₂ units; zero when disabled.
    pub erfc_amplitude: f64,
    /// B(E), the polynomial plus optional erfc, in f₂ units.
    pub background: Vec<f64>,
    /// Unshifted tabulated f₂, in electron units.
    pub f2: Vec<f64>,
    /// Matched signal s*mu-B, in f₂ units. This is not norm.
    pub fpp: Vec<f64>,
    /// Unweighted f₂+B-s*mu on all input points, including excluded near-edge data.
    pub residual: Vec<f64>,
    /// Sum of squared balanced residuals on fit_indices, in squared f₂ units.
    pub objective: f64,
    /// Rank of the final column-scaled Jacobian, including width when varied.
    pub rank: usize,
    /// Largest/smallest singular-value ratio of the column-scaled Jacobian.
    pub condition: f64,
    /// Number of linear solves used; one without erfc, multiple for bounded profiling.
    pub evaluations: usize,
    /// Fitted absorption edge step in the original mu units, positive and finite.
    pub edge_step: f64,
    /// Auxiliary pre-edge line P(E) on f₂+B, in f₂ units.
    pub pre_curve: Vec<f64>,
    /// Auxiliary quadratic post-edge curve on f₂+B, in f₂ units.
    pub post_curve: Vec<f64>,
    /// Auxiliary convention: linear pre-edge, quadratic post-edge, n_victoreen=0,
    /// existing rexafs endpoint/nearest-E₀ convention, with the resolved ranges.
    pub auxiliary_method: String,
    /// Dimensionless (s*mu-P)/Delta, where Delta=scale*edge_step.
    pub norm: Vec<f64>,
    /// Norm with the auxiliary post-edge trend removed above nearest E₀.
    pub flat: Vec<f64>,
    /// Nonfatal conditioning, boundary and initialization diagnostics.
    pub warnings: Vec<String>,
}

/// Full MBACK model and optional cached output. The polynomial normalization
/// remains the default for spectra. Start with `MBack::for_edge("Cu", "K")`;
/// arrays, tables and provenance are managed automatically. Ranges are E₀ offsets.
///
/// Historical empty placeholders deserialize, but need absorber/edge selection
/// before recalculation. Their stored arrays are left readable.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MBack {
    /// Fixed edge energy in eV, or None for automatic derivative detection.
    pub e0: Option<f64>,
    /// Last fitted step in input mu units. Unlike polynomial normalization this
    /// is an output, not an override: MBACK determines its scale from atomic data.
    pub edge_step: Option<f64>,
    /// Cached dimensionless normalized output; refreshed only after successful fit.
    pub norm: Option<Array>,
    /// Cached dimensionless flattened output; refreshed only after successful fit.
    pub flat: Option<Array>,
    /// Scientific settings; prefer builders for the common path.
    pub options: MbackOptions,
    /// Full saved result and diagnostics, absent until a successful calculation.
    pub result: Option<Box<MbackResult>>,
}
impl MBack {
    /// Historical empty constructor. Prefer `for_edge(element, edge)`; no absorber
    /// is guessed and fitting an empty model returns a selection error.
    pub fn new() -> Self {
        Self::default()
    }
    /// Select absorber and edge. Degree 2, erfc off, current offline reference.
    /// Unspecified ranges use the outer 80% of the available pre/post spans,
    /// bounded by neighboring tabulated edges with a 10 eV margin. All resolved
    /// intervals are retained; explicit intervals are never silently shortened.
    pub fn for_edge(element: impl Into<String>, edge: impl Into<String>) -> Self {
        Self {
            options: MbackOptions {
                element: element.into(),
                edge: edge.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    }
    /// Fix measured E₀ in eV without moving the reference table. It must lie inside the scan.
    pub fn e0(mut self, energy_ev: f64) -> Self {
        self.e0 = Some(energy_ev);
        self
    }
    /// Explicit inclusive pre-edge interval, in eV relative to E₀.
    pub fn pre_edge(mut self, range: RangeInclusive<f64>) -> Self {
        self.options.pre_edge = Some([*range.start(), *range.end()]);
        self
    }
    /// Explicit inclusive post-edge interval, in eV relative to E₀.
    pub fn post_edge(mut self, range: RangeInclusive<f64>) -> Self {
        self.options.post_edge = Some([*range.start(), *range.end()]);
        self
    }
    /// Smooth-background polynomial degree, 0–5; default 2. The auxiliary
    /// normalization always uses degree 2 regardless of this choice.
    pub fn degree(mut self, degree: usize) -> Self {
        self.options.degree = degree;
        self
    }
    /// Enable the optional bounded erfc background with an explicit emission choice.
    pub fn erfc(mut self, term: MbackErfc) -> Self {
        self.options.erfc = Some(term);
        self
    }
    /// Require the original atomic reference when replaying archived processing.
    pub fn reference(mut self, reference: AtomicReference) -> Self {
        self.options.reference = Some(reference);
        self
    }
    /// Normalize borrowed arrays without modifying this model or the inputs.
    /// Requires matching finite arrays, strictly increasing positive energy in eV,
    /// coverage of both regions, identifiable parameters, and a positive scale/step.
    /// Default data are loaded offline automatically. Returns norm/flat plus diagnostics.
    pub fn fit(&self, energy: &[f64], mu: &[f64]) -> Result<MbackResult, NormalizationError> {
        let data = AtomicData::new().map_err(invalid)?;
        self.fit_with_provider(energy, mu, &data)
    }
    /// Advanced dependency injection for a qualified reference provider. Ordinary
    /// callers use `fit`; a historical reference must match the returned identity.
    pub fn fit_with_provider(
        &self,
        energy: &[f64],
        mu: &[f64],
        data: &dyn AtomicDataProvider,
    ) -> Result<MbackResult, NormalizationError> {
        solver::fit(self, energy, mu, data)
    }
    /// Retained compatibility no-op. Defaults resolve from the input during fit.
    pub fn fill_parameter(&mut self) {}
}
impl Normalization for MBack {
    fn normalize(
        &mut self,
        energy: &DVector<f64>,
        mu: &DVector<f64>,
    ) -> Result<&mut Self, NormalizationError> {
        // Do not expose stale arrays after a failed standalone recalculation.
        self.norm = None;
        self.flat = None;
        self.result = None;
        self.edge_step = None;
        let result = self.fit(energy.as_slice(), mu.as_slice())?;
        self.e0 = Some(result.e0);
        self.options.reference = Some(result.reference.clone());
        self.edge_step = Some(result.edge_step);
        self.norm = Some(Array::from_vec(result.norm.clone()));
        self.flat = Some(Array::from_vec(result.flat.clone()));
        self.result = Some(Box::new(result));
        Ok(self)
    }
    fn get_e0(&self) -> Option<f64> {
        self.e0
    }
    fn get_edge_step(&self) -> Option<f64> {
        self.edge_step
    }
    fn get_norm(&self) -> Option<&Array> {
        self.norm.as_ref()
    }
    fn get_flat(&self) -> Option<&Array> {
        self.flat.as_ref()
    }
    fn set_e0(&mut self, e0: Option<f64>) -> &mut Self {
        self.e0 = e0;
        self
    }
    fn set_edge_step(&mut self, step: Option<f64>) -> &mut Self {
        self.edge_step = step;
        self
    }
}
