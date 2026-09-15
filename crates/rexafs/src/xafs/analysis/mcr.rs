//! Multivariate curve resolution by alternating least squares (MCR-ALS).
//!
//! Fit `D = C S + R`: D has one spectrum per row, C contains nonnegative
//! coefficients, S has one component spectrum per row, and R is the residual.
//! Each half-step minimizes squared residuals with the other factor fixed.
//! This is an independent Rust implementation of the bilinear model described
//! in <https://doi.org/10.6028/jres.124.018>, using nalgebra SVD and rexafs's
//! active-set constrained least squares. No upstream implementation is copied.
//! Initialization, stopping thresholds and API defaults are rexafs choices.
//!
//! A small residual does not identify unique pure spectra: rotations, scaling
//! and permutation can leave the mixture unchanged. Nonnegativity and closure
//! restrict this ambiguity without generally removing it. Known compositions
//! are additional experimental information, supplied explicitly as anchors.

use std::borrow::Borrow;

use nalgebra::{DMatrix, DVector};
use serde::{Deserialize, Serialize};

use super::{lcf::bounded_lstsq, spectrum_label, AnalysisInput, AnalysisSpace};
use crate::xafs::{errors::AnalysisError, xasspectrum::XASSpectrum};

/// An explicitly known composition, not an inferred pure sample.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McrAnchor {
    /// Zero-based row in the supplied collection.
    pub sample: usize,
    /// One finite nonnegative coefficient per component, summing to one when
    /// closure is enabled. For a known pure first component use `[1, 0, 0]`.
    pub weights: Vec<f64>,
}

/// Settings for native MCR-ALS (unreleased).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McrConfig {
    /// Normalized or flattened absorption. Default: normalized. Missing arrays
    /// are prepared on copies using input settings; existing results are reused.
    /// No smoothing, energy alignment or centering is introduced.
    pub space: AnalysisSpace,
    /// Energy offsets in eV from the FIRST spectrum's E0. Default: −20 to +30.
    /// The entire interval must lie within every spectrum's measured coverage.
    pub range: Option<(f64, f64)>,
    /// Number of estimated factors. Default: 3. PCA can inform this choice but
    /// does not prove a chemical species count. Must not exceed matrix rank.
    pub components: usize,
    /// Require each coefficient row to sum to one. Default: true, appropriate
    /// for convex mixtures of consistently normalized absorption standards.
    pub sum_to_one: bool,
    /// Constrain S to be nonnegative. Default: false, since baseline-subtracted
    /// absorption can be slightly negative. Input values are never clipped.
    pub nonnegative_spectra: bool,
    /// Maximum COMPLETE alternating iterations. Default: 500; reaching this
    /// limit returns a usable best result with `IterationLimit`, not convergence.
    pub max_iterations: usize,
    /// Stop when relative SSE improvement is at most this positive fraction.
    /// Default: 1e-8. Also stop at relative residual ≤1e-24 (numerical precision).
    pub tolerance: f64,
    /// Deterministic initialization seed. Default: 0 chooses the largest-norm
    /// sample first; other values choose `seed % n_samples`. Subsequent samples
    /// maximize distance from the selected linear span. Try multiple starts.
    pub seed: u64,
    /// Optional initial component rows on the resulting analysis grid, shape
    /// components × selected points. Supplying standards is reference-informed
    /// initialization and must not be described as blind recovery.
    pub initial_spectra: Option<DMatrix<f64>>,
    /// Fixed composition rows. Empty by default. These constrain C during every
    /// iteration and are retained with the result for provenance.
    pub anchors: Vec<McrAnchor>,
}

impl Default for McrConfig {
    fn default() -> Self {
        Self {
            space: AnalysisSpace::Norm,
            range: None,
            components: 3,
            sum_to_one: true,
            nonnegative_spectra: false,
            max_iterations: 500,
            tolerance: 1e-8,
            seed: 0,
            initial_spectra: None,
            anchors: Vec::new(),
        }
    }
}

/// Why alternating iterations stopped. None establishes chemical uniqueness.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum McrTermination {
    /// Relative objective change or numerical residual threshold was reached.
    Converged,
    /// The requested number of complete iterations was used.
    IterationLimit,
    /// The progress callback requested cancellation; the best result is retained.
    Cancelled,
    /// A numerical objective increase prevented another accepted iteration.
    Stalled,
}

