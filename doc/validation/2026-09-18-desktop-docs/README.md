# Desktop documentation capture review

The documentation refresh uses the macOS ARM64 archive from the v0.2.10
[Release builds run](https://github.com/Ameyanagi/rexafs/actions/runs/35316272629),
commit `0b3aca3331e38ac4a34364824a74e96271f5ca1d`. Its archive checksum was
verified before extraction. This is the tagged CI artifact before signing and
publication, not a qualification of the final signed download.

## Inputs and capture

All eleven images are original, unedited 2880 × 1800 Retina window captures
of a 1440 × 900 logical window. A separate rexafs process used isolated settings.
Native macOS accessibility controls and window-specific `screencapture` were
used because the computer-use connector could not initialize:
`CUA_REPL_ENABLED_SURFACES is required`. No mockups, plot substitutions, crops,
resizing or annotations were applied.

The only inputs are the public XAS Data Library room-temperature and 10 K
Cu foil measurements, retained unchanged in the repository with CC0 notices.
The [public capture manifest](../../../website/public/screenshots/0.2.10/capture.json)
records their original sources, checksums and attribution alongside the build
and every screenshot. No private ReGe measurement or private project was opened
in the capture process or copied into the documentation.

## Observed behavior

- Import preview retained 408 and 612 points respectively and used each file's
  stored `mutrans` column. The 10 K import screenshot precedes confirmation.
- Normalize switched between Polynomial and MBACK in the same sidebar. MBACK
  detected Cu/K from the header. Pre/post showed the polynomial baselines;
  MBACK fit showed the complete atomic model over measured absorption.
- Background displayed a single AUTOBK spline and its Fourier magnitude,
  without normalization fit overlays.
- Transform → Wavelet displayed aligned k and R marginal plots. Committing
  R maximum = 5 Å automatically recalculated the map without Calculate; 6 Å
  was restored for the documented setup.
- Assistant and Parameters remained visible together. Model and Access menus
  opened successfully. No message was sent; Review, the automatic model and
  disabled workspace commands were retained.
- Series → Use loaded groups created a two-frame series. Add trend displayed
  Flat/Maximum with 0–30 eV bounds and visible range handles. Wavelet/Integral
  copied the Transform setup and calculated both frames over k = 4–10 Å⁻¹ and
  R = 1–3 Å. The saved result reported 2/2 complete.
- Difference retained frame 1 as its reference. Right selected frame 2,
  changing the displayed spectrum while retaining the reference and saved
  integral trend. Hiding Parameters widened the overview without changing data.

These checks establish GUI behavior, not scientific accuracy. The two Cu files
have different acquisition conditions and were not aligned for the difference
illustration. They are not a time series or a controlled temperature experiment.
Independent experimental algorithm comparisons remain in the existing MBACK
and wavelet validation records.

## Documentation scope

The import, processing, Assistant and Series guides use the new figures and
version labels. Processing distinguishes MBACK fitting from AUTOBK, and Series
explains the route from Transform settings to saved wavelet-region trends.
Older captures remain intact with their original provenance. Download metadata
and Stable API membership are unchanged by this documentation update.

The images add approximately 4.6 MB to the repository. They are website assets;
no application build, raw measurement copy, generated website, test output or
private project is included.

## Website validation

- Astro check: 38 files, no errors, warnings or hints.
- Production build: 184 pages; Rust Stable/Next references and both browser
  engines prepared with their normal checksum checks.
- Website tests: 23 passed, including local links, anchors and assets.
- Browser suite: 25 passed, including accessibility, responsive layout,
  downloads and browser processing.
- Additional rendered-page review: all eleven new images loaded at their
  original dimensions with descriptive alternative text. Import, processing,
  Series and Assistant pages had no horizontal overflow at 390 px. Desktop
  MBACK and mobile wavelet-trend views were inspected visually.
- Archive/input/image hashes, image dimensions and `git diff --check` passed.

Build output, browser test screenshots and temporary GUI state remain outside
the committed documentation assets. No code or numerical API changed.
