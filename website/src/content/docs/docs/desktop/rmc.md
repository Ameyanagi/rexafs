---
title: "RMC structure refinement"
description: "Refine atomic coordinates with exact cached ReFEFF, inspect live fits, and resume saved runs."
audience: user
---

In **rexafs 0.2.11**, choose **Fit → Fit mode: RMC** at the upper right of the
fitting workspace. Reverse Monte Carlo (RMC) proposes random coordinate moves,
calculates their spectra and accepts or rejects them against the measured data
and configured constraints. It refines a periodic structure rather than the
path parameters used by ordinary EXAFS fitting.

The desktop uses one processed spectrum and one structure. The
[Rust API](/api/rust/rexafs/xafs/rmc/index.html) additionally supports evolutionary
search, weighted structures and joint datasets. Those controls are not yet in
the desktop, and RMC is not exposed by the Python or TypeScript bindings.

## Prepare the spectrum and structure

1. Import the measurement and inspect its normalization and **Background**
   result. The RMC input captures the processed spectrum and its preprocessing
   information, including AUTOBK Rbkg. It does not rerun AUTOBK for each move.
2. Select **RMC**, choose a fully occupied periodic structure and press
   **Use structure**. Resolve mixed or partial occupancy before refinement.
3. In **Supercell**, edit repeat counts and inspect the live preview. **Use
   suggested size** accounts for cluster radius and the allowed displacement.
   Larger cells cost more to calculate. Invalid edits preserve the last valid
   preview and block a new run; preview updates do not calculate scattering.
4. Check absorber, edge, cluster/path radii, maximum path legs and constraints.
   Blank absorber indices use all atoms of the selected element. Fixed atom
   indices are zero based. Pair minimum distances accept entries such as
   `Cu-O=1.5, Cu-Cu=2.0`, in Å.
5. In **Fit settings**, inspect calibration, k/R ranges, k weight and the
   attempt budget. Select **Preview initial fit** and compare its curves with
   experiment before choosing **Run RMC**.

New jobs start with 10,000 attempts, automatic moves beginning at 0.05 Å,
numerical Metropolis tolerance 0.001, a 0.2 Å displacement envelope, 1 Å global
minimum distance and atom 0 as an anchor. These are editable starting values,
not a physical model or a convergence guarantee. S₀² stays fixed during RMC;
ΔE₀ can remain fixed or be refined periodically. Calibrate both for the sample.
The Metropolis tolerance controls acceptance of worse trial scores; it is not a
measured thermodynamic temperature. Older saved runs retain their settings.

<figure>
  <img src="/screenshots/0.2.11/rmc-settings.jpg" alt="RMC fit settings with inherited k and R ranges, Auto CPU workers and automatic coordinate moves" loading="lazy" width="1192" height="768" />
  <figcaption>Signed 0.2.11 release: k = 2–12 Å⁻¹ and R = 1.5–3.5 Å copied from Transform; Auto resolves to ten workers on this machine. This is a public Cu workflow example.</figcaption>
</figure>

## Fit ranges and the background

The next action stays at the upper right on each RMC page, matching path fitting.
Fit settings places **Preview initial fit** beside **Run RMC →**.

