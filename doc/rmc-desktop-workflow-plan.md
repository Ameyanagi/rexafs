# RMC in the REXAFS Fitting workspace

Planning record, 18 September 2026. The initial plan below is preserved.
Implementation now proceeds on `feature/rmc-desktop-workflow`; see the
[implemented workflow and current scope](rmc-desktop-workflow.md). The reviewed source is `dev` at
`01e64d0a2c5d7c120193e52c50c2d204036a4912`, which includes PR #87.
`git pull --ff-only origin dev` reported that the checkout was up to date.
Planning branch: `plan/rmc-desktop-workflow`.

## Agreed interaction

Place a **Fit mode** selector at the **top right of the Fitting window**. Keep it
visible on every Fitting subpage, next to the existing controls and action
button. RMC belongs inside Fitting. Wavelet visualization and configuration
belong inside Transform.

| Fit mode | What changes | Existing engine |
| --- | --- | --- |
| Path fitting | Refines parameters of selected scattering paths; remains the default for existing projects. | `FeffFit` |
| RMC | Refines explicit atomic coordinates subject to constraints. | `RmcSession` |
| Hybrid EA–RMC | Searches a population of configurations, using local RMC moves within evolutionary generations. | `EvolutionSession` with positive `local_steps` |

“Fit mode” selects the refinement method. “Fit space” selects the residual
domain. “Single spectrum / joint datasets” selects the data scope. Keep these
three controls separate. The existing **R real + imaginary** objective remains
the initial RMC choice; a magnitude plot does not change the objective.

The intended path after selecting RMC is:

```text
Fitting                                      Fit mode: [RMC ▾]

Structure → Calculate → Model → Results

Pick structure(s)
  → build and inspect the explicit supercell
  → select absorbers and movable/fixed atoms
  → set constraints, calibration and fit ranges
  → prepare ReFEFF and inspect the initial calculation
  → run, pause, resume or stop
  → compare experimental / initial / best curves and structures
```

The arrows describe guidance, not a locked wizard. Reuse the existing reversible
navigation. First selection of RMC opens Structure. Later mode switches restore
that mode's previous page, draft settings and results. Selecting a mode alone
must not start scattering or refinement. Keep an active job attached to its
original mode and input snapshot while the user browses other modes; permit one
refinement job at a time initially and keep its progress and stop controls visible.

### Structure and calculation setup

Reuse the existing curated library, structure search/import and 3D viewer. Start
with one structure and one processed spectrum. Provide supercell repeat counts,
cell dimensions, atom count and absorber count before allocating a large job.
Use `Configuration::from_structure` for explicit periodic cells and `from_xyz`
for finite clusters. Preserve their distinct boundary conditions. Do not derive
the periodic RMC configuration from a display-only scattering cluster.

Allow selection of absorber element/sites, edge, movable atoms and fixed anchors.
Expose element-pair distances, displacement bounds, move size in Å and numerical
Metropolis tolerance. Show the actual configured values. Reject unresolved mixed
or partial occupancy with an actionable explanation, as the current core does.
Use `suggested_supercell_repeats` and `estimate_catalogue_resources` for advice;
their estimates do not establish either scientific adequacy or a runtime promise.

Calculate prepares **exact cached ReFEFF**, displays setup cost and calculates
the initial spectrum. Preserve the current 256 MiB cache default. The prepared
path catalogue, displacement envelope, path limits and reference electronic
potentials belong to the saved job. A changed path is recalculated; reference
potentials remain fixed, as documented in the current engine.

The existing Paths page remains part of Path fitting. For RMC, expose catalogue
inspection as a calculation diagnostic rather than requiring manually checked
FEFF path rows before Run becomes available. The current path-selection blocker
must dispatch according to fit mode.

Multiple weighted structures follow the first working flow: an **Add structure**
action supplies independent configurations and normalized mixture fractions.
Population members in EA are candidate solutions, not mixture components; never
average the population to calculate the fitted spectrum.

### Model and inputs

Build each job from the active processed `Spectrum` (`XASSpectrum`) using
`RmcDataset::from_spectrum` and `RmcSpectrumOptions`. Preserve the captured
preprocessing state, selected data indices, source identity and processing
recipe. The core snapshot is a state record, not a complete operation history;
desktop provenance must retain the corresponding group identity and recipe too.

