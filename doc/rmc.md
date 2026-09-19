# Experimental reverse Monte Carlo with ReFEFF

This **Rust API, introduced in 0.2.10**,
refines explicit atomic coordinates and mixture fractions against EXAFS.
ReFEFF is the primary calculator. The engine now includes calculation reuse,
chemical restraints, k/R/q/wavelet objectives, exact-resume sessions, evolutionary
search, weighted structures, dataset-specific calculator settings, and structural
analysis. See the [desktop workflow](rmc-desktop-workflow.md) for GUI controls.
The advanced session APIs below are Rust-only; Python and TypeScript bindings
do not yet expose RMC.

The implementation is original Rust code. It neither incorporates EVAX or
RMCProfile source nor reads their job formats. The
[original gap audit](rmc-evax-gap-analysis.md) describes the earlier reference
implementation; the [new validation and performance record](rmc-performance.md)
describes this extension. These capabilities do not establish EVAX numerical
parity or experimental accuracy.

## Refine theoretical ΔE₀ with fixed S₀² (0.2.11)

Version 0.2.11 adds optional energy refinement to `RmcSession`. Existing
jobs retain fixed energy shifts. For a new job:

```rust
use rexafs::rmc::SessionSettings;
let settings = SessionSettings::default()
    .with_auto_moves()
    .with_energy_refinement(-15.0..=15.0);
// Set each input dataset's s02 to its independently calibrated fixed value.
// let mut session = RmcSession::new(&problem, &settings, &mut calculator)?;
// session.run(&mut calculator)?;
// let shifts_eV = session.best().energy_shifts(session.problem())?;
```

