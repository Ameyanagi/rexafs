# RMC extension validation and performance

Validated on 2026-09-16, on the unreleased `feature/rmc-refeff` extension following
commit `f587cfd`. This record concerns the Rust API and examples. It supersedes
the eight requested capability gaps in the [historical audit](rmc-evax-gap-analysis.md);
it does not supersede the archived measurements in [the first validation](rmc-validation.md).

## Implemented scope

| Requested capability | Implementation and evidence |
| --- | --- |
| Faster scattering updates | Bounded local-input spectrum cache; pinned reference potential/phase artifacts with ReFEFF geometry/path updates; unchanged mixture component reuse; no scattering calls for weight-only moves. Real full/cache/approximation comparisons below. |
| Richer constraints | Element-pair exclusion, element displacement bounds, labeled bond bounds and harmonic penalties, coordination penalties, shifted Lennard–Jones energies. Finite/periodic analytic checks and hard-rejection call counts. |
| R/q/wavelet fitting | Existing rexafs Fourier transforms on whitened residuals, plus prepared scale-dependent Morlet kernels. R/q results checked against the existing transform API; wavelet localization checked with a known sinusoid. |
| Checkpoint/resume | Versioned settings, original positions, accepted/best states, component spectra and ChaCha8 state; atomic file replacement; exact recalculation on restore; unchanged state/RNG on injected failure. Split and uninterrupted runs match exactly. |
| Evolutionary search | Population, tournament selection, uniform atom crossover, weight crossover, elitism, bounded mutations, diversity/stagnation triggers and transactional generation checkpoints. Deterministic resume and failure recovery tested. |
| Weighted structures | Normalized mixture fractions, per-component absorber lists and optional weight-transfer moves. Synthetic 0.7/0.3 mixture recovered within 0.002 absolute fraction, with no scattering calls after initialization. |
| Dataset calculator settings | Independent ReFEFF options per dataset, including polarization. Real spectra and cache counters verify that polarization changes cannot use the wrong cached result. |
| Structural analysis | Sampled coordinate trajectories; periodic neighbor histograms, coordination, mean and variance; actual ReFEFF path contributions. Simple-cubic six-neighbor count and triangle single/multiple-scattering path sums tested. |

These are implementations of the requested capability categories, not a port of
EVAX's algorithms. They do not include EVAX's interpolation tables, Sutton–Chen
energies, every EVAX path-distribution restraint, PDF/Bragg fitting, or either
program's job format. Large experimental systems remain unqualified.

## Benchmark design

Reproduce with:

```sh
cargo run --release --locked -p rexafs --features refeff-runner \
  --example rmc_benchmark -- 12 > benchmark.json
```

Hardware: Apple M4 (`Mac16,12`), 32 GiB RAM, macOS 26.5.1 (25F80), arm64.
Compiler: Rust 1.98.1 (`48a229cea`, 2026-09-01), Cargo release profile. Workspace
package version: 0.2.9 with unreleased additions. Locked ReFEFF facade: 0.3.0;
locked engine/core/I/O crates: 0.2.0. ReFEFF uses one worker. No other builds or
tests from this task run during the timed trial evaluations. Compilation is
excluded; validation, local-cluster construction, pipeline execution, spectrum
interpolation and objective assembly are included.

The input contains two isolated equilateral Cu triangles with 2.5 Å sides,
separated by 20 Å, and four selected absorbers: atom indices 0, 1, 3 and 4. The
cluster/path radii are both 4 Å, maximum path order is three legs, both screening
thresholds are zero, and kmax is 12 Å⁻¹. Each evaluated spectrum has 121 measured
points from 3 to 9 Å⁻¹. Only atom 1 changes. Trials use prescribed signed x
displacements, from ±0.003 Å up to −0.030/+0.033 Å, returning to the reference
on trials 6 and 12. The exact sequence is implemented in the example.

Every mode evaluates **the same 12 trial geometries** in the same order. This
measures forward trial-evaluation throughput, not optimizer convergence speed
or accepted moves per second. Each mode starts with empty calculator caches;
"setup" times its initial evaluation, not a cold operating-system disk cache.
The uncached mode computes every absorber afresh. Exact caching shares identical
local environments across the isolated clusters, skips unaffected absorbers and
reuses reference-return trials. The pinned mode additionally reuses reference
potentials and phases for changed environments. All three run with path reporting
disabled. Modes run in fixed order; OS caching and CPU scheduling can affect times.

