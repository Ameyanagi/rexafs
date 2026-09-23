# Live acquisition: analyst workflow review

Reviewed on 2026-09-23 through Computer Use in the existing macOS application.
The installed bundle reports version 0.2.13. Source inspection used checkout
`2644b932`, whose Cargo package reports 0.2.12, with the pending Assistant
connected-app and file-import changes. The application was not rebuilt,
replaced or restarted for this review. Runtime observations and source findings
are identified separately below.

The background acquisition checks passed. The current display still needs an
independent monitor selection and visible background status before it can serve
as the compact, unobtrusive monitoring workflow requested by the analyst.
The side monitor described below is a proposal, not an implemented feature.

## Findings

### 1. Follow latest does not catch up after background arrivals

**Observed in the application.** With Follow latest enabled, the analyst stayed
on Data with the saved MCR-ALS component plot open and Live scan 1 selected.
Scans 5 and 6 arrived and were committed. Returning to Live showed **6 completed**
and an enabled Follow latest control, but the spectrum title still identified
`live_test_001.xdi`. The trend already contained six points. The same selection
remained after further idle polling. See the
[captured mismatch](return-shows-old-spectrum.jpg).

In [publication](../../../crates/rexafs-gui/src/app/shell/live.rs),
`publish_live` selects the newest group only when new records are published
while Live is open in the Series stage. `refresh_live_spectrum` reads the global
analyst selection. Returning to the view and enabling Follow latest do not
resolve the latest retained frame. This explains the observed mismatch without
requiring a lost or unprocessed scan.

The monitor should resolve its own latest committed frame immediately when
opened or when Follow latest is enabled. Its display selection must not change
the analyst's selection in Data, Normalize, Background, Transform or Fit.

### 2. Live publication writes the shared Series selection on every poll

**Source finding; a second saved-series scenario was not exercised in the UI.**
`publish_live` assigns `measurements.selected_series` and `selected_run` outside
the `changed` and Follow latest conditions. `poll_live` calls it for successful
empty polls as well. These shared fields also select the series and saved result
in the [measurement workspace](../../../crates/rexafs-gui/src/app/shell/measurements/view.rs).
Consequently, a running watcher can overwrite an analyst's selected saved trend
even without a new file. The separate overview source may still identify the
previous series, so the shared selection and cached display can disagree.

Publishing a record should update its session archive and monitor state only.
The main Series selection should change through an explicit navigation action.
This needs a regression check with two saved series and an idle watcher.

### 3. Background status and controls are hidden during analysis

**Observed in the application.** Data preserved the MCR-ALS view and selected
spectrum while acquisition progressed, but showed no Live scan count, pending
issue count or Pause action. The analyst has to navigate back to Live to inspect
those controls. See [analysis during background acquisition](background-analysis.jpg).

The Live page also places individual source errors after two large plots.
The [invalid-source check](follow-off-with-invalid-source.jpg) showed a compact
issue count near the top and a detailed error at the bottom. A small monitor
should keep the count visible and open details on demand.

## Computer-use checks

| Scenario | Observed result |
| --- | --- |
| New file while Data and MCR-ALS are open | The fifth Live frame committed. The selected first Live spectrum, saved component plot and enabled Resolve components control remained in place. No dialog interrupted the view. |
| New file while another application is active | Finder remained the foreground application while the sixth frame committed to the Live ledger. Returning to rexafs preserved the Data view. |
| Return to Live with Follow latest enabled | Failed the expected catch-up behavior: six completed frames, but the first spectrum was still displayed. |
| Inspect an old frame with Follow latest off | The seventh valid frame committed and the trend updated; the selected first spectrum remained displayed. |
| A malformed file and a valid file arrive together | The valid file completed. The malformed file became one source needing review, without a modal dialog or stopping intake. |
| Remove the deliberate error from the watched folder and retry | The malformed fixture was preserved outside the watched folder. Retry and Resume restored seven completed files and zero files needing review. |
| Stop and inspect retained results | Seven successful frames remained, with no duplicate ledger records. The [Series overview](seven-frame-overview.jpg) contained all seven. The watcher was left [stopped with zero issues](stopped-clean.jpg). |