Resolve the existing “follow Transform” setting once when preparing a job, then
save that resolved configuration. Processing edits produce a new input revision;
they must not alter a running job. Display stale results as belonging to the
original input, and offer a new run with the changed settings.

Use the existing native R transform for real-plus-imaginary residuals and retain
the experimental-power normalization used in the Cu₂O work. Enforce the saved
Rbkg lower bound and show the excluded low-R region. Do not silently increase
the user's selected range. Separate structural penalty terms from spectral
residuals in displays and reports.

Show S₀² and fitting ΔE₀ explicitly. Offer an explicit action to copy compatible
calibrated values from a selected path-fit result, recording their provenance.
Do not treat a path-fit ΔR or σ² as an automatic coordinate displacement or
silently optimize calibration parameters during coordinate-only RMC.

The current Spectrum adapter accepts **R space and one integer k weight from
0 through 3**. The existing path fitter also supports k/q objectives, multiple
k weights and noise estimation. The first RMC desktop slice must present this
capability difference clearly and reject incompatible settings before setup;
it must not discard extra weights or claim those features are connected.
Broader objectives require an explicit adapter extension, retaining provenance,
normalization and validation. Keep the present path fitter's behavior unchanged.

## Wavelet belongs to Transform

Add a Wavelet view to the existing Transform views (`k`, `R`, `k + R`, `q`).
Expose Morlet settings, k/R grids, mask selection and magnitude/real/imaginary
display. Gaussian STFT can be an advanced transform choice with its own label.
Wavelet maps should work on any prepared spectrum, independently of RMC.

Move the reusable local-spectrum mathematics out of the RMC namespace into a
shared transform module, provisionally `xafs::transform::local_spectrum`, with
an ergonomic `rexafs::transform` export. This is an additive organization change;
reuse existing Fourier implementations instead of rewriting `xrayfft` or the
tested fitting transform. RMC becomes a consumer of this shared code.

Currently `LocalSpectrumTransform` prepares direct or zero-padded FFT kernels,
returns complex coefficients and lives in `rmc/local_spectrum.rs`. It applies
neither k weighting nor noise scaling. Its mask affects scoring, not the returned
coefficients. Preserve these conventions and avoid applying k weights twice.
Introduce transform-specific errors; retain compatibility wrappers for existing
`rmc` APIs where an alias would change an error signature. Preserve old serialized
objective variants and checkpoint meaning during the extraction.

Keep transform calculation/settings separate from the objective adapter. A
wavelet objective compares complex residual coefficients with an explicit mask
and normalization. A magnitude heatmap is a display choice. Fitting can consume
a saved transform configuration, but changing the displayed map or color scale
must not silently change an active fit.

The Spectrum adapter currently does not impose a wavelet Rbkg policy. Before
enabling a Wavelet fit-space choice, specify and test its low-R mask rule and
how window leakage is communicated. Do not pretend that masking low-R cells
guarantees removal of background effects. Keep wavelet fitting unavailable in a
mode until its complete objective adapter and statistics are implemented.

## Code to reuse and concrete gaps

| Area | Current source | Planned integration |
| --- | --- | --- |
| Fit header and navigation | [fit_workspace.rs](../crates/rexafs-gui/src/app/shell/fit_workspace.rs) | Persistent top-right selector; mode-specific prerequisites, navigation and Run action. |
| Fit inputs and results | [fitting.rs](../crates/rexafs-gui/src/fitting.rs), [fit.rs](../crates/rexafs-gui/src/app/shell/fit.rs) | Reuse ranges and plots; preserve method-specific configuration and result types. |
| Worker lifecycle and stale results | [app.rs](../crates/rexafs-gui/src/app.rs) | Extend generation/input guards and job registration; place new orchestration in a dedicated module. |
| Processed input | [spectrum.rs](../crates/rexafs/src/xafs/rmc/spectrum.rs) | Use the existing snapshot constructor; add broader objective support only with validation. |
| Structures | [structure_view.rs](../crates/rexafs-gui/src/app/shell/structure_view.rs), [geometry.rs](../crates/rexafs/src/xafs/rmc/geometry.rs), [workflows.rs](../crates/rexafs/src/xafs/rmc/workflows.rs) | Reuse source structures, supercell conversion and resource estimates; add explicit configuration display. |
| ReFEFF and cache | [accelerated.rs](../crates/rexafs/src/xafs/rmc/accelerated.rs) | One prepared calculator per live job; reuse its cache, cancellation token and counters. |
| Refinement and recovery | [session.rs](../crates/rexafs/src/xafs/rmc/session.rs), [evolution.rs](../crates/rexafs/src/xafs/rmc/evolution.rs) | Drive incremental steps/generations; retain checkpoints and best state. |
| Convergence | [convergence.rs](../crates/rexafs/src/xafs/rmc/convergence.rs) | Display existing residual diagnostics and explicit termination reason. |
| Transform UI and local maps | [center.rs](../crates/rexafs-gui/src/app/shell/center.rs), [local_spectrum.rs](../crates/rexafs/src/xafs/rmc/local_spectrum.rs) | Transform owns wavelet controls; shared kernels serve plotting and objectives. |
| Projects and export | [project.rs](../crates/rexafs-gui/src/project.rs), [storage.rs](../crates/rexafs-gui/src/project/storage.rs), [publish.rs](../crates/rexafs-gui/src/app/shell/publish.rs) | Persist run provenance and resumable artifacts; export method-appropriate reports. |