New jobs copy Transform windows, widths and forward sampling as well as k bounds
and weight. Explicit Back FT R bounds are used; otherwise the starting R range
is Rbkg + 0.15 Å to max(4 Å, Rbkg + 1.15 Å). **Use spectrum ranges** copies these
settings again after processing edits. It does not retarget a saved run. ReFEFF
coverage expands for the fit window and ΔE₀, up to the adapter's 30 Å⁻¹ limit,
with actual returned support checked. Missing theory is rejected rather than
extrapolated. The [workflow guide](https://github.com/Ameyanagi/rexafs/blob/v0.2.11/doc/rmc-desktop-workflow.md)
explains these settings.

The default objective minimizes the normalized sum of squared **real and
imaginary R-space residuals**, using the same Fourier mapping as native path
fitting. The magnitude plot is a diagnostic; matching magnitude alone is not
the objective. Configured structural penalties are added separately.

Starting k limits follow the spectrum's Transform settings within valid measured
and calculator support, including the window. Without an explicit Back FT lower
bound, R minimum starts **0.15 Å above the saved AUTOBK Rbkg**. The desktop
rejects R minimum below Rbkg, where background removal can produce artifacts.
**Use spectrum ranges** reapplies the suggested
ranges without changing S₀² or ΔE₀. One integer k weight from 0 through 3 is
supported; automatic noise estimation and multiple k weights are not connected.

Exact cached **ReFEFF 0.4.0** is the default. Changed geometry receives exact
path calculations within fixed reference electronic potentials; unchanged paths
can be reused. This does not recalculate the electronic potential after every
move. Adaptive scattering remains experimental, opt-in in Rust, and absent from
these desktop controls.

The desktop allocates a prepared context for every selected absorbing site,
edge and setting. This removes the previous 128-context default limit for larger
cells without discarding absorbers. Independent path-count and memory-related
limits still apply, and larger cells cost more.

New runs also share electronic preparation for identical local inputs after
deterministic ordering of the scatterer rows. All absorbing sites
and their explicit paths are retained. **Run details** shows calculated and shared
electronic contexts. Sorting can change numerical summation and the representative
atom for a potential when nearest sites tie. Old checkpoints retain their original
ordering; restarting a new job opts into the improved preparation. See the
[startup profiling method](https://github.com/Ameyanagi/rexafs/blob/v0.2.11/doc/rmc-startup-profiling.md)
for reproducible timing and numerical-agreement checks.


<span id="cpu-workers-in-the-source-checkout"></span>

## CPU workers

**Fit settings → CPU workers** starts at **Auto** for
new jobs, using available logical CPUs up to 64. Enter a count from 1 to 64 to
override it, or clear the field to restore Auto. Independent absorbers run first.
**Advanced settings → Parallel paths**, enabled for new jobs, uses spare workers
for paths within an absorber. Both levels share one thread pool.

Older projects keep their existing scheduling. **Run details** shows the captured
worker count; saved jobs resume with that count. Changing the form affects a new
run. More workers may use more temporary memory and are not always faster.

<span id="energy-refinement-in-the-source-checkout"></span>

<figure>
  <img src="/screenshots/0.2.11/rmc-run-details.jpg" alt="Saved RMC run details showing two CPU workers and one calculated plus 31 shared electronic preparations" loading="lazy" width="1192" height="768" />
  <figcaption>The short check used two workers and retained all 32 absorbing sites. Displayed timings describe this one software check, not a performance benchmark.</figcaption>
</figure>

## Energy refinement

In **Fit settings**, choose **ΔE₀ → Refine** to update
the theoretical energy shift while keeping S₀² fixed. **Fixed** remains the default,
including for older projects. Set S₀² from an appropriate reference calibration.

A new run searches the starting shift before moving atoms. Further local searches
run every 250 attempts by default. Change the interval and energy bounds under
**Advanced → Energy refinement**. Bounds are in eV; the default initial search
uses 0.5 eV spacing and local searches use ±1 eV at 0.1 eV spacing. The grid spacing
is resolution, not uncertainty. Only a full-objective improvement is accepted.

Results show ΔE₀ for the best structure and warn when it reaches a bound. Saved
runs keep each structure's matching shift; exports include these values in
`fit-parameters.json` and the report. Experimental energy alignment and spectrum
preprocessing stay unchanged. This is alternating structural/energy optimization;
a small residual alone does not establish a unique structure.

<span id="calibration-and-local-refinement-in-the-source-checkout"></span>

## Calibration and local refinement

New jobs offer **Auto moves** (starting at 0.05 Å) and **Fixed moves**. Auto adjusts
the width during the early part of the original budget and cools the numerical
tolerance to zero. Existing saved runs keep their settings.

**Estimate calibration…** previews amplitude S₀² and theoretical energy shift
ΔE₀ using the starting structure. Review the bounds and choose **Use calibration**
explicitly; changes to the inputs require a new estimate. A suitable experimental
reference is needed to interpret these correlated parameters physically.

After pausing or finishing, choose **Refine best…** to try local coordinate
improvements. This uses numerical gradients, with full scattering verification
and the original constraints. It preserves the RMC checkpoint. **Refinement**
compares the previous and refined spectra; **Export result…** includes both,
refined XYZ coordinates and a separate JSON audit. A smaller residual does not
establish a unique structure. Local refinement uses numerical derivatives.

## Read the live results

Results show experiment, initial calculation and best calculation in k space,
R magnitude, R real and R imaginary. The best coordinates and curves describe
the same saved state. The current state can be worse than the best because the
Metropolis rule can accept uphill moves. **Hide initial** changes the display
scale without changing the objective.

The statistics separate best score, improvement, acceptance, hard-constraint
rejections and residual diagnosis. **Run details** includes timing, cache
statistics and saved-input provenance. Updates arrive between completed
calculations; one long scattering evaluation can delay the display or a pause.

<figure>
  <img src="/screenshots/0.2.11/rmc-results.jpg" alt="Reopened ten-attempt Cu RMC result with fixed amplitude, an energy-bound warning and convergence not assessed" loading="lazy" width="1192" height="768" />
  <figcaption>Saved results reopened in 0.2.11. This ten-attempt check used ±1 eV bounds and energy updates every five attempts. It reached +1 eV and is not a calibrated or converged scientific fit. <a href="/licenses/#desktop-0211-rmc-captures">Capture and input provenance</a>.</figcaption>
</figure>

## Pause, save and resume

**Pause** finishes the current move, saves a checkpoint and retains the prepared
calculator for a fast resume. **Stop and save** cancels work and retains the
last complete state. Wait for the paused or stopped status before closing when
the latest state is needed.

Checkpoints are saved after preparation, approximately every 30 seconds at move
boundaries, and on pause, stop, completion or calculation failure. A sudden exit
can lose moves since the preceding checkpoint. **Recover latest run** opens the
latest recovery file; **Open checkpoint…** selects another. Loading results does
not start calculation automatically. **Resume saved run** restores the saved
inputs, settings and random state and rebuilds the calculator when necessary.

Saving an `.rxs` project embeds the latest complete checkpoint and RMC setup.
Recovery files also remain under `~/.rexafs/rmc/run-*/checkpoint.json`. The project
currently retains one RMC run. After the budget finishes, enter **Additional
attempts** and choose **Continue optimization**; the starting increment is 10,000.

Editing preprocessing, changing the active spectrum or editing a new-run draft
does not alter a saved run. Resume uses its original problem, including its
background settings. An input mismatch is shown above the results. To use new
preprocessing, stop the old worker and prepare a new run.

## Decide whether to continue

The budget is separate from convergence. The desktop's empirical diagnostic
requires at least **3,000 attempts**, compares nonoverlapping **500-attempt
windows**, and needs **three consecutive passing comparisons**:

- Best-score improvement ≤ `1e-5 + 0.005 × |previous best|`.
- Absolute mean-score change ≤ `1e-5 + 0.01 × |previous mean|`.

The result is **Insufficient history**, **Still changing** or **Residual plateau**.
The recent view shows 2,000 attempts; **All retained attempts** shows the retained
history. A plateau does not stop the run automatically or prove a unique,
physically complete structure. Inspect both spectral components, constraints and
structural distributions, and compare independent seeds when drawing conclusions.

The [workflow and implementation record](https://github.com/Ameyanagi/rexafs/blob/v0.2.11/doc/rmc-desktop-workflow.md)
explains persistence, numerical conventions and software checks. Its short Cu₂O
verification run remained **StillChanging**, not converged. The
[Rust guide](https://github.com/Ameyanagi/rexafs/blob/v0.2.11/doc/rmc.md) covers
Spectrum inputs and the broader RMC and evolutionary APIs.
