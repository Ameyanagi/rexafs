//! Bounded, fixed-amplitude energy searches between RMC coordinate blocks.
use super::*;

/// Since 0.2.11: optional theoretical ΔE₀ optimization in [`RmcSession`].
///
/// S₀² remains fixed at each input dataset's value. Each dataset has one independent
/// shift shared by all its absorbers and mixture components. Measured energies,
/// normalization E₀ and the fitting window are unchanged. Positive ΔE₀ samples
/// smaller theoretical k. Bounds must retain valid theoretical k over the entire
/// input grid, including the Fourier taper; unavailable backend support is an error.
///
/// The initial search covers `bounds` and then refines near its minimum. Subsequent
/// searches cover `radius` around the current shift. The current value is always
/// included. Only a decrease verified with the full objective is accepted. This
/// alternating optimization is a rexafs policy, not equilibrium Monte Carlo or
/// a parameter-uncertainty estimate. ΔE₀ and bond distances can be correlated.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct EnergyRefinement {
    /// Inclusive shift limits in eV; default −15 to +15. Review for your experiment.
    pub bounds: [f64; 2],
    /// Coordinate/weight attempts between updates, including rejected attempts;
    /// default 250. Initial refinement also runs before the first attempt.
    pub interval: usize,
    /// Broad initial grid spacing in eV; default 0.5.
    pub initial_step: f64,
    /// Half-width of a local search in eV; default 1. Bounds clip this interval.
    pub radius: f64,
    /// Local grid spacing in eV; default 0.1. This is resolution, not uncertainty.
    pub step: f64,
}
impl Default for EnergyRefinement {
    fn default() -> Self {
        Self {
            bounds: [-15., 15.],
            interval: 250,
            initial_step: 0.5,
            radius: 1.,
            step: 0.1,
        }
    }
}
impl EnergyRefinement {
    /// Check finite ordered bounds, positive interval/spacing, and bounded work.
    /// Each search permits at most 1001 grid points, plus the current shift.
    /// Dataset and calculator k coverage is additionally checked when used.
    pub fn validate(&self) -> Result<(), RmcError> {
        require(
            self.bounds.iter().all(|v| v.is_finite())
                && self.bounds[0] < self.bounds[1]
                && self.interval > 0
                && [self.initial_step, self.radius, self.step]
                    .iter()
                    .all(|v| v.is_finite() && *v > 0.)
                && (self.bounds[1] - self.bounds[0]) / self.initial_step <= 1000.
                && (2. * self.radius).min(self.bounds[1] - self.bounds[0]) / self.step <= 1000.,
            "invalid ΔE₀ refinement bounds, interval or grid spacing",
        )
    }
    pub(super) fn validate_problem(&self, problem: &EnsembleProblem) -> Result<(), RmcError> {
        self.validate()?;
        for dataset in &problem.datasets {
            require(
                self.bounds[0] <= dataset.exafs.delta_e0
                    && dataset.exafs.delta_e0 <= self.bounds[1],
                "initial ΔE₀ lies outside its refinement bounds",
            )?;
            for shift in self.bounds {
                let mut data = dataset.exafs.clone();
                data.delta_e0 = shift;
                super::engine::shifted_grid(&data)?;
            }
        }
        Ok(())
    }
    fn grid(&self, center: f64, initial: bool) -> Vec<f64> {
        let [lo, hi] = if initial {
            self.bounds
        } else {
            [
                (center - self.radius).max(self.bounds[0]),
                (center + self.radius).min(self.bounds[1]),
            ]
        };
        let step = if initial {
            self.initial_step
        } else {
            self.step
        };
        // Add the upper endpoint separately. An inclusive rounded final step
        // can otherwise produce an almost-duplicate endpoint beyond the cap.
        let count = ((hi - lo) / step).ceil() as usize;
        let mut grid: Vec<_> = (0..count).map(|i| (lo + i as f64 * step).min(hi)).collect();
        grid.extend([center, hi]);
        grid.sort_by(f64::total_cmp);
        grid.dedup();
        grid
    }
}

