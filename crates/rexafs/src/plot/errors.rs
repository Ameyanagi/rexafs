use crate::xafs::XAFSError;
use thiserror::Error;

/// Plot configuration, prerequisite calculation, rendering or output failure.
/// Options are validated when rendering begins; errors do not roll back stages
/// already calculated on the source spectrum or group.
#[derive(Debug, Error)]
pub enum PlotError {
    #[error("missing data: {field}")]
    /// A selected panel requires an array that is unavailable.
    MissingData {
        /// Missing field or result name.
        field: &'static str,
    },

    #[error("index out of range: index {index}, len {len}")]
    /// A requested group member or fit dataset does not exist.
    IndexOutOfRange {
        /// Zero-based failing index.
        index: usize,
        /// Number of elements in the array or collection.
        len: usize,
    },

    #[error("invalid plotting option: {reason}")]
    /// A plot option was used with an incompatible panel/source or before selecting a panel.
    InvalidOption {
        /// Human-readable failure reason.
        reason: String,
    },

    #[error("spectrum compute failed at index {index}: {source}")]
    /// A prerequisite calculation failed for one group member; earlier members may already have been processed.
    SpectrumCompute {
        /// Zero-based failing index.
        index: usize,
        /// Underlying typed failure from the spectrum stage.
        #[source]
        source: XAFSError,
    },

    #[error("plot backend error: {0}")]
    /// The plotting backend failed to render or write output.
    Ruviz(#[from] ruviz::core::PlottingError),

    #[error("single-plot output requested for multi-panel selection")]
    /// An operation requiring one panel received a multi-panel selection; use PNG output or separate SVG panels.
    MultiPanelRenderUnsupported,

    #[error("no data selected for plotting")]
    /// No panels or spectrum members were selected for plotting.
    EmptySelection,

    #[error("xas computation failed: {0}")]
    /// A prerequisite calculation failed for a single spectrum.
    Xafs(#[from] XAFSError),
}

impl PlotError {
    /// Construct an owned invalid-option diagnostic for a panel/source combination.
    pub fn invalid_option(reason: impl Into<String>) -> Self {
        Self::InvalidOption {
            reason: reason.into(),
        }
    }
}