/// Owned MCR arrays and provenance; changing input spectra cannot alter them.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McrResult {
    /// First input's E0 in eV, retained for downstream component processing.
    /// Historical serialized results may lack it; supply an explicit E0 to
    /// `XASSpectrum::from_prepared` when converting such a result.
    #[serde(default)]
    pub e0: Option<f64>,
    /// Selected first-spectrum energy samples in eV, without extrapolation.
    pub x: DVector<f64>,
    /// Input names in coefficient-row order.
    pub labels: Vec<String>,
    /// Prepared dimensionless absorption, shape samples × points.
    pub data: DMatrix<f64>,
    /// Nonnegative coefficients, shape samples × components. These are not
    /// automatically mass fractions or uniquely identified concentrations.
    pub concentrations: DMatrix<f64>,
    /// Estimated dimensionless component spectra, shape components × points.
    pub spectra: DMatrix<f64>,
    /// Reconstruction C S, in the same shape and units as data.
    pub fit: DMatrix<f64>,
    /// Data minus fit, in the same shape and units as data.
    pub residual: DMatrix<f64>,
    /// Sum of squared residuals of the returned best complete iteration.
    pub sse: f64,
    /// SSE / sum(data²), dimensionless. Not a noise-weighted chi-square.
    pub relative_error: f64,
    /// SSE after each accepted complete iteration, in chronological order.
    pub objective_history: Vec<f64>,
    /// Complete iterations attempted, including an iteration rejected as stalled.
    pub iterations: usize,
    /// One-based accepted iteration corresponding to the returned arrays.
    pub best_iteration: usize,
    /// Explicit termination reason; an iteration limit is not convergence.
    pub termination: McrTermination,
    /// Exact settings, including any explicit anchors or supplied initialization.
    pub config: McrConfig,
    /// Selected input rows for automatic initialization; empty for supplied S.
    pub initial_samples: Vec<usize>,
}

impl McrResult {
    /// Copy one estimated component into an ordinary processable spectrum.
    /// The zero-based component index addresses rows of `spectra`. Preserves
    /// `Norm`/`Flat` input units and the recorded E0, with a unit edge step and
    /// no additional normalization by default. Explicitly supplying
    /// `set_normalization_method` allows a new pre/post-edge fit, including for
    /// flattened input. The returned spectrum owns its arrays;
    /// edits and EXAFS processing cannot change this MCR result.
    ///
    /// Call `calc_background()` and `fft()` on the returned spectrum. Only the
    /// resolved energy interval is available: a XANES-only result cannot provide
    /// the EXAFS range omitted from the MCR fit. An invalid index, missing E0,
    /// or invalid retained arrays returns an error. Retain this result alongside
    /// the component for full MCR settings, coefficients and source labels.
    pub fn component_spectrum(&self, index: usize) -> Result<XASSpectrum, crate::xafs::XAFSError> {
        use crate::xafs::errors::DataError;
        if index >= self.spectra.nrows() {
            return Err(DataError::IndexOutOfRange {
                index,
                length: self.spectra.nrows(),
            }
            .into());
        }
        let e0 = self.e0.ok_or_else(|| DataError::MissingData {
            field:
                "MCR E0; use XASSpectrum::from_prepared with an explicit E0 for historical results"
                    .into(),
        })?;
        let values: Vec<_> = self.spectra.row(index).iter().copied().collect();
        let mut spectrum =
            XASSpectrum::from_prepared(self.x.as_slice(), &values, self.config.space, e0)?;
        spectrum.set_name(format!("MCR component {}", index + 1));
        Ok(spectrum)
    }
}

fn invalid(reason: impl Into<String>) -> AnalysisError {
    AnalysisError::LinearAlgebra {
        reason: format!("MCR-ALS: {}", reason.into()),
    }
}

/// Resolve a collection of preprocessed spectra into a selected number of factors.
///
/// Uses normalized absorption by default, nonnegative coefficients and closure,
/// signed spectra and deterministic sample initialization. Missing normalization
/// is calculated on a temporary copy using each input's settings; existing results
/// are reused and originals remain unchanged.
/// Inputs are copied and interpolated linearly on the first spectrum's grid.
/// Invalid settings, nonfinite/unsorted arrays, insufficient rank, missing results,
/// incomplete coverage or failed constrained solves return contextual errors.
/// Use [`mcr_als_with_progress`] for cancellation and per-iteration progress.
pub fn mcr_als<S: Borrow<XASSpectrum>>(
    spectra: &[S],
    config: &McrConfig,
) -> Result<McrResult, AnalysisError> {
    mcr_als_with_progress(spectra, config, |_, _| true)
}

