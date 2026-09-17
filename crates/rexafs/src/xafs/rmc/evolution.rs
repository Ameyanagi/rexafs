use super::session::{
    evaluate_candidates, evaluate_prepared, normalize, prepare, validate_state, PreparedEnsemble,
};
use super::*;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

/// Evolutionary optimization settings. Individuals contain complete mixtures;
/// their populations are never averaged to produce an EXAFS spectrum. This is
/// optimization, not equilibrium sampling or an uncertainty estimator.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct EvolutionSettings {
    /// Individual count, 2..=1024; default 12.
    pub population: usize,
    /// Total generations; default 50.
    pub generations: usize,
    /// Lowest-scoring individuals copied unchanged each generation; default 2.
    pub elite: usize,
    /// Independent draws per tournament, sampled with replacement; default 3.
    pub tournament: usize,
    /// Probability of displacing each movable atom during mutation; default 0.25.
    pub mutation_probability: f64,
    /// Maximum constrained attempts to produce each child; default 16.
    /// Exhaustion retains its selected parent, so population size stays fixed.
    pub attempts_per_child: usize,
    /// Trigger larger mutation below this population RMS diversity in Å; default 0.02.
    pub minimum_diversity: f64,
    /// Trigger larger mutation after this many generations without improvement;
    /// zero disables the stagnation trigger, default 10.
    pub stagnation_generations: usize,
    /// Multiplier for coordinate and weight mutation when triggered; default 4.
    pub hypermutation_factor: f64,
    /// Convert a fraction difference into Å for the diversity metric; default 1.
    pub weight_distance_scale: f64,
    /// Unreleased: Metropolis attempts after crossover/mutation for each nonelite
    /// child. Zero preserves the original EA; start with one, then benchmark.
    /// The best state visited during refinement survives, including its start.
    pub local_steps: usize,
}
impl Default for EvolutionSettings {
    fn default() -> Self {
        Self {
            population: 12,
            generations: 50,
            elite: 2,
            tournament: 3,
            mutation_probability: 0.25,
            attempts_per_child: 16,
            minimum_diversity: 0.02,
            stagnation_generations: 10,
            hypermutation_factor: 4.,
            weight_distance_scale: 1.,
            local_steps: 0,
        }
    }
}
/// One completed generation's diagnostics.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EvolutionGeneration {
    /// One-based generation number.
    pub generation: usize,
    /// Best objective, with elitism preserving every improvement.
    pub best_score: f64,
    /// Mean population objective; None in historical records that did not retain it.
    #[serde(default)]
    pub mean_score: Option<f64>,
    /// Mean pairwise RMS over movable coordinates and scaled weights, in Å.
    /// Coordinates are unwrapped and are not rotationally or translationally aligned.
    pub diversity: f64,
    /// Whether larger mutation was used for this generation.
    pub hypermutation: bool,
    /// Children replaced by a parent after exhausting hard-constraint attempts.
    pub constraint_fallbacks: usize,
    /// Completed local Metropolis attempts this generation.
    #[serde(default)]
    pub local_attempts: usize,
    /// Accepted local moves this generation.
    #[serde(default)]
    pub local_accepted: usize,
    /// Local hard rejections before calculation, this generation.
    #[serde(default)]
    pub local_constraint_rejected: usize,
    /// Feedback scale after this generation, when hybrid refinement is enabled.
    #[serde(default = "default_scale")]
    pub step_scale: f64,
}
fn default_scale() -> f64 {
    1.
}
/// Serializable evolutionary population and RNG state. Restore through
/// [`EvolutionSession::resume`] to validate topology, constraints and spectra.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvolutionCheckpoint {
    version: u32,
    calculator: String,
    problem: EnsembleProblem,
    session: SessionSettings,
    settings: EvolutionSettings,
    population: Vec<EnsembleState>,
    rng: ChaCha8Rng,
    completed: usize,
    stagnation: usize,
    history: Vec<EvolutionGeneration>,
    #[serde(default)]
    local_completed: usize,
    #[serde(default)]
    adaptation: StepAdaptationState,
    #[serde(default)]
    revisions: Vec<CalculatorRevision>,
}
/// Tournament selection, atom-wise uniform crossover, elitism and bounded
/// mutation. A complete generation commits atomically; calculator errors leave
/// the previous population and RNG available for checkpointing and retry.
/// Unreleased: independent children are sent through calculator batches, bounded
/// to 32 candidates and approximately one million χ samples per batch. Reduction
/// and random draws keep a fixed order regardless of calculator worker count.
/// Optional local refinement reuses the RMC kernel, collective/species proposals,
/// cooling and acceptance feedback. Its bounds stay anchored to the original
/// problem. Session stopping rules apply only to RmcSession; hybrid work is bounded
/// by generations and local_steps. This is a REXAFS policy, not EVAX reproduction.
pub struct EvolutionSession {
    checkpoint: EvolutionCheckpoint,
    prepared: PreparedEnsemble,
}
fn validate(settings: &EvolutionSettings) -> Result<(), RmcError> {
    require(
        (2..=1024).contains(&settings.population)
            && settings.elite > 0
            && settings.elite < settings.population
            && settings.tournament > 0
            && settings.tournament <= 1024
            && settings.attempts_per_child > 0
            && settings.attempts_per_child <= 10000
            && settings.mutation_probability.is_finite()
            && (0. ..=1.).contains(&settings.mutation_probability)
            && settings.minimum_diversity.is_finite()
            && settings.minimum_diversity >= 0.
            && settings.hypermutation_factor.is_finite()
            && settings.hypermutation_factor >= 1.
            && settings.hypermutation_factor <= 1e6
            && settings.weight_distance_scale.is_finite()
            && settings.weight_distance_scale > 0.
            && settings.local_steps <= 1_000_000
            && settings
                .generations
                .checked_mul(settings.population - settings.elite)
                .and_then(|v| v.checked_mul(settings.local_steps))
                .is_some(),
        "invalid evolutionary settings",
    )
}
fn diversity(population: &[EnsembleState], movable: &[(usize, usize)], scale: f64) -> f64 {
    let mut sum = 0.;
    let mut count = 0.;
    for (i, a) in population.iter().enumerate() {
        for b in &population[..i] {
            let mut squares = 0.;
            for &(s, atom) in movable {
                for axis in 0..3 {
                    squares += (a.structures[s].configuration.atoms[atom].position[axis]
                        - b.structures[s].configuration.atoms[atom].position[axis])
                        .powi(2);
                }
            }
            for (x, y) in a.structures.iter().zip(&b.structures) {
                squares += ((x.weight - y.weight) * scale).powi(2);
            }
            sum += (squares / (movable.len() + a.structures.len()) as f64).sqrt();
            count += 1.;
        }
    }
    sum / count
}
fn tournament(population: &[EnsembleState], count: usize, rng: &mut ChaCha8Rng) -> usize {
    let mut best = rng.random_range(0..population.len());
    for _ in 1..count {
        let i = rng.random_range(0..population.len());
        if population[i].evaluation.score < population[best].evaluation.score {
            best = i;
        }
    }
    best
}
impl EvolutionSession {
    /// Borrow the validated input and its captured spectrum processing state
    /// (unreleased). This does not clone the population or run any processing.
    pub fn problem(&self) -> &EnsembleProblem {
        &self.checkpoint.problem
    }
    /// Evaluate the input mixture and initialize the population with constrained
    /// independent mutations around it. Uses the session seed; no GUI or files.
    pub fn new<C: ExafsCalculator + ?Sized>(
        problem: &EnsembleProblem,
        session: &SessionSettings,
        settings: &EvolutionSettings,
        calculator: &mut C,
    ) -> Result<Self, RmcError> {
        validate(settings)?;
        let mut problem = problem.clone();
        normalize(&mut problem.structures)?;
        let prepared = prepare(&problem, session)?;
        require(
            session.constraints.allowed(
                &problem.structures,
                &problem.structures,
                &session.moves,
                None,
            )?,
            "initial evolution structures violate constraints",
        )?;
        let initial = evaluate_prepared(
            &problem,
            session,
            &prepared,
            problem.structures.clone(),
            None,
            calculator,
        )?;
        let mut result = Self {
            checkpoint: EvolutionCheckpoint {
                version: 1,
                calculator: calculator.identity(),
                problem,
                session: session.clone(),
                settings: settings.clone(),
                population: vec![initial; settings.population],
                rng: ChaCha8Rng::seed_from_u64(session.moves.seed),
                completed: 0,
                stagnation: 0,
                history: Vec::new(),
                local_completed: 0,
                adaptation: StepAdaptationState::default(),
                revisions: Vec::new(),
            },
            prepared,
        };
        let mut rng = result.checkpoint.rng.clone();
        let mut population = result.checkpoint.population.clone();
        let proposals = (1..settings.population)
            .map(|_| {
                result
                    .propose_child(
                        &result.checkpoint.population[0],
                        &result.checkpoint.population[0],
                        settings.hypermutation_factor,
                        &mut rng,
                    )
                    .map(|(structures, _)| structures)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut evaluated = result
            .evaluate_children(&proposals, calculator)?
            .into_iter();
        for (individual, proposal) in population.iter_mut().skip(1).zip(proposals) {
            if proposal.is_some() {
                *individual = evaluated.next().expect("evaluated proposal");
            }
        }
        population.sort_by(|a, b| a.evaluation.score.total_cmp(&b.evaluation.score));
        result.checkpoint.population = population;
        result.checkpoint.rng = rng;
        Ok(result)
    }
    fn propose_child(
        &self,
        a: &EnsembleState,
        b: &EnsembleState,
        scale: f64,
        rng: &mut ChaCha8Rng,
    ) -> Result<(Option<Vec<WeightedStructure>>, bool), RmcError> {
        let cp = &self.checkpoint;
        for _ in 0..cp.settings.attempts_per_child {
            let mut structures = a.structures.clone();
            for &(s, atom) in &self.prepared.movable {
                if rng.random::<bool>() {
                    structures[s].configuration.atoms[atom].position =
                        b.structures[s].configuration.atoms[atom].position;
                }
                if rng.random::<f64>() < cp.settings.mutation_probability {
                    for x in &mut structures[s].configuration.atoms[atom].position {
                        let step = cp
                            .session
                            .proposals
                            .elements
                            .iter()
                            .find(|p| {
                                p.element == a.structures[s].configuration.atoms[atom].atomic_number
                            })
                            .map_or(cp.session.moves.step_size, |p| p.step_size);
                        *x += step * scale * (2. * rng.random::<f64>() - 1.);
                    }
                }
            }
            if cp.session.weight_move_probability > 0. {
                let t = rng.random::<f64>();
                for (s, other) in structures.iter_mut().zip(&b.structures) {
                    s.weight = (1. - t) * s.weight + t * other.weight;
                }
                if rng.random::<f64>() < cp.session.weight_move_probability {
                    let i = rng.random_range(0..structures.len());
                    let mut j = rng.random_range(0..structures.len() - 1);
                    if j >= i {
                        j += 1;
                    }
                    let d = cp.session.weight_step * scale * (2. * rng.random::<f64>() - 1.);
                    structures[i].weight += d;
                    structures[j].weight -= d;
                }
            }
            if structures.iter().any(|s| s.weight < 0.)
                || !cp.session.constraints.allowed(
                    &structures,
                    &cp.problem.structures,
                    &cp.session.moves,
                    None,
                )?
            {
                continue;
            }
            if structures == a.structures {
                return Ok((None, false));
            }
            return Ok((Some(structures), false));
        }
        Ok((None, true))
    }
    fn evaluate_children<C: ExafsCalculator + ?Sized>(
        &self,
        proposals: &[Option<Vec<WeightedStructure>>],
        calculator: &mut C,
    ) -> Result<Vec<EnsembleState>, RmcError> {
        evaluate_candidates(
            &self.checkpoint.problem,
            &self.checkpoint.session,
            &self.prepared,
            proposals.iter().filter_map(Clone::clone).collect(),
            calculator,
        )
    }
    /// Complete one generation, or return None after the configured limit.
    /// Population, best state and RNG remain unchanged on any calculator error.
    pub fn step<C: ExafsCalculator + ?Sized>(
        &mut self,
        calculator: &mut C,
    ) -> Result<Option<EvolutionGeneration>, RmcError> {
        let cp = &self.checkpoint;
        require(
            cp.calculator == calculator.identity(),
            "evolution calculator identity mismatch",
        )?;
        if cp.completed >= cp.settings.generations {
            return Ok(None);
        }
        let hypermutation = diversity(
            &cp.population,
            &self.prepared.movable,
            cp.settings.weight_distance_scale,
        ) < cp.settings.minimum_diversity
            || (cp.settings.stagnation_generations > 0
                && cp.stagnation >= cp.settings.stagnation_generations);
        let scale = if hypermutation {
            cp.settings.hypermutation_factor
        } else {
            1.
        };
        let mut rng = cp.rng.clone();
        let mut population = cp.population[..cp.settings.elite].to_vec();
        let mut fallbacks = 0;
        let mut proposals = Vec::new();
        let mut parents = Vec::new();
        for _ in cp.settings.elite..cp.settings.population {
            let a = tournament(&cp.population, cp.settings.tournament, &mut rng);
            let b = tournament(&cp.population, cp.settings.tournament, &mut rng);
            let (proposal, fallback) =
                self.propose_child(&cp.population[a], &cp.population[b], scale, &mut rng)?;
            fallbacks += usize::from(fallback);
            parents.push(a);
            proposals.push(proposal);
        }
        let mut evaluated = self.evaluate_children(&proposals, calculator)?.into_iter();
        for (proposal, parent) in proposals.into_iter().zip(parents) {
            population.push(if proposal.is_some() {
                evaluated.next().expect("evaluated proposal")
            } else {
                cp.population[parent].clone()
            });
        }
        let mut local_completed = cp.local_completed;
        let mut adaptation = cp.adaptation.clone();
        let mut local_accepted = 0;
        let mut local_constraint_rejected = 0;
        for child in population.iter_mut().skip(cp.settings.elite) {
            let mut current = child.clone();
            for _ in 0..cp.settings.local_steps {
                let (record, accepted) = super::session::attempt_move(
                    &cp.problem,
                    &cp.session,
                    &self.prepared,
                    &current,
                    child.evaluation.score,
                    local_completed,
                    adaptation.scale,
                    &mut rng,
                    calculator,
                )?;
                adaptation.observe(cp.session.adaptation.as_ref(), &record);
                local_completed += 1;
                local_accepted += usize::from(record.accepted);
                local_constraint_rejected += usize::from(record.constraint_rejected);
                if let Some(state) = accepted {
                    current = state;
                    if current.evaluation.score < child.evaluation.score {
                        *child = current.clone();
                    }
                }
            }
        }
        population.sort_by(|a, b| a.evaluation.score.total_cmp(&b.evaluation.score));
        let record = EvolutionGeneration {
            generation: cp.completed + 1,
            best_score: population[0].evaluation.score,
            mean_score: Some(
                population
                    .iter()
                    .map(|s| s.evaluation.score / population.len() as f64)
                    .sum::<f64>()
                    .max(population[0].evaluation.score),
            ),
            diversity: diversity(
                &population,
                &self.prepared.movable,
                cp.settings.weight_distance_scale,
            ),
            hypermutation,
            constraint_fallbacks: fallbacks,
            local_attempts: local_completed - cp.local_completed,
            local_accepted,
            local_constraint_rejected,
            step_scale: adaptation.scale,
        };
        let cp = &mut self.checkpoint;
        cp.stagnation = if record.best_score < cp.population[0].evaluation.score {
            0
        } else {
            cp.stagnation + 1
        };
        cp.local_completed = local_completed;
        cp.adaptation = adaptation;
        cp.population = population;
        cp.rng = rng;
        cp.completed += 1;
        super::session::push_bounded(&mut cp.history, record.clone(), cp.session.history_capacity);
        Ok(Some(record))
    }
    /// Run all remaining generations; retain this object to recover after an error.
    pub fn run<C: ExafsCalculator + ?Sized>(&mut self, calculator: &mut C) -> Result<(), RmcError> {
        while self.step(calculator)?.is_some() {}
        Ok(())
    }
    /// Best individual, preserved by elitism and sorted first.
    pub fn best(&self) -> &EnsembleState {
        &self.checkpoint.population[0]
    }
    /// All individuals, sorted by increasing objective.
    pub fn population(&self) -> &[EnsembleState] {
        &self.checkpoint.population
    }
    /// Recent generation records, bounded by the session history capacity.
    pub fn history(&self) -> &[EvolutionGeneration] {
        &self.checkpoint.history
    }
    /// Total local Metropolis attempts, used as the cooling-schedule index.
    pub fn local_completed(&self) -> usize {
        self.checkpoint.local_completed
    }
    /// Shared acceptance feedback over the population's local coordinate moves.
    pub fn adaptation(&self) -> &StepAdaptationState {
        &self.checkpoint.adaptation
    }
    /// Completed generation count.
    pub fn completed(&self) -> usize {
        self.checkpoint.completed
    }
    /// Unreleased: explicitly rescore every individual with a new calculator
    /// identity, then sort by the new objective. RNG, original displacement bounds,
    /// generation/local-attempt counts and feedback state remain unchanged. Clears
    /// the old score history and stagnation counter, records a revision boundary,
    /// and commits only after all calculations succeed. Retain the old checkpoint
    /// externally if its score trace is needed. This is an optimization stage change.
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
        let mut population = evaluate_candidates(
            &cp.problem,
            &cp.session,
            &self.prepared,
            cp.population.iter().map(|s| s.structures.clone()).collect(),
            calculator,
        )?;
        require(
            identity == calculator.identity(),
            "calculator identity changed while rebasing",
        )?;
        population.sort_by(|a, b| a.evaluation.score.total_cmp(&b.evaluation.score));
        let cp = &mut self.checkpoint;
        cp.revisions.push(CalculatorRevision {
            completed: cp.completed,
            previous: cp.calculator.clone(),
            next: identity.clone(),
        });
        cp.calculator = identity;
        cp.population = population;
        cp.history.clear();
        cp.stagnation = 0;
        Ok(())
    }
    #[cfg(feature = "refeff-runner")]
    pub(super) fn audit_context(
        &self,
    ) -> (&EnsembleProblem, &SessionSettings, &PreparedEnsemble, &str) {
        (
            &self.checkpoint.problem,
            &self.checkpoint.session,
            &self.prepared,
            &self.checkpoint.calculator,
        )
    }
    /// Explicit calculator-stage boundaries retained in the checkpoint.
    pub fn revisions(&self) -> &[CalculatorRevision] {
        &self.checkpoint.revisions
    }
    /// Owned checkpoint with the complete population and random stream.
    pub fn checkpoint(&self) -> EvolutionCheckpoint {
        self.checkpoint.clone()
    }
    /// Atomically save the evolutionary checkpoint to JSON, replacing the target.
    pub fn save_checkpoint(&self, path: impl AsRef<std::path::Path>) -> Result<(), RmcError> {
        super::session::save_json_atomic(path.as_ref(), &self.checkpoint)
    }
    /// Resume after validating every individual's topology, constraints and exact
    /// calculated state. Backend identity, dependency build and settings must match.
    pub fn resume<C: ExafsCalculator + ?Sized>(
        checkpoint: EvolutionCheckpoint,
        calculator: &mut C,
    ) -> Result<Self, RmcError> {
        require(
            checkpoint.version == 1 && checkpoint.calculator == calculator.identity(),
            "evolution checkpoint version or calculator differs",
        )?;
        validate(&checkpoint.settings)?;
        super::session::validate_revisions(
            &checkpoint.revisions,
            checkpoint.completed,
            &checkpoint.calculator,
        )?;
        checkpoint
            .adaptation
            .validate(checkpoint.session.adaptation.as_ref())?;
        require(
            Some(checkpoint.local_completed)
                == checkpoint
                    .completed
                    .checked_mul(checkpoint.settings.population - checkpoint.settings.elite)
                    .and_then(|v| v.checked_mul(checkpoint.settings.local_steps))
                && checkpoint.adaptation.attempts <= checkpoint.local_completed,
            "invalid hybrid checkpoint counters",
        )?;
        let prepared = prepare(&checkpoint.problem, &checkpoint.session)?;
        require(
            checkpoint.population.len() == checkpoint.settings.population
                && checkpoint.completed <= checkpoint.settings.generations
                && checkpoint.history.len() <= checkpoint.session.history_capacity,
            "invalid evolutionary checkpoint counters",
        )?;
        for state in &checkpoint.population {
            validate_state(state, &checkpoint.problem, &checkpoint.session)?;
            let computed = evaluate_prepared(
                &checkpoint.problem,
                &checkpoint.session,
                &prepared,
                state.structures.clone(),
                None,
                calculator,
            )?;
            require(
                state == &computed,
                "evolution checkpoint calculation differs from backend",
            )?;
        }
        require(
            checkpoint
                .population
                .windows(2)
                .all(|v| v[0].evaluation.score <= v[1].evaluation.score),
            "checkpoint population must be sorted",
        )?;
        Ok(Self {
            checkpoint,
            prepared,
        })
    }
}
