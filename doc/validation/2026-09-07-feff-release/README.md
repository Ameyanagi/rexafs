# FEFF release integration — 7 September 2026

Candidate: rexafs 0.1.3, ReFEFF 0.3.0 (component crates 0.2.0), FEFF10 0.2.3.
The native Mac build enables `refeff-runner,feff10-runner`.

The existing `metal_foils_fit_with_both_backends` desktop regression calculates
both engines from identical 8 Å fcc clusters and fits the same measured Cu and Ni
scans with the same preprocessing, first-shell selection, four-variable template
and fit ranges. The complete results are in `metal-foil-comparison.json`.
All 148 desktop tests passed (3 environment-dependent tests ignored) on Apple
Silicon with these dependency versions. Native Intel and Linux checks run in CI.

| Scan | ReFEFF R-factor | FEFF10 R-factor | Generated paths per engine |
|---|---:|---:|---:|
| Cu 150 K | 0.008073988458716588 | 0.008074666820109325 | 62 |
| Ni room temperature | 0.003079418766297067 | 0.003079418766297067 | 65 |

The regression requires convergence, R-factor below 0.02, backend R-factor
difference below 5e-5 and fitted-parameter differences below 5% of the smaller
reported uncertainty. It does not assert universal equivalence for every FEFF
card/material, and these numbers are fit-quality results rather than timings.

Package qualification additionally checks fcc Cu first-shell coordination 12,
two legs, distance 3.615/sqrt(2) Å and finite nonzero amplitudes, including FEFF10
worker re-execution. Graphical and final signed-package evidence is recorded as
qualification completes; local artifacts are not public release inputs.

## Graphical comparison and project round trip

A packaged Apple Silicon candidate was opened with isolated QA settings. In
Fit → Calculate, both engines were available and ReFEFF remained the default.
Each engine generated 62 paths from the same curated 8 Å Cu cluster. Both
sources remained visible, with engine labels, after switching engines. Selecting
one first shell and using **Deselect other sources** reduced the active model
from two paths/eight variables to one path/four variables without deleting paths
or expressions. Both fits converged in six iterations and remained in History.

`gui-comparison.json` records the fitted parameters and input hashes. The GUI
used its startup preprocessing and k = 2–12 Å⁻¹, R = 1–3 Å, k-weight 2: its
R-factors are 0.0024552892 (ReFEFF) and 0.0024557184 (FEFF10). These use a different
protocol from the automated regression above and should not be mixed with that
table. FEFF10 rewrites whitespace and comments in its workspace input; after
normalizing those, both engines' input cards and atom coordinates are identical.

The portable project was saved and reopened through the native file picker:
124 paths, both engine labels, parameter expressions and two history entries
were restored. The live fit is intentionally recomputed after reopening.
Screenshots `01`–`14` cover startup, structure, both engine selections, generated
paths, isolated selections, both fits, saving and reopening. One erroneous file
picker selection during automation produced a recoverable “Open a .rxs project
file” message before the correct project was selected; no project was changed.

Final local checks: 148 desktop tests passed (3 ignored), all five targeted
ReFEFF/FEFF10 integration tests passed, all 16 historical/current compatibility
fixtures passed, 14 Python maintenance/packaging tests passed, actionlint passed,
and cargo-deny licenses passed. The extracted app passed core and both-engine
self-checks, including native worker execution. These are candidate checks;
final signed distribution checks are a separate release gate.
