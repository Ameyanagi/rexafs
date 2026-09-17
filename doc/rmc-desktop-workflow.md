# Desktop RMC workflow (unreleased)

The first desktop implementation adds **Fit mode: Path fitting / RMC** at the
upper right of Fitting. It supports one processed spectrum and one explicit
periodic structure. The numerical engine is the existing `RmcSession`; ordinary
path fitting remains the default for older projects. This is development work,
not a feature in the released 0.2.9 desktop.

## Prepare and run

1. Import and process a spectrum through Background. RMC captures the processed
   `Spectrum` with `RmcDataset::from_spectrum`, including its preprocessing state,
   source identity and the desktop processing recipe. It does not run AUTOBK again.
2. Open Fitting, select **RMC**, choose a structure from the existing library or
   structure import tools, and press **Use structure**.
3. In Calculate, edit the supercell repeat counts. The 3D preview rebuilds in a
   background task after a 150 ms typing delay, without scattering calculations.
   Obsolete preview tasks cannot overwrite newer input. Invalid edits retain the
   last valid preview and block preparation/run. **Use suggested size** derives
   repeat counts from twice the cluster radius plus displacement margin. The core
   rejects allocations above 10,000 atoms. The preview shows coordinates, atom/
   absorber counts and periodic cell dimensions. Atom indices are
   zero based; fixed atom `0` is the default anchor. Blank absorber indices select
   every atom of the chosen element. Mixed/partial occupancy requires an explicit
   fully occupied realization and is rejected by the core.
4. Check cluster/path radii, maximum path legs, displacement envelope and minimum
   atom distance. Exact cached ReFEFF uses zero amplitude cutoffs and a fixed
   reference electronic potential. A changed geometry receives exact typed-path
   calculations within that reference model. Adaptive approximation is experimental
   and is not exposed by these desktop controls.
5. In Model, inspect S₀², fit ΔE₀, k/R ranges, integer k weight, move size,
   numerical Metropolis tolerance, seed and attempt budget. Pair constraints use
   entries such as `Cu-O=1.5, Cu-Cu=2.0`, in Å. Prepare the initial fit to inspect
   its curves before starting moves, or select Run RMC directly.

On first entering RMC with a processed spectrum, starting fit ranges use its
Transform k bounds, clipped to measured and calculator support including the
native taper, and R minimum is saved Rbkg + 0.15 Å. **Use spectrum ranges** applies
these suggestions again explicitly without changing S₀² or ΔE₀. Starting controls
use 10,000 attempts, 0.03 Å moves, numerical tolerance 0.001, 0.2 Å displacement,
a 1 Å minimum distance and a fixed anchor at atom 0. These are visible starting
values; calibration, constraints, cell size and convergence need sample-specific
assessment. Larger suggested cells can substantially increase computation cost.

The initial desktop objective is the native R-space **real plus imaginary**
residual, normalized by experimental power, with any configured structural penalty
added separately. The k/R magnitude plots are diagnostics. R minimum must be at
least the saved AUTOBK Rbkg: the adapter rejects an incompatible range rather than
quietly changing it. One integer k weight from 0 through 3 is supported. Automatic
noise estimation and multiple k weights are not connected here. S₀² and ΔE₀ remain
fixed during coordinate moves; the form shows their actual values.

## Live results, pause and recovery

Run opens Results. A dedicated background thread owns the calculator and optimizer.
The UI checks its bounded event queue every 250 ms; completed-state updates are
produced approximately every 500 ms. A single long scattering evaluation can delay
an update. Results include experimental/initial/best curves in k, R magnitude,
R real and R imaginary; the best configuration and its cell; current/best objective
history; acceptance, hard-constraint rejections, elapsed time and active-path reuse.
Best curves and coordinates come from the same saved state. Metropolis may accept
a worse current state, so the final current state is not necessarily the best fit.

**Pause** finishes the current move, saves a checkpoint, and retains the prepared
calculator for a fast warm resume. **Stop and save** requests calculator
cancellation and saves the last complete transactional state. A kernel that does
not inspect cancellation immediately may delay stopping. Wait for the stopped or
paused state before closing the application if the latest state is important.

Checkpoints are written atomically after preparation, about every 30 seconds at
move boundaries, and on pause, stop, budget completion or a calculation failure.
They live in `~/.rexafs/rmc/run-*/checkpoint.json` and contain current, initial and
best states, RNG state, frozen inputs and settings. An unexpected application exit
can lose work since the last completed checkpoint. **Recover latest run** discovers
the newest recovery file; **Open checkpoint…** selects a particular run. Opening
loads results without automatically launching a calculation. **Resume / continue**
rebuilds the same exact calculator and resumes its saved sequence. Completing the
budget enables an additional batch of the configured number of attempts.

