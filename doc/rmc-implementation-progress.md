# RMC implementation work record

Started 2026-09-17 on `feature/rmc-refeff`; unreleased. The user authorized all
recommended additions from the EVAX audit, while leaving explicitly deferred
workflows for later. ReFEFF remains the primary calculator. Interfaces are Rust
APIs and examples; desktop and language bindings are outside this work.

The source-based proposal is retained locally at
`/Users/ryuichi/dev/evax-analysis/EVAX_IMPLEMENTATION_PROPOSAL.md`, with the complete
EVAX file inventory and checksums beside it. No EVAX source is incorporated.

## Accepted scope and completion evidence

Checked items have implementation and regression coverage. These are API completion
checks, not claims of experimental convergence, EVAX equivalence, or universal
performance. Conditional approximations remain opt-in.

- [x] Stage and wrapper profiling, path/work counts, reproducible geometry benchmarks.
- [x] Prepared ReFEFF electronic/scattering context and explicit-path reference evaluation.
- [x] Stable periodic atom/path identities, catalogue validity limits and affected-path updates.
- [x] Shared representative scattering basis with fixed scientific identity, error diagnostics and reference fallback.
- [x] Optional controlled moment expansion with direct-sum validation.
- [x] Bounded parallel candidate/path evaluation and deterministic reductions.
- [x] Cooling, per-element and collective proposal policies, stopping diagnostics and exact resume state.
- [x] Reproducible amplitude/energy calibration workflow and normalized fit reports.
- [x] Wavelet masks, equivalent fast wavelet evaluation and an explicit STFT option.
- [x] Path/angle/displacement structural reports and opt-in MSD/path-distribution restraints.
- [x] Efficient population batches using the prepared calculator.
- [x] Seeded initial disorder/dopant helpers, supercell/resource estimates and streaming trajectory spectra.
- [x] Rust API documentation, runnable examples and meaningful regression checks.
- [x] Cu₂O end-to-end accuracy/performance comparison against the retained reference.

## Scope correction after the R-space EA benchmark (2026-09-17)

Historical audit status before the follow-up implementation below. Preserve this
correction when interpreting the earlier completion checklist.

The checklist above records implemented API categories, not complete EVAX
algorithm parity. In particular, the representative-basis item is **partial**:
it provides immutable tables, geometric guards and explicit reference diagnostics.
Adaptive geometric clustering, error-driven splitting, scheduled basis refresh
and automatic spectral-error control remain unimplemented. Treating that item
as completion of the proposal's full adaptive-basis recommendation was too broad.

The current `EvolutionSession` also does not combine selection/crossover with
EVAX's per-state Monte Carlo acceptance stage. Its standalone population policy
was benchmarked on Cu₂O; that experiment does not benchmark EVAX's full method.
Acceptance-driven step adjustment is another missing policy: the local EVAX source
contains an active branch in `calculation.cpp` around line 1385, despite commented
calls elsewhere. Existing REXAFS predetermined cooling remains available.
The local detailed research update is
`/Users/ryuichi/dev/evax-analysis/EVAX_REMAINING_GAPS.md`.

## Follow-up implementation after the remaining-gap audit

The [search and caching update](rmc-search-upgrade.md) now implements optional
hybrid EA–RMC through the shared Metropolis kernel, checkpointed acceptance
feedback, population geometry caches and unambiguous active-path counters.
It also adds explicit adaptive training stages with measured path/total errors,
splitting, conservative exact fallback and transactional optimizer rebasing, plus
four-parameter first-shell calibration through the ordinary native fitter.
These are implemented APIs with synthetic regression coverage; adaptive Cu₂O
accuracy and hybrid fit quality still require independent experimental qualification.
The prior R-space fit results remain historical and are not overwritten.

A one-absorber Cu₂O parent/child replay measured 7.9146 s with one snapshot versus
0.3508 s with 25 snapshots; all 240 spectra were bitwise identical. Exact path
calls fell from 12,619 to 499. This is a cache replay benchmark, not a whole-EA
speedup. Its settings, limitations and raw evidence are linked from the update.

## Deferred by user scope

GULP force-field fitting/execution, one-dimensional path refinement,
experimental amplitude/phase overrides, INVERT and specialized many-body force
fields, exact EVAX optimizer/score compatibility, and PDF/Bragg or
composition-changing refinement remain separate work. No automatic adaptive
experimental-data weighting is planned.

