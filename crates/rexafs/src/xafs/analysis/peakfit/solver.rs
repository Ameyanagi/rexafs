use super::*;
use nalgebra::{DMatrix, DVector};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

/// Numerical termination; a returned curve is not by itself evidence of convergence.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PeakTermination {
    /// All parameters were fixed/derived; the requested model was evaluated.
    FixedModel,
    /// The optimizer met its convergence criterion.
    Converged,
    /// The optimizer stopped without satisfying its convergence criterion.
    NotConverged,
    /// The progress callback stopped optimization; the best accepted model remains.
    Cancelled,
}

/// One component's curve on the retained fitted points and model summaries.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PeakContribution {
    /// Stable component identity from the initial definition.
    pub name: String,
    /// Scientific role, independent of mathematical shape.
    pub role: PeakRole,
    /// Mathematical shape used for evaluation.
    pub shape: PeakShape,
    /// Component values at the result's absolute-energy points, in signal units.
    pub curve: Vec<f64>,
    /// Peak/step center in absolute eV; None for polynomial baselines.
    pub center_ev: Option<f64>,
    /// Whole-axis model area in signal units × eV; None for steps/polynomials.
    pub area: Option<f64>,
    /// Peak contribution at its center, excluding all other components.
    pub height: Option<f64>,
    /// Peak FWHM in eV; true Voigt uses a numerical half-height root.
    pub fwhm_ev: Option<f64>,
    /// Conditional errors propagated with the full joint covariance; absent when
    /// local uncertainty is unavailable or the quantity does not apply.
    pub center_standard_error_ev: Option<f64>,
    /// Conditional whole-axis area error, in signal units × eV.
    pub area_standard_error: Option<f64>,
    /// Conditional peak-height error, in signal units.
    pub height_standard_error: Option<f64>,
    /// Conditional FWHM error, in eV, including both true-Voigt width parameters.
    pub fwhm_standard_error_ev: Option<f64>,
    /// Trapezoidal component integral over included native-grid segments only.
    /// Masked gaps are not bridged; this is not its whole-axis analytic area.
    pub sampled_integral: f64,
}

/// Owned result, including the exact starting definition and fitted point set.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PeakFitResult {
    /// Original requested model/settings, unchanged by optimization.
    pub definition: PeakFit,
    /// Resolved energy origin in eV, added to parameter centers/reference energies.
    pub origin_ev: f64,
    /// Absolute energy in eV, only the native points used by this fit.
    pub energy: Vec<f64>,
    /// Original zero-based point indices; preserves masks and sampling provenance.
    pub source_indices: Vec<usize>,
    /// Selected representation's measured values, in its signal units.
    pub data: Vec<f64>,
    /// Joint baseline + peaks + steps, in the same signal units.
    pub model: Vec<f64>,
    /// Unweighted data minus model, in signal units (also for weighted fits).
    pub residual: Vec<f64>,
    /// Supplied selected-space standard deviations on the fitted points, if any.
    pub standard_deviation: Option<Vec<f64>>,
    /// Component curves and summaries; polynomials and steps are not peak areas.
    pub components: Vec<PeakContribution>,
    /// Final values with conditional local standard errors when available.
    pub parameters: FitVariables,
    /// Sum of squared residuals, divided by supplied standard deviations if present.
    pub objective: f64,
    /// Number of fitted native data points (not EXAFS independent-point estimates).
    pub points: usize,
    /// Number of independent varying parameters; expression ties are excluded.
    pub free_parameters: usize,
    /// points − free_parameters. Fits with fewer points than variables are rejected.
    pub degrees_of_freedom: usize,
    /// Weighted numerical Jacobian rank under a 1e-10 relative singular-value cutoff.
    pub jacobian_rank: usize,
    /// Sorted independent parameter names defining covariance/correlation axes.
    pub covariance_names: Vec<String>,
    /// Local covariance; absolute-error scaling when standard deviations were given,
    /// otherwise multiplied by objective/degrees_of_freedom.
    pub covariance: Option<Vec<Vec<f64>>>,
    /// Dimensionless correlations corresponding to covariance_names.
    pub correlation: Option<Vec<Vec<f64>>>,
    /// Why covariance/standard errors were withheld, rather than replaced by zero.
    pub uncertainty_unavailable: Option<String>,
    /// Model peak-area-weighted center in absolute eV, excluding baseline/steps.
    pub peak_center_ev: Option<f64>,
    /// Conditional error in that center, using full parameter covariance.
    pub peak_center_standard_error_ev: Option<f64>,
    /// Explicit numerical termination category.
    pub termination: PeakTermination,
    /// Solver-specific termination detail, retained verbatim for diagnosis.
    pub termination_detail: String,
    /// Number of residual-vector evaluations during optimization, including numerical derivatives.
    pub evaluations: usize,
    /// Active bounds and other model/uncertainty limitations.
    pub warnings: Vec<String>,
}

