use super::objective::PreparedObjective;
use super::*;
use rand::{RngExt, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::collections::HashSet;

/// Extended session settings. The default is coordinate-only RMC, with bounded
/// history and no coordinate/path output. All values are copied into checkpoints.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct SessionSettings {
    /// Proposal scale, seed, hard-distance fallback, tolerance and total step limit.
    pub moves: RmcSettings,
    /// Additional hard rules and structural energies.
    pub constraints: Constraints,
    /// Probability of transferring mixture weight between two components; default 0.
    pub weight_move_probability: f64,
    /// Maximum absolute fraction transferred per proposal; default 0.05.
    /// Proposals crossing zero are rejected without clipping or renormalizing.
    pub weight_step: f64,
    /// Save coordinates every this many attempts; zero (default) disables trajectories.
    pub trajectory_stride: usize,
    /// Retain at most this many recent trajectory frames; default 1000.
    pub trajectory_capacity: usize,
    /// Retain at most this many recent move records; default 10000. Zero disables history.
    pub history_capacity: usize,
    /// Include individual scattering paths in states; default false to limit memory.
    pub retain_paths: bool,
    /// Fixed per-element and collective coordinate proposal mixture.
    pub proposals: ProposalSettings,
    /// Predetermined numerical annealing; constant by default.
    pub cooling: CoolingSchedule,
    /// Optional numerical stopping rules; all disabled by default.
    pub stopping: StoppingSettings,
    /// Optional acceptance-based coordinate widths; disabled by default (since 0.2.10).
    pub adaptation: Option<StepAdaptation>,
    /// Since 0.2.11: bounded theoretical ΔE₀ searches before the first move and
    /// between coordinate blocks. None (default) keeps shifts fixed. S₀² and
    /// experimental preprocessing always stay fixed. See [`EnergyRefinement`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub energy_refinement: Option<EnergyRefinement>,
}
impl Default for SessionSettings {
    fn default() -> Self {
        Self {
            moves: RmcSettings::default(),
            constraints: Constraints::default(),
            weight_move_probability: 0.,
            weight_step: 0.05,
            trajectory_stride: 0,
            trajectory_capacity: 1000,
            history_capacity: 10000,
            retain_paths: false,
            proposals: ProposalSettings::default(),
            cooling: CoolingSchedule::default(),
            stopping: StoppingSettings::default(),
            adaptation: None,
            energy_refinement: None,
        }
    }
}

impl SessionSettings {
    /// Since 0.2.11: refine each dataset's theoretical ΔE₀ with S₀² fixed.
    /// The inclusive bounds are in eV. Uses the documented [`EnergyRefinement`]
    /// defaults (250 attempts between updates, 0.1 eV local grid). Review bounds
    /// and timing for your experiment; validation occurs at session creation.
    /// Existing fixed-energy sessions are unchanged unless this is selected.
    ///
    /// ```
    /// use rexafs::rmc::SessionSettings;
    /// let settings = SessionSettings::default().with_energy_refinement(-10.0..=10.0);
    /// ```
    pub fn with_energy_refinement(mut self, bounds: std::ops::RangeInclusive<f64>) -> Self {
        self.energy_refinement = Some(EnergyRefinement {
            bounds: [*bounds.start(), *bounds.end()],
            ..Default::default()
        });
        self
    }

    /// Since 0.2.11: enable bounded move-size feedback and cooling for optimization.
    ///
    /// Starts at `moves.step_size` Å per Cartesian coordinate, adjusts every 100
    /// coordinate attempts, and bounds all proposal widths to 0.1–2 times their
    /// declared values. Widths shrink below 10% acceptance and grow above 40%, by
    /// a factor of 1.2. Feedback freezes after 80% of the original attempt budget;
    /// the numerical Metropolis tolerance decreases linearly to zero at that
    /// budget. Continuing a completed run retains the frozen widths and zero
    /// tolerance. Original displacement and distance constraints still apply.
    ///
    /// These are rexafs starting heuristics, not material-specific optimal values
    /// or an equilibrium sampling policy. Compare residual improvement per unit
    /// time across seeds. This replaces `adaptation` and `cooling`, changes no
    /// calibration, and leaves the historical `Default` behavior unchanged.
    ///
    /// ```
    /// use rexafs::rmc::{RmcSettings, SessionSettings};
    /// let settings = SessionSettings {
    ///     moves: RmcSettings { steps: 10_000, step_size: 0.05,
    ///         temperature: 0.001, ..Default::default() },
    ///     ..Default::default()
    /// }.with_auto_moves();
    /// ```
    pub fn with_auto_moves(mut self) -> Self {
        self.adaptation = Some(StepAdaptation {
            factor: 1.2,
            minimum_scale: 0.1,
            maximum_scale: 2.,
            freeze_after: Some(self.moves.steps.saturating_mul(4) / 5),
            ..Default::default()
        });
        self.cooling = CoolingSchedule::Linear {
            final_temperature: 0.,
            attempts: self.moves.steps.max(1),
        };
        self
    }
}

/// A symmetric proposal on coordinates or normalized mixture fractions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EnsembleMove {
    /// Displace one atom in one component.
    Atom {
        /// Mixture component index.
        structure: usize,
        /// Atom index.
        atom: usize,
    },
    /// Simultaneously move a fixed group within one component.
    Collective {
        /// Mixture component index.
        structure: usize,
        /// Moved atom indices in declared group order.
        atoms: Vec<usize>,
    },
    /// Transfer a uniformly sampled signed fraction between two components.
    Weight {
        /// Component receiving the signed transfer.
        first: usize,
        /// Component losing it.
        second: usize,
    },
}
/// Completed move. Calculator failures produce no record and consume no RNG state.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SessionStep {
    /// Since 0.2.11: optional energy search following this coordinate/weight attempt.
    /// `accepted` still describes the coordinate/weight move; `score` and
    /// `best_score` include any verified energy improvement. No RNG draws are used.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub energy: Option<EnergyUpdate>,
    /// One-based number of completed attempts.
    pub step: usize,
    /// Selected atom or weight pair.
    pub proposal: EnsembleMove,
    /// Whether the accepted state changed.
    pub accepted: bool,
    /// Whether a hard constraint rejected the move before scattering.
    pub constraint_rejected: bool,
    /// Evaluated proposal score, absent on hard rejection.
    pub trial_score: Option<f64>,
    /// Current total score after this attempt.
    pub score: f64,
    /// Best total score so far.
    pub best_score: f64,
    /// Numerical Metropolis tolerance used for this attempt. Historical records
    /// without this field deserialize as zero; their checkpoint settings are authoritative.
    #[serde(default)]
    pub temperature: f64,
    /// Dimensionless coordinate multiplier used by this proposal; one historically.
    #[serde(default = "unit_scale")]
    pub step_scale: f64,
}
fn unit_scale() -> f64 {
    1.
}

