//! Regression for issue #20. This exercises the legacy objective independently
//! of the fixed-penalty solver, for both array backends.
use super::*;

fn problem() -> AUTOBKSpline {
    let x: Vec<f64> = (0..9).map(|i| i as f64 * 1.5).collect();
    let y: Vec<f64> = x.iter().map(|v| 1.0 + 0.03 * v.cos()).collect();
    let (knots, coefs) = spline::interpolate(&x, &y, 3).unwrap();
    let k = DVector::from_iterator(241, (0..241).map(|i| i as f64 * 0.05));
    AUTOBKSpline {
        coefs: DVector::from_vec(coefs),
        knots: DVector::from_vec(knots),
        kraw: k.clone(),
        kout: k.clone(),
        mu: k.map(|v| 1.0 + 0.1 * (2.0 * v).sin()),
        chi_std: Some(k.map(|v| 0.01 * v.cos())),
        ftwin: k,
        nclamp: 3,
        clamp_lo: 2,
        clamp_hi: 1,
        irbkg: 35,
        ..Default::default()
    }
}

fn numerical_jacobian(p: &AUTOBKSpline, scale: Option<f64>) -> DMatrix<f64> {
    DMatrix::from_columns(
        &(0..p.coefs.len())
            .map(|j| {
                let mut plus = p.coefs.clone();
                let mut minus = p.coefs.clone();
                plus[j] += 1e-6;
                minus[j] -= 1e-6;
                (p.residual_vec_with_scale(&plus, scale) - p.residual_vec_with_scale(&minus, scale))
                    / 2e-6
            })
            .collect::<Vec<_>>(),
    )
}

#[test]
fn legacy_clamp_jacobian_matches_central_differences() {
    for nclamp in [0, 1, 3, 500] {
        for weights in [(0, 0), (0, 1), (2, 0), (2, 3)] {
            for irbkg in [0, 35] {
                let mut p = problem();
                p.nclamp = nclamp;
                (p.clamp_lo, p.clamp_hi) = weights;
                p.irbkg = irbkg;
                #[cfg(not(feature = "ndarray-compat"))]
                if nclamp == 3 {
                    p.ensure_precomputed_basis();
                }
                for scale in [None, Some(1.7)] {
                    let analytic = p.residual_jacobian_with_scale(&p.coefs, scale);
                    let numeric = numerical_jacobian(&p, scale);
                    let error = (&analytic - &numeric).norm() / numeric.norm().max(1.0);
                    assert!(
                        error < 1e-7,
                        "n={nclamp}, weights={weights:?}, R={irbkg}, scale={scale:?}: {error}"
                    );
                }
            }
        }
    }
}

#[derive(Clone)]
struct NumericProblem(AUTOBKSpline, bool);
impl LeastSquaresProblem<f64, Dyn, Dyn> for NumericProblem {
    type ParameterStorage = Owned<f64, Dyn>;
    type ResidualStorage = Owned<f64, Dyn>;
    type JacobianStorage = Owned<f64, Dyn, Dyn>;
    fn set_params(&mut self, p: &DVector<f64>) {
        self.0.coefs.copy_from(p);
    }
    fn params(&self) -> DVector<f64> {
        self.0.coefs.clone()
    }
    fn residuals(&self) -> Option<DVector<f64>> {
        Some(self.0.residual_vec(&self.0.coefs))
    }
    fn jacobian(&self) -> Option<DMatrix<f64>> {
        if self.1 {
            Some(self.0.residual_jacobian_with_scale(
                &self.0.coefs,
                Some(self.0.clamp_scale(&self.0.coefs)),
            ))
        } else {
            Some(numerical_jacobian(&self.0, None))
        }
    }
}

#[test]
fn legacy_lm_agrees_with_numerical_derivative_solution() {
    let solver = LevenbergMarquardt::new()
        .with_tol(1e-10)
        .with_patience(1000);
    let (analytic, report) = solver.minimize(problem());
    let (numeric, reference) = solver.minimize(NumericProblem(problem(), false));
    let (old, old_report) = solver.minimize(NumericProblem(problem(), true));
    let old_chi = old.0.chi_for_coefs(&old.0.coefs);
    let new_chi = analytic.chi_for_coefs(&analytic.coefs);
    println!("analytic: {report:?}; numerical: {reference:?}; omitted-chain-rule: {old_report:?}; old/new chi relative L2: {}", (&old_chi - &new_chi).norm()/new_chi.norm());
    assert!(report.termination.was_successful());
    assert!(reference.termination.was_successful());
    let a = analytic.chi_for_coefs(&analytic.coefs);
    let n = numeric.0.chi_for_coefs(&numeric.0.coefs);
    let error = (&a - &n).norm() / n.norm();
    assert!(error < 1e-6, "chi relative L2: {error}");
    assert!((report.objective_function - reference.objective_function).abs() < 1e-10);
}

#[test]
fn legacy_lm_does_not_return_nonfinite_fit_as_success() {
    let mut invalid = problem();
    invalid.mu[0] = f64::NAN;
    assert!(AUTOBK::solve_lm_problem(invalid).is_err());
}
