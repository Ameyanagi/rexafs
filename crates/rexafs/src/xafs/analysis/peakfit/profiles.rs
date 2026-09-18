//! Unit-area profiles with explicit FWHM parameters. These functions evaluate
//! the mathematical definitions in lmfit's line-shape reference, with rexafs
//! converting FWHM to Gaussian standard deviation and Lorentzian half-width.
//! See <https://lmfit.github.io/lmfit-py/builtin_models.html>.
use errorfunctions::{ComplexErrorFunctions, RealErrorFunctions};
use num_complex::Complex64;
use serde::{Deserialize, Serialize};
use std::f64::consts::{LN_2, PI, SQRT_2};

/// A peak or step's mathematical shape; roles are recorded separately.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PeakShape {
    /// Constant offset in the selected signal units.
    Constant,
    /// Offset plus slope times (E-reference); slope is signal units/eV.
    Linear,
    /// Unit-area normal distribution. `width` is FWHM, in eV.
    Gaussian,
    /// Unit-area Cauchy distribution. `width` is FWHM, in eV.
    Lorentzian,
    /// Unit-area mixture with common FWHM and Lorentzian `fraction` in `[0, 1]`.
    PseudoVoigt,
    /// Gaussian/Lorentzian convolution with separate `width` and `lorentz_width`
    /// FWHM contributions. Either contribution can be zero, but not both.
    Voigt,
    /// Step `height * (1 + erf((E-center)/scale))/2`; scale is positive eV.
    ErfStep,
    /// Step `height * (1/2 + atan((E-center)/scale)/pi)`; scale is positive eV.
    ArctanStep,
}

impl PeakShape {
    pub(crate) fn is_peak(self) -> bool {
        matches!(
            self,
            Self::Gaussian | Self::Lorentzian | Self::PseudoVoigt | Self::Voigt
        )
    }
    pub(crate) fn is_step(self) -> bool {
        matches!(self, Self::ErfStep | Self::ArctanStep)
    }
    pub(crate) fn parameter_names(self) -> &'static [&'static str] {
        match self {
            Self::Constant => &["offset"],
            Self::Linear => &["offset", "slope", "reference"],
            Self::Gaussian | Self::Lorentzian => &["center", "area", "width"],
            Self::PseudoVoigt => &["center", "area", "width", "fraction"],
            Self::Voigt => &["center", "area", "width", "lorentz_width"],
            Self::ErfStep | Self::ArctanStep => &["center", "height", "scale"],
        }
    }
}

pub(crate) fn gaussian(x: f64, width: f64) -> f64 {
    let sigma = width / (2. * (2. * LN_2).sqrt());
    (-0.5 * (x / sigma).powi(2)).exp() / (sigma * (2. * PI).sqrt())
}
pub(crate) fn lorentzian(x: f64, width: f64) -> f64 {
    let gamma = width / 2.;
    // Avoid squaring enormous coordinates or extremely small widths.
    let u = x / gamma;
    if u.abs() > 1e150 {
        gamma / x / x / PI
    } else {
        1. / (PI * gamma * (1. + u * u))
    }
}
pub(crate) fn voigt(x: f64, gaussian_width: f64, lorentz_width: f64) -> f64 {
    if gaussian_width == 0. {
        return lorentzian(x, lorentz_width);
    }
    if lorentz_width == 0. {
        return gaussian(x, gaussian_width);
    }
    let sigma = gaussian_width / (2. * (2. * LN_2).sqrt());
    Complex64::new(x / sigma / SQRT_2, lorentz_width / 2. / sigma / SQRT_2)
        .w()
        .re
        / (sigma * (2. * PI).sqrt())
}

