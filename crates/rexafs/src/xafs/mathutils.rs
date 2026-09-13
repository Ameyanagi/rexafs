//! Low-level interpolation, finite-difference and line-shape helpers.
//!
//! These routines assume finite arrays and, where stated, sorted grids. They are
//! not a replacement for [`crate::Spectrum::from_arrays`] input validation.
//! Coordinates may use any consistent unit; line-shape densities have inverse
//! coordinate units, while finite differences retain the input value units.

use enterpolation::{
    linear::{Linear, LinearError},
    Signal,
};
use errorfunctions::ComplexErrorFunctions;
use nalgebra::{DMatrix, DVector};
use num_complex::Complex64;

use super::errors::MathError;

/// Numerical operations on owned f64 vectors.
/// Methods returning vectors allocate new buffers. Shape widths and centers must
/// use the same units as the coordinates. Continuous area normalization of a
/// density does not guarantee unit area after sampling or finite truncation.
/// The [SciPy Voigt reference](https://docs.scipy.org/doc/scipy/reference/generated/scipy.special.voigt_profile.html)
/// defines the same Gaussian/Lorentzian widths and Faddeeva normalization;
/// rexafs clamps vanishing widths to machine epsilon rather than treating exact
/// delta-function limits separately.
pub trait MathUtils {
    /// Linearly interpolate paired knots `x`/`y` at the coordinates in `self`.
    /// Holds endpoint values outside the knot interval. Supply nonempty finite knots
    /// in increasing order; invalid spline structure returns `LinearError`, while empty
    /// or NaN knot arrays can panic before that validation.
    fn interpolate(&self, x: &[f64], y: &[f64]) -> Result<Self, LinearError>
    where
        Self: Sized;

    /// Return whether coordinates are nondecreasing; duplicate values are allowed.
    fn is_sorted(&self) -> bool;

    /// Return indices that sort finite values in ascending order. NaN comparisons panic.
    fn argsort(&self) -> Vec<usize>;

    /// Evaluate a unit-area Gaussian density at these coordinates. `sigma` is its
    /// standard deviation and `center` its mean. Uses exp(-(x-center)^2/(2*sigma^2))
    /// / (sigma*sqrt(2*pi)); nonpositive widths are clamped to machine epsilon.
    fn gaussian(self, center: f64, sigma: f64) -> DVector<f64>
    where
        Self: Into<DVector<f64>>,
    {
        let x: DVector<f64> = self.into().map(|value| value - center);
        let sigma = sigma.max(f64::EPSILON);
        let inverse_of_coefficient = sigma * (2.0 * std::f64::consts::PI).sqrt();
        x.map(|value| (-value.powi(2) / (2.0 * sigma.powi(2))).exp() / inverse_of_coefficient)
    }

    /// Evaluate a unit-area Lorentzian density at these coordinates. `sigma` is the
    /// half width at half maximum, not a standard deviation. The formula is
    /// sigma / (pi*((x-center)^2+sigma^2)); nonpositive widths are clamped to epsilon.
    fn lorentzian(self, center: f64, sigma: f64) -> DVector<f64>
    where
        Self: Into<DVector<f64>>,
    {
        let x: DVector<f64> = self.into().map(|value| value - center);
        let sigma = sigma.max(f64::EPSILON);
        let coefficient = sigma / std::f64::consts::PI;
        x.map(|value| coefficient / (value.powi(2) + sigma.powi(2)))
    }

    /// Evaluate the unit-area convolution of Gaussian and Lorentzian profiles.
    /// `sigma` is the Gaussian standard deviation and `gamma` the Lorentzian half width
    /// at half maximum. Uses the real Faddeeva function with argument
    /// ((x-center)+i*gamma)/(sigma*sqrt(2)), divided by sigma*sqrt(2*pi).
    /// Both widths are clamped to at least machine epsilon.
    fn voigt(self, center: f64, sigma: f64, gamma: f64) -> DVector<f64>
    where
        Self: Into<DVector<f64>>,
    {
        let x: DVector<f64> = self.into().map(|value| value - center);
        let sigma = sigma.max(f64::EPSILON);
        let gamma = gamma.max(f64::EPSILON);
        let inverse_of_coefficient = sigma * (2.0 * std::f64::consts::PI).sqrt();

        DVector::from_iterator(
            x.len(),
            x.iter().map(|value| {
                let z = Complex64::new(*value, gamma) / sigma / 2.0_f64.sqrt();
                z.w().re / inverse_of_coefficient
            }),
        )
    }