## Baseline

The pre-change RMC source revision is `c1a30d0a2571e5cdf0bc586ef08f07f59502676b`.
Existing uncommitted Cu₂O example/docs are preserved. The baseline used ReFEFF facade
0.3.0 with engine/core/I/O 0.2.0. This implementation was updated to the current
ReFEFF 0.4.0 release and component crates 0.3.0 (registry/release checked
2026-09-17). Scientific calculator identities include this dependency change. Use the shared build directory
`/Users/ryuichi/dev/rexafs/target`.

The archived 48-atom Cu₂O run took 1,142.435 s for 1,000 attempts. An independent
warm pinned-potential evaluation of its best structure took 1.019 s. These are
previous measurements, not fresh benchmarks or comparisons with original EVAX.

## Implementation and verification

The [prepared-path guide](rmc-acceleration.md) documents the APIs and scientific
conventions. New implementations reside under `crates/rexafs/src/xafs/rmc/`:

| Area | Source and validation |
| --- | --- |
| Typed ReFEFF and stable paths | `prepared.rs`, `paths.rs`, `accelerated.rs`; Cu₂O family multiplicities, central revisits, polarized pair and Cu multiple scattering compared to the file pipeline. |
| Local caching and bounded batches | `accelerated.rs`, `session.rs`, `evolution.rs`; rejected-trial restoration, cache clearing, deterministic ordered batches, generation rollback, cold-checkpoint restart and cancellation on cache hits. |
| Basis and controlled moments | Fixed representative identity, geometric fallback, path reports and direct comparisons; moment truncation/fallback and path-report mode transitions tested. |
| Optimizer and fitting | `proposals.rs`, `calibration.rs`; cooling/collective moves, stopping without retained history, exact continuation and synthetic S₀²/ΔE₀ recovery. |
| Fourier/local maps | `objective.rs`, `local_spectrum.rs`; direct/FFT complex agreement including boundaries/masks, irregular-grid rejection/fallback, and additive path R/q/maps. |
| Structure and workflows | `structural.rs`, `constraints.rs`, `workflows.rs`; arithmetic MSD, explicit images/histogram priors, seeded preparation, resource counts, truncated/topology-changing XYZ rejection and early streaming completion. |

`cargo test --release --locked -p rexafs --features refeff-runner` passed
**398 tests across 26 result groups**, with three existing ignored tests and no
failures. The ignored tests require external Larch or pymatgen; none
is a new RMC test. Raw log: local `output/rmc-full-tests-2026-09-17.log` in the
research workspace. Default-backend/Clippy/Rustdoc results are recorded below.

The self-contained `rmc_prepared` example ran 80 attempts, recovered synthetic
S₀²=0.85 and ΔE₀=2 eV, and reduced its objective from 0.016840 to 0.000029. It
successfully resumed a serialized checkpoint with a new cold calculator and
emitted structural/path reports, a local map and streamed trajectory scores.
Its input is a synthetic dimer, not experimental evidence.

## Experimental Cu₂O validation on ReFEFF 0.4.0

The retained local Athena `Cu oxides.prj` / `cu2o_abs` measurement and preprocessing
are described in the [Cu₂O record](rmc-cu2o-demo.md). New runs use 48 explicit atoms,
32 Cu absorbers, the 2×2×2 periodic cell, 4.5 Å path half-length, four legs, zero
path criteria, 0.4 Å displacement envelope and the original seed 20260916. The
numerical objective is k²-weighted with σ=1, fixed S₀²=0.9 and ΔE₀=10.3353009643 eV.
These remain illustrative settings rather than a calibrated physical protocol.

Hardware: Apple M4 (Mac16,12), 32 GiB, macOS 26.5.1, Rust 1.98.1 aarch64 release.
The exact prepared calculator uses four outer workers; pipeline ReFEFF uses one
worker. Timings exclude compilation, include optimizer/checkpoint work in the
sampling interval, and retain setup separately.

For **200 attempts using exact affected-path updates**:

- Setup: **17.588 s**; sampling: **20.881 s**; total recorded run: **38.469 s**.
- 144 accepted, including 16 uphill; best and last score **0.1051724804**, from
  **4.2833769712** initially.
