//! Fit EXAFS spectra with sums of theoretical scattering paths.
//!
//! Start with [`FeffFit`], [`FeffFitDataset`], and [`Param`]. Inputs are already
//! background-subtracted χ(k), with k in Å⁻¹; this module does not normalize
//! raw absorption automatically. A path carries fixed FEFF amplitudes/phases
//! and adjustable amplitude, energy, distance, and disorder terms. A fit
//! evaluates expressions, sums enabled paths, transforms data and model into
//! the selected [`FitSpace`], and minimizes their scaled squared discrepancy.
//!
//! [`FeffFitTransform`] documents numerical defaults; choose measured k and
//! physically justified R ranges instead of assuming those defaults suit every
//! sample. Joint fits share named variables across datasets; independent fits
//! start each dataset from the same copied initial variables. Repeated k-weights
//! and zero padding do not add independent information.
//!
//! [`FeffFitResult`] reports local covariance and numerical termination, not
//! proof that a structural model is unique. See the
//! [fit-statistics guide](https://rexafs.com/docs/science/fitting-statistics/)
//! for the implemented scaling and assumptions. For scattering theory see
//! [Rehr and Albers (2000)](https://doi.org/10.1103/RevModPhys.72.621);
//! [Larch's path reference](https://xraypy.github.io/xraylarch/xafs_feffpaths.html)
//! explains the related path-parameter convention.

pub mod builder;
pub mod errors;
pub mod expression;
pub mod feffdat;
pub mod path_model;
pub mod runner;
pub mod solver;
pub mod template;
pub mod transform;
pub mod types;
pub mod variables;

use nalgebra::DVector;

pub use builder::FeffFit;
pub use errors::FittingError;
pub use path_model::FF2ChiOutput;
pub use template::{
    apply_template, ParameterTemplate, PathAssignment, TemplateResult, TemplateVariable,
};
pub use transform::{estimate_noise, NoiseEstimate};
pub use types::{
    DatasetResult, FeffBatchExecutionStrategy, FeffBatchOptions, FeffDat, FeffExecutionMode,
    FeffFitDataset, FeffFitJacobianMode, FeffFitOptions, FeffFitResult, FeffFitSolverMethod,
    FeffFitTransform, FeffFlavor, FeffModuleCommand, FeffPathModel, FeffResolvedCommands,
    FeffRunRequest, FeffRunResult, FitSolverReport, FitSpace, FitVariable, FitVariables,
    FitWarning, KweightResult, Param, PathContribution, PathParamSpec,
};

use crate::xafs::{Result, XAFSError};

/// Read a FEFF85L-format path file into owned arrays and geometry.
///
/// Use [`FeffFlavor::Feff85L`] for compatible files, including those produced
/// by the bundled FEFF10/ReFEFF runners. The `Feff10` parser flavor is currently
/// unsupported. File I/O, malformed data, or unsupported flavors return an error.
pub fn parse_feff_path_file(path: &str, flavor: FeffFlavor) -> Result<FeffDat> {
    feffdat::parse_feff_path_file(path, flavor).map_err(XAFSError::from)
}

/// Read a path with file degeneracy, S₀² = 1, and all other corrections zero.
/// The returned model owns its data; see [`parse_feff_path_file`] for format errors.
pub fn feffpath(path: &str, flavor: FeffFlavor) -> Result<FeffPathModel> {
    path_model::feffpath(path, flavor).map_err(XAFSError::from)
}

/// Evaluate one path as dimensionless χ(k) on the supplied k grid in Å⁻¹.
///
/// Resolves variable expressions without modifying the path or variables.
/// Requires at least three finite k samples and valid FEFF arrays. Unresolved
/// expressions and invalid path data return an error. See [`path_model`] for
/// interpolation, low-k guards, and the first-sample extrapolation convention.
pub fn path2chi(
    path: &FeffPathModel,
    vars: &FitVariables,
    k: &DVector<f64>,
) -> Result<DVector<f64>> {
    path_model::path2chi(path, vars, k).map_err(XAFSError::from)
}

/// Sum enabled paths, returning total and individual dimensionless χ(k) arrays.
///
/// Uses the supplied grid in Å⁻¹ without applying k-weights or windows. Returns
/// an error when no paths are enabled or any enabled path fails evaluation.
pub fn ff2chi(
    paths: &[FeffPathModel],
    vars: &FitVariables,
    k: &DVector<f64>,
) -> Result<FF2ChiOutput> {
    path_model::ff2chi(paths, vars, k).map_err(XAFSError::from)
}

/// Fit all datasets jointly with shared variable names and default solver options.
/// Inputs are borrowed and remain unchanged; the result owns fitted values/arrays.
/// See [`feffit_joint_with_options`] for interpretation and errors.
pub fn feffit_joint(datasets: &[FeffFitDataset], vars: &FitVariables) -> Result<FeffFitResult> {
    solver::feffit_joint(datasets, vars).map_err(XAFSError::from)
}

/// Minimize the concatenated, noise-scaled residuals of all datasets.
///
/// Variables with the same name are shared; use distinct names for local values.
/// Missing noise settings mean εk = 1, not automatic noise estimation. Invalid
/// data, transforms, expressions, paths, or unavailable solvers return errors.
/// An `Ok` result can still report nonconvergence: inspect `solver_report`.
pub fn feffit_joint_with_options(
    datasets: &[FeffFitDataset],
    vars: &FitVariables,
    options: &FeffFitOptions,
) -> Result<FeffFitResult> {
    solver::feffit_joint_with_options(datasets, vars, options).map_err(XAFSError::from)
}

/// Fit each dataset separately from copied initial variables and solver options.
/// Returns one success or failure per input dataset in input order. Parallel
/// execution changes scheduling, not parameter sharing or initialization.
pub fn feffit_independent(
    datasets: &[FeffFitDataset],
    vars: &FitVariables,
    options: &FeffBatchOptions,
) -> Vec<std::result::Result<FeffFitResult, FittingError>> {
    solver::feffit_independent(datasets, vars, options)
}

/// Resolve backend commands without running a calculation.
/// Unsupported features or missing executables return an error.
pub fn resolve_feff_commands(request: &FeffRunRequest) -> Result<FeffResolvedCommands> {
    runner::resolve_feff_commands(request).map_err(XAFSError::from)
}

/// Run the requested scattering backend in its workspace and collect output paths.
/// This writes input/output/log files and can launch external processes. Inspect
/// [`FeffRunRequest`] for backend, timeout, and artifact-retention settings.
pub fn run_feff(request: &FeffRunRequest) -> Result<FeffRunResult> {
    runner::run_feff(request).map_err(XAFSError::from)
}

/// Run a scattering calculation, then parse its path files into owned models.
/// Use [`FeffFlavor::Feff85L`] for the compatible files emitted by current runners.
/// Calculation, file I/O, or path-parse failures return an error.
pub fn run_feff_and_load_paths(
    request: &FeffRunRequest,
    flavor: FeffFlavor,
) -> Result<Vec<FeffPathModel>> {
    runner::run_feff_and_load_paths(request, flavor).map_err(XAFSError::from)
}
