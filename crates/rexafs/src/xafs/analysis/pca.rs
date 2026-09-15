//! Principal component analysis of a set of spectra (Athena's PCA tool,
//! Larch's `pca_train` / `pca_fit`).
//!
//! # Algorithm
//!
//! The spectra are taken in the chosen [`AnalysisSpace`], interpolated onto
//! the first spectrum's grid restricted to the fit range, and stacked into a
//! data matrix `D` (n_spectra × n_points). Optionally the mean spectrum is
//! subtracted (`PcaConfig::center`; **off** by default). Without centering,
//! ideal linear mixtures of *m* independent species have rank at most *m*;
//! noise and inconsistent preprocessing can increase the numerical rank.
//! The thin singular value decomposition (SVD) `D = U Σ Vᵀ` gives orthonormal
//! components (rows of `Vᵀ`), eigenvalues `λᵢ = σᵢ² / n_spectra` of
//! `DᵀD / n_spectra`, fractions `σᵢ² / Σσ²`, and scores `U Σ`.
//! The eigenvalues describe second moments when uncentered, and covariance
//! with a population denominator `n_spectra` when centered; they are not
//! unbiased sample variances with denominator `n_spectra - 1`.
//!
//! The Malinowski indicator function for `k` retained components is
//!
//! ```text
//! IND(k) = sqrt( Σ_{j>k} λⱼ / (n_points · (n_spectra − k)) ) / (n_spectra − k)²
//! ```
//!
//! Here eigenvalues are indexed from one, `k` is the retained count, and
//! `n_points` and `n_spectra` are matrix dimensions. The indicator carries
//! spectral units because eigenvalues have squared spectral units. Its
//! minimum is a heuristic rank suggestion, not a chemical-species count.
//! The displayed equation specifies this implementation's eigenvalue scaling;
//! see [Malinowski (1977)](https://doi.org/10.1021/ac50012a027) for the
//! indicator-function method. Missing tail eigenvalues are treated as zero
//! when there are more spectra than grid points, which can make this
//! suggestion uninformative.
//!
//! A target transform ([`PcaModel::target_transform`]) projects a spectrum
//! onto the first `n` components (weights = `C · (y − mean)` since the
//! components are orthonormal) and reports the reconstruction quality.
//! Inputs are borrowed, results own their arrays, and missing processing stages
//! run on temporary copies using the input settings. See [Larch's PCA guide](https://xraypy.github.io/xraylarch/xafs_xanes.html#principal-component-analysis)
//! for a centered comparison workflow; rexafs defaults to no centering.

use std::borrow::Borrow;

use nalgebra::{DMatrix, DVector, SVD};
use serde::{Deserialize, Serialize};

use super::{as_refs, on_grid, r_factor, spectrum_label, AnalysisInput, AnalysisSpace};
use crate::xafs::errors::AnalysisError;
use crate::xafs::xasspectrum::XASSpectrum;

/// Configuration of a PCA training.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PcaConfig {
    /// Array to analyze; defaults to normalized absorption, [`AnalysisSpace::Norm`].
    pub space: AnalysisSpace,
    /// Range: relative to the first spectrum's E₀ for energy spaces,
    /// absolute k for `Chi`. `None` selects −20 to +30 eV or 3 to 12 Å⁻¹.
    /// Bounds must be finite, increasing and fully covered by every input.
    pub range: Option<(f64, f64)>,
    /// Subtract the mean spectrum before the SVD (default `false`).
    pub center: bool,
}

impl Default for PcaConfig {
    fn default() -> Self {
        Self {
            space: AnalysisSpace::Norm,
            range: None,
            center: false,
        }
    }
}

/// Trained PCA model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PcaModel {
    /// Spectral representation used for training and target interpolation.
    pub space: AnalysisSpace,
    /// Whether `mean` was subtracted before the decomposition.
    pub centered: bool,
    /// Common grid (energy or k).
    pub x: DVector<f64>,
    /// Labels of the training spectra.
    pub labels: Vec<String>,
    /// Training data on the grid, one row per spectrum (n_spectra × n_points).
    pub data: DMatrix<f64>,
    /// Mean spectrum (zeros when not centred).
    pub mean: DVector<f64>,
    /// Orthonormal components, one row per component (n_components × n_points),
    /// ordered by decreasing eigenvalue.
    pub components: DMatrix<f64>,
    /// Eigenvalues of `DᵀD / n_spectra`, in squared spectral units.
    /// These are second moments when uncentered; see the module's convention.
    pub eigenvalues: Vec<f64>,
    /// Fraction `σᵢ² / Σσ²` of centered variation or uncentered squared signal.
    /// All fractions are zero for an all-zero training matrix.
    pub variance_explained: Vec<f64>,
    /// Running sum of `variance_explained`.
    pub cumulative_variance: Vec<f64>,
    /// Malinowski indicator function; `ind[k]` is IND for `k` components,
    /// `k = 0 … n_spectra − 1`.
    pub ind: Vec<f64>,
    /// Scores (n_spectra × n_components): the training data expressed in the
    /// component basis, `data − mean = scores · components`.
    pub scores: DMatrix<f64>,
}