Raw inputs, timings, counters and diagnostics are retained in
[run 1](validation/2026-09-16-rmc/benchmark-1.json),
[run 2](validation/2026-09-16-rmc/benchmark-2.json), and
[run 3](validation/2026-09-16-rmc/benchmark-3.json).

Median of three runs (seconds; calculator-cache setup excluded from trial time):

| Mode | Setup | 12 trials | Trial-time range | Trials/s | Setup + trials |
| --- | ---: | ---: | ---: | ---: | ---: |
| Full uncached | 1.637 | 19.912 | 19.641–20.023 | 0.60 | 21.478 |
| Exact local cache | 0.826 | 8.240 | 8.154–8.359 | 1.46 | 9.083 |
| Pinned potentials | 1.635 | 0.167 | 0.157–0.177 | 71.94 | 1.802 |

Using ratios of median elapsed times, exact caching is **2.42× faster**
for trial evaluation and **2.36×** including setup. Pinned potentials are
**119.4× faster** for trial evaluation and **11.9×** including setup.
These measured ratios apply to this benchmark, not arbitrary RMC jobs.

The exact-cache spectra agree **bit for bit** with the uncached spectra on this
input. Pinned-potential spectra have aggregate relative L2 error
`||χ_pinned−χ_full||₂/||χ_full||₂ = 0.0007391644`, or **0.0739%**, across all trial
points. RMSE is `9.95445×10⁻⁶` and maximum absolute χ difference is
`3.65477×10⁻⁵`; χ is dimensionless. These are differences between two calculation
modes, not errors against experiment. The reference geometry itself matches
exactly, while changed geometry intentionally retains the reference electronic
potentials. Displacements, chemistry, cluster boundaries and SCF choice can
produce larger approximation errors.

Call counts are deterministic across repetitions:

| Mode | Full pipelines including setup | Path-only pipelines | Spectrum-cache hits |
| --- | ---: | ---: | ---: |
| Full uncached | 52 | 0 | 0 |
| Exact local cache | 22 | 0 | 30 |
| Pinned potentials | 4 | 20 | 28 |

The pinned run retains about 1.84 MB of reference artifact/key payload and
0.11 MB of native-spectrum/key payload. These counts are not total process memory.
Cache capacities and workload reuse determine actual savings. Many distinct
absorbers, complex high-order paths, larger clusters or cache eviction can reduce
the speedup. The faster mode does not establish equivalence to EVAX or the full
potential calculation at arbitrary geometries.

## End-to-end session example

The `rmc_session` example generated synthetic Cu-dimer χ at 2.5 Å, started at
2.65 Å, and ran 64 coordinate attempts with seed 42, zero Metropolis tolerance,
0.06 Å maximum per-axis step, and pinned reference potentials. It requested path
reports and a trajectory every four attempts, saved a checkpoint after attempt
16, reloaded it, and completed the run.

The score fell from `1.0236734297723815` to `2.0070912471427172×10⁻⁵`. The best
separation was `2.5005052387366935 Å`; 7 moves were accepted and 17 frames retained.
It used one full reference pipeline and 65 path updates, with three spectrum-cache
hits including checkpoint verification. The pinned target and refinement share
the same model, so this is a deterministic software demonstration rather than an
independent physical validation. The input and summary are retained in
[session-example.json](validation/2026-09-16-rmc/session-example.json).

## Verification

The complete core suite passes **369 tests**, with **3 existing ignored tests**.
The focused release ReFEFF tests validate changed/restored geometry, multiple
scattering, exact local caching, polarization separation, frozen-reference replay
and path decomposition. Analytic session tests check exact JSON resume, failure
rollback, best-state preservation, mixture recovery, constraint energies,
periodic distributions, Fourier agreement and wavelet localization.

Checks used:

```sh
cargo fmt --all -- --check
cargo clippy --locked -p rexafs --all-targets -- -D warnings
cargo clippy --locked -p rexafs --all-targets --features refeff-runner -- -D warnings
cargo test --locked -p rexafs
cargo test --release --locked -p rexafs --features refeff-runner \
  --test rmc_refeff --test rmc_acceleration --test rmc_session
cargo test --release --locked -p rexafs --features refeff-runner --lib rmc::
RUSTDOCFLAGS='-D missing_docs -D rustdoc::broken_intra_doc_links' \
  cargo doc --locked -p rexafs --no-deps --features refeff-runner
uv run --no-project --python 3.12 scripts/check-release-version.py
git diff --check
```

The existing `binrw 0.12.1` future-compatibility warning remains. No desktop or
language-binding interface was changed. The [guide](rmc.md) describes scientific
conventions, API defaults, approximation limits and checkpoint requirements.
