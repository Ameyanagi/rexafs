//! Projected, damped Gauss–Newton steps for the small composite model.
//!
//! This is a rexafs implementation, not an invocation of lmfit's solver. Bound
//! projection is included in the predicted step; outward-pointing variables at
//! active bounds are held while the remaining variables are refitted. Merely
//! clamping the parameters behind an unconstrained solver is not equivalent.
use super::{solver::Problem, PeakFitError, PeakTermination};
use nalgebra::{DMatrix, DVector};
use std::sync::atomic::Ordering;

pub(super) fn minimize(
    mut problem: Problem,
    iterations: usize,
    tolerance: f64,
    progress: &mut dyn FnMut(usize, f64) -> bool,
) -> Result<(Problem, PeakTermination, String, usize), PeakFitError> {
    let numerical = |s: &str| PeakFitError::Numerical(s.into());
    let mut residual = problem
        .residuals()
        .ok_or_else(|| numerical("invalid initial model"))?;
    let mut objective = residual.norm_squared();
    let mut damping = 1e-3_f64;
    let mut detail = "Iteration limit reached";
    let mut termination = PeakTermination::NotConverged;
    for iteration in 0..iterations {
        if !progress(iteration, objective) {
            detail = "Cancelled before next optimizer iteration";
            termination = PeakTermination::Cancelled;
            break;
        }
        let jac = problem
            .jacobian()
            .ok_or_else(|| numerical("invalid numerical Jacobian"))?;
        let gradient = jac.transpose() * &residual;
        let params = problem.params();
        let scales = DVector::from_iterator(
            params.len(),
            (0..params.len()).map(|i| jac.column(i).norm().max(1e-12)),
        );
        let free: Vec<_> = problem
            .names
            .iter()
            .enumerate()
            .filter_map(|(i, name)| {
                let p = &problem.variables.vars[name];
                let eps = 8. * f64::EPSILON * p.value.abs().max(1.);
                let outward = p
                    .min
                    .is_some_and(|b| p.value <= b + eps && gradient[i] > 0.)
                    || p.max
                        .is_some_and(|b| p.value >= b - eps && gradient[i] < 0.);
                (!outward).then_some(i)
            })
            .collect();
        let projected_gradient = free
            .iter()
            .map(|&i| gradient[i].abs() / scales[i])
            .fold(0., f64::max);
        if projected_gradient <= tolerance * residual.norm().max(1.) {
            detail = "Projected gradient converged";
            termination = PeakTermination::Converged;
            break;
        }
        // Column scaling gives damping comparable meaning for areas, energy,
        // widths and slopes. Solve an augmented least-squares problem directly
        // instead of squaring its condition number through normal equations.
        let mut lhs = DMatrix::zeros(jac.nrows() + free.len(), free.len());
        let mut rhs = DVector::zeros(lhs.nrows());
        rhs.rows_mut(0, residual.len()).copy_from(&(-&residual));
        for (column, &i) in free.iter().enumerate() {
            lhs.view_mut((0, column), (jac.nrows(), 1))
                .copy_from(&(jac.column(i) / scales[i]));
            lhs[(jac.nrows() + column, column)] = damping.sqrt();
        }
        let svd = lhs.svd(true, true);
        let step = svd
            .solve(&rhs, 1e-14)
            .map_err(|_| numerical("damped step is singular"))?;
        let mut proposed = params.clone();
        for (column, &i) in free.iter().enumerate() {
            proposed[i] += step[column] / scales[i];
        }
        if proposed.iter().any(|v| !v.is_finite()) {
            return Err(numerical("step overflow"));
        }
        let mut trial = problem.clone();
        trial.set_params(&proposed);
        let delta = trial.params() - &params;
        let predicted = objective - (&residual + &jac * &delta).norm_squared();
        let candidate = trial.residuals();
        let candidate_objective = candidate
            .as_ref()
            .map_or(f64::INFINITY, |r| r.norm_squared());
        let improvement = objective - candidate_objective;
        let ratio = improvement / predicted;
        if improvement >= 0. && predicted > 0. && ratio > 1e-4 {
            let relative_change = improvement / objective.max(f64::MIN_POSITIVE);
            let small_step = delta.component_mul(&scales).norm()
                <= tolerance * params.component_mul(&scales).norm().max(1.);
            problem = trial;
            residual = candidate.unwrap();
            objective = candidate_objective;
            if ratio > 0.75 {
                damping = (damping / 3.).max(1e-15);
            } else if ratio < 0.25 {
                damping = (damping * 4.).min(1e20);
            }
            // A small objective reduction alone is not a stationarity test.
            // Require a small actual (projected) step as well.
            if small_step && relative_change <= tolerance {
                detail = "Objective and projected step converged";
                termination = PeakTermination::Converged;
                break;
            }
        } else {
            damping = (damping * 10.).min(1e20);
            if damping >= 1e20 {
                // Numerical differentiation has a finite noise floor. Qualify
                // an otherwise stationary point using the square-root tolerance;
                // retain nonconvergence when the projected gradient is material.
                if projected_gradient <= tolerance.sqrt() * residual.norm().max(1.) {
                    detail = "Projected gradient reached numerical precision";
                    termination = PeakTermination::Converged;
                } else {
                    detail = "No improving feasible step";
                }
                break;
            }
        }
    }
    let evaluations = problem.evaluations.load(Ordering::Relaxed);
    Ok((problem, termination, detail.into(), evaluations))
}
