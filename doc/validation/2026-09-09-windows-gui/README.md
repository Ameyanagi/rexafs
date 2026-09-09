# Windows GUI interaction check — 2026-09-09

Checked the local Windows MSVC debug build with the Computer Use `@oai/sky`
API. The build included ReFEFF and excluded FEFF10, matching the Windows
release's engine selection. This is a functional interaction check, not an
optimized release performance qualification.

## Observed in the GUI

- The application opened as a single rexafs window, without a terminal window.
  The repeat launch used normal visibility, without console-hiding flags.
- Help → Open Cu example opened the import review. Selecting the absorption
  column, choosing eV, confirming units, and validating imported all 618 points.
- Data, Normalize, Background, and Transform displayed their expected plots.
- Mouse-wheel zoom changed the plot bounds; dragging panned the spectrum.
- Dragging the background cutoff changed Rbkg and recalculated both plots.
- The first run exposed a Windows keyboard defect: Ctrl+Z did nothing. The
  bindings used `cmd`, which GPUI interprets as the Windows key on Windows.
- After changing command shortcuts to GPUI's native `secondary` modifier,
  Ctrl+3 and Ctrl+4 switched stages. A cutoff drag changed Rbkg from 1.2 to
  1.5 Å, and Ctrl+Z restored 1.2 Å and the previous plots.
- Alt+F4 closed the temporary session successfully.

The repeat check opened a temporary copy of the retained 0.1.0 embedded project
fixture. No test project was saved over a repository fixture or user project.

## Automated checks and limits

The Windows build and all eight shortcut tests passed. Earlier checks passed
the seven accessibility diff tests, fifteen installer/release helper tests,
PE GUI-subsystem verification, and redirected executable diagnostics.

The computer-use launch helper failed to register its window-opened handler,
so the development process was launched through the shell; GUI clicks, keys,
drags, scrolling, and screenshots used Computer Use. Native accessibility text
was unavailable from the helper, so interaction used observed screenshots.

The accessibility diff microbenchmark measures only that update path. Neither
it nor these debug-build screenshots establish end-to-end release frame rates.

## Follow-up: E0 and structure lag

The application originally opened for interactive use was `target/debug/rexafs.exe`,
with the workspace's unoptimized development profile. A Windows release build
with `--no-default-features --features refeff-runner` now builds and passes the
packaged numerical self-check. `REXAFS_DEBUG_STATS=1` additionally logs GPUI's
selected adapter at startup. This session reports **Microsoft Basic Render Driver**
with **is_software_emulated: true**. Windows exposes QXL and Remote Display
adapters; this measurement does not represent a Windows machine with GPU
acceleration.

With compilation complete, the 618-point copper example was processed 60 times,
varying E0 by 0.1 eV around 8977.5 eV. The first ten runs were discarded. The
temporary standalone probe extracted the current GUI `process_arrays` function,
parameter defaults and background-weight helper, and linked each build's actual
core dependencies. Timing excludes file parsing, executor scheduling, debounce,
plot updates and display. It is a numerical probe, not end-to-end input latency.
Median full pipeline time was **14.93 ms debug / 0.453 ms release**; AUTOBK
accounted for **11.23 ms / 0.253 ms**. Normal parameter edits still wait 200 ms
before processing, and plot-handle drags use a 50 ms recomputation interval.

Computer Use then rotated the same curated Cu crystal in separate debug and
release windows: 177 cluster atoms, 804 displayed bonds, 5×5×5 cells, faded
surrounding atoms, light theme, 100% zoom and depth cue enabled. Two pairs of
opposite 150×55-pixel drags produced four input-bearing frames in each build.
Only frames with a recorded structure input event enter the comparison:

| Median time | Debug | Release |
| --- | ---: | ---: |
| Structure canvas CPU painting | 208.64 ms | 30.40 ms |
| Input handler to canvas completion | 268.55 ms | 63.17 ms |
| GPUI CPU frame, including submission/cleanup | 568.08 ms | 288.04 ms |

This small diagnostic sample shows that release removes substantial overhead but
does not make this software-rendered structure smooth. Frame times include CPU
work and submission, not physical display presentation or remote transport.
The numerical pipeline, structure renderer and scheduling behavior were unchanged
between builds. The release binary adds only the opt-in startup adapter log.

The release GUI also recalculated after dragging E0 from 8979 to 8999.1 eV;
Ctrl+Z restored 8979. These actions used a temporary project copy. The original
user session was retained. Raw timings and source/binary hashes are in
[`performance/timings.json`](performance/timings.json), with the CSV samples and
rotation logs beside it.
