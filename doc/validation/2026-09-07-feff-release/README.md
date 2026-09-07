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
