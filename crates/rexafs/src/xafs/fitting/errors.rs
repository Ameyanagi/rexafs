use thiserror::Error;

use super::types::{FeffExecutionMode, FeffFlavor};

#[derive(Error, Debug, Clone)]
/// Failures encountered while loading paths, evaluating models, fitting, or running backends.
/// This describes an operation failure; a successful fit result can separately
/// report numerical nonconvergence in its solver report.
pub enum FittingError {
    #[error("unsupported FEFF flavor for this build: {flavor:?}")]
    /// The selected path-file parser flavor is unavailable.
    UnsupportedFeffFlavor {
        /// Requested path-file format selector.
        flavor: FeffFlavor,
    },

    #[error("failed to parse FEFF path file '{path}': {reason}")]
    /// A path file could not be read or parsed.
    ParseFailed {
        /// Source or destination filesystem path associated with the failure.
        path: String,
        /// Explanation of the failed validation or underlying operation.
        reason: String,
    },

    #[error("invalid FEFF path data: {reason}")]
    /// Parsed path arrays or geometry are unusable for model evaluation.
    InvalidFeffData {
        /// Explanation of the failed validation or underlying operation.
        reason: String,
    },

    #[error("expression evaluation failed for '{expr}': {reason}")]
    /// A mathematical expression could not be parsed or evaluated finitely.
    ExpressionFailed {
        /// Expression text that could not be evaluated.
        expr: String,
        /// Explanation of the failed validation or underlying operation.
        reason: String,
    },

    #[error("undefined symbol in expression: {symbol}")]
    /// An expression refers to a variable with no resolved value.
    UndefinedSymbol {
        /// Named parameter involved in the unresolved or cyclic dependency.
        symbol: String,
    },

    #[error("cyclic variable expression dependency detected at: {symbol}")]
    /// Variable expressions depend cyclically on one another.
    CyclicExpression {
        /// Named parameter involved in the unresolved or cyclic dependency.
        symbol: String,
    },

    #[error("invalid fit transform configuration: {reason}")]
    /// The requested transform or residual-selection settings are unusable.
    InvalidTransform {
        /// Explanation of the failed validation or underlying operation.
        reason: String,
    },

    #[error("invalid fit dataset: {reason}")]
    /// Input arrays or fit parameter values fail a required consistency check.
    InvalidDataset {
        /// Explanation of the failed validation or underlying operation.
        reason: String,
    },

    #[error("fitting requires at least one active FEFF path")]
    /// No enabled theoretical paths are available to sum or fit.
    EmptyPaths,

    #[error("fitting requires at least one varying variable")]
    /// Legacy error variant for a fitting request without varying parameters.
    NoVaryingVariables,

    #[error("nonlinear solver failed: {reason}")]
    /// A numerical solver or its execution environment failed.
    SolverFailed {
        /// Explanation of the failed validation or underlying operation.
        reason: String,
    },

    #[error("invalid FEFF executable path '{path}': {reason}")]
    /// The configured external executable path is invalid.
    InvalidExecutablePath {
        /// Source or destination filesystem path associated with the failure.
        path: String,
        /// Explanation of the failed validation or underlying operation.
        reason: String,
    },

    #[error("FEFF workspace path does not exist or is not a directory: '{path}'")]
    /// The requested calculation workspace is absent or is not a directory.
    WorkspaceNotFound {
        /// Source or destination filesystem path associated with the failure.
        path: String,
    },

    #[error("FEFF input file was not found: '{path}'")]
    /// The requested scattering input file could not be located.
    FeffInputNotFound {
        /// Source or destination filesystem path associated with the failure.
        path: String,
    },

    #[error("required FEFF module executable could not be resolved: {module}")]
    /// A required external calculation stage could not be resolved.
    ExecutableNotFound {
        /// Calculation stage or backend name associated with the failure.
        module: String,
    },

    #[error("requested FEFF execution mode is not available in this build: {mode:?} ({reason})")]
    /// The requested calculation backend is disabled in this build.
    UnsupportedExecutionMode {
        /// Requested scattering backend.
        mode: FeffExecutionMode,
        /// Explanation of the failed validation or underlying operation.
        reason: String,
    },

    #[error("failed to spawn FEFF module '{module}' using '{executable}': {reason}")]
    /// An external calculation stage could not be started.
    ProcessSpawnFailed {
        /// Calculation stage or backend name associated with the failure.
        module: String,
        /// External executable path that could not be started.
        executable: String,
        /// Explanation of the failed validation or underlying operation.
        reason: String,
    },

    #[error("FEFF module '{module}' exited with non-zero status {code}")]
    /// An external calculation stage returned a failing exit status.
    ProcessFailed {
        /// Calculation stage or backend name associated with the failure.
        module: String,
        /// Recorded process exit code; may use a sentinel if the platform supplies none.
        code: i32,
    },

    #[error("FEFF module '{module}' timed out after {timeout_sec}s")]
    /// A calculation stage or cooperative backend run exceeded its requested limit.
    ProcessTimedOut {
        /// Calculation stage or backend name associated with the failure.
        module: String,
        /// Requested timeout interval in seconds.
        timeout_sec: u64,
    },

    #[error("failed to read FEFF module output for '{module}': {reason}")]
    /// Output from an external calculation stage could not be read.
    OutputReadFailed {
        /// Calculation stage or backend name associated with the failure.
        module: String,
        /// Explanation of the failed validation or underlying operation.
        reason: String,
    },

    #[error("FEFF10 pipeline failed: {reason}")]
    /// The FEFF10 backend failed to configure or complete a calculation.
    Feff10PipelineFailed {
        /// Explanation of the failed validation or underlying operation.
        reason: String,
    },

    #[error("ReFEFF pipeline failed: {reason}")]
    /// The ReFEFF backend failed to configure or complete a calculation.
    RefeffPipelineFailed {
        /// Explanation of the failed validation or underlying operation.
        reason: String,
    },

    #[error("FEFF execution produced no path output files (feffNNNN.dat) in '{workspace}'")]
    /// A completed calculation did not produce loadable path files.
    NoPathOutputs {
        /// Workspace that was inspected for generated path files.
        workspace: String,
    },

    #[error("I/O failure during {action} for '{path}': {reason}")]
    /// A filesystem operation failed while preparing or collecting a calculation.
    IOFailed {
        /// Description of the attempted filesystem operation.
        action: String,
        /// Source or destination filesystem path associated with the failure.
        path: String,
        /// Explanation of the failed validation or underlying operation.
        reason: String,
    },
}