/// Explicit scientific-calculator change at a completed attempt/generation.
/// Histories from different revisions must not be compared as one objective trace.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CalculatorRevision {
    /// Completed RMC attempts or EA generations at the boundary.
    pub completed: usize,
    /// Previous scientific identity.
    pub previous: String,
    /// New scientific identity used after rescoring all retained states.
    pub next: String,
}
/// Versioned, serializable session checkpoint, including the original displacement
/// reference, accepted/best states, history and ChaCha8 stream position. Restore
/// through [`RmcSession::resume`], which validates its data and calculator identity.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RmcCheckpoint {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    initial_energy: Option<EnergyUpdate>,
    version: u32,
    #[serde(default)]
    adaptation: StepAdaptationState,
    #[serde(default)]
    revisions: Vec<CalculatorRevision>,
    calculator: String,
    problem: EnsembleProblem,
    settings: SessionSettings,
    initial: EnsembleState,
    current: EnsembleState,
    best: EnsembleState,
    rng: ChaCha8Rng,
    completed: usize,
    history: Vec<SessionStep>,
    trajectory: Vec<TrajectoryFrame>,
    #[serde(default)]
    diagnostics: SessionDiagnostics,
}

/// Stateful RMC engine. Accepted state, best state and RNG are committed together
/// only after successful evaluation. On failure the session remains resumable;
/// a retry proposes the same move. Calculators may retain geometry-keyed caches.
pub struct RmcSession {
    checkpoint: RmcCheckpoint,
    prepared: PreparedEnsemble,
}

pub(super) struct PreparedEnsemble {
    objectives: Vec<PreparedObjective>,
    grids: Vec<Vec<f64>>,
    absorbers: Vec<Vec<Vec<usize>>>,
    pub(super) movable: Vec<(usize, usize)>,
}

pub(super) fn normalize(structures: &mut [WeightedStructure]) -> Result<(), RmcError> {
    require(
        !structures.is_empty()
            && structures
                .iter()
                .all(|s| s.weight.is_finite() && s.weight >= 0.),
        "mixture needs nonnegative finite weights",
    )?;
    let sum = structures.iter().map(|s| s.weight).sum::<f64>();
    require(
        sum.is_finite() && sum > 0.,
        "mixture weights need a finite positive sum",
    )?;
    for s in structures {
        s.weight /= sum;
    }
    Ok(())
}

pub(super) fn prepare(
    problem: &EnsembleProblem,
    settings: &SessionSettings,
) -> Result<PreparedEnsemble, RmcError> {
    require(
        !problem.structures.is_empty() && !problem.datasets.is_empty(),
        "structures and datasets must be nonempty",
    )?;
    let sum = problem.structures.iter().map(|s| s.weight).sum::<f64>();
    require(
        problem
            .structures
            .iter()
            .all(|s| s.weight.is_finite() && s.weight >= 0.)
            && (sum - 1.).abs() < 1e-10,
        "mixture weights must sum to one",
    )?;
    if let Some(policy) = &settings.energy_refinement {
        policy.validate_problem(problem)?;
    }
    let m = &settings.moves;
    settings.cooling.temperature(m.temperature, 0)?;
    settings.stopping.validate()?;
    if let Some(policy) = &settings.adaptation {
        policy.validate()?;
    }
    require(
        m.step_size.is_finite()
            && m.step_size > 0.
            && m.step_size <= 1e6
            && m.temperature.is_finite()
            && m.temperature >= 0.
            && m.min_distance.is_finite()
            && m.min_distance > 0.
            && m.max_displacement.is_none_or(|v| v.is_finite() && v > 0.),
        "invalid move scale, tolerance or distance",
    )?;
    require(
        settings.weight_move_probability.is_finite()
            && (0. ..=1.).contains(&settings.weight_move_probability)
            && settings.weight_step.is_finite()
            && settings.weight_step > 0.
            && settings.weight_step <= 1.
            && (settings.weight_move_probability == 0. || problem.structures.len() >= 2),
        "invalid mixture-weight proposals",
    )?;
    require(
        settings.trajectory_stride == 0 || settings.trajectory_capacity > 0,
        "enabled trajectory needs a positive capacity",
    )?;
    let mut movable = Vec::new();
    for (s, structure) in problem.structures.iter().enumerate() {
        structure.configuration.validate()?;
        let indices = structure.movable_atoms.clone().unwrap_or_else(|| {
            if m.movable_atoms.is_empty() {
                (0..structure.configuration.atoms.len()).collect()
            } else {
                m.movable_atoms.clone()
            }
        });
        let mut seen = HashSet::new();
        require(
            indices
                .iter()
                .all(|&a| a < structure.configuration.atoms.len() && seen.insert(a)),
            "invalid movable atom indices",
        )?;
        movable.extend(indices.into_iter().map(|a| (s, a)));
    }
    require(
        m.steps == 0 || !movable.is_empty() || settings.weight_move_probability == 1.,
        "no coordinate moves available; fix settings or use weight_move_probability=1",
    )?;
    settings.constraints.validate(&problem.structures)?;
    settings.proposals.validate(&problem.structures, &movable)?;
    let mut names = HashSet::new();
    let mut objectives = Vec::new();
    let mut grids = Vec::new();
    let mut absorbers = Vec::new();
    for d in &problem.datasets {
        d.validate_spectrum_source()?;
        require(names.insert(&d.exafs.name), "dataset names must be unique")?;
        require(
            d.absorbers_by_structure.is_empty()
                || d.absorbers_by_structure.len() == problem.structures.len(),
            "absorber overrides need one list per structure",
        )?;
        if let Some(o) = &d.refeff {
            o.validate()?;
        }
        let mut lists = Vec::new();
        let mut absorber_z = None;
        for (s, structure) in problem.structures.iter().enumerate() {
            let mut data = d.exafs.clone();
            if !d.absorbers_by_structure.is_empty() {
                data.absorbers = d.absorbers_by_structure[s].clone();
            }
            super::engine::validate_problem(&RmcProblem {
                configuration: structure.configuration.clone(),
                datasets: vec![data.clone()],
            })?;
            let z = structure.configuration.atoms[data.absorbers[0]].atomic_number;
            require(
                absorber_z.is_none_or(|previous| previous == z),
                "mixture dataset absorbers must have the same element",
            )?;
            absorber_z = Some(z);
            lists.push(data.absorbers);
        }
        objectives.push(d.objective.prepare(&d.exafs.k)?);
        grids.push(super::engine::shifted_grid(&d.exafs)?);
        absorbers.push(lists);
    }
    Ok(PreparedEnsemble {
        objectives,
        grids,
        absorbers,
        movable,
    })
}

