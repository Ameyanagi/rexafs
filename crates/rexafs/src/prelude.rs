//! Common spectrum, fitting, structure and data-treatment imports.
//!
//! Use `use rexafs::prelude::*;` for examples spanning several modules, or import
//! individual types from the crate root to keep dependencies explicit. This
//! prelude re-exports existing types and traits; it performs no initialization.
//! Feature-gated exports, such as plotting and ndarray conversion traits, are
//! available only when the corresponding Cargo feature is enabled.

pub use crate::xafs::xasgroup::XASGroup;
pub use crate::xafs::xasspectrum::XASSpectrum;

pub use crate::xafs::background::{AUTOBKClampScalePolicy, AUTOBKSolver, BackgroundMethod, AUTOBK};
pub use crate::xafs::fitting::{
    feffit_independent, feffit_joint, feffit_joint_with_options, feffpath, ff2chi,
    parse_feff_path_file, path2chi, resolve_feff_commands, run_feff, run_feff_and_load_paths,
    DatasetResult, FeffBatchExecutionStrategy, FeffBatchOptions, FeffDat, FeffExecutionMode,
    FeffFit, FeffFitDataset, FeffFitJacobianMode, FeffFitOptions, FeffFitResult,
    FeffFitSolverMethod, FeffFitTransform, FeffFlavor, FeffModuleCommand, FeffPathModel,
    FeffResolvedCommands, FeffRunRequest, FeffRunResult, FitSolverReport, FitSpace, FitVariable,
    FitVariables, FitWarning, KweightResult, NoiseEstimate, Param, PathContribution, PathParamSpec,
};
pub use crate::xafs::io;
pub use crate::xafs::io::athena::{AthenaGroup, AthenaParams, AthenaProject, AthenaValue};
pub use crate::xafs::io::xdi::{XdiColumn, XdiError, XdiFile, XdiHeader, XdiSignal};
pub use crate::xafs::lmutils::LMParameters;
// pub use crate::xafs::mathutils;
pub use crate::xafs::normalization::{Normalization, NormalizationMethod};
#[cfg(feature = "ndarray-compat")]
pub use crate::xafs::nshare::{ToNalgebra, ToNdarray1};
pub use crate::xafs::xafsutils::{FTWindow, XAFSUtils};
pub use crate::xafs::xrayfft::{FFTGrid, FFTUtils, XrayFFTF, XrayFFTR};

#[cfg(feature = "plotting")]
pub use crate::plot::{PlotError, PlotXAS, XASPlotBuilder};
pub use crate::xafs::analysis::{
    lcf, lcf_batch, lcf_batch_with_progress, lcf_combinatorial, mcr_als, mcr_als_with_progress,
    pca_train, AnalysisSpace, LcfComponent, LcfConfig, LcfResult, LcfSpace, McrAnchor, McrConfig,
    McrResult, McrTermination, PcaConfig, PcaCountBasis, PcaCountSuggestion, PcaFit, PcaModel,
    PcaReconstructionError,
};
pub use crate::xafs::errors::AnalysisError;
pub use crate::xafs::fitting::template::{
    apply_template, ParameterTemplate, PathAssignment, TemplateResult, TemplateVariable,
};
pub use crate::xafs::structure::{
    absorber_sites, build_cluster, read_cif, structure_from_cif, write_feff_inp, AbsorberSelection,
    Cluster, ClusterAtom, ClusterOptions, Edge, Element, FeffInputOptions, FeffInputStyle, Lattice,
    PathGeometry, PathLeg, Site, Species, Structure, StructureError, StructureHit, StructureQuery,
    StructureSource,
};
pub use crate::xafs::structure::{
    rank_paths, select_by, select_default, shells_of, BuiltinLibrary, PathInfo, ShellInfo, Xyz,
    XyzAbsorber,
};
pub use crate::xafs::tools::{
    difference, merge_spectra, DiffSpace, EdgeFeature, MergeConfig, MergeGrid, MergeWeight,
    RebinConfig, RebinMethod, RebinOutput,
};
pub use crate::xafs::xafsutils::ConvolveForm;
pub use crate::{Group, PrePostEdge, Spectrum};

pub use crate::xafs::analysis::metrics::{
    AxisOrigin, Measurement, MeasurementArrays, MeasurementResult, MeasurementSpace, Metric,
    MetricError, MetricValue,
};
pub use crate::xafs::analysis::peakfit::{
    PeakComponent, PeakContribution, PeakFit, PeakFitError, PeakFitResult, PeakRole, PeakShape,
    PeakTermination,
};

pub use crate::xafs::fluorescence::{
    AbsorptionMode, FluorescenceCorrection, FluorescenceCorrectionResult, FluorescenceError,
};
pub use crate::xafs::mback::{MBack, MbackErfc, MbackOptions, MbackResult};

// Since 0.2.10, native Cauchy wavelet analysis.
pub use crate::xafs::wavelet::{
    Wavelet, WaveletError, WaveletMap, WaveletPreparation, WaveletRegionValue, WaveletSize,
    WaveletWindow,
};
