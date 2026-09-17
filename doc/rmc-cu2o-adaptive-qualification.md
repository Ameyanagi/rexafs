# Cu₂O Spectrum-input and adaptive ReFEFF qualification

Measured 2026-09-17 on the unreleased `feature/rmc-refeff` branch. This is a
fixed-potential numerical qualification of REXAFS, not an EVAX speed comparison
or a validation of the physical approximations in ReFEFF.

## Decision

Keep **exact prepared ReFEFF as the recommended calculator for this Cu₂O workflow**.
The initial adaptive basis failed all six held-out spectral checks. All three
adaptive-start searches eventually selected exact fallback (at attempts 100,
100 and 500); most subsequent work was exact. Thus this experiment does not
qualify sustained adaptive acceleration. Final retained states pass by exact
evaluation, not by keeping an inaccurate approximation.

Median best exact residuals were 0.02668092 for exact-start
and 0.02732553 for adaptive-start runs. With three seeds
and a shared host, their difference is descriptive. It does not establish general
optimizer superiority. Raw attempt throughput also includes different paths
through coordinate space after the adaptive histories diverge.

**No run established the declared residual plateau.** These are time-limited optimization results.

The immediate usable result is the Spectrum input API with exact caching,
preprocessing provenance and explicit convergence reporting. Further adaptive
work should target the held-out geometry errors and basis lookup/training cost,
then repeat the same accuracy gates and full-budget experiment. Increasing the
error tolerance solely to obtain a speedup would change this qualification.

## Accepted-fit regression

`RmcDataset::from_spectrum` reproduced the accepted normalized complex-R residual
**0.022605971322439** exactly. Experimental arrays, crop, noise scales, calibration,
absorber selection and the objective settings were identical. Independently
recalculating all six archived best configurations with exact typed paths also
reproduced every stored score with a difference of **0.0**.

The source `Spectrum` retains the available original-project energy/absorption,
normalization and AUTOBK state reconstructed from the archived experiment. The
old archive did not retain an FFT cache. No background subtraction was rerun for
this comparison. A Spectrum snapshot preserves the state it receives, including
available preprocessing fields; it cannot recover missing historical operations.

## Initial adaptive basis: accuracy did not qualify

The initial basis was trained on the ideal reference only. Six previously saved
RMC/hybrid best structures were excluded from that training. For each structure,
the approximate and exact calculations shared fixed electronic potentials and
path catalogues. The audit reused the native real-plus-imaginary R objective.

For a dataset, let P be its objective against a zero model, D be the objective of
the approximate-minus-exact spectrum, and Fa/Fe be its approximate/exact data-fit
scores. The spectral error is sqrt(D/P); score drift is abs(Fa−Fe)/P. Both are
dimensionless. The declared limits were 0.001 and 0.0001 respectively. These are
numerical engineering tolerances, not experimental uncertainties.

| Held-out configuration | Exact residual | Spectral error (%) | Normalized score drift | Passes both? |
| --- | ---: | ---: | ---: | --- |
| RMC 20260918 | 0.02260597 | 0.382594 | 0.0000626755 | No |
| RMC 20260919 | 0.02381615 | 0.281930 | 0.0001020874 | No |
| RMC 20260920 | 0.02412252 | 0.153467 | 0.0000739933 | No |
| Hybrid 20260918 | 0.05222655 | 0.306171 | 0.0001909607 | No |
| Hybrid 20260919 | 0.06719034 | 0.215602 | 0.0000019197 | No |
| Hybrid 20260920 | 0.05260689 | 0.244317 | 0.0003819849 | No |

All six exceeded the 0.1% spectral-error limit. Exact agreement on the training
reference alone therefore would have been misleading. The matched experiment
below evaluates the full controller, including retraining and exact fallback.

## Matched experiment

Three paired seeds (20260921–20260923) each ran exact and guarded adaptive RMC
from the same ideal 2×2×2 Cu₂O supercell: 48 atoms, 32 Cu absorbers, one fixed atom,
47 movable atoms, fixed cubic cell length 8.537 Å. This initial model is the
unrefined full crystal with calibrated S₀²/ΔE₀, not the separately optimized
first-shell fit. Both modes use the same Metropolis coordinate search; the
variable under test is scattering evaluation, not EA versus RMC.

- Objective: the existing REXAFS fitter's real and imaginary R components,
  experimental-power normalized. R=1.15–4 Å, above Rbkg=1 Å; k² weighting,
  Hanning k window 3–11.5 Å⁻¹, 1 Å⁻¹ tapers, kstep=0.05 Å⁻¹, nfft=2048.
  Retained experimental support is 2.5–12 Å⁻¹. Low-R points are excluded from
  fitting but remain visible in plots.