/// Evidence used for a component-count suggestion; neither variant estimates
/// chemical species or experimental noise automatically.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PcaCountBasis {
    /// Singular values above the rexafs floating-point tolerance.
    NumericalRank,
    /// A positive interior minimum of the Malinowski indicator.
    IndicatorMinimum,
}

/// Suggested retained directions and the evidence behind the suggestion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PcaCountSuggestion {
    /// Number of varying directions (the mean is separate when centered).
    pub count: usize,
    /// Diagnostic used; inspect the curves before choosing a chemical model.
    pub basis: PcaCountBasis,
}

/// One point of the training reconstruction-error curve.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PcaReconstructionError {
    /// Retained directions; zero retains only the model's mean.
    pub components: usize,
    /// Sum of squared residuals over all rows and grid points, in squared signal units.
    pub sse: f64,
    /// SSE divided by sum of squared original training values; NaN for zero data.
    pub relative_error: f64,
}

/// Reconstruction of a spectrum from a subset of components.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PcaFit {
    /// Number of retained components; zero reconstructs only the stored mean.
    pub n_components: usize,
    /// Copied model grid: energy in eV or k in Å⁻¹.
    pub x: DVector<f64>,
    /// Input spectrum on the model grid.
    pub data: DVector<f64>,
    /// `mean + Σ wᵢ Cᵢ`.
    pub fit: DVector<f64>,
    /// `data − fit`.
    pub residual: DVector<f64>,
    /// Projection weights on the retained components.
    pub weights: Vec<f64>,
    /// Σ residual².
    pub chi_square: f64,
    /// `chi_square / max(n_points - n_components, 1)`, without a noise model.
    pub reduced_chi_square: f64,
    /// Σ residual² / Σ data²; NaN when the input has zero squared norm.
    pub r_factor: f64,
}

/// Train an owned PCA model from at least two spectra.
///
/// Linear interpolation uses the first spectrum's selected grid. Every input
/// must cover the entire requested interval; no extrapolation is performed.
/// Missing processing arrays are calculated on copies using input settings.
/// Returns an error for failed preparation, fewer than two selected points,
/// invalid interpolation inputs, or failed singular-value decomposition.
pub fn pca_train<S: Borrow<XASSpectrum>>(
    spectra: &[S],
    cfg: &PcaConfig,
) -> Result<PcaModel, AnalysisError> {
    let refs = as_refs(spectra);
    let n = refs.len();
    if n < 2 {
        return Err(AnalysisError::InsufficientSpectra { min: 2, actual: n });
    }
    let first = AnalysisInput::with_label(refs[0], cfg.space, "spectrum0".into())?;
    let bounds = first.bounds(cfg.space, cfg.range)?;
    let (x, y0) = first.select(bounds, 2)?;
    let p = x.len();

    let mut data = DMatrix::zeros(n, p);
    data.set_row(0, &y0.transpose());
    for (i, s) in refs.iter().enumerate().skip(1) {
        let y = AnalysisInput::with_label(s, cfg.space, format!("spectrum{i}"))?
            .interpolate(&x, bounds)?;
        data.set_row(i, &y.transpose());
    }
    let labels = refs
        .iter()
        .enumerate()
        .map(|(i, s)| spectrum_label(s, format!("spectrum{i}")))
        .collect();

    let mean = if cfg.center {
        DVector::from_fn(p, |j, _| data.column(j).sum() / n as f64)
    } else {
        DVector::zeros(p)
    };
    let mut centered = data.clone();
    if cfg.center {
        for i in 0..n {
            let row = centered.row(i) - mean.transpose();
            centered.set_row(i, &row);
        }
    }

    let svd = SVD::new(centered.clone(), true, true);
    let (u, v_t) = match (svd.u, svd.v_t) {
        (Some(u), Some(v_t)) => (u, v_t),
        _ => {
            return Err(AnalysisError::LinearAlgebra {
                reason: "SVD did not return singular vectors".to_string(),
            })
        }
    };
    let sigma = svd.singular_values;
    let n_comp = sigma.len();

    let sigma_sq: Vec<f64> = sigma.iter().map(|s| s * s).collect();
    let total: f64 = sigma_sq.iter().sum();
    let eigenvalues: Vec<f64> = sigma_sq.iter().map(|s| s / n as f64).collect();
    let variance_explained: Vec<f64> = sigma_sq
        .iter()
        .map(|s| if total > 0.0 { s / total } else { 0.0 })
        .collect();
    let cumulative_variance: Vec<f64> = variance_explained
        .iter()
        .scan(0.0, |acc, v| {
            *acc += v;
            Some(*acc)
        })
        .collect();

    // Scores = U Σ (n × n_comp).
    let mut scores = u.columns(0, n_comp).into_owned();
    for j in 0..n_comp {
        let mut col = scores.column_mut(j);
        col *= sigma[j];
    }

    let ind = malinowski_ind(&eigenvalues, n, p);

    Ok(PcaModel {
        space: cfg.space,
        centered: cfg.center,
        x,
        labels,
        data,
        mean,
        components: v_t.rows(0, n_comp).into_owned(),
        eigenvalues,
        variance_explained,
        cumulative_variance,
        ind,
        scores,
    })
}

