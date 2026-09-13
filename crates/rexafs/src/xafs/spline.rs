//! Interpolating B-splines used by AUTOBK and FEFF path resampling.
//!
//! [`interpolate`] solves cubic coefficients from the B-spline collocation matrix
//! using QR factorization; linear coefficients equal the sampled values.
//! [`cubic_resample`] instead solves a tridiagonal system for first derivatives.
//! Both construct exact interpolants, without smoothing or statistical weights.
//! Default `FixedPenalty` AUTOBK and FEFF resampling extend the end polynomial
//! pieces. The legacy AUTOBK evaluator instead holds boundary values outside the
//! knot interval. Such coordinate clamping does not impose zero derivatives at
//! the endpoints (the different meaning of a "clamped" spline boundary condition).
//!
//! A spline is `S(q) = sum_j c[j] * B[j](q)`, where `q` is a query coordinate,
//! `c[j]` is coefficient `j`, and `B[j]` is a dimensionless B-spline basis function
//! determined by the knots and degree. Coefficients and `S` have the value units;
//! knots and query coordinates share coordinate units. Consequently the derivative
//! of `S` with respect to `c[j]` is simply `B[j]`, which [`coefficient_jacobian`]
//! evaluates analytically. This is not a derivative with respect to `q`.
//! The [SciPy B-spline reference](https://docs.scipy.org/doc/scipy/reference/generated/scipy.interpolate.BSpline.html)
//! gives the basis recurrence used by [`basis`].

use nalgebra::{DMatrix, DVector};

/// Interpolate paired `x`/`y` samples with degree 1 (linear) or 3 (cubic).
///
/// Cubic interpolation uses not-a-knot boundaries: the first two pieces, and the
/// last two pieces, share their cubic polynomial. See the
/// [SciPy boundary definition](https://docs.scipy.org/doc/scipy/reference/generated/scipy.interpolate.CubicSpline.html).
/// The output owns a knot vector of length `x.len() + degree + 1` and equally long
/// coefficients. Only the first `x.len()` coefficients are active; the trailing
/// zeros retain the historical layout expected by AUTOBK. Knots have the units
/// of `x`, and coefficients the units of `y`. Inputs are borrowed without changes.
///
/// Returns an error for unsupported degree, mismatched lengths, fewer than
/// `degree + 1` points, non-finite samples, non-increasing `x`, a singular cubic
/// system or non-finite coefficients. Small, distinct knot spacings may still
/// cause ill-conditioning; no condition-number threshold is enforced here.
pub(crate) fn interpolate(
    x: &[f64],
    y: &[f64],
    degree: usize,
) -> Result<(Vec<f64>, Vec<f64>), String> {
    if !matches!(degree, 1 | 3) || x.len() != y.len() || x.len() <= degree {
        return Err(format!(
            "spline degree {degree} requires matching arrays with more than {degree} points"
        ));
    }
    if x.iter().chain(y).any(|value| !value.is_finite())
        || x.windows(2).any(|pair| pair[0] >= pair[1])
    {
        return Err("spline inputs must be finite with strictly increasing coordinates".into());
    }

    let (knots, mut coefficients) = if degree == 1 {
        // A linear interpolant has the measured values as its B-spline
        // coefficients. This also handles two- and three-point FEFF grids.
        let mut knots = Vec::with_capacity(x.len() + 2);
        knots.push(x[0]);
        knots.extend_from_slice(x);
        knots.push(x[x.len() - 1]);
        (knots, y.to_vec())
    } else {
        // Repeated endpoints and omission of the second/penultimate data
        // points give the cubic not-a-knot interpolant used by the pipeline.
        let mut knots = Vec::with_capacity(x.len() + degree + 1);
        knots.extend(std::iter::repeat_n(x[0], degree + 1));
        knots.extend_from_slice(&x[2..x.len() - 2]);
        knots.extend(std::iter::repeat_n(x[x.len() - 1], degree + 1));
        let matrix = coefficient_jacobian(&knots, x.len(), degree, x, false);
        let coefficients = matrix
            .qr()
            .solve(&DVector::from_column_slice(y))
            .ok_or("spline interpolation matrix is singular")?;
        (knots, coefficients.as_slice().to_vec())
    };
    coefficients.resize(knots.len(), 0.0);
    if coefficients.iter().any(|value| !value.is_finite()) {
        return Err("spline interpolation produced non-finite coefficients".into());
    }
    Ok((knots, coefficients))
}

