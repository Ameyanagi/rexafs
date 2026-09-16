use super::geometry::{distance, image_bounds};
use super::*;
use crate::structure::{
    build_cluster, AbsorberSelection, Cluster, ClusterOptions, Element, OccupancyPolicy, Site,
    Structure, Xyz, XyzAbsorber, XyzAtom,
};
use ::refeff::{ArtifactSelection, CancellationToken, MemoryRunRequest, Runner};
use std::collections::BTreeSet;
use std::num::NonZeroUsize;
use std::time::{Duration, Instant};

/// ReFEFF settings for full recalculation. Defaults are numerical starting points;
/// converge cluster radius, path radius and scattering order for the sample.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct RefeffOptions {
    /// Radius of atoms supplied around each absorber in Å; default 6, range (0.5,50).
    pub cluster_radius: f64,
    /// Maximum half-path length in Å (RPATH); default 4, no larger than the cluster.
    pub path_radius: f64,
    /// Maximum scattering-path leg count, 2..=6; default 4. Two means single
    /// scattering only. Higher orders can increase runtime sharply.
    pub max_legs: u8,
    /// FEFF CRITERIA percentages [curved-wave, plane-wave]; default [4.0, 2.5].
    /// These screen weak paths. [0.0, 0.0] disables these two filters for small
    /// convergence tests, potentially increasing cost substantially. A leg-count
    /// limit above two does not guarantee that multiple-scattering paths survive.
    pub path_criteria: [f64; 2],
    /// EXAFS calculation limit in Å⁻¹; default 16, range (0,30]. The actual returned
    /// k support is checked, and requested points outside it cause an error.
    pub kmax: f64,
    /// Optional self-consistent-field radius in Å, no larger than the cluster.
    /// Default None uses ReFEFF's non-SCF potentials. Potentials and scattering are
    /// still regenerated for each geometry. Some configurations may fail SCF.
    pub scf_radius: Option<f64>,
    /// Optional linear-polarization vector in the Cartesian coordinate frame.
    /// Default None uses orientational averaging; nonzero vectors are normalized.
    pub polarization: Option<[f64; 3]>,
    /// Worker count passed to ReFEFF; default 1 for repeatable reference runs.
    pub threads: usize,
    /// Cooperative timeout per absorber calculation in seconds; default 300.
    /// Timeout aborts refinement with an error, rather than rejecting a move.
    pub timeout_seconds: f64,
}

impl Default for RefeffOptions {
    fn default() -> Self {
        Self {
            cluster_radius: 6.0,
            path_radius: 4.0,
            max_legs: 4,
            path_criteria: [4.0, 2.5],
            kmax: 16.0,
            scf_radius: None,
            polarization: None,
            threads: 1,
            timeout_seconds: 300.0,
        }
    }
}

/// Primary RMC calculator, using ReFEFF in the current process. Each call requests
/// a fresh EXAFS pipeline with `CONTROL 1 1 1 1 1 1`; no reference-geometry
/// amplitudes, Debye–Waller factors or intermediate caches are reused. ReFEFF
/// internally uses temporary workspace files, removed by its runner.
pub struct RefeffCalculator {
    options: RefeffOptions,
    cancellation: CancellationToken,
    diagnostics: BTreeSet<String>,
}

impl RefeffCalculator {
    /// Validate options and create a calculator. No scattering calculation runs
    /// here. Invalid radii, scattering orders, vectors and resource limits fail.
    pub fn new(options: RefeffOptions) -> Result<Self, RmcError> {
        require(
            options.cluster_radius.is_finite()
                && options.cluster_radius > 0.5
                && options.cluster_radius < 50.0,
            "ReFEFF cluster radius must be between 0.5 and 50 Å",
        )?;
        require(
            options.path_radius.is_finite()
                && options.path_radius > 0.0
                && options.path_radius <= options.cluster_radius,
            "ReFEFF path radius must be positive and no larger than the cluster",
        )?;
        require(
            (2..=6).contains(&options.max_legs)
                && options.kmax.is_finite()
                && options.kmax > 0.0
                && options.kmax <= 30.0,
            "ReFEFF requires 2..=6 legs and 0<kmax<=30 Å⁻¹",
        )?;
        require(
            options
                .path_criteria
                .iter()
                .all(|v| v.is_finite() && *v >= 0.0 && *v <= 100.0),
            "ReFEFF path criteria must be finite percentages in [0,100]",
        )?;
        require(
            options.threads > 0
                && options.timeout_seconds.is_finite()
                && options.timeout_seconds > 0.0
                && options.timeout_seconds <= 86400.0,
            "ReFEFF needs positive threads and a timeout of at most one day",
        )?;
        if let Some(radius) = options.scf_radius {
            require(
                radius.is_finite() && radius > 0.0 && radius <= options.cluster_radius,
                "SCF radius must be positive and no larger than the cluster",
            )?;
        }
        if let Some(vector) = options.polarization {
            require(
                vector.iter().all(|x| x.is_finite())
                    && distance(vector, [0.0; 3]).is_finite()
                    && distance(vector, [0.0; 3]) > 0.0,
                "polarization must be a finite nonzero vector",
            )?;
        }
        Ok(Self {
            options,
            cancellation: CancellationToken::default(),
            diagnostics: BTreeSet::new(),
        })
    }

