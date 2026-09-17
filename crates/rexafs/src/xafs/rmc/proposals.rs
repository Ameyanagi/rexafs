use super::*;
use rand::Rng;

/// Predetermined numerical tolerance schedule, indexed by completed attempts.
/// This is optimizer annealing, not a physical temperature or equilibrium
/// sampler at one temperature. The attempt index is already checkpointed.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub enum CoolingSchedule {
    /// Keep `RmcSettings::temperature` fixed (default).
    #[default]
    Constant,
    /// `max(floor, initial * factor^attempt)`, with `0 < factor <= 1`.
    Geometric {
        /// Dimensionless multiplier after each completed attempt.
        factor: f64,
        /// Nonnegative dimensionless lower bound, no larger than initial tolerance.
        floor: f64,
    },
    /// Linear interpolation from initial to final tolerance over `attempts`;
    /// subsequent moves use the final tolerance. Extending a run does not rescale it.
    Linear {
        /// Nonnegative final numerical tolerance, no larger than the initial value.
        final_temperature: f64,
        /// Positive number of attempted moves in the cooling interval.
        attempts: usize,
    },
}
impl CoolingSchedule {
    /// Validate and evaluate the schedule. All tolerances are dimensionless.
    pub fn temperature(&self, initial: f64, completed: usize) -> Result<f64, RmcError> {
        require(
            initial.is_finite() && initial >= 0.,
            "invalid initial annealing tolerance",
        )?;
        match *self {
            Self::Constant => Ok(initial),
            Self::Geometric { factor, floor } => {
                require(
                    factor.is_finite()
                        && factor > 0.
                        && factor <= 1.
                        && floor.is_finite()
                        && floor >= 0.
                        && floor <= initial,
                    "invalid geometric cooling schedule",
                )?;
                Ok((initial * factor.powf(completed as f64)).max(floor))
            }
            Self::Linear {
                final_temperature,
                attempts,
            } => {
                require(
                    attempts > 0
                        && final_temperature.is_finite()
                        && final_temperature >= 0.
                        && final_temperature <= initial,
                    "invalid linear cooling schedule",
                )?;
                let fraction = (completed as f64 / attempts as f64).min(1.);
                Ok(initial * (1. - fraction) + final_temperature * fraction)
            }
        }
    }
}