- Fixed calibration: S₀²=0.9840213777547031 and ΔE₀=8.762386660183811 eV.
  Adaptive training uses `dataset.exafs.theoretical_k()?`, the same shifted
  grid requested by the RMC engine; experimental k is not silently substituted.
- Proposals: Cartesian components in ±0.06 Å before feedback scaling, maximum
  displacement 0.4 Å; minimum distances Cu–O 1.5 Å, Cu–Cu 2.35 Å, O–O 2.7 Å
  (global floor 1.4 Å). Fixed Metropolis tolerance 0.0009566461630675192;
  step-width feedback uses 100-attempt windows and the archived settings.
- ReFEFF 0.4.0 facade with locked 0.3 component crates, typed paths through four
  legs, path radius 4.5 Å and electronic cluster radius 6 Å. Each process uses
  one worker and one ReFEFF thread. Prepared paths, fixed electronic contexts
  and exact unchanged-path caching apply to **both** modes.
- Adaptive feature radius 0.02 Å. Audit initial/current/best every 100 attempts;
  at most four accepted refreshes and 24 extra training geometries per component.
  An unsuccessful refresh, or exhausted budget, permanently selects exact paths.
- Each run has 1,800 seconds including setup, training, proposals, audits,
  retraining/fallback, periodic exact monitoring and checkpoint I/O. An operation
  already in progress may overrun. Terminal audit, exact verification and final
  output are recorded separately, outside that budget.
- Six one-worker processes ran concurrently on an Apple M4 Mac (`Mac16,12`,
  32 GiB RAM). Development tests and builds also shared the host. This is a
  matched shared-host wall-time experiment, not isolated hardware throughput.

Exact monitoring of the retained best occurs every 100 attempts in both modes.
Final selection takes the lowest exact score among these monitored structures
and terminal initial/current/best. It does not claim to find the exact best of
every proposed or rejected geometry. Adaptive changes can alter acceptance
history, so paired seeds do not imply identical trajectories after divergence.
Auditing retained states cannot establish a uniform error bound between audits.

The cumulative `work` path/request counters cover **proposals only**, summed
across calculator stages. Active reuse is reused_active_paths divided by
(reused_active_paths + exact_paths + basis_paths); basis share of newly evaluated
paths is basis_paths divided by (exact_paths + basis_paths). Neither statistic
counts outside-radius zeros as useful cache hits. Setup, audit and monitoring
costs are included in wall time, even for discarded replacement calculators.
Final-stage statistics alone are not whole-run performance statistics.

## Convergence rule

The existing `residual_trend` implementation compares nonoverlapping 500-attempt
windows ending at the latest retained record. It requires at least 3,000 total
attempts and three successive comparisons satisfying **both**:

- Best-score improvement ≤ 1e−5 + 0.005 × |previous best|.
- Absolute change in current-score mean ≤ 1e−5 + 0.01 × |previous mean|.

Rejections remain in the mean. A stage change resets residual history, so a new
stage needs enough consecutive history for its own comparisons. Too few records
produce `InsufficientHistory`; failed tolerances produce `StillChanging`;
passing produces `ResidualPlateau`. These are project-specific stopping
diagnostics. A plateau does not prove a unique structure, a global optimum or
agreement within experimental uncertainty. Runs stop by budget, not by a claim
that this criterion has passed.

## Reproducing the workflow

The bounded research example is `crates/rexafs/examples/rmc_qualify.rs`:

```sh
cargo build --release --locked -p rexafs --features refeff-runner \
  --example rmc_qualify
# Import a prepared Spectrum and require accepted-score agreement.
rmc_qualify import SPECTRUM.json OLD_JOB.json ACCEPTED_STATE.json NEW_IMPORT_DIR
# STATES.json is an array of saved EnsembleState file paths.
rmc_qualify check NEW_IMPORT_DIR/job.json ACCEL.json STATES.json NEW_CHECK_DIR
rmc_qualify run exact NEW_IMPORT_DIR/job.json ACCEL.json 20260921 1800 NEW_EXACT_DIR
rmc_qualify run adaptive NEW_IMPORT_DIR/job.json ACCEL.json 20260921 1800 NEW_ADAPTIVE_DIR
```

Use the built executable at `target/release/examples/rmc_qualify` (or add its
containing directory to PATH). Every output directory must be new. This example
requires one R-space dataset and one structure; the library's general API also
supports multiple datasets, weighted structures and evolutionary search.

