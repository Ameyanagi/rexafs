use std::collections::BTreeMap;
use std::num::NonZeroUsize;
use std::path::PathBuf;

use nalgebra::DVector;
use serde::{Deserialize, Serialize};

use crate::xafs::xafsutils::FTWindow;

/// Parser format selector, separate from the calculation backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FeffFlavor {
    /// Supported FEFF85L-style columns, also emitted by current FEFF10/ReFEFF runners.
    Feff85L,
    /// Reserved format; selecting it currently returns `UnsupportedFeffFlavor`.
    Feff10,
}

/// Space in which the fit residual is evaluated (Larch `fitspace`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum FitSpace {
    /// Real and imaginary parts of the windowed chi(R) between `rmin..rmax` (default).
    #[default]
    R,
    /// `k^w * chi(k)` between `kmin..kmax` (no k-window, Larch semantics).
    K,
    /// Real part of the back-transformed chi(q) between `kmin..kmax`.
    Q,
}

/// Scattering calculation backend; optional backends require their Cargo features.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum FeffExecutionMode {
    /// External FEFF85L module executables; the default mode.
    #[default]
    Feff85LModules,
    /// FEFFRS backend (`feff10-runner` feature), using bundled FEFF binaries.
    Feff10Pipeline,
    /// Pure-Rust ReFEFF backend (`refeff-runner` feature).
    RefeffPipeline,
}

/// Scheduling for independent fits; results retain input order for every strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum FeffBatchExecutionStrategy {
    /// Run one dataset at a time on the calling thread.
    Sequential,
    /// Use Rayon's shared global thread pool; the default.
    #[default]
    GlobalPool,
    /// Use a separate Rayon pool with a positive number of worker threads.
    DedicatedPool {
        /// Positive number of workers created for this independent batch.
        threads: NonZeroUsize,
    },
}

/// Nonlinear least-squares solver; convergence does not validate the physical model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FeffFitSolverMethod {
    /// Dog-leg trust-region solver; default with the `trust-region` feature.
    TrustRegionDogLeg,
    /// Levenberg–Marquardt solver; default when `trust-region` is unavailable.
    LevenbergMarquardt,
}

impl Default for FeffFitSolverMethod {
    fn default() -> Self {
        default_feff_fit_solver_method()
    }
}

#[cfg(feature = "trust-region")]
fn default_feff_fit_solver_method() -> FeffFitSolverMethod {
    FeffFitSolverMethod::TrustRegionDogLeg
}

#[cfg(not(feature = "trust-region"))]
fn default_feff_fit_solver_method() -> FeffFitSolverMethod {
    FeffFitSolverMethod::LevenbergMarquardt
}

/// Jacobian storage/assembly choice for the trust-region solver.
/// Levenberg–Marquardt uses its dense finite-difference path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum FeffFitJacobianMode {
    /// Select sparse assembly for multiple datasets, dense for a single dataset;
    /// the recommended default. The choice is based on dataset count.
    #[default]
    Auto,
    /// Assemble a dense residual-by-parameter matrix.
    Dense,
    /// Use dataset/variable dependency sparsity where supported.
    Sparse,
}

/// Solver settings; use [`Self::default`] unless an explicit comparison is needed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FeffFitOptions {
    /// Solver selected by enabled features; see [`FeffFitSolverMethod`].
    pub solver_method: FeffFitSolverMethod,
    /// Default [`FeffFitJacobianMode::Auto`]; does not change the fit objective.
    pub jacobian_mode: FeffFitJacobianMode,
}

impl Default for FeffFitOptions {
    fn default() -> Self {
        Self {
            solver_method: FeffFitSolverMethod::default(),
            jacobian_mode: FeffFitJacobianMode::Auto,
        }
    }
}

impl FeffFitOptions {
    /// Create a value with the defaults documented on this type.
    pub fn new() -> Self {
        Self::default()
    }

    /// Select Levenberg–Marquardt with the remaining default options.
    pub fn levenberg_marquardt() -> Self {
        Self {
            solver_method: FeffFitSolverMethod::LevenbergMarquardt,
            ..Self::default()
        }
    }

    /// Select the dog-leg trust-region solver; fitting requires the trust-region feature.
    pub fn trust_region() -> Self {
        Self {
            solver_method: FeffFitSolverMethod::TrustRegionDogLeg,
            ..Self::default()
        }
    }

    /// Replace the solver selection and return the updated settings.
    pub fn with_solver_method(mut self, solver_method: FeffFitSolverMethod) -> Self {
        self.solver_method = solver_method;
        self
    }

    /// Replace the Jacobian mode and return the updated settings.
    pub fn with_jacobian_mode(mut self, jacobian_mode: FeffFitJacobianMode) -> Self {
        self.jacobian_mode = jacobian_mode;
        self
    }
}

/// Independent-batch execution settings; each fit starts from the supplied variables.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FeffBatchOptions {
    /// Thread-pool strategy; defaults to the shared global pool.
    pub strategy: FeffBatchExecutionStrategy,
    /// Positive scheduling chunk size; default 256 datasets.
    pub chunk_size: NonZeroUsize,
    /// Numerical solver settings copied to each independent fit.
    pub solver_options: FeffFitOptions,
}

impl Default for FeffBatchOptions {
    fn default() -> Self {
        Self {
            strategy: FeffBatchExecutionStrategy::GlobalPool,
            chunk_size: NonZeroUsize::new(256).expect("nonzero constant"),
            solver_options: FeffFitOptions::default(),
        }
    }
}

impl FeffBatchOptions {
    /// Create a value with the defaults documented on this type.
    pub fn new() -> Self {
        Self::default()
    }

    /// Use the default shared global thread pool for independent fits.
    pub fn parallel() -> Self {
        Self::default()
    }

    /// Run independent fits one at a time with the remaining default options.
    pub fn sequential() -> Self {
        Self {
            strategy: FeffBatchExecutionStrategy::Sequential,
            ..Self::default()
        }
    }

    /// Use a dedicated pool with the requested positive worker count.
    pub fn dedicated(threads: NonZeroUsize) -> Self {
        Self {
            strategy: FeffBatchExecutionStrategy::DedicatedPool { threads },
            ..Self::default()
        }
    }

