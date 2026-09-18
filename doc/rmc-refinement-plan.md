# RMC refinement improvements — unreleased

This work builds on the local startup and transform-range corrections based on
`dev` commit `d3a6b6f`. It is not part of the published 0.2.10 desktop. No private
experimental arrays or checkpoints belong in this document or its test fixtures.

## Intended workflow

1. Check preprocessing and scattering-path coverage. The Fourier fit interval
   must have contributions represented by the chosen scattering model. Fourier
   R is not a phase-corrected interatomic distance, so the path-radius warning is
   a prompt to inspect coverage, not a strict physical equality.
2. Review amplitude and energy calibration before structural optimization. The
   existing native `calibrate_dataset` searches a declared theoretical energy
   shift grid and solves bounded least-squares amplitude at each shift, with
   geometry fixed. The desktop preview requires an explicit **Use calibration**
   action and rejects stale previews after input changes. A limit-hitting estimate
   is flagged. A perfect starting crystal is not necessarily a suitable reference
   for a disordered sample; a low residual alone does not validate calibration.
3. Explore with **Auto moves**, or choose **Fixed moves** to retain manual control.
   New desktop drafts start at 0.05 Å per Cartesian coordinate. Auto uses the
   core `SessionSettings::with_auto_moves()` policy: every 100 coordinate attempts,
   multiply or divide the width by 1.2 when the accepted fraction is above 40%
   or below 10%, respectively. The multiplier stays between 0.1 and 2. It freezes
   after 80% of the original budget. The dimensionless Metropolis tolerance
   decreases linearly to zero at the original budget. Continuing does not restart
   cooling or change the original displacement bounds. These are rexafs tuning
   heuristics, not universally optimal physical parameters. Old serialized drafts
   without the new selector retain fixed moves, and old checkpoints keep their
   saved settings and random sequence.
4. Pause or finish RMC and choose **Refine best**. The first stable implementation
   uses numerical derivatives of the full configured scattering calculation, one
   atom's three coordinates at a time. It uses central 0.001 Å probes where
   feasible and one-sided probes at constraint boundaries. A normalized downhill
   direction starts at 0.02 Å and is halved until it improves the objective or
   falls below 0.0001 Å. All trials retain the original hard constraints. Defaults
   limit work to three passes and 5,000 geometry evaluations. This is a rexafs
   local search, not AutoGrad, equilibrium sampling or a convergence certificate.
   The RMC checkpoint remains unchanged. The separate audit retains inputs,
   calibration, constraints, initial/best spectra, accepted moves and stop reason.
5. Compare multiple seeds, residuals, bond distributions and physical plausibility.
   Compare performance at equal elapsed time and scattering work, not only equal
   attempted moves. Numerical descent cannot establish a unique atomic structure.

