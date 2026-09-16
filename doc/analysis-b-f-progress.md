# Analysis milestones B–F (development)

Work proceeds on `feature/analysis-b-f`, based on the Phase A and Series/import
work at `8958681`. The [complete design](complete-analysis-design.md) remains
the scientific and workflow contract. No milestone below is released or complete
merely because its first module exists.

1. **B — Live acquisition:** integrate the earlier readiness/snapshot commits
   into a bounded folder coordinator, frozen processing, durable publication,
   paused recovery, and Series controls. Test synthetic incremental writers,
   changed files, multi-scan sources, retries and cancellation through computer use.
2. **C — XANES peak fitting:** native peak/step/baseline models, constrained fits,
   retained diagnostics and parameter trends; current, series and Live workflows.
3. **D — MBACK:** verify the atomic-data provider and licensing, implement the
   specified full objective and optional erfc term, then integrate normalization.
4. **E — Fluorescence correction:** implement the declared thick-sample XANES
   model, geometry/composition checks, separate internal/final normalization and
   preservation of the uncorrected branch.
5. **F — Wavelets:** explicit Cauchy kernel/order/grid, complex maps, linked views,
   comparisons, region trends and numerical export with bounded map memory.

Each numerical operation needs a simple Rust API, Python/TypeScript exposure,
documented units/defaults/assumptions, retained project results, and appropriate
runtime/editor checks. Reference implementations inform qualification; they are
not runtime dependencies. GUI checks use a separate application and generated
or already attributed public fixtures. Private experimental inputs are not copied
into this repository or public screenshots.

## Initial audit

The earlier `feature/live-acquisition` branch contained a tested readiness tracker
and immutable snapshots, but no running Live workspace. Its two commits were
preserved by cherry-picking them into this branch. Work on their original branch
was left unchanged. The remaining integration is still in progress.

## B: integrated development increment

The Live workspace now has a folder/filter preview, configurable quiet-file
observations (default three at one-second intervals), explicit existing-file
inclusion, frozen processing/metric recipes, bounded per-pass processing, and
SQLite publication records. Original bytes and converted spectra are retained;
embedded projects include both. Multi-scan sources retain separate frames, and
changed layouts require review. Recovery starts paused.

Computer use on an isolated macOS application and synthetic writer confirmed
partial-write intake, Pause with three arrivals held back, Resume from nine to
12 spectra, Follow latest off retaining frame 1 while frames 13 and 14 arrived,
and a visible trend spike at synthetic frame 7. GUI save/reopen identified cache artifacts being treated as extra catalog inputs;
these now have a separate project-file classification. Recovery counters and
retained-trend reconstruction are included in the final computer-use checks. No experimental input was used.

The focused run `cargo test --locked -p rexafs-gui --bin rexafs live` passed
26 selected tests. This includes existing tests whose names contain `live`, not
26 newly authored acquisition tests. New checks include exact source retention,
embedded relocation, distinct paths/revisions, failure rows, multi-scan sources,
reviewed mappings, cancellation, bounded batches and restart deduplication.

Native Windows/Linux and real network-share qualification remain outstanding.
C–F have not been implemented by this increment.

The full GUI test run passed **572 tests**, with **6 ignored**, before the final
QD regression and project-artifact classification changes. Subsequent focused
Live tests passed all 26 selected tests, and project compatibility tests passed
31 tests with one ignored. The new QD test reuses the already-attributed KEK
academic/nonmilitary fixture and checks angle-derived energy, absorption arrays
and original source retention. Release builds succeeded on this macOS host.

The [final computer-use record](validation/2026-09-17-live/README.md) confirms
embedded-project membership, paused recovery, retained trend reconstruction and
editable quiet-file timing. It includes only synthetic screenshots.

## C1: native peak-fit core

The unreleased [`PeakFit` API](xanes-peak-fitting.md) now provides Gaussian,
Lorentzian, pseudo-Voigt and true-Voigt peaks, error-function/arctangent steps,
and constant/linear baselines. Joint fitting supports fixed values, bounds,
restricted expression ties, native-point masks, selected-space errors, baseline
initialization, cancellation and independent batch fits. Result objects retain
initial/final definitions, component arrays, residuals and conditional covariance.

The full default-backend core unit suite passes all 208 tests. Ten focused peak
unit tests also pass on the ndarray compatibility backend, and two peak
integration tests pass on the default backend.
The integration tests compare all four peak shapes against pinned lmfit 1.3.4
references and check automatic preparation without mutating the source. A bound
regression test caught failure to refit other parameters after clamping; the new
projected optimizer passes the analytical constrained solution. Fixtures and
their integration test are excluded from the crate archive.

GUI parameter editing, retained current/series diagnostics, Live peak recipes,
Python/TypeScript bindings and full milestone-C qualification remain in progress.
No GUI peak-fitting completion is claimed by this core increment.
