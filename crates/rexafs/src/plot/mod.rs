//! Optional spectrum, group and fit plotting through ruviz.
//!
//! Enable Cargo feature `plotting`, import [`PlotXAS`], and select one or more
//! panels from `spectrum.plot()`. Rendering may compute missing processing stages
//! and therefore requires a mutable spectrum/group. A fit-result plot uses the
//! stored fit data; it does not rerun the optimizer.
//!
//! ```no_run
//! use rexafs::{io, plot::PlotXAS};
//! let mut spectrum = io::read_qas_transmission("scan.dat")?;
//! spectrum.plot().k().kweight(2.0).r().save_png("overview.png")?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! Plot options following a panel selector modify that panel. PNG combines
//! multiple panels; [`XASPlotBuilder::to_svg_panels`] returns separate SVG strings.
//! A Fourier plot uses the processed signal's units and phase convention, so an
//! uncorrected R peak is not directly a bond length. Plotting changes presentation,
//! not the physical interpretation described in the
//! [processing guide](https://rexafs.com/docs/science/processing/).

mod builder;
mod config;
mod errors;
mod fitting_plots;
mod group_plots;
mod panels;
mod spectrum_plots;
mod traits;

pub use builder::XASPlotBuilder;
pub use errors::PlotError;
pub use traits::PlotXAS;