/// As [`mcr_als`], calling `progress(iteration, relative_error)` after each
/// accepted complete iteration. Return false to cancel and retain the best
/// result. The callback runs on the caller's thread; desktop callers should
/// invoke this function on a worker. No global state or input arrays are changed.
pub fn mcr_als_with_progress<S: Borrow<XASSpectrum>>(
    spectra: &[S],
    config: &McrConfig,
    mut progress: impl FnMut(usize, f64) -> bool,
) -> Result<McrResult, AnalysisError> {
    let k = config.components;
    if spectra.len() < 2
        || k == 0
        || k > spectra.len()
        || config.max_iterations == 0
        || !config.tolerance.is_finite()
        || config.tolerance <= 0.0
        || config.tolerance >= 1.0
    {
        return Err(invalid("require at least two samples, 1..=n components, positive iterations and 0 < tolerance < 1"));
    }
    if !matches!(config.space, AnalysisSpace::Norm | AnalysisSpace::Flat) {
        return Err(invalid("use normalized or flattened absorption"));
    }
    let first = AnalysisInput::with_label(spectra[0].borrow(), config.space, "sample 0".into())?;
    let bounds = first.bounds(config.space, config.range)?;
    let e0 = first.e0.expect("energy bounds validated the edge origin");
    let (grid, first_y) = first.select(bounds, k.max(3))?;
    let mut labels = Vec::with_capacity(spectra.len());
    let mut data = DMatrix::zeros(spectra.len(), grid.len());
    data.set_row(0, &first_y.transpose());
    for (i, spectrum) in spectra.iter().enumerate() {
        labels.push(spectrum_label(spectrum.borrow(), format!("sample {i}")));
        if i > 0 {
            let y =
                AnalysisInput::with_label(spectrum.borrow(), config.space, format!("sample {i}"))?
                    .interpolate(&grid, bounds)?;
            data.set_row(i, &y.transpose());
        }
    }
    let data_norm = data.norm_squared();
    if !data_norm.is_finite() || data_norm <= 0.0 {
        return Err(invalid("data must have finite nonzero squared norm"));
    }
    let mut anchors = vec![None; data.nrows()];
    for anchor in &config.anchors {
        if anchor.sample >= data.nrows()
            || anchor.weights.len() != k
            || anchor.weights.iter().any(|w| !w.is_finite() || *w < 0.0)
            || (config.sum_to_one && (anchor.weights.iter().sum::<f64>() - 1.0).abs() > 1e-10)
        {
            return Err(invalid("invalid known-composition anchor"));
        }
        if anchors[anchor.sample].is_some() {
            return Err(invalid("duplicate anchor sample"));
        }
        anchors[anchor.sample] = Some(DVector::from_vec(anchor.weights.clone()));
    }
    let (mut s, initial_samples) = if let Some(initial) = &config.initial_spectra {
        if initial.shape() != (k, grid.len())
            || initial.iter().any(|v| !v.is_finite())
            || (config.nonnegative_spectra && initial.iter().any(|v| *v < 0.0))
        {
            return Err(invalid(
                "initial spectra have invalid shape, values or nonnegativity",
            ));
        }
        (initial.clone(), Vec::new())
    } else {
        initialize(&data, k, config.seed, config.nonnegative_spectra)?
    };
    require_rank(&s.transpose(), k)?;
    let mut history = Vec::new();
    let mut best: Option<(DMatrix<f64>, DMatrix<f64>, f64, usize)> = None;
    let mut termination = McrTermination::IterationLimit;
    let mut iterations = 0;
    for iteration in 1..=config.max_iterations {
        iterations = iteration;
        let a = s.transpose();
        let mut c = DMatrix::zeros(data.nrows(), k);
        for (i, anchor) in anchors.iter().enumerate() {
            let weights = match anchor {
                Some(weights) => weights.clone(),
                None => constrained(&a, &data.row(i).transpose(), config.sum_to_one)?,
            };
            c.row_mut(i).copy_from(&weights.transpose());
        }
        require_rank(&c, k)?;
        s = if config.nonnegative_spectra {
            let mut next = DMatrix::zeros(k, data.ncols());
            for j in 0..data.ncols() {
                next.column_mut(j).copy_from(&constrained(
                    &c,
                    &data.column(j).into_owned(),
                    false,
                )?);
            }
            next
        } else {
            let svd = c.clone().svd(true, true);
            let cutoff = svd.singular_values[0] * 1e-12;
            svd.solve(&data, cutoff).map_err(invalid)?
        };
        let sse = (&data - &c * &s).norm_squared();
        if !sse.is_finite() || s.iter().any(|v| !v.is_finite()) {
            return Err(invalid("nonfinite ALS iterate"));
        }
        let previous = history.last().copied();
        if previous.is_some_and(|last| sse > last + 1e-12 * data_norm) {
            termination = McrTermination::Stalled;
            break;
        }
        history.push(sse);
        if best.as_ref().is_none_or(|(_, _, value, _)| sse < *value) {
            best = Some((c, s.clone(), sse, iteration));
        }
        if !progress(iteration, sse / data_norm) {
            termination = McrTermination::Cancelled;
            break;
        }
        if sse / data_norm <= 1e-24
            || previous.is_some_and(|last: f64| {
                (last - sse).abs() <= config.tolerance * last.max(data_norm * 1e-24)
            })
        {
            termination = McrTermination::Converged;
            break;
        }
    }
    let (concentrations, spectra, sse, best_iteration) =
        best.ok_or_else(|| invalid("no complete iterate"))?;
    let fit = &concentrations * &spectra;
    let residual = &data - &fit;
    Ok(McrResult {
        e0: Some(e0),
        x: grid,
        labels,
        data,
        concentrations,
        spectra,
        fit,
        residual,
        sse,
        relative_error: sse / data_norm,
        objective_history: history,
        iterations,
        best_iteration,
        termination,
        config: config.clone(),
        initial_samples,
    })
}

