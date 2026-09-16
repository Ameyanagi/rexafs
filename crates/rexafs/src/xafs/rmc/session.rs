use super::objective::PreparedObjective;
use super::*;
use rand::{Rng, SeedableRng};
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
        }
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
}

/// Versioned, serializable session checkpoint, including the original displacement
/// reference, accepted/best states, history and ChaCha8 stream position. Restore
/// through [`RmcSession::resume`], which validates its data and calculator identity.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RmcCheckpoint {
    version: u32,
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
    let m = &settings.moves;
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
    let mut names = HashSet::new();
    let mut objectives = Vec::new();
    let mut grids = Vec::new();
    let mut absorbers = Vec::new();
    for d in &problem.datasets {
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
    let mut component_chi = Vec::new();
    let mut paths = Vec::new();
    let mut fits = Vec::new();
    let penalty = settings.constraints.energy(&structures)?;
    let mut total = penalty;
    for (d, data) in problem.datasets.iter().enumerate() {
        let mut components = Vec::new();
        for (s, structure) in structures.iter().enumerate() {
            if let Some((previous, changed)) = reuse {
                if changed != Some(s) {
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
            let q = &prepared.grids[d];
            let mut chi = vec![0.; q.len()];
            for &absorber in indices {
                let spectrum = calculator.calculate_request(CalculationRequest {
                    structure: s,
                    configuration: &structure.configuration,
                    absorber,
                    edge: data.exafs.edge,
                    k: q,
                    options: data.refeff.as_ref(),
                    paths: settings.retain_paths,
                })?;
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
        let trajectory = if settings.trajectory_stride > 0 {
            vec![TrajectoryFrame {
                step: 0,
                structures: initial.structures.clone(),
                score: initial.evaluation.score,
            }]
        } else {
            Vec::new()
        };
        Ok(Self {
            checkpoint: RmcCheckpoint {
                version: 1,
                calculator: calculator.identity(),
                problem,
                settings: settings.clone(),
                initial: initial.clone(),
                current: initial.clone(),
                best: initial,
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
        checkpoint: RmcCheckpoint,
        calculator: &mut C,
    ) -> Result<Self, RmcError> {
        require(
            checkpoint.version == 1 && checkpoint.calculator == calculator.identity(),
            "checkpoint version or calculator identity mismatch",
        )?;
        let prepared = prepare(&checkpoint.problem, &checkpoint.settings)?;
        require(
            checkpoint.completed <= checkpoint.settings.moves.steps
                && checkpoint.history.len() <= checkpoint.settings.history_capacity
                && checkpoint.trajectory.len() <= checkpoint.settings.trajectory_capacity,
            "invalid checkpoint counters",
        )?;
        for state in [&checkpoint.initial, &checkpoint.current, &checkpoint.best] {
            validate_state(state, &checkpoint.problem, &checkpoint.settings)?;
            let recalculated = evaluate_prepared(
                &checkpoint.problem,
                &checkpoint.settings,
                &prepared,
                state.structures.clone(),
                None,
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
    /// Initial state and normalized fractions.
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
        if cp.completed >= cp.settings.moves.steps {
            return Ok(None);
        }
        let mut rng = cp.rng.clone();
        let mut structures = cp.current.structures.clone();
        let weight = cp.settings.weight_move_probability > 0.
            && rng.random::<f64>() < cp.settings.weight_move_probability;
        let (proposal, changed, allowed) = if weight {
            let first = rng.random_range(0..structures.len());
            let mut second = rng.random_range(0..structures.len() - 1);
            if second >= first {
                second += 1;
            }
            let delta = cp.settings.weight_step * (2. * rng.random::<f64>() - 1.);
            structures[first].weight += delta;
            structures[second].weight -= delta;
            let allowed = structures[first].weight >= 0. && structures[second].weight >= 0.;
            (EnsembleMove::Weight { first, second }, None, allowed)
        } else {
            let (structure, atom) =
                self.prepared.movable[rng.random_range(0..self.prepared.movable.len())];
            for x in &mut structures[structure].configuration.atoms[atom].position {
                *x += cp.settings.moves.step_size * (2. * rng.random::<f64>() - 1.);
            }
            let allowed = cp.settings.constraints.allowed(
                &structures,
                &cp.problem.structures,
                &cp.settings.moves,
                Some((structure, atom)),
            )?;
            (
                EnsembleMove::Atom { structure, atom },
                Some(structure),
                allowed,
            )
        };
        let mut record = SessionStep {
            step: cp.completed + 1,
            proposal,
            accepted: false,
            constraint_rejected: !allowed,
            trial_score: None,
            score: cp.current.evaluation.score,
            best_score: cp.best.evaluation.score,
        };
        let mut accepted_state = None;
        if allowed {
            let trial = evaluate_prepared(
                &cp.problem,
                &cp.settings,
                &self.prepared,
                structures,
                Some((&cp.current, changed)),
                calculator,
            )?;
            let delta = trial.evaluation.score - cp.current.evaluation.score;
            record.trial_score = Some(trial.evaluation.score);
            record.accepted = delta <= 0.
                || (cp.settings.moves.temperature > 0.
                    && rng.random::<f64>().ln() < -0.5 * delta / cp.settings.moves.temperature);
            if record.accepted {
                record.score = trial.evaluation.score;
                record.best_score = record.best_score.min(record.score);
                accepted_state = Some(trial);
            }
        }
        let cp = &mut self.checkpoint;
        if let Some(state) = accepted_state {
            cp.current = state;
            if cp.current.evaluation.score < cp.best.evaluation.score {
                cp.best = cp.current.clone();
            }
        }
        cp.rng = rng;
        cp.completed += 1;
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
