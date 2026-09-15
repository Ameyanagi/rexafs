//! Multi-spectrum analysis: linear combination fitting (LCF), principal
//! component analysis (PCA) and multivariate curve resolution (MCR-ALS).
//!
//! These tools operate on one of the [`AnalysisSpace`]s of a spectrum
//! (normalized μ, flattened μ, dμ/dE of normalized μ, or k-weighted χ(k))
//! over a fit range, with every spectrum interpolated linearly onto the grid
//! of a reference spectrum (the unknown for LCF, the first spectrum for PCA/MCR).
//! Missing processing stages run on copies using input settings; all inputs
//! must cover a finite increasing interval. Original spectra remain unchanged.
//!
//! * [`lcf()`] / [`lcf_combinatorial`] — Athena / Larch style linear combination
//!   fitting with bounded weights, optional sum-to-one constraint and optional
//!   per-standard energy shifts.
//! * [`pca_train`] / [`PcaModel`] — SVD based principal component analysis,
//!   Malinowski's indicator function and target transformation.
//! * [`mcr_als`] — constrained factorization into coefficients and spectra.

pub mod lcf;
pub mod mcr;
pub mod pca;

use std::borrow::{Borrow, Cow};

use nalgebra::DVector;
use serde::{Deserialize, Serialize};

use super::errors::AnalysisError;
use super::normalization::Normalization;
use super::tools::{dmude, interp_linear};
use super::xasspectrum::XASSpectrum;

pub use lcf::{
    lcf, lcf_batch, lcf_batch_with_progress, lcf_combinatorial, LcfComponent, LcfConfig, LcfResult,
};
pub use mcr::{mcr_als, mcr_als_with_progress, McrAnchor, McrConfig, McrResult, McrTermination};
pub use pca::{
    pca_train, PcaConfig, PcaCountBasis, PcaCountSuggestion, PcaFit, PcaModel,
    PcaReconstructionError,
};

/// Which array of a spectrum an analysis is performed on.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub enum AnalysisSpace {
    /// Normalized μ(E); missing normalization is calculated on a copy.
    #[default]
    Norm,
    /// Flattened normalized μ(E); missing normalization is calculated on a copy.
    Flat,
    /// Derivative dμ/dE of normalized μ(E); missing normalization is calculated on a copy.
    Deriv,
    /// k-weighted χ(k)·k^kweight; missing background is calculated on a copy.
    /// The real exponent is used directly, unlike the processing FFT's floored
    /// nonnegative integer weight. For dimensionless χ, y has units Å⁻ᵏʷᵉⁱᵍʰᵗ.
    Chi {
        /// Real exponent applied directly to k in Å⁻¹; choose a finite value.
        /// Noninteger or negative values require care at k = 0.
        kweight: f64,
    },
}

/// Alias used by the LCF API.
pub type LcfSpace = AnalysisSpace;

impl AnalysisSpace {
    /// Whether the x axis of this space is k (Å⁻¹) rather than energy (eV).
    pub fn is_k_space(&self) -> bool {
        matches!(self, Self::Chi { .. })
    }

    /// Athena's default fit range: −20…+30 eV relative to E₀ for energy
    /// spaces, 3…12 Å⁻¹ for χ(k).
    pub fn default_range(&self) -> (f64, f64) {
        if self.is_k_space() {
            (3.0, 12.0)
        } else {
            (-20.0, 30.0)
        }
    }

    /// Owned, validated `(x, y)` arrays in this representation.
    ///
    /// Reuses existing results. If missing, runs normalization (Norm, Flat,
    /// Deriv) or background subtraction (Chi) on a temporary copy using the
    /// spectrum's settings, or stage defaults when unset. Inputs are unchanged.
    /// Energy axes use eV, k axes Å⁻¹; norm/flat absorption is dimensionless
    /// and its derivative has units eV⁻¹. Nonfinite values, unequal lengths,
    /// nonincreasing axes and failed preparation return contextual errors.
    /// A prepared Norm-only component needs explicit normalization settings
    /// before Flat can be calculated; its stored representation is never relabeled.
    pub fn arrays(
        &self,
        spectrum: &XASSpectrum,
    ) -> Result<(DVector<f64>, DVector<f64>), AnalysisError> {
        let input = AnalysisInput::new(spectrum, *self)?;
        Ok((input.x, input.y))
    }

