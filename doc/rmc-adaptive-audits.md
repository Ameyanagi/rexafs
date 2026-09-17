# Automatic adaptive-basis audits

Unreleased Rust API, source checkout after 0.2.9. Implemented in
[`adaptive_control.rs`](../crates/rexafs/src/xafs/rmc/adaptive_control.rs), with
context sharing in
[`accelerated.rs`](../crates/rexafs/src/xafs/rmc/accelerated.rs).
This is a REXAFS optimization policy, not a reproduction of EVAX's policy.

An immutable adaptive basis can approximate a new geometry inside its feature
radius without guaranteeing its spectral error. `AdaptiveBasisController`
checks that error periodically, retrains when necessary and switches permanently
to exact typed paths if its refresh budget is exhausted or the new stage still
fails the audit. Explicitly enable `AccelerationSettings::adaptive` first;
ordinary exact calculations do not need this controller.

## What is checked

For each retained state and each dataset, the controller calculates exact typed
paths using the **same fixed electronic potentials, path catalogue and dataset
settings**. It then compares the approximate and exact mixture spectra, including
absorber averages, structure weights, S₀² and ΔE₀. Checks use the existing
`Objective` implementation: an R-space objective uses the native fitter's complex
real/imaginary convention, not R magnitude alone. Noise scaling, k weighting,
transform masks and experimental point selection are unchanged.

Let `P` be that dataset's objective against a zero model (experimental power),
`D` its objective applied to the approximate-minus-exact spectrum, and `Fa`, `Fe`
the approximate and exact data-fit scores. The dimensionless audit errors are
`sqrt(D/P)` and `abs(Fa-Fe)/P`. Both must pass. Dataset weight cancels through this
normalization. Structural penalties are not part of these error measures: the
compared states have identical geometry and weights, so those penalties agree.
A dataset with zero power is rejected because this normalization is undefined.
This normalization and its thresholds are project-specific numerical choices,
not measured experimental uncertainties or statistical confidence levels.

Default policy:

| Setting | Default | Meaning |
| --- | ---: | --- |
| `interval` | 100 | RMC attempts, or EA generations when using the EA driver |
| `spectral_tolerance` | 0.001 | Maximum normalized spectral norm difference |
| `score_tolerance` | 0.0001 | Maximum normalized absolute score drift |
| `max_refreshes` | 4 | Accepted retraining stages before exact fallback |
| `max_training_geometries` | 24 | Extra geometries per component, excluding reference |

These thresholds are independent of the training manifest's path and summed-χ
error tolerances. Choose an audit interval with its potentially substantial exact
calculation cost in mind. EA generations can include many local MC attempts;
100 generations is a different amount of work from 100 single-chain attempts.

## Scheduling and stage changes

`step_rmc` audits before the first proposal and whenever the interval has elapsed.
It checks initial/current/best states. `step_evolution` checks every population
member before a due generation, including runs with local RMC steps. A passing
audit records evidence without changing the model or clearing history.

On failure, training adds retained geometries, prioritizing the currently audited
states and retaining older training geometries up to the declared cap. The
original electronic reference is always included. The controller then audits
**all** retained states against the new stage, even when the training cap is
smaller than the population. A passing stage is committed through the optimizer's
transactional `rebase`; otherwise the exact reference calculator is committed.
After exact fallback, redundant audits stop.

Every retained state is rescored consistently at a model change. RMC keeps its
current geometry and picks the lowest new score among initial/current/best as
its new best; EA reorders the full population. Original displacement bounds,
random stream, completed counts and coordinate-step feedback are preserved.
Old residual histories and stopping windows are cleared because they describe
a different model. Calculator revision records and controller reports preserve
stage boundaries; save a checkpoint before a boundary if the entire old trace
is required. A convergence plateau must be established within the new stage.

Failed electronic setup, training limits, cancellation or other calculation
errors return an error without changing optimizer/controller state; the same
audit remains due on retry. Resource-limit failures are not silently converted
into extra expensive work. Calculator caches may warm on failure. A successful
audit commits before the subsequent proposal: if that proposal fails, save the
newly audited state. A basis change alters the objective, so this is a staged
optimizer, not a stationary Monte Carlo sampler.

Call `audit_rmc` or `audit_evolution` explicitly once more before reporting a
result stopped by wall time. A successful check only bounds retained states at
that boundary. It does not retroactively validate rejected trials, prove a
uniform error bound between audits, or establish structural uniqueness.

## Performance and resume

`refreshed_basis` and `exact_reference` share immutable prepared electronic
contexts and catalogues through reference-counted ownership. They do not copy
phase tensors, rerun electronic preparation for those contexts or carry over
approximate geometry spectra. Fresh contexts remain lazy. Sharing uses the same
cancellation token. Counters `electronic_preparations` and
`shared_electronic_contexts` distinguish actual setup from reuse. Work counters
belong to the current calculator stage; they are not cumulative run totals.

Save `checkpoint_rmc` or `checkpoint_evolution`, which combines optimizer RNG,
original bounds, population/states, controller policy/schedule, audit reports,
calculator options, electronic references and the exact training manifest.
Their `save` methods atomically replace a JSON file. Deserialize the corresponding
`AdaptiveRmcCheckpoint`/`AdaptiveEvolutionCheckpoint` and call `resume` to rebuild
and validate the model. Numerical caches and electronic tensors are not serialized;
a cold resume prepares/trains again and recalculates retained states. Matching
backend dependencies and floating-point behavior remain required.

## Runnable example and checks

```sh
cargo run --release --locked -p rexafs --features refeff-runner \
  --example rmc_adaptive -- NEW_DIRECTORY
```

[`rmc_adaptive.rs`](../crates/rexafs/examples/rmc_adaptive.rs) demonstrates a
synthetic Cu dimer, native complex-R scoring, periodic auditing, a saved/cold
midpoint resume, final audit and combined checkpoint. It is an API fixture, not
a physical Cu₂O result. With nonzero ΔE₀, the adaptive training k grid must be the
shifted **theoretical** grid; requests on other grids fall back to exact paths.

Tests cover measured-error refresh, electronic-context reuse, cold checkpoint
continuation with the same random stream, full-population exact rescoring,
cancellation, failed training, identity mismatch, passing-audit intervals and
native R objective/weight normalization. These are software guarantees. They do
not qualify approximate-basis speed or accuracy for experimental Cu₂O, fresh
self-consistent potentials, omitted paths or higher scattering orders.
