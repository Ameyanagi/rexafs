use super::*;

/// ReFEFF settings for full or pinned-potential calculations. Defaults are numerical starting points;
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
    /// regenerated for each geometry in the full mode and only at the explicit
    /// reference in pinned-potential mode. Some configurations may fail SCF.
    pub scf_radius: Option<f64>,
    /// Optional linear-polarization vector in the Cartesian coordinate frame.
    /// Default None uses orientational averaging; nonzero vectors are normalized.
    pub polarization: Option<[f64; 3]>,
    /// Internal ReFEFF pipeline threads; default 1 for reference runs. In the
    /// prepared calculator this controls electronic preparation, not the later
    /// cached path evaluations. Use `AccelerationSettings::workers` for absorber
    /// and path concurrency; keeping this at 1 avoids competing thread pools.
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

impl RefeffOptions {
    /// Validate physical ranges, polarization and resource limits without running ReFEFF.
    pub fn validate(&self) -> Result<(), RmcError> {
        require(
            self.cluster_radius.is_finite()
                && self.cluster_radius > 0.5
                && self.cluster_radius < 50.0,
            "ReFEFF cluster radius must be between 0.5 and 50 Å",
        )?;
        require(
            self.path_radius.is_finite()
                && self.path_radius > 0.0
                && self.path_radius <= self.cluster_radius,
            "ReFEFF path radius must be positive and no larger than the cluster",
        )?;
        require(
            (2..=6).contains(&self.max_legs)
                && self.kmax.is_finite()
                && self.kmax > 0.0
                && self.kmax <= 30.0,
            "ReFEFF requires 2..=6 legs and 0<kmax<=30 Å⁻¹",
        )?;
        require(
            self.path_criteria
                .iter()
                .all(|v| v.is_finite() && *v >= 0.0 && *v <= 100.0),
            "ReFEFF path criteria must be finite percentages in [0,100]",
        )?;
        require(
            self.threads > 0
                && self.timeout_seconds.is_finite()
                && self.timeout_seconds > 0.0
                && self.timeout_seconds <= 86400.0,
            "ReFEFF needs positive threads and a timeout of at most one day",
        )?;
        if let Some(radius) = self.scf_radius {
            require(
                radius.is_finite() && radius > 0.0 && radius <= self.cluster_radius,
                "SCF radius must be positive and no larger than the cluster",
            )?;
        }
        if let Some(vector) = self.polarization {
            require(
                vector.iter().all(|x| x.is_finite())
                    && super::geometry::distance(vector, [0.0; 3]).is_finite()
                    && super::geometry::distance(vector, [0.0; 3]) > 0.0,
                "polarization must be a finite nonzero vector",
            )?;
        }
        Ok(())
    }
}
