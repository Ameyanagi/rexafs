//! Periodic exact audits around immutable adaptive stages. Unreleased.
use super::*;
use crate::rmc::session::{evaluate_candidates, PreparedEnsemble};

/// Automatic audit policy (unreleased). These are conservative numerical defaults,
/// not experimental uncertainties. Explicitly enable an adaptive basis first.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct AdaptiveAuditSettings {
    /// Completed attempts between RMC audits, or generations between EA audits.
    /// Default 100; the initial state is also audited before the first step.
    pub interval: usize,
    /// Maximum sqrt(objective(approximate − exact)/objective(experiment)).
    /// Uses each dataset's own transform, noise and k weighting; default 0.001.
    pub spectral_tolerance: f64,
    /// Maximum absolute change in data-fit score divided by experimental power,
    /// checked separately for each dataset; default 0.0001.
    pub score_tolerance: f64,
    /// Maximum successful retraining stages before a failed audit switches to
    /// exact paths permanently. Default 4. Zero immediately uses exact fallback.
    pub max_refreshes: usize,
    /// Maximum retained training geometries per component, excluding the fixed
    /// reference. Default 24, range 1..=1024. New audited states take priority;
    /// all states are still checked even if the training budget is smaller.
    pub max_training_geometries: usize,
}
impl Default for AdaptiveAuditSettings {
    fn default() -> Self {
        Self {
            interval: 100,
            spectral_tolerance: 0.001,
            score_tolerance: 0.0001,
            max_refreshes: 4,
            max_training_geometries: 24,
        }
    }
}
impl AdaptiveAuditSettings {
    fn validate(&self) -> Result<(), RmcError> {
        require(
            self.interval > 0
                && (1..=1024).contains(&self.max_training_geometries)
                && self.spectral_tolerance.is_finite()
                && self.spectral_tolerance >= 0.
                && self.score_tolerance.is_finite()
                && self.score_tolerance >= 0.,
            "invalid adaptive audit settings",
        )
    }
}
/// Why a scheduled or explicit audit completed. A passing audit bounds only the
/// retained states at that boundary, not unseen or rejected trial configurations.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum AdaptiveAuditAction {
    /// All retained states pass without changing the model.
    Passed,
    /// A new trained stage passes, and every retained state has been rescored.
    Refreshed,
    /// The refresh budget was exhausted or the refreshed stage failed its audit.
    /// All retained states now use exact typed paths with the same potentials.
    ExactFallback,
}
/// Audit evidence at one state boundary. All errors are dimensionless, evaluated
/// in the actual dataset objectives; no independent uncertainty is inferred.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AdaptiveAuditReport {
    /// Completed RMC attempts or EA generations, before the next proposal.
    pub completed: usize,
    /// Number of checked retained states (RMC initial/current/best or entire EA).
    pub states: usize,
    /// Maximum normalized spectral norm over states and datasets before refresh.
    pub spectral_error: f64,
    /// Maximum normalized data-fit score drift before refresh.
    pub score_error: f64,
    /// Maximum normalized spectral error of the committed model.
    pub final_spectral_error: f64,
    /// Maximum normalized score drift of the committed model.
    pub final_score_error: f64,
    /// Chosen action.
    pub action: AdaptiveAuditAction,
    /// Calculator identity before this audit.
    pub previous: String,
    /// Identity after this audit (unchanged for Passed).
    pub next: String,
}
/// Unreleased: periodically audit an adaptive calculator against exact typed
/// paths, retrain on retained geometries and atomically rebase the optimizer.
/// Use `step_rmc` or `step_evolution` for automatic scheduling, and explicitly
/// audit once more before publishing a time-limited result. This is a staged
/// optimizer, not a stationary Monte Carlo sampler or EVAX-identical policy.
///
/// Audits reuse the native objective, including complex R real/imaginary terms.
/// Electronic contexts are shared, and every population member is rescored on a
/// model change. Rebase clears the old optimizer trend window, so a plateau must
/// be re-established within the new stage. Reports preserve the boundaries.
/// Failed calculations preserve the controller and optimizer; calculator caches
/// may warm. An audit that succeeds is committed before the next proposal, so a
/// subsequent proposal failure leaves the newly audited stage available to save.
/// Serialize the combined checkpoints below to retain the schedule and manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdaptiveBasisController {
    settings: AdaptiveAuditSettings,
    identity: String,
    last_audit: Option<usize>,
    refreshes: usize,
    exact: bool,
    reports: Vec<AdaptiveAuditReport>,
}
trait AuditSession {
    fn context(&self) -> (&EnsembleProblem, &SessionSettings, &PreparedEnsemble, &str);
    fn states(&self) -> Vec<EnsembleState>;
    fn completed(&self) -> usize;
    fn rebase(&mut self, calc: &mut PreparedRefeffCalculator) -> Result<(), RmcError>;
}
impl AuditSession for RmcSession {
    fn context(&self) -> (&EnsembleProblem, &SessionSettings, &PreparedEnsemble, &str) {
        self.audit_context()
    }
    fn states(&self) -> Vec<EnsembleState> {
        vec![
            self.initial().clone(),
            self.current().clone(),
            self.best().clone(),
        ]
    }
    fn completed(&self) -> usize {
        self.completed()
    }
    fn rebase(&mut self, calc: &mut PreparedRefeffCalculator) -> Result<(), RmcError> {
        self.rebase(calc)
    }
}
impl AuditSession for EvolutionSession {
    fn context(&self) -> (&EnsembleProblem, &SessionSettings, &PreparedEnsemble, &str) {
        self.audit_context()
    }
    fn states(&self) -> Vec<EnsembleState> {
        self.population().to_vec()
    }
    fn completed(&self) -> usize {
        self.completed()
    }
    fn rebase(&mut self, calc: &mut PreparedRefeffCalculator) -> Result<(), RmcError> {
        self.rebase(calc)
    }
}
fn evaluate<S: AuditSession>(
    session: &S,
    states: &[EnsembleState],
    calc: &mut PreparedRefeffCalculator,
) -> Result<Vec<EnsembleState>, RmcError> {
    let (problem, settings, prepared, _) = session.context();
    evaluate_candidates(
        problem,
        settings,
        prepared,
        states.iter().map(|s| s.structures.clone()).collect(),
        calc,
    )
}
fn errors(
    problem: &EnsembleProblem,
    model: &[EnsembleState],
    exact: &[EnsembleState],
) -> Result<(f64, f64), RmcError> {
    require(model.len() == exact.len(), "audit state count mismatch")?;
    let mut spectral: f64 = 0.;
    let mut score: f64 = 0.;
    for (index, dataset) in problem.datasets.iter().enumerate() {
        let d = &dataset.exafs;
        let power = dataset.objective.score(d, &vec![0.; d.k.len()])?;
        require(
            power.is_finite() && power > 0.,
            "adaptive audit needs positive experimental objective power",
        )?;
        let prepared = dataset.objective.prepare(&d.k)?;
        for (model, exact) in model.iter().zip(exact) {
            let a = &model.evaluation.datasets[index];
            let b = &exact.evaluation.datasets[index];
            let mut difference = d.clone();
            difference.chi.clone_from(&b.chi);
            let error = (prepared.score(&difference, &a.chi)? / power).sqrt();
            let drift = (a.score - b.score).abs() / power;
            require(
                error.is_finite() && drift.is_finite(),
                "nonfinite adaptive audit error",
            )?;
            spectral = spectral.max(error);
            score = score.max(drift);
        }
    }
    Ok((spectral, score))
}
impl AdaptiveBasisController {
    /// Attach to an explicitly configured adaptive calculator. Copies policy and
    /// identity; does no scattering until the initial audit. Exact/legacy frozen
    /// calculators are rejected. Defaults are described in AdaptiveAuditSettings.
    pub fn new(
        settings: AdaptiveAuditSettings,
        calc: &PreparedRefeffCalculator,
    ) -> Result<Self, RmcError> {
        settings.validate()?;
        require(
            calc.settings.adaptive.is_some(),
            "automatic controller requires an adaptive basis",
        )?;
        Ok(Self {
            settings,
            identity: calc.identity(),
            last_audit: None,
            refreshes: 0,
            exact: false,
            reports: Vec::new(),
        })
    }
    /// Completed audit reports in chronological order; no scattering is run.
    pub fn reports(&self) -> &[AdaptiveAuditReport] {
        &self.reports
    }
    /// True after permanently switching this controller to exact typed paths.
    pub fn uses_exact_paths(&self) -> bool {
        self.exact
    }
    fn check<S: AuditSession>(
        &self,
        session: &S,
        calc: &PreparedRefeffCalculator,
    ) -> Result<(), RmcError> {
        self.settings.validate()?;
        require(
            calc.references.len() == session.context().0.structures.len()
                && self.identity == calc.identity()
                && self.identity == session.context().3
                && self.last_audit.is_none_or(|n| n <= session.completed())
                && self.refreshes <= self.settings.max_refreshes
                && self.exact == (calc.settings.basis == ScatteringBasis::Exact)
                && (self.exact || calc.settings.adaptive.is_some()),
            "adaptive controller, calculator and session do not match",
        )
    }
    fn due<S: AuditSession>(&self, session: &S) -> bool {
        !self.exact
            && self
                .last_audit
                .is_none_or(|n| session.completed().saturating_sub(n) >= self.settings.interval)
    }
    fn passes(&self, e: (f64, f64)) -> bool {
        e.0 <= self.settings.spectral_tolerance && e.1 <= self.settings.score_tolerance
    }
    fn audit<S: AuditSession>(
        &mut self,
        session: &mut S,
        calc: &mut PreparedRefeffCalculator,
    ) -> Result<(), RmcError> {
        self.check(session, calc)?;
        if self.exact {
            return Ok(());
        }
        let states = session.states();
        let mut exact_calc = calc.exact_reference()?;
        let exact = evaluate(session, &states, &mut exact_calc)?;
        let before = errors(session.context().0, &states, &exact)?;
        let mut after = before;
        let mut action = AdaptiveAuditAction::Passed;
        let mut next = None;
        if !self.passes(before) {
            if self.refreshes < self.settings.max_refreshes {
                let mut manifest = calc.settings.adaptive.clone().unwrap();
                manifest.epoch = manifest
                    .epoch
                    .checked_add(1)
                    .ok_or_else(|| RmcError::Invalid("adaptive epoch overflow".into()))?;
                let old = manifest.training.clone();
                manifest.training = vec![Vec::new(); calc.references.len()];
                for (i, training) in manifest.training.iter_mut().enumerate() {
                    for geometry in states
                        .iter()
                        .map(|s| &s.structures[i].configuration)
                        .chain(old.get(i).into_iter().flatten())
                    {
                        if training.len() == self.settings.max_training_geometries {
                            break;
                        }
                        if geometry != &calc.references[i] && !training.contains(geometry) {
                            training.push(geometry.clone());
                        }
                    }
                }
                let mut candidate = calc.refreshed_basis(manifest)?;
                let refreshed = evaluate(session, &states, &mut candidate)?;
                after = errors(session.context().0, &refreshed, &exact)?;
                if self.passes(after) {
                    action = AdaptiveAuditAction::Refreshed;
                    next = Some(candidate);
                }
            }
            if next.is_none() {
                action = AdaptiveAuditAction::ExactFallback;
                after = (0., 0.);
                next = Some(exact_calc);
            }
        }
        let previous = self.identity.clone();
        if let Some(mut candidate) = next {
            // Session rebase is transactional; neither model nor controller changes
            // until all retained states have been calculated under this identity.
            session.rebase(&mut candidate)?;
            *calc = candidate;
            self.identity = calc.identity();
            self.exact = action == AdaptiveAuditAction::ExactFallback;
            if action == AdaptiveAuditAction::Refreshed {
                self.refreshes += 1;
            }
        }
        self.last_audit = Some(session.completed());
        self.reports.push(AdaptiveAuditReport {
            completed: session.completed(),
            states: states.len(),
            spectral_error: before.0,
            score_error: before.1,
            final_spectral_error: after.0,
            final_score_error: after.1,
            action,
            previous,
            next: self.identity.clone(),
        });
        Ok(())
    }
    /// Perform a due audit, then one ordinary RMC attempt. The initial audit runs
    /// before the first proposal. Exact fallback disables later redundant audits.
    pub fn step_rmc(
        &mut self,
        session: &mut RmcSession,
        calc: &mut PreparedRefeffCalculator,
    ) -> Result<Option<SessionStep>, RmcError> {
        self.check(session, calc)?;
        if session.stop_reason().is_some() {
            return Ok(None);
        }
        if self.due(session) {
            self.audit(session, calc)?;
        }
        session.step(calc)
    }
    /// Perform a due audit, then one complete evolutionary generation, including
    /// configured local RMC steps. Interval units are generations for this method.
    pub fn step_evolution(
        &mut self,
        session: &mut EvolutionSession,
        calc: &mut PreparedRefeffCalculator,
    ) -> Result<Option<EvolutionGeneration>, RmcError> {
        self.check(session, calc)?;
        if self.due(session) {
            self.audit(session, calc)?;
        }
        session.step(calc)
    }
    /// Audit now, regardless of interval, for example before reporting a final fit.
    /// Audits initial/current/best; may change their ordering through rebasing.
    pub fn audit_rmc(
        &mut self,
        session: &mut RmcSession,
        calc: &mut PreparedRefeffCalculator,
    ) -> Result<(), RmcError> {
        self.audit(session, calc)
    }
    /// Audit every population member now, regardless of interval, before reporting.
    pub fn audit_evolution(
        &mut self,
        session: &mut EvolutionSession,
        calc: &mut PreparedRefeffCalculator,
    ) -> Result<(), RmcError> {
        self.audit(session, calc)
    }
    /// Copy a matched optimizer, controller and calculator manifest into one
    /// serializable checkpoint. Numerical caches and electronic tensors are omitted.
    pub fn checkpoint_rmc(
        &self,
        session: &RmcSession,
        calc: &PreparedRefeffCalculator,
    ) -> Result<AdaptiveRmcCheckpoint, RmcError> {
        self.check(session, calc)?;
        Ok(AdaptiveRmcCheckpoint {
            version: 1,
            controller: self.clone(),
            session: session.checkpoint(),
            options: calc.options.clone(),
            references: calc.references.clone(),
            acceleration: calc.settings.clone(),
        })
    }
    /// Copy a matched population, controller and manifest into one checkpoint.
    /// Restoring recalculates states to validate identity and exact reproducibility.
    pub fn checkpoint_evolution(
        &self,
        session: &EvolutionSession,
        calc: &PreparedRefeffCalculator,
    ) -> Result<AdaptiveEvolutionCheckpoint, RmcError> {
        self.check(session, calc)?;
        Ok(AdaptiveEvolutionCheckpoint {
            version: 1,
            controller: self.clone(),
            session: session.checkpoint(),
            options: calc.options.clone(),
            references: calc.references.clone(),
            acceleration: calc.settings.clone(),
        })
    }
}
macro_rules! checkpoint {
    ($name:ident, $checkpoint:ty, $session:ty) => {
        /// Unreleased: combined optimizer/controller checkpoint, including immutable
        /// basis training and original electronic references. Deserialize with serde,
        /// then resume to validate/recalculate retained states. No cached tensors or
        /// approximate path snapshots are trusted from disk.
        #[derive(Debug, Clone, Serialize, Deserialize)]
        #[serde(deny_unknown_fields)]
        pub struct $name {
            version: u32,
            controller: AdaptiveBasisController,
            session: $checkpoint,
            options: RefeffOptions,
            references: Vec<Configuration>,
            acceleration: AccelerationSettings,
        }
        impl $name {
            /// Reconstruct the same model, verify retained states and restore the RNG
            /// and audit schedule. Electronic setup/training run again on a cold
            /// resume; floating-point/dependency compatibility remains required.
            pub fn resume(
                self,
            ) -> Result<($session, PreparedRefeffCalculator, AdaptiveBasisController), RmcError>
            {
                require(self.version == 1, "unsupported adaptive checkpoint version")?;
                let mut calc = PreparedRefeffCalculator::new(
                    self.options,
                    self.references,
                    self.acceleration,
                )?;
                let session = <$session>::resume(self.session, &mut calc)?;
                self.controller.check(&session, &calc)?;
                Ok((session, calc, self.controller))
            }
            /// Atomically replace a JSON file via flushed temporary sibling/rename.
            pub fn save(&self, path: impl AsRef<std::path::Path>) -> Result<(), RmcError> {
                crate::rmc::session::save_json_atomic(path.as_ref(), self)
            }
        }
    };
}
checkpoint!(AdaptiveRmcCheckpoint, RmcCheckpoint, RmcSession);
checkpoint!(
    AdaptiveEvolutionCheckpoint,
    EvolutionCheckpoint,
    EvolutionSession
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fitting::FeffFitTransform;
    use crate::structure::Edge;

    fn fixture() -> (EnsembleProblem, SessionSettings, PreparedRefeffCalculator) {
        let reference = Configuration {
            atoms: vec![
                Atom {
                    atomic_number: 29,
                    position: [0.; 3],
                },
                Atom {
                    atomic_number: 29,
                    position: [2.5, 0., 0.],
                },
            ],
            cell: None,
        };
        let k: Vec<_> = (0..121).map(|i| 2. + i as f64 * 0.05).collect();
        let options = RefeffOptions {
            cluster_radius: 3.5,
            path_radius: 3.,
            max_legs: 2,
            path_criteria: [0., 0.],
            kmax: 12.,
            ..Default::default()
        };
        let acceleration = AccelerationSettings {
            catalogue: PathCatalogueSettings {
                radius: 3.,
                max_legs: 2,
                displacement: 0.3,
                ..Default::default()
            },
            basis: ScatteringBasis::Frozen {
                max_leg_change: 0.2,
                max_angle_change: 0.2,
            },
            adaptive: Some(AdaptiveBasisSettings {
                k: k.clone(),
                geometry_radius: 0.2,
                relative_error: 1e-10,
                ..Default::default()
            }),
            snapshots_per_context: 5,
            ..Default::default()
        };
        let mut calc =
            PreparedRefeffCalculator::new(options, vec![reference.clone()], acceleration).unwrap();
        let chi = calc.calculate(&reference, 0, Edge::K, &k).unwrap();
        let mut moved = reference;
        moved.atoms[1].position[0] += 0.04;
        let mut problem: EnsembleProblem = RmcProblem {
            configuration: moved,
            datasets: vec![ExafsDataset {
                name: "Cu audit fixture".into(),
                absorbers: vec![0],
                edge: Edge::K,
                sigma: vec![1.; k.len()],
                k,
                chi,
                weight: 1.,
                kweight: 2,
                s02: 1.,
                delta_e0: 0.,
            }],
        }
        .into();
        problem.datasets[0].objective = Objective::R(FeffFitTransform {
            kmin: 2.5,
            kmax: 7.5,
            rmin: 1.,
            rmax: 3.,
            kstep: Some(0.05),
            nfft: 1024,
            ..Default::default()
        });
        let settings = SessionSettings {
            moves: RmcSettings {
                steps: 8,
                seed: 43,
                movable_atoms: vec![1],
                min_distance: 1.,
                max_displacement: Some(0.1),
                step_size: 0.02,
                temperature: 0.,
            },
            ..Default::default()
        };
        (problem, settings, calc)
    }
    fn policy() -> AdaptiveAuditSettings {
        AdaptiveAuditSettings {
            interval: 2,
            spectral_tolerance: 1e-8,
            score_tolerance: 1e-8,
            max_refreshes: 2,
            ..Default::default()
        }
    }
    fn json(v: &impl Serialize) -> serde_json::Value {
        serde_json::to_value(v).unwrap()
    }

    #[test]
    fn controller_refreshes_rmc_reuses_electronics_and_resumes_the_same_random_stream() {
        let (problem, settings, mut calc) = fixture();
        let mut session = RmcSession::new(&problem, &settings, &mut calc).unwrap();
        let mut controller = AdaptiveBasisController::new(policy(), &calc).unwrap();
        controller.audit_rmc(&mut session, &mut calc).unwrap();
        assert_eq!(
            controller.reports()[0].action,
            AdaptiveAuditAction::Refreshed
        );
        assert!(controller.reports()[0].spectral_error > 1e-4);
        assert!(controller.reports()[0].final_spectral_error < 1e-8);
        assert_eq!(calc.stats().electronic_preparations, 0);
        assert_eq!(calc.stats().shared_electronic_contexts, 1);
        assert_eq!(session.revisions().len(), 1);
        let cp = controller.checkpoint_rmc(&session, &calc).unwrap();
        let restored: AdaptiveRmcCheckpoint =
            serde_json::from_str(&serde_json::to_string(&cp).unwrap()).unwrap();
        let (mut resumed, mut cold, mut control2) = restored.resume().unwrap();
        for _ in 0..8 {
            assert_eq!(
                json(&controller.step_rmc(&mut session, &mut calc).unwrap()),
                json(&control2.step_rmc(&mut resumed, &mut cold).unwrap())
            );
        }
        assert_eq!(json(&session.checkpoint()), json(&resumed.checkpoint()));
        assert_eq!(json(&controller), json(&control2));
        assert!(controller
            .reports()
            .iter()
            .all(|r| r.final_spectral_error <= 1e-8));
        assert!(
            controller.uses_exact_paths(),
            "strict error/budget must eventually fall back"
        );
        let (_, _, restored_control) = controller
            .checkpoint_rmc(&session, &calc)
            .unwrap()
            .resume()
            .unwrap();
        assert!(restored_control.uses_exact_paths());
    }

    #[test]
    fn controller_fallback_rescores_whole_population_and_preserves_checkpoint_schedule() {
        let (problem, settings, mut calc) = fixture();
        let evolution = EvolutionSettings {
            population: 4,
            elite: 1,
            generations: 3,
            local_steps: 2,
            ..Default::default()
        };
        let mut session =
            EvolutionSession::new(&problem, &settings, &evolution, &mut calc).unwrap();
        let mut controller = AdaptiveBasisController::new(
            AdaptiveAuditSettings {
                max_refreshes: 0,
                ..policy()
            },
            &calc,
        )
        .unwrap();
        controller.audit_evolution(&mut session, &mut calc).unwrap();
        assert_eq!(
            controller.reports()[0].action,
            AdaptiveAuditAction::ExactFallback
        );
        assert_eq!(controller.reports()[0].states, 4);
        assert_eq!(calc.stats().electronic_preparations, 0);
        let direct = evaluate(
            &session,
            session.population(),
            &mut calc.exact_reference().unwrap(),
        )
        .unwrap();
        assert_eq!(session.population(), direct);
        let checkpoint = controller.checkpoint_evolution(&session, &calc).unwrap();
        let checkpoint: AdaptiveEvolutionCheckpoint =
            serde_json::from_str(&serde_json::to_string(&checkpoint).unwrap()).unwrap();
        let (mut resumed, mut cold, mut control2) = checkpoint.resume().unwrap();
        for _ in 0..3 {
            assert_eq!(
                json(&controller.step_evolution(&mut session, &mut calc).unwrap()),
                json(&control2.step_evolution(&mut resumed, &mut cold).unwrap())
            );
        }
        assert_eq!(json(&session.checkpoint()), json(&resumed.checkpoint()));
        assert_eq!(json(&controller), json(&control2));
    }

    #[test]
    fn cancelled_audit_is_transactional_and_wrong_identity_is_rejected() {
        let (problem, settings, mut calc) = fixture();
        let mut session = RmcSession::new(&problem, &settings, &mut calc).unwrap();
        let mut controller = AdaptiveBasisController::new(policy(), &calc).unwrap();
        let before_session = json(&session.checkpoint());
        let before_controller = json(&controller);
        let mut wrong = calc.exact_reference().unwrap();
        assert!(controller.step_rmc(&mut session, &mut wrong).is_err());
        calc.cancellation_token().cancel();
        assert!(controller.step_rmc(&mut session, &mut calc).is_err());
        assert_eq!(before_session, json(&session.checkpoint()));
        assert_eq!(before_controller, json(&controller));
        assert_eq!(session.completed(), 0);
    }

    #[test]
    fn failed_training_keeps_optimizer_and_controller_and_retry_schedule_intact() {
        let (problem, settings, calc) = fixture();
        let mut acceleration = calc.settings.clone();
        let manifest = acceleration.adaptive.as_mut().unwrap();
        manifest.max_training_samples = manifest.k.len();
        let mut calc = calc.with_shared_contexts(acceleration).unwrap();
        let mut session = RmcSession::new(&problem, &settings, &mut calc).unwrap();
        let mut control = AdaptiveBasisController::new(policy(), &calc).unwrap();
        let before = json(&session.checkpoint());
        let controller = json(&control);
        let error = control.step_rmc(&mut session, &mut calc).unwrap_err();
        assert!(error.to_string().contains("sample budget"), "{error}");
        assert_eq!(before, json(&session.checkpoint()));
        assert_eq!(controller, json(&control));
        assert!(control.checkpoint_rmc(&session, &calc).is_ok());
    }

    #[test]
    fn passing_audits_keep_the_model_and_run_at_the_checkpointed_interval() {
        let (problem, settings, mut calc) = fixture();
        let mut session = RmcSession::new(&problem, &settings, &mut calc).unwrap();
        let mut control = AdaptiveBasisController::new(
            AdaptiveAuditSettings {
                spectral_tolerance: 10.,
                score_tolerance: 10.,
                ..policy()
            },
            &calc,
        )
        .unwrap();
        for _ in 0..3 {
            control.step_rmc(&mut session, &mut calc).unwrap();
        }
        let (mut session, mut calc, mut control) = control
            .checkpoint_rmc(&session, &calc)
            .unwrap()
            .resume()
            .unwrap();
        for _ in 0..4 {
            control.step_rmc(&mut session, &mut calc).unwrap();
        }
        assert_eq!(
            control
                .reports()
                .iter()
                .map(|r| r.completed)
                .collect::<Vec<_>>(),
            vec![0, 2, 4, 6]
        );
        assert!(control
            .reports()
            .iter()
            .all(|r| r.action == AdaptiveAuditAction::Passed));
        assert!(session.revisions().is_empty());
        assert_eq!(session.history().len(), 7);
    }

    #[test]
    fn audit_uses_native_complex_r_objective_and_ignores_common_weight_scale() {
        let (problem, settings, mut calc) = fixture();
        let session = RmcSession::new(&problem, &settings, &mut calc).unwrap();
        let states = session.states();
        let exact = evaluate(&session, &states, &mut calc.exact_reference().unwrap()).unwrap();
        let (spectral, drift) = errors(&problem, &states, &exact).unwrap();
        let dataset = &problem.datasets[0];
        let power = dataset
            .objective
            .score(&dataset.exafs, &vec![0.; dataset.exafs.k.len()])
            .unwrap();
        let mut data = dataset.exafs.clone();
        data.chi.clone_from(&exact[0].evaluation.datasets[0].chi);
        let native = (dataset
            .objective
            .score(&data, &states[0].evaluation.datasets[0].chi)
            .unwrap()
            / power)
            .sqrt();
        assert!((spectral - native).abs() < 1e-14);
        let mut scaled = problem.clone();
        scaled.datasets[0].exafs.weight *= 7.;
        let scale = |states: &[EnsembleState]| {
            states
                .iter()
                .map(|s| {
                    let mut s = s.clone();
                    s.evaluation.datasets[0].score *= 7.;
                    s
                })
                .collect::<Vec<_>>()
        };
        let other = errors(&scaled, &scale(&states), &scale(&exact)).unwrap();
        assert!((spectral - other.0).abs() < 1e-14);
        assert!((drift - other.1).abs() < 1e-14);
    }
}