Bounds are theoretical energy shifts in eV, not normalization E₀ or experimental
energy alignment. Positive ΔE₀ evaluates theory at smaller k according to the
formula below. S₀² remains exactly the input value for each dataset. Each dataset
has one independent shift shared by its absorbers and structure components.
The [Larch path documentation](https://xraypy.github.io/xraylarch/xafs_feffpaths.html)
describes these EXAFS parameters; this alternating optimizer is a rexafs policy.

`EnergyRefinement` defaults to a broad initial grid with 0.5 eV spacing, followed
by a local ±1 eV search at 0.1 eV spacing. It then performs a local search every
250 coordinate/weight attempts, counting rejected attempts. The interval is a
starting setting, not a universal optimum. The inclusive bounds clip local grids;
the current shift is always included. Grid spacing is numerical resolution, not
an uncertainty estimate. The full objective, including structural penalties, is
recalculated at the chosen shifts. A change is kept only if that score decreases.
The procedure is an optimizer, not equilibrium sampling. Review correlations
between energy shift and bond distance and compare independently calibrated data.

The input problem is unchanged. `EnsembleState::energy_shifts` returns the shifts
matching its coordinates, spectra and score. `initial()` retains the original
pre-search state; `initial_energy()` records the initial search. Periodic updates
appear in `SessionStep::energy`, and trajectory frames retain their shifts.
Energy searches do not consume random draws or count as extra coordinate attempts.
A failing periodic search rolls back its entire coordinate step, including RNG,
current/best states and history. Resume and calculator rebase evaluate every state
at its own shifts. Local coordinate refinement holds its starting shifts fixed.
Old checkpoints retain fixed-energy behavior. Evolutionary crossover currently
rejects this option explicitly; use `RmcSession` for alternating energy refinement.

Bounds must preserve real, increasing theoretical k across the full measured grid,
including Fourier tapers. Backend support errors abort without extrapolation.
Per-search candidate counts are bounded, and an unresolved calibration is an error.
The desktop expands requested ReFEFF k support to cover the configured bounds.

Implementation: [energy search](../crates/rexafs/src/xafs/rmc/energy_refinement.rs),
[transactional session and state evaluation](../crates/rexafs/src/xafs/rmc/session.rs),
[state representation](../crates/rexafs/src/xafs/rmc/ensemble.rs).
These additions are available from 0.2.11 in Rust and the native desktop.
Python and TypeScript do not yet expose RMC.

<a id="unreleased-absorber-first-cpu-parallelism"></a>

## Absorber-first CPU parallelism (0.2.11)

The prepared calculator has one bounded Rayon pool. Its `workers` setting is the
total CPU thread budget, not a process count. Independent absorbers run concurrently
first. With `parallel_paths: true`, a batch with fewer absorbers than workers can
also evaluate independent scattering paths concurrently using the same pool.
This includes calculations with a single absorbing site. The two levels do not
multiply the thread count. Electronic preparation remains serial across contexts;
identical electronic inputs can still share preparation.

```rust
use rexafs::rmc::AccelerationSettings;
let acceleration = AccelerationSettings {
    workers: 4,
    parallel_paths: true,
    ..Default::default()
};
```

`workers` accepts 1–64. The core defaults remain one worker and no path fallback
for compatibility. The GUI enables automatic parallelism for new jobs, choosing
available logical CPUs up to 64. Users can set a smaller or larger explicit budget
within that limit. More threads can increase temporary memory and do not guarantee
lower elapsed time. Compare complete runs on the intended hardware.

Path spectra are summed in catalogue order and absorbers in request order, keeping
floating-point reductions independent of scheduling. Cached results, rejected
trials and cancellation retain the existing rules. Coordinate attempts and
energy-search stages remain sequential. Worker settings are captured in a job's
calculator identity; resume keeps them rather than redetecting CPUs.
`RefeffOptions::threads` separately controls internal backend preparation and
remains one by default. It does not accelerate the later prepared path loop.

Implementation: [bounded pool and path evaluation](../crates/rexafs/src/xafs/rmc/accelerated.rs),
[desktop job settings](../crates/rexafs-gui/src/rmc_fitting.rs).

## Default calculator and experimental adaptive mode

Use `PreparedRefeffCalculator` with `AccelerationSettings::default()` for the
recommended **exact caching** mode. Unchanged paths are reused and changed paths
are recalculated with typed ReFEFF kernels at fixed reference electronic
potentials. The default cache budget is 256 MiB; adaptive training and moment
approximations are disabled. Omitted JSON settings use these same defaults.

**Adaptive mode is experimental and opt-in.** It requires an explicit frozen
basis and `adaptive: Some(AdaptiveBasisSettings { ... })`. The
[Cu₂O qualification](rmc-cu2o-adaptive-qualification.md) found that the initial
basis failed the spectral-error gate on all six held-out structures and every
adaptive-start search fell back to exact evaluation. No sustained speedup was
established. Use [periodic exact audits](rmc-adaptive-audits.md) and verify final
spectra with exact paths when researching this mode. Existing explicitly
configured jobs and checkpoints retain their chosen mode.

## Start with a processed Spectrum

The recommended input is `RmcDataset::from_spectrum(&spectrum,
options)`. `Spectrum` is the public alias of `XASSpectrum`. This constructor
captures the processing state and reads the authoritative `k()` and `chi()`
getters, so users do not need to extract arrays or assemble provenance manually.
See the [spectrum-input guide](rmc-spectrum-input.md) for a complete example,
defaults, the Rbkg guard and checkpoint behavior.

```rust,ignore
use rexafs::{rmc::*, structure::Edge};

// `spectrum` already has AUTOBK results; `transform` is an explicit FeffFitTransform.
let mut input = RmcSpectrumOptions::new(absorbers, Edge::K, transform);
input.k_range = Some([2.5, 12.0]); // Include the chosen k-window tapers.
input.s02 = 0.98402;              // Example calibration, not a universal default.
input.delta_e0 = 8.76239;         // Fitting shift in eV, not spectrum.e0().
let dataset = RmcDataset::from_spectrum(&spectrum, input)?;
let problem = EnsembleProblem::single(configuration, dataset);
let mut session = RmcSession::new(&problem, &settings, &mut calculator)?;
session.run(&mut calculator)?;
let trend = residual_trend(session.history(), &ResidualTrendSettings::default())?;
```

This path uses an experimental-power-normalized real-plus-imaginary R objective
by default. The array-only APIs below retain their original defaults, including
k-space scoring. No previously saved job or published result is changed.

## Session controls and array-only input

Enable `refeff-runner` for the ReFEFF backend. The session, objectives, constraints,
and analysis types are available without that feature for custom calculators.

```rust,ignore
use rexafs::rmc::*;

// `basic` is an RmcProblem with explicit geometry and measured, unweighted χ(k).
let problem: EnsembleProblem = basic.into();
let settings = SessionSettings {
    moves: RmcSettings {
        steps: 1000,
        movable_atoms: vec![1, 2, 3], // zero-based; other atoms stay fixed
        ..Default::default()
    },
    trajectory_stride: 10,
    ..Default::default()
};
let mut calculator = RefeffCalculator::new(RefeffOptions::default())?;
let mut session = RmcSession::new(&problem, &settings, &mut calculator)?;
while let Some(record) = session.step(&mut calculator)? {
    if record.step % 100 == 0 {
        session.save_checkpoint("run.json")?;
    }
}
let best = session.best();
```

`new` validates and copies the inputs, normalizes mixture weights, and evaluates
the starting state. `step` completes one attempt; `run` completes the remaining
configured attempts. `current`, `initial`, and `best` retain coordinates and their
matching spectra. `set_step_limit` extends a run without resetting its RNG or
original displacement reference. `evaluate_ensemble` evaluates without moves.
The older `evaluate`, `refine`, and callback-based `refine_with_progress` APIs
remain available for a single configuration with the original k-space objective.
Their `RmcResult` is a result record; use `RmcSession` for recoverable runs.

Two runnable ReFEFF examples are included:

```sh
cargo run --release --locked -p rexafs --features refeff-runner \
  --example rmc_session -- /tmp/rexafs-rmc-session
cargo run --release --locked -p rexafs --features refeff-runner \
  --example rmc_refeff -- --demo /tmp/rexafs-rmc-basic 12
```

Both require a new output directory. The session example generates a synthetic
Cu dimer target, refines coordinates with pinned potentials, saves/reloads a
checkpoint mid-run, and writes `job.json`, `checkpoint.json`, `best.json`,
`best.xyz`, `trajectory.xyz`, `distances.json`, and `calculator.json`. Path
contributions are in `best.json`; provenance includes the pinned reference,
settings, diagnostics and calculator identity. This demonstrates software
self-consistency, not a realistic material or structural uniqueness. The basic
example also accepts `JOB.json NEW_DIR`, where the job contains `problem`,
`settings`, and `refeff` fields.

## Geometry, data and weighted structures

`Configuration` contains atoms with atomic numbers and Cartesian positions in Å.
Its optional `cell` contains three Cartesian lattice **row vectors**, also in Å.
Coordinates remain unwrapped. Atom order provides stable IDs; neither cell
refinement, atom insertion, species changes nor symmetry expansion occurs during
refinement. `from_xyz` imports a finite cluster. `from_structure` constructs an
explicit repeated cell from already expanded, fully occupied, single-species
sites. General nonsingular triclinic cells are supported; image enumeration,
rather than a fractional-rounding nearest-image shortcut, checks distances.

Each `RmcDataset` wraps an `ExafsDataset` with increasing nonnegative k in Å⁻¹,
unweighted dimensionless χ, positive noise scales σ, positive dataset weight,
kweight 0–3, fixed positive S₀², and fixed ΔE₀ in eV. Theory is sampled at
`sqrt(k² − ETOK ΔE₀)`, with ETOK in Å⁻²/eV. Negative arguments or extrapolation
outside calculated support fail. No extra Debye–Waller factor is added: disorder
comes from explicit coordinates and absorber averaging.

An `EnsembleProblem` has multiple `WeightedStructure` components. Their
nonnegative input weights are normalized once. For dataset d, the model is

`m_d(k) = S₀²_d Σ_s w_s [Σ_{a∈A_ds} χ_dsa(k) / |A_ds|]`.

Here s indexes structures, w is their dimensionless fraction, A is that dataset's
selected absorber list, and χ is the single-absorber calculation. Each list must
contain distinct atoms of the same element; the element must also agree across
components. `absorbers_by_structure` supplies one list per component; an empty
outer list uses `exafs.absorbers` everywhere. Fractions describe contributions to
the normalized EXAFS signal and are not automatically mass or volume fractions.

Set `weight_move_probability` above zero to refine fractions. A proposal chooses
two components and transfers a uniform signed amount in ±`weight_step` while
preserving their sum. Negative fractions cause hard rejection. Weight moves
reuse all component spectra and require **zero scattering calls**. Coordinate
moves recalculate only their component. A zero-weight component still has its
spectrum evaluated and its structural constraints enforced. `movable_atoms=None`
uses the session atom list, or all atoms when that list is empty; `Some(vec![])`
fixes an entire component. A completely fixed mixture can use weight probability
1, or zero steps for evaluation only.

Per-dataset `refeff: Some(RefeffOptions { ... })` overrides all calculator defaults
for that dataset, including polarization, SCF radius, cluster/path radii, leg
count and path screening. An absent override uses the calculator defaults.
Custom `ExafsCalculator` implementations can override `calculate_request` to
support these settings and path reports; the default rejects unsupported requests.

## Performance and the two ReFEFF modes

`RefeffCalculator::new` preserves geometry-dependent potential recalculation.
Its exact cache stores native-grid spectra keyed by the complete generated local
FEFF input and calculator options. Repeated environments, unaffected distant
atoms, repeated datasets and revisited geometries avoid the pipeline. Requested
k grids are interpolated afterward, so changing sampling does not invalidate the
native spectrum. Cache hits preserve the corresponding uncached result; this is
exactness relative to the adapter's printed input and ReFEFF's numerical handoffs.
It is not a claim of infinite coordinate precision.

For substantially faster changed-environment evaluation, explicitly pin
reference structures:

```rust,ignore
let references = problem.structures.iter()
    .map(|s| s.configuration.clone()).collect();
let mut calculator = RefeffCalculator::new(options)?
    .with_frozen_potentials(references)?;
```

A full reference calculation supplies `pot.bin`, `phase.bin`, and `xsect.dat` for
each component/absorber/edge/options combination. Subsequent geometries use
`CONTROL 0 0 0 1 1 1`: ReFEFF reruns PATH, GENFMT and FF2X, including angular and
multiple-scattering geometry changes. Potential indices are sorted by element,
so neighbor-distance reordering cannot silently change species assignments.
Topology/cell changes and changes in the local potential species set fail.
Reference artifacts are keyed independently of trial history: neither rejection
nor cache eviction changes the scientific model. The official
[FEFF control-card documentation](https://feff.phys.washington.edu/feff/Docs/feff8/feff8web/node5.html)
describes the modular execution controls; the locked ReFEFF runner's artifact
handoff is exercised directly in our tests.

**Pinned potentials are an approximation.** Changes to the electronic potential,
phase shifts and atomic cross-section caused by moving atoms are omitted. This
also freezes a requested SCF solution at the reference geometry. Compare against
full calculations for representative displacements and chemistry before choosing
this mode. Recompute and revalidate an entire new session to change references;
there is no history-dependent automatic refresh. This is not EVAX's scattering
interpolation-table algorithm.

Each of the two LRU caches has a default 64 MiB payload/key limit. Entries larger
than the limit are not retained. `set_cache_capacity(0)` disables caching for a
baseline; `clear_cache` discards entries without changing references.
`stats()` reports full/path pipeline attempts, cache hits, evictions and retained
payload bytes. Those bytes exclude allocator overhead, reference coordinates,
states and ReFEFF's temporary workspace. `diagnostics()` retains nonfatal backend
messages. Cancellation is checked even on cache hits; create a new uncancelled
calculator to resume after cancellation.

Defaults are cluster radius 6 Å, half-path limit 4 Å, maximum four legs, kmax
16 Å⁻¹, screening `[4, 2.5]` percent, no SCF, orientational averaging, one worker
and a 300-second timeout per pipeline. `max_legs=2` selects single scattering.
`path_criteria=[0,0]` retains weaker paths and can sharply increase cost. Radii,
order, filtering and reference choice need material-specific convergence checks.
Large-cluster path enumeration can still dominate: this release has no persistent
spatial neighbor list or path-level derivative update. Geometry limits are 10,000
explicit atoms and 1,000 atoms in a local absorber cluster, with bounded periodic
image searches; they are resource guards, not affordable-size guarantees.

## Objectives and acceptance

For each measured point i, first form the residual
`u_i = [(m_i − χ_i)/σ_i] (k_i/k_ref)^w`, with `k_ref=1 Å⁻¹` and integer kweight w.
Noise scaling and kweight apply once, before any transform. Dataset weight W
multiplies the mean squared transformed residual. `Objective::K` gives
`F_d = W_d Σ_i u_i²/N_d`, reproducing the original API.

`Objective::R(transform)` and `Q(transform)` use the existing
[rexafs fitting transforms](../crates/rexafs/src/xafs/fitting/transform.rs).
Residuals are linearly interpolated to a uniform grid starting at zero and set
to zero outside measured support. `kstep` sets that grid; its default is the
first measured spacing. Requested k fit bounds must lie inside measured support.
The transform's kweight/kweights and fitspace fields do not supply additional
weighting: the dataset and enum variant determine those choices.

The forward convention is `U(R) = Δk/√π Σ_j u_j K_j exp(−2i k_j R)`, where K is
the chosen k window and `R_l = lπ/(nfft Δk)` in Å. The R window multiplies this
complex transform. R scoring averages `|U(R_l) Rwindow_l|²` over selected bins.
Q scoring averages the squared real back-transform over the selected q bins,
using rexafs's existing Larch-compatible inverse scaling. See the
[processing theory](processing-theory.md) for Fourier conventions. The FFT length
must be a power of two from 16 to 65536, measured support must fit in its first
half, and R fits must lie within 0–10 Å and below Nyquist. Fourier R peaks are
not automatically phase-corrected bond lengths. Numeric Å units are used in these
objectives; their scales differ from the k objective, so dataset weights and
Metropolis tolerance must be chosen for the selected space.

`Objective::Wavelet(WaveletSettings { k_centers, r, omega0 })` uses a
scale-dependent Morlet kernel. At Fourier distance R in Å, define
`s = omega0/(2R)` in Å⁻¹, `t_i=k_i−k_center`, and
`h_i = Δk_i [exp(2iR t_i)−exp(−omega0²/2)] exp[−t_i²/(2s²)]`.
Δk_i are trapezoidal integration widths. Normalize each sampled kernel by
`√Σ_i |h_i|²`; the score is W times the mean of `|Σ_i h_i u_i|²` over all requested
center/R pairs. Kernels are prepared once per session. The positive sign selects
the conjugate convention for real residuals and leaves squared magnitudes
unchanged. `omega0=6` is a useful starting value: higher R narrows the k window,
and larger omega0 improves relative R resolution at the cost of k localization.
The Morlet background is described by
[Torrence and Compo (1998), §3](https://paos.colorado.edu/research/wavelets/bams_79_01_0061.pdf);
the discrete quadrature, zero-mean correction and unit-row normalization here are
explicit project choices. Finite support truncates edge windows; no cone-of-
influence significance test, deconvolution, or independent-observation correction
is inferred. This is not an assertion of EVAX wavelet normalization equivalence.

Total F sums dataset objectives and structural energies. Coordinate proposals
choose one movable atom and add independent uniform Cartesian components in
±`step_size` Å. Hard violations reject the whole proposal without clipping or
resampling. Uphill moves have acceptance probability `exp[−ΔF/(2T)]`, where T is
a dimensionless numerical tolerance; zero selects greedy descent. The general
RMC approach follows [McGreevy and Pusztai (1988)](https://doi.org/10.1080/08927028808080958).
These normalization and weighting choices are specific to rexafs. Transformed
bins are correlated, and the resulting objective is not reduced chi-square or
a calibrated posterior. Combining multiple spaces for the same measured data
can count the same information repeatedly.

## Chemical rules and energies

`Constraints` adds serializable rules to the global session minimum distance
and spherical displacement limit:

| Rule | Effect |
| --- | --- |
| `PairDistance` | An unordered element-pair minimum in Å, overriding the global fallback for that pair; includes periodic self images. |
| `ElementDisplacement` | Element-specific maximum displacement in Å from the original unwrapped coordinates; overrides the global displacement bound. |
| `BondRestraint` | A labeled atom pair in one component, optional hard distance range, and λ(r−r₀)². r and r₀ are in Å; λ is in objective units per Å². Periodic bonds use the nearest image. |
| `CoordinationRestraint` | Strength × (n−target)² for the count n of a chosen neighbor element inside an exclusive cutoff sphere in Å. |
| `LennardJones` | Pair energy `4ε[(σ/r)^12−(σ/r)^6]−V(cutoff)` for r below the cutoff; zero above it. σ is in Å; ε is in objective units. |

Bond and coordination penalties sum over listed restraints. Pair energies count
each pair once, including half-weighted directed periodic-image pairs, and sum
over structures without mixture weighting. Attractive energies can make F
negative; Metropolis uses differences and remains defined. These are numerical
regularizers, not a supplied force field or thermodynamic temperature mapping.
There is no Sutton–Chen model or EVAX-specific path-distribution restraint here.
Initial states must satisfy every hard rule. Invalid settings fail before any
scattering request, and rejected proposals skip scattering.

## Checkpoints, failures and analysis

`RmcCheckpoint` contains the original problem and displacement references,
settings, normalized fractions, initial/current/best states, cached component
spectra, ChaCha8 RNG state, counters, bounded history and coordinate frames.
`save_checkpoint` writes a temporary sibling, flushes it, and renames it over
the target. `load_checkpoint` or `resume` validates its schema, backend identity,
topology and constraints and recomputes stored states, requiring exact agreement.
Keep the same backend build, locked dependencies, platform and thread settings;
this is not a cross-platform bitwise reproducibility promise. ReFEFF identity
fingerprints default settings and pinned references; the checkpoint stores each
dataset's overrides. Custom calculators must include all scientific settings in
`identity()` and be deterministic.

A failed step commits neither its RNG draws nor proposed coordinates. `best()`
and `current()` remain available; save the checkpoint after catching the error,
then retry with an equivalent calculator. Restore rebuilds transient transform
plans and scattering caches, so a cold restart may incur setup cost but does not
reset the objective. History defaults to the last 10,000 completed attempts;
trajectory sampling is off by default, with capacity 1,000 when enabled. Frames
represent accepted state after the attempt, including repeated frames on rejection.

`distance_distribution` computes neighbor counts per selected absorber in
user-supplied Å bins, coordination within the selected histogram range, mean
distance and population variance in Å². Periodic images are explicit. This is a
count histogram, **not density-normalized g(r)**. `retain_paths=true` retains
backend path index, leg count, degeneracy, half length and χ contribution per
dataset/component/absorber. Contributions already include degeneracy; average
absorbers, multiply mixture fractions and apply S₀² to reconstruct the model.
Reporting paths increases storage and artifact parsing cost. Plain XYZ export
omits cell metadata; preserve JSON alongside coordinate trajectories.

## Evolutionary search

`EvolutionSession::new(&problem, &session_settings, &evolution_settings, &mut calc)`
initializes complete mixture individuals with constrained mutations. `step`
selects parents by tournaments with replacement, performs atom-wise uniform
crossover and convex weight crossover, mutates movable atoms and enabled weights,
and carries the best elite individuals forward unchanged. A fixed atom always
retains its input position. Exhausted hard-constraint attempts retain the selected
parent, and generation diagnostics count those fallbacks.

Mean pairwise RMS distance across movable atom displacement vectors and scaled
fraction differences measures diversity. Coordinates stay in their unwrapped
input frame; no alignment is applied. With M movable atoms and S structures,
its pair metric is `sqrt[(Σ_m |x_m−y_m|² + Σ_s [L(w_s−v_s)]²)/(M+S)]`, where
L=`weight_distance_scale` in Å. Low diversity or score stagnation increases
mutation amplitude by `hypermutation_factor`; this is a project-specific control,
not a reproduction of EVAX selection mathematics. The population is distinct
from a weighted mixture and is never spectrally averaged. Elitism preserves the
best score. A whole generation commits transactionally; serializable
`EvolutionCheckpoint` supports the same validated, exact-resume approach.

## Validation scope

```sh
cargo test --locked -p rexafs --test rmc --test rmc_session
cargo test --release --locked -p rexafs --features refeff-runner \
  --test rmc_refeff --test rmc_acceleration
cargo run --release --locked -p rexafs --features refeff-runner \
  --example rmc_benchmark -- 12 > benchmark.json
```

The tests cover deterministic continuation, injected failures, weighted recovery
without scattering calls, constraints and periodic distributions, existing FFT
agreement, wavelet frequency localization, evolutionary elitism, per-dataset
settings, cache correctness, actual ReFEFF path sums and pinned-reference replay.
The [performance record](rmc-performance.md) states measured boundaries and
approximation errors. Further qualification on larger, diverse structures and
experimental data remains necessary. No PDF/Bragg fitting, RMCProfile adapter,
EVAX job compatibility, uncertainty quantification or desktop workflow is added.

## Prepared paths and new workflows (0.2.10)

The [prepared-path guide](rmc-acceleration.md) documents ReFEFF 0.4.0 contexts,
exact affected-path updates, optional representative tables and controlled moments,
population batches, cooling and collective proposals, calibration, local Fourier
maps, structural restraints, and streaming trajectories. The
[work record](rmc-implementation-progress.md) separates measurements from remaining
scientific qualification. Existing examples below that cite older dependencies
remain historical; backend upgrades require a new checkpoint identity.