/// Build the not-a-knot cubic knot vector without fitting coefficients.
///
/// Repeats each endpoint four times and omits the second and penultimate sample
/// coordinates from the interior knots, giving `x.len() + 4` entries in the units
/// of `x`. Returns an error unless there are at least four finite, strictly
/// increasing points. AUTOBK's direct solver uses this when an initial interpolant
/// is unnecessary; [`interpolate`] describes the boundary convention.
pub(crate) fn cubic_knots(x: &[f64]) -> Result<Vec<f64>, String> {
    if x.len() < 4 || x.iter().any(|v| !v.is_finite()) || x.windows(2).any(|w| w[0] >= w[1]) {
        return Err("cubic knots require at least four finite, strictly increasing points".into());
    }
    let mut knots = vec![x[0]; 4];
    knots.extend_from_slice(&x[2..x.len() - 2]);
    knots.extend(std::iter::repeat_n(x[x.len() - 1], 4));
    Ok(knots)
}

/// Resample a not-a-knot cubic interpolant, extrapolating the end polynomials.
///
/// Solves the tridiagonal first-derivative system in O(n) time and memory for
/// `n = x.len()`, then locates each of `m = query.len()` points by binary search.
/// Total time is O(n + m log n). Each located interval is evaluated as a Hermite
/// cubic, specified by its endpoint values and first derivatives. This avoids a
/// dense collocation matrix while representing the same cubic as [`interpolate`].
/// See SciPy CubicSpline's not-a-knot boundary equations:
/// <https://github.com/scipy/scipy/blob/v1.17.1/scipy/interpolate/_cubic.py>.
///
/// Returns a new vector in query order, with the units of `y`. Queries may be
/// unsorted or empty; all coordinates use the units of `x`. Extrapolation can
/// overshoot the measured values and is not constrained to remain positive.
/// Returns an error for fewer than four samples, mismatched lengths, non-finite
/// samples/queries, non-increasing `x`, or non-finite derivatives/results.
/// Unlike SciPy's general constructor, this helper does not special-case grids
/// with two or three samples; callers must select linear interpolation there.
pub(crate) fn cubic_resample(x: &[f64], y: &[f64], query: &[f64]) -> Result<Vec<f64>, String> {
    if x.len() < 4
        || x.len() != y.len()
        || x.iter().chain(y).chain(query).any(|v| !v.is_finite())
        || x.windows(2).any(|w| w[0] >= w[1])
    {
        return Err("cubic interpolation requires at least four finite, strictly increasing points and matching values".into());
    }
    let n = x.len();
    let h: Vec<_> = x.windows(2).map(|w| w[1] - w[0]).collect();
    let d: Vec<_> = y
        .windows(2)
        .zip(&h)
        .map(|(w, h)| (w[1] - w[0]) / h)
        .collect();
    let mut lower = vec![0.0; n];
    let mut diag = vec![0.0; n];
    let mut upper = vec![0.0; n];
    let mut deriv = vec![0.0; n];
    for i in 1..n - 1 {
        lower[i] = h[i];
        diag[i] = 2.0 * (h[i - 1] + h[i]);
        upper[i] = h[i - 1];
        deriv[i] = 3.0 * (h[i] * d[i - 1] + h[i - 1] * d[i]);
    }
    let span = h[0] + h[1];
    diag[0] = h[1];
    upper[0] = span;
    deriv[0] = ((h[0] + 2.0 * span) * h[1] * d[0] + h[0] * h[0] * d[1]) / span;
    let span = h[n - 3] + h[n - 2];
    lower[n - 1] = span;
    diag[n - 1] = h[n - 3];
    deriv[n - 1] =
        (h[n - 2] * h[n - 2] * d[n - 3] + (2.0 * span + h[n - 2]) * h[n - 3] * d[n - 2]) / span;
    for i in 1..n {
        let ratio = lower[i] / diag[i - 1];
        diag[i] -= ratio * upper[i - 1];
        deriv[i] -= ratio * deriv[i - 1];
    }
    deriv[n - 1] /= diag[n - 1];
    for i in (0..n - 1).rev() {
        deriv[i] = (deriv[i] - upper[i] * deriv[i + 1]) / diag[i];
    }
    let values: Vec<_> = query
        .iter()
        .map(|&q| {
            let i = x.partition_point(|&v| v <= q).saturating_sub(1).min(n - 2);
            let t = (q - x[i]) / h[i];
            let a = (deriv[i] + deriv[i + 1] - 2.0 * d[i]) / h[i];
            let b = 3.0 * d[i] - 2.0 * deriv[i] - deriv[i + 1];
            y[i] + t * h[i] * (deriv[i] + t * (b + t * h[i] * a))
        })
        .collect();
    if deriv.iter().chain(&values).any(|v| !v.is_finite()) {
        return Err("cubic interpolation produced non-finite values".into());
    }
    Ok(values)
}

