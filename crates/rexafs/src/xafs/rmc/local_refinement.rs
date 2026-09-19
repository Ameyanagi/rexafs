//! Bounded coordinate descent with numerical derivatives of the full calculator.
use super::*;
use std::ops::ControlFlow;

/// Since 0.2.11: controls for deterministic local refinement after RMC exploration.
/// This uses numerical derivatives, not automatic differentiation. Each block is
/// one movable atom's three Cartesian coordinates; all selected absorbers still
/// contribute. Full calculator evaluations include geometry-dependent amplitudes
/// and angles. With prepared ReFEFF, reference electronic potentials stay fixed.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct LocalRefinementSettings {
    /// Maximum passes over movable atoms, in stable index order; default 3.
    pub sweeps: usize,
    /// Per-coordinate finite-difference displacement in Å; default 0.001.
    /// Central differences are used where both probes satisfy the original hard
    /// constraints, otherwise a feasible one-sided difference is used.
    pub difference_step: f64,
    /// Maximum total displacement of an atom in one line search, in Å; default 0.02.
    pub maximum_step: f64,
    /// Stop halving a line-search step below this length, in Å; default 0.0001.
    pub minimum_step: f64,
    /// Maximum geometry evaluations, including the starting state; default 5000.
    /// One geometry evaluation can calculate many absorbers and datasets.
    pub evaluations: usize,
}
impl Default for LocalRefinementSettings {
    fn default() -> Self {
        Self {
            sweeps: 3,
            difference_step: 0.001,
            maximum_step: 0.02,
            minimum_step: 0.0001,
            evaluations: 5000,
        }
    }
}
impl LocalRefinementSettings {
    pub(super) fn validate(&self) -> Result<(), RmcError> {
        require(
            self.sweeps > 0
                && self.sweeps <= 10_000
                && self.evaluations > 0
                && self.evaluations <= 1_000_000
                && [self.difference_step, self.maximum_step, self.minimum_step]
                    .iter()
                    .all(|x| x.is_finite() && *x > 0. && *x <= 1.)
                && self.minimum_step <= self.maximum_step,
            "invalid local refinement budget or displacement (Å)",
        )
    }
}