    /// Return the minimum value. Empty arrays or NaN comparisons can panic.
    fn min(&self) -> f64;
    /// Return the maximum value. Empty arrays or NaN comparisons can panic.
    fn max(&self) -> f64;
    /// Return successive differences `x[i+1]-x[i]`, with length n-1 (zero when empty).
    fn diff(&self) -> Self;
    /// Return derivatives with respect to sample index, using centered differences
    /// inside and one-sided differences at the ends. No physical grid spacing is applied.
    /// Empty or one-point vectors return equally sized zeros.
    fn gradient(&self) -> Self;
}

impl MathUtils for Vec<f64> {
    fn interpolate(&self, x: &[f64], y: &[f64]) -> Result<Self, LinearError> {
        let x_left = *x.iter().min_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();
        let x_right = *x.iter().max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();
        let lin = Linear::builder().elements(y).knots(x).build()?;
        let result: Vec<f64> = lin
            .sample(self.iter().map(|a| match a {
                a if a > &x_right => x_right,
                a if a < &x_left => x_left,
                _ => *a,
            }))
            .collect();
        Ok(result)
    }

    fn is_sorted(&self) -> bool {
        is_sorted(self.as_slice())
    }

    fn argsort(&self) -> Vec<usize> {
        argsort(self.as_slice())
    }

    fn min(&self) -> f64 {
        *self
            .iter()
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap()
    }

    fn max(&self) -> f64 {
        *self
            .iter()
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap()
    }

    fn diff(&self) -> Self {
        let mut result = Vec::with_capacity(self.len().saturating_sub(1));
        for i in 0..self.len().saturating_sub(1) {
            result.push(self[i + 1] - self[i]);
        }
        result
    }

    fn gradient(&self) -> Self {
        match self.len() {
            0..=1 => vec![0.0; self.len()],
            2 => vec![self[1] - self[0], self[1] - self[0]],
            _ => {
                let mut result = Vec::with_capacity(self.len());
                result.push(self[1] - self[0]);
                for i in 1..self.len() - 1 {
                    result.push((self[i + 1] - self[i - 1]) / 2.0);
                }
                result.push(self[self.len() - 1] - self[self.len() - 2]);
                result
            }
        }
    }
}

impl MathUtils for DVector<f64> {
    fn interpolate(&self, x: &[f64], y: &[f64]) -> Result<Self, LinearError> {
        let x_left = *x.iter().min_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();
        let x_right = *x.iter().max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();
        let lin = Linear::builder().elements(y).knots(x).build()?;
        let result: Vec<f64> = lin
            .sample(self.iter().map(|a| match a {
                a if a > &x_right => x_right,
                a if a < &x_left => x_left,
                _ => *a,
            }))
            .collect();
        Ok(DVector::from_vec(result))
    }

    fn is_sorted(&self) -> bool {
        is_sorted(self.as_slice())
    }

    fn argsort(&self) -> Vec<usize> {
        argsort(self.as_slice())
    }

    fn min(&self) -> f64 {
        *self
            .iter()
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap()
    }

    fn max(&self) -> f64 {
        *self
            .iter()
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap()
    }

    fn diff(&self) -> Self {
        let mut result = Vec::with_capacity(self.len().saturating_sub(1));
        for i in 0..self.len().saturating_sub(1) {
            result.push(self[i + 1] - self[i]);
        }
        DVector::from_vec(result)
    }

    fn gradient(&self) -> Self {
        match self.len() {
            0..=1 => DVector::zeros(self.len()),
            2 => DVector::from_vec(vec![self[1] - self[0], self[1] - self[0]]),
            _ => {
                let mut result = DVector::zeros(self.len());
                result[0] = self[1] - self[0];
                for i in 1..self.len() - 1 {
                    result[i] = (self[i + 1] - self[i - 1]) / 2.0;
                }
                result[self.len() - 1] = self[self.len() - 1] - self[self.len() - 2];
                result
            }
        }
    }
}

fn is_sorted(data: &[f64]) -> bool {
    data.windows(2).all(|pair| pair[0] <= pair[1])
}

