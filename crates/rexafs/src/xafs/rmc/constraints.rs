use super::analysis::{bond_distance, visit_neighbors};
use super::geometry::distance;
use super::*;

/// Species-specific hard exclusion, overriding the session's global minimum.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PairDistance {
    /// Unordered atomic-number pair; identical elements are allowed.
    pub elements: [u8; 2],
    /// Minimum allowed distance in Å, including periodic images.
    pub minimum: f64,
}
/// Species-specific displacement bound measured from the original coordinates.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ElementDisplacement {
    /// Atomic number.
    pub element: u8,
    /// Maximum Euclidean displacement in Å; overrides the session default.
    pub maximum: f64,
}
/// Fixed labeled pair with optional hard bounds and a harmonic distance penalty.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BondRestraint {
    /// Mixture component index.
    pub structure: usize,
    /// Two distinct atom indices; periodic cells use their nearest separation.
    pub atoms: [usize; 2],
    /// Optional inclusive `[minimum, maximum]` separation in Å.
    pub bounds: Option<[f64; 2]>,
    /// Reference separation r₀ in Å.
    pub target: f64,
    /// Nonnegative coefficient λ in Å⁻²; adds λ(r−r₀)² to the objective.
    pub strength: f64,
}
/// Penalty on the number of neighbors of a chosen element inside a sphere.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CoordinationRestraint {
    /// Mixture component index.
    pub structure: usize,
    /// Central atom index.
    pub atom: usize,
    /// Neighbor atomic number.
    pub element: u8,
    /// Exclusive neighbor cutoff in Å, including periodic images.
    pub cutoff: f64,
    /// Nonnegative target coordination; fractional targets are allowed.
    pub target: f64,
    /// Nonnegative coefficient adding strength × (count−target)².
    pub strength: f64,
}
/// Truncated, energy-shifted Lennard–Jones pair restraint. This is a numerical
/// regularizer, not a validated force field. ε is expressed in objective units.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LennardJones {
    /// Unordered atomic-number pair, applied to every structure.
    pub elements: [u8; 2],
    /// Positive length σ in Å; pair minimum occurs at 2^(1/6)σ.
    pub sigma: f64,
    /// Nonnegative well depth ε in objective units.
    pub epsilon: f64,
    /// Exclusive cutoff in Å; subtracts V(cutoff) below this radius.
    pub cutoff: f64,
}
/// Optional restraint on arithmetic mean-square atomic displacement (unreleased).
/// This is an explicitly chosen structural prior, not experimental evidence.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MsdRestraint {
    /// Mixture component index; the penalty is independent of mixture weight.
    pub structure: usize,
    /// Explicit fixed-topology displacement reference; copied into checkpoints.
    pub reference: Configuration,
    /// Distinct atom indices included in the mean.
    pub atoms: Vec<usize>,
    /// Remove mean translation before computing squared displacement if true.
    pub remove_translation: bool,
    /// Nonnegative target mean-square displacement in Å².
    pub target: f64,
    /// Nonnegative coefficient λ in Å⁻⁴; adds λ(MSD−target)².
    pub strength: f64,
}
/// Optional restraint on the empirical distribution of explicit path half-lengths.
/// All listed directed paths have equal weight. This numerical prior is opt-in;
/// it does not implement EVAX's INVERT or maximum-likelihood workflows.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PathDistributionRestraint {
    /// Mixture component index.
    pub structure: usize,
    /// Fixed nonempty list of explicit paths and periodic images.
    pub paths: Vec<ScatteringPath>,
    /// Strictly increasing nonnegative half-length bin edges in Å, final edge exclusive.
    pub edges: Vec<f64>,
    /// Nonnegative target bin fractions, summing to one.
    pub fractions: Vec<f64>,
    /// Nonnegative objective coefficient. Adds strength times the sum of squared
    /// bin-fraction differences plus squared underflow/overflow fractions. Counts
    /// are divided by all listed paths, so escaped probability mass is penalized.
    pub strength: f64,
}
/// Serializable hard rules and structural energies. Penalties sum over structures
/// independently of mixture fractions, so zero fractions cannot disable a restraint.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Constraints {
    /// Element-pair minimum distances; at most one rule per unordered pair.
    pub pairs: Vec<PairDistance>,
    /// Element displacement bounds; at most one rule per element.
    pub displacements: Vec<ElementDisplacement>,
    /// Labeled bond distances and optional hard ranges.
    pub bonds: Vec<BondRestraint>,
    /// Neighbor-count penalties.
    pub coordination: Vec<CoordinationRestraint>,
    /// Pair-energy regularizers; signed energies may make the total score negative.
    pub lennard_jones: Vec<LennardJones>,
    /// Opt-in mean-square displacement priors with explicit fixed references.
    pub msd: Vec<MsdRestraint>,
    /// Opt-in path half-length histogram priors.
    pub path_distributions: Vec<PathDistributionRestraint>,
}
fn pair(a: [u8; 2], b: [u8; 2]) -> bool {
    a == b || a == [b[1], b[0]]
}
fn element(z: u8) -> bool {
    crate::structure::Element::from_z(z).is_some()
}
impl Constraints {
    pub(super) fn validate(&self, structures: &[WeightedStructure]) -> Result<(), RmcError> {
        for m in &self.msd {
            require(
                m.structure < structures.len()
                    && m.target.is_finite()
                    && m.target >= 0.
                    && m.strength.is_finite()
                    && m.strength >= 0.,
                "invalid MSD restraint",
            )?;
            displacement_report(
                &m.reference,
                &structures[m.structure].configuration,
                &m.atoms,
                m.remove_translation,
            )?;
        }
        for p in &self.path_distributions {
            require(
                p.structure < structures.len()
                    && !p.paths.is_empty()
                    && p.paths.len() <= 1_000_000
                    && p.strength.is_finite()
                    && p.strength >= 0.
                    && p.edges.iter().all(|v| *v >= 0.)
                    && p.edges.len() == p.fractions.len() + 1
                    && p.fractions.iter().all(|v| v.is_finite() && *v >= 0.)
                    && (p.fractions.iter().sum::<f64>() - 1.).abs() < 1e-10,
                "invalid path-distribution restraint",
            )?;
            path_length_distribution(&structures[p.structure].configuration, &p.paths, &p.edges)?;
        }
        for (i, p) in self.pairs.iter().enumerate() {
            require(
                p.elements.iter().all(|&z| element(z))
                    && p.minimum.is_finite()
                    && p.minimum > 0.
                    && !self.pairs[..i].iter().any(|q| pair(p.elements, q.elements)),
                "invalid or duplicate pair-distance rule",
            )?;
        }
        for (i, d) in self.displacements.iter().enumerate() {
            require(
                element(d.element)
                    && d.maximum.is_finite()
                    && d.maximum > 0.
                    && !self.displacements[..i]
                        .iter()
                        .any(|v| v.element == d.element),
                "invalid or duplicate displacement rule",
            )?;
        }
        for b in &self.bonds {
            require(
                b.structure < structures.len()
                    && b.atoms[0] != b.atoms[1]
                    && b.atoms
                        .iter()
                        .all(|&a| a < structures[b.structure].configuration.atoms.len())
                    && b.target.is_finite()
                    && b.target > 0.
                    && b.strength.is_finite()
                    && b.strength >= 0.
                    && b.bounds.is_none_or(|v| {
                        v.iter().all(|x| x.is_finite() && *x >= 0.) && v[1] >= v[0]
                    }),
                "invalid bond restraint",
            )?;
        }
        for c in &self.coordination {
            require(
                c.structure < structures.len()
                    && c.atom < structures[c.structure].configuration.atoms.len()
                    && element(c.element)
                    && c.cutoff.is_finite()
                    && c.cutoff > 0.
                    && c.target.is_finite()
                    && c.target >= 0.
                    && c.strength.is_finite()
                    && c.strength >= 0.,
                "invalid coordination restraint",
            )?;
        }
        for l in &self.lennard_jones {
            require(
                l.elements.iter().all(|&z| element(z))
                    && l.sigma.is_finite()
                    && l.sigma > 0.
                    && l.epsilon.is_finite()
                    && l.epsilon >= 0.
                    && l.cutoff.is_finite()
                    && l.cutoff > 0.,
                "invalid Lennard-Jones restraint",
            )?;
        }
        Ok(())
    }
    pub(super) fn allowed(
        &self,
        structures: &[WeightedStructure],
        original: &[WeightedStructure],
        settings: &RmcSettings,
        moved: Option<(usize, usize)>,
    ) -> Result<bool, RmcError> {
        let radius = self
            .pairs
            .iter()
            .map(|p| p.minimum)
            .fold(settings.min_distance, f64::max);
        for (s, structure) in structures.iter().enumerate() {
            if moved.is_some_and(|(t, _)| s != t) {
                continue;
            }
            let c = &structure.configuration;
            let indices: Vec<_> =
                moved.map_or_else(|| (0..c.atoms.len()).collect(), |(_, a)| vec![a]);
            for i in indices {
                let a = &c.atoms[i];
                let limit = self
                    .displacements
                    .iter()
                    .find(|d| d.element == a.atomic_number)
                    .map(|d| d.maximum)
                    .or(settings.max_displacement);
                if !a.position.iter().all(|v| v.is_finite() && v.abs() <= 1e8)
                    || limit.is_some_and(|l| {
                        distance(a.position, original[s].configuration.atoms[i].position) > l
                    })
                {
                    return Ok(false);
                }
                if !visit_neighbors(c, i, radius, |j, r| {
                    r >= self
                        .pairs
                        .iter()
                        .find(|p| pair(p.elements, [a.atomic_number, c.atoms[j].atomic_number]))
                        .map_or(settings.min_distance, |p| p.minimum)
                })? {
                    return Ok(false);
                }
            }
        }
        for b in &self.bonds {
            if moved.is_none_or(|(s, a)| s == b.structure && b.atoms.contains(&a)) {
                if let Some([lo, hi]) = b.bounds {
                    let r = bond_distance(
                        &structures[b.structure].configuration,
                        b.atoms[0],
                        b.atoms[1],
                    )?;
                    if r < lo || r > hi {
                        return Ok(false);
                    }
                }
            }
        }
        Ok(true)
    }
    /// Calculate structural energies only. Checks indices, configuration validity
    /// and parameters; hard constraints are separately enforced by the session.
    pub fn penalty(&self, structures: &[WeightedStructure]) -> Result<f64, RmcError> {
        for s in structures {
            s.configuration.validate()?;
        }
        self.validate(structures)?;
        self.energy(structures)
    }
    pub(super) fn energy(&self, structures: &[WeightedStructure]) -> Result<f64, RmcError> {
        let mut energy = 0.;
        for m in &self.msd {
            if m.strength > 0. {
                let report = displacement_report(
                    &m.reference,
                    &structures[m.structure].configuration,
                    &m.atoms,
                    m.remove_translation,
                )?;
                energy += m.strength * (report.mean_square - m.target).powi(2);
            }
        }
        for p in &self.path_distributions {
            if p.strength > 0. {
                let report = path_length_distribution(
                    &structures[p.structure].configuration,
                    &p.paths,
                    &p.edges,
                )?;
                let n = report.count as f64;
                let bins = report
                    .counts
                    .iter()
                    .zip(&p.fractions)
                    .map(|(&count, &target)| (count as f64 / n - target).powi(2))
                    .sum::<f64>();
                energy += p.strength
                    * (bins
                        + (report.underflow as f64 / n).powi(2)
                        + (report.overflow as f64 / n).powi(2));
            }
        }
        for b in &self.bonds {
            if b.strength > 0. {
                energy += b.strength
                    * (bond_distance(
                        &structures[b.structure].configuration,
                        b.atoms[0],
                        b.atoms[1],
                    )? - b.target)
                        .powi(2);
            }
        }
        for c in &self.coordination {
            let conf = &structures[c.structure].configuration;
            let mut n = 0.;
            visit_neighbors(conf, c.atom, c.cutoff, |j, _| {
                if conf.atoms[j].atomic_number == c.element {
                    n += 1.;
                }
                true
            })?;
            energy += c.strength * (n - c.target).powi(2);
        }
        for l in &self.lennard_jones {
            if l.epsilon == 0. {
                continue;
            }
            let potential = |r: f64| {
                let v = (l.sigma / r).powi(6);
                4. * l.epsilon * (v * v - v)
            };
            let shift = potential(l.cutoff);
            for s in structures {
                let c = &s.configuration;
                for (i, a) in c.atoms.iter().enumerate() {
                    if !l.elements.contains(&a.atomic_number) {
                        continue;
                    }
                    visit_neighbors(c, i, l.cutoff, |j, r| {
                        if pair(l.elements, [a.atomic_number, c.atoms[j].atomic_number]) {
                            energy += 0.5 * (potential(r) - shift);
                        }
                        true
                    })?;
                }
            }
        }
        require(
            energy.is_finite(),
            "structural energy overflow or coincident atoms",
        )?;
        Ok(energy)
    }
}