/// Numerical termination reason; none establishes a unique physical structure.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum LocalRefinementStop {
    /// Completed the requested number of passes over the movable atoms.
    SweepLimit,
    /// Exhausted the number of allowed geometry evaluations.
    EvaluationLimit,
    /// One complete pass found no accepted descent within the tested step sizes.
    NoDescent,
    /// The progress callback requested cancellation at an evaluation boundary.
    Cancelled,
}
/// One accepted local move, verified using the full configured calculator.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LocalRefinementStep {
    /// One-based pass through movable atoms.
    pub sweep: usize,
    /// Zero-based mixture component and atom indices.
    pub structure: usize,
    /// Zero-based atom index within the component.
    pub atom: usize,
    /// Actual Cartesian displacement length in Å.
    pub displacement: f64,
    /// Total spectral objective plus structural penalty before this move.
    pub before: f64,
    /// Total objective after verification; strictly below `before`.
    pub after: f64,
}
/// Lightweight callback payload. Counts geometry evaluations, not Monte Carlo attempts.
#[derive(Debug, Clone, Copy)]
pub struct LocalRefinementProgress {
    /// Number of full geometry evaluations so far.
    pub evaluations: usize,
    /// One-based pass, or zero while verifying the starting state.
    pub sweep: usize,
    /// Lowest verified total objective so far.
    pub best_score: f64,
}
/// Owned local-refinement audit, separate from the original RMC checkpoint.
/// Calibration, mixture fractions and cell vectors remain fixed. The problem and
/// session settings retain the original displacement reference and constraints.
/// `best` is always a fully evaluated feasible state, including on cancellation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalRefinementResult {
    /// Fixed calculator scientific identity.
    pub calculator: String,
    /// Original RMC inputs, including the displacement-reference structure.
    pub problem: EnsembleProblem,
    /// Original constraints, coordinate selection and numerical settings.
    pub session_settings: SessionSettings,
    /// Local solver settings, copied exactly.
    pub settings: LocalRefinementSettings,
    /// RMC attempt from which local refinement was launched.
    pub source_attempt: usize,
    /// Re-evaluated starting best state; original scores are never blindly reused.
    pub initial: EnsembleState,
    /// Lowest verified feasible state found by this operation.
    pub best: EnsembleState,
    /// Accepted steps only, each verified against the full calculator.
    pub history: Vec<LocalRefinementStep>,
    /// Full geometry evaluations, including finite-difference probes and rejected trials.
    pub evaluations: usize,
    /// Why this local search stopped.
    pub stop: LocalRefinementStop,
}
impl LocalRefinementResult {
    /// Validate an imported audit's settings, topology, constraints and array sizes.
    /// This does not recompute scattering or certify the reported scientific fit.
    pub fn validate(&self) -> Result<(), RmcError> {
        self.settings.validate()?;
        super::session::prepare(&self.problem, &self.session_settings)?;
        require(
            self.evaluations > 0 && self.evaluations <= self.settings.evaluations,
            "invalid local refinement evaluation count",
        )?;
        for state in [&self.initial, &self.best] {
            super::session::validate_state(state, &self.problem, &self.session_settings)?;
            require(
                state.evaluation.score.is_finite()
                    && state.penalty.is_finite()
                    && state.evaluation.datasets.len() == self.problem.datasets.len()
                    && state
                        .evaluation
                        .datasets
                        .iter()
                        .zip(&self.problem.datasets)
                        .all(|(fit, data)| {
                            fit.score.is_finite()
                                && fit.chi.len() == data.exafs.k.len()
                                && fit.chi.iter().all(|x| x.is_finite())
                        }),
                "invalid local refinement spectrum or score",
            )?;
        }
        require(
            self.best.delta_e0 == self.initial.delta_e0
                && self.best.evaluation.score <= self.initial.evaluation.score
                && self
                    .best
                    .structures
                    .iter()
                    .zip(&self.initial.structures)
                    .all(|(a, b)| a.weight == b.weight)
                && self.history.iter().all(|h| {
                    h.before.is_finite()
                        && h.after.is_finite()
                        && h.after < h.before
                        && h.displacement.is_finite()
                        && h.displacement > 0.
                        && h.displacement <= self.settings.maximum_step
                        && h.sweep > 0
                        && h.sweep <= self.settings.sweeps
                        && self
                            .problem
                            .structures
                            .get(h.structure)
                            .is_some_and(|s| h.atom < s.configuration.atoms.len())
                }),
            "invalid local refinement descent history or mixture fractions",
        )
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn refine_local<C, F>(
    problem: &EnsembleProblem,
    session: &SessionSettings,
    start: &EnsembleState,
    source_attempt: usize,
    settings: &LocalRefinementSettings,
    calculator: &mut C,
    mut progress: F,
) -> Result<LocalRefinementResult, RmcError>
where
    C: ExafsCalculator + ?Sized,
    F: FnMut(&LocalRefinementProgress) -> ControlFlow<()>,
{
    settings.validate()?;
    let prepared = super::session::prepare(problem, session)?;
    require(
        session.constraints.allowed(
            &start.structures,
            &problem.structures,
            &session.moves,
            None,
        )?,
        "local refinement starts outside the original constraints",
    )?;
    let identity = calculator.identity();
    let initial = super::session::evaluate_state(problem, session, &prepared, start, calculator)?;
    let mut out = LocalRefinementResult {
        calculator: identity.clone(),
        problem: problem.clone(),
        session_settings: session.clone(),
        settings: settings.clone(),
        source_attempt,
        best: initial.clone(),
        initial,
        history: Vec::new(),
        evaluations: 1,
        stop: LocalRefinementStop::SweepLimit,
    };
    'search: for sweep in 1..=settings.sweeps {
        let before_sweep = out.history.len();
        for &(structure, atom) in &prepared.movable {
            let mut gradient = [0.; 3];
            for (axis, derivative) in gradient.iter_mut().enumerate() {
                let mut values = [None, None];
                for (index, sign) in [-1., 1.].into_iter().enumerate() {
                    if should_stop(&mut out, sweep, &mut progress) {
                        break 'search;
                    }
                    let mut trial = out.best.structures.clone();
                    trial[structure].configuration.atoms[atom].position[axis] +=
                        sign * settings.difference_step;
                    if !session.constraints.allowed(
                        &trial,
                        &problem.structures,
                        &session.moves,
                        Some((structure, atom)),
                    )? {
                        continue;
                    }
                    let evaluated = super::session::evaluate_prepared(
                        problem,
                        session,
                        &prepared,
                        trial,
                        Some((&out.best, Some(structure))),
                        calculator,
                    )?;
                    out.evaluations += 1;
                    values[index] = Some(evaluated.evaluation.score);
                }
                let base = out.best.evaluation.score;
                *derivative = match values {
                    [Some(lo), Some(hi)] => (hi - lo) / (2. * settings.difference_step),
                    [None, Some(hi)] => (hi - base) / settings.difference_step,
                    [Some(lo), None] => (base - lo) / settings.difference_step,
                    _ => 0.,
                };
            }
            let norm = gradient.iter().fold(0_f64, |n, &g| n.hypot(g));
            require(norm.is_finite(), "nonfinite local numerical gradient")?;
            if norm == 0. {
                continue;
            }
            let mut length = settings.maximum_step;
            while length >= settings.minimum_step {
                if should_stop(&mut out, sweep, &mut progress) {
                    break 'search;
                }
                let mut trial = out.best.structures.clone();
                for (axis, &g) in gradient.iter().enumerate() {
                    trial[structure].configuration.atoms[atom].position[axis] -=
                        length * (g / norm);
                }
                if session.constraints.allowed(
                    &trial,
                    &problem.structures,
                    &session.moves,
                    Some((structure, atom)),
                )? {
                    let evaluated = super::session::evaluate_prepared(
                        problem,
                        session,
                        &prepared,
                        trial,
                        Some((&out.best, Some(structure))),
                        calculator,
                    )?;
                    out.evaluations += 1;
                    let before = out.best.evaluation.score;
                    let after = evaluated.evaluation.score;
                    if after < before {
                        out.history.push(LocalRefinementStep {
                            sweep,
                            structure,
                            atom,
                            displacement: length,
                            before,
                            after,
                        });
                        out.best = evaluated;
                        break;
                    }
                }
                length *= 0.5;
            }
        }
        if out.history.len() == before_sweep {
            out.stop = LocalRefinementStop::NoDescent;
            break;
        }
    }
    require(
        identity == calculator.identity(),
        "calculator identity changed during local refinement",
    )?;
    Ok(out)
}
fn should_stop<F: FnMut(&LocalRefinementProgress) -> ControlFlow<()>>(
    out: &mut LocalRefinementResult,
    sweep: usize,
    progress: &mut F,
) -> bool {
    if progress(&LocalRefinementProgress {
        evaluations: out.evaluations,
        sweep,
        best_score: out.best.evaluation.score,
    })
    .is_break()
    {
        out.stop = LocalRefinementStop::Cancelled;
        true
    } else if out.evaluations >= out.settings.evaluations {
        out.stop = LocalRefinementStop::EvaluationLimit;
        true
    } else {
        false
    }
}