fn argsort(v: &[f64]) -> Vec<usize> {
    let mut idx = (0..v.len()).collect::<Vec<_>>();
    idx.sort_by(|a, b| v[*a].partial_cmp(&v[*b]).unwrap());
    idx
}

/// Find the preceding index on an ascending finite array, clamped to its ends.
/// Returns an error when empty. This linear-search legacy helper assumes sorted
/// data despite its name; unsorted/NaN arrays can panic.
pub fn index_of(array: &[f64], value: &f64) -> Result<usize, MathError> {
    if array.is_empty() {
        return Err(MathError::IndexOutOfBounds { index: 0, len: 0 });
    }

    let min_value = array
        .iter()
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .copied()
        .ok_or(MathError::IndexOutOfBounds { index: 0, len: 0 })?;
    if &min_value > value {
        return Ok(0);
    }

    Ok(array
        .iter()
        .enumerate()
        .find_map(|(i, x)| if x > value { Some(i - 1) } else { None })
        .unwrap_or(array.len() - 1))
}

/// Find the final coordinate at or below `value` on a sorted finite array.
/// Uses binary partitioning, clamps outside values to the ends and errors when empty.
pub fn index_of_sorted(array: &[f64], value: &f64) -> Result<usize, MathError> {
    if array.is_empty() {
        return Err(MathError::IndexOutOfBounds { index: 0, len: 0 });
    }
    let idx = array.partition_point(|x| x <= value);
    Ok(idx.saturating_sub(1))
}

/// Find the index minimizing absolute distance to `value`; the first tie wins.
/// Legacy behavior panics for empty arrays or NaN comparisons. Use
/// [`index_nearest_sorted`] for checked empty-input handling on sorted grids.
pub fn index_nearest(array: &[f64], value: &f64) -> Result<usize, MathError> {
    Ok(array
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| (*a - value).abs().partial_cmp(&(*b - value).abs()).unwrap())
        .unwrap()
        .0)
}

/// Find the nearest coordinate by binary search on a sorted finite array.
/// Ties choose the lower index, out-of-range values choose the nearest endpoint,
/// and an empty array returns `MathError`.
pub fn index_nearest_sorted(array: &[f64], value: &f64) -> Result<usize, MathError> {
    if array.is_empty() {
        return Err(MathError::IndexOutOfBounds { index: 0, len: 0 });
    }

    let idx = array.partition_point(|x| x < value);
    if idx == 0 {
        return Ok(0);
    }
    if idx >= array.len() {
        return Ok(array.len() - 1);
    }

    let prev = array[idx - 1];
    let next = array[idx];
    if (prev - value).abs() <= (next - value).abs() {
        Ok(idx - 1)
    } else {
        Ok(idx)
    }
}

#[allow(non_snake_case)]
/// Evaluate the modified Bessel function I0 by its power series for a
/// dimensionless argument. This legacy helper stops when the sum no longer changes
/// or becomes non-finite; large inputs can overflow. The FFT window implementation
/// uses the dedicated Bessel helper in its own module. The defining series is
/// [DLMF equation 10.25.2](https://dlmf.nist.gov/10.25.E2) at order zero.
pub fn bessel_I0(x: f64) -> f64 {
    let base = x * x / 4.0;
    let mut addend = 1.0;
    let mut sum = 1.0;
    for j in 1.. {
        addend = addend * base / (j * j) as f64;
        let old = sum;
        sum += addend;
        if sum == old || !sum.is_finite() {
            break;
        }
    }
    sum
}

/// Evaluate the B-spline Jacobian with respect to its coefficients.
/// Rows correspond to query coordinates `x`; columns to the length of `c`.
/// The coefficient values themselves are unused because the spline is linear
/// in those coefficients. `t` supplies knots, `k` the spline degree, and `e == 3`
/// requests endpoint clamping. Coordinates and knots must use the same units;
/// the basis derivatives with respect to coefficients are dimensionless.
/// Delegates to `spline::coefficient_jacobian`; this legacy wrapper has
/// no separate validation or error return.
pub fn splev_jacobian(t: Vec<f64>, c: Vec<f64>, k: usize, x: Vec<f64>, e: usize) -> DMatrix<f64> {
    super::spline::coefficient_jacobian(&t, c.len(), k, &x, e == 3)
}