- The matched pinned-pipeline run accepted the **same 200-step Boolean trace**
  and produced **identical final/best coordinates**, with score **0.1051650701**.
- Best-spectrum k²-weighted relative L2 difference: **0.00690694%**.
- 32 prepared contexts; 80,768 reserved paths; 57,957 exact path evaluations and
  14,644,430 unchanged catalogue entries reused over 6,432 absorber requests.
- `/usr/bin/time -l` maximum resident set: **172,703,744 bytes (164.70 MiB)**;
  retained numerical snapshot payload: **8,566,952 bytes**. These are distinct
  measures; immutable contexts and process overhead are not counted as cache bytes.

The 200-step pipeline control overlapped unrelated build work and took 417 s;
**do not use that interval for a speed ratio**. A separate consecutive 20-step
comparison without our compilation running is recorded below. Neither comparison
measures original EVAX or proves asymptotic speed across structures.

### Approximation accuracy that did not pass

A separate 200-attempt run allowed frozen-table reuse for leg changes ≤0.1 Å and
angle changes ≤0.2 rad. It was fast (20.455 s setup, about 9.9 s sampling), but its
best spectrum differed from direct typed scattering by **9.274% in k²-weighted
relative L2**, or 3.110% unweighted. This fails the proposal's illustrative 0.5%
weighted target. Its approximate objective was 0.130280; direct typed scattering
gave 0.117415 and the pinned pipeline 0.117419. A fully refreshed electronic
calculation gave 0.115742. Fixed electronic potentials themselves introduced a
6.355% weighted spectral difference on that particular structure.

Consequently **Exact remains the recommended/default path model**. The frozen
basis, moments, error reports and geometric fallback are implemented but require
configuration-specific validation; a loose geometric guard is not an accuracy
certificate. No claim is made that the approximation preserves acceptance.

All previous one-absorber benchmark attempts and the earlier 1,000-step run are
preserved. Early benchmark files with omitted central revisits or uncanonicalized
path direction are development records and must not be treated as qualified
results. Tests now cover both corrections.

### Retained evidence and qualification limits

Local raw evidence is under `/Users/ryuichi/dev/evax-analysis/output/`:
`cu2o-exact-run-2026-09-17`, `cu2o-pipeline-control-2026-09-17`,
`cu2o-exact-comparison-2026-09-17.json`, `cu2o-prepared-run-2026-09-17`,
`cu2o-prepared-check-2026-09-17`, and `rmc-prepared-api-demo-2026-09-17`.
The user measurement and large generated trajectories are not copied into the
source repository. Jobs, checkpoints, options, spectra, counters and derived
plots remain local and reviewable.

This completes the recommended Rust mechanisms, not the proposal's full research
qualification campaign. Multiple-seed time-to-quality experiments, larger-cell
convergence, independently validated physical priors and broad material/edge
coverage remain scientific validation work. No eight-leg qualification, automatic
history-dependent basis adaptation or unvalidated importance pruning is asserted.
Geometric guards and explicit new-calculator identities supply controlled basis
changes; all directed paths within the declared catalogue are retained.

## Matched timing and output checks

The consecutive **20-attempt** timing jobs used identical measurement, initial
geometry, proposal seed, constraints and unscreened path settings. There was no
concurrent agent compilation during either timed job. These are single observations,
not estimates with confidence intervals.

| Interval | Pinned pipeline | Exact affected paths (4 workers) |
| --- | ---: | ---: |
| Setup | 16.699 s | 16.680 s |
| Sampling, 20 attempts | 17.314 s | 1.697 s |
| Total | 34.013 s | 18.377 s |

This is **10.20× faster sampling**, or **1.85× including setup** for this short
run. The speedup includes outer parallelism as well as avoided whole-pipeline
and unaffected-path work. It is not a comparison with original EVAX.
Raw evidence: `cu2o-matched-timing-2026-09-17.json` and the corresponding
`cu2o-timing-{pipeline,exact}-2026-09-17` directories in the research workspace.

The 200-attempt exact run's k/R plots were generated and visually inspected.
NumPy and Rust Fourier arrays agree to the plotting script's 1e−10 absolute
threshold. Its experimental-normalized squared k² residual is **0.503064**;
the 97.54% improvement from the starting crystal does not mean the experiment
is already reproduced well or the run is converged. The best state is written
as JSON and XYZ alongside the last accepted state and coordinate trajectory.

