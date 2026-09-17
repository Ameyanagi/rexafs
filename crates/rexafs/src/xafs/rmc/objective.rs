use super::*;
use crate::xafs::fitting::{transform::apply_kweight_transform, FeffFitTransform};
use nalgebra::DVector;
use num_complex::Complex64;

/// Comparison of noise-scaled, k-weighted residuals. Fourier transforms use the
/// existing rexafs EXAFS conventions; these scores are numerical objectives,
/// not likelihoods with independent transformed observations. See `doc/rmc.md`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub enum Objective {
    /// Mean squared residual on the measured k grid (default).
    #[default]
    K,
    /// Mean squared complex residual in the selected R interval, in Å.
    /// Transform kweight/kweights/fitspace are ignored: the dataset supplies kweight.
    R(FeffFitTransform),
    /// Mean squared real back-transformed residual in the selected q interval.
    /// Transform kweight/kweights/fitspace are ignored: the dataset supplies kweight.
    Q(FeffFitTransform),
    /// Mean squared complex, Gaussian-windowed Fourier residual on a k–R grid.
    Wavelet(WaveletSettings),
    /// Masked Morlet or Gaussian STFT map with direct/FFT evaluation. The dataset's
    /// noise scale and k weight apply once before this transform.
    LocalSpectrum(LocalSpectrumSettings),
}

pub(super) enum PreparedObjective {
    K,
    Fourier {
        settings: FeffFitTransform,
        grid: DVector<f64>,
        mapping: Vec<Option<(usize, f64)>>,
        q: bool,
    },
    Wavelet(Vec<Vec<Complex64>>),
    LocalSpectrum(LocalSpectrumTransform),
}

impl Objective {
    /// Score a finite model χ on this dataset's measured grid. Includes dataset
    /// weight, noise scaling and kweight exactly once. Does not calculate scattering.
    pub fn score(&self, dataset: &ExafsDataset, model: &[f64]) -> Result<f64, RmcError> {
        require(
            dataset.k.len() >= 2
                && dataset.k.len() == dataset.chi.len()
                && dataset.k.len() == dataset.sigma.len(),
            "invalid objective dataset arrays",
        )?;
        require(
            dataset.k.iter().all(|v| v.is_finite() && *v >= 0.)
                && dataset.k.windows(2).all(|w| w[1] > w[0])
                && dataset.chi.iter().all(|v| v.is_finite())
                && dataset.sigma.iter().all(|v| v.is_finite() && *v > 0.)
                && dataset.weight.is_finite()
                && dataset.weight > 0.
                && dataset.kweight <= 3,
            "invalid objective dataset values",
        )?;
        self.prepare(&dataset.k)?.score(dataset, model)
    }

    pub(super) fn prepare(&self, k: &[f64]) -> Result<PreparedObjective, RmcError> {
        match self {
            Self::K => Ok(PreparedObjective::K),
            Self::LocalSpectrum(settings) => Ok(PreparedObjective::LocalSpectrum(
                LocalSpectrumTransform::new(k, settings)?,
            )),
            Self::R(t) | Self::Q(t) => {
                crate::xafs::fitting::transform::validate_transform(t)
                    .map_err(|e| RmcError::Invalid(e.to_string()))?;
                let step = t.kstep.unwrap_or(k[1] - k[0]);
                require(
                    [t.kmin, t.kmax, t.dk, t.rmin, t.rmax, t.dr, step]
                        .iter()
                        .all(|v| v.is_finite())
                        && t.dk2.is_none_or(|v| v.is_finite() && v >= 0.)
                        && t.dr2.is_none_or(|v| v.is_finite() && v >= 0.)
                        && t.dk >= 0.
                        && t.dr >= 0.
                        && step > 0.
                        && t.kmin >= k[0]
                        && t.kmax <= k[k.len() - 1]
                        && t.rmin >= 0.
                        && t.rmax <= 10.
                        && t.rmax < std::f64::consts::PI / (2. * step)
                        && t.nfft <= 65536
                        && t.nfft.is_power_of_two(),
                    "invalid Fourier support, window, FFT size or sampling",
                )?;
                let extent = k[k.len() - 1] / step;
                require(
                    extent.is_finite() && extent < (t.nfft / 2) as f64,
                    "Fourier grid exceeds nfft/2",
                )?;
                let count = extent.floor() as usize + 1;
                require(
                    count >= 2 && count <= t.nfft / 2,
                    "Fourier k grid needs 2..=nfft/2 points; increase nfft or kstep",
                )?;
                let grid = DVector::from_iterator(count, (0..count).map(|i| i as f64 * step));
                let mapping = grid
                    .iter()
                    .map(|&x| {
                        if x < k[0] || x > k[k.len() - 1] {
                            None
                        } else {
                            let h = k.partition_point(|&v| v < x).clamp(1, k.len() - 1);
                            Some((h - 1, (x - k[h - 1]) / (k[h] - k[h - 1])))
                        }
                    })
                    .collect();
                let mut settings = t.clone();
                settings.kstep = Some(step);
                // Validate that discrete masks really contain points before starting a run.
                apply_kweight_transform(&grid, &DVector::zeros(count), &settings, 0.)
                    .map_err(|e| RmcError::Invalid(e.to_string()))?;
                Ok(PreparedObjective::Fourier {
                    settings,
                    grid,
                    mapping,
                    q: matches!(self, Self::Q(_)),
                })
            }
            Self::Wavelet(w) => {
                require(
                    w.omega0.is_finite()
                        && w.omega0 > 0.
                        && !w.k_centers.is_empty()
                        && !w.r.is_empty()
                        && w.k_centers
                            .iter()
                            .all(|v| v.is_finite() && *v >= k[0] && *v <= k[k.len() - 1])
                        && w.k_centers.windows(2).all(|v| v[1] > v[0])
                        && w.r.iter().all(|v| v.is_finite() && *v > 0.)
                        && w.r.windows(2).all(|v| v[1] > v[0]),
                    "invalid wavelet grid or width",
                )?;
                require(
                    w.k_centers
                        .len()
                        .saturating_mul(w.r.len())
                        .saturating_mul(k.len())
                        <= 4_000_000,
                    "wavelet kernel exceeds four million coefficients",
                )?;
                let quadrature: Vec<_> = (0..k.len())
                    .map(|i| {
                        if i == 0 {
                            (k[1] - k[0]) / 2.
                        } else if i + 1 == k.len() {
                            (k[i] - k[i - 1]) / 2.
                        } else {
                            (k[i + 1] - k[i - 1]) / 2.
                        }
                    })
                    .collect();
                let mut kernels = Vec::new();
                for &center in &w.k_centers {
                    for &r in &w.r {
                        let width = w.omega0 / (2. * r);
                        let correction = (-0.5 * w.omega0 * w.omega0).exp();
                        let mut row: Vec<_> = k
                            .iter()
                            .zip(&quadrature)
                            .map(|(&x, &dk)| {
                                let t = x - center;
                                (Complex64::from_polar(1., 2. * r * t) - correction)
                                    * (-0.5 * (t / width).powi(2)).exp()
                                    * dk
                            })
                            .collect();
                        let norm = row.iter().map(|v| v.norm_sqr()).sum::<f64>().sqrt();
                        require(
                            norm.is_finite() && norm > 1e-100,
                            "degenerate wavelet kernel",
                        )?;
                        for v in &mut row {
                            *v /= norm;
                        }
                        kernels.push(row);
                    }
                }
                Ok(PreparedObjective::Wavelet(kernels))
            }
        }
    }
}

