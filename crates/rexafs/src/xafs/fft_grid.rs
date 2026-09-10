use serde::{Deserialize, Serialize};

/// Forward-transform sampling and window domain. The default backend retains
/// `Input`; the optional ndarray backend retains its extended `Larch` grid.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FFTGrid {
    /// Weight/window the supplied grid without resampling (historical default).
    #[default]
    Input,
    /// Resample from k=0, extend the window through kmax+dk2, then truncate
    /// the weighted chi and window to the measured range, as in Larch.
    Larch,
}
