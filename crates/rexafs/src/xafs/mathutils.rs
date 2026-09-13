//! Low-level interpolation, finite-difference and line-shape helpers.
//!
//! These routines assume finite arrays and, where stated, sorted grids. They are
//! not a replacement for [`crate::Spectrum::from_arrays`] input validation.
//! Coordinates may use any consistent unit; line-shape densities have inverse
//! coordinate units, while finite differences retain the input value units.
//! This is the default nalgebra implementation. Enabling `ndarray-compat`
//! selects a separate legacy implementation under the same module path.

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
    /// Holds endpoint values outside the knot interval. Supply matching finite
    /// `x`/`y` arrays with at least two knots and strictly increasing `x`.
    /// Between knots, the result is `y[j] + u * (y[j+1] - y[j])`, where
    /// `u = (q - x[j]) / (x[j+1] - x[j])` and `q` is a query coordinate.
    /// Query and knot coordinates use the same units; output has the units of `y`
    /// and the same length as `self`, including an empty result for no queries.
    /// Length/order errors return `LinearError`, while empty or NaN knot arrays
    /// can panic before that validation. Non-finite values are not otherwise checked.
    fn interpolate(&self, x: &[f64], y: &[f64]) -> Result<Self, LinearError>
    where
        Self: Sized;

    /// Return whether coordinates are nondecreasing; duplicate values are allowed.
    /// Empty and one-point vectors return true. This does not validate finiteness.
    fn is_sorted(&self) -> bool;

    /// Return indices that sort finite values in ascending order, without changing
    /// the input. Equal values preserve their original order. NaN comparisons panic.
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
    /// at half maximum. Uses the real part of the complex Faddeeva function with argument
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
    /// inside and one-sided differences at the ends. Interior entry `i` is
    /// `(self[i+1] - self[i-1]) / 2`; endpoints use the adjacent difference.
    /// No physical grid spacing is applied, so output retains input value units.
    /// Divide by the spacing for a physical derivative on a uniform grid.
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

/// Check adjacent comparisons without allocating or rejecting isolated NaNs.
fn is_sorted(data: &[f64]) -> bool {
    data.windows(2).all(|pair| pair[0] <= pair[1])
}

/// Stable ascending indirect sort; unordered floating-point comparisons panic.
fn argsort(v: &[f64]) -> Vec<usize> {
    let mut idx = (0..v.len()).collect::<Vec<_>>();
    idx.sort_by(|a, b| v[*a].partial_cmp(&v[*b]).unwrap());
    idx
}

/// Return the final index at or below a finite `value`, clamped to the array ends.
/// The input must be finite and ascending; equal coordinates are allowed.
/// Returns an error when empty. This linear-search legacy helper assumes sorted
/// data despite its name; unsorted/NaN arrays can panic. Coordinates and `value`
/// use the same units, and the input is unchanged.
///
/// ```
/// use rexafs::xafs::mathutils::index_of;
/// assert_eq!(index_of(&[1.0, 2.0, 3.0, 4.0], &3.4).unwrap(), 2);
/// ```
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

/// Return the final index at or below a finite `value` on a sorted finite array.
/// Uses binary partitioning, clamps outside values to the ends and errors when
/// empty. Equal coordinates are allowed, with the last exact match selected.
/// Coordinates and `value` use the same units. Sorting/finiteness are assumed,
/// not checked; neither the input nor its order is changed.
pub fn index_of_sorted(array: &[f64], value: &f64) -> Result<usize, MathError> {
    if array.is_empty() {
        return Err(MathError::IndexOutOfBounds { index: 0, len: 0 });
    }
    let idx = array.partition_point(|x| x <= value);
    Ok(idx.saturating_sub(1))
}

/// Find the index minimizing absolute distance to `value`; the first tie wins.
/// The array need not be sorted. Supply finite coordinates and a finite `value`
/// in the same units; inputs are unchanged. Legacy behavior panics for empty
/// arrays or NaN comparisons despite the `Result` return type. Use
/// [`index_nearest_sorted`] for checked empty-input handling on sorted grids.
///
/// ```
/// use rexafs::xafs::mathutils::index_nearest;
/// assert_eq!(index_nearest(&[1.0, 2.0, 3.0, 4.0], &3.4).unwrap(), 2);
/// ```
pub fn index_nearest(array: &[f64], value: &f64) -> Result<usize, MathError> {
    Ok(array
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| (*a - value).abs().partial_cmp(&(*b - value).abs()).unwrap())
        .unwrap()
        .0)
}

/// Return the nearest index by binary search on a sorted finite array.
/// A distance tie chooses the lower of the two bracketing indices; an exact
/// duplicate match selects the first duplicate. Out-of-range values choose the
/// nearest endpoint, and an empty array returns `MathError`. Supply a finite
/// `value` in the coordinate units. Sorting/finiteness are assumed, not checked;
/// inputs are unchanged.
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
/// dimensionless argument, returning a dimensionless value. It accumulates
/// `sum_{j=0..infinity} (x*x/4)^j / (j!)^2`, where `j` is a nonnegative integer.
/// This legacy helper stops when the sum no longer changes or becomes non-finite;
/// large inputs can overflow. There is no error estimate or user-set tolerance.
/// The FFT window implementation uses [`super::bessel_i0::bessel_i0`] instead.
/// The defining series is
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
/// For the spline `S(x) = sum_j c[j] * B[j](x)`, the entry `(i, j)` is `B[j](x[i])`,
/// not the derivative with respect to the coordinate. Missing coefficient columns
/// are omitted and historical zero-padding columns remain zero. All values of `e`
/// other than 3 extend the endpoint polynomial pieces; they do not select other
/// FITPACK error modes. The [SciPy B-spline reference](https://docs.scipy.org/doc/scipy/reference/generated/scipy.interpolate.BSpline.html)
/// defines this basis convention.
///
/// This legacy wrapper consumes its vector arguments and allocates the result.
/// It does not validate inputs: `k` must be at most 5, and `t` must contain at least
/// `2 * (k + 1)` finite, nondecreasing knots with a valid base interval. Invalid
/// dimensions can panic. This is the same basis used by the internal spline evaluator.
pub fn splev_jacobian(t: Vec<f64>, c: Vec<f64>, k: usize, x: Vec<f64>, e: usize) -> DMatrix<f64> {
    super::spline::coefficient_jacobian(&t, c.len(), k, &x, e == 3)
}