impl PreparedObjective {
    pub(super) fn score(&self, d: &ExafsDataset, model: &[f64]) -> Result<f64, RmcError> {
        require(
            model.len() == d.k.len() && model.iter().all(|v| v.is_finite()),
            "calculator returned invalid spectrum",
        )?;
        let residual: Vec<_> = model
            .iter()
            .zip(&d.chi)
            .zip(&d.sigma)
            .zip(&d.k)
            .map(|(((&m, &y), &s), &k)| (m - y) / s * k.powi(i32::from(d.kweight)))
            .collect();
        let raw = match self {
            Self::K => residual.iter().map(|v| v * v).sum::<f64>() / residual.len() as f64,
            Self::LocalSpectrum(transform) => transform.score(&residual)?,
            Self::Wavelet(kernels) => {
                kernels
                    .iter()
                    .map(|row| {
                        row.iter()
                            .zip(&residual)
                            .map(|(v, r)| v * r)
                            .sum::<Complex64>()
                            .norm_sqr()
                    })
                    .sum::<f64>()
                    / kernels.len() as f64
            }
            Self::Fourier {
                settings,
                grid,
                mapping,
                q,
            } => {
                let values = DVector::from_iterator(
                    mapping.len(),
                    mapping.iter().map(|m| {
                        m.map_or(0., |(i, t)| (1. - t) * residual[i] + t * residual[i + 1])
                    }),
                );
                let result = apply_kweight_transform(grid, &values, settings, 0.)
                    .map_err(|e| RmcError::Invalid(e.to_string()))?;
                if *q {
                    require(!result.q_mask.is_empty(), "q fit range selects no points")?;
                    result
                        .q_mask
                        .iter()
                        .map(|&i| result.chiq[i].powi(2))
                        .sum::<f64>()
                        / result.q_mask.len() as f64
                } else {
                    result
                        .r_space
                        .mask_indices
                        .iter()
                        .map(|&i| result.r_space.chir[i].norm_sqr())
                        .sum::<f64>()
                        / result.r_space.mask_indices.len() as f64
                }
            }
        };
        let score = d.weight * raw;
        require(
            score.is_finite(),
            "objective overflow; check noise and weights",
        )?;
        Ok(score)
    }
}

/// Transform an individual path with the same Fourier support/interpolation as
/// RMC objectives (unreleased). Input χ is dimensionless and already includes
/// degeneracy; apply S₀² and absorber/mixture weights explicitly when comparing
/// components to an experimental fit. The measured k grid is in Å⁻¹ and may
/// start above zero; unmeasured support is padded with zero. The returned owned
/// complex R and real filtered-q arrays use the existing rexafs conventions.
/// kweight is 0..=3 and applied exactly once, with no noise scaling. Complex
/// contributions add linearly; their magnitudes do not. No scattering runs here.
pub fn transform_path_fourier(
    path: &PathContribution,
    k: &[f64],
    kweight: u8,
    settings: &FeffFitTransform,
) -> Result<crate::fitting::transform::KweightTransform, RmcError> {
    require(
        k.len() >= 2
            && k.len() == path.chi.len()
            && kweight <= 3
            && k.iter().all(|v| v.is_finite() && *v >= 0.)
            && k.windows(2).all(|w| w[1] > w[0])
            && path.chi.iter().all(|v| v.is_finite()),
        "invalid path Fourier arrays or kweight",
    )?;
    let PreparedObjective::Fourier {
        settings,
        grid,
        mapping,
        ..
    } = Objective::R(settings.clone()).prepare(k)?
    else {
        unreachable!()
    };
    let values = DVector::from_iterator(
        grid.len(),
        mapping
            .iter()
            .map(|entry| entry.map_or(0., |(i, t)| path.chi[i] * (1. - t) + path.chi[i + 1] * t)),
    );
    apply_kweight_transform(&grid, &values, &settings, f64::from(kweight))
        .map_err(|e| RmcError::Invalid(e.to_string()))
}
