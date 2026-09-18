//! Local Fourier maps with direct and equivalent zero-padded convolution (unreleased).
use super::{require, TransformError};
use easyfft::dyn_size::{DynFftMut, DynIfftMut};
use num_complex::Complex64;
use serde::{Deserialize, Serialize};

/// Morlet analysis settings. This explicitly normalized project convention is
/// not an assertion of EVAX wavelet equivalence. It resolves a local oscillation
/// exp(2 i R k); R is Fourier distance, not a phase-corrected bond length.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct WaveletSettings {
    /// Window centers in Å⁻¹, finite, increasing and inside measured k support.
    pub k_centers: Vec<f64>,
    /// Positive increasing Fourier distances in Å.
    pub r: Vec<f64>,
    /// Dimensionless Morlet carrier frequency, typically 6. The Gaussian width
    /// at Fourier distance R is omega0/(2R) in Å⁻¹. Larger values improve
    /// relative R resolution while reducing k localization.
    pub omega0: f64,
}

/// Gaussian analysis window for a k–R map. R is Fourier distance in Å and
/// is not a phase-corrected bond length. These are rexafs-normalized transforms,
/// not claims of compatibility with EVAX's wavelet normalization.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LocalSpectrumWindow {
    /// Morlet window with width `omega0/(2*R)` in Å⁻¹ and admissibility
    /// correction `exp(-omega0²/2)`. R must be positive; omega0=6 is a useful start.
    Morlet {
        /// Positive dimensionless carrier frequency; larger values broaden k localization.
        omega0: f64,
    },
    /// Short-time Fourier transform (STFT) with a fixed Gaussian width and no
    /// admissibility correction. Unlike Morlet, its k resolution is independent of R.
    GaussianStft {
        /// Positive Gaussian standard deviation in Å⁻¹, e.g. 1.
        width: f64,
    },
}
/// Evaluation method. The FFT uses zero padding, never circular wraparound.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum LocalSpectrumAlgorithm {
    /// Use FFT for at least 32 centers on a uniform input grid with aligned
    /// centers, otherwise direct evaluation. Both use identical discrete kernels.
    #[default]
    Auto,
    /// Precompute dense kernels; also supports irregular grids and off-grid centers.
    Direct,
    /// Require uniform input k and centers at its sample positions; no silent resampling.
    Fft,
}
/// Local Fourier transform and mask, added after 0.2.9 (unreleased). The complex
/// kernel is `[exp(2 i R (k-center)) − correction] exp(-(k-center)²/(2 width²))`,
/// multiplied by trapezoidal quadrature weights and normalized to unit Euclidean
/// norm for each cell. Coefficients preserve the input's numerical units.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LocalSpectrumSettings {
    /// Increasing window centers inside measured k support, in Å⁻¹.
    pub k_centers: Vec<f64>,
    /// Increasing nonnegative Fourier distances in Å; Morlet requires strictly positive R.
    pub r: Vec<f64>,
    /// Morlet or fixed-width Gaussian STFT window.
    pub window: LocalSpectrumWindow,
    /// Empty means weight one everywhere. Otherwise nonnegative weights in
    /// center-major, R-minor order. Zero excludes a cell; at least one must be positive.
    /// The objective divides the weighted sum of coefficient magnitudes squared
    /// by the sum of mask weights. Masking does not alter returned coefficients.
    #[serde(default)]
    pub mask: Vec<f64>,
    /// Direct, FFT, or automatic evaluation; default Auto.
    #[serde(default)]
    pub algorithm: LocalSpectrumAlgorithm,
}
impl LocalSpectrumSettings {
    /// Construct a Morlet map using the rexafs discrete normalization,
    /// with all cells included and automatic direct/FFT selection.
    pub fn morlet(wavelet: WaveletSettings) -> Self {
        Self {
            k_centers: wavelet.k_centers,
            r: wavelet.r,
            window: LocalSpectrumWindow::Morlet {
                omega0: wavelet.omega0,
            },
            mask: Vec::new(),
            algorithm: Default::default(),
        }
    }
}
enum Kernel {
    Direct(Vec<Vec<Complex64>>),
    Fft {
        spectra: Vec<Vec<Complex64>>,
        indices: Vec<usize>,
        norms: Vec<f64>,
        size: usize,
    },
}
/// Reusable transform with fixed sampling/window/mask. Preparation copies settings
/// and computes kernels; repeated transforms reuse them. No scattering runs here.
/// Kernel payloads are bounded to four million complex values.
pub struct LocalSpectrumTransform {
    settings: LocalSpectrumSettings,
    k: Vec<f64>,
    quadrature: Vec<f64>,
    kernel: Kernel,
}
impl LocalSpectrumTransform {
    /// Validate settings and prepare dense kernels or FFT convolution spectra.
    /// Uniform-grid/alignment tolerance is `1e-10 * max(1, |k|)` in Å⁻¹.
    pub fn new(k: &[f64], settings: &LocalSpectrumSettings) -> Result<Self, TransformError> {
        require(
            k.len() >= 2
                && k.len() <= 65536
                && k.iter().all(|v| v.is_finite())
                && k.windows(2).all(|w| w[1] > w[0]),
            "invalid local-spectrum input grid",
        )?;
        require(
            !settings.k_centers.is_empty()
                && !settings.r.is_empty()
                && settings
                    .k_centers
                    .iter()
                    .all(|&v| v.is_finite() && v >= k[0] && v <= k[k.len() - 1])
                && settings.k_centers.windows(2).all(|v| v[1] > v[0])
                && settings.r.iter().all(|v| v.is_finite() && *v >= 0.)
                && settings.r.windows(2).all(|v| v[1] > v[0]),
            "invalid local-spectrum centers or R grid",
        )?;
        match settings.window {
            LocalSpectrumWindow::Morlet { omega0 } => require(
                omega0.is_finite() && omega0 > 0. && settings.r[0] > 0.,
                "Morlet requires positive width and R",
            )?,
            LocalSpectrumWindow::GaussianStft { width } => require(
                width.is_finite() && width > 0.,
                "STFT width must be positive",
            )?,
        }
        let cells = settings.k_centers.len().saturating_mul(settings.r.len());
        require(
            cells <= 4_000_000
                && (settings.mask.is_empty()
                    || (settings.mask.len() == cells
                        && settings.mask.iter().all(|v| v.is_finite() && *v >= 0.)
                        && settings.mask.iter().sum::<f64>().is_finite()
                        && settings.mask.iter().any(|v| *v > 0.))),
            "invalid local-spectrum mask or cell count",
        )?;
        let step = k[1] - k[0];
        let close = |a: f64, b: f64| (a - b).abs() <= 1e-10 * a.abs().max(b.abs()).max(1.);
        let uniform = k
            .iter()
            .enumerate()
            .all(|(i, &q)| close(q, k[0] + i as f64 * step));
        let indices: Vec<_> = settings
            .k_centers
            .iter()
            .map(|&q| ((q - k[0]) / step).round() as usize)
            .collect();
        let aligned = indices
            .iter()
            .zip(&settings.k_centers)
            .all(|(&i, &q)| i < k.len() && close(q, k[i]));
        let fft = match settings.algorithm {
            LocalSpectrumAlgorithm::Direct => false,
            LocalSpectrumAlgorithm::Auto => uniform && aligned && settings.k_centers.len() >= 32,
            LocalSpectrumAlgorithm::Fft => {
                require(
                    uniform && aligned,
                    "FFT local spectrum requires uniform k and grid-aligned centers",
                )?;
                true
            }
        };
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
        let kernel = if fft {
            let size = (3 * k.len() - 2).next_power_of_two();
            require(
                size.saturating_mul(settings.r.len()) <= 4_000_000,
                "FFT local-spectrum kernels exceed four million complex values",
            )?;
            let mut spectra = Vec::new();
            let mut norms = vec![0.; cells];
            for (ri, &r) in settings.r.iter().enumerate() {
                let mut h = vec![Complex64::new(0., 0.); size];
                for (i, value) in h.iter_mut().enumerate().take(2 * k.len() - 1) {
                    let lag = i as isize - (k.len() - 1) as isize;
                    *value = window(&settings.window, -(lag as f64) * step, r);
                }
                h.fft_mut();
                spectra.push(h);
                for (ci, &center) in settings.k_centers.iter().enumerate() {
                    let norm = k
                        .iter()
                        .zip(&quadrature)
                        .map(|(&q, &dk)| (window(&settings.window, q - center, r) * dk).norm_sqr())
                        .sum::<f64>()
                        .sqrt();
                    require(
                        norm.is_finite() && norm > 1e-100,
                        "degenerate local-spectrum kernel",
                    )?;
                    norms[ci * settings.r.len() + ri] = norm;
                }
            }
            Kernel::Fft {
                spectra,
                indices,
                norms,
                size,
            }
        } else {
            require(
                cells.saturating_mul(k.len()) <= 4_000_000,
                "direct local-spectrum kernels exceed four million complex values",
            )?;
            let mut rows = Vec::new();
            for &center in &settings.k_centers {
                for &r in &settings.r {
                    let mut row: Vec<_> = k
                        .iter()
                        .zip(&quadrature)
                        .map(|(&q, &dk)| window(&settings.window, q - center, r) * dk)
                        .collect();
                    let norm = row.iter().map(|v| v.norm_sqr()).sum::<f64>().sqrt();
                    require(
                        norm.is_finite() && norm > 1e-100,
                        "degenerate local-spectrum kernel",
                    )?;
                    for v in &mut row {
                        *v /= norm;
                    }
                    rows.push(row);
                }
            }
            Kernel::Direct(rows)
        };
        Ok(Self {
            settings: settings.clone(),
            k: k.to_vec(),
            quadrature,
            kernel,
        })
    }
    /// True when preparation selected the zero-padded FFT implementation.
    pub fn uses_fft(&self) -> bool {
        matches!(self.kernel, Kernel::Fft { .. })
    }
    /// Compute complex coefficients in center-major, R-minor order. Input values
    /// are copied to temporary FFT buffers; settings and caller arrays are unchanged.
    /// This function applies neither k weighting nor noise scaling.
    pub fn transform(&self, values: &[f64]) -> Result<Vec<Complex64>, TransformError> {
        require(
            values.len() == self.k.len() && values.iter().all(|v| v.is_finite()),
            "invalid local-spectrum values",
        )?;
        let result = match &self.kernel {
            Kernel::Direct(rows) => rows
                .iter()
                .map(|row| row.iter().zip(values).map(|(v, x)| v * x).sum())
                .collect(),
            Kernel::Fft {
                spectra,
                indices,
                norms,
                size,
            } => {
                let mut signal = vec![Complex64::new(0., 0.); *size];
                for ((out, &v), &dk) in signal.iter_mut().zip(values).zip(&self.quadrature) {
                    *out = Complex64::new(v * dk, 0.);
                }
                signal.fft_mut();
                let mut result = vec![Complex64::new(0., 0.); norms.len()];
                for (ri, h) in spectra.iter().enumerate() {
                    let mut convolution: Vec<_> =
                        signal.iter().zip(h).map(|(x, y)| x * y).collect();
                    convolution.ifft_mut();
                    for (ci, &i) in indices.iter().enumerate() {
                        let cell = ci * spectra.len() + ri;
                        result[cell] =
                            convolution[i + self.k.len() - 1] / (*size as f64 * norms[cell]);
                    }
                }
                result
            }
        };
        require(
            result
                .iter()
                .all(|v: &Complex64| v.re.is_finite() && v.im.is_finite()),
            "local-spectrum overflow",
        )?;
        Ok(result)
    }
    pub(crate) fn score(&self, residual: &[f64]) -> Result<f64, TransformError> {
        let coefficients = self.transform(residual)?;
        if self.settings.mask.is_empty() {
            Ok(coefficients.iter().map(|v| v.norm_sqr()).sum::<f64>() / coefficients.len() as f64)
        } else {
            Ok(coefficients
                .iter()
                .zip(&self.settings.mask)
                .map(|(v, w)| v.norm_sqr() * w)
                .sum::<f64>()
                / self.settings.mask.iter().sum::<f64>())
        }
    }
}
fn window(kind: &LocalSpectrumWindow, t: f64, r: f64) -> Complex64 {
    let (width, correction) = match *kind {
        LocalSpectrumWindow::Morlet { omega0 } => {
            (omega0 / (2. * r), (-0.5 * omega0 * omega0).exp())
        }
        LocalSpectrumWindow::GaussianStft { width } => (width, 0.),
    };
    (Complex64::from_polar(1., 2. * r * t) - correction) * (-0.5 * (t / width).powi(2)).exp()
}