/// Return the first active coefficient index and up to `degree + 1` basis weights.
///
/// Uses the B-spline recurrence described in the module documentation, assigning
/// zero to a term whose knot-span denominator vanishes. Only the first
/// `degree + 1` entries of the six-element workspace are meaningful. With
/// `count = knots.len() - degree - 1`, `clamp` clips `x` to the base interval
/// `knots[degree]..=knots[count]`; otherwise the span stays in that interval but
/// `x` does not, extending its end polynomial pieces. Weights are dimensionless.
///
/// Callers must supply finite, nondecreasing knots and a finite query. Panics when
/// `degree > 5`, there are fewer than `2 * (degree + 1)` knots, or clamping sees an
/// invalid base interval. This low-level helper does not validate the full geometry.
fn basis(knots: &[f64], degree: usize, x: f64, clamp: bool) -> (usize, [f64; 6]) {
    assert!(degree <= 5 && knots.len() >= 2 * (degree + 1));
    let count = knots.len() - degree - 1;
    let x = if clamp {
        x.clamp(knots[degree], knots[count])
    } else {
        x
    };
    let span = knots
        .partition_point(|&knot| knot <= x)
        .saturating_sub(1)
        .clamp(degree, count - 1);
    let mut weights = [0.0; 6];
    let mut left = [0.0; 6];
    let mut right = [0.0; 6];
    weights[0] = 1.0;
    for order in 1..=degree {
        left[order] = x - knots[span + 1 - order];
        right[order] = knots[span + order] - x;
        let mut carried = 0.0;
        for index in 0..order {
            let width = right[index + 1] + left[order - index];
            let term = if width == 0.0 {
                0.0
            } else {
                weights[index] / width
            };
            weights[index] = carried + right[index + 1] * term;
            carried = left[order - index] * term;
        }
        weights[order] = carried;
    }
    (span - degree, weights)
}

/// Evaluate the spline on borrowed query coordinates, allocating one value per query.
///
/// The sum and units are defined in the module documentation. `clamp` holds the
/// endpoint value outside the base interval; false extends the end polynomials.
/// Query order is preserved and empty queries produce an empty vector. Missing
/// coefficients contribute zero; extra historical padding coefficients are ignored.
/// No finiteness check is performed, and invalid geometry can panic as described
/// in [`basis`]. Prefer knot vectors validated by [`interpolate`] or [`cubic_knots`].
pub(crate) fn evaluate(
    knots: &[f64],
    coefficients: &[f64],
    degree: usize,
    x: &[f64],
    clamp: bool,
) -> Vec<f64> {
    x.iter()
        .map(|&point| {
            let (start, weights) = basis(knots, degree, point, clamp);
            weights[..=degree]
                .iter()
                .enumerate()
                .map(|(index, weight)| {
                    weight * coefficients.get(start + index).copied().unwrap_or(0.0)
                })
                .sum()
        })
        .collect()
}