impl PeakFitResult {
    /// Copy the definition with fitted parameter values for explicit reuse or
    /// evaluation on another grid. Historical initial values in this result are
    /// untouched. This does not automatically warm-start subsequent batch frames.
    pub fn fitted_model(&self) -> PeakFit {
        let mut model = self.definition.clone();
        model.parameters = self.parameters.clone();
        for value in model.parameters.vars.values_mut() {
            value.init_value = value.value;
            value.stderr = None;
        }
        model
    }
}

#[derive(Clone)]
pub(super) struct Problem {
    definition: PeakFit,
    pub(super) variables: FitVariables,
    pub(super) names: Vec<String>,
    x: Vec<f64>,
    y: Vec<f64>,
    errors: Vec<f64>,
    pub(super) evaluations: Arc<AtomicUsize>,
}
impl Problem {
    fn resolved(&self) -> Result<BTreeMap<String, f64>, PeakFitError> {
        let values = self
            .variables
            .resolve_values()
            .map_err(|e| PeakFitError::Numerical(e.to_string()))?;
        self.definition.validate_values(&values)?;
        Ok(values)
    }
    fn curves(&self) -> Result<Vec<Vec<f64>>, PeakFitError> {
        let values = self.resolved()?;
        self.definition
            .components
            .iter()
            .map(|c| {
                let p = c.values(&values)?;
                self.x
                    .iter()
                    .map(|&x| {
                        let value = profiles::evaluate(c.shape, x, &p);
                        if value.is_finite() {
                            Ok(value)
                        } else {
                            Err(PeakFitError::Numerical(format!(
                                "nonfinite component {}",
                                c.name
                            )))
                        }
                    })
                    .collect()
            })
            .collect()
    }
    fn model(&self) -> Result<Vec<f64>, PeakFitError> {
        let mut result = vec![0.; self.x.len()];
        for curve in self.curves()? {
            for (sum, value) in result.iter_mut().zip(curve) {
                *sum += value;
            }
        }
        if result.iter().any(|v| !v.is_finite()) {
            return Err(PeakFitError::Numerical("nonfinite model sum".into()));
        }
        Ok(result)
    }
    fn shifted(&self, column: usize, delta: f64) -> Option<Self> {
        if !delta.is_finite() || delta == 0. {
            return None;
        }
        let mut candidate = self.clone();
        let parameter = candidate.variables.vars.get_mut(&self.names[column])?;
        parameter.value += delta;
        candidate.resolved().ok()?;
        Some(candidate)
    }
    fn steps(&self, column: usize) -> (f64, f64) {
        let p = &self.variables.vars[&self.names[column]];
        let step = f64::EPSILON.sqrt() * p.value.abs().max(1.);
        (
            p.max.map_or(step, |b| (b - p.value).min(step)).max(0.),
            p.min.map_or(step, |b| (p.value - b).min(step)).max(0.),
        )
    }
    fn local_covariance(&self, scale: f64) -> (usize, Option<DMatrix<f64>>) {
        let Some(jacobian) = self.jacobian() else {
            return (0, None);
        };
        if self.names.is_empty() {
            return (0, None);
        }
        let svd = jacobian.svd(false, true);
        let maximum = svd.singular_values.iter().copied().fold(0., f64::max);
        let rank = svd
            .singular_values
            .iter()
            .filter(|&&s| s > maximum * 1e-10 && s > 0.)
            .count();
        if rank != self.names.len() {
            return (rank, None);
        }
        let vt = svd.v_t.unwrap();
        let inverse = DMatrix::from_diagonal(&svd.singular_values.map(|s| scale / s.powi(2)));
        let covariance = vt.transpose() * inverse * vt;
        (
            rank,
            covariance
                .iter()
                .all(|v| v.is_finite())
                .then_some(covariance),
        )
    }
    fn propagated(
        &self,
        covariance: &DMatrix<f64>,
        f: impl Fn(&Self) -> Option<f64>,
    ) -> Option<f64> {
        let base = f(self)?;
        let mut gradient = DVector::zeros(self.names.len());
        for i in 0..self.names.len() {
            let (forward, backward) = self.steps(i);
            let plus = self.shifted(i, forward).and_then(|p| f(&p));
            let minus = self.shifted(i, -backward).and_then(|p| f(&p));
            gradient[i] = match (plus, minus) {
                (Some(a), Some(b)) => (a - b) / (forward + backward),
                (Some(a), None) => (a - base) / forward,
                (None, Some(b)) => (base - b) / backward,
                _ => return None,
            };
        }
        let variance = (gradient.transpose() * covariance * gradient)[(0, 0)];
        (variance.is_finite() && variance >= 0.).then(|| variance.sqrt())
    }
    fn peak_center(&self) -> Option<f64> {
        let values = self.resolved().ok()?;
        let (mut area, mut moment) = (0., 0.);
        for component in &self.definition.components {
            if component.role != PeakRole::Peak || !component.shape.is_peak() {
                continue;
            }
            let p = component.values(&values).ok()?;
            area += p[1];
            moment += p[1] * p[0];
        }
        if area <= f64::EPSILON || !area.is_finite() {
            None
        } else {
            let center = moment / area;
            center.is_finite().then_some(center)
        }
    }
}
impl Problem {
    pub(super) fn set_params(&mut self, values: &DVector<f64>) {
        let _ = self.variables.apply_parameter_vector(&self.names, values);
    }
    pub(super) fn params(&self) -> DVector<f64> {
        self.variables.parameter_vector(&self.names)
    }
    pub(super) fn residuals(&self) -> Option<DVector<f64>> {
        self.evaluations.fetch_add(1, Ordering::Relaxed);
        let model = self.model().ok()?;
        let result = DVector::from_iterator(
            self.x.len(),
            self.y
                .iter()
                .zip(model)
                .zip(&self.errors)
                .map(|((&data, model), error)| (data - model) / error),
        );
        result.iter().all(|v| v.is_finite()).then_some(result)
    }
    pub(super) fn jacobian(&self) -> Option<DMatrix<f64>> {
        let base = self.residuals()?;
        let mut jac = DMatrix::zeros(self.x.len(), self.names.len());
        for i in 0..self.names.len() {
            let (forward, backward) = self.steps(i);
            let plus = self.shifted(i, forward).and_then(|p| p.residuals());
            let minus = self.shifted(i, -backward).and_then(|p| p.residuals());
            let column = match (plus, minus) {
                (Some(a), Some(b)) => (a - b) / (forward + backward),
                (Some(a), None) => (a - &base) / forward,
                (None, Some(b)) => (&base - b) / backward,
                _ => return None,
            };
            jac.set_column(i, &column);
        }
        jac.iter().all(|v| v.is_finite()).then_some(jac)
    }
}

