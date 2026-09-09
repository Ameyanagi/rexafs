# UI and import implementation review — 2026-09-08

Unreleased candidate on `feat/ui-simplification`, based on PR #45.
The maintainer's original checkout and active app sessions were preserved.

## Behavior

- Compact shell and scientific stage navigation; named SVG icons and platform menus.
  Import retains a visible label. Overview plots are optional. Everyday plot
  controls remain visible after maintainer feedback.
  A sun/moon button keeps theme switching directly available in the top bar.
  Ordinary startup opens the full Data workspace with an empty Groups panel,
  blank plotting area and Source/tools inspector, with no central import prompt.
- Common inspector parameters stay open; advanced controls have disclosure panels
  and override indicators. Source mapping is visible at the top of Data unless
  an explicitly opened tool takes that position.
- New imports require an explicit main spectrum and optional channels, grouped by
  layout. Detected and saved mappings are suggestions. Only selected channels are
  created. Mapping preview and fixed footer preserve units, file/group counts,
  source validation, missing-source repair and recipe controls.
- Remove marked captures every marked identity, includes hidden marks in its count,
  refuses changed selection/project state, and uses the existing single Undo entry.
- Processing tools and Merge use previews of private copies; Apply retains the
  existing scientific operation. Operand names and affected counts are explicit.
- Fit has compact steps, contextual result controls and responsive setup panels.
  Structure appearance uses a menu; model/data identity and blocking errors remain.
- The structure toolbar exposes Center focus: outer atoms and bonds fade smoothly
  while the absorber and selected scattering path remain clear. Path focus hides
  the surrounding bond network and leaves faint atom context. The shared sliders
  expose native numeric values and keyboard adjustment. These are display effects;
  global opacity and clipping still apply and calculation geometry is untouched.
  A separate Depth cue, enabled by default, distinguishes rear and front atoms,
  bonds and faces using camera-relative contrast, with a back/front key. It combines
  with center fading without changing alpha and adapts to both themes.
  Orbit dragging follows the pointer in both directions. Scattering legs join
  the projected atom centers; repeated traversals separate between those centers
  and their arrowheads follow the curve. Structure path/cutoff inputs and the
  processing standard filter now update with the application theme.
  Center fading now retains atom lighting at fractional opacity. Sphere
  tessellation is cached as non-overlapping bands. Bonds use their previous
  stroke appearance. Camera interaction uses a retained viewport and coalesces
  zoom readout updates after scrolling; GPUI may still rebuild ancestor views.
- Assistant keeps model, reasoning, mode and sharing/access switches visible.
  Conversation utilities and longer descriptions use Settings. Permission and
  approval behavior retain the existing backend and busy-state guards.
  Its retained view now follows application theme changes, including composer and
  history inputs in both docked and separate-window hosts. Access text uses the
  theme warning color.
- Series opens the actual scan selector, suppresses invalid frame/run UI and
  preserves marks during frame navigation. Publish has one export area, explicit
  format/scope and a large preview; Style remains open by default.
- Publish provides XANES/full-energy flattened views with a Normalized toggle,
  weighted χ(k) to FFT k-max + 1, |χ(R)| and R-space fits over 0–6 Å (expanded for
  wider fit ranges). Common views come first, FFT windows are hidden, and defaults
  are 300 DPI with legends, grid and Typst title/axis labels. Optional components
  remain available, and CSV arrays retain their full measured/computed grids.
- Auto normalization maximum now follows the spectrum endpoint instead of the
  library constructor's 2000 eV value. Explicit limits stay explicit. Background
  k-weight has a default-off Link to FFT that preserves the independent setting,
  and participates in project persistence, cache invalidation, copy/reset and undo.
- Colors offers three cycles and three gradients, swatch previews, reverse and
  reset. Assignments cover the plot scope and persist per group, including colors
  of members beyond the display sampling cap. Group swatches, all processing
  quadrants and legends share the assignment; changing current cannot reshuffle
  colors. Batch color changes undo as one action. The theme toggle follows Save
  Project, and Help is rightmost.
- AccessKit connects rendered controls, focus handles and actual callbacks to
  native accessibility. Main icons, navigation, inputs, dialogs and disclosure
  controls expose names and states. Adapters are included for macOS, Windows and
  Linux; native validation so far is macOS only.

## Validation

- `cargo test -p rexafs-gui`: **433 passed, 0 failed, 5 ignored** (159 s).
  Ignored cases include the opt-in external-folder test, run separately below.
- Targeted tests passed for explicit selected-channel intake, replacement of the
  main channel, immutable processing previews, and complete marked identity capture.
- After adding structure focus and camera-relative contrast, nine depth tests and
  five molecular-view tests passed, including equal-radius front/back distinction,
  rotation, dark/light contrast, unchanged alpha, highlighted-path picking and
  preservation of explicit clipping.