/// Fixed species-dependent single-atom proposals. Selection probabilities cannot
/// depend on the current geometry; this keeps the coordinate proposal symmetric.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ElementProposal {
    /// Atomic number, at most one rule per element.
    pub element: u8,
    /// Positive uniform Cartesian half-width in Å, replacing the global step size.
    pub step_size: f64,
    /// Nonnegative selection weight for each movable atom of this element.
    /// Other elements have weight one. Zero excludes these single-atom proposals.
    pub weight: f64,
}
/// A fixed collective proposal acting within one structure.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CollectiveProposal {
    /// Mixture component index.
    pub structure: usize,
    /// Nonempty, distinct movable atom indices.
    pub atoms: Vec<usize>,
    /// Uniform Cartesian half-width in Å.
    pub step_size: f64,
    /// Positive selection weight relative to individual atom weights.
    pub weight: f64,
    /// True draws an independent displacement for each member (an all-atom
    /// move when all are listed); false translates the group by one vector.
    pub independent: bool,
}
/// Predetermined proposal mixture, copied into checkpoints. Empty lists preserve
/// the original uniform single-atom generator and its exact random-draw sequence.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct ProposalSettings {
    /// Optional per-element scales and selection weights.
    pub elements: Vec<ElementProposal>,
    /// Optional group translations or simultaneous independent displacements.
    pub collective: Vec<CollectiveProposal>,
}
impl ProposalSettings {
    pub(super) fn validate(
        &self,
        structures: &[WeightedStructure],
        movable: &[(usize, usize)],
    ) -> Result<(), RmcError> {
        for (i, e) in self.elements.iter().enumerate() {
            require(
                crate::structure::Element::from_z(e.element).is_some()
                    && e.step_size.is_finite()
                    && e.step_size > 0.
                    && e.step_size <= 1e6
                    && e.weight.is_finite()
                    && e.weight >= 0.
                    && !self.elements[..i].iter().any(|x| x.element == e.element),
                "invalid element proposal",
            )?;
        }
        for c in &self.collective {
            let unique: std::collections::BTreeSet<_> = c.atoms.iter().copied().collect();
            require(
                c.structure < structures.len()
                    && !c.atoms.is_empty()
                    && unique.len() == c.atoms.len()
                    && c.atoms.iter().all(|&a| movable.contains(&(c.structure, a)))
                    && c.step_size.is_finite()
                    && c.step_size > 0.
                    && c.step_size <= 1e6
                    && c.weight.is_finite()
                    && c.weight > 0.,
                "invalid collective proposal or fixed atom in group",
            )?;
        }
        let total = self.total_weight(structures, movable);
        require(
            total.is_finite() && (total > 0. || movable.is_empty()),
            "coordinate proposal weights have no finite positive sum",
        )
    }
    fn rule(&self, z: u8) -> Option<&ElementProposal> {
        self.elements.iter().find(|e| e.element == z)
    }
    fn total_weight(&self, structures: &[WeightedStructure], movable: &[(usize, usize)]) -> f64 {
        movable
            .iter()
            .map(|&(s, a)| {
                self.rule(structures[s].configuration.atoms[a].atomic_number)
                    .map_or(1., |e| e.weight)
            })
            .sum::<f64>()
            + self.collective.iter().map(|c| c.weight).sum::<f64>()
    }
    pub(super) fn propose(
        &self,
        structures: &mut [WeightedStructure],
        movable: &[(usize, usize)],
        scale: f64,
        multiplier: f64,
        rng: &mut impl Rng,
    ) -> EnsembleMove {
        let selected = if self.elements.is_empty() && self.collective.is_empty() {
            Some(movable[rng.random_range(0..movable.len())])
        } else {
            let mut choice = rng.random::<f64>() * self.total_weight(structures, movable);
            let mut selected = None;
            for &(s, a) in movable {
                choice -= self
                    .rule(structures[s].configuration.atoms[a].atomic_number)
                    .map_or(1., |e| e.weight);
                if choice < 0. {
                    selected = Some((s, a));
                    break;
                }
            }
            if selected.is_none() {
                let mut group = self.collective.last();
                for c in &self.collective {
                    choice -= c.weight;
                    if choice < 0. {
                        group = Some(c);
                        break;
                    }
                }
                // Resolve a rounded cumulative-sum endpoint to the last positive
                // collective choice, including mixtures with zero atom weights.
                let group = group.or_else(|| self.collective.last());
                if let Some(c) = group {
                    let mut displacement: [f64; 3] = std::array::from_fn(|_| {
                        c.step_size * multiplier * (2. * rng.random::<f64>() - 1.)
                    });
                    for (i, &atom) in c.atoms.iter().enumerate() {
                        if c.independent && i > 0 {
                            displacement = std::array::from_fn(|_| {
                                c.step_size * multiplier * (2. * rng.random::<f64>() - 1.)
                            });
                        }
                        for (x, delta) in structures[c.structure].configuration.atoms[atom]
                            .position
                            .iter_mut()
                            .zip(displacement)
                        {
                            *x += delta;
                        }
                    }
                    return EnsembleMove::Collective {
                        structure: c.structure,
                        atoms: c.atoms.clone(),
                    };
                }
                // Only possible at a floating-point cumulative-sum endpoint.
                selected = movable.iter().rev().copied().find(|&(s, a)| {
                    self.rule(structures[s].configuration.atoms[a].atomic_number)
                        .is_none_or(|e| e.weight > 0.)
                });
            }
            selected
        };
        let (structure, atom) = selected.expect("validated positive proposal weight");
        let atom = &mut structures[structure].configuration.atoms[atom];
        let step = self.rule(atom.atomic_number).map_or(scale, |e| e.step_size) * multiplier;
        for x in &mut atom.position {
            *x += step * (2. * rng.random::<f64>() - 1.);
        }
        EnsembleMove::Atom {
            structure,
            atom: selected.unwrap().1,
        }
    }
}

