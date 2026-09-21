# Synthetic copper tutorial validation

This record covers the unreleased 50-frame raw-absorption teaching example,
checked on 2026-09-22. It is separate from the historical 100 random mixtures.
The [user tutorial](../../synthetic-copper-reduction.md) explains the workflow,
scientific interpretation and commands for reproducing the calculations.

## Retained inputs and results

- The embedded [project](../../../crates/rexafs-gui/data/examples/cu-reduction.rxs)
  retains three measured references, 50 raw mixtures and full-precision truth.
  Its SHA-256 is
  `06ca5f24db0390565b8b6955c3904ac2c544a1608824b992e02b9b46a212ac72`.
- The [compact numerical record](../../../website/public/figures/synthetic-copper/results.json)
  retains the plotted arrays, reference edge steps, settings and measured errors.
  Its SHA-256 is
  `3fff804e0c15fb1216b64cc4517a081bc7f61b47bfc02f2220d917afbfbf9c6c`.
- The [ruviz renderer](../../../crates/rexafs/examples/cu_reduction_figures.rs)
  reads that record without fitting, smoothing or regenerating spectra. The
  selected figures use ruviz 0.14.2, pinned by `Cargo.lock`. Recipe, PCA and
  comparison are each retained as SVG, PNG and PDF. The
  [export helper](../../../scripts/export-cu-figures.py) uses CairoSVG 2.8.2 with
  Cairo 1.18.4 to convert the SVGs to vector PDFs, then losslessly recompresses
  PNG streams while preserving all pixels and metadata.

The user supplied the original Athena project and requested the bundled example.
Its original bytes remain outside this checkout; its checksum and source-group
labels are retained in the project and tutorial. The software license is not
presented as a license for the original group measurements.

## Numerical checks

The optimized local macOS GUI used automatic desktop processing, uncentered
Flat-space PCA over its default interval, and default three-component MCR over
the common full energy interval. The saved MCR run converged in 110 iterations.
Known-reference native LCF used the same 517 absolute energy points for each of
the 50 mixtures. Its processed input arrays matched the saved MCR data exactly.
An independent Python enumeration of the seven nonempty reference subsets
agreed with native LCF coefficients within 1.9 × 10⁻¹².

The extraction and `--desktop-defaults` commands were then run from the bundled
project without reading the private Athena source or requiring a GUI session.
This reproduced the MCR iteration count, PCA contributions and MCR/LCF residuals.
The maximum fraction errors matched the earlier saved-GUI comparison exactly;
mean errors differed by less than 3 × 10⁻¹⁵ percentage points from the separate
Python edge-step conversion. The table and ruviz figures were generated from
this native reproduction, not rounded values copied from a screenshot.

Reported fraction errors use the same approximate reference edge-step conversion
for MCR and LCF. Automatic E₀ and baseline choices prevent interpreting this as
an exact inverse normalization for every mixture. Raw labels, normalized fit
weights and converted estimates remain separate in the numerical record.

Full GUI projects, spectral matrices, the independent comparison scripts and
browser captures are retained locally under `target/cu-reduction-defaults-20260922`
and `target/cu-tutorial-20260922`, rather than duplicating them in source control.
No runtime benchmark or claim about experimental concentration accuracy is made.

## Documentation and rendering checks

The native calculation and ruviz examples compiled and ran with `--locked
--release`; extraction verified all 55 embedded members. Rust formatting, Python
syntax and local document links passed. All displayed summary errors were
recomputed from the retained 150 coefficients.

`astro check` reported no errors, warnings or hints. The new tutorial was reviewed
in a local browser at desktop (1440 × 1000) and mobile (390 × 844) sizes. All
three figures and their download links loaded; nine math expressions rendered
without KaTeX errors. The mobile document width was 390 pixels. No page JavaScript
errors or axe accessibility violations were reported for the main content.
The ruviz images and the rendered page were also inspected visually.

The figures were revised to use ruviz's default styling in response to review;
the earlier custom Helvetica typography, palette, line styles, marker shapes
and hand-drawn legends were removed. Each panel now starts with `Plot::new()`.
Only the 5.4 × 4.05 inch export canvas, 600 dpi resolution, scientific content,
axis ranges, panel arrangement and automatic legend placement are specified.
A subsequent review reduced comparison markers to 3 points; their shape and
palette colors, and the PCA markers, remain unchanged.
The smaller canvas keeps each PNG below the repository's 1 MiB file limit while
preserving the default aspect ratio and all 50 frames. The numerical record and
bundled project hashes above are unchanged. Both PCA panels retain linear scales.

Recipe is 137.16 × 102.87 mm (3240 × 2430 pixels), PCA is 274.32 × 102.87 mm
(6480 × 2430 pixels), and comparison is 274.32 × 205.74 mm (6480 × 4860 pixels).
Each PNG includes 600 dpi metadata; each PDF embeds fonts and contains no raster
image objects. The comparison SVG has unique clipping IDs across panels. Its
PDF has one page at 777.6 × 583.2 points. All three species use the same default
palette sequence in every relevant panel; both error panels use matching limits.
The PDF rendering and native PNG were inspected separately. Means and maxima
still agree with the unchanged numerical record above.

## Pull-request qualification

All repository commit checks passed, including fixture integrity and release-tool
regressions. Strict default-feature core Clippy passed. The two tutorial examples
also passed Clippy with plotting enabled when the existing `type_complexity`
lint in `plot/fitting_plots.rs::view_r_data` was allowed; an unqualified strict
plotting run still reports that pre-existing lint.

The full native desktop suite passed 653 tests and ignored nine, with three
RMC worker tests hitting their 60-second event timeouts during parallel execution.
Those exact three tests all passed when rerun serially (212.43 seconds total).
The bundled-project, common-range, storage-cleanup and updater tests passed in
the original run. No RMC implementation or timeout was changed for this work.

The export helper was checked by comparing PNG decompressed filter/pixel streams
and every non-image-data chunk before and after compression. All were identical.
The nine downloadable figure files and the compact result record total about
1.57 MiB; the website displays the SVGs. Browser checks passed again with all
three final SVGs and all PDF/PNG download links.

The page lives under **Next tutorials · unreleased** and is excluded from default
search. These changes are local; this record does not claim public deployment or
a new desktop release.