    fn prepare<'a>(
        &self,
        spectrum: &'a XASSpectrum,
    ) -> Result<Cow<'a, XASSpectrum>, AnalysisError> {
        let ready = match self {
            Self::Norm | Self::Deriv => spectrum
                .normalization
                .as_ref()
                .and_then(|n| n.get_norm())
                .is_some(),
            Self::Flat => spectrum
                .normalization
                .as_ref()
                .and_then(|n| n.get_flat())
                .is_some(),
            Self::Chi { .. } => spectrum.k().is_some() && spectrum.chi().is_some(),
        };
        if ready {
            return Ok(Cow::Borrowed(spectrum));
        }
        if *self == Self::Flat
            && spectrum.preserves_prepared_values()
            && spectrum.prepared_space() == Some(Self::Norm)
        {
            return Err(AnalysisError::InvalidInput {
                spectrum: spectrum_label(spectrum, "unnamed spectrum".into()),
                reason: "a prepared Norm component has no Flat array; explicitly set a normalization method to refit it, or select Norm".into(),
            });
        }
        let mut prepared = spectrum.clone();
        let result = if self.is_k_space() {
            prepared.calc_background()
        } else {
            prepared.normalize()
        };
        result.map_err(|source| AnalysisError::Preparation {
            spectrum: spectrum_label(spectrum, "unnamed spectrum".into()),
            source: Box::new(source),
        })?;
        Ok(Cow::Owned(prepared))
    }

    fn stored_arrays(
        &self,
        spectrum: &XASSpectrum,
    ) -> Result<(DVector<f64>, DVector<f64>), AnalysisError> {
        let missing = |field: &str| AnalysisError::MissingArray {
            field: field.to_string(),
        };
        match self {
            Self::Norm | Self::Flat | Self::Deriv => {
                let energy = spectrum.energy.clone().ok_or_else(|| missing("energy"))?;
                let y = match self {
                    Self::Flat => spectrum
                        .flat()
                        .ok_or_else(|| missing("flat (run normalize() first)"))?,
                    _ => spectrum
                        .norm()
                        .ok_or_else(|| missing("norm (run normalize() first)"))?,
                };
                if energy.len() != y.len() {
                    return Err(super::errors::DataError::LengthMismatch {
                        energy_len: energy.len(),
                        mu_len: y.len(),
                    }
                    .into());
                }
                if energy.len() < 2 {
                    return Err(super::errors::DataError::InsufficientData {
                        min: 2,
                        actual: energy.len(),
                    }
                    .into());
                }
                let y = if matches!(self, Self::Deriv) {
                    dmude(&energy, &y)
                } else {
                    y
                };
                Ok((energy, y))
            }
            Self::Chi { kweight } => {
                let k = spectrum
                    .k()
                    .ok_or_else(|| missing("k (run calc_background() first)"))?;
                let chi = spectrum
                    .chi()
                    .ok_or_else(|| missing("chi (run calc_background() first)"))?;
                if k.len() != chi.len() {
                    return Err(super::errors::DataError::LengthMismatch {
                        energy_len: k.len(),
                        mu_len: chi.len(),
                    }
                    .into());
                }
                let y = DVector::from_fn(k.len(), |i, _| chi[i] * k[i].powf(*kweight));
                Ok((DVector::from_column_slice(k), y))
            }
        }
    }
}

/// E₀ of a spectrum: the explicit `e0`, else the one found during normalization.
pub(crate) fn spectrum_e0(spectrum: &XASSpectrum) -> Option<f64> {
    spectrum
        .e0
        .or_else(|| spectrum.normalization.as_ref().and_then(|n| n.get_e0()))
}

/// Name of a spectrum, or `fallback` if it has none.
pub(crate) fn spectrum_label(spectrum: &XASSpectrum, fallback: String) -> String {
    spectrum.name.clone().unwrap_or(fallback)
}

/// Prepared arrays and the edge origin used by every analysis frontend.
pub(crate) struct AnalysisInput {
    pub x: DVector<f64>,
    pub y: DVector<f64>,
    pub e0: Option<f64>,
    pub label: String,
}

impl AnalysisInput {
    pub fn new(spectrum: &XASSpectrum, space: AnalysisSpace) -> Result<Self, AnalysisError> {
        Self::with_label(spectrum, space, "unnamed spectrum".into())
    }

    pub fn with_label(
        spectrum: &XASSpectrum,
        space: AnalysisSpace,
        fallback: String,
    ) -> Result<Self, AnalysisError> {
        let label = spectrum_label(spectrum, fallback);
        let prepared = space.prepare(spectrum).map_err(|error| match error {
            AnalysisError::Preparation { source, .. } => AnalysisError::Preparation {
                spectrum: label.clone(),
                source,
            },
            AnalysisError::InvalidInput { reason, .. } => AnalysisError::InvalidInput {
                spectrum: label.clone(),
                reason,
            },
            other => other,
        })?;
        let (x, y) = space.stored_arrays(&prepared)?;
        if x.len() < 2
            || x.iter().chain(y.iter()).any(|v| !v.is_finite())
            || x.as_slice().windows(2).any(|w| w[0] >= w[1])
        {
            return Err(AnalysisError::InvalidInput {
                spectrum: label,
                reason: "need at least two finite samples on a strictly increasing axis".into(),
            });
        }
        Ok(Self {
            x,
            y,
            e0: spectrum_e0(&prepared),
            label,
        })
    }

