# Live acquisition (experimental)

Live acquisition watches completed files and adds their spectra and a chosen
trend to **Series**. It reads the acquisition folder without changing its files.
Introduced as an experimental workflow in 0.2.10. Physical Windows/Linux
acquisition and network-share behavior remain unqualified; the release does not
claim that the automated checks establish those operating conditions.

The side/window monitor, simultaneous signal selection and optional averages
below are **unreleased source-checkout changes**. Released 0.2.10 selected one
reviewed signal per scan and used the main selection for its Live plot. The
[historical review](validation/2026-09-23-live-workflow/README.md) records those
limitations before these changes.

## Start a session

1. Open **Series → Live…**, choose the acquisition folder, and enter a filename
   filter such as `*.qd`. The filter is case-insensitive and supports `*` and `?`.
   Under **File handling**, turn on **Include subfolders** only when those folders belong to this acquisition.
2. Open **File handling** to adjust the quiet-file check. The default is **three matching observations, one
   second apart**. Size and modification evidence must remain unchanged, and
   the captured bytes must parse successfully. Increase the interval for a writer
   that pauses between chunks. Completion is **inferred**: a long pause can look
   like a finished scan. A later content change is retained as another revision.
3. Choose a saved analysis recipe, **Copy selected spectrum’s settings**, or
   **Reference: …** and a named spectrum. A reference supplies normalization,
   background, and transform settings independently of the current selection.
   Its settings and identity are captured at Preview and frozen at Start.
   Without a recipe, Live measures the mean flattened μ(E) from −20 to +30 eV
   relative to each spectrum's E₀.
   Settings are frozen when previewed; automatic processing values still resolve
   for each spectrum. To change the measurement, save a recipe from **Add trend**.
4. Select **Include existing files** to process files already present. With it off, the
   revisions present during Preview are excluded; new or subsequently changed
   files remain eligible. Preview reports the matching file count.
5. Click **Preview sample** to read a representative file. If it contains several named
   signals, select the desired **Signals** (for example transmission,
   fluorescence and reference), then Preview again. All selected conversions and
   processing paths must succeed; the plot shows the first selected signal.
   The representative file is the first matching filename. Narrow the filter
   when the folder contains different layouts.
6. Choose **Individual scans**, **Running averages**, or **Average every N scans**.
   Individual scans is the default. For sets, choose N from 2 to 100,000 (the
   initial field value is 10). Preview any changed choice before **Start acquisition**.

An ambiguous detector layout is never chosen by column order. Select the named
signals in Live, or import a representative source through the normal measurement
preview, review its mapping, select that spectrum, then return to Live and Preview. Reuse requires matching
column names, units, detector choices and conversion metadata. Unnamed columns
require review outside Live. Each compatible scan within a multi-scan source
becomes a distinct frame. An explicit selection accepts up to eight named signals. All selected channels
from one source commit together; an unreadable channel holds that source for
review. Other compatible sources continue. Unselected channels remain in the
retained original bytes.

## Reuse an EXAFS fit (unreleased)

First fit a representative spectrum in **Fit** and inspect its result. In Live,
choose that recorded fit under **EXAFS fit**, then **Preview sample**. EXAFS fitting
and **XANES peaks** are separate operations. EXAFS uses scattering paths and fits
χ(k); XANES uses empirical peak/step profiles near the absorption edge.

The EXAFS preview copies enabled path files, variables, constraints, k/R ranges
and fit weights from the selected single-spectrum fit. The selected processing
recipe supplies normalization and AUTOBK settings. Missing prerequisites run on
an owned copy. The preview shows weighted χ(k), its fitted curve and R-factor;
a nonconverged preview prevents Start. Path bytes are frozen before starting,
so later edits to the original FEFF workspace do not change the Live model.
Multiple-dataset fits are unavailable here because Live fits outputs independently.

With **Individual scans**, every accepted channel is fitted. With averaging,
Live fits each published average revision. Several files can arrive in one poll;
they contribute to one updated running average, so intermediate unpublished
averages are not fitted. Each fit starts from the saved variables; there is no
implicit warm start. Channel compatibility is the analyst's responsibility.

