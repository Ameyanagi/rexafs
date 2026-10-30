# rexafs 0.2.15

Published on 29 October 2026 in Japan (28 October UTC), with multi-panel
publication figures, a combined Process stage, fit windows that follow
processing, and safer project saving. The tagged build, signed Mac artifacts,
installed-DMG checks and public package verification passed. See the
[downloads](https://github.com/Ameyanagi/rexafs/releases/tag/v0.2.15) and
[qualification record](validation/2026-10-29-release-0.2.15/review.md).

## Multi-panel publication figures

**Publish → Multi-panel** lays out XANES, χ(k), |χ(R)| and fit figures as
labelled (a), (b), (c) panels at journal column widths, with a combined caption.
Export PNG, TIFF, SVG or PDF at the printed size and resolution. Save several
named recipes per project; older layouts open as **Figure 1**.

Each panel can show several workspace samples, stacked with vertical offsets so
spectra do not overlap. Short figure labels keep the full sample names in the
caption. Choose an ordered viridis, Okabe–Ito or workspace palette, or set
individual curve colours. Labels sit just above each curve in its colour, on
the side of the panel where fewer labels touch another curve; a legend remains
available per panel. Optional insets show pre-edge or white-line detail, or
normalized differences from the first sample, over editable E₀-relative ranges.

Every composed figure can also be exported as unshifted curve CSVs and an
editable matplotlib script that needs only Python 3, numpy and matplotlib.
Reports and supporting-information bundles include each named recipe under
`figures/composed/`. See the [publication guide](publication.md).

## Single-figure and report output

- **Fit + residual** figures for χ(k) and |χ(R)|; k-space figures are scaled
  symmetrically about zero.
- Export the latest fit's parameters as a LaTeX, CSV or Word table, with
  uncertainties rounded to their significant figures.
- Reports include fit statistics, and the methods paragraph describes the
  settings actually applied. A supporting-information zip contains the report,
  a self-contained project and CSV data.
- SVG files record their physical size in inches, so a 3.3 in figure imports at
  3.3 in. Plain labels stay editable `<text>`.
- Built-in plot labels and legends use an Arial-style sans-serif face. A legend
  placement setting has an **Auto** choice that avoids the data.

## Process stage

Normalize, Background and Transform are now one **Process** stage. Its linked
μ(E), normalized μ(E), kʷχ(k) and |χ(R)| plots show every range on each plot it
affects. ⌘1–⌘5 select Data, Process, Fit, Series and Publish.

Parameter labels use sentence case with units, for example "Spline k max".
Hovering a label explains it, gives its unit and Auto value, and names the
matching Larch or Athena parameter. **Normalization unreliable** appears when
the post-edge fit has too few points or too short a span, or the edge step is
not trustworthy. Group rows show the name on one line and the source file,
quantity and E₀ on a second.

## EXAFS fitting

New FEFF fits follow each spectrum's Process transform window and taper,
independently of the k-weight choice. Custom k and R windows remain available.
Older projects keep their Kaiser–Bessel window with dk = 4 so their saved fits
reproduce. Fit history and reports record the windows actually used.

Fit curves are saved with the project, so Fit → Results shows the last fit
after reopening without refitting. **Needs update** still appears when the
inputs or model changed. Parameters are named by symbol, unit and path, for
example "ΔR (Å) · Cu–Cu 2.557 Å". Reduced χ² reads "n/a (noise not estimated)"
when the noise level ε was not estimated from the data.

The trust-region solver now uses apex-solver 1.5. rexafs enforces parameter
bounds itself, as before, so fit results are unchanged.

## Projects and desktop

- Closing the window or quitting with unsaved changes asks to save, discard or
  cancel.
- **Save project** starts with **Include source files** selected, so saved
  projects are portable by default.
- Merge saved spectra from their retained arrays without the original files,
  and use the merged result as an alignment standard in the same session.
- On macOS, the Dock icon reopens the analysis window and Finder opens `.rxs`
  files through the normal project loader.
- **Settings…** (⌘, or Ctrl+,) and an **About** panel show the version, commit,
  update channel, licence and third-party notices.
- Normalization, Wavelet and XANES peak-fit failures appear in the problems
  drawer. A failed background job no longer leaves later settings changes
  unapplied.
- Structure radius edits outside 2–12 Å are rejected and the last valid radius
  is restored.

## Security

Continuous integration installs Node 24.22.1 or later and Bun 1.4.4, which fix
the Brotli remote-code-execution flaw CVE-2026-111497. Website dependencies and
the Larch fixture environment are updated. Rust dependencies with RustSec
advisories (rustls, rustls-webpki, webpki-roots, zip, zstd-sys) are updated, and
CI now fails on unreviewed advisories. The remaining exceptions in `deny.toml`
record why each is not reachable.

## Compatibility and platforms

Project format remains 1. Saved composer panels without a label setting keep
their legend; panels saved with curve-end labels open with labels above the
curves. The 0.2.15 writer adds linked and embedded compatibility fixtures;
historical fixture bytes and data attribution remain intact.

Platform policy is unchanged: macOS desktop and Python distributions require
Apple Silicon. Windows and Linux desktop packages remain previews. The release
qualifies three Python ABI3 wheels on CPython 3.10–3.14 and keeps the Rust,
Python, npm/WebAssembly and desktop versions coordinated.