fn require_rank(a: &DMatrix<f64>, k: usize) -> Result<(), AnalysisError> {
    let values = a.clone().svd(false, false).singular_values;
    if values.len() < k || values[0] == 0.0 || values[k - 1] <= values[0] * 1e-12 {
        Err(invalid(
            "rank-deficient factors; reduce components or change initialization",
        ))
    } else {
        Ok(())
    }
}

fn initialize(
    data: &DMatrix<f64>,
    k: usize,
    seed: u64,
    nonnegative: bool,
) -> Result<(DMatrix<f64>, Vec<usize>), AnalysisError> {
    let mut residual = data.clone();
    let mut chosen = Vec::new();
    let mut s = DMatrix::zeros(k, data.ncols());
    for j in 0..k {
        let i = if j == 0 && seed != 0 {
            (seed % data.nrows() as u64) as usize
        } else {
            (0..data.nrows())
                .max_by(|&a, &b| {
                    residual
                        .row(a)
                        .norm_squared()
                        .total_cmp(&residual.row(b).norm_squared())
                })
                .unwrap()
        };
        let mut direction = residual.row(i).transpose();
        let norm = direction.norm();
        if norm <= data.norm() * 1e-12 {
            return Err(invalid("selected component count exceeds data rank"));
        }
        direction /= norm;
        for row in 0..data.nrows() {
            let projection = residual.row(row).transpose().dot(&direction);
            for col in 0..data.ncols() {
                residual[(row, col)] -= projection * direction[col];
            }
        }
        s.row_mut(j).copy_from(&data.row(i));
        chosen.push(i);
    }
    if nonnegative {
        s.apply(|v| *v = v.max(0.0));
    }
    Ok((s, chosen))
}

/// Verify feasibility and the bound/equality optimality conditions because the
/// legacy LCF solver returns weights without exposing its iteration-limit status.
fn constrained(
    a: &DMatrix<f64>,
    y: &DVector<f64>,
    closure: bool,
) -> Result<DVector<f64>, AnalysisError> {
    let k = a.ncols();
    let weights = bounded_lstsq(
        a,
        y,
        &vec![0.0; k],
        &vec![if closure { 1.0 } else { f64::INFINITY }; k],
        closure.then_some(1.0),
    )?;
    if weights.iter().any(|w| !w.is_finite() || *w < -1e-10)
        || (closure && (weights.sum() - 1.0).abs() > 1e-9)
    {
        return Err(invalid("constrained subproblem is infeasible"));
    }
    let gradient = a.transpose() * (a * &weights - y);
    let tol = 1e-7 * (1.0 + a.norm_squared() * weights.amax() + (a.transpose() * y).amax());
    let mut lower = f64::NEG_INFINITY;
    let mut upper = f64::INFINITY;
    for i in 0..k {
        if weights[i] <= 1e-9 {
            lower = lower.max(-gradient[i] - tol);
        } else if closure && weights[i] >= 1.0 - 1e-9 {
            upper = upper.min(-gradient[i] + tol);
        } else {
            lower = lower.max(-gradient[i] - tol);
            upper = upper.min(-gradient[i] + tol);
        }
    }
    if lower > upper || (!closure && (lower > 0.0 || upper < 0.0)) {
        return Err(invalid(
            "constrained subproblem did not satisfy optimality conditions",
        ));
    }
    Ok(weights)
}
