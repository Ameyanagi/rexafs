//! Legacy interpolation, finite-difference and line-shape helpers.
//!
//! Enabling `ndarray-compat` selects this implementation as `xafs::mathutils`.
//! It supports owned Vec, DVector and Array1 inputs. Line-shape methods return
//! Array1 densities in inverse coordinate units; differences retain value units.
//! Inputs are not validated as spectra. In particular, `diff` requires a nonempty
//! vector in this backend, unlike the default nalgebra implementation.

use enterpolation::{
    linear::{Linear, LinearError},
    Signal,
};
use errorfunctions::ComplexErrorFunctions;
use nalgebra::DMatrix;
use ndarray::{Array1, ArrayBase, Ix1, OwnedRepr};
use num_complex::Complex64;
use std::error::Error;

use super::errors::MathError;

#[deny(clippy::reversed_empty_ranges)]

/// Numerical helpers for owned one-dimensional arrays in the legacy backend.
///
/// Returned arrays own new buffers; borrowed inputs are unchanged. Line-shape
/// methods consume the coordinate array. Centers and widths use the coordinate
/// unit, and continuous unit area does not guarantee a sampled or truncated
/// density sums to one. Widths below machine epsilon are clamped to epsilon;
/// this does not implement exact delta-function limits. See the
/// [SciPy Voigt reference](https://docs.scipy.org/doc/scipy/reference/generated/scipy.special.voigt_profile.html)
/// for the Gaussian/Lorentzian width and Faddeeva conventions.
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
    fn gaussian(self, center: f64, sigma: f64) -> Array1<f64>
    where
        Self: Into<Array1<f64>>,
    {
        let x: Array1<f64> = self.into() - center;
        let sigma = sigma.max(f64::EPSILON);
        let inverse_of_coefficient = sigma * (2.0 * std::f64::consts::PI).sqrt();
        x.map(|x| (-x.powi(2) / (2.0 * sigma.powi(2))).exp() / inverse_of_coefficient)
    }

    /// Evaluate a unit-area Lorentzian density at these coordinates. `sigma` is the
    /// half width at half maximum, not a standard deviation. The formula is
    /// sigma / (pi*((x-center)^2+sigma^2)); nonpositive widths are clamped to epsilon.
    fn lorentzian(self, center: f64, sigma: f64) -> Array1<f64>
    where
        Self: Into<Array1<f64>>,
    {
        let x: Array1<f64> = self.into() - center;
        let sigma = sigma.max(f64::EPSILON);
        let coefficient = sigma / std::f64::consts::PI;
        x.map(|x| coefficient / (x.powi(2) + sigma.powi(2)))
    }

    /// Evaluate the unit-area convolution of Gaussian and Lorentzian profiles.
    /// `sigma` is the Gaussian standard deviation and `gamma` the Lorentzian half width
    /// at half maximum. Uses the real part of the complex Faddeeva function with argument
    /// ((x-center)+i*gamma)/(sigma*sqrt(2)), divided by sigma*sqrt(2*pi).
    /// Both widths are clamped to at least machine epsilon.
    fn voigt(self, center: f64, sigma: f64, gamma: f64) -> Array1<f64>
    where
        Self: Into<Array1<f64>>,
    {
        let x: Array1<f64> = self.into() - center;
        let sigma = sigma.max(f64::EPSILON);
        let gamma = gamma.max(f64::EPSILON);
        let inverse_of_coefficient = sigma * (2.0 * std::f64::consts::PI).sqrt();

        x.iter()
            .map(|x| {
                let z = Complex64::new(*x, gamma) / sigma / 2.0_f64.sqrt();
                z.w().re / inverse_of_coefficient
            })
            .collect()
    }

    /// Return the minimum value. Empty arrays or NaN comparisons can panic.
    fn min(&self) -> f64;
    /// Return the maximum value. Empty arrays or NaN comparisons can panic.
    fn max(&self) -> f64;
    /// Return successive differences `x[i+1] - x[i]` in a new array of length n-1.
    /// Values keep the input units. This legacy backend requires at least one
    /// sample; empty vectors can panic through subtraction underflow or slicing.
    fn diff(&self) -> Self;
    /// Return derivatives with respect to sample index, using centered differences
    /// inside and one-sided differences at the ends. Interior entry `i` is
    /// `(self[i+1] - self[i-1]) / 2`; endpoints use the adjacent difference.
    /// No physical grid spacing is applied, so output retains input value units.
    /// Divide by the spacing for a physical derivative on a uniform grid.
    /// Empty or one-point vectors return equally sized zeros.
    fn gradient(&self) -> Self;
    /// Return the peak-to-peak range, `max() - min()`, in input value units.
    /// Empty inputs or unordered NaN comparisons can panic.
    fn ptp(&self) -> f64
    where
        Self: IntoIterator<Item = f64> + Sized,
    {
        self.max() - self.min()
    }

    /// Return the first index of the minimum value.
    /// Clones the input for iteration; empty arrays or NaN comparisons can panic.
    fn argmin(&self) -> usize
    where
        Self: IntoIterator<Item = f64> + Sized + Clone,
    {
        self.clone()
            .into_iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .unwrap()
            .0
    }

    /// Return the last index of the maximum value when values tie.
    /// Clones the input for iteration; empty arrays or NaN comparisons can panic.
    fn argmax(&self) -> usize
    where
        Self: IntoIterator<Item = f64> + Sized + Clone,
    {
        self.clone()
            .into_iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .unwrap()
            .0
    }

    /// Return the first index with the smallest absolute value.
    /// Clones the input for iteration; empty arrays or NaN comparisons can panic.
    fn abs_argmin(&self) -> usize
    where
        Self: IntoIterator<Item = f64> + Sized + Clone,
    {
        self.clone()
            .into_iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| a.abs().partial_cmp(&b.abs()).unwrap())
            .unwrap()
            .0
    }
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
        is_sorted(self)
    }

    fn argsort(&self) -> Vec<usize> {
        argsort(self)
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
        let mut result = Vec::with_capacity(self.len() - 1);
        for i in 0..self.len() - 1 {
            result.push(self[i + 1] - self[i]);
        }
        result
    }

    /// Estimate the derivative with respect to sample index using central differences.
    ///
    /// # Example
    /// ```
    /// use rexafs::xafs::mathutils::MathUtils;
    /// let v = vec![1., 2., 4., 7., 11., 16.];
    /// assert_eq!(v.gradient(), vec![1. , 1.5, 2.5, 3.5, 4.5, 5. ]);
    ///
    /// let v = vec![0.];
    /// assert_eq!(v.gradient(), vec![0.]);
    ///
    /// let v = vec![1., 2.];
    /// assert_eq!(v.gradient(), vec![1., 1.]);
    ///
    /// ```
    fn gradient(&self) -> Self {
        let mut result = Vec::with_capacity(self.len());

        match self.len() {
            0..=1 => vec![0.; self.len()],
            2 => vec![self[1] - self[0], self[1] - self[0]],
            _ => {
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

impl MathUtils for nalgebra::DVector<f64> {
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
        Ok(Self::from_vec(result))
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
        let mut result = Vec::with_capacity(self.len() - 1);
        for i in 0..self.len() - 1 {
            result.push(self[i + 1] - self[i]);
        }
        Self::from_vec(result)
    }

    /// Estimate the derivative with respect to sample index using central differences.
    ///
    /// # Example
    /// ```
    /// use rexafs::xafs::mathutils::MathUtils;
    /// use nalgebra::DVector;
    ///
    /// let v = DVector::from_vec(vec![1., 2., 4., 7., 11., 16.]);
    /// assert_eq!(v.gradient(), DVector::from_vec(vec![1. , 1.5, 2.5, 3.5, 4.5, 5. ]));
    ///
    /// let v = DVector::from_vec(vec![0.]);
    /// assert_eq!(v.gradient(), DVector::from_vec(vec![0.]));
    ///
    /// let v = DVector::from_vec(vec![1., 2.]);
    /// assert_eq!(v.gradient(), DVector::from_vec(vec![1., 1.]));
    ///
    /// ```
    fn gradient(&self) -> Self {
        match self.len() {
            0..=1 => Self::zeros(self.len()),
            2 => Self::from_vec(vec![self[1] - self[0], self[1] - self[0]]),
            _ => {
                let mut result = Self::zeros(self.len());
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

impl MathUtils for ArrayBase<OwnedRepr<f64>, Ix1> {
    fn interpolate(&self, x: &[f64], y: &[f64]) -> Result<Self, LinearError> {
        let x_left = *x.iter().min_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();
        let x_right = *x.iter().max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();
        let lin = Linear::builder().elements(y).knots(x).build()?;
        let result: Vec<f64> = lin
            .sample(self.map(|a| match a {
                a if a > &x_right => x_right,
                a if a < &x_left => x_left,
                _ => *a,
            }))
            .collect();

        Ok(result.into())
    }

    fn is_sorted(&self) -> bool {
        is_sorted(self.to_vec())
    }

    fn argsort(&self) -> Vec<usize> {
        argsort(&self.to_vec())
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
        &self.slice(ndarray::s![1..]) - &self.slice(ndarray::s![..-1])
    }

    fn gaussian(self, center: f64, sigma: f64) -> Array1<f64> {
        let x: Array1<f64> = self - center;
        let sigma = sigma.max(f64::EPSILON);
        let inverse_of_coefficient = sigma * (2.0 * std::f64::consts::PI).sqrt();
        x.map(|x| (-x.powi(2) / (2.0 * sigma.powi(2))).exp() / inverse_of_coefficient)
    }

    fn lorentzian(self, center: f64, sigma: f64) -> Array1<f64> {
        let x: Array1<f64> = self - center;
        let sigma = sigma.max(f64::EPSILON);
        let coefficient = sigma / std::f64::consts::PI;
        x.map(|x| coefficient / (x.powi(2) + sigma.powi(2)))
    }

    fn voigt(self, center: f64, sigma: f64, gamma: f64) -> Array1<f64> {
        let x: Array1<f64> = self - center;
        let sigma = sigma.max(f64::EPSILON);
        let gamma = gamma.max(f64::EPSILON);
        let inverse_of_coefficient = sigma * (2.0 * std::f64::consts::PI).sqrt();

        x.iter()
            .map(|x| {
                let z = Complex64::new(*x, gamma) / sigma / 2.0_f64.sqrt();
                z.w().re / inverse_of_coefficient
            })
            .collect()
    }
    /// Estimate the derivative with respect to sample index using central differences.
    ///
    /// # Example
    /// ```
    /// use rexafs::xafs::mathutils::MathUtils;
    /// use ndarray::Array1;
    ///
    /// let v = Array1::from_vec(vec![1., 2., 4., 7., 11., 16.]);
    /// assert_eq!(v.gradient(), Array1::from_vec(vec![1. , 1.5, 2.5, 3.5, 4.5, 5. ]));
    ///
    /// let v = Array1::from_vec(vec![0.]);
    /// assert_eq!(v.gradient(), Array1::from_vec(vec![0.]));
    ///
    /// let v = Array1::from_vec(vec![1., 2.]);
    /// assert_eq!(v.gradient(), Array1::from_vec(vec![1., 1.]));
    ///
    /// ```
    #[allow(clippy::reversed_empty_ranges)]
    fn gradient(&self) -> Self {
        match self.len() {
            0..=1 => Array1::zeros(self.len()),
            2 => Array1::from_vec(vec![self[1] - self[0], self[1] - self[0]]),
            _ => {
                let mut result = Array1::zeros(self.len());

                result
                    .slice_mut(ndarray::s![0])
                    .assign(&(&self.slice(ndarray::s![1]) - &self.slice(ndarray::s![0])));
                result.slice_mut(ndarray::s![1..-1]).assign(
                    &((&self.slice(ndarray::s![2..]) - &self.slice(ndarray::s![..-2])) / 2.0),
                );
                result
                    .slice_mut(ndarray::s![-1])
                    .assign(&(&self.slice(ndarray::s![-1]) - &self.slice(ndarray::s![-2])));
                result
            }
        }
    }
}

/// Check adjacent comparisons without allocating or rejecting isolated NaNs.
fn is_sorted<I>(data: I) -> bool
where
    I: IntoIterator,
    I::Item: PartialOrd,
{
    let mut it = data.into_iter();
    match it.next() {
        None => true,
        Some(first) => it
            .scan(first, |state, next| {
                let cmp = *state <= next;
                *state = next;
                Some(cmp)
            })
            .all(|b| b),
    }
}

/// Stable ascending indirect sort; unordered floating-point comparisons panic.
fn argsort<T: PartialOrd>(v: &[T]) -> Vec<usize> {
    let mut idx = (0..v.len()).collect::<Vec<_>>();
    idx.sort_by(|a, b| v[*a].partial_cmp(&v[*b]).unwrap());
    idx
}

/// Standalone equivalent of [`MathUtils::gaussian`], with the same width and unit conventions.
fn gaussian<T: Into<Array1<f64>>>(x: T, center: f64, sigma: f64) -> Array1<f64> {
    let x: Array1<f64> = x.into() - center;
    let sigma = sigma.max(f64::EPSILON);
    let inverse_of_coefficient = sigma * (2.0 * std::f64::consts::PI).sqrt();
    x.map(|x| (-x.powi(2) / (2.0 * sigma.powi(2))).exp() / inverse_of_coefficient)
}

/// Standalone equivalent of [`MathUtils::lorentzian`], with the same width and unit conventions.
fn lorentzian<T: Into<Array1<f64>>>(x: T, center: f64, sigma: f64) -> Array1<f64> {
    let x: Array1<f64> = x.into() - center;
    let sigma = sigma.max(f64::EPSILON);
    let coefficient = sigma / std::f64::consts::PI;
    x.map(|x| coefficient / (x.powi(2) + sigma.powi(2)))
}

/// Standalone equivalent of [`MathUtils::voigt`], with the same width and unit conventions.
fn voigt<T: Into<Array1<f64>>>(x: T, center: f64, sigma: f64, gamma: f64) -> Array1<f64> {
    let x: Array1<f64> = x.into() - center;
    let sigma = sigma.max(f64::EPSILON);
    let gamma = gamma.max(f64::EPSILON);
    let inverse_of_coefficient = sigma * (2.0 * std::f64::consts::PI).sqrt();

    x.iter()
        .map(|x| {
            let z = Complex64::new(*x, gamma) / sigma / 2.0_f64.sqrt();
            z.w().re / inverse_of_coefficient
        })
        .collect()
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

#[cfg(test)]
mod tests {
    use std::vec;

    use super::*;
    use crate::xafs::tests::PARAM_LOADTXT;
    use crate::xafs::tests::TEST_TOL;
    use crate::xafs::tests::TOP_DIR;
    use approx::assert_abs_diff_eq;
    use nalgebra::DVector;

    const NUMERICAL_TEST_TOL: f64 = 1e-6;

    #[test]
    fn test_argsort_float() {
        let v = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        assert_eq!(v.argsort(), vec![0, 1, 2, 3, 4]);
        let v = vec![1.0, 2.0, 3.0, 4.0, 5.0, 3.0];
        assert_eq!(v.argsort(), vec![0, 1, 2, 5, 3, 4]);

        let v = Array1::from(vec![1.0, 2.0, 3.0, 4.0, 5.0]);
        assert_eq!(v.argsort(), vec![0, 1, 2, 3, 4]);
        let v = Array1::from(vec![1.0, 2.0, 3.0, 4.0, 5.0, 3.0]);
        assert_eq!(v.argsort(), vec![0, 1, 2, 5, 3, 4]);
    }

    #[test]
    fn test_interpolation() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![0.0, 2.0, 2.5, 4.0, 5.0];
        let z = vec![10.0, 20.0, 30.0, 40.0, 50.0];

        let expected = [15.0, 20.0, 33.333333333333336, 40.0, 50.0];
        x.interpolate(&y, &z)
            .unwrap()
            .iter()
            .zip(expected.iter())
            .for_each(|(a, b)| {
                assert_abs_diff_eq!(a, b, epsilon = TEST_TOL);
            });
    }

    #[test]
    fn test_gaussian() {
        let x = Array1::from_vec(vec![0., 1.0, 2.0, 3.0, 4.0, 5.0, 6., 7., 8., 9.]);
        let y = gaussian(x.clone(), 5.0, 1.0);
        let expected = Array1::from_vec(vec![
            1.4867195147342979e-6,
            0.00013383022576488537,
            0.0044318484119380075,
            0.05399096651318806,
            0.24197072451914337,
            0.3989422804014327,
            0.24197072451914337,
            0.05399096651318806,
            0.0044318484119380075,
            0.00013383022576488537,
        ]);

        let _ = y.iter().zip(expected.iter()).map(|(a, b)| {
            assert_abs_diff_eq!(a, b, epsilon = TEST_TOL);
        });

        // assert_eq!(
        //     y,
        //     Array1::from(vec![
        //         1.4867195147342979e-6,
        //         0.00013383022576488537,
        //         0.0044318484119380075,
        //         0.05399096651318806,
        //         0.24197072451914337,
        //         0.3989422804014327,
        //         0.24197072451914337,
        //         0.05399096651318806,
        //         0.0044318484119380075,
        //         0.00013383022576488537
        //     ])
        // );

        let y = x.clone().gaussian(5.0, 1.0).to_vec();
        let expected = [
            1.4867195147342979e-6,
            0.00013383022576488537,
            0.0044318484119380075,
            0.05399096651318806,
            0.24197072451914337,
            0.3989422804014327,
            0.24197072451914337,
            0.05399096651318806,
            0.0044318484119380075,
            0.00013383022576488537,
        ];

        let _ = y.iter().zip(expected.iter()).map(|(a, b)| {
            assert_abs_diff_eq!(a, b, epsilon = TEST_TOL);
        });
    }

    #[test]
    #[allow(clippy::approx_constant)]
    fn test_lorentzian() {
        let x = Array1::from_vec(vec![0., 1.0, 2.0, 3.0, 4.0, 5.0, 6., 7., 8., 9.]);
        let y = lorentzian(x.clone(), 5.0, 1.0);

        let expected = Array1::from_vec(vec![
            0.012242687930145796,
            0.01872411095198769,
            0.03183098861837907,
            0.06366197723675814,
            0.15915494309189535,
            0.3183098861837907,
            0.15915494309189535,
            0.06366197723675814,
            0.03183098861837907,
            0.01872411095198769,
        ]);

        let _ = y.iter().zip(expected.iter()).map(|(a, b)| {
            assert_abs_diff_eq!(a, b, epsilon = TEST_TOL);
        });

        let y = x.clone().lorentzian(5.0, 1.0).to_vec();

        let expected = [
            0.012242687930145796,
            0.01872411095198769,
            0.03183098861837907,
            0.06366197723675814,
            0.15915494309189535,
            0.3183098861837907,
            0.15915494309189535,
            0.06366197723675814,
            0.03183098861837907,
            0.01872411095198769,
        ];

        y.iter().zip(expected.iter()).for_each(|(a, b)| {
            assert_abs_diff_eq!(a, b, epsilon = TEST_TOL);
        });
    }

    #[test]
    fn test_voigt() {
        let x = Array1::from_vec(vec![0., 1.0, 2.0, 3.0, 4.0, 5.0, 6., 7., 8., 9.]);
        let y = voigt(x.clone(), 5.0, 1.0, 1.0);

        let expected = Array1::from_vec(vec![
            0.013884921288571273,
            0.022813635258707103,
            0.04338582232367969,
            0.09071519942627546,
            0.16579566268916654,
            0.20870928052036772,
            0.16579566268916654,
            0.09071519942627546,
            0.04338582232367969,
            0.022813635258707103,
        ]);

        y.iter().zip(expected.iter()).for_each(|(a, b)| {
            assert_abs_diff_eq!(a, b, epsilon = TEST_TOL);
        });

        let y = x.clone().voigt(5.0, 1.0, 1.0).to_vec();

        let expected = [
            0.013884921288571273,
            0.022813635258707103,
            0.04338582232367969,
            0.09071519942627546,
            0.16579566268916654,
            0.20870928052036772,
            0.16579566268916654,
            0.09071519942627546,
            0.04338582232367969,
            0.022813635258707103,
        ];

        y.iter().zip(expected.iter()).for_each(|(a, b)| {
            assert_abs_diff_eq!(a, b, epsilon = TEST_TOL);
        });
    }

    #[test]
    fn test_min_vec() {
        let v = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        assert_eq!(v.min(), 1.0);
        let v = vec![1.0, 2.0, 3.0, 4.0, 5.0, 3.0, 0.0];
        assert_eq!(v.min(), 0.0);
    }

    #[test]
    fn test_min_array() {
        let v = Array1::from_vec(vec![1.0, 2.0, 3.0, 4.0, 5.0]);
        assert_eq!(v.min(), 1.0);
        let v = Array1::from_vec(vec![1.0, 2.0, 3.0, 4.0, 5.0, 3.0, 0.0]);
        assert_eq!(v.min(), 0.0);
    }

    #[test]
    fn test_max_vec() {
        let v = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        assert_eq!(v.max(), 5.0);
        let v = vec![1.0, 2.0, 3.0, 4.0, 5.0, 3.0, 0.0];
        assert_eq!(v.max(), 5.0);
    }
    #[test]
    fn test_max_array() {
        let v = Array1::from_vec(vec![1.0, 2.0, 3.0, 4.0, 5.0]);
        assert_eq!(v.max(), 5.0);
        let v = Array1::from_vec(vec![1.0, 2.0, 3.0, 4.0, 5.0, 3.0, 0.0]);
        assert_eq!(v.max(), 5.0);
    }

    #[test]
    fn test_diff_vec() {
        let v = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        assert_eq!(v.diff(), vec![1.0, 1.0, 1.0, 1.0]);
    }

    #[test]
    fn test_diff_array() {
        let v = Array1::from_vec(vec![1.0, 2.0, 3.0, 4.0, 5.0]);
        assert_eq!(v.diff(), Array1::from_vec(vec![1.0, 1.0, 1.0, 1.0]));
    }

    #[test]
    fn test_splev_jacobian() {
        let x = vec![
            0.0, 0.555, 1.111, 1.666, 2.222, 2.777, 3.333, 3.888, 4.444, 5.0,
        ];
        let y = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, -1.0, -2.0, -3.0, -4.0];
        let order = 3;

        let (t, c) = super::super::spline::interpolate(&x, &y, order).unwrap();
        let k = order;

        let e = 3; // Example end condition

        let analytical_jacobian = splev_jacobian(t.clone(), c.clone(), k, x.clone(), e);

        use crate::xafs::lmutils::forward_jacobian_nalgebra_f64;

        let spline_function = |coef: &DVector<f64>| {
            DVector::from(super::super::spline::evaluate(
                &t,
                coef.as_slice(),
                k,
                &x,
                e == 3,
            ))
        };

        let numerical_jacobian = forward_jacobian_nalgebra_f64(&DVector::from(c), &spline_function);

        analytical_jacobian
            .iter()
            .zip(numerical_jacobian.iter())
            .for_each(|(a, b)| {
                assert_abs_diff_eq!(a, b, epsilon = NUMERICAL_TEST_TOL);
            });
    }
}
