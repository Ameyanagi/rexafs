# Hybrid search, adaptive stages and population caching

Unreleased development API, implemented on `feature/rmc-refeff` on 2026-09-17.
ReFEFF 0.4.0 remains the primary calculator. The native REXAFS complex-R residual,
including both real and imaginary components, is unchanged. These additions
address the follow-up EVAX source audit; they do not reproduce EVAX's entire
optimizer or establish a converged experimental structure.

## Search policies and what their names mean

| Term | Role in this implementation |
| --- | --- |
| Single-coordinate change | A proposal generator: displace one movable atom. It does not itself decide acceptance. |
| Metropolis | An acceptance rule: accept improvements and sometimes uphill proposals according to the declared numerical tolerance. |
| RMC | Fit structural configurations to experimental spectra with coordinate proposals, constraints, calculated spectra and an acceptance rule. `RmcSession` uses Metropolis. |
| EA / genetic algorithm | Maintain a population, select parents, cross over coordinates, mutate and retain elites. `EvolutionSession` implements this optimizer. |
| Hybrid EA–RMC | Apply ordinary local Metropolis attempts to each nonelite offspring after the EA stage, retaining its best visited state. Enable with `EvolutionSettings::local_steps`. |

The shared proposal/evaluation/acceptance kernel is
[`attempt_move`](../crates/rexafs/src/xafs/rmc/session.rs). For an objective increase
ΔF, it accepts with probability exp(−ΔF/(2T)), where F is the configured spectral
objective plus penalties and T is a numerical tolerance in matching objective
units. T is not a thermodynamic temperature. The existing
[RMC guide](rmc.md) describes the objective and assumptions.

`local_steps = 0` preserves the existing EA policy and random draw sequence.
Start with one local attempt per offspring and benchmark alternatives. The Cu₂O
`hybrid` example uses eight as an explicit experimental policy. Selection and
crossover themselves are not a reversible sampling kernel. A local chain can
accept uphill moves, but its best visited state is retained for optimization.
This is a REXAFS hybrid policy; EVAX's active population mutation stage does not
establish that EVAX uses these repeated inner sweeps or the same family policy.
Original displacement references remain fixed throughout all generations.

Hybrid local moves reuse species selection, collective proposals, weight moves,
constraints and Metropolis evaluation. Cooling uses the **total completed local
attempt count across all offspring and generations**. The original EA mutation
still uses its configured mutation probability and hypermutation multiplier;
acceptance feedback controls only the local MC coordinate proposals. Session
stopping rules apply to `RmcSession`; EA uses its generation/local-step limits.
A whole generation commits transactionally: errors leave population, RNG,
adaptation and counters unchanged, although calculator work counters can advance.

```rust
use rexafs::rmc::{EvolutionSettings, SessionSettings, StepAdaptation};
let session = SessionSettings {
    adaptation: Some(StepAdaptation::default()),
    ..Default::default()
};
let evolution = EvolutionSettings { local_steps: 1, ..Default::default() };
```

## Acceptance feedback and convergence

`StepAdaptation` is opt-in. By default, every 100 coordinate attempts it shrinks
the proposal multiplier by 1.1 below 10% acceptance and grows it by 1.1 above 40%.
The scale is bounded to [0.05, 1], so the declared coordinate widths remain caps.
These are empirical REXAFS defaults, not an exact copy of EVAX's 10% update rule.
Hard coordinate rejections count as unsuccessful attempts; mixture-weight moves
are excluded, and calculator failures do not advance adaptation. Species and
collective widths receive the same multiplier. `freeze_after` optionally ends
feedback after a declared number of coordinate attempts.

The scale, partial window counts, last accepted fraction and update count are
checkpointed even when ordinary history is disabled. Move records report the
scale used; hybrid generation records report attempts, acceptances, hard
rejections and the scale for the next generation. Resume preserves these values.
Adaptation is an optimizer convenience, not an equilibrium-sampling guarantee.

`evolution_residual_trend` reuses the same best-plus-mean window criterion as
`residual_trend`, using each generation's population mean. Its window and minimum
count are **generations**, so pass explicit settings; the example uses 10-generation
windows, three successive stable comparisons and at least 60 generations. Missing
historical means cannot establish a plateau. All `rmc_cu2o_search` modes now write
`convergence.json`; a time budget does not itself imply convergence.

## Population cache and measured result

