use serde::{Deserialize, Serialize};

/// Forward-transform sampling and window domain. The default backend retains
/// `Input`; the optional ndarray backend retains its extended `Larch` grid.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FFTGrid {
    /// Weight/window the supplied grid without resampling (historical default).
    /// For physical R interpretation, supply uniform samples beginning at zero
    /// with spacing equal to `kstep`; this mode does not enforce those assumptions.
    #[default]
    Input,
    /// Resample from k=0, extend the window through kmax+dk2, then truncate
    /// the weighted chi and window to the measured range, as in Larch.
    /// Linear interpolation holds the nearest endpoint outside measured k.
    /// Requires nonnegative k and `nfft` large enough for the extended window.
    Larch,
}
