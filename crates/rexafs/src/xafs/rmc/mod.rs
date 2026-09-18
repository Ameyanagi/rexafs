//! Experimental reverse Monte Carlo (RMC) refinement, added after 0.2.9 (unreleased).
//!
//! RMC proposes atomic displacements and compares calculated extended X-ray
//! absorption fine structure (EXAFS) with measured, unweighted χ(k). This module
//! uses symmetric single-atom proposals and Metropolis acceptance. It is a
//! Rust implementation with exact affected-path updates and an opt-in
//! ReFEFF mode that pins reference potentials while updating geometry and paths.
//! No EVAX or RMCProfile source is incorporated.
//!
//! Start from processed [`crate::Spectrum`] with [`RmcDataset::from_spectrum`],
//! [`RmcSpectrumOptions`] and [`EnsembleProblem::single`]. This retains preprocessing
//! state and checks the R fit range against AUTOBK Rbkg. The input is never
//! reprocessed during refinement; fitting settings remain explicit.
//!
//! Use [`RmcSession`] with [`EnsembleProblem`] and [`SessionSettings`] for resumable
//! runs, weighted structures, constraints, K/R/q/wavelet objectives and analysis.
//! [`EvolutionSession`] adds population search. The simpler [`RmcProblem`],
//! [`RmcSettings`] and [`refine`] interface remains available. Enable Cargo feature
//! `refeff-runner` for `RefeffCalculator`. Finite clusters and explicit periodic
//! cells are supported. `PreparedRefeffCalculator` adds an immutable path catalogue,
//! bounded batches and exact affected-path caching by default. Use
//! `AccelerationSettings::default()` for this recommended mode. Frozen/adaptive
//! representative bases are experimental, disabled by default, and require
//! explicit opt-in. The experimental `AdaptiveBasisController` schedules exact
//! audits and transactional stage changes; independent accuracy and end-to-end
//! speed still need qualification.
//! Atom order is
//! stable and there is no symmetry expansion
//! during refinement. Results are owned and serializable. Inputs remain unchanged.
//!
//! For dataset d with the default k-space objective, the spectral term is
//! `F_d = weight_d / N_d * Σ_i [(k_i / k_ref)^w (model_i − chi_i) / sigma_i]²`,
//! where `k_ref = 1 Å⁻¹`, `w` is the integer k weight, `N_d` is the point count,
//! and χ and its positive noise scale σ are dimensionless. Total F also includes
//! configured structural energies. Model χ averages selected absorbers within
//! each structure, then combines normalized mixture fractions and fixed
//! S₀². The fixed energy shift samples theory at
//! `q = sqrt(k² − ETOK * delta_e0)`, with `ETOK` in Å⁻²/eV. Negative q² is an
//! error. No extrapolation or extra Debye–Waller damping is applied.
//!
//! Uphill proposals are accepted with probability `exp(−ΔF / (2*T))`, where T
//! is a dimensionless numerical tolerance, not a physical temperature. At T=0,
//! only non-increasing scores are accepted. Dataset normalization and k weighting
//! are rexafs choices; this score is not a reduced chi-square. Neither a low score
//! nor accepted configurations establish structural uniqueness or confidence
//! intervals. See `doc/rmc.md` for scope, usage, limitations and validation.
//!
//! Method background: [McGreevy and Pusztai (1988)](https://doi.org/10.1080/08927028808080958).
//! Scattering background: [Rehr and Albers (2000)](https://doi.org/10.1103/RevModPhys.72.621).

#[cfg(feature = "refeff-runner")]
mod accelerated;
mod analysis;
mod calibration;
mod constraints;
mod convergence;
mod engine;
mod ensemble;
mod evolution;
mod first_shell;
mod geometry;
mod local_spectrum;
mod moments;
mod objective;
mod options;
mod paths;
#[cfg(feature = "refeff-runner")]
mod prepared;
mod proposals;
#[cfg(feature = "refeff-runner")]
mod refeff;
mod session;
mod spectrum;
mod structural;
mod workflows;

#[cfg(feature = "refeff-runner")]
pub use accelerated::*;
pub use analysis::*;
pub use calibration::*;
pub use constraints::*;
pub use convergence::*;
pub use engine::{evaluate, refine, refine_with_progress};
pub use ensemble::*;
pub use evolution::*;
pub use first_shell::*;
pub use geometry::{Atom, Configuration};
pub use local_spectrum::*;
pub use moments::*;
pub use objective::{transform_path_fourier, transform_spectrum_fourier, Objective};
pub use options::RefeffOptions;
pub use paths::*;
#[cfg(feature = "refeff-runner")]
pub use prepared::*;
pub use proposals::*;
#[cfg(feature = "refeff-runner")]
pub use refeff::{RefeffCacheStats, RefeffCalculator, RefeffStageTiming};
pub use session::*;
pub use spectrum::*;
pub use structural::*;
pub use workflows::*;

