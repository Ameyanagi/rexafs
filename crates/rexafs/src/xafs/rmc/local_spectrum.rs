//! Compatibility adapter for local-spectrum APIs originally exposed by RMC.
use super::RmcError;
pub use crate::transform::{
    LocalSpectrumAlgorithm, LocalSpectrumSettings, LocalSpectrumWindow, WaveletSettings,
};
use num_complex::Complex64;

/// Compatibility wrapper around [`crate::transform::LocalSpectrumTransform`].
/// New transform-only callers should use that shared API. This wrapper retains
/// the original RMC error type and numerical behavior, including mask scoring.
/// Settings and sample grids are copied once; repeated evaluations reuse kernels.
pub struct LocalSpectrumTransform(crate::transform::LocalSpectrumTransform);

impl LocalSpectrumTransform {
    /// Prepare the shared transform, retaining the original `RmcError` signature.
    /// Rejects invalid grids, windows, masks and excessive kernel allocations.
    pub fn new(k: &[f64], settings: &LocalSpectrumSettings) -> Result<Self, RmcError> {
        crate::transform::LocalSpectrumTransform::new(k, settings)
            .map(Self)
            .map_err(legacy_error)
    }

    /// Whether the prepared transform uses zero-padded FFT convolution.
    pub fn uses_fft(&self) -> bool {
        self.0.uses_fft()
    }

    /// Evaluate complex coefficients in center-major, R-minor order without
    /// changing caller arrays. Applies neither k weighting nor noise scaling.
    pub fn transform(&self, values: &[f64]) -> Result<Vec<Complex64>, RmcError> {
        self.0.transform(values).map_err(legacy_error)
    }

    pub(super) fn score(&self, residual: &[f64]) -> Result<f64, RmcError> {
        self.0.score(residual).map_err(legacy_error)
    }
}

fn legacy_error(error: crate::transform::TransformError) -> RmcError {
    match error {
        crate::transform::TransformError::Invalid(message) => RmcError::Invalid(message),
    }
}
