# Series fits, Live fit trends and plot ranges

This is an unreleased working-tree qualification on macOS 26.5.1 (25F80),
Apple Silicon, using Rust 1.98.1 and ruviz/ruviz-gpui 0.14.2. Cargo identifies the
checkout as 0.2.12; this is a separately named review application, not an official
release. The installed application and its original Cu project were preserved.

The later [pull-request integration check](pr-integration.md) records validation
on the current 0.2.13 development base separately from these desktop captures.

## Data and calculation

The retained fixture is
[`cufoil_abs.xdi`](../../../crates/rexafs/tests/fixtures/analysis/cu-mixtures/standards/cufoil_abs.xdi),
with 517 points and SHA-256
`174439b52d7c8c59c28487b0c5577ead6be63db15243b8c7b03d0f2cb4bb4d7d`.
The replay files contain identical bytes. They test intake, averaging, fitting,
plotting and recovery; they are not independent measurements or evidence of an
improvement in signal-to-noise ratio.

The saved single-spectrum model used one Cu–Cu FEFF path, k = 2–12 Å⁻¹,
R = 1–3 Å, k weight 2, and four varying parameters: s02, e0, dr_1 and ss_1.
The R-space fit gave R-factor approximately 0.00612. The custom expression
`reff + dr_1` gave 2.54160 ± 0.00438 Å, using the selected path's fixed geometry
and the fitted covariance. These are model-dependent fit estimates. Convergence
and replay agreement do not establish physical accuracy. See
[Series fitting](../../series-fitting.md) for definitions and uncertainty assumptions.

## Ten-minute replay

[feed-events.jsonl](feed-events.jsonl) retains all publication times.
[feed-results.json](feed-results.json) records the final read-only ledger audit.
The feeder ran for 600.007 seconds, adding files 003–062 at ten-second intervals,
following two existing files. Its first and last publications were
2026-09-22 23:37:10.571779 UTC and 23:47:00.559652 UTC.

The session accepted 62 distinct source paths and retained 62 frames. It made
61 updates to a single running average and 61 converged EXAFS fits with no fit
errors. The first two existing files were accepted in one update. File 017 was
written in two parts separated by 1.5 seconds; the three-observation stability
check accepted its completed bytes once. The other new files were published
by atomic rename. Pausing and resuming without adding files left the source,
frame and fit counts unchanged.

The local replay directory is `/tmp/rexafs-live-workflow-20260923/Cu_foil_live_test`.
The session ledger and immutable fit artifacts are retained under
`/tmp/rexafs-series-fit-20260923/live-sessions/850d917562a90be12be4efe4c578ca29d5fd0d11328e93882e19190d4f488f22`.
These local files are not included in the repository.

## Desktop review

Computer use exercised the Series Fit tab, saved model selection, data/fit
curves, parameter selection, custom distance expression, and error-bar toggle.
The retained images [series-fit.jpg](series-fit.jpg) and
[series-expression.jpg](series-expression.jpg) show the two-frame Series run.
[live-fit-trends.jpg](live-fit-trends.jpg) shows the 61-update Live distance trend.
The new compact monitor keeps the trend visible and collapses the optional fit
comparison. Screenshots are unedited application captures.

The processing-reference picker was checked with the Cu foil chosen explicitly,
then a different synthetic spectrum selected in the group list. The picker
retained Cu and Preview successfully processed the Cu replay. Internal cached
average files no longer crowd the reference list.

Saving a resumed average exposed two identical embedded inputs sharing one
original source locator. Saving now writes that locator once when hashes,
lengths and kinds agree, and rejects conflicting revisions before replacing a
project. The failed review save was preserved. A separate recovery copy removed
only the duplicate manifest row after verifying its payload length and SHA-256;
it reopened with all retained fit trends. This recovery did not change spectra,
fit results or the original project. A regression test exercises both identical
copies and conflicting revisions, including preservation of a previous save.

The axis editor uses independent numeric or automatic endpoints. Computer use
verified setting Y minimum to zero on the Cu Fourier-transform plot. Automatic
endpoints are resolved from the original display data, so a narrow fit trend can
be extended to zero without the plotting library's interactive zoom limit.
See [axis ranges](../../plot-axis-ranges.md).

