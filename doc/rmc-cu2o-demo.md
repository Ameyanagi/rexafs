# Experimental Cu₂O reverse Monte Carlo example

This source-checkout example is **unreleased**. It refines an explicit periodic
Cu₂O configuration against the `cu2o_abs` measurement in a local Athena
`Cu oxides.prj`, using ReFEFF and the public Rust RMC API. It differs from the
synthetic two-atom example: both Cu and O coordinates move, all 32 Cu absorbers
contribute, and the acceptance tolerance is nonzero. The input project is read-only.

## Data and initial structure

The checked local project was written by Athena 0.8.061 on 12 September 2016.
Its SHA-256 is
`18684eb2c4776d5d3661f6ad6fdfae6fb81d91571bda00611ec33bd686c67dc6`.
The Cu₂O record has tag `fvnh`. Acquisition temperature, experimental uncertainty,
public source URL and upstream redistribution terms are not established by this
record. Generated experimental arrays remain in the local output directory.

The starting structure is the bundled [cuprite CIF](../crates/rexafs/data/builtin_cifs/cu2o_cuprite.cif),
COD 9007497, attributed to Kirfel and Eichhorn (1990),
[doi:10.1107/S0108767389012596](https://doi.org/10.1107/S0108767389012596).
The CIF records corrected atomic labels. Its cubic lattice parameter is
4.2685 Å. A 2 × 2 × 2 expansion produces 48 atoms: 32 Cu and 16 O, in a fixed
8.537 Å cubic periodic cell. The initial positions are crystalline. CIF
displacement tensors are not added as damping factors; disorder in the RMC
calculation comes entirely from the explicit coordinates.

## Processing and model choices

The [Rust example](../crates/rexafs/examples/rmc_cu2o.rs) imports the measurement
through `AthenaProject`, then recalculates normalization and background in
REXAFS. It does not assert numerical identity with Athena. Recorded settings are:

| Operation | Setting |
| --- | --- |
| Edge reference | 8978.207 eV, from the project |
| Pre-edge fit | −150 to −75 eV relative to the edge |
| Post-edge fit | +150 to +692.0118 eV; quadratic polynomial |
| Background | AUTOBK, Rbkg = 1 Å, k = 0.5–14.418 Å⁻¹, k weight 1 |
| Background window | Hanning, width 1 Å⁻¹; explicit override |
| RMC comparison | k = 2.5–12 Å⁻¹ in 0.05 Å⁻¹ steps, k weight 2 |
| ReFEFF | 6 Å cluster, half-path length ≤4.5 Å, at most 4 legs |
| Path screening | Curved/plane-wave criteria 4%/2.5% |
| Potentials | Non-SCF, pinned to the initial crystal |
| Polarization | Orientational average |

The archived background window is Kaiser–Bessel with zero shape parameter;
the current normalized window becomes zero there and cannot support the spline
solve. The Hanning replacement is recorded explicitly. The current fixed-penalty
AUTOBK solver also differs from historical Athena's objective. See
[background processing](autobk-fixed-penalty.md) and the saved processing state.

The measured run assumes S₀² = 0.90 and fixes the theoretical energy offset at
10.335300964301545 eV. This offset came from a preliminary two-parameter fit of
the crystalline spectrum's energy offset and a global exponential damping
factor, with amplitude held at 0.90. That diagnostic used scipy differential
evolution with seed 42, energy bounds [−12, 12] eV and damping variance bounds
[0, 0.025] Å². Its fitted variance was approximately 0.0218 Å². The global damping
factor is **not** used during coordinate refinement. This preliminary fit is a
numerical alignment choice, not an independent amplitude or energy calibration.
The exact selected offset is retained in the run's `job.json`.

The numerical objective is the mean squared difference between experimental
and calculated k²χ(k), with sigma = 1 and k numerically expressed in Å⁻¹.
Sigma is a scale choice, not a measured noise estimate. R space is a diagnostic
view of this k-space refinement, not an additional independent fitted dataset.
The [RMC guide](rmc.md) defines the score, energy-shift convention and Metropolis
acceptance rule; the [session code](../crates/rexafs/src/xafs/rmc/session.rs)
implements them.

## Coordinate sampling

The recorded run requests 1,000 single-atom proposals with ChaCha8 seed 20260916.
Each movable atom's Cartesian components change independently by a uniform
value in [−0.06, +0.06) Å. Atom zero fixes the origin; all other Cu and O atoms
are movable. The total displacement from each initial position is bounded by
0.4 Å. Hard pair minima are Cu–O 1.50 Å, Cu–Cu 2.35 Å and O–O 2.70 Å. These
are explicit exclusion choices, not fitted potential-energy parameters.

The Metropolis tolerance is T = 0.0002 in numerical objective units, not kelvin.
An uphill score change ΔF is accepted with probability exp[−ΔF/(2T)]. Accepted
uphill proposals are counted and plotted. The example stores a checkpoint,
current spectrum and best spectrum every 20 attempts; coordinate trajectories
also have a 20-attempt stride. `best.json` is the lowest-score state encountered;
`final.json` is the last accepted chain state. Earlier plots called the best state **final fit**. A 2026-09-17 correction now
labels it **Best RMC fit · N trials**: reaching the attempt limit is not evidence
of convergence, and plots from runs with different budgets must be distinguished.

## Reproduce and plot

Build the example with the `refeff-runner` feature. Output directories for
`prepare`, `run` and `check` must not already exist:

```sh
cargo run --release --locked -p rexafs --features refeff-runner \
  --example rmc_cu2o -- prepare '/path/to/Cu oxides.prj' /tmp/cu2o-prepared
```

Preparation writes experimental arrays, a crystalline path calculation,
provenance and `job-template.json`. Review the template before a new scientific
run. For the recorded example, copy it to `job.json`, set
`problem.datasets[0].exafs.delta_e0` to the recorded value above and
`settings.moves.step_size` to `0.06`. Preserve the exact job to reproduce the
coordinates; the diagnostic alignment alone is not a substitute for that input.

```sh
cargo run --release --locked -p rexafs --features refeff-runner \
  --example rmc_cu2o -- run /tmp/cu2o-prepared/job.json /tmp/cu2o-run
cargo run --release --locked -p rexafs --features refeff-runner \
  --example rmc_cu2o -- curves /tmp/cu2o-run/job.json /tmp/cu2o-run
uv run --no-project --python 3.12 --with numpy --with matplotlib \
  python scripts/plot-cu2o-rmc.py /tmp/cu2o-run
cargo run --release --locked -p rexafs --features refeff-runner \
  --example rmc_cu2o -- check /tmp/cu2o-run/job.json \
  /tmp/cu2o-run/best.json /tmp/cu2o-full-check
```

Open `/tmp/cu2o-run/plots.html`. The script also writes PNG, SVG, PDF and numeric
CSV exports. `--watch` updates the local plots while a run proceeds. Export Rust
`curves.json` after completion; the plotting script checks its Fourier arrays
against Rust's `XrayFFTF` before rendering. Both use a negative Fourier exponent,
Δk/√π scaling, k² weighting, NFFT = 2048 and 1 Å⁻¹ Hanning tapers entirely inside
the 2.5–12 Å⁻¹ support. Plot units are Å⁻² for k²χ and Å⁻³ for χ(R).
R-space magnitude and real component are shown. No scattering-phase correction
is applied, so Fourier peak positions are not direct bond lengths. See the
[transform implementation](../crates/rexafs/src/xafs/xrayfft.rs).

## Interpretation limits

This is one finite refinement from one initial structure and one seed, with a
small periodic supercell, fixed lattice, fixed nuisance parameters, screened
paths and pinned potentials. It demonstrates experimental RMC; it does not
establish sampling convergence, parameter uncertainties or a unique physical
structure. The initial perfect crystal has no thermal disorder, so a large
relative improvement from that starting score can coexist with a material
residual against the measurement. Inspect the experimental-normalized residual
as well as the initial-to-best improvement. A fresh-potential recalculation of
the final structure checks the pinned-potential approximation for that state;
it does not test all cluster radii, path cutoffs or structural hypotheses.

## New prepared-path check (2026-09-17, unreleased)

The original 1,000-attempt results above are preserved. A new ReFEFF 0.4.0
validation ran 200 attempts using exact affected-path updates and four outer
workers. Its accepted decisions and best coordinates match an unscreened pinned
pipeline control; the k²-weighted spectral difference is 0.00691%. Sampling took
20.881 s after 17.588 s setup. A separate consecutive 20-attempt timing comparison
measured 10.20× faster sampling, with setup costing about 16.7 s in both modes.

The [implementation record](rmc-implementation-progress.md) gives settings,
limitations, raw evidence locations, and the frozen-basis test that failed its
accuracy target. The Rust example now exposes `run-prepared JOB ACCELERATION NEW_DIR`
and `check-prepared JOB STATE ACCELERATION NEW_DIR`; the JSON acceleration can
select `"basis": "Exact"` (recommended), a displacement envelope and worker count.
The job must explicitly use zero path criteria and matching radius/order.

## Unreleased comparison using the standard R-space fitter

`rmc_cu2o_search` reuses `Objective::R`, `FeffFitTransform`, `RmcSession` and
`EvolutionSession` to compare the two searches from the same saved input crystal.
It fits real and imaginary χ(R) over 0.8–4 Å, using k² weighting and one-Å⁻¹
Hanning tapers inside the 2.5–12 Å⁻¹ measured support. The objective is divided
by the transformed experimental power, giving a dimensionless squared error
ratio. This fixed normalization does not change the least-squares minimum.
The example requires constant unit noise scales and verifies its initial and
best scores against the path fitter's `residual_for_dataset` implementation.
It does not fit magnitude alone or count the real part twice.

```sh
cargo run --release --locked -p rexafs --features refeff-runner \
  --example rmc_cu2o_search -- \
  rmc SOURCE_K_JOB ACCELERATION_JSON 800 NEW_RMC_DIR 20260916
cargo run --release --locked -p rexafs --features refeff-runner \
  --example rmc_cu2o_search -- \
  ea SOURCE_K_JOB ACCELERATION_JSON 800 NEW_EA_DIR 20260916
```

The arguments select the method, existing source job, prepared-calculator
settings, wall-time budget in seconds, new output directory and random seed.
The copied job records the change to R space and the normalization. The source
job and experimental measurement remain unchanged. The Metropolis tolerance is
rescaled to preserve its experimental-normalized k-space value; EA uses selection
and does not use that tolerance. Other structural and scattering settings remain
those of the source job. The initial electronic calculation is timed separately;
EA population initialization is included in search time. A generation commits
atomically, so EA can exceed the budget by its final generation's duration.

Both methods save best structures, checkpoints and residual histories. EA also
records population mean, diversity and constrained-child fallbacks. Reaching a
time limit does not establish convergence, and one seed is not a general method
comparison. Use candidate-evaluation counts and elapsed time alongside residuals;
a generation is not equivalent to a Monte Carlo attempt.

The example can also export the standard fitter's arrays without scattering:

```sh
cargo run --locked -p rexafs --features refeff-runner \
  --example rmc_cu2o_search -- inspect R_JOB STATE_JSON FITTER_OUTPUT_JSON
```

This exports real, imaginary and magnitude arrays for plots, the fit mask, and
the interleaved real/imaginary residual and data-only vectors. It reuses the
fitting implementation instead of providing a separate Fourier transform.
