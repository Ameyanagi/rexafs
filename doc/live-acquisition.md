# Live acquisition (unreleased)

Live acquisition watches completed files and adds their spectra and a chosen
trend to **Series**. It reads the acquisition folder without changing its files.
This is a development workflow on `feature/analysis-b-f`; native Windows/Linux
and network-share qualification are still required before release.

## Start a session

1. Open **Series → Live…**, choose the acquisition folder, and enter a filename
   filter such as `*.qd`. The filter is case-insensitive and supports `*` and `?`.
   Turn on **Subfolders** only when those folders belong to this acquisition.
2. Set the quiet-file check. The default is **three matching observations, one
   second apart**. Size and modification evidence must remain unchanged, and
   the captured bytes must parse successfully. Increase the interval for a writer
   that pauses between chunks. Completion is **inferred**: a long pause can look
   like a finished scan. A later content change is retained as another revision.
3. Choose a saved analysis recipe, or use **Current processing**, which measures
   the mean flattened μ(E) from −20 to +30 eV relative to each spectrum's E₀.
   Settings are frozen when previewed; automatic processing values still resolve
   for each spectrum. To change the measurement, save a recipe from **Add trend**.
4. Select **Include existing** to process files already present. With it off, the
   revisions present during Preview are excluded; new or subsequently changed
   files remain eligible. Preview reports the matching file count.
5. Click **Preview**, inspect the representative spectrum, then **Start**. The
   representative file is the first matching filename. Narrow the filter when
   a folder contains different measurement layouts.

An ambiguous detector layout is never chosen by column order. Import a
representative source through the normal measurement preview, choose its signal,
select that spectrum, then return to Live and Preview. Reuse requires matching
column names, units, detector choices and conversion metadata. Unnamed columns
require review outside Live. Each compatible scan within a multi-scan source
becomes a distinct frame. This increment selects one reviewed signal per scan;
extra detector channels remain available in the retained original source.

## During acquisition

**Pause** stops scheduling new work; already committed results remain available.
An in-progress file finishes or observes cancellation at a scan boundary.
**Resume** reconciles the folder, including files that arrived while paused.
**Stop** closes the current run after outstanding work settles, preserving its
results. Resume can continue that same definition; **New recipe…** starts a
separate session without changing earlier results.

**Follow latest** selects each newly published spectrum. Turn it off to inspect
an earlier frame while intake continues. The top plot follows the selected
spectrum; the trend includes every committed frame, with failed calculations
retained as gaps and accompanying reasons. File counts describe source paths,
not the number of scans or content revisions.

Changed layouts and unreadable inputs appear below the plots for review.
**Retry** rechecks these sources. A revised detector mapping or processing recipe
requires a new session. The Series overview, trend table and CSV export retain
all committed results. Sequence means completion order; it is not an acquisition
timestamp. Labels include a short content revision to distinguish reused names.

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
longer than requested during a backlog. Display processing uses the existing
bounded spectrum cache. Growing HDF5/SWMR datasets and instrument control are
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

Implementation: [readiness and snapshots](../crates/rexafs-gui/src/live_intake.rs),
[coordinator and layout checks](../crates/rexafs-gui/src/live.rs),
[durable storage](../crates/rexafs-gui/src/live/store.rs), and
[Series controls](../crates/rexafs-gui/src/app/shell/live.rs). The completion and
queue choices are rexafs-specific engineering policies. See the
[development record](analysis-b-f-progress.md) for qualification status.

See the [synthetic computer-use record](validation/2026-09-17-live/README.md)
for screenshots and the exact scope of local validation.