The final desktop check added files 063 and 064 after the ten-minute replay;
their publication records are [axis-update.json](axis-update.json) and
[axis-update-064.json](axis-update-064.json). File 064 arrived while the axis
editor was open. Applying X minimum = 1 and Y minimum = 0, with both maxima
automatic, successfully updated the replacement Live trend. Reopening the
editor retained those values. All Auto followed by Enter restored the natural
range around the parameter uncertainties. The captures
[axis-editor.jpg](axis-editor.jpg) and [live-axis-zero.jpg](live-axis-zero.jpg)
show the mixed manual/automatic limits. The earlier invalid-range check
rejected X minimum = 70 and maximum = 1 without changing the plot.

Acquisition was then paused at 64 accepted source files and 63 converged fits,
with no fit errors. The application saved a new portable project at
`/tmp/rexafs-series-fit-20260923/Cu-live-final.rxs` using the corrected save path.
[final-project-audit.json](final-project-audit.json) records its checksum,
282 distinct manifest locators, 212 embedded payloads verified against their
lengths and SHA-256 values, 64 Live frames, 63 Live fits, and the separate
two-frame Series fit run. Both retain the custom `reff + dr_1` expression.
The application was quit and relaunched with that exact file. Computer use
confirmed the restored data/fit curves and custom distance trend with error
bars in the [main view](reopened-live-fit.jpg) and
[compact monitor](reopened-compact-monitor.jpg). The review application was
left open with acquisition paused. The original ten-minute replay record
remains unchanged.

### Follow-up interface review

A subsequent release build in
`/tmp/rexafs-ui-review-20260923/rexafs UI Review.app` used the same saved Cu
project and the original review acquisition ledger, without resuming intake.
The compact expression editor now labels Name, Unit and Expression, gives the
expression its own full-width row, and includes Cancel. Its first draft uses
the selected parameter's expression, unit and path. Computer use entered a
draft name, cancelled and reopened the editor, confirming that the draft was
retained without adding a parameter; see
[compact-expression-editor.jpg](compact-expression-editor.jpg).

The axis wrapper now applies stored limits before the first render of a
replacement plot and removes expired view references as Live plots are rebuilt.
Computer use checked the two-frame Series fit: Y minimum = 0 survived advancing
to the second frame and switching error bars off; a different parameter retained
its own automatic range. See
[series-range-after-refresh.jpg](series-range-after-refresh.jpg).
Setting zero in the compact monitor also updated the main Live trend; see
[shared-live-range.jpg](shared-live-range.jpg). Reset axes to Auto restored the
uncertainty range. The final monitor displays Cu–Cu distance with error bars,
with acquisition still paused. These checks changed display state only; the
saved project and ten-minute replay evidence were not overwritten.

The follow-up release build passed with the existing 13 warnings. The targeted
Live trend test and all three axis-range tests passed, as did formatting and
whitespace checks. Their logs are
[compact-editor-tests.log](compact-editor-tests.log),
[axis-polish-tests.log](axis-polish-tests.log), and
[ui-polish-release.log](ui-polish-release.log).

## Automated checks

- Broad GUI regression: 638 passed, 7 ignored, 30 filtered, 118.28 seconds.
  Command: `cargo test --locked -p rexafs-gui -- --test-threads=4 --skip feff --skip rmc`.
  The command intentionally excludes FEFF/RMC-named tests; it is not the full suite.
- Project round trips after the duplicate-locator fix: 30 passed, 1 ignored.
- Axis-range tests: 3 passed, including changing observable data, independent
  Auto endpoints, exact zero for a narrow trend, invalid ranges and log domains.
- Linked wavelet axes: 3 passed.
- Earlier targeted Series fit tests: 2 passed; Live tests: 18 passed;
  Live fit-trend cursor/cache/gap test: 1 passed.
- Release builds, formatting and whitespace checks passed. Clippy completed
  with existing warnings; this is not a warning-free qualification.

Logs are retained beside this record with trailing whitespace normalized;
original terminal captures remain in the local review directories. Numerical
results and test outcomes are unchanged. The desktop checks cover macOS only.