Use a small desktop `FitMode` and separate saved drafts for path/RMC/hybrid
settings. Add a result enum or dedicated RMC result record instead of squeezing
RMC into `FeffFitResult`. Shared plotting consumes common spectrum/curve data;
parameter covariance, reduced χ² and parameter correlations remain tied to the
methods that actually calculate them. An RMC residual plateau is not a parameter
uncertainty estimate.

## Run lifecycle and performance

Prepare the Spectrum snapshot, transforms and calculator once in a background
worker. Drive `RmcSession::step`, or `EvolutionSession::step` for hybrid search.
Reusing `run_spectrum_fit` would run the wrong optimizer; reuse its desktop
lifecycle pattern, not its path-model solver invocation.

Publish bounded progress snapshots at a proposed 2–4 Hz, with heavier spectrum
and structure refreshes less frequently or when the best state changes. These
are design targets to benchmark, not measured performance. Avoid processing the
Spectrum, rebuilding kernels, cloning full histories or enumerating path reports
on each repaint or attempted move.

Separate the run state (preparing/running/pausing/paused/stopped/completed/failed)
from residual trend status. Pause at a committed attempt or generation boundary
and retain the warm calculator. Stop preserves the best state and a checkpoint.
Use the existing calculator cancellation token for interruption; cancellation is
cooperative and a single typed path kernel is not interruptible. After cancelling
the token, recreate an identical calculator before checkpoint resume. Checkpoint
creation and disk writes must be throttled and kept off the UI thread.

Report initialization time, attempts or local moves, generations for EA,
acceptance and constraint-rejection rates, elapsed time, proposal time per step,
inclusive time per step and cache-counter deltas. Identify each reuse ratio's
denominator. Setup and cold-resume cost must not disappear from reported timings.
Keep a bounded residual history large enough for the selected convergence rule.

Exact caching stays the default for both RMC modes. Adaptive scattering remains
an explicitly enabled **Experimental** advanced option with audits, fallback
events and exact final verification visible. It is not a fourth fit mode and is
not required for the initial desktop workflow. Existing Cu₂O measurements did
not establish a sustained adaptive speed benefit.

## Results, convergence and persistence

Reuse the existing Results page and provide the following result views:

| View | Contents |
| --- | --- |
| Summary | Fit mode, input spectrum/structure, objective and ranges, initial/best residual, fractional improvement, per-dataset residuals, structural penalties, termination reason and residual-trend status. |
| Fit plots | Experimental, initial and best-fit curves in k and R; residuals; R magnitude, real and imaginary views. |
| Structure | Initial/best coordinates and supercell, displacement and distance distributions, movable/fixed atom display and export. |
| Convergence | Current/best score history, recent-window changes, acceptance and constraint rejection, and EA population diversity when applicable. |
| Performance | Setup and elapsed time, completed attempts/generations, proposal and inclusive time per step, and clearly defined cache reuse counters. |
| History and export | Saved runs with their frozen inputs, resume/continue, comparison and method-specific reports. |

The prominent **final fit statistics** refer to the best saved state. Include
its attempt/generation number and label the last current state separately.
For each reported residual, record its domain, weighting and normalization;
calculate initial-to-best improvement only on the same objective and calculator
revision, handling a zero initial score explicitly. Do not compare an R residual
and a wavelet score as though they were the same statistic. Reuse the path-fit
statistics panel where applicable, but do not populate its covariance or reduced
χ² fields with unrelated RMC quantities.

