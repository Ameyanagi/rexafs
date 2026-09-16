# rexafs 0.2.9

Published on 16 September 2026. The
[qualification record](validation/2026-09-16-release-0.2.9/review.md) tracks the
reviewed source, exact-tag build, signing and package publication.

## Collection analysis

The desktop adds native multivariate curve resolution by alternating least
squares (MCR-ALS), alongside linear combination fitting (LCF) and principal
component analysis (PCA). These calculations use the Rust core. MCR estimates
component spectra and nonnegative mixture coefficients, with optional closure
and spectral nonnegativity. Small residuals do not establish unique chemical
components. See the [analysis API guide](analysis-api.md) for the mathematical
assumptions, defaults, constraints and references.

Missing normalization or background arrays are now prepared on temporary copies
using each spectrum's settings. Existing selected arrays are reused and source
spectra remain unchanged. Core defaults stay on normalized absorption; select
`AnalysisSpace::Flat` explicitly for flattened absorption. The desktop starts
on flat and permits choosing norm. Energy bounds are offsets from E₀; all
analyses reject reversed, nonfinite or incompletely covered ranges.

Batch LCF preserves a result or contextual error for every completed target,
including when cancelled. PCA offers reconstruction error versus component
count, numerical-rank and indicator diagnostics, and linear or logarithmic
axes. Centered PCA can describe closed three-component mixtures with two
varying directions plus the mean; its count suggestion is not a species count.

MCR components and LCF calculated spectra can be added to Groups with their
source identities and settings. MCR components can continue through background
subtraction, Fourier transforms and fitting over the measured range retained
in the calculation. They preserve their prepared scale by default; an explicit
pre/post-edge refit is available, including for flattened inputs.

## Desktop workflows

The collection plot offers all spectra, a sampled preview and a color gradient.
View presets and calculation bounds are separate, so zooming does not change
the fit interval. Series plots now follow the selected frame. Analysis results
and metadata survive project save/reopen and are available in exports.

Multi-scan imports retain selections across scans, including Athena and Larix
projects. The Assistant receives bounded context and can retrieve relevant
analysis details instead of sending whole project arrays in every request.

The release also contains the macOS, Windows and Linux updater and expanded
measurement discovery introduced in [0.2.8](release-notes-0.2.8.md). The 0.2.8
registry packages were published; its GitHub desktop release remained a draft
when 0.2.9 was prepared. Existing tags and package bytes are preserved.
Desktop users on 0.2.7 or earlier need one manual installation to acquire the
new updater; subsequent supported installations offer **Update and restart**.

## Compatibility and validation

The coordinated release includes Rust, Python, npm/WebAssembly and six desktop
targets. **Public Python and TypeScript LCF/PCA/MCR bindings remain planned**;
this release adds the native Rust APIs and desktop workflows. Project format 1
is retained, with additive analysis state and new linked/embedded fixtures.

The [Cu experiment](cu-mixture-analysis.md) uses 100 reproducible synthetic
mixtures and retained Cu foil, Cu₂O and CuO references. Its tests and desktop
checks distinguish synthetic recovery from experimental accuracy and blind
factorization from reference-informed analysis. Test measurements, synthetic
fixtures and screenshots are excluded from the published Rust crate.

Local checks and screenshots support review. Publication still requires the
complete GitHub build of the immutable release tag, verified package bytes and
signed Mac downloads. Windows and Linux remain desktop previews; automated
runner checks do not establish physical-desktop interactive qualification.