use crate::structure::Edge;
use serde::{Deserialize, Serialize};

/// Validation or calculation failure. Calculator failures abort the run instead
/// of being counted as rejected moves, which would bias the proposal process.
/// Stateful sessions preserve their accepted/best states and RNG on failure.
#[derive(Debug, thiserror::Error)]
pub enum RmcError {
    /// A setting, geometry, dataset or calculated array is invalid.
    #[error("invalid RMC input: {0}")]
    Invalid(String),
    /// The scattering backend failed, timed out or was interrupted.
    #[error("RMC calculator failed: {0}")]
    Calculator(String),
}

pub(crate) fn require(condition: bool, message: impl Into<String>) -> Result<(), RmcError> {
    if condition {
        Ok(())
    } else {
        Err(RmcError::Invalid(message.into()))
    }
}

/// Geometry-to-spectrum interface. Implementations must recompute the requested
/// configuration as a deterministic function of the request and fixed calculator
/// settings. Geometry-keyed caches are allowed; history-dependent potentials are
/// not. The
/// returned dimensionless χ must have exactly `k.len()` finite values, with
/// S₀²=1 and no added disorder damping. `k` is strictly increasing, in Å⁻¹.
/// The engine performs absorber averaging, fixed amplitude scaling and weighting.
pub trait ExafsCalculator {
    /// Human-readable backend identity included in the result provenance.
    fn name(&self) -> &str;
    /// Calculate one absorber's spectrum at the supplied theoretical k grid.
    /// Fail on unavailable k support rather than extrapolating. Atom indices are
    /// zero-based positions in `configuration.atoms`.
    fn calculate(
        &mut self,
        configuration: &Configuration,
        absorber: usize,
        edge: Edge,
        k: &[f64],
    ) -> Result<Vec<f64>, RmcError>;
    /// Stable scientific identity used to reject incompatible checkpoint resumes.
    /// Custom backends should include versions and all settings affecting χ.
    fn identity(&self) -> String {
        self.name().to_owned()
    }

    /// Extended calculation with per-dataset settings and optional path output.
    /// The default supports legacy calculators when neither option is requested.
    fn calculate_request(
        &mut self,
        request: CalculationRequest<'_>,
    ) -> Result<CalculatedSpectrum, RmcError> {
        require(
            request.options.is_none() && !request.paths,
            "this calculator does not support per-dataset options or path reports",
        )?;
        Ok(CalculatedSpectrum {
            chi: self.calculate(
                request.configuration,
                request.absorber,
                request.edge,
                request.k,
            )?,
            paths: Vec::new(),
        })
    }
    /// Evaluate independent absorber requests in input order. The default is
    /// serial; prepared calculators can share immutable scattering contexts and
    /// use a bounded worker pool. Errors must not be interpreted as MC rejections.
    fn calculate_batch(
        &mut self,
        requests: &[CalculationRequest<'_>],
    ) -> Result<Vec<CalculatedSpectrum>, RmcError> {
        requests
            .iter()
            .map(|request| self.calculate_request(*request))
            .collect()
    }
}

/// One measured EXAFS dataset. All fields are explicit to prevent silent changes
/// in the objective when data are imported. Use unweighted χ, not k²χ.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ExafsDataset {
    /// Unique, nonempty name retained in calculated output.
    pub name: String,
    /// Distinct, zero-based atoms of one element, averaged with equal weights.
    /// Supply a representative set; selecting a single atom does not average a cell.
    pub absorbers: Vec<usize>,
    /// Absorption edge; the same edge is used for all selected atoms.
    pub edge: Edge,
    /// Strictly increasing nonnegative wave numbers in Å⁻¹; at least two points.
    pub k: Vec<f64>,
    /// Unweighted, dimensionless experimental χ(k), one value per k point.
    pub chi: Vec<f64>,
    /// Positive noise scales for unweighted χ, one per k point. Correlations are
    /// not modeled. A constant vector is allowed but must be chosen explicitly.
    pub sigma: Vec<f64>,
    /// Positive relative dataset weight, applied after averaging squared residuals.
    pub weight: f64,
    /// Exponent 0..=3 of k/(1 Å⁻¹) applied to residuals; start with 0.
    pub kweight: u8,
    /// Fixed positive amplitude reduction S₀², usually calibrated independently.
    pub s02: f64,
    /// Fixed energy shift in eV. Positive values sample theory at smaller k.
    pub delta_e0: f64,
}

