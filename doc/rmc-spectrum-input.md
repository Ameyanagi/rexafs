# RMC from a processed Spectrum

This API is **unreleased**, added after rexafs 0.2.9. `Spectrum` is the public
alias of `XASSpectrum`. Use a processed spectrum directly; keep its background
and fitting settings visible rather than copying χ(k) into an unrelated job.

```rust,ignore
use rexafs::{fitting::FeffFitTransform, rmc::*, structure::Edge};
use rexafs::xafs::xafsutils::FTWindow;

// `spectrum` is your existing processed Spectrum. If background is missing,
// explicitly call spectrum.calc_background()? before creating the dataset.
let transform = FeffFitTransform {
    kmin: 3.0, kmax: 11.5, kweight: 2.0,
    dk: 1.0, dk2: Some(1.0), window: FTWindow::Hanning,
    rmin: 1.15, rmax: 4.0,
    ..Default::default()
};
let mut input = RmcSpectrumOptions::new(absorbers, Edge::K, transform);
input.k_range = Some([2.5, 12.0]); // Measured support, including window tapers.
// Set independently calibrated input.s02 and input.delta_e0 here, if available.
let dataset = RmcDataset::from_spectrum(&spectrum, input)?;
let problem = EnsembleProblem::single(configuration.clone(), dataset);

// Exact path updates with fixed reference electronic potentials.
let mut calculator = PreparedRefeffCalculator::new(
    RefeffOptions { path_criteria: [0.0; 2], ..Default::default() },
    vec![configuration],
    AccelerationSettings::default(),
)?;
let settings = SessionSettings {
    moves: RmcSettings {
        steps: 10_000, step_size: 0.03, temperature: 0.001,
        max_displacement: Some(0.2), // Matches the default catalogue envelope.
        movable_atoms,              // Explicit zero-based indices; fix an anchor.
        ..Default::default()
    },
    ..Default::default()
};
let mut session = RmcSession::new(&problem, &settings, &mut calculator)?;
session.save_checkpoint("checkpoint.json")?;
if let Err(error) = session.run(&mut calculator) {
    session.save_checkpoint("checkpoint.json")?;
    return Err(error.into());
}
session.save_checkpoint("checkpoint.json")?;
let trend = residual_trend(session.history(), &ResidualTrendSettings::default())?;
println!("{:?}", trend.status);
let resumed = RmcSession::load_checkpoint("checkpoint.json", &mut calculator)?;
let original = resumed.problem().datasets[0].source.as_ref().unwrap().spectrum();
```

The numerical choices above are illustrative. Calibrate amplitude/energy and
choose geometry constraints, tolerance, ranges, path order and supercell for the
sample. ReFEFF remains the primary calculator (`refeff-runner` feature). Exact
prepared path updates still pin the electronic potentials; the input adapter
does not change that approximation. `EvolutionSession` accepts the same problem
and retains the same snapshot. See [the session guide](rmc.md) for controls.

## What is retained and calculated

[`RmcDataset::from_spectrum`](../crates/rexafs/src/xafs/rmc/spectrum.rs) copies the
selected **unweighted** experimental samples and the complete available Spectrum
state: baseline/current energy and absorption arrays, edge energy, normalization,
AUTOBK (including resolved Rbkg), and any stored forward/inverse Fourier settings
and results. Original full-grid indices and import options are retained as well.
Baseline arrays may already reflect calibration, rebinning or other edits: this
is a state snapshot, not a complete operation history or original acquisition.
Record input-file hashes, attribution and the source revision separately when
needed; the snapshot records the package version, which can be shared by multiple
unreleased revisions.

The constructor neither mutates the input nor runs any processing stage. It
requires successful AUTOBK output and a resolved positive Rbkg. Invalid or missing
results produce actionable errors. Direct edits to legacy public spectrum fields
still require `invalidate_derived()` and explicit reprocessing; neither the
constructor nor ordinary getters can detect arbitrary stale caches.

The explicit `FeffFitTransform` determines fitting. Cached plotted χ(R) and the
spectrum's plotting ranges are not used as the objective. The constructor accepts
R fitting with one integer k weight from 0 through 3, rejecting other spaces or
multiple/fractional weights rather than silently changing them. Existing lower-level
RMC objectives remain available for other workflows.