    /// Replace the batch scheduling strategy; parameter sharing is unchanged.
    pub fn with_strategy(mut self, strategy: FeffBatchExecutionStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    /// Set the positive number of datasets per scheduling chunk.
    pub fn with_chunk_size(mut self, chunk_size: NonZeroUsize) -> Self {
        self.chunk_size = chunk_size;
        self
    }

    /// Replace the numerical options used by every independent fit.
    pub fn with_solver_options(mut self, solver_options: FeffFitOptions) -> Self {
        self.solver_options = solver_options;
        self
    }
}

/// Filesystem and backend settings for a scattering calculation.
/// Defaults are a template: choose a workspace and backend/input before running.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FeffRunRequest {
    /// External executable or module location; backend-specific resolution applies.
    pub executable_path: PathBuf,
    /// Working directory for generated input, output, and log files; initially empty.
    pub workspace_dir: PathBuf,
    /// Optional source input file; `None` uses `feff.inp` in the workspace.
    pub feffinp: Option<PathBuf>,
    /// Calculation backend; defaults to external FEFF85L modules.
    pub mode: FeffExecutionMode,
    /// Optional timeout in seconds: per external module/FEFF10 stage, or a
    /// cooperative deadline for the full ReFEFF run. None adds no requested limit.
    pub timeout_sec: Option<u64>,
    /// Add the SFCONV card for FEFF10/ReFEFF; false adds no card automatically.
    /// An SFCONV card already in the input is preserved. Unused by FEFF85L mode.
    pub use_sfconv: bool,
    /// Preserve additional ReFEFF calculation artifacts; false by default.
    /// ReFEFF otherwise materializes path files and its run log. External
    /// FEFF85L/FEFF10 outputs are retained independently of this flag.
    pub keep_all_outputs: bool,
}