`AccelerationSettings::snapshots_per_context` defaults to one. Population workflows
can retain parents and children, within the existing global byte budget. The
calculator chooses the retained same-grid geometry with the fewest changed atoms;
ties prefer the most recent. Exact affected-path updates and fixed-order reductions
are unchanged. Geometry count is a resource setting, excluded from scientific
identity; changing it cannot change a spectrum or invalidate an exact checkpoint.

`PreparedRefeffStats` now distinguishes:

- All catalogue visits, including currently inactive envelope paths.
- Unchanged entries reused, including inactive entries.
- Reused active entries, as a subset of reused entries.
- Changed entries excluded by the current radius, exact evaluations and basis evaluations.
- Identical-geometry hits, snapshot count and conservative retained payload bytes.

For direct path summation, the active-entry reuse fraction is
`reused_active / (reused_active + exact + basis)`. The all-entry accounting identity
is `visited = reused + exact + basis + outside_radius`. These definitions avoid
calling inactive entries nonzero scattering hits. Cached bytes exclude prepared
potentials, immutable bases and transient batch storage; shared arrays may be
counted more than once, making the retained estimate conservative.

A release-build benchmark on the local Mac16,12 (10 logical CPUs, 32 GiB) replayed
240 identical requests for one Cu absorber in a 48-atom 2×2×2 Cu₂O cell. It used
12 synthetically displaced parents, 120 local children, 191 k samples over
2.5–12 Å⁻¹, radius 4.5 Å, four legs, fixed reference potentials and exact paths.
The one-slot run preceded the 25-slot run; setup/parent warm-up is excluded below.
Some development checks were running concurrently, so timings are indicative,
while work counts and equality checks are deterministic.

| Cache geometries/context | Update time | Time/request | Exact paths | Active-entry reuse | Final payload |
| --- | ---: | ---: | ---: | ---: | ---: |
| 1 | 7.9146 s | 32.977 ms | 12,619 | 47.94% | 0.223 MiB |
| 25 | 0.3508 s | 1.462 ms | 499 | 97.94% | 5.680 MiB |

All 240 output spectra matched **bit for bit**; measured update time improved
22.56× for this replay. This is not a 22.56× whole-EA or experimental-fit speedup.
The [runnable benchmark](../crates/rexafs/examples/rmc_population_benchmark.rs)
records seeds, settings, counts, timings and equality checks. The local raw record
is `/Users/ryuichi/dev/evax-analysis/output/cu2o-population-cache-2026-09-17.json`.

## Adaptive representative training with explicit stage boundaries

`AccelerationSettings::adaptive = Some(AdaptiveBasisSettings { ... })` enables
error-driven training under `ScatteringBasis::Frozen`; moment approximation must
be disabled. Its geometric feature radius replaces the legacy frozen leg/angle
guards. Supply an audit k grid and optional training configurations for each
mixture component; the electronic reference is always included. The manifest,
including its epoch and training coordinates, participates in calculator identity.

Within each electronic context, training shares representatives across paths and
training population members of the same ordered species family. Unpolarized
features include all pair distances; polarized features retain absorber-relative
coordinates. Features use the existing 1e−8 Å geometric quantization. The nearest
representative within `geometry_radius` supplies a fixed amplitude/phase table
with the actual path propagation length. No representative is shared between
incompatible electronic contexts.

Every active training path is compared with direct typed ReFEFF scattering on the
declared grid. If its error exceeds the declared tolerance, training adds a
representative at that geometry and revisits earlier members whose nearest
representative may have changed. Exhausting the representative budget disables
the family and uses exact paths. The summed spectrum is also checked: cancellation
can amplify total relative error even when every path meets its own tolerance.
A failing total conservatively disables its contributing families, and all totals
are checked again. Preparation respects cancellation, timeouts and sample budgets.

The test is `||k^w(χ_model−χ_exact)||₂ ≤ absolute_error + relative_error*||k^wχ_exact||₂`.
Here k is in Å⁻¹, χ is dimensionless, w is the declared integer k weight, and the
norm is the discrete square-root sum over audit samples. The absolute allowance
has the units of k^wχ; the relative allowance is dimensionless. Defaults are
w=2, relative_error=0.001 and absolute_error=1e−10. These are software defaults,
not experimental uncertainty estimates. Zero-norm references use the absolute
allowance. `adaptive_reports()` records splits, fallback families and measured
summed-spectrum errors for each prepared context.