The calibration warning follows the correlations discussed in the
[EvAX manual, experimental-data chapter](https://www.dragon.lv/evax/downloads/EvAX_manual.pdf):
energy shift correlates with bond lengths, and amplitude correlates with disorder.
rexafs shifts the theoretical k grid; EvAX's experimental-axis convention has
an opposite energy-shift sign. Do not transfer a number without checking its
convention. Our automatic step policy is not a reimplementation of EvAX's
simulated-annealing schedule.

## Rust API

```rust,ignore
let settings = SessionSettings {
    moves: RmcSettings {
        steps: 10_000,
        step_size: 0.05,       // Å per Cartesian coordinate
        temperature: 0.001,   // dimensionless numerical tolerance
        ..Default::default()
    },
    ..Default::default()
}.with_auto_moves();
let mut session = RmcSession::new(&problem, &settings, &mut calculator)?;
session.run(&mut calculator)?;
let local = session.refine_best(&LocalRefinementSettings::default(), &mut calculator)?;
// session.checkpoint() is unchanged by local refinement.
// local.best contains the verified coordinates and spectra.
```

Implementation: [session settings and entry point](../crates/rexafs/src/xafs/rmc/session.rs),
[local solver](../crates/rexafs/src/xafs/rmc/local_refinement.rs),
[calibration](../crates/rexafs/src/xafs/rmc/calibration.rs),
and [desktop diagnostics](../crates/rexafs-gui/src/rmc_fitting/diagnostics.rs).
Python and TypeScript exposure of the new refinement entry point is not included
in this change.

## Differentiation decision

The unsuccessful compiler-based automatic-differentiation prototype was removed
from this checkout on 2026-09-18. Historical source and compiler logs remain in a
local research archive. There is no autodiff dependency, feature, build requirement
or production implementation in this work. Numerical coordinate refinement remains
available and is documented above.

Forward-mode dual numbers propagate a value and its derivative through each
operation. They avoid finite-difference truncation, although floating-point
rounding still applies. A Rust implementation such as
[`num-dual`](https://docs.rs/num-dual/latest/num_dual/) requires the differentiated
functions to be generic over the dual-number type. Current prepared scattering
uses `f64`, `Complex64` and numerical array APIs; wrapping only its outer call
would not differentiate the calculations inside it. This would require a separate
numerical-kernel refactor and validation of angular, amplitude and phase effects.
It is not justified as a prerequisite for optimizing one energy-shift parameter.

Reverse-mode differentiation is generally attractive for a scalar objective with
many input coordinates, whereas forward mode propagates one or more input
directions at a time. Neither method removes discontinuities from hard constraints
or discrete path selection. See the
[automatic-differentiation survey](https://jmlr.org/papers/v18/17-468.html).

## Implemented in the source checkout: ΔE₀ with S₀² fixed

The proposal from 18 September was implemented on 19 September 2026 and remains
unreleased. The [Rust guide](rmc.md)
and [desktop workflow](rmc-desktop-workflow.md#unreleased-fitting-controls) document
the controls and APIs. No Enzyme or other automatic-differentiation library is used.

`SessionSettings::with_energy_refinement(-15.0..=15.0)` enables the initial broad
search and periodic local searches. Each dataset gets its own shift while its S₀²
remains fixed. The original experimental alignment, normalization E₀, input arrays,
fit ranges and displacement-reference structures remain unchanged. The full
objective verifies each update before committing it. A backend failure during an
update rolls back the coordinate attempt, including its random draws.

Shifts are saved on initial/current/best states and trajectory frames. The initial
search and periodic updates have separate audits. Resume, calculator rebase,
adaptive audits and numerical coordinate refinement use the shifts of the state
they evaluate. Evolutionary crossover explicitly rejects this policy until energy
inheritance is defined; it never silently discards a fitted shift. GUI exports add
`fit-parameters.json` beside the existing curves and structures.

Initial searches use a coarse 0.5 eV grid and then a ±1 eV grid at 0.1 eV spacing;
periodic searches use the latter. Each includes the current value and honors the
global bounds. The initial interval is 250 coordinate/weight attempts. It is a
numerical starting point to qualify against elapsed time, not a physical constant.
Changing the theoretical k grid invalidates cached sampled paths, so search cost
must be measured rather than inferred from the single-parameter count.

Energy shift correlates with distance; fixed S₀² should come from appropriate
calibration because amplitude errors can bias recovered disorder. See the
[EvAX manual](https://www.dragon.lv/evax/downloads/EvAX_manual.pdf). The alternating
policy is a rexafs choice. It does not establish equilibrium sampling or physical
convergence. Independent experimental convergence studies remain necessary.

## Earlier coordinate-refinement validation

The 22 native session tests, 13 desktop RMC tests (one manual 256-site test
ignored), and the new Rust API documentation example passed. Strict core Clippy,
formatting and whitespace checks passed. These include automatic-policy adjustment
and exact resume, local descent, fixed atoms, original displacement bounds, budgets,
cancellation and failure rollback. Private experimental qualification and its
derived results are recorded separately outside the repository.

Computer-use checks used the public `cu_150k.xmu` example, a four-atom periodic Cu
cell, 100 RMC attempts and a separate test application. Auto/Fixed selection,
explicit calibration application, bound warnings, local-refinement curves and
combined result export were exercised. The final release build also reloaded the
saved refinement and displayed the corrected stale-input warning without losing
it to deferred field events. Export tests verify floating-point CSV round trips
and reject audits with mismatched calculator identity or session settings.
Local refinement completed three passes with 90 geometry evaluations and
preserved the RMC checkpoint. The small cell is
for workflow testing, not representative structural analysis.

## Energy-refinement validation

The 19 September source checkout passes 30 session tests, eight spectrum-input
tests, six ReFEFF integration tests and 15 desktop RMC tests. One manual
large-cell desktop test remains ignored. Strict core Clippy, formatting,
whitespace checks and the new Rust builder example pass. The final PR check also
passes the full default core suite (470 tests, three ignored), the full desktop
suite (626 tests, 7 ignored), all-target default core Clippy and website
diagnostics. The targeted counts above overlap these full suites.

The energy tests cover independent dataset shifts, fixed amplitudes, untouched
experimental inputs, current/best state consistency, exact checkpoint resume,
rollback of a failed energy step including random draws, mixture-weight cache
reuse, calculator rebase, local coordinate refinement and old fixed-energy
checkpoints. Evolutionary sessions reject the unsupported energy policy explicitly.

Computer-use checks on a separate release-build application used the public
`cu_150k.xmu` example and a four-atom Cu cell. A saved fixed-energy project opened
with **Fixed** selected. Selecting **Refine**, setting bounds and an interval of
two attempts, running six attempts, saving/exporting, reopening the checkpoint,
and continuing to ten attempts all worked. Energy searches were recorded at the
expected steps and S₀² stayed fixed. The displayed shift matched the JSON export;
the exported CSV curves matched saved arrays within floating-point text rounding.
The final build also started a new search at ΔE₀=0 and found +8.00 eV with
S₀² fixed at 0.500; the displayed initial and best curves and saved project
retained those distinct parameter states.
The small cell and short budget test the workflow, not structural convergence.

A local qualification protocol compares two seeds, 500 attempts and intervals of
100, 250 and 500 attempts. It checks independent evaluations at each saved best
structure and shift, exact checkpoint continuation, end-of-budget residuals and
equal elapsed times. Experimental measurements, derived results and detailed
timing reports remain local-only. The 250-attempt default is a starting heuristic;
these short budgets cannot establish an optimal cadence or physical accuracy.

## Remaining qualification

Multi-seed, calibrated experimental convergence studies have not been completed.
The short ΔE₀ timing checks above are separate from the earlier coordinate-only
checks; longer runs with calibrated amplitude are still needed to assess physical
convergence and cadence sensitivity.
No dual-number coordinate differentiation is planned in this change.