impl Default for FeffRunRequest {
    fn default() -> Self {
        Self {
            executable_path: PathBuf::new(),
            workspace_dir: PathBuf::new(),
            feffinp: None,
            mode: FeffExecutionMode::Feff85LModules,
            timeout_sec: None,
            use_sfconv: false,
            keep_all_outputs: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
/// One external executable or internal backend stage in a resolved calculation.
pub struct FeffModuleCommand {
    /// Backend module or stage name, in execution order.
    pub module: String,
    /// Resolved external executable path or internal backend stage identifier.
    pub executable: PathBuf,
}

impl Default for FeffModuleCommand {
    fn default() -> Self {
        Self {
            module: String::new(),
            executable: PathBuf::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
/// Selected backend and its ordered stages; resolving this value does not run them.
pub struct FeffResolvedCommands {
    /// Scattering backend selected for this calculation.
    pub mode: FeffExecutionMode,
    /// Ordered backend stages resolved for the requested calculation.
    pub modules: Vec<FeffModuleCommand>,
}

impl Default for FeffResolvedCommands {
    fn default() -> Self {
        Self {
            mode: FeffExecutionMode::Feff85LModules,
            modules: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
/// Owned paths and execution metadata from a completed scattering calculation.
/// This records filesystem locations; it does not embed the generated files.
pub struct FeffRunResult {
    /// Scattering backend selected for this calculation.
    pub mode: FeffExecutionMode,
    /// Calculation workspace containing generated artifacts.
    pub workspace_dir: PathBuf,
    /// Resolved source input path used for the calculation.
    pub feffinp_path: PathBuf,
    /// Backend stages associated with the completed calculation.
    pub resolved: FeffResolvedCommands,
    /// Paths to captured calculation logs when emitted by the backend.
    pub logs: Vec<PathBuf>,
    /// Generated FEFF path files for subsequent parsing and modeling.
    pub path_files: Vec<PathBuf>,
}

impl Default for FeffRunResult {
    fn default() -> Self {
        Self {
            mode: FeffExecutionMode::Feff85LModules,
            workspace_dir: PathBuf::new(),
            feffinp_path: PathBuf::new(),
            resolved: FeffResolvedCommands::default(),
            logs: Vec::new(),
            path_files: Vec::new(),
        }
    }
}

/// A numerical path correction or an expression evaluated against fit variables.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PathParamSpec {
    /// Fixed number in the units of the path field; default 0.0.
    Value(f64),
    /// Expression with named variables and path-local `reff`/`degen` values.
    Expression(String),
}

impl Default for PathParamSpec {
    fn default() -> Self {
        Self::Value(0.0)
    }
}

impl From<f64> for PathParamSpec {
    fn from(value: f64) -> Self {
        Self::Value(value)
    }
}

impl From<&str> for PathParamSpec {
    fn from(value: &str) -> Self {
        Self::Expression(value.to_string())
    }
}

impl From<String> for PathParamSpec {
    fn from(value: String) -> Self {
        Self::Expression(value)
    }
}

impl PathParamSpec {
    /// Borrow the expression text, or return None for a fixed numerical value.
    pub fn as_expression(&self) -> Option<&str> {
        match self {
            Self::Expression(expr) => Some(expr.as_str()),
            Self::Value(_) => None,
        }
    }
}

/// Named fit value, bounds, expression constraint, and optional local standard error.
/// Units follow its use in path expressions; no dimensional analysis is performed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FitVariable {
    /// Current numerical value; default 0.0.
    pub value: f64,
    /// Whether this is independently adjusted; default false. Expressions are fixed.
    pub vary: bool,
    /// Optional inclusive lower bound; `None` means unbounded.
    pub min: Option<f64>,
    /// Optional inclusive upper bound; `None` means unbounded.
    pub max: Option<f64>,
    /// Optional expression in other variables; cycles and unknown names are errors.
    pub expr: Option<String>,
    /// Local one-standard-error estimate in this variable's units, when available.
    pub stderr: Option<f64>,
    /// Initial value retained for reporting; set by [`Self::new`].
    pub init_value: f64,
}

impl Default for FitVariable {
    fn default() -> Self {
        Self {
            value: 0.0,
            vary: false,
            min: None,
            max: None,
            expr: None,
            stderr: None,
            init_value: 0.0,
        }
    }
}

impl FitVariable {
    /// Create a variable with the given current/initial value and variation flag; no bounds.
    pub fn new(value: f64, vary: bool) -> Self {
        Self {
            value,
            vary,
            init_value: value,
            ..Self::default()
        }
    }

    /// Set optional bounds in the variable’s units; None leaves that side unbounded.
    pub fn with_bounds(mut self, min: Option<f64>, max: Option<f64>) -> Self {
        self.min = min;
        self.max = max;
        self
    }

    /// Set an expression constraint and disable independent variation of this variable.
    pub fn with_expr<S: Into<String>>(mut self, expr: S) -> Self {
        self.expr = Some(expr.into());
        self.vary = false;
        self
    }

    /// Clamp a value to the stored lower and upper bounds without modifying this variable.
    pub fn clamp(&self, value: f64) -> f64 {
        let mut out = value;
        if let Some(min) = self.min {
            out = out.max(min);
        }
        if let Some(max) = self.max {
            out = out.min(max);
        }
        out
    }
}

/// Lightweight parameter specification for the builder API.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Param {
    /// Identifier used to reference this parameter in expressions.
    pub name: String,
    /// Initial numerical value in the units implied by its path expression.
    pub value: f64,
    /// Whether to fit this value independently; expression constraints override it.
    pub vary: bool,
    /// Optional lower bound in parameter units; None leaves it unbounded.
    pub min: Option<f64>,
    /// Optional upper bound in parameter units; None leaves it unbounded.
    pub max: Option<f64>,
    /// Optional expression defining a constrained value from other named parameters.
    pub expr: Option<String>,
}

impl Default for Param {
    fn default() -> Self {
        Self {
            name: String::new(),
            value: 0.0,
            vary: true,
            min: None,
            max: None,
            expr: None,
        }
    }
}

impl Param {
    /// Varying parameter with initial value.
    pub fn new(name: impl Into<String>, value: f64) -> Self {
        Self {
            name: name.into(),
            value,
            vary: true,
            min: None,
            max: None,
            expr: None,
        }
    }

    /// Fixed parameter (vary=false).
    pub fn fixed(name: impl Into<String>, value: f64) -> Self {
        Self {
            name: name.into(),
            value,
            vary: false,
            min: None,
            max: None,
            expr: None,
        }
    }

    /// Expression-derived parameter (vary=false).
    pub fn expr(name: impl Into<String>, expr: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: 0.0,
            vary: false,
            min: None,
            max: None,
            expr: Some(expr.into()),
        }
    }

    /// Set bounds (consuming Self).
    pub fn bounds(mut self, min: f64, max: f64) -> Self {
        self.min = Some(min);
        self.max = Some(max);
        self
    }

    /// Convert to FitVariable.
    pub fn to_fit_variable(&self) -> FitVariable {
        let mut var = FitVariable::new(self.value, self.vary).with_bounds(self.min, self.max);
        if let Some(expr) = self.expr.as_ref() {
            var = var.with_expr(expr.clone());
        }
        var
    }
}

/// Named global variables and expression constraints used by every dataset.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct FitVariables {
    /// Name-to-variable map; equal names share a value across paths and datasets.
    pub vars: BTreeMap<String, FitVariable>,
}

impl FitVariables {
    /// Create a value with the defaults documented on this type.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert or replace a named variable, returning this collection for chaining.
    pub fn insert<S: Into<String>>(&mut self, name: S, variable: FitVariable) -> &mut Self {
        self.vars.insert(name.into(), variable);
        self
    }

    /// Borrow a named variable, or return None when it is absent.
    pub fn get(&self, name: &str) -> Option<&FitVariable> {
        self.vars.get(name)
    }

    /// Borrow a named variable mutably, or return None when it is absent.
    pub fn get_mut(&mut self, name: &str) -> Option<&mut FitVariable> {
        self.vars.get_mut(name)
    }
}

/// Parsed scattering-path columns and geometry owned by a path model.
/// The empty default is a serialization/template value, not a usable calculation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FeffDat {
    /// Source path-file name, retained for provenance.
    pub filename: String,
    /// Calculation title read from the file header.
    pub title: String,
    /// Backend version text read from the file header.
    pub version: String,
    /// Absorber metadata when present in the header.
    pub absorber: Option<String>,
    /// Absorption-edge shell metadata when present.
    pub shell: Option<String>,
    /// Reference half-path length in Å; a single-scattering path's bond distance.
    pub reff: f64,
    /// Dimensionless path multiplicity from the file.
    pub degen: f64,
    /// Number of legs in the closed scattering path; two for single scattering.
    pub nleg: usize,
    /// Tabulated photoelectron wave number in Å⁻¹.
    pub k: DVector<f64>,
    /// Central-atom phase contribution in radians, as stored by FEFF.
    pub real_phc: DVector<f64>,
    /// Magnitude of the effective scattering amplitude, in Å.
    pub mag_feff: DVector<f64>,
    /// Effective scattering phase in radians.
    pub pha_feff: DVector<f64>,
    /// Dimensionless amplitude reduction factor from the path calculation.
    pub red_fact: DVector<f64>,
    /// Photoelectron mean free path in Å.
    pub lam: DVector<f64>,
    /// Real part of the effective photoelectron momentum in Å⁻¹.
    pub rep: DVector<f64>,
    /// Total phase `real_phc + pha_feff`, in radians.
    pub pha: DVector<f64>,
    /// Reduced amplitude `mag_feff * red_fact`, in Å.
    pub amp: DVector<f64>,
    /// Original text geometry rows, retained alongside parsed atoms.
    pub geometry: Vec<String>,
    /// Leg coordinates from the path file (absorber first), when present.
    pub geometry_atoms: Vec<PathAtom>,
}

/// One atom of a scattering path as listed in a FEFF path file
/// (`x y z ipot z label`), absorber at the origin.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct PathAtom {
    /// Cartesian x coordinate in Å, relative to the absorber.
    pub x: f64,
    /// Cartesian y coordinate in Å, relative to the absorber.
    pub y: f64,
    /// Cartesian z coordinate in Å, relative to the absorber.
    pub z: f64,
    /// FEFF potential index; zero identifies the absorber potential.
    pub ipot: u16,
    /// Atomic number of this path atom.
    pub atomic_number: u8,
    /// Human-readable path or atom identifier.
    pub label: String,
}

impl Default for FeffDat {
    fn default() -> Self {
        Self {
            filename: String::new(),
            title: String::new(),
            version: String::new(),
            absorber: None,
            shell: None,
            reff: 0.0,
            degen: 1.0,
            nleg: 0,
            k: DVector::zeros(0),
            real_phc: DVector::zeros(0),
            mag_feff: DVector::zeros(0),
            pha_feff: DVector::zeros(0),
            red_fact: DVector::zeros(0),
            lam: DVector::zeros(0),
            rep: DVector::zeros(0),
            pha: DVector::zeros(0),
            amp: DVector::zeros(0),
            geometry: Vec::new(),
            geometry_atoms: Vec::new(),
        }
    }
}

/// A theoretical path with adjustable physical corrections.
///
/// Load with [`Self::from_feffdat`] or [`super::feffpath`] to retain the file's
/// degeneracy. Each correction accepts a fixed value or an expression; the
/// underlying FEFF amplitudes and phases remain fixed when parameters change.
/// S₀² = 1 and zero corrections are reference-model defaults, not fitted
/// estimates or recommended physical bounds for every material.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FeffPathModel {
    /// Human-readable identifier used in contribution plots.
    pub label: String,
    /// Owned reference scattering calculation.
    pub feff: FeffDat,
    /// Include this path in multi-path sums/fits; default true.
    pub use_path: bool,
    /// Dimensionless degeneracy N; loaded from FEFF by [`Self::from_feffdat`].
    pub degen: PathParamSpec,
    /// Dimensionless amplitude reduction S₀²; default 1.0, correlated with N.
    pub s02: PathParamSpec,
    /// Relative threshold shift ΔE₀ in eV; default 0.0. Positive values reduce q².
    pub e0: PathParamSpec,
    /// Imaginary energy correction in eV; default 0.0, affecting complex momentum.
    pub ei: PathParamSpec,
    /// Half-path-length change ΔR in Å; default 0.0. Physical length is reff + ΔR.
    pub deltar: PathParamSpec,
    /// Second cumulant σ² in Å²; default 0.0. Positive values damp high-k signal.
    pub sigma2: PathParamSpec,
    /// Third cumulant in Å³; default 0.0, describing asymmetric distance disorder.
    pub third: PathParamSpec,
    /// Fourth cumulant in Å⁴; default 0.0. It is a cumulant, not a fourth moment.
    pub fourth: PathParamSpec,
}

impl Default for FeffPathModel {
    fn default() -> Self {
        Self {
            label: String::new(),
            feff: FeffDat::default(),
            use_path: true,
            degen: PathParamSpec::Value(1.0),
            s02: PathParamSpec::Value(1.0),
            e0: PathParamSpec::Value(0.0),
            ei: PathParamSpec::Value(0.0),
            deltar: PathParamSpec::Value(0.0),
            sigma2: PathParamSpec::Value(0.0),
            third: PathParamSpec::Value(0.0),
            fourth: PathParamSpec::Value(0.0),
        }
    }
}

impl FeffPathModel {
    /// Own the parsed path data and retain its file degeneracy; other corrections use defaults.
    pub fn from_feffdat<S: Into<String>>(label: S, feff: FeffDat) -> Self {
        let degen = feff.degen;
        Self {
            label: label.into(),
            degen: PathParamSpec::Value(degen),
            feff,
            ..Self::default()
        }
    }