/// Malinowski's IND for `k = 0 … n_spectra − 1` retained components.
fn malinowski_ind(eigenvalues: &[f64], n_spectra: usize, n_points: usize) -> Vec<f64> {
    let c = n_spectra;
    let r = n_points as f64;
    (0..c)
        .map(|k| {
            let tail: f64 = eigenvalues.iter().skip(k).sum();
            let remaining = (c - k) as f64;
            (tail / (r * remaining)).sqrt() / (remaining * remaining)
        })
        .collect()
}

impl PcaModel {
    /// Number of spectra used to train the model.
    pub fn n_spectra(&self) -> usize {
        self.data.nrows()
    }

    /// Number of available SVD components, at most min(spectra, grid points).
    pub fn n_components(&self) -> usize {
        self.components.nrows()
    }

    /// Numerical rank using σ > ε max(n, p) ‖D‖F, where ε is f64 precision,
    /// n/p are training dimensions and D is the original (uncentered) matrix.
    /// This rexafs tolerance suppresses centering roundoff; it is not a measured
    /// noise threshold and should not be interpreted as a chemical-species count.
    pub fn numerical_rank(&self) -> usize {
        let tolerance =
            f64::EPSILON * self.data.nrows().max(self.data.ncols()) as f64 * self.data.norm();
        self.eigenvalues
            .iter()
            .filter(|&&v| {
                v.is_finite() && (v.max(0.0) * self.n_spectra() as f64).sqrt() > tolerance
            })
            .count()
    }

    /// Suggest a retained count with its basis, consistently across frontends.
    /// Exact low-rank data use numerical rank; otherwise a positive interior IND
    /// minimum is a heuristic. Zero data or a boundary minimum return `None`.
    /// Centered counts describe varying directions in addition to the mean.
    pub fn component_count_suggestion(&self) -> Option<PcaCountSuggestion> {
        let rank = self.numerical_rank();
        let possible = self
            .n_components()
            .min(self.n_spectra().saturating_sub(usize::from(self.centered)));
        if rank == 0 {
            return None;
        }
        if rank < possible {
            return Some(PcaCountSuggestion {
                count: rank,
                basis: PcaCountBasis::NumericalRank,
            });
        }
        let limit = possible.min(self.ind.len().saturating_sub(1));
        let best = (1..limit)
            .filter(|&k| self.ind[k].is_finite() && self.ind[k] > 0.0)
            .min_by(|&a, &b| self.ind[a].total_cmp(&self.ind[b]))?;
        self.ind
            .get(limit)
            .is_some_and(|&last| self.ind[best] < last && self.ind[best] < self.ind[0])
            .then_some(PcaCountSuggestion {
                count: best,
                basis: PcaCountBasis::IndicatorMinimum,
            })
    }

