use super::session::{evaluate_prepared, normalize, prepare, validate_state, PreparedEnsemble};
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
    /// Mean pairwise RMS over movable coordinates and scaled weights, in Å.
    /// Coordinates are unwrapped and are not rotationally or translationally aligned.
    pub diversity: f64,
    /// Whether larger mutation was used for this generation.
    pub hypermutation: bool,
    /// Children replaced by a parent after exhausting hard-constraint attempts.
    pub constraint_fallbacks: usize,
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
}
/// Tournament selection, atom-wise uniform crossover, elitism and bounded
/// mutation. A complete generation commits atomically; calculator errors leave
/// the previous population and RNG available for checkpointing and retry.
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
            && settings.weight_distance_scale > 0.,
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
            },
            prepared,
        };
        let mut rng = result.checkpoint.rng.clone();
        let mut population = result.checkpoint.population.clone();
        for individual in population.iter_mut().skip(1) {
            let (child, _) = result.child(
                &result.checkpoint.population[0],
                &result.checkpoint.population[0],
                settings.hypermutation_factor,
                &mut rng,
                calculator,
            )?;
            *individual = child;
        }
        population.sort_by(|a, b| a.evaluation.score.total_cmp(&b.evaluation.score));
        result.checkpoint.population = population;
        result.checkpoint.rng = rng;
        Ok(result)
    }
    fn child<C: ExafsCalculator + ?Sized>(
        &self,
        a: &EnsembleState,
        b: &EnsembleState,
        scale: f64,
        rng: &mut ChaCha8Rng,
        calculator: &mut C,
    ) -> Result<(EnsembleState, bool), RmcError> {
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
                        *x += cp.session.moves.step_size * scale * (2. * rng.random::<f64>() - 1.);
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
                return Ok((a.clone(), false));
            }
            return Ok((
                evaluate_prepared(
                    &cp.problem,
                    &cp.session,
                    &self.prepared,
                    structures,
                    None,
                    calculator,
                )?,
                false,
            ));
        }
        Ok((a.clone(), true))
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
        while population.len() < cp.settings.population {
            let a = tournament(&cp.population, cp.settings.tournament, &mut rng);
            let b = tournament(&cp.population, cp.settings.tournament, &mut rng);
            let (child, fallback) = self.child(
                &cp.population[a],
                &cp.population[b],
                scale,
                &mut rng,
                calculator,
            )?;
            fallbacks += usize::from(fallback);
            population.push(child);
        }
        population.sort_by(|a, b| a.evaluation.score.total_cmp(&b.evaluation.score));
        let record = EvolutionGeneration {
            generation: cp.completed + 1,
            best_score: population[0].evaluation.score,
            diversity: diversity(
                &population,
                &self.prepared.movable,
                cp.settings.weight_distance_scale,
            ),
            hypermutation,
            constraint_fallbacks: fallbacks,
        };
        let cp = &mut self.checkpoint;
        cp.stagnation = if record.best_score < cp.population[0].evaluation.score {
            0
        } else {
            cp.stagnation + 1
        };
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
    /// Completed generation count.
    pub fn completed(&self) -> usize {
        self.checkpoint.completed
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