Changing the active spectrum, processing or draft settings does not retarget a
saved/running job. Resume always uses the saved problem. Stop the existing worker
before starting a new problem or a path fit. An input mismatch is shown above the
results. Paused workers still own their calculator and must be stopped before a
second fitting job starts.

Saving an `.rxs` project embeds the latest completed checkpoint with the RMC draft
and selected mode. This first implementation retains one run in the project;
recovery files remain independently available. Large trajectory collections and
linked checkpoint archives are later work. Existing project size limits apply.

## What convergence means

The desktop calls the shared `residual_trend` diagnostic. It requires at least
3,000 attempts, compares nonoverlapping 500-attempt windows and requires three
consecutive passing comparisons. For each comparison:

- Best-score improvement must be at most `1e-5 + 0.005 × |previous best|`.
- Absolute mean-score change must be at most `1e-5 + 0.01 × |previous mean|`.

The display distinguishes insufficient history, still changing and residual
plateau. This is an empirical numerical criterion, not proof of a unique or
physically complete structure. It does not automatically stop this desktop run.
Reaching the attempt limit is reported separately. Short software checks are not
final scientific refinements. History retains the most recent 10,000 attempts.

Elapsed time includes setup and active execution but excludes time paused; the
reported seconds per attempt is inclusive and is not an isolated scattering
benchmark. Cache reuse is `reused active paths / (reused active paths + exact path
calculations)` for the current calculator, which resets after a cold resume.

## Export and Transform wavelets

Pause or finish before **Export result…**. A new result folder contains the full
checkpoint, initial/best XYZ, configuration JSON preserving periodic cells,
experimental/initial/best k and R CSV files, convergence settings/report and a
Markdown summary. Ordinary XYZ alone does not preserve a periodic cell.

Wavelet display is under **Transform → Wavelet**. It uses processed unweighted
χ(k), applies the selected Transform k weight once, and displays magnitude, real
or imaginary Morlet coefficients. Controls set Fourier-distance limits, carrier
frequency and grid sizes. R is Fourier distance, not a phase-corrected bond length.
This map does not change the RMC objective. The shared
`rexafs::transform::LocalSpectrumTransform` also supports Gaussian STFT and direct
or FFT evaluation; the existing RMC API retains its numerical/error compatibility.

## Scope and implementation

Hybrid EA–RMC, multiple weighted structures, joint datasets, automatic copying of
path-fit calibration, advanced proposal/annealing controls, structural distributions
and path-report inspection remain desktop follow-up work. The underlying Rust
engine already supports many of these. Do not read the two-option desktop selector
as implying an EA calculation.

Implementation and regression evidence are in:

- [Desktop adapter and worker](../crates/rexafs-gui/src/rmc_fitting.rs).
- [RMC workspace](../crates/rexafs-gui/src/app/shell/rmc.rs).
- [Wavelet view adapter](../crates/rexafs-gui/src/wavelet.rs).
- [Shared transform](../crates/rexafs/src/xafs/transform/local_spectrum.rs).
- [Transform compatibility tests](../crates/rexafs/tests/rmc_transforms.rs).
- [Original plan and later slices](rmc-desktop-workflow-plan.md).

Automated worker tests use a synthetic Cu–O dimer to verify frozen Spectrum
inputs, prepare/pause, exact cold continuation against an uninterrupted native
session, project round trips, invalid checkpoint rejection, live progress, stop
recovery and exports. This is software verification, not a Cu₂O fit-quality claim.

## Local verification, 18 September 2026

- Default core suite: 404 passed, 3 ignored; strict core Clippy passed.
- Desktop release suite: 559 passed, 6 ignored, including live worker/stop,
  exact continuation, Spectrum snapshots, default ranges and bounded Cu₂O
  supercell previews.
- Stable and Next Rust API references regenerated successfully.
- Desktop compilation without optional calculation backends also passed.
- A release desktop build was launched with a copy of `Cu oxides.prj`.
- Native computer-use verification is pending: this session reports no enabled
  computer-use surfaces. Automated tests do not establish visual usability.

The desktop guide, project defaults and live builder are development work;
no new converged Cu₂O refinement is claimed by these software checks.