/// Optional stopping rules. Disabled by default. These detect numerical targets,
/// stagnation or a low acceptance rate; none proves structural convergence.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct StoppingSettings {
    /// Stop once the best total numerical score is at or below this finite value.
    pub target_score: Option<f64>,
    /// Stop after this many attempts without the declared score improvement.
    pub patience: Option<usize>,
    /// Nonnegative required improvement in objective units to reset patience.
    pub minimum_improvement: f64,
    /// Number of recent attempts used for low-acceptance stopping; zero disables.
    pub acceptance_window: usize,
    /// Stop when the complete window's accepted fraction is below this value.
    pub minimum_acceptance: Option<f64>,
}
impl StoppingSettings {
    pub(super) fn validate(&self) -> Result<(), RmcError> {
        require(
            self.target_score.is_none_or(f64::is_finite)
                && self.patience.is_none_or(|v| v > 0)
                && self.minimum_improvement.is_finite()
                && self.minimum_improvement >= 0.
                && self.acceptance_window <= 1_000_000
                && self.minimum_acceptance.is_none_or(|v| {
                    v.is_finite() && (0. ..=1.).contains(&v) && self.acceptance_window > 0
                }),
            "invalid RMC stopping settings",
        )
    }
}
/// Reason no further move is attempted, distinct from a calculator failure.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum StopReason {
    /// Configured total attempt limit reached.
    StepLimit,
    /// Best score reached the requested numerical target.
    TargetScore,
    /// Improvement smaller than requested for the patience interval.
    Stagnation,
    /// Complete rolling window has too few accepted moves.
    LowAcceptance,
}
/// Persistent optimizer counters. An older checkpoint starts a new observation
/// interval on resume; `attempts` states how many moves these counters cover.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct SessionDiagnostics {
    /// Attempts included in these diagnostic counters.
    pub attempts: usize,
    /// Accepted moves in the observation interval.
    pub accepted: usize,
    /// Accepted uphill moves in the observation interval.
    pub uphill: usize,
    /// Hard constraint rejections in the observation interval.
    pub constraint_rejected: usize,
    /// Absolute attempt index of the last improvement that reset patience.
    pub last_improvement: usize,
    /// Best score at the last patience reset, in objective units.
    pub improvement_anchor: f64,
    /// Bounded acceptance window, independent of retained move history.
    pub recent_acceptance: Vec<bool>,
}