**Training agreement does not bound unseen configurations.** Calls on another
k grid, or paths beyond trained geometric support, use exact paths. Inside that
support, spectral errors must be checked on independent configurations using
`compare_reference`; its returned unweighted arrays can also be evaluated with
the native fitting objective. A new basis remains opt-in until it is qualified on
the actual system. No faster approximate Cu₂O final fit is claimed here.

`refreshed_basis(new_manifest)` constructs a new calculator with a larger epoch.
Already prepared electronic contexts and catalogues are shared immutably.
Basis training is repeated lazily, while geometry caches start empty; include
training/audit cost when choosing refresh frequency. No hidden refresh occurs mid-trial. Call
`RmcSession::rebase` or `EvolutionSession::rebase` explicitly to rescore the retained
initial/current/best states or the entire population. Rescoring is transactional;
RNG and original displacement bounds remain unchanged. Histories from the old
model are cleared, stopping/stagnation score windows reset, and an identity
revision is checkpointed. Preserve the previous checkpoint externally if the
old trace is needed. Reconstructing the same manifest on resume reproduces the
same model; it does not depend on the order of previous accepted/rejected trials.

## Four-parameter first-shell calibration

`calibrate_first_shell` calls the existing native `feffit_joint_with_options`
optimizer on a copied `FeffFitDataset`. The caller selects first-shell two-leg
paths, fixed degeneracies, data, noise scale and complex-R range. Existing path
corrections are replaced with four common parameters: S₀², ΔE₀, distance ratio
α and σ². Each path uses ΔR=reff·(α−1), with reff in Å and σ² in Å²; imaginary
energy and higher cumulants are zero. All windows, weights, residual calculations
and solver diagnostics come from the ordinary tested fitter. Input arrays are
unchanged; ReFEFF-produced paths use the normal path-loading API.

The result stores input, settings, source provenance, fit values/errors and solver
termination. `apply_amplitude_energy` copies a problem and transfers only S₀² and
ΔE₀ after numerical convergence. Distance/disorder remain recorded as first-shell
model parameters; they are not silently imposed on explicit atomic configurations.
This avoids double-counting fitted Debye–Waller damping on top of coordinate
variance. Inspect the model and boundary solutions before treating calibration
as physical evidence. A synthetic retained Cu path test recovers all four known
parameters to 1e−5 absolute tolerance and a native R factor below 1e−10.

## Examples and remaining qualification

```sh
cargo run --release --locked -p rexafs --features refeff-runner \
  --example rmc_population_benchmark -- NEW_OUTPUT_JSON
cargo run --release --locked -p rexafs --features refeff-runner \
  --example rmc_cu2o_search -- hybrid SOURCE_K_JOB ACCELERATION SECONDS NEW_DIR SEED [R_MIN]
```

The Cu₂O search example now defaults to R=1.15–4 Å, above the source
background radius of 1 Å. Pass R_MIN=0.8 explicitly to reproduce the historical
range; archived results retain their original objectives.

The hybrid example uses exact scattering unless its input acceleration manifest
explicitly selects an approximation. It saves settings, checkpoint, population,
local-move diagnostics, complex-R native-fitter agreement and convergence status.
Short API checks are not final fits. Compare multiple seeds at declared larger
budgets before claiming hybrid EA improves the experimental fit.

Optional validated family screening, higher-order qualification beyond six legs,
EVAX-specific family competition/norm conventions and the previously deferred
force-field/one-dimensional workflows are not added in this update. The opt-in automatic controller is described in
[the audit guide](rmc-adaptive-audits.md). Its defaults do not establish a universal
error bound for unseen geometries.

## Retained checks from this update

Core, native fitting-parity, transform, session and prepared-path tests were run,
including cold reconstruction of adaptive training, out-of-support exact fallback,
failure during local search, explicit model revisions, four-parameter recovery and
feedback scaling of species/collective moves. Clippy checks all targets with
ReFEFF enabled and warnings denied. The session APIs were also tested without
the optional ReFEFF backend.

The retained Cu₂O hybrid **API check** completed 4 generations,
320 local Metropolis attempts and 209 local acceptances.
It used the original structure, seed 20260917, exact prepared scattering and the
existing normalized complex-R objective. Setup took 19.016 s and search took
98.971 s; a requested 90 s budget finished its current generation.
Its best native-fitter ratio was 0.27158102. The declared trend status is
**InsufficientHistory**. This short run is neither a converged final fit nor
a fair comparison with the earlier 800 s RMC/EA runs. It does not replace their
retained best structures. The full local record is
`/Users/ryuichi/dev/evax-analysis/output/cu2o-hybrid-api-check-2026-09-17/`.