    /// Set dimensionless S₀² to a fixed value or expression; initial default 1.0.
    pub fn set_s02(mut self, spec: impl Into<PathParamSpec>) -> Self {
        self.s02 = spec.into();
        self
    }

    /// Set the relative threshold shift in eV to a value or expression; initial default 0.0.
    pub fn set_e0(mut self, spec: impl Into<PathParamSpec>) -> Self {
        self.e0 = spec.into();
        self
    }

    /// Set the imaginary energy correction in eV; initial default 0.0.
    pub fn set_ei(mut self, spec: impl Into<PathParamSpec>) -> Self {
        self.ei = spec.into();
        self
    }

    /// Set the half-path-length correction in Å; initial default 0.0.
    pub fn set_deltar(mut self, spec: impl Into<PathParamSpec>) -> Self {
        self.deltar = spec.into();
        self
    }

    /// Set the second distance cumulant in Å²; initial default 0.0.
    pub fn set_sigma2(mut self, spec: impl Into<PathParamSpec>) -> Self {
        self.sigma2 = spec.into();
        self
    }

    /// Set the third distance cumulant in Å³; initial default 0.0.
    pub fn set_third(mut self, spec: impl Into<PathParamSpec>) -> Self {
        self.third = spec.into();
        self
    }

    /// Set the fourth distance cumulant in Å⁴; initial default 0.0.
    pub fn set_fourth(mut self, spec: impl Into<PathParamSpec>) -> Self {
        self.fourth = spec.into();
        self
    }

    /// Set dimensionless path degeneracy to a fixed value or expression.
    pub fn set_degen(mut self, spec: impl Into<PathParamSpec>) -> Self {
        self.degen = spec.into();
        self
    }

    /// Replace the display label and return the owned model.
    pub fn set_label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    /// Include or exclude this path from multi-path sums; enabled by default.
    pub fn set_use_path(mut self, use_path: bool) -> Self {
        self.use_path = use_path;
        self
    }
}

/// Transform and residual-selection settings for EXAFS path fitting.
///
/// Defaults are k = 0–20 Å⁻¹, R = 1–3 Å, weight 2, and R-space fitting.
/// Set ranges that are supported by the measured data and physical model.
/// Data are expected on a uniform k grid; these settings do not resample it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FeffFitTransform {
    /// Lower k bound in Å⁻¹; default 0.0.
    pub kmin: f64,
    /// Upper k bound in Å⁻¹; default 20.0 and must exceed kmin.
    pub kmax: f64,
    /// Nonnegative real exponent applied as k^w; default 2.0.
    /// Unlike the processing FFT, fitting does not floor fractional weights.
    pub kweight: f64,
    /// Low-k window parameter; default 4.0. Geometry uses Å⁻¹, while
    /// Kaiser–Bessel also uses this number as its dimensionless shape parameter.
    pub dk: f64,
    /// High-k window parameter; `None` uses dk.
    pub dk2: Option<f64>,
    /// k-window family; default Kaiser–Bessel. K-space residuals omit this window.
    pub window: FTWindow,
    /// FFT length; default 2048, minimum 16. Use at least the input sample count.
    pub nfft: usize,
    /// k spacing in Å⁻¹; default Some(0.05). `None` infers transform spacing
    /// from the first two k samples, but noise conversion still assumes 0.05.
    /// Set it explicitly when using a different grid to keep scaling consistent.
    pub kstep: Option<f64>,
    /// Lower R bound in Å; default 1.0.
    pub rmin: f64,
    /// Upper R bound in Å; default 3.0 and must exceed rmin.
    pub rmax: f64,
    /// Low-R window parameter in Å for Hanning; default 0.0 (hard boundary).
    pub dr: f64,
    /// High-R window parameter; `None` uses dr.
    pub dr2: Option<f64>,
    /// R-window family; default Hanning. Used by R- and Q-space fits.
    pub rwindow: FTWindow,
    /// Residual representation; real plus imaginary R-space values by default.
    pub fitspace: FitSpace,
    /// Simultaneous k-weights (Larch `kweight=(1,2,3)`). Empty means "use `kweight`".
    /// The residual concatenates one transformed block per k-weight.
    pub kweights: Vec<f64>,
}

