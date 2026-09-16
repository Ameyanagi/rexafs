# Complete analysis implementation

Development branch: `feature/complete-analysis`, based on `dev` at `513cc1c`.
The [design](complete-analysis-design.md) specifies eight milestones. This record
distinguishes implemented work from future scope; no new capability is released yet.

## Current increment: first offline full-frame measurements

Implemented portions of A1–A3:

- Native-grid point, maximum, integral and mean operators with strict coverage
  and finite-value checks; an advanced array centroid operator.
- A one-call spectrum API that prepares only the required stage on a copy.
  Normalized XANES does not require background subtraction or a Fourier transform.
- Ordered group identities, input/settings revisions, definitions and per-frame outcomes.
- Bounded calculation chunks off the UI thread, cancellation, compatible resume
  and retained history.
- A Series measurement editor for imported groups and folder scans, named-frame
  preview, every-frame trends, paginated results, and complete CSV/JSON export.
- Project save/reopen of definitions, settings, identities, results and provenance.

The [API and workflow guide](full-frame-measurements.md) describes the current
contract. [Computer-use qualification](validation/2026-09-16-full-frame-measurements/README.md)
records 513 synthetic spectra, their known transient, save/reopen and exports.
Core tests passed (357); GUI tests passed (526); strict core Clippy passed.

Series organization is also implemented: membership editing, natural and manual
ordering, timezone-aware acquisition ordering, and ID-keyed physical-coordinate
CSV import/export. Computer use verified 513 coordinate-aware results, repeated
and missing coordinates, temperature/time plots and unchanged historical values
after membership revisions. Ten focused measurement tests passed.

**Phase A is not complete.** Remaining gates include named measurement presets,
automatic revision checking, recovery-journal UI, plot-gesture authoring,
independent-error propagation, and broader resource/platform qualification.

## API rule for every milestone

Keep the common operation to one call with scientific arguments and a small
result type. Prepare prerequisites automatically without changing the borrowed
input. Keep project identities, background workers, cache policy and replay
records out of the normal core call. Put advanced options in an explicit
configuration object or builder. Document defaults and units beside the public
signature. The current example is
`spectrum.measure(&Measurement::mean(-20.0..=30.0))?`, with the scalar directly
available as `result.value`.

## Remaining milestones

B: Live completion policies, immutable snapshots, queue/recovery and paused reopen.
C: Peak model/residual/noise contracts, current/series fitting and Live integration.
D: Licensed atomic data, full MBACK and processing integration.
E: Domain-qualified fluorescence correction and ordered preparation.
F: Explicit Cauchy wavelets, maps and full-frame region measurements.
G: Leakage-safe labeled datasets, native PLS1/LASSO and frozen predictions.
H: Qualified profiles, bootstrap, sensitivity and prediction intervals.

Python/TypeScript bindings and recipe replay follow each stabilized core contract;
installed-package and editor-help tests are completion gates. All numerical
reference artifacts need source/version/license records. Unpublished experimental
data and private workflow reports must remain outside this checkout.
