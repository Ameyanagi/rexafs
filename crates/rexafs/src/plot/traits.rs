use crate::plot::builder::XASPlotBuilder;
use crate::xafs::fitting::types::FeffFitResult;
use crate::xafs::xasgroup::XASGroup;
use crate::xafs::xasspectrum::XASSpectrum;

/// Begin a plot of a spectrum, group or stored fit result.
/// Available with Cargo feature `plotting`; rendering can calculate missing
/// spectrum prerequisites, so the builder borrows its source mutably.
pub trait PlotXAS {
    /// Create a builder with no selected panels, 800 × 600 pixel output and a visible legend.
    /// Select `.mu()`, `.norm()`, `.k()` or `.r()` before rendering.
    fn plot(&mut self) -> XASPlotBuilder<'_>;
}

impl PlotXAS for XASSpectrum {
    fn plot(&mut self) -> XASPlotBuilder<'_> {
        XASPlotBuilder::for_spectrum(self)
    }
}

impl PlotXAS for XASGroup {
    fn plot(&mut self) -> XASPlotBuilder<'_> {
        XASPlotBuilder::for_group(self)
    }
}

impl PlotXAS for FeffFitResult {
    fn plot(&mut self) -> XASPlotBuilder<'_> {
        XASPlotBuilder::for_fit(self)
    }
}