    /// Borrow the validated options, which remain fixed for this calculator.
    pub fn options(&self) -> &RefeffOptions {
        &self.options
    }

    /// Clone the shared cancellation token. Another thread can call `cancel()`
    /// to interrupt ReFEFF cooperatively. Cancellation returns a calculator error;
    /// create a new calculator to start a fresh uncancelled run.
    pub fn cancellation_token(&self) -> CancellationToken {
        self.cancellation.clone()
    }

    /// Distinct nonfatal diagnostics accumulated from completed ReFEFF runs.
    /// Save these with results; a successful calculation can still have warnings.
    pub fn diagnostics(&self) -> &BTreeSet<String> {
        &self.diagnostics
    }

    /// Generate the exact FEFF-compatible input for one absorber, without running
    /// ReFEFF. Useful for auditing scientific settings. S₀² is fixed at one here;
    /// the RMC engine applies each dataset's calibrated amplitude afterward.
    /// Coordinates are written to 12 decimal places in Å.
    pub fn input_for(
        &self,
        configuration: &Configuration,
        absorber: usize,
        edge: Edge,
    ) -> Result<String, RmcError> {
        configuration.validate()?;
        require(
            absorber < configuration.atoms.len(),
            "absorber index out of range",
        )?;
        let cluster = self.cluster(configuration, absorber)?;
        require(
            cluster.atoms.len() > 1,
            "absorber cluster contains no scattering neighbors",
        )?;
        require(
            cluster.atoms.len() <= 1000,
            "reference RMC backend limits absorber clusters to 1000 atoms",
        )?;
        require(
            cluster.atoms.iter().skip(1).all(|a| a.distance > 1e-6),
            "absorber coincides with a scattering atom",
        )?;
        let mut input = format!("TITLE rexafs RMC full geometry calculation\nHOLE {} 1.0\nCONTROL 1 1 1 1 1 1\nPRINT 0 0 0 0 0 3\nRPATH {:.12}\nNLEG {}\nEXAFS {:.12}\n",
            edge.hole_index(), self.options.path_radius, self.options.max_legs, self.options.kmax);
        input.push_str(&format!(
            "CRITERIA {:.12} {:.12}\n",
            self.options.path_criteria[0], self.options.path_criteria[1]
        ));
        if let Some(radius) = self.options.scf_radius {
            input.push_str(&format!("SCF {radius:.12}\n"));
        }
        if let Some(vector) = self.options.polarization {
            let norm = distance(vector, [0.0; 3]);
            let [x, y, z] = vector.map(|v| v / norm);
            input.push_str(&format!("POLARIZATION {x:.12} {y:.12} {z:.12}\n"));
        }
        input.push_str("POTENTIALS\n");
        for potential in &cluster.potentials {
            input.push_str(&format!(
                "{} {} {}\n",
                potential.ipot, potential.z, potential.symbol
            ));
        }
        input.push_str("ATOMS\n");
        for atom in &cluster.atoms {
            let [x, y, z] = atom.cart;
            input.push_str(&format!(
                "{x:.12} {y:.12} {z:.12} {} {}\n",
                atom.ipot, atom.symbol
            ));
        }
        input.push_str("END\n");
        Ok(input)
    }

