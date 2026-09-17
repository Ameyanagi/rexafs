# Prepared ReFEFF paths for RMC

These Rust APIs are **unreleased**, added after 0.2.9. Enable `refeff-runner`.
ReFEFF remains the primary scattering calculator. No EVAX source is incorporated.
The existing `RefeffCalculator` remains available for fresh-potential and
pinned-potential pipeline comparisons; its stage and wrapper timings are now
reported by `RefeffCacheStats`.

## What is reused

`PreparedRefeffContext` runs the atomic, potential and phase stages at an explicit
reference absorber. It retains typed phase tensors, radial transition factors,
Legendre normalization and the GENFMT driver setup. Later explicit paths call
the public ReFEFF numerical kernels directly, without rerunning those stages or
creating intermediate files. This uses locked ReFEFF facade 0.4.0 and core/engine/I/O
0.3.0; it does not require modifications to the dependency or registry sources.

The adapter is implemented in
[`prepared.rs`](../crates/rexafs/src/xafs/rmc/prepared.rs). It calls
`genfmt_ordinary_path_evaluation_from_driver_setup`, the I/O handoff adapters,
`remove_phase_jump`, and `terp1` from [ReFEFF](https://github.com/Ameyanagi/refeff).
The dependency was checked against the live registry on 2026-09-17 and updated
to [ReFEFF 0.4.0](https://github.com/Ameyanagi/refeff/releases/tag/v0.4.0), published
2026-09-13. Older 0.3.0 checkpoints intentionally fail the backend identity check.
It implements ordinary EXAFS summation with S₀²=1, no extra Debye–Waller factor,
and no FF2X energy corrections. RMC applies calibrated S₀² and ΔE₀ outside this
adapter. Linear polarization is retained in the prepared context.

`PathCatalogue` enumerates directed closed walks with stable `(atom, image)`
vertices. It includes central-atom revisits and nonconsecutive repeated vertices.
Reverse walks count separately, with degeneracy one. Each is evaluated in
ReFEFF's canonical time direction: averaging two raw directions would differ
numerically from the backend's degeneracy convention. No amplitude screening
is used; prepared calculations require `path_criteria: [0., 0.]` and matching
catalogue/ReFEFF radius and leg limits.

The catalogue reserves paths that might enter the radius after a move. If each
atom can move at most d Å and the path has n legs, its half-length can shorten by
at most n d Å. Enumeration therefore retains the corresponding reference envelope.
Requests outside the declared displacement bound fail explicitly. Limits on
vertices, extensions, paths and contexts prevent unbounded enumeration. The
atom-to-path map identifies which contributions require updating after a move.

## Default exact caching and experimental representatives

`AccelerationSettings::default()` selects **exact caching**: `ScatteringBasis::Exact`,
a 256 MiB snapshot cache, `adaptive: None` and `moments: None`. Omitted JSON
settings select the same mode. This is the recommended default.
`ScatteringBasis::Exact` reuses unchanged paths and recalculates affected paths
using the typed kernels.
“Exact” refers to these path evaluations at **fixed reference potentials**; it
does not mean exact electronic physics, infinite scattering order or equality
with the rounded file pipeline.

The **experimental, opt-in** `ScatteringBasis::Frozen` computes immutable representative amplitude/phase
tables from the reference. Equivalent reference geometries share a table, within
one electronic context. Every actual path retains its current half-length in the
`2 k R` phase. This approximation neglects changes in scattering amplitude and
angular dependence while its leg-length and angle guards are satisfied. Exceeding
either guard uses direct typed scattering for that path. These geometric guards
are **not spectral error guarantees**.

Use `compare_reference` to measure absolute and relative χ differences against
direct typed paths at the same potentials and catalogue. Independently compare
to `RefeffCalculator` with fresh potentials to assess the electronic approximation.
The basis never updates from accepted or rejected trials. Its settings and
reference coordinates are part of the calculator identity. Tightening them
changes the model and requires a new run, rather than silently resuming a chain
under a different objective.

Optional `MomentSettings` accelerates sums of length-dependent phases in groups
that share a frozen table. It uses centered moments, a Taylor remainder bound,
a floating accumulation estimate, and direct-sum fallback when tolerance cannot
be met. `PathMomentExpansion::check_direct` measures agreement on an explicit k
grid. Its tolerance concerns the sum of unit phasors, before multiplication by
scattering amplitude. It is separate from the representative-basis error. Requesting
individual path reports uses direct path sums.

## Determinism and resources

`PreparedRefeffCalculator::calculate_batch` uses a bounded Rayon pool and returns
results in request order. Path sums use fixed catalogue/group order. Last-geometry
snapshots may describe a rejected trial; changed-atom comparisons update them
correctly on the next request. Clearing or evicting caches does not change the
chosen model. Serializing these caches is unnecessary for exact session resume.

`cache_bytes` bounds retained numerical snapshots, not total process memory.
Immutable phase tensors, catalogues, representative tables and transient results
also consume memory. `max_contexts`, `max_total_paths`, catalogue limits, and
`workers` bound these other work dimensions. A shared cancellation token is checked between paths (even on cache hits) and
during electronic setup; a single typed kernel cannot be interrupted mid-call.
Per-absorber timeouts also apply. `EvolutionSession` batches independent children
in unchanged RNG order, limiting a transient batch to 32 candidates and roughly
one million χ samples (or one larger candidate). Its crossover, selection and
hypermutation policy remains distinct from Metropolis cooling.

Preparation remains a separate cost;
do not report a warm cache hit as end-to-end RMC performance.

## Validation and reproducible timing

[`rmc_path_benchmark.rs`](../crates/rexafs/examples/rmc_path_benchmark.rs) constructs
a 48-atom, 2×2×2 Cu₂O cell, selects a Cu absorber, displaces one neighboring O,
and writes raw spectra, options, timings and error metrics to a new JSON file.
It times setup and updates separately, and compares the frozen basis to direct
typed paths and the pinned-potential pipeline. This benchmark uses no experimental
data and does not measure original EVAX.
[`rmc_transform_benchmark.rs`](../crates/rexafs/examples/rmc_transform_benchmark.rs)
separately measures prepared direct/FFT local maps and controlled moment sums,
with raw errors and setup costs. Its explicit 1e−6 phasor tolerance is a benchmark
choice, not the default 1e−10.

```sh
CARGO_TARGET_DIR=/Users/ryuichi/dev/rexafs/target cargo run --release --locked \
  -p rexafs --features refeff-runner --example rmc_path_benchmark -- NEW_RESULT.json
```

[`rmc_paths.rs`](../crates/rexafs/tests/rmc_paths.rs) checks finite/periodic
identities, paths entering the radius, central revisits, polarized single
scattering, multiple scattering, cache invalidation, fallbacks and moment/direct
report transitions. The prepared module also checks Cu₂O path-family membership
and explicit records against the ReFEFF pipeline.
`path_reports` attaches stable atom/image membership, reference-family labels,
k-weighted importance norms and optional direct-path accuracy to active paths.
Importance is a diagnostic, not a rigorous screening bound: no weak family is
silently discarded. `transform_path_fourier` returns complex R and real filtered-q
contributions; `LocalSpectrumTransform::transform` accepts k-weighted path arrays.
Complex contributions add, while their magnitudes do not. Numerical agreement is a
software check; it does not establish physical model adequacy or uniqueness.

## Optimizer, fitting and analysis additions

`SessionSettings` now supports per-element scales/selection weights, fixed group
translations or simultaneous displacements, predetermined cooling, and optional
score/stagnation/acceptance stopping rules. All proposal distributions remain
symmetric in coordinates. Cooling changes the numerical tolerance and is an
optimization policy, not equilibrium sampling at a physical temperature. The
attempt index and diagnostic state are checkpointed; errors commit no move or RNG
state. Default settings preserve the original random-draw sequence.

`calibrate_dataset` keeps geometry and mixture fractions fixed, computes a union
of requested theoretical k points once per absorber, searches an explicit ΔE₀ grid,
and solves the bounded S₀² optimum at each energy. The result includes the full
candidate trace and calculator identity. `fit_report` reports χ RMSE and a
noise-scaled, k-weighted normalized residual separately from the chosen objective.
These are numerical measures, not reduced chi-square or confidence intervals.

`Objective::LocalSpectrum` adds weighted masks and direct/FFT Morlet or Gaussian
STFT maps. FFT mode requires uniform k with grid-aligned centers; automatic mode
falls back to direct evaluation for irregular/off-grid data. Zero padding prevents
circular wraparound. Both modes use the same trapezoidal quadrature and
unit-norm discrete kernels; tests compare boundary values and masks. The existing
`Objective::Wavelet` convention remains available unchanged.

Structural APIs report unwrapped arithmetic mean-square displacements, their
tensor and optional translation removal; explicit path legs/angles; path-length
histograms; and periodic bond-angle distributions. Optional MSD and path-histogram
restraints are declared priors in objective units. They do not implement EVAX's
median-based historical MSD statistic, INVERT, or a calibrated force field.

Deferred GULP, one-dimensional path refinement, experimental phase/amplitude
overrides and other deferred workflows remain outside this implementation.

## Initialization and trajectory workflows

`seeded_disorder` copies a configuration and adds independent Cartesian uniform
noise with a recorded ChaCha8 seed. It is initialization, not thermal disorder.
`seeded_substitution` selects an exact number of host sites without replacement;
prepare new fixed references and absorber selections after changing chemistry.
`suggested_supercell_repeats` uses interplanar spacings, including triclinic cells.
A repeat-size recommendation is a geometric starting point, not a convergence
claim. `estimate_catalogue_resources` counts actual geometric paths without running
ReFEFF and reports numerical spectrum payload separately from total RAM.

`stream_xyz_spectra` reads one conventional XYZ frame at a time with fixed cell,
species order, mixture fractions and experimental settings. It emits evaluated
states to a callback, supports early completion, and rejects truncated frames,
changed topology, excessively long lines or constraint violations. Extended-XYZ
cell metadata is not interpreted. Coordinates written by `to_xyz` have decimal
rounding; the JSON checkpoint retains the exact state for restart.

## Runnable Rust example

[`rmc_prepared.rs`](../crates/rexafs/examples/rmc_prepared.rs) exercises the APIs
without external data. Its Cu dimer data are synthetic and its calibration uses
the independently known generating geometry. It writes calibration provenance,
optimizer settings, a checkpoint, structural/path reports, a local Fourier map
and streamed trajectory scores to a new directory.

```sh
cargo run --release --locked -p rexafs --features refeff-runner \
  --example rmc_prepared -- NEW_DEMO_DIRECTORY
```

A minimal prepared calculator uses explicit unscreened path settings:

```rust,ignore
let options = RefeffOptions { path_criteria: [0., 0.], ..Default::default() };
let acceleration = AccelerationSettings {
    catalogue: PathCatalogueSettings { displacement: 0.4, ..Default::default() },
    workers: 4,
    ..Default::default() // Exact affected-path updates; no frozen-basis approximation.
};
let mut calculator = PreparedRefeffCalculator::new(options, references, acceleration)?;
let mut session = RmcSession::new(&problem, &settings, &mut calculator)?;
session.run(&mut calculator)?;
```

Use [`rmc_cu2o.rs`](../crates/rexafs/examples/rmc_cu2o.rs) commands `run-prepared`
and `check-prepared` for a local experimental job. The JSON job must explicitly
choose zero path criteria; archived screened calculations are not overwritten.
See the [work record](rmc-implementation-progress.md) for measured validation,
including approximation settings that failed the accuracy target.

## Residual-trend monitoring (unreleased)

`residual_trend` compares successive nonoverlapping windows of current and best
objective records. Defaults use 500 attempts per window, three stable windows,
a preceding mean baseline and at least 3,000 total attempts. A plateau requires
both best-score improvement ≤1e−5 + 0.005|previous best| and mean-score change
≤1e−5 + 0.01|previous mean| in each recent window. These are explicit numerical
criteria, not statistical convergence or structural uniqueness tests. Missing or
short histories report `InsufficientHistory`; changing scores report `StillChanging`.
A stable but poor local fit can report `ResidualPlateau` and still need model or
optimizer changes. This helper is observational and never stops a session itself.

The Cu₂O example writes `convergence.json`. Its `resume-prepared` command extends
an exact saved chain into a new directory, preserving RNG and original geometry
references; it records prior attempts separately from the new invocation's timing.
Plots now label the attempted-move count instead of claiming a “final fit.”

## Follow-up search and basis APIs

See [hybrid search, adaptive stages and population caching](rmc-search-upgrade.md)
for acceptance feedback, immutable trained basis stages, explicit rescoring,
first-shell fitter integration and the measured population-cache comparison.

## Experimental adaptive audits (unreleased)

Adaptive training is disabled by default. It is a research option, with no
established overall speedup in the [Cu₂O qualification](rmc-cu2o-adaptive-qualification.md).
Keep exact caching for routine refinement; audit and verify exact final spectra
when explicitly evaluating adaptive mode.

[`AdaptiveBasisController`](../crates/rexafs/src/xafs/rmc/adaptive_control.rs)
adds periodic exact checks in the actual dataset objectives, error-triggered
retraining, transactional optimizer rescoring and exact fallback. Prepared
electronic contexts and catalogues are shared across stages. See the
[audit guide](rmc-adaptive-audits.md) for numerical tolerances, checkpoint
semantics, costs, limitations and a runnable Rust example.
