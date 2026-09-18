//! First-shell calibration through the ordinary REXAFS path fitter (since 0.2.10).
use super::*;
use crate::fitting::{
    feffit_joint_with_options, FeffFitDataset, FeffFitOptions, FeffFitResult, FitSpace,
    FitVariable, FitVariables, PathParamSpec,
};

/// Four-parameter first-shell fit. These empirical starting values/bounds must
/// be chosen for the reference material; they are not universal physical priors.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct FirstShellSettings {
    /// Initial S₀² (dimensionless), ΔE₀ (eV), distance ratio, and σ² (Å²), in order.
    pub initial: [f64; 4],
    /// Inclusive bounds in the same order and units as `initial`.
    pub bounds: [[f64; 2]; 4],
    /// Ordinary fitting solver settings; no second optimizer is implemented here.
    pub solver: FeffFitOptions,
}
impl Default for FirstShellSettings {
    fn default() -> Self {
        Self {
            initial: [0.9, 0., 1., 0.003],
            bounds: [[0.1, 1.5], [-20., 20.], [0.8, 1.2], [0., 0.04]],
            solver: FeffFitOptions::default(),
        }
    }
}
/// Complete, serializable calibration provenance. The fitted distance/disorder
/// describe the first-shell path model; transferring them into explicit atomic
/// configurations requires a separate structural choice. Never multiply RMC
/// spectra by an extra fitted Debye–Waller factor when coordinate disorder is
/// already represented explicitly.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FirstShellCalibration {
    /// User-supplied material, measurement and scattering-reference provenance.
    pub source: String,
    /// Copied input with the shared four-parameter path expressions applied.
    pub dataset: FeffFitDataset,
    /// Declared starting values, bounds and solver.
    pub settings: FirstShellSettings,
    /// Native fitter output including numerical convergence and parameter errors.
    pub fit: FeffFitResult,
}
/// Fit S₀², ΔE₀, a common distance ratio and σ² using the tested ordinary fitter.
/// Copies the dataset; all active paths must be two-leg paths with positive
/// fixed degeneracy. Their old corrections are replaced by the four declared
/// parameters, with ΔR = reff*(distance_ratio−1); imaginary energy and higher
/// cumulants are zero. Disabled paths are retained but ignored. The caller selects
/// the first shell and its R range. Complex R (both real and imaginary components)
/// is required; k windows, noise scales and weights are used exactly as supplied.
/// An Ok result may still report optimizer nonconvergence in `fit.solver_report`.
/// Paths can be produced by ReFEFF and loaded through the standard fitting API.
/// No external calculator, coordinate mutation or RMC parameter transfer runs here.
pub fn calibrate_first_shell(
    dataset: &FeffFitDataset,
    settings: &FirstShellSettings,
    source: impl Into<String>,
) -> Result<FirstShellCalibration, RmcError> {
    let source = source.into();
    require(
        !source.trim().is_empty(),
        "first-shell calibration needs source provenance",
    )?;
    require(
        dataset.transform.fitspace == FitSpace::R,
        "first-shell calibration requires the native complex-R objective",
    )?;
    for (&initial, bounds) in settings.initial.iter().zip(&settings.bounds) {
        require(
            initial.is_finite()
                && bounds.iter().all(|v| v.is_finite())
                && bounds[0] < bounds[1]
                && initial >= bounds[0]
                && initial <= bounds[1],
            "invalid first-shell calibration starts or bounds",
        )?;
    }
    require(
        settings.bounds[0][0] > 0. && settings.bounds[2][0] > 0. && settings.bounds[3][0] >= 0.,
        "amplitude/distance must be positive and disorder nonnegative",
    )?;
    let mut dataset = dataset.clone();
    require(
        dataset.paths.iter().any(|p| p.use_path),
        "first-shell calibration needs enabled paths",
    )?;
    for path in dataset.paths.iter_mut().filter(|p| p.use_path) {
        require(
            path.feff.nleg == 2
                && path.feff.reff.is_finite()
                && path.feff.reff > 0.
                && matches!(path.degen, PathParamSpec::Value(v) if v.is_finite() && v > 0.),
            "first-shell calibration needs two-leg paths and fixed positive degeneracy",
        )?;
        path.s02 = "rmc_s02".into();
        path.e0 = "rmc_delta_e0".into();
        path.deltar = format!("{:.17} * (rmc_distance_scale - 1)", path.feff.reff).into();
        path.sigma2 = "rmc_sigma2".into();
        path.ei = 0.0.into();
        path.third = 0.0.into();
        path.fourth = 0.0.into();
    }
    let mut variables = FitVariables::new();
    for (i, name) in [
        "rmc_s02",
        "rmc_delta_e0",
        "rmc_distance_scale",
        "rmc_sigma2",
    ]
    .iter()
    .enumerate()
    {
        variables.insert(
            *name,
            FitVariable::new(settings.initial[i], true)
                .with_bounds(Some(settings.bounds[i][0]), Some(settings.bounds[i][1])),
        );
    }
    let fit =
        feffit_joint_with_options(std::slice::from_ref(&dataset), &variables, &settings.solver)
            .map_err(|e| RmcError::Invalid(format!("first-shell fit: {e}")))?;
    Ok(FirstShellCalibration {
        source,
        dataset,
        settings: settings.clone(),
        fit,
    })
}
impl FirstShellCalibration {
    /// Copy a problem and transfer only the fitted S₀² and ΔE₀ to one dataset.
    /// Refuses an unconverged fit or nonfinite values. Geometry, explicit disorder,
    /// objective and other datasets are unchanged. Retain this calibration object
    /// with the new job: it records the first-shell model and fitted nuisance
    /// parameters. A successful solver is not proof of a physically adequate fit.
    pub fn apply_amplitude_energy(
        &self,
        problem: &EnsembleProblem,
        dataset: usize,
    ) -> Result<EnsembleProblem, RmcError> {
        require(
            dataset < problem.datasets.len(),
            "calibration target dataset out of range",
        )?;
        require(
            self.fit.solver_report.as_ref().is_some_and(|r| r.converged),
            "first-shell solver has not converged; inspect the retained fit",
        )?;
        let parameter = |name| {
            self.fit
                .variables
                .get(name)
                .map(|p| p.value)
                .filter(|v| v.is_finite())
                .ok_or_else(|| RmcError::Invalid(format!("missing finite fitted {name}")))
        };
        let s02 = parameter("rmc_s02")?;
        let delta_e0 = parameter("rmc_delta_e0")?;
        require(s02 > 0., "fitted amplitude must be positive")?;
        let mut result = problem.clone();
        result.datasets[dataset].exafs.s02 = s02;
        result.datasets[dataset].exafs.delta_e0 = delta_e0;
        super::engine::shifted_grid(&result.datasets[dataset].exafs)?;
        Ok(result)
    }
}