impl Default for FeffFitTransform {
    fn default() -> Self {
        Self {
            kmin: 0.0,
            kmax: 20.0,
            kweight: 2.0,
            dk: 4.0,
            dk2: None,
            window: FTWindow::KaiserBessel,
            nfft: 2048,
            kstep: Some(0.05),
            rmin: 1.0,
            rmax: 3.0,
            dr: 0.0,
            dr2: None,
            rwindow: FTWindow::Hanning,
            fitspace: FitSpace::R,
            kweights: Vec::new(),
        }
    }
}

impl FeffFitTransform {
    /// k-weights actually used by the fit: `kweights` when non-empty, otherwise `[kweight]`.
    pub fn effective_kweights(&self) -> Vec<f64> {
        if self.kweights.is_empty() {
            vec![self.kweight]
        } else {
            self.kweights.clone()
        }
    }

    /// First effective k-weight (Larch `get_kweight()`), used for the primary plot arrays.
    pub fn primary_kweight(&self) -> f64 {
        self.kweights.first().copied().unwrap_or(self.kweight)
    }
}

/// One already-processed dataset, its transform, path models, and noise scales.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FeffFitDataset {
    /// Owned uniform k grid in Å⁻¹, conventionally starting at zero.
    pub k: DVector<f64>,
    /// Owned dimensionless background-subtracted χ(k), matching k in length.
    pub chi: DVector<f64>,
    /// Residual scale in k; `None` means 1.0, not automatic estimation.
    /// Values used in residuals are floored at 1e-12; see `estimate_noise`.
    pub epsilon_k: Option<f64>,
    /// Transform, fit-space, and range settings.
    pub transform: FeffFitTransform,
    /// Theoretical scattering paths; only paths with use_path=true contribute.
    pub paths: Vec<FeffPathModel>,
    /// Per-k-weight noise estimates, aligned with `transform.effective_kweights()`.
    /// Empty means "use `epsilon_k` for every k-weight block".
    pub epsilon_ks: Vec<f64>,
}

impl Default for FeffFitDataset {
    fn default() -> Self {
        Self {
            k: DVector::zeros(0),
            chi: DVector::zeros(0),
            epsilon_k: None,
            transform: FeffFitTransform::default(),
            paths: Vec::new(),
            epsilon_ks: Vec::new(),
        }
    }
}

impl FeffFitDataset {
    /// Create a value with the defaults documented on this type.
    pub fn new() -> Self {
        Self::default()
    }

    /// Copy the prepared k grid (Å⁻¹) and dimensionless χ(k) arrays into the dataset.
    pub fn data(mut self, k: &DVector<f64>, chi: &DVector<f64>) -> Self {
        self.k = k.clone();
        self.chi = chi.clone();
        self
    }

    /// Set the scalar k-space residual divisor; absent settings otherwise use 1.0.
    pub fn epsilon_k(mut self, value: f64) -> Self {
        self.epsilon_k = Some(value);
        self
    }

    /// Append an owned theoretical path to the dataset.
    pub fn add_path(mut self, path: FeffPathModel) -> Self {
        self.paths.push(path);
        self
    }

    /// Set the fit interval in k (Å⁻¹); ensure it is supported by measured data.
    pub fn krange(mut self, kmin: f64, kmax: f64) -> Self {
        self.transform.kmin = kmin;
        self.transform.kmax = kmax;
        self
    }

    /// Set the fit interval in R (Å); these Fourier positions are not phase-corrected distances.
    pub fn rrange(mut self, rmin: f64, rmax: f64) -> Self {
        self.transform.rmin = rmin;
        self.transform.rmax = rmax;
        self
    }

    /// Set the nonnegative real exponent w in k^w; used when kweights is empty.
    pub fn kweight(mut self, value: f64) -> Self {
        self.transform.kweight = value;
        self
    }

    /// Set the low-k window parameter; Kaiser–Bessel also uses it to control shape.
    pub fn dk(mut self, value: f64) -> Self {
        self.transform.dk = value;
        self
    }

    /// Set the k-window family; K-space residuals do not apply this window.
    pub fn window(mut self, value: FTWindow) -> Self {
        self.transform.window = value;
        self
    }

    /// Set the R-window family for R- and Q-space fits.
    pub fn rwindow(mut self, value: FTWindow) -> Self {
        self.transform.rwindow = value;
        self
    }

    /// Set the low-R window parameter (Å for Hanning); default zero gives a hard edge.
    pub fn dr(mut self, value: f64) -> Self {
        self.transform.dr = value;
        self
    }

    /// Fit several k-weights simultaneously (Larch `kweight=(1,2,3)`).
    pub fn kweights(mut self, values: &[f64]) -> Self {
        self.transform.kweights = values.to_vec();
        self
    }

    /// Select the fit space (R, K or Q).
    pub fn fitspace(mut self, value: FitSpace) -> Self {
        self.transform.fitspace = value;
        self
    }

