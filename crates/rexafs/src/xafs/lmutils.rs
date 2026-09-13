//! Finite-difference derivatives and unscaled least-squares matrix helpers.
//!
//! For n parameters x and m residuals r(x), the Jacobian J has shape m by n
//! and entries `dr[i]/dx[j]`. For loss `L = 0.5 * sum(r[i]^2)`, differentiation
//! gives `H = J^T J + sum(r[i] * Hessian(r[i]))`. The Gauss–Newton approximation
//! drops the second term: it is exact for affine residuals, and can be useful
//! for small residuals when residual curvature is modest. See the
//! [SciPy least-squares reference](https://docs.scipy.org/doc/scipy/reference/generated/scipy.optimize.least_squares.html).
//!
//! These helpers do not fit parameters, whiten residuals, estimate noise or check
//! convergence. Numerical differencing always uses the same absolute step
//! sqrt(f64::EPSILON), about 1.49e-8 in each parameter's numerical units. Parameter
//! scaling is the caller's responsibility; very large/small coordinates or
//! discontinuous residuals can make this fixed step inappropriate.

use levenberg_marquardt::{LeastSquaresProblem, LevenbergMarquardt};
use nalgebra::{DMatrix, DVector, Dyn, Owned};

const EPS_F64: f64 = f64::EPSILON;

/// Evaluate f after temporarily adding y to one parameter.
///
/// The selected coordinate becomes `x[idx] + y`, then is restored after f returns
/// normally. The perturbation has that coordinate's units; the return value has
/// the type and units supplied by f. An out-of-range index panics, and if f panics
/// the coordinate is not restored. No input validation or allocation is performed
/// by this helper beyond the callback's own work.
pub fn mod_and_calc_nalgebra_f64<T>(
    x: &mut DVector<f64>,
    f: &dyn Fn(&DVector<f64>) -> T,
    idx: usize,
    y: f64,
) -> T {
    let xtmp = x[idx];
    x[idx] = xtmp + y;
    let fx1 = (f)(x);
    x[idx] = xtmp;
    fx1
}

/// Allocate a forward-difference Jacobian of residuals at x.
///
/// Column j is `(fs(x + h*e_j) - fs(x))/h`, where e_j selects parameter j and
/// h=sqrt(f64::EPSILON) is a fixed absolute step. For m residuals and n parameters,
/// the output is m by n, requiring n+1 callback evaluations. Each entry has
/// residual units divided by the corresponding parameter units. The borrowed x
/// is unchanged; callback values must be finite and retain the same length or
/// matrix operations may return nonfinite values or panic. No adaptive step,
/// parameter scaling or uncertainty weighting is applied.
pub fn forward_jacobian_nalgebra_f64(
    x: &DVector<f64>,
    fs: &dyn Fn(&DVector<f64>) -> DVector<f64>,
) -> DMatrix<f64> {
    let fx = (fs)(x);
    let mut xt = x.clone();
    let mut jac = DMatrix::zeros(fx.len(), x.len());
    for i in 0..x.len() {
        let fx1 = mod_and_calc_nalgebra_f64(&mut xt, fs, i, EPS_F64.sqrt());
        jac.set_column(i, &((fx1 - &fx) / EPS_F64.sqrt()));
    }
    jac
}

/// Allocate a central-difference Jacobian of residuals at x.
///
/// Column j is `(fs(x + h*e_j) - fs(x - h*e_j))/(2*h)`, with the same fixed
/// absolute h=sqrt(f64::EPSILON) as the forward helper. The m by n result needs
/// 2*n+1 callback evaluations (including one to determine m). For a sufficiently
/// smooth residual this reduces truncation error relative to forward differences,
/// but cancellation and poorly scaled parameters can still dominate. Units,
/// unchanged-input behavior and callback restrictions match the forward helper.
pub fn center_jacobian_nalgebra_f64(
    x: &DVector<f64>,
    fs: &dyn Fn(&DVector<f64>) -> DVector<f64>,
) -> DMatrix<f64> {
    let fx = (fs)(x);
    let mut xt = x.clone();
    let mut jac = DMatrix::zeros(fx.len(), x.len());
    for i in 0..x.len() {
        let fx1 = mod_and_calc_nalgebra_f64(&mut xt, fs, i, EPS_F64.sqrt());
        let fx2 = mod_and_calc_nalgebra_f64(&mut xt, fs, i, -EPS_F64.sqrt());
        jac.set_column(i, &((fx1 - fx2) / (2.0 * EPS_F64.sqrt())));
    }
    jac
}

/// Allocate the Gauss–Newton matrix `J.transpose() * J` at x.
///
/// J is computed by the forward-difference helper. This n by n matrix approximates
/// the Hessian of `0.5 * sum(fs(x)^2)`, not of the unhalved sum of squares. It omits
/// residual second derivatives, as explained in the module documentation. Entry
/// (j,k) has residual-unit squared divided by the units of parameters j and k
/// when all residuals share a consistent scale. No positive-definiteness,
/// conditioning, convergence or finiteness checks are performed.
pub fn approx_hessian_nalgebra_f64(
    x: &DVector<f64>,
    fs: &dyn Fn(&DVector<f64>) -> DVector<f64>,
) -> DMatrix<f64> {
    let jac = forward_jacobian_nalgebra_f64(x, fs);
    jac.transpose() * &jac
}