pub(super) fn evaluate_prepared<C: ExafsCalculator + ?Sized>(
    problem: &EnsembleProblem,
    settings: &SessionSettings,
    prepared: &PreparedEnsemble,
    structures: Vec<WeightedStructure>,
    reuse: Option<(&EnsembleState, Option<usize>)>,
    calculator: &mut C,
) -> Result<EnsembleState, RmcError> {
    let shifts = reuse
        .map(|(state, _)| state.delta_e0.clone())
        .unwrap_or_else(|| {
            if settings.energy_refinement.is_some() {
                problem.datasets.iter().map(|d| d.exafs.delta_e0).collect()
            } else {
                Vec::new()
            }
        });
    evaluate_at_shifts(
        problem, settings, prepared, structures, &shifts, reuse, calculator,
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn evaluate_at_shifts<C: ExafsCalculator + ?Sized>(
    problem: &EnsembleProblem,
    settings: &SessionSettings,
    prepared: &PreparedEnsemble,
    structures: Vec<WeightedStructure>,
    delta_e0: &[f64],
    reuse: Option<(&EnsembleState, Option<usize>)>,
    calculator: &mut C,
) -> Result<EnsembleState, RmcError> {
    evaluate_prepared_with(
        problem,
        settings,
        prepared,
        structures,
        delta_e0,
        reuse,
        &mut |requests| calculator.calculate_batch(requests),
    )
}

pub(super) fn evaluate_state<C: ExafsCalculator + ?Sized>(
    problem: &EnsembleProblem,
    settings: &SessionSettings,
    prepared: &PreparedEnsemble,
    state: &EnsembleState,
    calculator: &mut C,
) -> Result<EnsembleState, RmcError> {
    evaluate_at_shifts(
        problem,
        settings,
        prepared,
        state.structures.clone(),
        &state.delta_e0,
        None,
        calculator,
    )
}

// Keep objective assembly identical for single proposals and population batches.
#[allow(clippy::too_many_arguments)]
fn evaluate_prepared_with(
    problem: &EnsembleProblem,
    settings: &SessionSettings,
    prepared: &PreparedEnsemble,
    structures: Vec<WeightedStructure>,
    delta_e0: &[f64],
    reuse: Option<(&EnsembleState, Option<usize>)>,
    calculate: &mut impl FnMut(&[CalculationRequest<'_>]) -> Result<Vec<CalculatedSpectrum>, RmcError>,
) -> Result<EnsembleState, RmcError> {
    require(
        delta_e0.is_empty() || delta_e0.len() == problem.datasets.len(),
        "state ΔE₀ count differs",
    )?;
    let mut component_chi = Vec::new();
    let mut paths = Vec::new();
    let mut fits = Vec::new();
    let penalty = settings.constraints.energy(&structures)?;
    let mut total = penalty;
    for (d, data) in problem.datasets.iter().enumerate() {
        let shifted;
        let q = if let Some(&shift) = delta_e0.get(d) {
            let mut input = data.exafs.clone();
            input.delta_e0 = shift;
            shifted = super::engine::shifted_grid(&input)?;
            &shifted
        } else {
            &prepared.grids[d]
        };
        let mut components = Vec::new();
        for (s, structure) in structures.iter().enumerate() {
            if let Some((previous, changed)) = reuse {
                if changed != Some(s) && previous.delta_e0 == delta_e0 {
                    components.push(previous.component_chi[d][s].clone());
                    if settings.retain_paths {
                        paths.extend(
                            previous
                                .paths
                                .iter()
                                .filter(|p| p.dataset == d && p.structure == s)
                                .cloned(),
                        );
                    }
                    continue;
                }
            }
            let indices = &prepared.absorbers[d][s];
            let mut chi = vec![0.; q.len()];
            let requests: Vec<_> = indices
                .iter()
                .map(|&absorber| CalculationRequest {
                    structure: s,
                    configuration: &structure.configuration,
                    absorber,
                    edge: data.exafs.edge,
                    k: q,
                    options: data.refeff.as_ref(),
                    paths: settings.retain_paths,
                })
                .collect();
            let spectra = calculate(&requests)?;
            require(
                spectra.len() == indices.len(),
                "calculator returned wrong batch size",
            )?;
            for (&absorber, spectrum) in indices.iter().zip(spectra) {
                require(
                    spectrum.chi.len() == q.len()
                        && spectrum.chi.iter().all(|v| v.is_finite())
                        && spectrum.paths.iter().all(|p| {
                            p.chi.len() == q.len()
                                && p.chi.iter().all(|v| v.is_finite())
                                && p.half_length.is_finite()
                                && p.half_length > 0.
                                && p.degeneracy.is_finite()
                                && p.degeneracy > 0.
                        }),
                    "calculator returned invalid spectrum or paths",
                )?;
                for (sum, value) in chi.iter_mut().zip(spectrum.chi) {
                    *sum += value / indices.len() as f64;
                }
                if settings.retain_paths {
                    paths.push(AbsorberPaths {
                        dataset: d,
                        structure: s,
                        absorber,
                        paths: spectrum.paths,
                    });
                }
            }
            components.push(chi);
        }
        let mut chi = vec![0.; data.exafs.k.len()];
        for (s, component) in structures.iter().zip(&components) {
            for (v, &x) in chi.iter_mut().zip(component) {
                *v += s.weight * x;
            }
        }
        for v in &mut chi {
            *v *= data.exafs.s02;
        }
        let score = prepared.objectives[d].score(&data.exafs, &chi)?;
        total += score;
        fits.push(DatasetFit {
            name: data.exafs.name.clone(),
            chi,
            score,
        });
        component_chi.push(components);
    }
    require(total.is_finite(), "total objective overflow")?;
    Ok(EnsembleState {
        delta_e0: delta_e0.to_vec(),
        structures,
        evaluation: Evaluation {
            score: total,
            datasets: fits,
        },
        penalty,
        component_chi,
        paths,
    })
}

// Bound transient batches to 32 candidates and approximately one million output
// samples. One candidate is always allowed; the calculator enforces its own
// per-request limits. Requests and reductions retain candidate/dataset/atom order.
pub(super) fn evaluate_candidates<C: ExafsCalculator + ?Sized>(
    problem: &EnsembleProblem,
    settings: &SessionSettings,
    prepared: &PreparedEnsemble,
    candidates: Vec<Vec<WeightedStructure>>,
    calculator: &mut C,
) -> Result<Vec<EnsembleState>, RmcError> {
    let samples = prepared
        .absorbers
        .iter()
        .zip(&prepared.grids)
        .fold(0usize, |n, (lists, k)| {
            lists.iter().fold(n, |n, atoms| {
                n.saturating_add(atoms.len().saturating_mul(k.len()))
            })
        });
    let batch_size = (1_000_000 / samples.max(1)).clamp(1, 32);
    let mut result = Vec::with_capacity(candidates.len());
    let mut candidates = candidates.into_iter();
    loop {
        let chunk: Vec<_> = candidates.by_ref().take(batch_size).collect();
        if chunk.is_empty() {
            break;
        }
        let mut requests = Vec::new();
        for structures in &chunk {
            for (d, data) in problem.datasets.iter().enumerate() {
                for (s, structure) in structures.iter().enumerate() {
                    for &absorber in &prepared.absorbers[d][s] {
                        requests.push(CalculationRequest {
                            structure: s,
                            configuration: &structure.configuration,
                            absorber,
                            edge: data.exafs.edge,
                            k: &prepared.grids[d],
                            options: data.refeff.as_ref(),
                            paths: settings.retain_paths,
                        });
                    }
                }
            }
        }
        let spectra = calculator.calculate_batch(&requests)?;
        require(
            spectra.len() == requests.len(),
            "calculator returned wrong candidate batch size",
        )?;
        drop(requests);
        let mut spectra = spectra.into_iter();
        for structures in chunk {
            result.push(evaluate_prepared_with(
                problem,
                settings,
                prepared,
                structures,
                &[],
                None,
                &mut |requests| Ok(spectra.by_ref().take(requests.len()).collect()),
            )?);
        }
    }
    Ok(result)
}

/// Evaluate a normalized copy of a mixture without making moves. Applies all
/// structural hard rules to the input and includes the configured energy terms.
pub fn evaluate_ensemble<C: ExafsCalculator + ?Sized>(
    problem: &EnsembleProblem,
    settings: &SessionSettings,
    calculator: &mut C,
) -> Result<EnsembleState, RmcError> {
    let mut problem = problem.clone();
    normalize(&mut problem.structures)?;
    let prepared = prepare(&problem, settings)?;
    require(
        settings.constraints.allowed(
            &problem.structures,
            &problem.structures,
            &settings.moves,
            None,
        )?,
        "initial structures violate hard constraints",
    )?;
    evaluate_prepared(
        &problem,
        settings,
        &prepared,
        problem.structures.clone(),
        None,
        calculator,
    )
}

impl RmcSession {
    /// Since 0.2.11: improve the best RMC state with constrained numerical gradients.
    ///
    /// Uses full calculator evaluations and the original displacement reference.
    /// Does not mutate this session, its random stream, calibration or checkpoint.
    /// Defaults perform three passes with 0.001 Å derivative probes, at most
    /// 0.02 Å local moves and 5000 geometry evaluations. This is local optimization,
    /// not automatic differentiation or uncertainty sampling. The returned audit
    /// includes initial/best spectra, constraints, settings and accepted steps.
    /// Backend failures return an error and leave the session unchanged.
    pub fn refine_best<C: ExafsCalculator + ?Sized>(
        &self,
        settings: &LocalRefinementSettings,
        calculator: &mut C,
    ) -> Result<LocalRefinementResult, RmcError> {
        self.refine_best_with_progress(
            settings,
            calculator,
            |_| std::ops::ControlFlow::Continue(()),
        )
    }

    /// Local refinement with cancellation/progress at geometry-evaluation boundaries.
    /// Returning `Break(())` preserves the best verified local state in the result.
    /// See [`Self::refine_best`] for units, defaults, provenance and limitations.
    pub fn refine_best_with_progress<C, F>(
        &self,
        settings: &LocalRefinementSettings,
        calculator: &mut C,
        progress: F,
    ) -> Result<LocalRefinementResult, RmcError>
    where
        C: ExafsCalculator + ?Sized,
        F: FnMut(&LocalRefinementProgress) -> std::ops::ControlFlow<()>,
    {
        require(
            self.checkpoint.calculator == calculator.identity(),
            "local refinement calculator differs from the saved RMC model",
        )?;
        super::local_refinement::refine_local(
            &self.checkpoint.problem,
            &self.checkpoint.settings,
            &self.checkpoint.best,
            self.completed(),
            settings,
            calculator,
            progress,
        )
    }
    /// Borrow the validated input, including any spectrum processing snapshots
    /// (since 0.2.10). Mixture weights are normalized; the original displacement
    /// reference and experimental arrays remain fixed throughout the run.
    pub fn problem(&self) -> &EnsembleProblem {
        &self.checkpoint.problem
    }
    /// Validate inputs, normalize mixture fractions once, and calculate the initial
    /// state. Copies all inputs; no mutation is made to the caller's structures.
    pub fn new<C: ExafsCalculator + ?Sized>(
        problem: &EnsembleProblem,
        settings: &SessionSettings,
        calculator: &mut C,
    ) -> Result<Self, RmcError> {
        let mut problem = problem.clone();
        normalize(&mut problem.structures)?;
        let prepared = prepare(&problem, settings)?;
        require(
            settings.constraints.allowed(
                &problem.structures,
                &problem.structures,
                &settings.moves,
                None,
            )?,
            "initial structures violate hard constraints",
        )?;
        let initial = evaluate_prepared(
            &problem,
            settings,
            &prepared,
            problem.structures.clone(),
            None,
            calculator,
        )?;
        let (current, initial_energy) = if settings.energy_refinement.is_some() {
            let (state, update) = super::energy_refinement::refine_energy(
                &problem, settings, &prepared, &initial, true, calculator,
            )?;
            (state, Some(update))
        } else {
            (initial.clone(), None)
        };
        let trajectory = if settings.trajectory_stride > 0 {
            vec![TrajectoryFrame {
                step: 0,
                structures: current.structures.clone(),
                score: current.evaluation.score,
                delta_e0: current.delta_e0.clone(),
            }]
        } else {
            Vec::new()
        };
        Ok(Self {
            checkpoint: RmcCheckpoint {
                version: 1,
                initial_energy,
                adaptation: StepAdaptationState::default(),
                revisions: Vec::new(),
                calculator: calculator.identity(),
                problem,
                settings: settings.clone(),
                initial: initial.clone(),
                current: current.clone(),
                diagnostics: SessionDiagnostics {
                    improvement_anchor: current.evaluation.score,
                    ..Default::default()
                },
                best: current,
                rng: ChaCha8Rng::seed_from_u64(settings.moves.seed),
                completed: 0,
                history: Vec::new(),
                trajectory,
            },
            prepared,
        })
    }
    /// Restore a checkpoint after validating version, settings, state topology,
    /// constraints, backend identity and recomputed initial/current/best spectra.
    /// Exact continuation requires the same deterministic backend/dependency build.
    /// Cache contents and timing counters are intentionally not checkpointed.
    pub fn resume<C: ExafsCalculator + ?Sized>(
        mut checkpoint: RmcCheckpoint,
        calculator: &mut C,
    ) -> Result<Self, RmcError> {
        require(
            checkpoint.version == 1 && checkpoint.calculator == calculator.identity(),
            "checkpoint version or calculator identity mismatch",
        )?;
        validate_revisions(
            &checkpoint.revisions,
            checkpoint.completed,
            &checkpoint.calculator,
        )?;
        let prepared = prepare(&checkpoint.problem, &checkpoint.settings)?;
        if checkpoint.diagnostics.attempts == 0 && checkpoint.completed > 0 {
            checkpoint.diagnostics.last_improvement = checkpoint.completed;
            checkpoint.diagnostics.improvement_anchor = checkpoint.best.evaluation.score;
        }
        checkpoint
            .adaptation
            .validate(checkpoint.settings.adaptation.as_ref())?;
        require(
            checkpoint.adaptation.attempts <= checkpoint.completed,
            "adaptation attempts exceed session attempts",
        )?;
        let diagnostics = &checkpoint.diagnostics;
        require(
            diagnostics.attempts <= checkpoint.completed
                && diagnostics.accepted <= diagnostics.attempts
                && diagnostics.uphill <= diagnostics.accepted
                && diagnostics.constraint_rejected <= diagnostics.attempts - diagnostics.accepted
                && diagnostics.last_improvement <= checkpoint.completed
                && diagnostics.improvement_anchor.is_finite()
                && diagnostics.recent_acceptance.len()
                    <= checkpoint.settings.stopping.acceptance_window,
            "invalid checkpoint optimizer diagnostics",
        )?;
        require(
            checkpoint.completed <= checkpoint.settings.moves.steps
                && checkpoint.history.len() <= checkpoint.settings.history_capacity
                && checkpoint.trajectory.len() <= checkpoint.settings.trajectory_capacity,
            "invalid checkpoint counters",
        )?;
        match (
            &checkpoint.settings.energy_refinement,
            &checkpoint.initial_energy,
        ) {
            (Some(policy), Some(initial)) => {
                initial.validate(policy, checkpoint.problem.datasets.len())?;
                require(
                    initial.calculator
                        == checkpoint
                            .revisions
                            .first()
                            .map_or(checkpoint.calculator.as_str(), |revision| {
                                revision.previous.as_str()
                            }),
                    "checkpoint initial energy audit has an unknown calculator",
                )?;
                require(
                    initial.before == checkpoint.initial.energy_shifts(&checkpoint.problem)?
                        && initial.before
                            == checkpoint
                                .problem
                                .datasets
                                .iter()
                                .map(|d| d.exafs.delta_e0)
                                .collect::<Vec<_>>(),
                    "checkpoint initial ΔE₀ differs from the input",
                )?;
                if initial.calculator == checkpoint.calculator {
                    require(
                        initial.score_before == checkpoint.initial.evaluation.score,
                        "checkpoint initial energy audit differs from its initial state",
                    )?;
                }
                for record in &checkpoint.history {
                    require(
                        record.energy.is_some() == record.step.is_multiple_of(policy.interval),
                        "checkpoint energy-update cadence differs",
                    )?;
                    if let Some(update) = &record.energy {
                        update.validate(policy, checkpoint.problem.datasets.len())?;
                        require(
                            update.calculator == checkpoint.calculator
                                && update.score_after == record.score,
                            "checkpoint energy audit differs from the move record",
                        )?;
                    }
                }
                for frame in &checkpoint.trajectory {
                    require(
                        frame.delta_e0.len() == checkpoint.problem.datasets.len()
                            && frame.delta_e0.iter().all(|v| {
                                v.is_finite() && *v >= policy.bounds[0] && *v <= policy.bounds[1]
                            }),
                        "checkpoint trajectory has invalid ΔE₀",
                    )?;
                }
            }
            (None, None) => {
                require(
                    checkpoint.history.iter().all(|h| h.energy.is_none())
                        && checkpoint.trajectory.iter().all(|t| t.delta_e0.is_empty()),
                    "fixed-energy checkpoint contains energy updates",
                )?;
            }
            _ => {
                return Err(RmcError::Invalid(
                    "checkpoint initial energy audit is missing or unexpected".into(),
                ))
            }
        }
        for state in [&checkpoint.initial, &checkpoint.current, &checkpoint.best] {
            validate_state(state, &checkpoint.problem, &checkpoint.settings)?;
            let recalculated = evaluate_state(
                &checkpoint.problem,
                &checkpoint.settings,
                &prepared,
                state,
                calculator,
            )?;
            require(
                state == &recalculated,
                "checkpoint calculation differs from this backend; refusing inexact continuation",
            )?;
        }
        require(
            checkpoint.initial.structures == checkpoint.problem.structures
                && checkpoint.best.evaluation.score <= checkpoint.current.evaluation.score
                && checkpoint.best.evaluation.score <= checkpoint.initial.evaluation.score,
            "invalid checkpoint initial or best state",
        )?;
        Ok(Self {
            checkpoint,
            prepared,
        })
    }
    /// Since 0.2.10: rescore initial/current/best states on an explicitly changed
    /// calculator (for example a refreshed adaptive basis). The lowest new score
    /// among those three becomes best; the accepted current geometry stays current.
    /// Original displacement references, RNG, attempt count and adaptation remain.
    /// Old residual history/trajectory and stopping windows are cleared because
    /// they used a different model; the boundary is recorded in the checkpoint.
    /// All changes commit together after successful calculation. Save the previous
    /// checkpoint/report first if its full trace is needed for provenance.
    pub fn rebase<C: ExafsCalculator + ?Sized>(
        &mut self,
        calculator: &mut C,
    ) -> Result<(), RmcError> {
        let cp = &self.checkpoint;
        let identity = calculator.identity();
        require(
            identity != cp.calculator,
            "rebasing requires a changed calculator identity",
        )?;
        let mut states = [&cp.initial, &cp.current, &cp.best]
            .into_iter()
            .map(|state| {
                evaluate_state(&cp.problem, &cp.settings, &self.prepared, state, calculator)
            })
            .collect::<Result<Vec<_>, _>>()?;
        require(
            identity == calculator.identity(),
            "calculator identity changed while rebasing",
        )?;
        let best = states
            .iter()
            .min_by(|a, b| a.evaluation.score.total_cmp(&b.evaluation.score))
            .unwrap()
            .clone();
        let initial = states.remove(0);
        let current = states.remove(0);
        let cp = &mut self.checkpoint;
        cp.revisions.push(CalculatorRevision {
            completed: cp.completed,
            previous: cp.calculator.clone(),
            next: identity.clone(),
        });
        cp.calculator = identity;
        cp.initial = initial;
        cp.current = current;
        cp.best = best;
        cp.history.clear();
        cp.trajectory.clear();
        cp.diagnostics.last_improvement = cp.completed;
        cp.diagnostics.improvement_anchor = cp.best.evaluation.score;
        cp.diagnostics.recent_acceptance.clear();
        Ok(())
    }
    #[cfg(feature = "refeff-runner")]
    pub(super) fn audit_context(
        &self,
    ) -> (&EnsembleProblem, &SessionSettings, &PreparedEnsemble, &str) {
        (
            &self.checkpoint.problem,
            &self.checkpoint.settings,
            &self.prepared,
            &self.checkpoint.calculator,
        )
    }
    /// Explicit calculator-stage boundaries retained in the checkpoint.
    pub fn revisions(&self) -> &[CalculatorRevision] {
        &self.checkpoint.revisions
    }
    /// Return an owned checkpoint suitable for serde JSON; includes RNG and best state.
    pub fn checkpoint(&self) -> RmcCheckpoint {
        self.checkpoint.clone()
    }
    /// Atomically replace a JSON checkpoint using a temporary sibling and rename.
    /// Flushes the temporary file before replacement; errors preserve this session.
    pub fn save_checkpoint(&self, path: impl AsRef<std::path::Path>) -> Result<(), RmcError> {
        save_json_atomic(path.as_ref(), &self.checkpoint)
    }
    /// Read and validate a JSON checkpoint, recomputing stored states on this backend.
    pub fn load_checkpoint<C: ExafsCalculator + ?Sized>(
        path: impl AsRef<std::path::Path>,
        calculator: &mut C,
    ) -> Result<Self, RmcError> {
        let bytes =
            std::fs::read(path).map_err(|e| RmcError::Invalid(format!("checkpoint read: {e}")))?;
        let checkpoint = serde_json::from_slice(&bytes)
            .map_err(|e| RmcError::Invalid(format!("checkpoint JSON: {e}")))?;
        Self::resume(checkpoint, calculator)
    }
    /// Current accepted state, available even after a failed step.
    pub fn current(&self) -> &EnsembleState {
        &self.checkpoint.current
    }
    /// Lowest-scoring accepted state, available even after a failed step.
    pub fn best(&self) -> &EnsembleState {
        &self.checkpoint.best
    }
    /// Since 0.2.11: initial fixed-geometry energy-search audit, when enabled.
    pub fn initial_energy(&self) -> Option<&EnergyUpdate> {
        self.checkpoint.initial_energy.as_ref()
    }
    /// Initial state before optional energy refinement, and normalized fractions.
    pub fn initial(&self) -> &EnsembleState {
        &self.checkpoint.initial
    }
    /// Completed attempts, including hard rejections.
    pub fn completed(&self) -> usize {
        self.checkpoint.completed
    }
    /// Retained recent move records; bounded by history_capacity.
    pub fn history(&self) -> &[SessionStep] {
        &self.checkpoint.history
    }
    /// Retained recent coordinate frames; bounded by trajectory_capacity.
    pub fn trajectory(&self) -> &[TrajectoryFrame] {
        &self.checkpoint.trajectory
    }
    /// Persistent counters independent of history/trajectory capacity.
    pub fn diagnostics(&self) -> &SessionDiagnostics {
        &self.checkpoint.diagnostics
    }
    /// Current feedback state, also retained when history recording is disabled.
    pub fn adaptation(&self) -> &StepAdaptationState {
        &self.checkpoint.adaptation
    }
    /// Why the session will stop before its next proposal, if any. Stagnation and
    /// acceptance checks are optimizer diagnostics, not convergence certificates.
    pub fn stop_reason(&self) -> Option<StopReason> {
        let cp = &self.checkpoint;
        let stop = &cp.settings.stopping;
        if cp.completed >= cp.settings.moves.steps {
            return Some(StopReason::StepLimit);
        }
        if stop
            .target_score
            .is_some_and(|v| cp.best.evaluation.score <= v)
        {
            return Some(StopReason::TargetScore);
        }
        if stop
            .patience
            .is_some_and(|n| cp.completed - cp.diagnostics.last_improvement >= n)
        {
            return Some(StopReason::Stagnation);
        }
        if let Some(minimum) = stop.minimum_acceptance {
            let window = &cp.diagnostics.recent_acceptance;
            if window.len() == stop.acceptance_window
                && window.iter().filter(|&&v| v).count() as f64 / (window.len() as f64) < minimum
            {
                return Some(StopReason::LowAcceptance);
            }
        }
        None
    }
    /// Change only the total attempt limit, for extending a completed run.
    pub fn set_step_limit(&mut self, total: usize) -> Result<(), RmcError> {
        require(
            total >= self.completed(),
            "step limit cannot precede completed attempts",
        )?;
        let mut settings = self.checkpoint.settings.clone();
        settings.moves.steps = total;
        let prepared = prepare(&self.checkpoint.problem, &settings)?;
        self.checkpoint.settings = settings;
        self.prepared = prepared;
        Ok(())
    }
    /// Complete all remaining attempts. On any error, inspect or save the session;
    /// no accepted coordinates, history, best state or random draws have been lost.
    pub fn run<C: ExafsCalculator + ?Sized>(&mut self, calculator: &mut C) -> Result<(), RmcError> {
        while self.step(calculator)?.is_some() {}
        Ok(())
    }
    /// Attempt one move transactionally. None means the configured limit was reached.
    /// Calling with a different scientific calculator identity is an error.
    pub fn step<C: ExafsCalculator + ?Sized>(
        &mut self,
        calculator: &mut C,
    ) -> Result<Option<SessionStep>, RmcError> {
        require(
            calculator.identity() == self.checkpoint.calculator,
            "calculator identity changed during session",
        )?;
        let cp = &self.checkpoint;
        if self.stop_reason().is_some() {
            return Ok(None);
        }
        let mut rng = cp.rng.clone();
        let (mut record, mut accepted_state) = attempt_move(
            &cp.problem,
            &cp.settings,
            &self.prepared,
            &cp.current,
            cp.best.evaluation.score,
            cp.completed,
            cp.adaptation.scale,
            &mut rng,
            calculator,
        )?;
        if let Some(policy) = &cp.settings.energy_refinement {
            if (cp.completed + 1).is_multiple_of(policy.interval) {
                let current = accepted_state.as_ref().unwrap_or(&cp.current);
                let (refined, update) = super::energy_refinement::refine_energy(
                    &cp.problem,
                    &cp.settings,
                    &self.prepared,
                    current,
                    false,
                    calculator,
                )?;
                record.score = refined.evaluation.score;
                record.best_score = cp.best.evaluation.score.min(record.score);
                record.energy = Some(update);
                accepted_state = Some(refined);
            }
        }
        require(
            calculator.identity() == cp.calculator,
            "calculator identity changed during session step",
        )?;
        let cp = &mut self.checkpoint;
        cp.adaptation
            .observe(cp.settings.adaptation.as_ref(), &record);
        cp.diagnostics.attempts += 1;
        cp.diagnostics.accepted += usize::from(record.accepted);
        cp.diagnostics.uphill += usize::from(
            record.accepted
                && record
                    .trial_score
                    .is_some_and(|v| v > cp.current.evaluation.score),
        );
        cp.diagnostics.constraint_rejected += usize::from(record.constraint_rejected);
        if let Some(state) = accepted_state {
            cp.current = state;
            if cp.current.evaluation.score < cp.best.evaluation.score {
                cp.best = cp.current.clone();
            }
        }
        cp.rng = rng;
        cp.completed += 1;
        if cp.best.evaluation.score
            < cp.diagnostics.improvement_anchor - cp.settings.stopping.minimum_improvement
        {
            cp.diagnostics.last_improvement = cp.completed;
            cp.diagnostics.improvement_anchor = cp.best.evaluation.score;
        }
        push_bounded(
            &mut cp.diagnostics.recent_acceptance,
            record.accepted,
            cp.settings.stopping.acceptance_window,
        );
        push_bounded(
            &mut cp.history,
            record.clone(),
            cp.settings.history_capacity,
        );
        if cp.settings.trajectory_stride > 0
            && cp.completed.is_multiple_of(cp.settings.trajectory_stride)
        {
            push_bounded(
                &mut cp.trajectory,
                TrajectoryFrame {
                    step: cp.completed,
                    structures: cp.current.structures.clone(),
                    score: cp.current.evaluation.score,
                    delta_e0: cp.current.delta_e0.clone(),
                },
                cp.settings.trajectory_capacity,
            );
        }
        Ok(Some(record))
    }
}

pub(super) fn validate_state(
    state: &EnsembleState,
    problem: &EnsembleProblem,
    settings: &SessionSettings,
) -> Result<(), RmcError> {
    let shifts = state.energy_shifts(problem)?;
    if let Some(policy) = &settings.energy_refinement {
        require(
            state.delta_e0.len() == problem.datasets.len()
                && shifts
                    .iter()
                    .all(|s| *s >= policy.bounds[0] && *s <= policy.bounds[1]),
            "checkpoint ΔE₀ is missing or outside refinement bounds",
        )?;
    } else {
        require(
            state.delta_e0.is_empty(),
            "fixed-energy checkpoint contains variable ΔE₀",
        )?;
    }
    require(
        state.structures.len() == problem.structures.len(),
        "checkpoint structure count differs",
    )?;
    for (s, o) in state.structures.iter().zip(&problem.structures) {
        s.configuration.validate()?;
        require(
            s.configuration.cell == o.configuration.cell
                && s.configuration.atoms.len() == o.configuration.atoms.len()
                && s.configuration
                    .atoms
                    .iter()
                    .zip(&o.configuration.atoms)
                    .all(|(a, b)| a.atomic_number == b.atomic_number)
                && s.movable_atoms == o.movable_atoms
                && s.weight.is_finite()
                && s.weight >= 0.,
            "checkpoint topology, movement rules or weight changed",
        )?;
        let movable = o
            .movable_atoms
            .clone()
            .unwrap_or_else(|| {
                if settings.moves.movable_atoms.is_empty() {
                    (0..o.configuration.atoms.len()).collect()
                } else {
                    settings.moves.movable_atoms.clone()
                }
            })
            .into_iter()
            .collect::<HashSet<_>>();
        require(
            s.configuration
                .atoms
                .iter()
                .zip(&o.configuration.atoms)
                .enumerate()
                .all(|(i, (a, b))| movable.contains(&i) || a.position == b.position),
            "checkpoint changed a fixed atom",
        )?;
    }
    require(
        (state.structures.iter().map(|s| s.weight).sum::<f64>() - 1.).abs() < 1e-10,
        "checkpoint weights do not sum to one",
    )?;
    require(
        settings.constraints.allowed(
            &state.structures,
            &problem.structures,
            &settings.moves,
            None,
        )?,
        "checkpoint violates constraints",
    )
}
pub(super) fn push_bounded<T>(values: &mut Vec<T>, value: T, capacity: usize) {
    if capacity == 0 {
        return;
    }
    if values.len() >= capacity {
        values.remove(0);
    }
    values.push(value);
}
pub(super) fn save_json_atomic(
    path: &std::path::Path,
    value: &impl Serialize,
) -> Result<(), RmcError> {
    use std::io::Write;
    let mut tmp = path.as_os_str().to_os_string();
    tmp.push(format!(".{}.tmp", std::process::id()));
    let tmp = std::path::PathBuf::from(tmp);
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&tmp)
        .map_err(|e| RmcError::Invalid(format!("checkpoint temporary file: {e}")))?;
    let result = (|| {
        serde_json::to_writer(&mut file, value).map_err(std::io::Error::other)?;
        file.flush()?;
        file.sync_all()?;
        std::fs::rename(&tmp, path)
    })()
    .map_err(|e| RmcError::Invalid(format!("checkpoint write: {e}")));
    if result.is_err() {
        let _ = std::fs::remove_file(tmp);
    }
    result
}

// Shared proposal/evaluation/acceptance kernel: callers clone RNG and commit all
// optimizer state only after success. `problem` always holds the original bounds.
#[allow(clippy::too_many_arguments)]
pub(super) fn attempt_move<C: ExafsCalculator + ?Sized>(
    problem: &EnsembleProblem,
    settings: &SessionSettings,
    prepared: &PreparedEnsemble,
    current: &EnsembleState,
    best_score: f64,
    completed: usize,
    step_scale: f64,
    rng: &mut ChaCha8Rng,
    calculator: &mut C,
) -> Result<(SessionStep, Option<EnsembleState>), RmcError> {
    let temperature = settings
        .cooling
        .temperature(settings.moves.temperature, completed)?;
    let mut structures = current.structures.clone();
    let weight = settings.weight_move_probability > 0.
        && rng.random::<f64>() < settings.weight_move_probability;
    let (proposal, changed, allowed) = if weight {
        let first = rng.random_range(0..structures.len());
        let mut second = rng.random_range(0..structures.len() - 1);
        if second >= first {
            second += 1;
        }
        let delta = settings.weight_step * (2. * rng.random::<f64>() - 1.);
        structures[first].weight += delta;
        structures[second].weight -= delta;
        let allowed = structures[first].weight >= 0. && structures[second].weight >= 0.;
        (EnsembleMove::Weight { first, second }, None, allowed)
    } else {
        let proposal = settings.proposals.propose(
            &mut structures,
            &prepared.movable,
            settings.moves.step_size,
            step_scale,
            rng,
        );
        let (structure, moved) = match &proposal {
            EnsembleMove::Atom { structure, atom } => (*structure, Some((*structure, *atom))),
            EnsembleMove::Collective { structure, .. } => (*structure, None),
            EnsembleMove::Weight { .. } => {
                unreachable!("coordinate proposal returned a weight move")
            }
        };
        let allowed = settings.constraints.allowed(
            &structures,
            &problem.structures,
            &settings.moves,
            moved,
        )?;
        (proposal, Some(structure), allowed)
    };
    let mut record = SessionStep {
        energy: None,
        step: completed + 1,
        proposal,
        accepted: false,
        constraint_rejected: !allowed,
        trial_score: None,
        score: current.evaluation.score,
        best_score,
        temperature,
        step_scale,
    };
    let mut accepted_state = None;
    if allowed {
        let trial = evaluate_prepared(
            problem,
            settings,
            prepared,
            structures,
            Some((current, changed)),
            calculator,
        )?;
        let delta = trial.evaluation.score - current.evaluation.score;
        record.trial_score = Some(trial.evaluation.score);
        record.accepted = delta <= 0.
            || (temperature > 0. && rng.random::<f64>().ln() < -0.5 * delta / temperature);
        if record.accepted {
            record.score = trial.evaluation.score;
            record.best_score = record.best_score.min(record.score);
            accepted_state = Some(trial);
        }
    }
    Ok((record, accepted_state))
}

pub(super) fn validate_revisions(
    revisions: &[CalculatorRevision],
    completed: usize,
    identity: &str,
) -> Result<(), RmcError> {
    require(
        revisions.iter().all(|r| {
            r.completed <= completed
                && !r.previous.is_empty()
                && !r.next.is_empty()
                && r.previous != r.next
        }) && revisions
            .windows(2)
            .all(|r| r[0].completed <= r[1].completed && r[0].next == r[1].previous)
            && revisions.last().is_none_or(|r| r.next == identity),
        "invalid calculator revision history",
    )
}