    /// Per-k-weight noise estimates (aligned with `kweights`).
    pub fn epsilon_ks(mut self, values: &[f64]) -> Self {
        self.epsilon_ks = values.to_vec();
        self
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
/// One enabled path's owned χ(k), χ(R), and filtered χ(q) plot arrays.
/// Transform arrays use the dataset's primary k-weight and the fitter's conventions.
pub struct PathContribution {
    /// Human-readable path or atom identifier.
    pub label: String,
    /// Dimensionless χ(k) of this path on the reported k grid.
    pub chi: DVector<f64>,
    /// Real part of this path’s k-weighted Fourier transform, in Å^(-(w+1)).
    pub chir_re: DVector<f64>,
    /// Imaginary part of this path’s k-weighted Fourier transform, in Å^(-(w+1)).
    pub chir_im: DVector<f64>,
    /// Magnitude of this path’s k-weighted Fourier transform, in Å^(-(w+1)).
    pub chir_mag: DVector<f64>,
    /// Real part of chi(q) (Larch `chiq_re`) on the same grid as `DatasetResult::q`.
    pub chiq: DVector<f64>,
}

impl Default for PathContribution {
    fn default() -> Self {
        Self {
            label: String::new(),
            chi: DVector::zeros(0),
            chir_re: DVector::zeros(0),
            chir_im: DVector::zeros(0),
            chir_mag: DVector::zeros(0),
            chiq: DVector::zeros(0),
        }
    }
}

/// Transformed data/model arrays for one k-weight of a (possibly multi-k-weight) fit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct KweightResult {
    /// Exponent w used by the primary transformed arrays.
    pub kweight: f64,
    /// Noise estimate in k used for this block (1.0 when the dataset carries none).
    pub epsilon_k: f64,
    /// Noise estimate in R derived from `epsilon_k` (Larch `epsilon_r`).
    pub epsilon_r: f64,
    /// Number of residual points contributed by this block.
    pub n_data: usize,
    /// Dimensionless k-window on the reported k grid.
    pub kwin: DVector<f64>,
    /// `k^w * chi(k)` (no window) for data and model.
    pub data_chik: DVector<f64>,
    /// Modeled k^w χ(k), without a window, in Å^(-w).
    pub model_chik: DVector<f64>,
    /// Real part of the measured k-windowed transform, in Å^(-(w+1)).
    pub data_chir_re: DVector<f64>,
    /// Imaginary part of the measured k-windowed transform, in Å^(-(w+1)).
    pub data_chir_im: DVector<f64>,
    /// Magnitude of the measured k-windowed transform, in Å^(-(w+1)).
    pub data_chir_mag: DVector<f64>,
    /// Real part of the modeled k-windowed transform, in Å^(-(w+1)).
    pub model_chir_re: DVector<f64>,
    /// Imaginary part of the modeled k-windowed transform, in Å^(-(w+1)).
    pub model_chir_im: DVector<f64>,
    /// Magnitude of the modeled k-windowed transform, in Å^(-(w+1)).
    pub model_chir_mag: DVector<f64>,
    /// q grid (`0..kmax+2`, Larch convention) for the back-transformed arrays.
    pub q: DVector<f64>,
    /// Real part of chi(q) (Larch `chiq_re`) after the R-window.
    pub data_chiq: DVector<f64>,
    /// Real part of the R-filtered model using the fitting q-transform convention.
    pub model_chiq: DVector<f64>,
}

impl Default for KweightResult {
    fn default() -> Self {
        Self {
            kweight: 2.0,
            epsilon_k: 1.0,
            epsilon_r: 1.0,
            n_data: 0,
            kwin: DVector::zeros(0),
            data_chik: DVector::zeros(0),
            model_chik: DVector::zeros(0),
            data_chir_re: DVector::zeros(0),
            data_chir_im: DVector::zeros(0),
            data_chir_mag: DVector::zeros(0),
            model_chir_re: DVector::zeros(0),
            model_chir_im: DVector::zeros(0),
            model_chir_mag: DVector::zeros(0),
            q: DVector::zeros(0),
            data_chiq: DVector::zeros(0),
            model_chiq: DVector::zeros(0),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
/// One dataset's fit statistics, unweighted χ(k), and weighted transform arrays.
/// Real and imaginary plot arrays precede R-windowing; residuals apply that window.
/// See [`FeffFitResult`] for reporting formulas and their statistical limitations.
pub struct DatasetResult {
    /// Number of scalar residual entries, including every k-weight and R component.
    pub n_data: usize,
    /// Reported squared residual sum S·n_idp/n_data; see the fitting-statistics guide.
    pub chi_square: f64,
    /// Reported chi-square divided by max(n_idp - relevant varying count, 1e-12).
    pub reduced_chi_square: f64,
    /// Relative squared discrepancy in the scaled fit space; zero for a zero data norm.
    pub r_factor: f64,
    /// Bandwidth estimate 1 + 2·Δk·ΔR/π per dataset, summed for a joint fit.
    pub n_idp: f64,
    /// Photoelectron k grid in Å⁻¹, copied from the dataset.
    pub k: DVector<f64>,
    /// Dimensionless measured χ(k), before fit weights/windows.
    pub data_chi: DVector<f64>,
    /// Dimensionless summed path model χ(k), before fit weights/windows.
    pub model_chi: DVector<f64>,
    /// Exponent w used by the primary transformed arrays.
    pub kweight: f64,
    /// Selected lower k bound in Å⁻¹, when recorded.
    pub kmin: Option<f64>,
    /// Selected upper k bound in Å⁻¹, when recorded.
    pub kmax: Option<f64>,
    /// Dimensionless k-window on the reported k grid.
    pub kwin: DVector<f64>,
    /// Fourier R grid in Å; these uncorrected peak positions are not bond lengths.
    pub r: DVector<f64>,
    /// Selected lower R bound in Å, when recorded.
    pub rmin: Option<f64>,
    /// Selected upper R bound in Å, when recorded.
    pub rmax: Option<f64>,
    /// Real part of the measured k-windowed transform, in Å^(-(w+1)).
    pub data_chir_re: DVector<f64>,
    /// Imaginary part of the measured k-windowed transform, in Å^(-(w+1)).
    pub data_chir_im: DVector<f64>,
    /// Real part of the modeled k-windowed transform, in Å^(-(w+1)).
    pub model_chir_re: DVector<f64>,
    /// Imaginary part of the modeled k-windowed transform, in Å^(-(w+1)).
    pub model_chir_im: DVector<f64>,
    /// Magnitude of the modeled k-windowed transform, in Å^(-(w+1)).
    pub model_chir_mag: DVector<f64>,
    /// Individual enabled path contributions for the primary k-weight.
    pub path_contributions: Vec<PathContribution>,
    /// Space the residual was evaluated in.
    pub fitspace: FitSpace,
    /// All k-weights used by the fit (primary arrays above use the first one).
    pub kweights: Vec<f64>,
    /// q grid for `data_chiq`/`model_chiq` (Larch: `0..kmax+2`).
    pub q: DVector<f64>,
    /// Real part of the back-transformed chi(q) of the data (primary k-weight).
    pub data_chiq: DVector<f64>,
    /// Real part of the back-transformed chi(q) of the model (primary k-weight).
    pub model_chiq: DVector<f64>,
    /// Per-k-weight transformed arrays, aligned with `kweights`.
    pub kweight_results: Vec<KweightResult>,
}

impl Default for DatasetResult {
    fn default() -> Self {
        Self {
            n_data: 0,
            chi_square: 0.0,
            reduced_chi_square: 0.0,
            r_factor: 0.0,
            n_idp: 0.0,
            k: DVector::zeros(0),
            data_chi: DVector::zeros(0),
            model_chi: DVector::zeros(0),
            kweight: 2.0,
            kmin: None,
            kmax: None,
            kwin: DVector::zeros(0),
            r: DVector::zeros(0),
            rmin: None,
            rmax: None,
            data_chir_re: DVector::zeros(0),
            data_chir_im: DVector::zeros(0),
            model_chir_re: DVector::zeros(0),
            model_chir_im: DVector::zeros(0),
            model_chir_mag: DVector::zeros(0),
            path_contributions: Vec::new(),
            fitspace: FitSpace::R,
            kweights: Vec::new(),
            q: DVector::zeros(0),
            data_chiq: DVector::zeros(0),
            model_chiq: DVector::zeros(0),
            kweight_results: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
/// Explanation of a variable inferred by the convenience fit builder.
/// Check the starting value and role instead of interpreting inference as a physical prior.
pub struct FitWarning {
    /// Variable name whose starting value was inferred by the builder.
    pub symbol: String,
    /// Path parameter role that supplied the inferred starting value.
    pub inferred_from: String,
    /// Inferred starting value in the units of that path parameter role.
    pub default_value: f64,
    /// Explanation of the inference or conflicting defaults for user review.
    pub message: String,
}

impl Default for FitWarning {
    fn default() -> Self {
        Self {
            symbol: String::new(),
            inferred_from: String::new(),
            default_value: 0.0,
            message: String::new(),
        }
    }
}

/// Numerical termination information. Convergence does not establish that the
/// selected paths provide a scientifically adequate model of the data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FitSolverReport {
    /// Numerical solver actually used.
    pub method: FeffFitSolverMethod,
    /// Whether the optimizer met its stopping rules; not model validation.
    pub converged: bool,
    /// Solver termination description, including nonconvergence reasons.
    pub termination: String,
    /// Iteration count when exposed by the selected solver.
    pub iterations: Option<usize>,
    /// Residual-evaluation count when exposed by the selected solver.
    pub evaluations: Option<usize>,
    /// Half the sum of squared residuals in the selected, weighted fit space.
    pub initial_cost: f64,
    /// Half the final sum of squared, noise-scaled scalar residuals.
    pub final_cost: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
/// Owned fitted parameters, diagnostics, and primary/per-dataset output arrays.
///
/// # Statistics implemented by the solver
///
/// Let r be the concatenated real residual after the selected transforms,
/// windows, k-weights, and noise divisors. R-space contributes real and
/// imaginary entries separately. The optimizer minimizes S = sum(r_i²).
/// With M residual entries, p independently varying parameters, and
/// N_idp = sum_d[1 + 2 Δk_d ΔR_d / π], the reporting conventions are:
///
/// ```text
/// dof = max(N_idp - p, 1e-12)
/// chi_square = S · N_idp / max(M, 1)
/// reduced_chi_square = chi_square / dof
/// covariance ≈ [S / dof] · inverse(JᵀJ)
/// stderr_j = sqrt(covariance[j,j])
/// correlation[i,j] = covariance[i,j] / sqrt(covariance[i,i] covariance[j,j])
/// ```
///
/// Here d indexes datasets; Δk is the selected span in Å⁻¹, ΔR is in Å,
/// and N_idp is dimensionless. J has entries ∂r_i/∂θ_j for independently
/// varying physical parameters θ. Covariance units are products of parameter
/// units; standard errors have parameter units. This local linear approximation
/// assumes an adequate model and meaningful noise scales; it is not a confidence
/// interval or protection against nonidentifiability. Covariance is absent
/// when an inverse cannot be formed, and active bounds can make symmetric
/// standard errors misleading. The positive denominator guard is numerical,
/// not statistical justification for too many parameters.
///
/// The additive one in N_idp and the raw-S covariance scaling specify rexafs;
/// see [`super::transform::compute_n_idp`] and [`super::solver`]. Related
/// information-counting theory is discussed by
/// [Stern (1993)](https://doi.org/10.1103/PhysRevB.48.9825), while the
/// [SciPy leastsq reference](https://docs.scipy.org/doc/scipy/reference/generated/scipy.optimize.leastsq.html)
/// explains inverse-curvature covariance and residual-variance scaling.
///
/// R-factor is sum ||data-model||² / sum ||data||² in the same scaled
/// representation, combining dataset numerators and denominators before division.
/// It is dimensionless and returns zero for a numerically zero data denominator;
/// that special value does not certify a fit to an absent signal. Compare R-factors
/// only with compatible fit spaces, scales, and intervals.
///
/// Top-level plotting fields mirror the first dataset, while global statistics
/// combine all datasets. Inputs remain unchanged. Inspect `solver_report` even
/// when the fit returned Ok: optimizer convergence does not validate the physics.
pub struct FeffFitResult {
    /// Absent in results saved before optimizer diagnostics were recorded.
    pub solver_report: Option<FitSolverReport>,
    /// Fitted independent and expression-derived values, with available errors.
    pub variables: FitVariables,
    /// Ordering of varying parameters used for covariance/correlation matrix rows and columns.
    pub varying_names: Vec<String>,
    /// Number of independently varying parameters; constrained expressions do not count.
    pub n_vary: usize,
    /// Number of scalar residual entries, including every k-weight and R component.
    pub n_data: usize,
    /// Reported squared residual sum S·n_idp/n_data; see the fitting-statistics guide.
    pub chi_square: f64,
    /// Reported chi-square divided by max(n_idp - relevant varying count, 1e-12).
    pub reduced_chi_square: f64,
    /// Relative squared discrepancy in the scaled fit space; zero for a zero data norm.
    pub r_factor: f64,
    /// Covariance matrix scaled from raw LM residuals using `raw_chi_square / (n_idp - n_vary)`.
    /// This matches Larch-style stderr scaling.
    pub covariance: Option<Vec<Vec<f64>>>,
    /// Correlation matrix derived from `covariance` with the same parameter ordering as
    /// `varying_names`.
    pub correlation: Option<Vec<Vec<f64>>>,
    /// Photoelectron k grid in Å⁻¹, copied from the dataset.
    pub k: DVector<f64>,
    /// Dimensionless measured χ(k), before fit weights/windows.
    pub data_chi: DVector<f64>,
    /// Dimensionless summed path model χ(k), before fit weights/windows.
    pub model_chi: DVector<f64>,
    /// Exponent w used by the primary transformed arrays.
    pub kweight: f64,
    /// Selected lower k bound in Å⁻¹, when recorded.
    pub kmin: Option<f64>,
    /// Selected upper k bound in Å⁻¹, when recorded.
    pub kmax: Option<f64>,
    /// Dimensionless k-window on the reported k grid.
    pub kwin: DVector<f64>,
    /// Fourier R grid in Å; these uncorrected peak positions are not bond lengths.
    pub r: DVector<f64>,
    /// Selected lower R bound in Å, when recorded.
    pub rmin: Option<f64>,
    /// Selected upper R bound in Å, when recorded.
    pub rmax: Option<f64>,
    /// Real part of the measured k-windowed transform, in Å^(-(w+1)).
    pub data_chir_re: DVector<f64>,
    /// Imaginary part of the measured k-windowed transform, in Å^(-(w+1)).
    pub data_chir_im: DVector<f64>,
    /// Real part of the modeled k-windowed transform, in Å^(-(w+1)).
    pub model_chir_re: DVector<f64>,
    /// Imaginary part of the modeled k-windowed transform, in Å^(-(w+1)).
    pub model_chir_im: DVector<f64>,
    /// Magnitude of the modeled k-windowed transform, in Å^(-(w+1)).
    pub model_chir_mag: DVector<f64>,
    /// Individual enabled path contributions for the primary k-weight.
    pub path_contributions: Vec<PathContribution>,
    /// Per-dataset results in input order; top-level plot arrays mirror the first dataset.
    pub datasets: Vec<DatasetResult>,
    /// Bandwidth estimate 1 + 2·Δk·ΔR/π per dataset, summed for a joint fit.
    pub n_idp: f64,
    /// Builder inference warnings retained for review and provenance.
    pub warnings: Vec<FitWarning>,
    /// Fit space of the primary dataset.
    pub fitspace: FitSpace,
    /// k-weights of the primary dataset (primary arrays use the first one).
    pub kweights: Vec<f64>,
    /// q grid of the primary dataset for `data_chiq`/`model_chiq`.
    pub q: DVector<f64>,
    /// Real part of the R-filtered data using the fitting q-transform convention.
    pub data_chiq: DVector<f64>,
    /// Real part of the R-filtered model using the fitting q-transform convention.
    pub model_chiq: DVector<f64>,
    /// Per-k-weight transformed arrays of the primary dataset.
    pub kweight_results: Vec<KweightResult>,
}

impl Default for FeffFitResult {
    fn default() -> Self {
        Self {
            solver_report: None,
            variables: FitVariables::default(),
            varying_names: Vec::new(),
            n_vary: 0,
            n_data: 0,
            chi_square: 0.0,
            reduced_chi_square: 0.0,
            r_factor: 0.0,
            covariance: None,
            correlation: None,
            k: DVector::zeros(0),
            data_chi: DVector::zeros(0),
            model_chi: DVector::zeros(0),
            kweight: 2.0,
            kmin: None,
            kmax: None,
            kwin: DVector::zeros(0),
            r: DVector::zeros(0),
            rmin: None,
            rmax: None,
            data_chir_re: DVector::zeros(0),
            data_chir_im: DVector::zeros(0),
            model_chir_re: DVector::zeros(0),
            model_chir_im: DVector::zeros(0),
            model_chir_mag: DVector::zeros(0),
            path_contributions: Vec::new(),
            datasets: Vec::new(),
            n_idp: 0.0,
            warnings: Vec::new(),
            fitspace: FitSpace::R,
            kweights: Vec::new(),
            q: DVector::zeros(0),
            data_chiq: DVector::zeros(0),
            model_chiq: DVector::zeros(0),
            kweight_results: Vec::new(),
        }
    }
}

impl FeffFitResult {
    /// Borrow the indexed dataset result, or return None when the index is absent.
    pub fn dataset(&self, index: usize) -> Option<&DatasetResult> {
        self.datasets.get(index)
    }

    /// Copy first-dataset plotting/settings fields into the top-level convenience fields.
    pub fn sync_primary_dataset_fields(&mut self) {
        if let Some(dataset) = self.datasets.first() {
            self.k = dataset.k.clone();
            self.data_chi = dataset.data_chi.clone();
            self.model_chi = dataset.model_chi.clone();
            self.kweight = dataset.kweight;
            self.kmin = dataset.kmin;
            self.kmax = dataset.kmax;
            self.kwin = dataset.kwin.clone();
            self.r = dataset.r.clone();
            self.rmin = dataset.rmin;
            self.rmax = dataset.rmax;
            self.data_chir_re = dataset.data_chir_re.clone();
            self.data_chir_im = dataset.data_chir_im.clone();
            self.model_chir_re = dataset.model_chir_re.clone();
            self.model_chir_im = dataset.model_chir_im.clone();
            self.model_chir_mag = dataset.model_chir_mag.clone();
            self.path_contributions = dataset.path_contributions.clone();
            self.fitspace = dataset.fitspace;
            self.kweights = dataset.kweights.clone();
            self.q = dataset.q.clone();
            self.data_chiq = dataset.data_chiq.clone();
            self.model_chiq = dataset.model_chiq.clone();
            self.kweight_results = dataset.kweight_results.clone();
        }
    }
}