    pub fn bounds(
        &self,
        space: AnalysisSpace,
        range: Option<(f64, f64)>,
    ) -> Result<(f64, f64), AnalysisError> {
        let (lo, hi) = range.unwrap_or_else(|| space.default_range());
        validate_range(lo, hi)?;
        let origin = if space.is_k_space() {
            0.0
        } else {
            self.e0
                .filter(|v| v.is_finite())
                .ok_or_else(|| AnalysisError::InvalidInput {
                    spectrum: self.label.clone(),
                    reason: "need a finite E0 to resolve energy offsets".into(),
                })?
        };
        let bounds = (origin + lo, origin + hi);
        validate_range(bounds.0, bounds.1)?;
        Ok(bounds)
    }

    pub fn select(
        &self,
        bounds: (f64, f64),
        min: usize,
    ) -> Result<(DVector<f64>, DVector<f64>), AnalysisError> {
        self.cover(bounds)?;
        let (lo, hi) = bounds;
        let idx: Vec<_> = (0..self.x.len())
            .filter(|&i| self.x[i] >= lo && self.x[i] <= hi)
            .collect();
        if idx.len() < min {
            return Err(AnalysisError::EmptyRange {
                lo,
                hi,
                n_points: idx.len(),
                min,
            });
        }
        Ok((
            DVector::from_iterator(idx.len(), idx.iter().map(|&i| self.x[i])),
            DVector::from_iterator(idx.len(), idx.iter().map(|&i| self.y[i])),
        ))
    }

    pub fn cover(&self, (lo, hi): (f64, f64)) -> Result<(), AnalysisError> {
        let available_lo = self.x[0];
        let available_hi = self.x[self.x.len() - 1];
        if available_lo > lo || available_hi < hi {
            return Err(AnalysisError::IncompleteCoverage {
                spectrum: self.label.clone(),
                lo,
                hi,
                available_lo,
                available_hi,
            });
        }
        Ok(())
    }

    pub fn interpolate(
        &self,
        grid: &DVector<f64>,
        bounds: (f64, f64),
    ) -> Result<DVector<f64>, AnalysisError> {
        self.cover(bounds)?;
        Ok(interp_linear(grid, &self.x, &self.y)?)
    }
}

/// Require finite increasing bounds in the selected axis units.
/// Energy-space callers pass offsets from E₀; k-space callers pass Å⁻¹.
/// No bounds are reordered and no automatic interval replaces invalid input.
pub fn validate_range(lo: f64, hi: f64) -> Result<(), AnalysisError> {
    if !lo.is_finite() || !hi.is_finite() || lo >= hi {
        return Err(AnalysisError::InvalidRange { lo, hi });
    }
    Ok(())
}

/// Interpolate only inside the measured coverage of the prepared input.
pub(crate) fn on_grid(
    spectrum: &XASSpectrum,
    space: AnalysisSpace,
    grid: &DVector<f64>,
) -> Result<DVector<f64>, AnalysisError> {
    let input = AnalysisInput::new(spectrum, space)?;
    input.interpolate(grid, (grid[0], grid[grid.len() - 1]))
}

/// Relative squared discrepancy Σ(data − fit)² / Σ data².
///
/// Pass equal-length arrays in the same units. Returns NaN when the data norm
/// is zero. This differs from the EXAFS path fitter's zero-denominator guard.
/// No measurement-noise weighting or statistical probability is implied.
pub fn r_factor(data: &DVector<f64>, fit: &DVector<f64>) -> f64 {
    let num: f64 = data
        .iter()
        .zip(fit.iter())
        .map(|(d, f)| (d - f).powi(2))
        .sum();
    let den: f64 = data.iter().map(|d| d * d).sum();
    if den > 0.0 {
        num / den
    } else {
        f64::NAN
    }
}

/// Collect `&XASSpectrum` references from any slice of owned or borrowed spectra.
pub(crate) fn as_refs<S: Borrow<XASSpectrum>>(spectra: &[S]) -> Vec<&XASSpectrum> {
    spectra.iter().map(|s| s.borrow()).collect()
}