/// Caller validates finite parameters and physical width/fraction domains.
pub(crate) fn evaluate(shape: PeakShape, x: f64, p: &[f64]) -> f64 {
    let offset = x - p[0];
    match shape {
        PeakShape::Constant => p[0],
        PeakShape::Linear => p[0] + p[1] * (x - p[2]),
        PeakShape::Gaussian => p[1] * gaussian(offset, p[2]),
        PeakShape::Lorentzian => p[1] * lorentzian(offset, p[2]),
        PeakShape::PseudoVoigt => {
            p[1] * ((1. - p[3]) * gaussian(offset, p[2]) + p[3] * lorentzian(offset, p[2]))
        }
        PeakShape::Voigt => p[1] * voigt(offset, p[2], p[3]),
        PeakShape::ErfStep => p[1] * (1. + RealErrorFunctions::erf(offset / p[2])) / 2.,
        PeakShape::ArctanStep => p[1] * (0.5 + (offset / p[2]).atan() / PI),
    }
}

/// Exact Gaussian/Lorentzian limits; true Voigt width is a numerical half-height
/// root, not the commonly quoted empirical combined-width approximation.
pub(crate) fn fwhm(shape: PeakShape, p: &[f64]) -> Option<f64> {
    if !shape.is_peak() {
        return None;
    }
    if shape != PeakShape::Voigt {
        return Some(p[2]);
    }
    if p[2] == 0. {
        return Some(p[3]);
    }
    if p[3] == 0. {
        return Some(p[2]);
    }
    let scale = p[2].max(p[3]);
    let (gaussian, lorentzian) = (p[2] / scale, p[3] / scale);
    let target = voigt(0., gaussian, lorentzian) / 2.;
    let (mut low, mut high) = (0., gaussian + lorentzian);
    for _ in 0..64 {
        if voigt(high, gaussian, lorentzian) <= target {
            break;
        }
        high *= 2.;
    }
    for _ in 0..80 {
        let mid = low + (high - low) / 2.;
        if voigt(mid, gaussian, lorentzian) > target {
            low = mid;
        } else {
            high = mid;
        }
    }
    let width = (low + high) * scale;
    width.is_finite().then_some(width)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn analytic_heights_widths_and_step_limits() {
        let width = 4.;
        assert!((gaussian(0., width) - 2. * (LN_2 / PI).sqrt() / width).abs() < 1e-15);
        assert!((lorentzian(0., width) - 2. / (PI * width)).abs() < 1e-15);
        for shape in [
            PeakShape::Gaussian,
            PeakShape::Lorentzian,
            PeakShape::PseudoVoigt,
            PeakShape::Voigt,
        ] {
            let p = [5., 3., width, 0.4];
            let half_width = fwhm(shape, &p).unwrap() / 2.;
            let center = evaluate(shape, 5., &p);
            assert!((evaluate(shape, 5. + half_width, &p) / center - 0.5).abs() < 1e-12);
        }
        for shape in [PeakShape::ErfStep, PeakShape::ArctanStep] {
            let p = [3., 2., 0.5];
            assert_eq!(evaluate(shape, 3., &p), 1.);
            assert!((evaluate(shape, 1e12, &p) - 2.).abs() < 1e-11);
            assert!(evaluate(shape, -1e12, &p).abs() < 1e-11);
            assert_eq!(fwhm(shape, &p), None);
        }
    }
    #[test]
    fn voigt_zero_width_limits_and_center_reference() {
        // w(i*y) = exp(y*y) * erfc(y), independent of complex evaluator.
        let sigma = 1.;
        let gamma = 0.5;
        let width = 2. * (2. * LN_2).sqrt() * sigma;
        let y = gamma / sigma / SQRT_2;
        let expected = (y * y).exp() * RealErrorFunctions::erfc(y) / (sigma * (2. * PI).sqrt());
        assert!((voigt(0., width, 2. * gamma) - expected).abs() < 2e-15);
        for x in [-10., -1., 0., 1., 10.] {
            assert_eq!(voigt(x, width, 0.), gaussian(x, width));
            assert_eq!(voigt(x, 0., 2. * gamma), lorentzian(x, 2. * gamma));
        }
        let reference = fwhm(PeakShape::Voigt, &[0., 1., 1., 0.5]).unwrap();
        for scale in [1e-200, 1e200] {
            let width = fwhm(PeakShape::Voigt, &[0., 1., scale, scale * 0.5]).unwrap();
            assert!((width / scale - reference).abs() < 1e-14);
        }
        assert_eq!(fwhm(PeakShape::Voigt, &[0., 1., f64::MAX, f64::MAX]), None);
    }
}
