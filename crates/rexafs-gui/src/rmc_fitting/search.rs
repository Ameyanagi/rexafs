//! Desktop search modes keep numerical sessions and checkpoints typed.
use super::*;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mode {
    #[default]
    Rmc,
    Genetic,
    Hybrid,
}
impl Mode {
    pub const ALL: [Self; 3] = [Self::Rmc, Self::Genetic, Self::Hybrid];
    pub fn label(self) -> &'static str {
        match self {
            Self::Rmc => "RMC",
            Self::Genetic => "Genetic / EA",
            Self::Hybrid => "Hybrid EA–RMC",
        }
    }
}
/// The untagged RMC variant preserves the exact historical desktop JSON shape.
/// New evolutionary checkpoints carry an explicit wrapper and a full population.
#[derive(Clone, Serialize)]
#[serde(untagged)]
pub enum Checkpoint {
    Rmc(RmcCheckpoint),
    Evolution { evolution: EvolutionCheckpoint },
}
// Serde's untagged Content buffer cannot deserialize ChaCha's u128 word
// position. Dispatch raw JSON, then let the typed checkpoint decode directly;
// floating-point spectra and the complete integer random-stream position survive.
impl<'de> Deserialize<'de> for Checkpoint {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::Error;
        let raw = Box::<serde_json::value::RawValue>::deserialize(deserializer)?;
        #[derive(Deserialize)]
        struct Tag {
            evolution: Option<Box<serde_json::value::RawValue>>,
        }
        let tag: Tag = serde_json::from_str(raw.get()).map_err(D::Error::custom)?;
        if tag.evolution.is_some() {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct Wrapper {
                evolution: EvolutionCheckpoint,
            }
            let wrapper: Wrapper = serde_json::from_str(raw.get()).map_err(D::Error::custom)?;
            Ok(Self::Evolution {
                evolution: wrapper.evolution,
            })
        } else {
            serde_json::from_str(raw.get())
                .map(Self::Rmc)
                .map_err(D::Error::custom)
        }
    }
}
impl Checkpoint {
    pub fn numerical_value(&self) -> Result<serde_json::Value, String> {
        match self {
            Self::Rmc(cp) => serde_json::to_value(cp),
            Self::Evolution { evolution } => serde_json::to_value(evolution),
        }
        .map_err(|e| e.to_string())
    }
}
#[cfg(feature = "refeff-runner")]
pub(super) enum Session {
    Rmc(RmcSession),
    Evolution {
        session: EvolutionSession,
        initial: Arc<EnsembleState>,
    },
}
#[cfg(feature = "refeff-runner")]
impl Session {
    pub fn new(
        request: &Request,
        calculator: &mut PreparedRefeffCalculator,
    ) -> Result<Self, RmcError> {
        let baseline = RmcSession::new(&request.problem, &request.settings, calculator)?;
        Ok(if let Some(settings) = &request.evolution {
            let initial = Arc::new(baseline.initial().clone());
            // The second initial evaluation reuses exact cached scattering. The
            // baseline remains the user's input, not the best randomized member.
            Self::Evolution {
                session: EvolutionSession::new(
                    &request.problem,
                    &request.settings,
                    settings,
                    calculator,
                )?,
                initial,
            }
        } else {
            Self::Rmc(baseline)
        })
    }
    pub fn resume(
        saved: &SavedRun,
        calculator: &mut PreparedRefeffCalculator,
    ) -> Result<Self, RmcError> {
        Ok(match &saved.checkpoint {
            Checkpoint::Rmc(cp) => Self::Rmc(RmcSession::resume(cp.clone(), calculator)?),
            Checkpoint::Evolution { evolution } => {
                let baseline =
                    RmcSession::new(&saved.request.problem, &saved.request.settings, calculator)?;
                if baseline.initial() != saved.progress.initial.as_ref() {
                    return Err(RmcError::Invalid(
                        "Saved initial evolutionary spectrum differs from its backend.".into(),
                    ));
                }
                Self::Evolution {
                    session: EvolutionSession::resume(evolution.clone(), calculator)?,
                    initial: saved.progress.initial.clone(),
                }
            }
        })
    }
    pub fn initial(&self) -> &EnsembleState {
        match self {
            Self::Rmc(s) => s.initial(),
            Self::Evolution { initial, .. } => initial,
        }
    }
    pub fn current(&self) -> &EnsembleState {
        match self {
            Self::Rmc(s) => s.current(),
            Self::Evolution { session, .. } => session.best(),
        }
    }
    pub fn best(&self) -> &EnsembleState {
        match self {
            Self::Rmc(s) => s.best(),
            Self::Evolution { session, .. } => session.best(),
        }
    }
    pub fn completed(&self) -> usize {
        match self {
            Self::Rmc(s) => s.completed(),
            Self::Evolution { session, .. } => session.completed(),
        }
    }
    pub fn history(&self) -> &[SessionStep] {
        match self {
            Self::Rmc(s) => s.history(),
            _ => &[],
        }
    }
    pub fn evolution_history(&self) -> &[EvolutionGeneration] {
        match self {
            Self::Evolution { session, .. } => session.history(),
            _ => &[],
        }
    }
    pub fn local_attempts(&self) -> usize {
        match self {
            Self::Evolution { session, .. } => session.local_completed(),
            _ => 0,
        }
    }
    pub fn adaptation(&self) -> &StepAdaptationState {
        match self {
            Self::Rmc(s) => s.adaptation(),
            Self::Evolution { session, .. } => session.adaptation(),
        }
    }
    pub fn stop_reason(&self) -> Option<StopReason> {
        match self {
            Self::Rmc(s) => s.stop_reason(),
            _ => None,
        }
    }
    pub fn version(&self) -> u32 {
        match self {
            Self::Rmc(_) => 1,
            _ => 2,
        }
    }
    pub fn checkpoint(&self) -> Checkpoint {
        match self {
            Self::Rmc(s) => Checkpoint::Rmc(s.checkpoint()),
            Self::Evolution { session, .. } => Checkpoint::Evolution {
                evolution: session.checkpoint(),
            },
        }
    }
    pub fn set_step_limit(&mut self, total: usize) -> Result<(), RmcError> {
        match self {
            Self::Rmc(s) => s.set_step_limit(total),
            Self::Evolution { session, .. } => session.set_generation_limit(total),
        }
    }
    /// Returns accepted/rejected local counts; a generation is never an MC attempt.
    pub fn step(
        &mut self,
        calculator: &mut PreparedRefeffCalculator,
    ) -> Result<Option<(usize, usize)>, RmcError> {
        match self {
            Self::Rmc(s) => Ok(s
                .step(calculator)?
                .map(|s| (usize::from(s.accepted), usize::from(s.constraint_rejected)))),
            Self::Evolution { session, .. } => Ok(session
                .step(calculator)?
                .map(|s| (s.local_accepted, s.local_constraint_rejected))),
        }
    }
    pub fn refine_best_with_progress<F>(
        &self,
        settings: &LocalRefinementSettings,
        calculator: &mut PreparedRefeffCalculator,
        progress: F,
    ) -> Result<LocalRefinementResult, RmcError>
    where
        F: FnMut(&LocalRefinementProgress) -> std::ops::ControlFlow<()>,
    {
        match self {
            Self::Rmc(s) => s.refine_best_with_progress(settings, calculator, progress),
            Self::Evolution { session, .. } => {
                session.refine_best_with_progress(settings, calculator, progress)
            }
        }
    }
}