/// Audit of one fixed-geometry energy update (since 0.2.11). Includes unsuccessful
/// searches; `before == after` means the verified objective did not improve.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EnergyUpdate {
    /// Calculator identity used for this update. An initial update may precede
    /// a subsequently recorded explicit calculator rebase.
    pub calculator: String,
    /// Per-dataset theoretical shifts before the search, in eV.
    pub before: Vec<f64>,
    /// Per-dataset shifts actually retained, in eV.
    pub after: Vec<f64>,
    /// Full objective including structural penalties before the search.
    pub score_before: f64,
    /// Full verified objective actually retained; no larger than `score_before`.
    pub score_after: f64,
    /// Number of tested energy candidates for each dataset, including both
    /// stages for the initial update. Does not count verification evaluations.
    pub candidates: Vec<usize>,
    /// Per-dataset flags that the retained shift touches a global bound.
    pub at_bound: Vec<bool>,
}

impl EnergyUpdate {
    pub(super) fn validate(
        &self,
        policy: &EnergyRefinement,
        datasets: usize,
    ) -> Result<(), RmcError> {
        require(
            !self.calculator.is_empty()
                && self.before.len() == datasets
                && self.after.len() == datasets
                && self.candidates.len() == datasets
                && self.at_bound.len() == datasets
                && self
                    .before
                    .iter()
                    .chain(&self.after)
                    .all(|v| v.is_finite() && *v >= policy.bounds[0] && *v <= policy.bounds[1])
                && self.candidates.iter().all(|v| *v > 0 && *v <= 2004)
                && self
                    .after
                    .iter()
                    .zip(&self.at_bound)
                    .all(|(v, bound)| policy.bounds.contains(v) == *bound)
                && self.score_before.is_finite()
                && self.score_after.is_finite()
                && self.score_after <= self.score_before,
            "invalid saved ΔE₀ update",
        )
    }
}

pub(super) fn refine_energy<C: ExafsCalculator + ?Sized>(
    problem: &EnsembleProblem,
    settings: &SessionSettings,
    prepared: &super::session::PreparedEnsemble,
    start: &EnsembleState,
    initial: bool,
    calculator: &mut C,
) -> Result<(EnsembleState, EnergyUpdate), RmcError> {
    let identity = calculator.identity();
    let policy = settings
        .energy_refinement
        .as_ref()
        .ok_or_else(|| RmcError::Invalid("ΔE₀ refinement is disabled".into()))?;
    let before = start.energy_shifts(problem)?;
    let mut shifts = before.clone();
    let mut fixed = problem.clone();
    fixed.structures = start.structures.clone();
    let mut counts = Vec::new();
    for (index, &center) in before.iter().enumerate() {
        let amplitude = fixed.datasets[index].exafs.s02;
        let mut best_shift = center;
        let mut best_score = start.evaluation.datasets[index].score;
        let mut count = 0;
        for broad in [true, false].into_iter().filter(|broad| initial || !broad) {
            let grid = policy.grid(best_shift, broad);
            count += grid.len();
            let result = calibrate_dataset(
                &fixed,
                index,
                &CalibrationSettings {
                    delta_e0: grid,
                    s02_bounds: [amplitude, amplitude],
                },
                calculator,
            )?;
            if result.best.score < best_score {
                best_score = result.best.score;
                best_shift = result.best.delta_e0;
            }
        }
        shifts[index] = best_shift;
        counts.push(count);
    }
    // Recompute component arrays and optional paths at the selected grids. Never
    // reuse arrays sampled at an older shift, or trust the union-grid score alone.
    let verified = super::session::evaluate_at_shifts(
        problem,
        settings,
        prepared,
        start.structures.clone(),
        &shifts,
        None,
        calculator,
    )?;
    let best = if verified.evaluation.score < start.evaluation.score {
        verified
    } else {
        start.clone()
    };
    let after = best.energy_shifts(problem)?;
    let update = EnergyUpdate {
        calculator: identity.clone(),
        at_bound: after.iter().map(|v| policy.bounds.contains(v)).collect(),
        before,
        after,
        score_before: start.evaluation.score,
        score_after: best.evaluation.score,
        candidates: counts,
    };
    require(
        calculator.identity() == identity,
        "calculator identity changed during ΔE₀ refinement",
    )?;
    Ok((best, update))
}