    fn cluster(&self, configuration: &Configuration, absorber: usize) -> Result<Cluster, RmcError> {
        let radius = self.options.cluster_radius;
        let result = if let Some(lattice) = configuration.lattice()? {
            let bounds = image_bounds(&lattice, radius)?;
            let candidates = bounds
                .iter()
                .map(|n| (2 * n + 1) as usize)
                .product::<usize>();
            require(
                candidates.saturating_mul(configuration.atoms.len()) <= 2_000_000,
                "periodic cluster enumeration exceeds the reference backend's resource limit",
            )?;
            let sites = configuration
                .atoms
                .iter()
                .enumerate()
                .map(|(i, a)| {
                    Site::new(
                        &format!("atom{i}"),
                        Element::from_z(a.atomic_number).unwrap().symbol,
                        lattice.to_frac(a.position).map(|v| v.rem_euclid(1.0)),
                    )
                })
                .collect();
            let structure = Structure::new("RMC explicit periodic cell", lattice, sites);
            build_cluster(
                &structure,
                &AbsorberSelection::SiteIndex(absorber),
                &ClusterOptions {
                    radius,
                    include_hydrogen: true,
                    occupancy: OccupancyPolicy::Majority,
                },
            )
        } else {
            let xyz = Xyz {
                comment: "RMC explicit finite cluster".into(),
                atoms: configuration
                    .atoms
                    .iter()
                    .map(|a| XyzAtom {
                        symbol: Element::from_z(a.atomic_number).unwrap().symbol.into(),
                        z: a.atomic_number,
                        cart: a.position,
                    })
                    .collect(),
            };
            xyz.to_cluster(&XyzAbsorber::Index(absorber), Some(radius))
        };
        result.map_err(|e| RmcError::Invalid(e.to_string()))
    }
}

impl ExafsCalculator for RefeffCalculator {
    fn name(&self) -> &str {
        "ReFEFF (full geometry recalculation)"
    }

    fn calculate(
        &mut self,
        configuration: &Configuration,
        absorber: usize,
        edge: Edge,
        k: &[f64],
    ) -> Result<Vec<f64>, RmcError> {
        require(
            k.len() >= 2
                && k.iter()
                    .all(|v| v.is_finite() && *v >= 0.0 && *v <= self.options.kmax)
                && k.windows(2).all(|w| w[1] > w[0]),
            "requested ReFEFF k grid is invalid or exceeds kmax",
        )?;
        let input = self.input_for(configuration, absorber, edge)?;
        let output = Runner::new()
            .with_threads(NonZeroUsize::new(self.options.threads).unwrap())
            .with_cancellation(self.cancellation.clone())
            .with_deadline(Instant::now() + Duration::from_secs_f64(self.options.timeout_seconds))
            .with_artifacts(ArtifactSelection::None)
            .run_in_memory(MemoryRunRequest::new(input.into_bytes()))
            .map_err(|e| {
                RmcError::Calculator(format!("absorber {absorber}, {} edge: {e}", edge.label()))
            })?;
        for diagnostic in &output.report.diagnostics {
            self.diagnostics
                .insert(format!("{}: {}", diagnostic.code, diagnostic.message));
        }
        let chi = output
            .spectra
            .chi
            .ok_or_else(|| RmcError::Calculator("ReFEFF returned no chi spectrum".into()))?;
        interpolate(&chi.wave_number.to_vec(), &chi.chi.to_vec(), k)
    }
}

fn interpolate(grid: &[f64], values: &[f64], query: &[f64]) -> Result<Vec<f64>, RmcError> {
    require(
        grid.len() >= 2
            && grid.len() == values.len()
            && grid.iter().chain(values).all(|v| v.is_finite())
            && grid.windows(2).all(|w| w[1] > w[0]),
        "ReFEFF returned invalid chi arrays",
    )?;
    let mut result = Vec::with_capacity(query.len());
    for &q in query {
        require(
            q >= grid[0] && q <= grid[grid.len() - 1],
            format!(
                "requested k={q} outside calculated support [{}, {}] Å⁻¹",
                grid[0],
                grid[grid.len() - 1]
            ),
        )?;
        let high = grid.partition_point(|&k| k < q).clamp(1, grid.len() - 1);
        let low = high - 1;
        let t = (q - grid[low]) / (grid[high] - grid[low]);
        result.push((1.0 - t) * values[low] + t * values[high]);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interpolation_preserves_endpoints_and_forbids_extrapolation() {
        assert_eq!(
            interpolate(&[1.0, 2.0, 3.0], &[4.0, 8.0, 4.0], &[1.0, 1.5, 3.0]).unwrap(),
            vec![4.0, 6.0, 4.0]
        );
        assert!(interpolate(&[1.0, 2.0], &[0.0, 1.0], &[2.01]).is_err());
        assert!(interpolate(&[1.0, 1.0], &[0.0, 1.0], &[1.0]).is_err());
    }
}
