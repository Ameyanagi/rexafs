use super::*;

/// One independently simulated structure contributing to the same measured χ.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct WeightedStructure {
    /// Explicit atoms and fixed optional periodic cell.
    pub configuration: Configuration,
    /// Nonnegative mixture fraction. Fractions are normalized at session creation.
    pub weight: f64,
    /// Movable atom indices; None uses session indices (or all atoms), Some([]) fixes this structure.
    pub movable_atoms: Option<Vec<usize>>,
}

/// EXAFS data plus an objective and independently chosen forward-model settings.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RmcDataset {
    /// Measured unweighted χ, fixed amplitude and noise scales, and initial
    /// theoretical energy shift (fixed unless session refinement is enabled).
    pub exafs: ExafsDataset,
    /// Comparison space; K reproduces the original RMC objective.
    pub objective: Objective,
    /// Per-dataset ReFEFF settings; None uses calculator defaults.
    pub refeff: Option<RefeffOptions>,
    /// One absorber list per structure. Empty uses `exafs.absorbers` for every structure.
    pub absorbers_by_structure: Vec<Vec<usize>>,
    /// Captured processing state when built by [`Self::from_spectrum`]. Older
    /// array-only jobs deserialize as None. Stored once in the problem/checkpoint,
    /// never in individual trial states; no preprocessing runs during refinement.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<Box<RmcSpectrumSource>>,
}

impl From<ExafsDataset> for RmcDataset {
    fn from(exafs: ExafsDataset) -> Self {
        Self {
            exafs,
            objective: Objective::K,
            refeff: None,
            absorbers_by_structure: Vec::new(),
            source: None,
        }
    }
}

/// A weighted mixture of structures jointly fitted to one or more datasets.
/// Components are physical mixture candidates, distinct from evolutionary individuals.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EnsembleProblem {
    /// Nonempty structures with stable component and atom indices.
    pub structures: Vec<WeightedStructure>,
    /// Nonempty datasets with unique names.
    pub datasets: Vec<RmcDataset>,
}

impl EnsembleProblem {
    /// Create a one-structure, one-dataset problem without changing its objective
    /// or processing snapshot (since 0.2.10). Takes ownership, assigns unit mixture
    /// weight, and uses the session's movable atoms. Validation runs at session
    /// creation. Add further datasets/components through the public fields.
    pub fn single(configuration: Configuration, dataset: RmcDataset) -> Self {
        Self {
            structures: vec![WeightedStructure {
                configuration,
                weight: 1.,
                movable_atoms: None,
            }],
            datasets: vec![dataset],
        }
    }
}

impl From<RmcProblem> for EnsembleProblem {
    fn from(problem: RmcProblem) -> Self {
        Self {
            structures: vec![WeightedStructure {
                configuration: problem.configuration,
                weight: 1.,
                movable_atoms: None,
            }],
            datasets: problem.datasets.into_iter().map(Into::into).collect(),
        }
    }
}

/// Borrowed request for one absorber. The structure index identifies a pinned
/// reference structure in calculators that freeze scattering potentials.
#[derive(Clone, Copy)]
pub struct CalculationRequest<'a> {
    /// Zero-based mixture component index, stable throughout a session.
    pub structure: usize,
    /// Current explicit geometry.
    pub configuration: &'a Configuration,
    /// Zero-based absorbing atom.
    pub absorber: usize,
    /// Absorption edge.
    pub edge: crate::structure::Edge,
    /// Strictly increasing theoretical wave numbers in Å⁻¹.
    pub k: &'a [f64],
    /// Dataset override of calculator defaults.
    pub options: Option<&'a RefeffOptions>,
    /// Request individual path contributions; unsupported backends must fail explicitly.
    pub paths: bool,
}

/// One backend scattering path, including its contribution before absorber
/// averaging, mixture weighting and S₀². Degeneracy is already included in χ.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PathContribution {
    /// Backend path number, local to this absorber calculation.
    pub index: usize,
    /// Number of propagation legs (two is single scattering).
    pub legs: usize,
    /// Equivalent-path multiplicity used by the backend.
    pub degeneracy: f64,
    /// Half of the total path length, in Å.
    pub half_length: f64,
    /// Unweighted dimensionless contribution on the request's k grid.
    pub chi: Vec<f64>,
}

/// One absorber's calculated χ and optional decomposition.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CalculatedSpectrum {
    /// Unweighted dimensionless χ, S₀²=1, no additional disorder damping.
    pub chi: Vec<f64>,
    /// Empty unless path output was requested.
    pub paths: Vec<PathContribution>,
}

/// Path report with sufficient indices to reconstruct mixture contributions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AbsorberPaths {
    /// Dataset index in the problem.
    pub dataset: usize,
    /// Structure index in the mixture.
    pub structure: usize,
    /// Atom index of the absorber.
    pub absorber: usize,
    /// Paths before mixture/absorber/amplitude scaling.
    pub paths: Vec<PathContribution>,
}

/// Evaluated mixture with cached component spectra. Weight moves need no new
/// scattering calculation; a coordinate move recalculates only its component.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EnsembleState {
    /// Unreleased: theoretical ΔE₀ for each dataset, in eV. Populated when energy
    /// refinement is enabled. Empty historical/fixed states use the input values;
    /// prefer [`Self::energy_shifts`] to resolve either representation.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub delta_e0: Vec<f64>,
    /// Current configurations and normalized mixture fractions.
    pub structures: Vec<WeightedStructure>,
    /// Total objective, including structural penalties, and mixture spectra.
    pub evaluation: Evaluation,
    /// Structural contribution to `evaluation.score`.
    pub penalty: f64,
    /// Unscaled absorber-averaged χ indexed by dataset, structure, k point.
    pub component_chi: Vec<Vec<Vec<f64>>>,
    /// Optional per-absorber path output; enabled through session settings.
    pub paths: Vec<AbsorberPaths>,
}

impl EnsembleState {
    /// Resolve each dataset's theoretical energy shift in eV, including fixed
    /// values from historical states. Returns an owned vector; neither input is
    /// changed. Invalid state dimensions or nonfinite shifts return an error.
    pub fn energy_shifts(&self, problem: &EnsembleProblem) -> Result<Vec<f64>, RmcError> {
        let shifts = if self.delta_e0.is_empty() {
            problem.datasets.iter().map(|d| d.exafs.delta_e0).collect()
        } else {
            require(
                self.delta_e0.len() == problem.datasets.len(),
                "state ΔE₀ count differs from datasets",
            )?;
            self.delta_e0.clone()
        };
        require(shifts.iter().all(|v| v.is_finite()), "nonfinite state ΔE₀")?;
        Ok(shifts)
    }
}
