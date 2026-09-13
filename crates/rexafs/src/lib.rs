//! Rust-powered X-ray absorption analysis.
//!
//! Start with [`Spectrum`]: calling `fft()` computes missing prerequisite stages.
//! Advanced APIs live in [`fitting`], [`structure`] and [`io`].
//! Developed under the codename xraytsubaki.
//!
//! Scientific background: [Newville et al., AUTOBK (1993)](https://doi.org/10.1103/PhysRevB.47.14126)
//! and [Rehr and Albers, XAFS theory (2000)](https://doi.org/10.1103/RevModPhys.72.621).
//! The fixed endpoint penalty is a rexafs-specific choice; Fourier peaks are not automatically phase corrected.

pub mod parser;
#[cfg(feature = "plotting")]
pub mod plot;
pub mod prelude;
pub mod xafs;

pub use xafs::background::{BackgroundMethod, AUTOBK};
pub use xafs::normalization::{NormalizationMethod, PrePostEdge};
pub use xafs::xasgroup::XASGroup as Group;
pub use xafs::xasspectrum::XASSpectrum as Spectrum;
pub use xafs::xrayfft::{FFTGrid, XrayFFTF, XrayFFTR};
pub use xafs::{analysis, fitting, io, structure, tools, Result, XAFSError as Error};
