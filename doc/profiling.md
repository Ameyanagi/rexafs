# Historical profiling of normalization, AUTOBK, and Fourier transforms

For the published packages, see the [2026-09-06 matched Larch/rexafs benchmark
and profiling report](benchmarks/2026-09-06-larch/README.md). It retains 112 timing
and output comparisons plus Python and native CPU profiles for both packages.

The measurements below are historical records from the codename period; figures
retain their original profiler labels. They do not describe the performance or
default solver of the current release. Current algorithm choices are explained
in [fixed-penalty AUTOBK](autobk-fixed-penalty.md), and the current regression
commands are in the [Rust workflow](../.github/workflows/rust.yml).

Profiling was performed using the following command:

```bash
sudo cargo flamegraph --bench xas_group_benchmark_parallel
```

## Results

In these runs, AUTOBK took most of the processing time with either a numerical
or an analytical Jacobian. A Jacobian is the matrix of residual derivatives
with respect to fitted parameters. The analytical Jacobian was reported to give
roughly a three- to fourfold speedup over finite differences. These profiles
motivated investigating a direct linear solve, which was added later; the
reported ratio is not a benchmark of today's default solver.

![profile for numerical Jacobian](img/flamegraph_rexafs_numerical_optimization.svg)

![profile for analytical Jacobian](img/flamegraph_rexafs_analytical_optimization.svg)

## Appendix: Profiling of xraylarch

The Python/xraylarch normalization, AUTOBK and Fourier transform pipeline was
also profiled. AUTOBK was the bottleneck in that run, and the historical
single-core rexafs implementation with a numerical Jacobian had similar
performance to xraylarch.

![profile of xraylarch](img/flamegraph_xraylarch.svg)

## Baseline (Slice A, post-build-unblock)

Date: 2026-02-07

Environment:

- `rustc 1.93.0`
- `cargo 1.93.0`
- `Darwin 25.2.0 arm64`

Dependency state used for this baseline:

- Removed `fftconvolve` dependency path.
- Single FFT stack in runtime path: `easyfft 0.4.2`.
- `cargo tree -p rexafs | rg "easyfft|fftconvolve|anymap"` confirms no `fftconvolve` and only `anymap3` via `easyfft`.

Commands:

```bash
cargo test -p rexafs
/usr/bin/time -l cargo bench -p rexafs --bench xas_group_benchmark_single -- --noplot
/usr/bin/time -l cargo bench -p rexafs --bench xas_group_benchmark_parallel -- --noplot
```

Runtime metrics (Criterion sample p50/p95 from `target/criterion/*/new/sample.json`):

- `xas_group_benchmark_single`:
  - p50: `0.763605 s`
  - p95: `0.932267 s`
  - max RSS (`/usr/bin/time -l`): `261750784` bytes
- `xas_group_benchmark_parallel`:
  - p50: `8.569100 s`
  - p95: `9.034220 s`
  - max RSS (`/usr/bin/time -l`): `1363558400` bytes

Allocation count status:

- Captured with allocator-instrumented runner:
  - command: `cargo run -p rexafs --example alloc_baseline --release`
  - `xas_group_benchmark_single_alloc`:
    - `alloc_calls=4929638`
    - `dealloc_calls=4927810`
    - `realloc_calls=140016`
    - `alloc_bytes=1857394996`
    - `dealloc_bytes=1849003156`
  - `xas_group_benchmark_parallel_alloc`:
    - `alloc_calls=492960408`
    - `dealloc_calls=492780024`
    - `realloc_calls=14000032`
    - `alloc_bytes=185711339084`
    - `dealloc_bytes=184872996900`

Note:

- Allocator instrumentation materially increases runtime; use Criterion numbers above for runtime baselines.

## Slice C/D Follow-up Metrics (post-refactor)

Date: 2026-02-08

Commands:

```bash
cargo test -p rexafs
/usr/bin/time -l cargo bench -p rexafs --bench xas_group_benchmark_single -- --noplot
/usr/bin/time -l cargo bench -p rexafs --bench xas_group_benchmark_parallel -- --noplot
cargo run -p rexafs --example alloc_baseline --release
```

Runtime metrics:

- `xas_group_benchmark_single`:
  - median point estimate: `0.271351 s`
  - p95 sample time: `0.569398 s`
  - max RSS (`/usr/bin/time -l`): `812859392` bytes
  - vs Slice A baseline p50: `+64.47%` faster
- `xas_group_benchmark_parallel`:
  - median point estimate: `6.734452 s`
  - p95 sample time: `8.750525 s`
  - max RSS (`/usr/bin/time -l`): `1203322880` bytes
  - vs Slice A baseline p50: `+21.41%` faster