/// Allocate the derivative of spline values with respect to their coefficients.
///
/// Matrix entry `(i, j)` is the dimensionless basis weight `B[j](x[i])`. Rows follow
/// query order; there are exactly `coefficient_count` columns. The result does not
/// depend on coefficient values. Columns beyond the active basis remain zero;
/// a smaller requested count omits later basis functions. `clamp` and geometry
/// requirements match [`evaluate`]. Empty queries give a zero-row matrix.
pub(crate) fn coefficient_jacobian(
    knots: &[f64],
    coefficient_count: usize,
    degree: usize,
    x: &[f64],
    clamp: bool,
) -> DMatrix<f64> {
    let mut matrix = DMatrix::zeros(x.len(), coefficient_count);
    for (row, &point) in x.iter().enumerate() {
        let (start, weights) = basis(knots, degree, point, clamp);
        for (offset, &weight) in weights[..=degree].iter().enumerate() {
            if start + offset < coefficient_count {
                matrix[(row, start + offset)] = weight;
            }
        }
    }
    matrix
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;

    #[test]
    fn cubic_polynomial_is_reproduced_including_extrapolation() {
        let x: Vec<f64> = vec![-2.0, -1.2, 0.0, 0.4, 2.1, 3.0, 7.0];
        let polynomial = |x: f64| 0.2 * x.powi(3) - 2.0 * x + 1.0;
        let y: Vec<_> = x.iter().map(|&x| polynomial(x)).collect();
        let (knots, coefficients) = interpolate(&x, &y, 3).unwrap();
        let query = [8.0, -3.0, 0.5, 7.0, -2.0, 2.1];
        for (&x, actual) in query
            .iter()
            .zip(evaluate(&knots, &coefficients, 3, &query, false))
        {
            assert_abs_diff_eq!(actual, polynomial(x), epsilon = 1e-11);
        }
        let clamped = evaluate(&knots, &coefficients, 3, &query[..2], true);
        assert_abs_diff_eq!(clamped[0], polynomial(7.0), epsilon = 1e-11);
        assert_abs_diff_eq!(clamped[1], polynomial(-2.0), epsilon = 1e-11);
    }

    #[test]
    fn banded_cubic_matches_independent_collocation_on_irregular_grids() {
        for n in [4, 5, 8, 31, 120] {
            let x: Vec<_> = (0..n)
                .map(|i| (i as f64 / n as f64).powi(3) * 20.0)
                .collect();
            let y: Vec<_> = x.iter().map(|x| x.sin() + 0.1 * x * x).collect();
            let query: Vec<_> = (-1..=100).map(|i| i as f64 * x[n - 1] / 100.0).collect();
            let (knots, coefs) = interpolate(&x, &y, 3).unwrap();
            let expected = evaluate(&knots, &coefs, 3, &query, false);
            let actual = cubic_resample(&x, &y, &query).unwrap();
            for (a, b) in actual.iter().zip(expected) {
                assert_abs_diff_eq!(*a, b, epsilon = 2e-8);
            }
        }
    }

    #[test]
    fn linear_short_grids_and_empty_queries() {
        for x in [vec![1.0, 4.0], vec![1.0, 2.0, 4.0]] {
            let y: Vec<_> = x.iter().map(|x| 3.0 * x - 2.0).collect();
            let (knots, coefficients) = interpolate(&x, &y, 1).unwrap();
            let query = [0.0, 1.0, 2.5, 4.0, 5.0];
            for (actual, expected) in evaluate(&knots, &coefficients, 1, &query, false)
                .into_iter()
                .zip([-2.0, 1.0, 5.5, 10.0, 13.0])
            {
                assert_abs_diff_eq!(actual, expected, epsilon = 4e-15);
            }
            assert!(evaluate(&knots, &coefficients, 1, &[], true).is_empty());
            assert_eq!(
                coefficient_jacobian(&knots, coefficients.len(), 1, &[], true).nrows(),
                0
            );
        }
    }

    #[test]
    fn invalid_interpolation_data_returns_errors() {
        for (x, y, degree) in [
            (vec![], vec![], 3),
            (vec![0.0, 1.0], vec![1.0], 1),
            (vec![1.0, 1.0], vec![0.0, 1.0], 1),
            (vec![1.0, 0.0], vec![0.0, 1.0], 1),
            (vec![0.0, f64::NAN], vec![0.0, 1.0], 1),
            (vec![0.0, 1.0], vec![0.0, f64::INFINITY], 1),
            (vec![0.0, 1.0, 2.0], vec![0.0, 1.0, 2.0], 3),
        ] {
            assert!(interpolate(&x, &y, degree).is_err());
        }
    }

    #[test]
    fn interpolant_and_coefficient_derivatives_match_scipy() {
        #[derive(serde::Deserialize)]
        struct Reference {
            x: Vec<f64>,
            y: Vec<f64>,
            query: Vec<f64>,
            knots: Vec<f64>,
            coefficients: Vec<f64>,
            extrapolated: Vec<f64>,
            clamped: Vec<f64>,
            extrapolated_basis: Vec<Vec<f64>>,
            clamped_basis: Vec<Vec<f64>>,
        }
        let data: Reference = serde_json::from_str(include_str!(
            "../../tests/testfiles/spline_scipy_reference.json"
        ))
        .unwrap();
        let resampled = cubic_resample(&data.x, &data.y, &data.query).unwrap();
        for (&actual, &expected) in resampled.iter().zip(&data.extrapolated) {
            assert_abs_diff_eq!(actual, expected, epsilon = 2e-11);
        }
        let (knots, coefficients) = interpolate(&data.x, &data.y, 3).unwrap();
        assert_eq!(knots, data.knots);
        for (&actual, &expected) in coefficients.iter().zip(&data.coefficients) {
            assert_abs_diff_eq!(actual, expected, epsilon = 2e-12);
        }
        for (clamp, expected, expected_basis) in [
            (false, &data.extrapolated, &data.extrapolated_basis),
            (true, &data.clamped, &data.clamped_basis),
        ] {
            for (actual, &expected) in evaluate(&knots, &coefficients, 3, &data.query, clamp)
                .iter()
                .zip(expected)
            {
                assert_abs_diff_eq!(*actual, expected, epsilon = 2e-11);
            }
            let jacobian = coefficient_jacobian(&knots, coefficients.len(), 3, &data.query, clamp);
            for row in 0..jacobian.nrows() {
                for col in 0..jacobian.ncols() {
                    assert_abs_diff_eq!(
                        jacobian[(row, col)],
                        expected_basis[row][col],
                        epsilon = 2e-12
                    );
                }
                assert_abs_diff_eq!(jacobian.row(row).sum(), 1.0, epsilon = 2e-12);
            }
        }
    }
}