/// Unreleased: bounded acceptance feedback for optimization, disabled by default
/// through `SessionSettings::adaptation = None`. This history-dependent policy
/// is not an equilibrium sampler. All coordinate widths (including species and
/// collective widths) are multiplied by the recorded scale; fraction moves are
/// unchanged and excluded from the feedback window.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct StepAdaptation {
    /// Coordinate attempts per nonoverlapping feedback window; default 100.
    pub window: usize,
    /// Shrink below this accepted fraction; default 0.1.
    pub lower_acceptance: f64,
    /// Grow above this accepted fraction; default 0.4.
    pub upper_acceptance: f64,
    /// Multiplicative growth/division factor, greater than one; default 1.1.
    pub factor: f64,
    /// Positive lower scale bound, at most one; default 0.05.
    pub minimum_scale: f64,
    /// Upper scale bound, at least one; default 1 (the declared proposal width).
    pub maximum_scale: f64,
    /// Freeze after this many coordinate attempts; None keeps adapting.
    pub freeze_after: Option<usize>,
}
impl Default for StepAdaptation {
    fn default() -> Self {
        Self {
            window: 100,
            lower_acceptance: 0.1,
            upper_acceptance: 0.4,
            factor: 1.1,
            minimum_scale: 0.05,
            maximum_scale: 1.,
            freeze_after: None,
        }
    }
}
impl StepAdaptation {
    pub(super) fn validate(&self) -> Result<(), RmcError> {
        require(
            self.window > 0
                && self.window <= 1_000_000
                && self.lower_acceptance.is_finite()
                && self.upper_acceptance.is_finite()
                && 0. <= self.lower_acceptance
                && self.lower_acceptance <= self.upper_acceptance
                && self.upper_acceptance <= 1.
                && self.factor.is_finite()
                && self.factor > 1.
                && self.factor <= 100.
                && self.minimum_scale.is_finite()
                && self.minimum_scale > 0.
                && self.minimum_scale <= 1.
                && self.maximum_scale.is_finite()
                && self.maximum_scale >= 1.
                && self.maximum_scale <= 100.,
            "invalid step adaptation",
        )
    }
}
/// Checkpointed acceptance feedback. The scale starts at one. Hard coordinate
/// rejections count as unsuccessful attempts; calculator failures count nothing.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct StepAdaptationState {
    /// Scale used by the next coordinate proposal, dimensionless.
    pub scale: f64,
    /// Completed coordinate attempts, including attempts after freezing.
    pub attempts: usize,
    /// Attempts accumulated in the current window.
    pub window_attempts: usize,
    /// Accepted attempts in the current window.
    pub window_accepted: usize,
    /// Number of completed feedback windows (including unchanged decisions).
    pub updates: usize,
    /// Accepted fraction in the last completed window, if any.
    pub last_acceptance: Option<f64>,
}
impl Default for StepAdaptationState {
    fn default() -> Self {
        Self {
            scale: 1.,
            attempts: 0,
            window_attempts: 0,
            window_accepted: 0,
            updates: 0,
            last_acceptance: None,
        }
    }
}
impl StepAdaptationState {
    pub(super) fn validate(&self, policy: Option<&StepAdaptation>) -> Result<(), RmcError> {
        let Some(p) = policy else {
            return require(self == &Self::default(), "adaptation state without policy");
        };
        p.validate()?;
        require(
            self.scale.is_finite()
                && self.scale >= p.minimum_scale
                && self.scale <= p.maximum_scale
                && self.window_attempts < p.window
                && self.window_accepted <= self.window_attempts
                && self
                    .updates
                    .checked_mul(p.window)
                    .and_then(|v| v.checked_add(self.window_attempts))
                    .is_some_and(|v| v == self.attempts.min(p.freeze_after.unwrap_or(usize::MAX)))
                && self.last_acceptance.is_some() == (self.updates > 0)
                && self
                    .last_acceptance
                    .is_none_or(|v| v.is_finite() && (0. ..=1.).contains(&v)),
            "invalid adaptation checkpoint",
        )
    }
    pub(super) fn observe(&mut self, policy: Option<&StepAdaptation>, record: &SessionStep) {
        let Some(p) = policy else {
            return;
        };
        if matches!(record.proposal, EnsembleMove::Weight { .. }) {
            return;
        }
        self.attempts += 1;
        if p.freeze_after.is_some_and(|n| self.attempts > n) {
            return;
        }
        self.window_attempts += 1;
        self.window_accepted += usize::from(record.accepted);
        if self.window_attempts == p.window {
            let fraction = self.window_accepted as f64 / p.window as f64;
            if fraction < p.lower_acceptance {
                self.scale /= p.factor;
            } else if fraction > p.upper_acceptance {
                self.scale *= p.factor;
            }
            self.scale = self.scale.clamp(p.minimum_scale, p.maximum_scale);
            self.last_acceptance = Some(fraction);
            self.updates += 1;
            self.window_attempts = 0;
            self.window_accepted = 0;
        }
    }
}