On this exact-run best geometry, fresh electronic potentials gave score
**0.10484209**, compared with **0.10516507**
for the pinned pipeline. Their k²-weighted relative spectral difference was
**2.345%**. This quantifies the fixed-electronic-reference limitation separately
from the much smaller typed-path/pipeline assembly difference.

## Final build checks and transform measurements

- Strict Clippy passed for all targets, both with and without `refeff-runner`.
- The default-feature RMC subset passed all 18 tests, including the final proposal
  endpoint fix, streaming validation and complex path transforms.
- Rustdoc regenerated successfully with missing documentation and broken internal
  links treated as errors. Local API entry: shared `target/doc/rexafs/xafs/rmc/index.html`.
- Workspace formatting, diff whitespace, release-version consistency and the new
  Markdown relative links passed. No released website pages were regenerated or
  published; the API additions remain explicitly unreleased.

The synthetic transform benchmark used 191 k points, 191 centers, 48 R values
and 100 repeated Morlet maps. Direct/FFT complex maps differed by at most
**6.02×10⁻¹⁴**. The moment workload used 4,000 lengths narrowly distributed around
2.5 Å and an explicit 10⁻⁶ absolute tolerance: measured error was **9.71×10⁻⁹**,
with no direct fallbacks. These examples illustrate useful workloads, not a
universal acceleration guarantee; preparation and approximation tolerance matter.
Raw dimensions, settings, timing boundaries and errors are retained in
`output/rmc-transform-benchmark-2026-09-17.json` in the research workspace.

For that single local-map workload, 100 direct maps took **0.076653 s**
and 100 FFT maps took **0.014051 s**, after preparation.
Setup took 0.022117 s and 0.011020 s respectively.
Moment preparation took 0.000043 s; one 191-point direct sum
took 0.002420 s and the prepared expansion took 0.000006 s.

All 12 repository tooling test suites also passed (`scripts/check-tooling.py`).

## Automatic audit controller and dev merge, 2026-09-17

The feature branch now includes dev head `bb83016` through merge commit `765869b`.
The preceding validated RMC implementation was committed as `ccf31f0`.
The [automatic audit guide](rmc-adaptive-audits.md) documents periodic native-
objective checks, error-triggered retraining, exact fallback, transactional
rescoring and combined checkpoints. Already prepared electronic contexts and
catalogues are shared across stages; geometry spectra and trained tables remain
specific to their scientific calculator identity.

The merged tree passed 253 release checks: 208 library, 9 basic RMC, 19 session,
8 path, 4 transform and 5 native Larch fit-space parity tests. The default-feature
session suite separately passed 19 checks. Six new controller tests cover both
optimizer types, cold resume, scheduling, measured errors, weight normalization,
shared electronic preparation, cancellation and training failures.

The release `rmc_adaptive` example completed 30 synthetic dimer attempts with a
cold midpoint resume. It refreshed at attempts 0, 5, 10 and 20, passed unchanged
at 15 and 25, and switched to exact paths on the final audit at 30. The final
stage reused one electronic context and performed zero fresh electronic
preparations. Its final score was 3.2288741453e-8; this is an API fixture, not an
experimental material fit or structural convergence evidence.

The `rmc_cu2o_calibrate` example retains ordinary ReFEFF path output and native
four-parameter first-shell fits for three ranges/two initial guesses. The Cu₂O
search example accepts an explicit R_MIN and defaults to 1.15 Å, above its saved
AUTOBK radius of 1 Å. Archived 0.8–4 Å jobs remain unchanged and reproducible with
an explicit R_MIN=0.8. New matched three-seed exact-path comparisons and raw
residual vectors live in the research workspace under
`output/cu2o-above-rbkg-comparison-2026-09-17`; the scientific report is
`CU2O_ABOVE_RBKG_COMPARISON.md` there. Their residuals are not directly comparable
to the old range/calibration.

Clippy with warnings denied and all targets, workspace formatting/whitespace,
and Rust reference generation (including strict Next documentation/link checks)
passed. The generated references remain local; no site was published. These
checks do not yet qualify approximate-basis Cu₂O performance, optional weak-path
screening, higher scattering orders or previously deferred workflows.