The existing Data/MCR-ALS view was restored after testing. No fit was started or
overwritten. The original 53 groups and 50 marked synthetic spectra remained;
seven test imports bring the current project to 60 groups. Four imports came
from the preceding walkthrough; this review added three more.

## Compact monitor proposal

Use one dockable side monitor, with an option to detach it when the analyst
needs the main window's parameter panel. Closing or collapsing the monitor
should hide its view while acquisition continues. Pause and Stop remain explicit
actions, and a small status control in the main window should reopen the monitor.
The monitor should never raise its window or steal keyboard focus on arrival.

Keep normal operation to a short header, one spectrum plot and an optional small
trend. The header needs the run state, completed scan count and Pause/Resume.
Show an issue count only when attention is needed. Put configuration, source
details and recovery actions behind a details control.

The display selector can offer Latest, recent spectra overlaid, and a running
average when that feature is implemented. Selecting an older monitor frame
should suspend following within the monitor and show an explicit return-to-latest
action. New arrivals should preserve the analyst's main plot range, selection,
fit model, marked groups and unsaved parameter edits.

Reuse a reviewed processing recipe and an optional saved fit model. Existing
Live supports a scalar measurement and a saved XANES peak model; this review did
not manually qualify automatic EXAFS, linear-combination fitting, live overlays
or running averages. These should not be represented as currently available
Live functions. An average would need explicit membership for repeated scans
under the same conditions, one updated derived result, and retained originals.
The changing Cu-mixture series used here is an arrival test, not a qualification
dataset for averaging or a signal-to-noise stopping rule.

The Assistant should configure a concrete workflow and report exceptions through
dedicated session controls. The current
[Assistant tool catalogue](../../../crates/rexafs-gui/src/codex_client.rs) has no
Live session configuration, status, pause or resume tools. File import alone
does not provide an unattended monitor. Routine file readiness, processing and
fitting should continue in the existing background engine, without one model
turn or chat message per scan.

Before qualifying this proposal, repeat the computer-use checks with the monitor
docked and detached, with a narrow main window, and while editing another fit.
Check that returning to Latest catches up immediately, older selections remain
stable when following is off, closing the monitor does not stop intake, and
errors are visible without interrupting typing. Check a second saved series to
cover the shared-selection finding above.

## Automated checks and evidence

Both targeted suites passed in this checkout:

```sh
cargo test --locked -p rexafs-gui live::tests -- --test-threads=1
# 12 passed
cargo test --locked -p rexafs-gui live_intake::tests -- --test-threads=1
# 10 passed
```

These cover durable publication and recovery, revisions, cancellation, partial
writes, layout review, saved XANES peak results and processing preparation. They
do not test the GUI selection mismatch, a detached monitor, operating-system
sleep, real instrument control or network-share acquisition. A narrow-window
review was not completed; the proposal has not been visually implemented.

The [evidence manifest](evidence.json) records the application version, group
counts and SHA-256 digests of the unedited Computer Use captures. Accessibility
transcripts are retained locally under the ignored directory
`target/live-ux-review-20260923/`.

The watched folder contains byte-for-byte copies of synthetic Cu-mixture frames
1, 25, 50, 38, 8, 14 and 20 from the analyst's already loaded project. The original
XDI headers and attribution were retained. The local test manifest records
source and destination paths and their SHA-256 digests; every original and copy
was checked against those digests after the review. The malformed source was
created specifically for this test and is labeled as non-measurement data.
The preceding four-file test record remains unchanged.

See the [Live acquisition guide](../../live-acquisition.md) for the existing
completion policy, recipe setup and retained-data behavior.
