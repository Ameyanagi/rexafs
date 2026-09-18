//! Reusable spectral transforms, independent of refinement methods (since 0.2.10).
//!
//! [`LocalSpectrumTransform`] prepares Morlet wavelet or Gaussian short-time
//! Fourier maps from a measured k grid. Callers apply k weighting and noise
//! scaling explicitly. Preparing or evaluating a map never runs preprocessing,
//! scattering or an optimizer. Existing forward/inverse XAFS Fourier APIs remain
//! available through [`crate::xafs::xrayfft`]; fitting uses the established
//! [`crate::fitting::transform`] conventions.

mod local_spectrum;
pub use local_spectrum::*;

/// Invalid transform settings, incompatible samples or nonfinite output.
/// No caller-owned arrays or settings are changed on failure.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TransformError {
    /// The message identifies the rejected grid, window, mask or data values.
    #[error("invalid spectral transform: {0}")]
    Invalid(String),
}

fn require(condition: bool, message: impl Into<String>) -> Result<(), TransformError> {
    if condition {
        Ok(())
    } else {
        Err(TransformError::Invalid(message.into()))
    }
}