pub(super) fn fit(
    definition: &PeakFit,
    energy: &[f64],
    signal: &[f64],
    e0: Option<f64>,
    errors: Option<&[f64]>,
    progress: &mut dyn FnMut(usize, f64) -> bool,
) -> Result<PeakFitResult, PeakFitError> {
    definition.validate()?;
    let invalid = |s: &str| PeakFitError::Invalid(s.into());
    if energy.len() < 2
        || energy.len() != signal.len()
        || energy.iter().chain(signal).any(|v| !v.is_finite())
        || energy.windows(2).any(|w| w[0] >= w[1])
    {
        return Err(invalid(
            "need equal finite arrays on a strictly increasing energy grid",
        ));
    }
    if errors
        .is_some_and(|e| e.len() != energy.len() || e.iter().any(|v| !v.is_finite() || *v <= 0.))
    {
        return Err(invalid(
            "standard deviations must be finite, positive and match the native points",
        ));
    }
    let origin = match definition.origin {
        AxisOrigin::Absolute => 0.,
        AxisOrigin::E0 => {
            e0.ok_or_else(|| invalid("E₀ is required for relative centers and ranges"))?
        }
        AxisOrigin::Reference { energy_ev } => energy_ev,
    };
    if !origin.is_finite() {
        return Err(invalid("energy origin must be finite"));
    }
    let bounds = definition.range.map(|v| v + origin);
    if !bounds.iter().all(|v| v.is_finite())
        || bounds[0] < energy[0]
        || bounds[1] > *energy.last().unwrap()
    {
        return Err(invalid(
            "the full fit interval must be covered by the input energy grid",
        ));
    }
    let indices: Vec<_> = energy
        .iter()
        .enumerate()
        .filter_map(|(i, &e)| {
            (e >= bounds[0]
                && e <= bounds[1]
                && !definition
                    .exclude
                    .iter()
                    .any(|r| e - origin >= r[0] && e - origin <= r[1]))
            .then_some(i)
        })
        .collect();
    let names = definition.parameters.varying_names();
    if indices.len() < 2 || indices.len() <= names.len() {
        return Err(invalid("the unmasked fit interval needs more points than free parameters and at least two points"));
    }
    let problem = Problem {
        definition: definition.clone(),
        variables: definition.parameters.clone(),
        names,
        x: indices.iter().map(|&i| energy[i] - origin).collect(),
        y: indices.iter().map(|&i| signal[i]).collect(),
        evaluations: Arc::new(AtomicUsize::new(0)),
        errors: indices
            .iter()
            .map(|&i| errors.map_or(1., |v| v[i]))
            .collect(),
    };
    problem
        .residuals()
        .ok_or_else(|| PeakFitError::Numerical("initial residual is nonfinite".into()))?;
    let (solved, termination, detail, evaluations) = if problem.names.is_empty() {
        (
            problem,
            PeakTermination::FixedModel,
            "No independently varying parameters".into(),
            1,
        )
    } else {
        optimizer::minimize(
            problem,
            definition.max_iterations,
            definition.tolerance,
            progress,
        )?
    };
    let model = solved.model()?;
    let residual: Vec<_> = solved.y.iter().zip(&model).map(|(a, b)| a - b).collect();
    let weighted = solved
        .residuals()
        .ok_or_else(|| PeakFitError::Numerical("final residual is nonfinite".into()))?;
    let objective = weighted.dot(&weighted);
    if !objective.is_finite() {
        return Err(PeakFitError::Numerical("objective overflow".into()));
    }
    let dof = solved.x.len() - solved.names.len();
    let mut warnings = Vec::new();
    let values = solved.resolved()?;
    for (name, p) in &solved.variables.vars {
        if !p.vary && p.expr.is_none() {
            continue;
        }
        let value = values[name];
        let epsilon = 1e-7 * value.abs().max(1.);
        if p.min.is_some_and(|b| (value - b).abs() <= epsilon)
            || p.max.is_some_and(|b| (value - b).abs() <= epsilon)
        {
            warnings.push(format!("Active bound: {name}"));
        }
    }
    let (rank, mut covariance) = solved.local_covariance(if errors.is_some() {
        1.
    } else {
        objective / dof as f64
    });
    let unavailable = if termination == PeakTermination::Cancelled {
        Some("Fit was cancelled".into())
    } else if termination == PeakTermination::NotConverged {
        Some("Optimizer did not converge".into())
    } else if solved.names.is_empty() {
        Some("No independently varying parameters".into())
    } else if !warnings.is_empty() {
        Some("Local covariance is not reported at active parameter bounds".into())
    } else if covariance.is_none() {
        Some("Weighted Jacobian is rank deficient or numerically singular".into())
    } else {
        None
    };
    if unavailable.is_some() {
        covariance = None;
    }
    let mut parameters = solved.variables.clone();
    for (name, p) in &mut parameters.vars {
        p.value = values[name];
        p.stderr = covariance.as_ref().and_then(|cov| {
            if !p.vary && p.expr.is_none() {
                None
            } else {
                solved.propagated(cov, |problem| problem.resolved().ok()?.get(name).copied())
            }
        });
    }
    let curves = solved.curves()?;
    let components = definition
        .components
        .iter()
        .zip(curves)
        .map(|(c, curve)| {
            let p = c.values(&values)?;
            let mut sampled_integral = 0.;
            for i in 1..indices.len() {
                if indices[i] == indices[i - 1] + 1 {
                    sampled_integral +=
                        (solved.x[i] - solved.x[i - 1]) * (curve[i] + curve[i - 1]) / 2.;
                }
            }
            let propagate = |quantity: fn(PeakShape, &[f64]) -> Option<f64>| {
                covariance.as_ref().and_then(|cov| {
                    solved.propagated(cov, |problem| {
                        let p = c.values(&problem.resolved().ok()?).ok()?;
                        quantity(c.shape, &p)
                    })
                })
            };
            Ok(PeakContribution {
                name: c.name.clone(),
                role: c.role,
                shape: c.shape,
                center_ev: (c.shape.is_peak() || c.shape.is_step()).then(|| p[0] + origin),
                area: c.shape.is_peak().then(|| p[1]),
                height: c
                    .shape
                    .is_peak()
                    .then(|| profiles::evaluate(c.shape, p[0], &p)),
                fwhm_ev: profiles::fwhm(c.shape, &p),
                center_standard_error_ev: propagate(|shape, p| {
                    (shape.is_peak() || shape.is_step()).then_some(p[0])
                }),
                area_standard_error: propagate(|shape, p| shape.is_peak().then(|| p[1])),
                height_standard_error: propagate(|shape, p| {
                    shape.is_peak().then(|| profiles::evaluate(shape, p[0], p))
                }),
                fwhm_standard_error_ev: propagate(profiles::fwhm),
                sampled_integral,
                curve,
            })
        })
        .collect::<Result<Vec<_>, PeakFitError>>()?;
    let peak_center = solved.peak_center();
    if peak_center.is_none()
        && definition
            .components
            .iter()
            .any(|c| c.role == PeakRole::Peak)
    {
        warnings.push("Peak-area-weighted center unavailable: total peak area is too small".into());
    }
    let center_error = covariance
        .as_ref()
        .and_then(|cov| solved.propagated(cov, Problem::peak_center));
    let correlation = covariance.as_ref().map(|cov| {
        DMatrix::from_fn(cov.nrows(), cov.ncols(), |i, j| {
            let denominator = (cov[(i, i)] * cov[(j, j)]).sqrt();
            if denominator > 0. {
                cov[(i, j)] / denominator
            } else if i == j {
                1.
            } else {
                0.
            }
        })
    });
    let nested = |matrix: DMatrix<f64>| {
        (0..matrix.nrows())
            .map(|i| (0..matrix.ncols()).map(|j| matrix[(i, j)]).collect())
            .collect()
    };
    Ok(PeakFitResult {
        definition: definition.clone(),
        origin_ev: origin,
        energy: indices.iter().map(|&i| energy[i]).collect(),
        source_indices: indices,
        data: solved.y.clone(),
        model,
        residual,
        standard_deviation: errors.map(|_| solved.errors.clone()),
        components,
        parameters,
        objective,
        points: solved.x.len(),
        free_parameters: solved.names.len(),
        degrees_of_freedom: dof,
        jacobian_rank: rank,
        covariance_names: solved.names,
        covariance: covariance.map(nested),
        correlation: correlation.map(nested),
        uncertainty_unavailable: unavailable,
        peak_center_ev: peak_center.map(|v| v + origin),
        peak_center_standard_error_ev: center_error,
        termination,
        termination_detail: detail,
        evaluations,
        warnings,
    })
}