    /// Training error versus retained count from zero through all SVD directions.
    /// SSE is n times the sum of discarded eigenvalues, equivalent to the sum
    /// of squared reconstruction residuals up to SVD roundoff. The denominator
    /// for relative error is the original data norm even for a centered model.
    /// No logarithm or display floor is applied; plotting scale is a frontend choice.
    pub fn reconstruction_errors(&self) -> Vec<PcaReconstructionError> {
        let norm = self.data.norm_squared();
        (0..=self.n_components())
            .map(|components| {
                let sse =
                    self.eigenvalues.iter().skip(components).sum::<f64>() * self.n_spectra() as f64;
                PcaReconstructionError {
                    components,
                    sse,
                    relative_error: if norm > 0.0 { sse / norm } else { f64::NAN },
                }
            })
            .collect()
    }

    /// Number of significant components suggested by the minimum of the
    /// Malinowski indicator function (at least 1). Historical compatibility helper;
    /// new callers should use [`Self::component_count_suggestion`] for boundary
    /// and roundoff checks and an explicit `None` when no suggestion is supported.
    pub fn suggested_components_ind(&self) -> usize {
        let mut best = 1;
        let mut best_val = f64::INFINITY;
        for (k, &v) in self.ind.iter().enumerate().skip(1) {
            if v.is_finite() && v < best_val {
                best_val = v;
                best = k;
            }
        }
        best.min(self.n_components().max(1))
    }

    /// Smallest number of components whose cumulative explained variance
    /// reaches `threshold` (e.g. `0.999`).
    /// Use a finite fraction from 0 to 1; the argument is not validated.
    /// Returns all available components if the threshold is never reached.
    pub fn suggested_components_variance(&self, threshold: f64) -> usize {
        self.cumulative_variance
            .iter()
            .position(|&v| v >= threshold)
            .map(|i| i + 1)
            .unwrap_or(self.n_components())
    }

    /// Reconstruct a spectrum given on the model grid from its first
    /// `n_components` components, returning owned arrays without changing the model.
    /// The input length must equal the model grid length; no interpolation occurs.
    /// Zero components returns the mean (zero for an uncentered model).
    pub fn reconstruct(
        &self,
        y: &DVector<f64>,
        n_components: usize,
    ) -> Result<PcaFit, AnalysisError> {
        if n_components > self.n_components() {
            return Err(AnalysisError::TooManyComponents {
                requested: n_components,
                available: self.n_components(),
            });
        }
        if y.len() != self.x.len() {
            return Err(AnalysisError::LinearAlgebra {
                reason: format!(
                    "spectrum has {} points, model grid has {}",
                    y.len(),
                    self.x.len()
                ),
            });
        }
        let comps = self.components.rows(0, n_components);
        let centered = y - &self.mean;
        let w = comps * &centered;
        let fit = &self.mean + comps.transpose() * &w;
        let residual = y - &fit;
        let chi_square: f64 = residual.iter().map(|r| r * r).sum();
        let dof = y.len().saturating_sub(n_components).max(1) as f64;
        Ok(PcaFit {
            n_components,
            x: self.x.clone(),
            r_factor: r_factor(y, &fit),
            data: y.clone(),
            fit,
            residual,
            weights: w.iter().copied().collect(),
            chi_square,
            reduced_chi_square: chi_square / dof,
        })
    }

    /// Reconstruct training spectrum `index` from its first `n_components` components.
    pub fn reconstruct_training(
        &self,
        index: usize,
        n_components: usize,
    ) -> Result<PcaFit, AnalysisError> {
        if index >= self.n_spectra() {
            return Err(crate::xafs::errors::DataError::IndexOutOfRange {
                index,
                length: self.n_spectra(),
            }
            .into());
        }
        let y = self.data.row(index).transpose();
        self.reconstruct(&y, n_components)
    }

    /// Target transform: project `spectrum` (interpolated onto the model grid)
    /// onto the first `n_components` components and report the fit quality.
    pub fn target_transform(
        &self,
        spectrum: &XASSpectrum,
        n_components: usize,
    ) -> Result<PcaFit, AnalysisError> {
        let y = on_grid(spectrum, self.space, &self.x)?;
        self.reconstruct(&y, n_components)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ind_minimum_at_true_rank() {
        // Two large eigenvalues, three noise-level ones.
        let eig = [10.0, 1.0, 1e-4, 1e-4, 1e-4];
        let ind = malinowski_ind(&eig, 5, 200);
        let argmin = ind
            .iter()
            .enumerate()
            .skip(1)
            .min_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .unwrap()
            .0;
        assert_eq!(argmin, 2);
    }
}