- After the drag/path follow-up, all nine molecular-view tests passed, including
  pointer direction across camera angles, exact path endpoints, separation of
  repeated legs, clipping continuity and arrow tangent alignment.
- After correcting Assistant theme synchronization, all 47 Assistant tests passed.
- Optimized build passed with the distributed `refeff-runner,feff10-runner` features.
- The earlier optimized package at source commit `cb4838d` passed the extracted Cu
  example/pipeline and both FEFF backend self-checks. Its build metadata records a
  clean source tree and the same commit as the embedded build identity.
- `cargo fmt --all` and `git diff --check` passed.

A local opt-in integration test used the maintainer-designated 102-file folder.
It discovered one layout, validated Fluorescence and Reference for every source,
accepted exactly 102 groups of each channel and zero Transmission groups, and
processed all 204 spectra successfully. It checked 127,296 raw points and verified
that source size/mtime revisions did not change. No source data or identifying
header metadata is included in this repository.

Reproduce locally with `REXAFS_IMPORT_VALIDATION_DIR=/path/to/folder cargo test
-p rexafs-gui validate_selected_channels_in_external_folder -- --ignored --nocapture`.
This opt-in test expects a folder suitable for Fluorescence and Reference.

## Native observations and limits

Earlier debug candidates were inspected in isolated macOS app bundles. Native
accessibility exposed application controls, selected/disabled states, import
mapping fields and modal isolation; setting an E0 field through accessibility
updated the app. Native inspection found invisible SVG strokes and missing Tab
stops, which were corrected. The maintainer used those candidates and identified
excessive hiding and automatic Transmission import; this implementation includes
those corrections.

Earlier native checks were interrupted by a failed computer-use connection. The
connection subsequently recovered, and optimized candidates were inspected with
Computer Use on macOS. Verified workflows now include:

- Cu example import: explicit μ-column selection, eV confirmation, validation,
  one imported group and the expected E0 of about 8977.5 eV.
- Normalize's automatic upper limit displays “spectrum end”. Applying Okabe–Ito
  changes both the group swatch and live spectrum curve.
- Remove marked shows the captured file and count, removes the disposable group,
  and one Undo restores its identity, mark, color and publication preview.
- Link to FFT replaces the independent AUTOBK weight with “2 · from FFT”;
  unlinking restores the independent Auto (1) setting and replots the spectrum.
- Publish renders the Cu XANES figure, legend, grid and Typst axis labels, and
  exposes the 300 DPI default. Native inspection identified clipped Style and
  unlinked background-weight labels; the controls now receive enough width.
- Restored structure bonds, center fading, depth cue, rotation and wheel zoom
  were inspected. The later performance checks and their limits are recorded in
  [the measured renderer review](../2026-09-09-structure-performance/review.md).

Broader native qualification remains open for narrow-window workflows, populated
Series navigation, completed Fit/Assistant busy and approval states, and complete
keyboard/screen-reader coverage. Windows/Linux native adapters are not runtime
qualified in this macOS session. These limits do not replace the automated
pipeline, import, project and fitting coverage above.

## Follow-up qualification (2026-09-09 JST)

`cargo test -p rexafs-gui` passed **449 tests, 0 failures, 5 ignored** (119.76 s).
The follow-up suite covers publication presets/Typst output, normalization Auto
on both complete and truncated spectra, the actual AUTOBK/FFT link on/off pipeline,
non-overlapping sphere opacity, palette repetition
and gradient reversal, cross-quadrant colors with mixed FFT weights, and saved
identity-based palette undo/redo. Rendered publication PNGs were visually inspected
for XANES (flattened and normalized), full flattened energy, χ(k), |χ(R)| and the
R-fit layout. The R-fit layout uses a synthetic internal fixture; it is not an
experimental fit result. Legends with punctuation remain literal while axis math
uses Typst.

Native access initially failed for this follow-up, then recovered. The current
observations and measured CPU rendering costs are listed above; no physical
display frame-rate guarantee is claimed.

The maintainer reported worse-looking bonds in `dacd740`. The subsequent correction
restores bond strokes, widths, shading branches and depth subdivision exactly from
`dc9ed30`, and removes the new gradient-stick renderer. Cached atom shading and
retained-viewport camera interaction remain.
All ten molecular-view tests passed after the bond restoration. The bond paint
branch and depth-line helper were also compared directly with `dc9ed30` and match
exactly. Formatting and diff checks passed.


The performance revision passes **449 GUI tests, zero failures, five ignored**
with both distributed FEFF features (10.02 s after compilation). Its new regression
compares straight-stroke vertices and triangle coverage against the original Lyon
renderer, including short segments that collapse at the tessellator tolerance.