The experiment executable and source were frozen before starting the batch.
Subsequent harness-only cleanup rejects unsupported input shapes, explicitly
forces exact mode when an adaptive manifest is supplied, and records an absent
audit as null after exact fallback. The frozen experiment used a valid single
R dataset/structure and an exact baseline manifest. Its audit log repeats the
last controller report after fallback; plots count unique report `completed`
values, not repeated no-op polling as fresh failures. These cleanup changes do
not alter the numerical kernels or the recorded runs.

Full user measurement arrays, states, checkpoints, executables and logs remain
in the private research workspace under
`output/cu2o-adaptive-qualification-2026-09-17`. Repository evidence contains
aggregate metrics without the measurement arrays. Fixed-potential agreement
cannot validate changing electronic potentials, missing scattering orders,
finite-supercell effects, constraints or background sensitivity.

## Matched results

| Run | Attempts | Best exact R residual | Budget s / attempt | Proposal s / attempt | Active reuse | Basis / new paths | Status |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| exact-20260921 | 5200 | 0.02253889 | 0.3474 | 0.3096 | 94.23% | 0.00% | StillChanging |
| adaptive-20260921 | 4992 | 0.02832203 | 0.3606 | 0.2994 | 94.22% | 2.26% | StillChanging |
| exact-20260922 | 5205 | 0.02917704 | 0.3458 | 0.3106 | 94.23% | 0.00% | StillChanging |
| adaptive-20260922 | 5224 | 0.02732553 | 0.3446 | 0.3039 | 94.21% | 0.17% | StillChanging |
| exact-20260923 | 5258 | 0.02668092 | 0.3424 | 0.3099 | 94.23% | 0.00% | StillChanging |
| adaptive-20260923 | 5100 | 0.02367083 | 0.3538 | 0.3085 | 94.21% | 0.17% | StillChanging |

Median exact residual: **0.02668092**. Median adaptive-start residual: **0.02732553**. These are three seeds per method, not a statistical superiority test.

| Adaptive run | Accepted refreshes | Exact fallback at attempt | Terminal spectral error |
| --- | ---: | ---: | ---: |
| 20260921 | 4 | 500 | 0 |
| 20260922 | 0 | 100 | 0 |
| 20260923 | 0 | 100 | 0 |

| Run | Latest window | Best improvement | Mean change (signed) | Latest window passes both? |
| --- | --- | ---: | ---: | --- |
| exact-20260921 | 4701–5200 | 0 | -4.76195e-05 | Yes |
| adaptive-20260921 | 4493–4992 | 0 | 0.00921234 | No |
| exact-20260922 | 4706–5205 | 0 | 0.000911048 | No |
| adaptive-20260922 | 4725–5224 | 0 | -0.00235363 | No |
| exact-20260923 | 4759–5258 | 0 | -0.00701583 | No |
| adaptive-20260923 | 4601–5100 | 0.00098927 | -0.00549024 | No |

All window values come from the native residual-trend implementation. A passing latest window alone is insufficient: the rule needs three successive passing comparisons. Full window histories and explicit tolerances are retained in the metrics JSON.

Improvement per budget minute is also saved, but it is dominated by the very poor initial ideal-crystal score (~32.82). The residual-versus-time curves and terminal exact residual are more informative for comparing late refinement.

## Implementation and validation

- `cargo test --locked -p rexafs`: **403 passed, 3 intentionally ignored**.
- ReFEFF-enabled release Spectrum tests: **7 passed**, including exact native-R
  scoring, the theoretical-grid request contract, Rbkg rejection/override,
  immutable input, and preprocessing survival through RMC/EA failure and resume.
- Default all-target strict Clippy and ReFEFF-enabled release strict Clippy for
  the library, Spectrum tests and qualification example passed.
- Release builds of `rmc_qualify` and `rmc_cu2o_search` passed. The final harness
  import again reproduced 0.0226059713224390. Formatting and release-version
  consistency passed; package version remains 0.2.9 and additions are unreleased.
- Stable Rust reference was built from the verified published crate. Next Rust
  reference was regenerated with strict documentation/link checks and includes
  the new API.
- Updated `origin/dev` at `bb83016` was already included. Experimental user data
  was not added to the source repository.

Related implementations and guides: [Spectrum input](rmc-spectrum-input.md),
[audit controller](rmc-adaptive-audits.md), [RMC and search policies](rmc.md),
[native residual-trend code](../crates/rexafs/src/xafs/rmc/convergence.rs),
[qualification example](../crates/rexafs/examples/rmc_qualify.rs), and
[aggregate measurements](validation/2026-09-17-cu2o-adaptive/metrics.json).