/// Invert the unscaled Gauss–Newton matrix, returning None if inversion fails.
///
/// The returned n by n inverse `inv(J.transpose()*J)` is not automatically a
/// statistical parameter covariance. Covariance interpretation requires an
/// appropriate residual/noise model and local identifiability. This function does
/// not apply residual-variance, degrees-of-freedom or reduced-chi-square scaling,
/// and does not test whether a successful inverse is well conditioned. For
/// unit-variance independent residuals in a suitable linear model, entries have
/// the product units of the corresponding parameters.
pub fn approx_covariance_matrix_nalgebra_f64(
    x: &DVector<f64>,
    fs: &dyn Fn(&DVector<f64>) -> DVector<f64>,
) -> Option<DMatrix<f64>> {
    let hess = approx_hessian_nalgebra_f64(x, fs);
    hess.try_inverse()
}

/// Convenience methods for the finite-difference helpers on parameter vectors.
/// These methods do not run a Levenberg–Marquardt fit; they evaluate derivative
/// matrices at the current parameters using the callback's residual definition.
pub trait LMParameters<T> {
    /// Allocate the forward-difference residual Jacobian; see `forward_jacobian_nalgebra_f64`.
    fn jacobian(&self, f: T) -> DMatrix<f64>;
    /// Allocate JᵀJ for half the residual sum of squares; this is a Gauss–Newton approximation.
    fn hessian(&self, f: T) -> DMatrix<f64>;
    /// Return the inverse JᵀJ without noise or degrees-of-freedom scaling; see `approx_covariance_matrix_nalgebra_f64`.
    fn covariance(&self, f: T) -> Option<DMatrix<f64>>;
}

impl LMParameters<&dyn Fn(&DVector<f64>) -> DVector<f64>> for DVector<f64> {
    fn jacobian(&self, f: &dyn Fn(&DVector<f64>) -> DVector<f64>) -> DMatrix<f64> {
        forward_jacobian_nalgebra_f64(self, f)
    }

    fn hessian(&self, f: &dyn Fn(&DVector<f64>) -> DVector<f64>) -> DMatrix<f64> {
        approx_hessian_nalgebra_f64(self, f)
    }

    fn covariance(&self, f: &dyn Fn(&DVector<f64>) -> DVector<f64>) -> Option<DMatrix<f64>> {
        approx_covariance_matrix_nalgebra_f64(self, f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::xafs::tests::PARAM_LOADTXT;
    use crate::xafs::tests::TEST_TOL;
    use crate::xafs::tests::TOP_DIR;
    use approx::assert_abs_diff_eq;

    const NUM_DIFF_TOL: f64 = 1e-6;

    fn residuals(p: &DVector<f64>) -> DVector<f64> {
        let mut res = DVector::zeros(p.len());

        res.iter_mut()
            .enumerate()
            .for_each(|(i, x)| *x = p[i] - (i as f64 + 1.0));

        res
    }

    #[test]
    fn test_forward_jacobian_nalgebra_f64() {
        let x = DVector::from_vec(vec![1.0, 2.0, 3.0]);
        let fs = |x: &DVector<f64>| {
            let mut y = DVector::zeros(x.len());
            y[0] = 2.0 * x[0] + 3.0 * x[1] + 4.0 * x[2];
            y[1] = 3.0 * x[1] + 4.0 * x[2];
            y[2] = 4.0 * x[2] + 5.0 + x[1].exp();
            y
        };
        let jac = forward_jacobian_nalgebra_f64(&x, &fs);
        let jac_ref = DMatrix::from_vec(
            3,
            3,
            vec![2.0, 0.0, 0.0, 3.0, 3.0, 2.0_f64.exp(), 4.0, 4.0, 4.0],
        );

        assert_abs_diff_eq!(jac, jac_ref, epsilon = NUM_DIFF_TOL);
    }

    #[test]
    fn test_center_jacobian_nalgebra_f64() {
        let x = DVector::from_vec(vec![1.0, 2.0, 3.0]);
        let fs = |x: &DVector<f64>| {
            let mut y = DVector::zeros(x.len());
            y[0] = 2.0 * x[0] + 3.0 * x[1] + 4.0 * x[2];
            y[1] = 3.0 * x[1] + 4.0 * x[2];
            y[2] = 4.0 * x[2] + 5.0 + x[1].exp();
            y
        };
        let jac = center_jacobian_nalgebra_f64(&x, &fs);
        let jac_ref = DMatrix::from_vec(
            3,
            3,
            vec![2.0, 0.0, 0.0, 3.0, 3.0, 2.0_f64.exp(), 4.0, 4.0, 4.0],
        );

        assert_abs_diff_eq!(jac, jac_ref, epsilon = NUM_DIFF_TOL);
    }
}