Choose **EXAFS · k** or **EXAFS · R** in the monitor to compare data and fitted
curves. The k view uses the model's first k-weight and its selected k interval.
The R view shows the corresponding Fourier magnitudes; these uncorrected peaks
are not bond lengths. The compact status shows convergence and R-factor, while
notices and failures remain available under attention details. Expand **Fit
parameters** to inspect fitted values and their local uncertainty estimates;
units follow the saved model. See
[fitting statistics](fitting-statistics.md) for the limits of these statistics.
The source of the calculation is `fitting::run_spectrum_fit`; Live orchestration
and immutable records are in `live/exafs.rs`.

Results are retained by input revision before publication. Resume reuses retained
fits, and reopening a session does not launch new fits until Resume. A running
fit finishes before cancellation is observed; cancellation leaves the unfinished
output eligible. Earlier average fit artifacts remain in the session cache;
reopening the acquisition loads the current output's fit. Embedded projects
retain the fit results attached to that project, including arrays, diagnostics
and input revisions. A failed fit
keeps its spectrum; change the model in a new session to reevaluate its assumptions.

## During acquisition

To fit XANES peaks as scans arrive, first save a model in
[Data → XANES peak fit](xanes-peak-fitting.md#desktop-workflow). Before starting
Live, choose that revision in **XANES peaks** and run **Preview sample**. The preview fits
the representative scan in the model's recorded representation and interval;
nonconvergence prevents starting with that preview. A converged fit with a bound
warning still needs scientific inspection. The scalar recipe remains active too;
its preview value describes that scalar measurement, not the peak-fit objective.

The Live configuration captures a copy of the named model revision. Each arriving
scan uses the same initial values, native mask and processing choices. A fit
failure remains a row and does not remove the imported spectrum. Full fit arrays
are retained on disk before the acquisition ledger publishes the frame. Pausing
during an unfinished source leaves it eligible for processing on Resume.

**Inspect peak fits** opens the retained fit/correlation/trend workspace. Acquisition
can continue while inspecting it. An open peak trend updates as rows arrive.
Peak fitting still evaluates each arriving channel independently; averages do
not receive an automatic additional peak fit. Use the saved peak-fit workspace
to inspect convergence, warnings and uncertainties. **Finish acquisition** retains results; restarting
the application restores the session paused. Recovery reconstructs peak rows from
the committed ledger without refitting or duplicating existing frames. Choose
**New acquisition…** to change the peak model for subsequent work.

**Pause** stops scheduling new work; already committed results remain available.
An in-progress file finishes or observes cancellation between operations; an
uncommitted source remains eligible. Resume is disabled until the worker settles.
A committed source is not imported a second time when resumed. The coordinator
may reread bytes to verify a changed file or reconcile after reopening; reading
again does not add statistical weight.
**Resume** reconciles the folder, including files that arrived while paused.
**Finish acquisition** closes the current run after outstanding work settles, preserving its
results. Resume can continue that same definition; **New acquisition…** starts a
separate session without changing earlier results.

The top-bar **Live** indicator opens a separate compact monitor. The open-window
icon in the Live workspace does the same. In the monitor's **Live options** menu,
choose **Dock in sidebar** to place it beside analysis. Closing or hiding the
monitor hides only its display; acquisition continues while the main analysis
window remains open. Switching stages or applications also leaves it running.
The status indicator reports whether acquisition is watching, paused or finished.

The monitor has one **Pause/Resume** action and separate **Signal** and **View**
selectors. Occasional actions live under **Live options**: acquisition details,
window placement, source visibility, retry, finish, and new acquisition. New
acquisition becomes available after the current worker has paused or finished.
Press Escape to dismiss a selector; arrow keys and Enter choose its options.

Choose a channel and **Latest scan**, **Last 5 scans**, or **Average**. Live owns its own
cursor and never changes the analyst's selected spectrum or selected saved
series. **Follow latest** catches up immediately when enabled or when returning from
another stage. Turn it off and use **Previous/Next** to hold an older frame;
Last 5 scans shows up to five frames ending there. Holding an average retains the displayed revision while new contributions
continue in the background; enable Follow latest to catch up.
The full Live trend contains committed frames from the selected channel only,
with failed calculations retained as gaps. File counts describe source paths,
not scans, channels or content revisions. A previous plot remains visible while
its replacement is calculated; a processing error is shown beside it.

Changed layouts and unreadable inputs appear below the plots for review.
**Retry** rechecks these sources. A revised detector mapping or processing recipe
requires a new session. The Series overview, trend table and CSV export retain
all committed results. Sequence means completion order; it is not an acquisition
timestamp. Labels include a short content revision to distinguish reused names.

## Separate averages for simultaneous channels (unreleased)

Running averages maintains one output for each selected signal. Transmission,
fluorescence and reference therefore produce three outputs. Sets of N scans
keeps one output per signal per set; a partial final set is visible immediately.
A scan in a multi-scan source counts once per set, independently of its number
of selected signals. Membership follows first-accepted order, not filename or
acquisition timestamp. A rewritten source retains its original set and replaces
its earlier contribution; older records remain available as provenance.

Each output is an equal-weight arithmetic mean of the converted, **raw**
absorption signal, before normalization or background removal. Energy is in eV.
Transmission represents dimensionless optical thickness, ln(I0/It); a downstream
reference uses ln(It/Ir). Fluorescence uses the selected detector sum divided by
I0 and retains the resulting relative scale. I0, It and Ir denote the incident,
sample-transmitted and reference-transmitted intensities. Detector units must
support the selected conversion. These interpretations follow
[Newville, Fundamentals of XAFS (2014)](https://doi.org/10.2138/rmg.2014.78.2);
[the author's preprint](https://millenia.cars.aps.anl.gov/xraylarch/downloads/2018Workshop/NewvilleEXAFS_RIMG78_ColorPreprint.pdf)
is openly available. Mapping details are implemented in the
[measurement reader](../crates/rexafs/src/xafs/io/reader/model.rs).

The mean uses the first accepted contributor's energy grid, linearly interpolates
other contributors, and retains only their common energy range. Inputs must be
finite with strictly increasing energy and at least two overlapping grid points.
A one-scan output copies its raw arrays. Channels never share an accumulator.
Only the latest committed revision for each source contributes, so rewrites do
not increase its weight. This is an average of converted absorption, not a ratio
of summed detector counts. No alignment, noise estimate, rejection threshold or
automatic stopping criterion is inferred. Average only repeated measurements of
the same sample state with compatible units and geometry; otherwise retain
individual scans or choose separate sets. These averaging and batching policies
are rexafs-specific choices implemented by
[build_averages](../crates/rexafs-gui/src/live/averages.rs) and
[StreamingAverage](../crates/rexafs-gui/src/params.rs).

The monitor applies the frozen processing recipe to each new average, including
any frozen energy offset exactly once. Automatic processing values resolve on
that averaged spectrum. Displayed outputs retain stable group identities while
new immutable cache files retain earlier mean revisions. Contributor arrays are
streamed one at a time during rebuilding, but calculation work grows with the
number of retained contributors; this is not a constant-time accumulator.

The group list hides contributors in averaged sessions by default. **Show Live
source scans** exposes them, including after reopening an embedded project;
marked and current contributors stay visible. Nothing is deleted. Editing an
average's processing settings in the analysis workspace does not change Live's
frozen recipe. New arrivals preserve those edits and hold the main analysis
view; **Refresh** loads the changed average explicitly. Fit results must be
checked against their retained input revisions.

## Recovery and retained data

The desktop keeps original source bytes, converted two-column spectra, a frozen
configuration and a SQLite journal in its application storage under
`live-sessions`. It synchronizes artifacts before committing a result, then
publishes that result to the UI. Path, content digest and session identity prevent
repeat publication after restart; identical files at different paths remain
separate frames. Only one window may own a session at a time.

Reopen a retained session with **Open paused**. It always starts paused and needs
an explicit **Resume**. Saving an **embedded** `.rxs` project includes the original
source snapshots and converted spectra; a linked project needs its referenced
files. Historical producer paths remain provenance. The local acquisition ledger
and permission to watch its folder do not move with an embedded project.

The file payload limit is 256 MiB. Intake processes one payload at a time and
checks at most 16 pending locators per worker pass; reconciliation retains the
backlog. The GUI refresh timer is 500 ms, so the actual observation spacing can be
longer than requested during a backlog. The main analysis uses its existing bounded spectrum cache; the Live monitor
processes at most five displayed spectra per refresh. Growing HDF5/SWMR datasets and instrument control are
outside this completed-file workflow.

## Reproducible synthetic example

The [synthetic writer](../scripts/generate-live-example.py) creates simple energy
and μ(E) tables without experimental data. It refuses to overwrite files. Choose
an output folder outside the checkout:

```sh
python3 scripts/generate-live-example.py /tmp/rexafs-live-example --count 1
# Preview this folder with *.xdi and Include existing enabled, then Start.
python3 scripts/generate-live-example.py /tmp/rexafs-live-example --start 2 --count 20
```

The writer flushes an incomplete table, pauses, then completes it. Frame 7 has a
larger synthetic peak for checking the trend. This is a software demonstration,
not a calibrated physical spectrum.

For three simulated channels, add `--three-signals` to both writer commands.
This encodes distinct synthetic transmission, fluorescence and reference curves
as detector counts; it is not a calibrated instrument simulation.

Implementation: [readiness and snapshots](../crates/rexafs-gui/src/live_intake.rs),
[coordinator and layout checks](../crates/rexafs-gui/src/live.rs),
[durable storage](../crates/rexafs-gui/src/live/store.rs), and
[Series controls](../crates/rexafs-gui/src/app/shell/live.rs). The completion and
queue choices are rexafs-specific engineering policies. See the
[development record](analysis-b-f-progress.md) for qualification status.

See the [synthetic computer-use record](validation/2026-09-17-live/README.md)
for screenshots and the exact scope of local validation.

## Live fit-parameter plots (unreleased)

Choose **EXAFS · k** or **EXAFS · R**, then choose **Parameter** below the
comparison. Model variables and fitted path distances are available immediately.
**Expression…** adds quantities such as `reff + dr_1` with a display unit and
selected FEFF path. The editor starts from the selected parameter's expression,
unit and path; enter a name, then choose **Add**. **Cancel** closes the editor
without adding a trend and retains the draft for later editing. Unit labels do
not convert calculated values. **Error bars** defaults on and shows ±1 local
standard error
propagated with the full fit covariance. The FEFF geometry is treated as exact;
missing covariance does not become zero uncertainty. See
[Series fitting](series-fitting.md#error-bars) for propagation assumptions.

The horizontal axis is **Fit update** in accepted order for the selected channel.
Individual mode fits incoming scans; Running averages fits each published average;
Batches fits each published batch update. Several simultaneously accepted scans
may produce one average update. These histories therefore need not contain one
point per source file. Repeated averages share scans, so their points are not
independent measurements. Unconverged/failed fits leave gaps. Follow off holds
the curve and the trend at the same retained fit; Follow on catches up.

The side monitor shows the parameter trend by default. Expand **Fit comparison**
to inspect its curve. Changing the parameter or expression reads saved fits
without rerunning the optimizer. Save the project to retain custom expressions.
The immutable fit artifacts retain full covariance;
[live fit trends](../crates/rexafs-gui/src/app/shell/live/fit_trends.rs) caches
verified summaries and shares the covariance calculation with Series.

Right-click a spectrum or parameter plot and choose **Axis range…** to set
independent automatic or numeric bounds. For example, Y minimum can be `0`
while Y maximum follows new frames. See [plot axis ranges](plot-axis-ranges.md).