Results show experimental, initial and best-fit χ(k), R magnitude, R real and
R imaginary using the same transform settings. Display the current-state curve
only as an explicit additional choice: Metropolis can accept a worse current
state, so the last state is not necessarily the best result.

Add current/best residual traces, recent window changes, acceptance diagnostics,
and initial/best structure comparison with cell information. Keep spectra and
coordinates paired to the same state. Show structural distributions and path
reports on demand. Export coordinates with periodic-cell metadata; ordinary XYZ
alone does not preserve the periodic configuration.

Reuse the existing residual criterion: at least 3,000 attempts, 500-attempt
windows and three consecutive passing comparisons. Best improvement must be at
most `1e-5 + 0.005 × |previous best|`, and absolute mean change at most
`1e-5 + 0.01 × |previous mean|`. Display `InsufficientHistory`, `StillChanging`
or `ResidualPlateau`, with the actual configured thresholds. Budget exhaustion,
manual stop and calculator failure are separate termination reasons. Calculator
revision boundaries must not be blended into a single convergence trace.
EA diagnostics must use their generation/population semantics; do not label
generations as individual Monte Carlo attempts.

Add versioned project metadata for mode, per-mode drafts, source/processing
identity, structure/cell, resolved objective, calibration, constraints, seed,
calculator identity and run summaries. Default missing mode to Path fitting.
Reuse the core checkpoint formats for optimizer state and RNG. Store large
checkpoints and trajectories as associated artifacts with portable references
and hashes, supporting the existing linked/embedded storage choices. Budget
sizes against current archive limits and preserve unknown project metadata.
Cold resume rebuilds caches and verifies calculator identity and stored scores;
it must not claim exact continuation with a different calculator. Opening a
project restores its results without automatically restarting a calculation.

## Implementation sequence and acceptance gates

| Slice | Deliverable | Required evidence before enabling it |
| --- | --- | --- |
| 1. Shared transforms | Extract local-spectrum kernels; retain compatibility; add Wavelet under Transform. | Existing direct/FFT, mask, edge and irregular-grid regressions remain valid; new Spectrum invalidation and transform-view checks pass; existing checkpoint interpretation is unchanged. |
| 2. Complete single-spectrum RMC flow | Top-right selector, structure/supercell setup, exact calculator, worker, pause/stop/resume, native R objective, curves and best structure. | Same frozen inputs and seed agree with the Rust API; selected-path prerequisites do not block RMC; input edits cannot retarget a running job; failures preserve best state. |
| 3. Saved desktop workflow | Project round trips, cold resume, history, portable artifacts and publication report. | Legacy projects still open as Path fitting; resumed sequence agrees with uninterrupted execution; source changes and calculator mismatches are detected; cancellation cannot attach late results to another project. |
| 4. Hybrid EA–RMC | Enable the third mode with population, diversity and local-move controls. | Positive `local_steps` is explicit; generation/local-move counters and checkpoints are correct; mode switches preserve drafts; equal-budget comparisons use the same objective and exact final checks. |
| 5. Broader data and objective support | Weighted structures, joint datasets and validated k/q/wavelet input adapters. | Per-dataset settings and preprocessing guards survive save/resume; masks and normalization agree with shared transforms; unsupported combinations remain explicit. |

Keep disabled or hidden controls out of released workflows until their slice is
complete. Batch RMC across a series should follow the joint/mixture work with an
explicit compute budget, cancellation and per-frame recovery design. GULP,
one-dimensional path refinement and experimental phase/amplitude overrides remain
deferred as previously agreed.

For the first end-to-end qualification, replay the accepted Cu₂O problem through
the desktop adapter and compare the initial and archived exact scores to the
[existing evidence](rmc-cu2o-adaptive-qualification.md). Use short runs for UI and
performance development; final presented fits need a larger declared budget and
a residual-trend report. Do not overwrite historical data or call a completed
budget convergence. Benchmark the desktop against the same API job on the same
machine, including setup, rendering and checkpoint overhead.

The immediate implementation target is slices 1 and 2, followed by persistence
before treating the desktop workflow as complete. Publish progress and plots
to the existing results site during implementation. This planning pass changed
no numerical implementation and did not run new fitting experiments.