Allocation counters (allocator-instrumented runner):

- `xas_group_benchmark_single_alloc`:
  - `alloc_calls=4927039`
  - `dealloc_calls=4925211`
  - `realloc_calls=139716`
  - `alloc_bytes=1847362572`
  - `dealloc_bytes=1838970732`
- `xas_group_benchmark_parallel_alloc`:
  - `alloc_calls=492700407`
  - `dealloc_calls=492520024`
  - `realloc_calls=13970032`
  - `alloc_bytes=184708089048`
  - `dealloc_bytes=183869746912`

## Slice E Follow-up Metrics (AUTOBK direct solver rollout)

Date: 2026-02-08

Commands:

```bash
cargo test -p rexafs
cargo bench -p rexafs --bench autobk_stage_benchmark -- --noplot
cargo bench -p rexafs --bench xas_group_benchmark_single -- --noplot
cargo bench -p rexafs --bench xas_group_benchmark_parallel -- --noplot
bash crates/rexafs/scripts/bench_regression_gate.sh informational
```

Runtime metrics (Criterion median point estimate):

- `autobk_stage_legacy_lm`: `2.772355 ms`
- `autobk_stage_linear_direct`: `2.676096 ms`
  - vs legacy stage on same run: `+3.47%` faster
- `xas_group_benchmark_single`: `0.252894 s`
  - vs Slice C/D median `0.271351 s`: `+6.80%` faster
- `xas_group_benchmark_parallel`: `6.546560 s`
  - vs Slice C/D median `6.734452 s`: `+2.79%` faster

Notes:

- AUTOBK now supports runtime solver selection:
  - `AUTOBKSolver::LinearDirect` (default)
  - `AUTOBKSolver::LegacyLm`
- Direct solver guards ill-conditioned systems and can fall back to LM automatically (`linear_fallback_to_lm = true` by default).
- Regression baseline now includes AUTOBK stage benchmarks in `crates/rexafs/benchmarks/baseline.json`.

## Benchmark Methodology Update (Matched Workload)

Date: 2026-02-08

To ensure fair sequential vs parallel comparison:

- Legacy IDs are preserved for tooling compatibility:
  - `xas_group_benchmark_single` runs `10_000` spectra.
  - `xas_group_benchmark_parallel` runs `10_000` spectra.
- Additional matched-size groups are included:
  - `xas_group_seq_matched/{100,10000}`
  - `xas_group_par_matched/{100,10000}`
- Both use the same source spectrum file and identical group construction.
- Criterion throughput is set with `Throughput::Elements(n_spectra as u64)`.

Use these commands for matched comparisons:

```bash
cargo bench -p rexafs --bench xas_group_benchmark_single -- --nocapture
cargo bench -p rexafs --bench xas_group_benchmark_parallel -- --nocapture
```

Derived metrics:

- Per-spectrum latency: `benchmark_time / n_spectra`
- Spectra per second: `n_spectra / benchmark_time_seconds`

## Slice F Matched Workload Results

Date: 2026-02-08

Commands:

```bash
cargo bench -p rexafs --bench xas_group_benchmark_single -- --nocapture
cargo bench -p rexafs --bench xas_group_benchmark_parallel -- --nocapture
bash crates/rexafs/scripts/bench_regression_gate.sh informational
```

Runtime metrics (Criterion time interval):

- Legacy IDs (both now run `10_000` spectra):
  - `xas_group_benchmark_single`: `[1.5028 s 1.5133 s 1.5279 s]`
  - `xas_group_benchmark_parallel`: `[361.67 ms 372.48 ms 385.35 ms]`
  - speedup (parallel vs sequential, center estimate): `~4.06x`
- Matched group `100`:
  - `xas_group_seq_matched/100`: `[15.121 ms 15.292 ms 15.477 ms]`
  - `xas_group_par_matched/100`: `[4.0073 ms 4.0944 ms 4.2582 ms]`
  - speedup (parallel vs sequential, center estimate): `~3.74x`
- Matched group `10_000`:
  - `xas_group_seq_matched/10000`: `[1.5405 s 1.5497 s 1.5581 s]`
  - `xas_group_par_matched/10000`: `[373.65 ms 382.14 ms 390.36 ms]`
  - speedup (parallel vs sequential, center estimate): `~4.05x`

Conclusion:

- After fixing benchmark methodology, parallel processing is consistently faster than sequential on both matched workloads.
- Regression gate is green in informational mode with the updated Slice F baseline.