Default `normalize_by_experiment=true` divides the sum of squared real and
imaginary R residuals by the experimental power in the **same window**, using
the existing native fitting transform. This matches the convention requested for
the Cu₂O comparison. The dataset's `sigma` scales and k weight are applied exactly
once. `sigma=None` supplies unit numerical scales, not measured uncertainty;
`mu_stddev` is in absorption units and is not imported as χ uncertainty. Supplied
`sigma` must align with the full processed k grid before `k_range` selection.

`S₀²=1` and fitting `ΔE₀=0 eV` are explicit constructor defaults, not fitted values.
The measured edge energy is retained separately. Positive fitting ΔE₀ can require
a nonzero lower measured k bound to keep theoretical k real. The constructor does
not silently remove these samples. `k_range=None` copies all available samples;
an explicit range selects existing points without resampling. Include window
tapers in that support. Fourier interpolation is performed by the existing RMC
objective, exactly as for array-only input.

Normalization becomes a fixed dataset weight. If you later change the transform,
k weight or noise scales, call `normalize_experimental_power()` explicitly to
rescale it. That method replaces the old dataset weight transactionally. Zero
experimental power is an error; set `normalize_by_experiment=false` at import to
use the unnormalized mean-squared objective instead. Scores are not reduced χ² or
parameter confidence intervals.

## Rbkg guard and explicit exceptions

AUTOBK adjusts background using low-R components, and first-shell leakage can
extend into that region, especially for short bonds to light neighbors. See the
[official algorithm documentation](https://xraypy.github.io/xraylarch/xafs_autobk.html).
The rexafs input guard therefore requires `transform.rmin >= saved Rbkg`. It
does not change χ(k) or move the fitting window. The Cu₂O 0.15 Å margin remains
an explicit experimental choice, not a universal default.

To reproduce a historical fit deliberately:

```rust,ignore
input.rbkg_policy = RmcRbkgPolicy::AllowBelowRbkg {
    reason: "Reproduce the archived 0.8–4 Å fit before comparing ranges".into(),
};
```

A blank reason fails. This policy is recorded in the snapshot/checkpoint and
checked again at session creation/resume against the current R or q objective.
The check concerns the nominal fitting bound; it does not eliminate Fourier
leakage or establish that all excluded features are artifacts. The guard does not
invent an R cutoff for k-space or wavelet objectives selected later through the
lower-level API. Array-only legacy jobs have no processing snapshot/Rbkg guard.

## Checkpoints, cost and the runnable example

The snapshot lives once per dataset in the problem, shared by all trial states
through the session's borrowed problem. It is not copied or processed per move.
Cloning a problem/checkpoint copies the owned snapshot; saving a checkpoint writes
it with the rest of the job, increasing checkpoint size and I/O. RMC and EA restore
the same snapshot. On setup/resume, experimental k/χ are checked against their
recorded indices; edits that break this connection require rebuilding the dataset.
The public API exposes a read-only spectrum through `source.spectrum()`.

Old serialized jobs/checkpoints without `source` remain readable. Their default
objectives and recorded values remain unchanged. New Rust struct literals for
`RmcDataset` need `source: None`; prefer `ExafsDataset::into()` for array-only jobs.

The small [rmc_spectrum example](../crates/rexafs/examples/rmc_spectrum.rs) accepts
serialized Spectrum and Configuration inputs, writes the complete job, performs
a cold checkpoint resume, and exports best state/XYZ and residual trends:

```sh
cargo run --release --locked -p rexafs --features refeff-runner \
  --example rmc_spectrum -- spectrum.json configuration.json new-run-directory 20
```

Export existing objects using `serde_json::to_vec_pretty(&spectrum)` and
`serde_json::to_vec_pretty(&configuration)`. Configuration JSON preserves periodic
cell vectors, unlike conventional XYZ. This example selects Cu K-edge absorbers
and shows its fitting/calculator/proposal settings directly in the source. Its
short default run is an API demonstration, not a new final Cu₂O fit. Increase the
attempt budget for research and inspect `convergence.json`; time or step limits
alone do not establish a plateau. The default diagnostic needs at least 3,000
attempts and three consecutive stable comparisons of 500-attempt windows.

Regression coverage is in [rmc_spectrum.rs](../crates/rexafs/tests/rmc_spectrum.rs):
native-fitter residual agreement, preprocessing immutability, input rejection,
Rbkg overrides, calculator failure, exact RMC/EA resume and legacy checkpoints.