impl ExafsDataset {
    /// Copy the theoretical k grid in Å⁻¹ after applying this dataset's fixed
    /// fitting ΔE₀ (unreleased). Uses the same conversion as the optimizer;
    /// use this for `AdaptiveBasisSettings::k` to avoid an accidental exact-path
    /// fallback caused by training on the unshifted experimental grid.
    /// Requires at least two increasing finite nonnegative k values and finite
    /// ΔE₀. Imaginary or numerically unresolved shifted values are errors.
    /// No scattering, interpolation or preprocessing runs; inputs are unchanged.
    pub fn theoretical_k(&self) -> Result<Vec<f64>, RmcError> {
        require(
            self.k.len() >= 2
                && self.k.iter().all(|k| k.is_finite() && *k >= 0.)
                && self.k.windows(2).all(|w| w[1] > w[0])
                && self.delta_e0.is_finite(),
            "theoretical k needs increasing nonnegative samples and finite delta_e0",
        )?;
        engine::shifted_grid(self)
    }
}

/// Geometry and all datasets to refine together. No experimental data are
/// downloaded or preprocessed automatically.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RmcProblem {
    /// Initial explicit atoms and optional periodic cell, copied during refinement.
    pub configuration: Configuration,
    /// Nonempty set of uniquely named EXAFS datasets.
    pub datasets: Vec<ExafsDataset>,
}

/// Proposal and acceptance settings. Defaults are illustrative numerical values,
/// not material-specific constraints; calibrate noise and distances for the sample.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct RmcSettings {
    /// Number of attempted moves, including hard-constraint rejections; default 100.
    pub steps: usize,
    /// ChaCha8 seed; default 0. Repeatability also requires identical backend,
    /// threading, inputs and dependency versions.
    pub seed: u64,
    /// Maximum Cartesian displacement per axis per proposal in Å; default 0.05.
    /// Each component is drawn uniformly from [−step_size, step_size).
    pub step_size: f64,
    /// Dimensionless Metropolis tolerance T; default 1. Zero selects greedy descent.
    pub temperature: f64,
    /// Global hard lower bound on pair distances in Å; default 1.0. Periodic
    /// images, including images of the same atom, are checked. Must be positive.
    pub min_distance: f64,
    /// Optional spherical displacement limit in Å relative to each initial atom;
    /// default Some(0.5). Uses unwrapped coordinates even in periodic cells.
    pub max_displacement: Option<f64>,
    /// Distinct movable atom indices. Empty selects all atoms; unlisted atoms are
    /// fixed. Fix at least one atom when absolute translation is irrelevant.
    pub movable_atoms: Vec<usize>,
}

impl Default for RmcSettings {
    fn default() -> Self {
        Self {
            steps: 100,
            seed: 0,
            step_size: 0.05,
            temperature: 1.0,
            min_distance: 1.0,
            max_displacement: Some(0.5),
            movable_atoms: Vec::new(),
        }
    }
}

/// Calculated spectrum on the corresponding dataset's experimental k grid.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DatasetFit {
    /// Input dataset name.
    pub name: String,
    /// Averaged and S₀²-scaled, unweighted dimensionless χ(k).
    pub chi: Vec<f64>,
    /// Weighted mean squared residual contribution to the total objective.
    pub score: f64,
}

/// Objective and calculated spectra for one geometry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Evaluation {
    /// Sum of dataset scores and, for an ensemble session, structural penalties.
    /// A numerical objective in the chosen comparison convention.
    pub score: f64,
    /// Calculations in input dataset order.
    pub datasets: Vec<DatasetFit>,
}

/// Geometry paired with its exact calculated spectra and score.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RmcState {
    /// Explicit atom positions; periodic coordinates remain unwrapped.
    pub configuration: Configuration,
    /// Evaluation of this configuration, never a rejected proposal's spectrum.
    pub evaluation: Evaluation,
}

/// Outcome of one attempted move, also sent to the progress callback.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RmcStep {
    /// One-based attempted move number.
    pub step: usize,
    /// Zero-based moved atom index.
    pub atom: usize,
    /// Whether the current state was replaced by this proposal.
    pub accepted: bool,
    /// Whether a hard constraint rejected the proposal before calculation.
    pub constraint_rejected: bool,
    /// Trial score, or None if no scattering calculation was performed.
    pub trial_score: Option<f64>,
    /// Current state's score after acceptance or rejection.
    pub score: f64,
    /// Smallest score encountered so far.
    pub best_score: f64,
}

/// Owned refinement result. This is a record, not an exact-resume checkpoint or
/// a posterior sample. Retain the input problem and backend options alongside it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RmcResult {
    /// Backend's declared identity.
    pub calculator: String,
    /// Exact settings used for proposals and acceptance.
    pub settings: RmcSettings,
    /// Initial configuration and calculation.
    pub initial: RmcState,
    /// Lowest-scoring accepted state, including the initial state.
    pub best: RmcState,
    /// Last accepted state; may have a higher score than `best` when T>0.
    pub final_state: RmcState,
    /// One record per attempted move. Does not contain coordinate trajectories.
    pub history: Vec<RmcStep>,
    /// True if the progress callback requested termination, even on the last step.
    pub stopped: bool,
}
