# Experimental reverse Monte Carlo refinement with ReFEFF

This is an **unreleased reference implementation** on `feature/rmc-refeff`, based
on rexafs 0.2.9. The Rust API and command-line example refine explicit atomic
coordinates against extended X-ray absorption fine structure (EXAFS). ReFEFF is
the primary calculator and runs in the current process. Desktop controls and
Python/JavaScript bindings are not implemented yet.

The implementation is original Rust code. It does not incorporate EVAX or
RMCProfile source, execute either program, or claim compatibility with their
input formats. Reverse Monte Carlo (RMC) supplies the general proposal and
acceptance method; the specific objective and defaults below are rexafs choices.
See [McGreevy and Pusztai (1988)](https://doi.org/10.1080/08927028808080958) for the
original RMC method and the [RMCProfile manual, §1.2](https://rmcprofile.ornl.gov/wp-content/uploads/2023/11/rmcprofilemanual.pdf)
for an accessible description of accepting and rejecting atomic moves.

## Run a complete example

From this checkout, use a new output directory:

```sh
cargo run --locked -p rexafs --features refeff-runner --example rmc_refeff -- \
  --demo /tmp/rexafs-rmc-demo 12
```

The example computes a noise-free synthetic Cu K-edge spectrum for a two-atom
cluster with a 2.5 Å separation. It starts refinement at 2.7 Å, fixes atom 0,
and moves atom 1 with seed 42, a maximum displacement of 0.08 Å per Cartesian
component, and zero acceptance tolerance. It uses only single scattering for
this small demonstration. This is a software self-consistency test, not a
realistic copper sample, experimental validation, or proof of uniqueness.
Compilation time is excluded from the elapsed time saved in the result.

The output includes:

| File | Contents |
| --- | --- |
| `job.json` | Exact geometry, data, constraints, seed and ReFEFF options. |
| `result.json` | Initial, best and final states; spectra; scores; each attempted move; diagnostics; rexafs version; elapsed refinement time. |
| `best.xyz`, `final.xyz` | Unwrapped coordinates in Å, in the original atom order. Cell information is retained in JSON, not plain XYZ. |
| `dataset-0.csv`, etc. | Experimental k, observed χ, noise scale, initial, best and final calculated χ. |
| `initial-dataset-0-atom-0.inp`, etc. | Exact ReFEFF inputs for every selected absorber in initial and best states. |
| `synthetic-truth.json`, `.xyz` | Known target geometry, only for `--demo`. |
| `synthetic-diagnostics.json` | ReFEFF diagnostics during synthetic target generation. |

The example refuses to overwrite an existing output directory. Failures return
a nonzero exit code; the input and any already written diagnostic files remain.
Intermediate ReFEFF artifacts are held in the runner's temporary workspaces.

To refine your own data, edit the generated `job.json`, or serialize a Rust
problem using the types below, then run:

```sh
cargo run --locked -p rexafs --features refeff-runner --example rmc_refeff -- \
  /path/to/job.json /tmp/rexafs-rmc-my-sample
```

The JSON object has three fields: `problem`, `settings`, and `refeff`. Unknown
fields in those input objects cause an error, so a misspelled setting cannot
silently select a default. `settings` and `refeff` permit omitted fields and use
the documented defaults. Geometry and datasets must be explicit. Array lengths,
indices, finite values and hard constraints are checked before refinement.

## Prepare geometry and spectra

`Configuration` stores `atoms: [{atomic_number, position: [x,y,z]}, ...]` and
`cell`, either `null` for a finite cluster or three Cartesian lattice **row
vectors** in Å for a periodic cell. Atom vector order supplies stable zero-based
IDs for absorber and movable-atom selection. No atom insertion, swapping,
chemical identity change, cell refinement or symmetry constraint occurs.

Use `Configuration::from_xyz(&parsed_xyz)` for a finite cluster. A parsed XYZ
file's count and syntax follow the existing permissive structure reader, so
inspect the imported atom count. Use
`Configuration::from_structure(&structure, [nx, ny, nz])` for a periodic
supercell of an already expanded crystal structure. Repetitions are positive
integers along a, b and c. Expansion uses image-major, then site-major order.
Mixed and partially occupied sites are rejected: build an explicit occupancy
realization yourself. Hydrogen is retained in calculator clusters.

Each `ExafsDataset` supplies:

- A unique `name`, an `edge` such as `"K"`, and distinct `absorbers` of the same
  element. Each absorber is calculated and spectra are averaged with equal
  weights. Selecting one absorber represents only that site's environment.
- Strictly increasing `k` in Å⁻¹, unweighted dimensionless `chi`, and positive
  unweighted noise scales `sigma`, all of the same length, at least two points.
  Extract the desired k range before creating the dataset. For a processed
  rexafs `Spectrum`, copy its `k()` and `chi()` arrays after background removal.
- Positive `weight`, integer `kweight` from 0 to 3, fixed positive `s02`, and
  fixed `delta_e0` in eV. Supply calibrated S₀² and energy alignment explicitly;
  this implementation does not fit them together with the atom positions.

Different edges and elements can be refined jointly by adding datasets. Their
absorber selections refer to the same configuration. Polarization is currently
one global ReFEFF option, so datasets requiring different polarization settings
need a future extension. Spectra from multiple independent configurations are
not averaged by this implementation.

## Rust entry points

```rust,no_run
use rexafs::rmc::{evaluate, refine, RefeffCalculator, RefeffOptions,
    RmcProblem, RmcSettings};

fn fit(problem: &RmcProblem) -> Result<(), Box<dyn std::error::Error>> {
    let mut calculator = RefeffCalculator::new(RefeffOptions::default())?;
    let before = evaluate(problem, &mut calculator)?;
    let settings = RmcSettings {
        steps: 100,
        seed: 42,
        movable_atoms: vec![1], // Atom 0 and all unlisted atoms remain fixed.
        ..Default::default()
    };
    let result = refine(problem, &settings, &mut calculator)?;
    println!("{} -> {}", before.score, result.best.evaluation.score);
    Ok(())
}
```

Enable `refeff-runner` to compile this example. `evaluate` calculates without
refining and does not apply a separate set of RMC hard constraints. `refine`
validates the starting geometry against its hard constraints, copies inputs,
and computes its own initial spectrum. Avoid a preceding `evaluate` if you do
not need a separate preview. Zero steps returns the initial state.

`refine_with_progress` calls a closure after every attempted move. Returning
`std::ops::ControlFlow::Break(())` finishes successfully with `stopped=true` and
the last accepted state. It cannot interrupt a running scattering calculation.
For in-flight interruption, clone `RefeffCalculator::cancellation_token()` and
call `cancel()` from another thread. Backend cancellation, timeout and numerical
failure return an error; they are **not counted as rejected proposals**.

`RefeffCalculator::input_for` exposes the exact input text without calculating.
`diagnostics()` retains distinct warnings from completed calculations. The
`ExafsCalculator` trait permits an alternative backend or an analytic test
model, but it must recalculate the supplied geometry and follow the documented
unit/amplitude contract. The production implementation here uses ReFEFF.

## Objective and acceptance

For dataset d with M_d selected absorbers and N_d measured k points, the model is

\[
q_{di}=\sqrt{k_{di}^2-C\,\Delta E_{0d}},\qquad
m_{di}=S_{0d}^{2}\frac{1}{M_d}\sum_{a=1}^{M_d}\chi_a(q_{di}).
\]

Here k and q are wave numbers in Å⁻¹; C is the rexafs `ETOK` conversion in
Å⁻²/eV; ΔE₀ is a fixed energy shift in eV; and χ and the amplitude multiplier
S₀² are dimensionless. Positive ΔE₀ samples theory at smaller wave numbers.
Negative q², nonfinite values, or unavailable theoretical k support cause errors.
Linear interpolation uses the actual ReFEFF k grid and never extrapolates.
The relation between energy and wave number and the path expansion underlying
EXAFS are discussed by [Rehr and Albers (2000)](https://doi.org/10.1103/RevModPhys.72.621).
Absorber averaging and fixed-parameter handling here are implemented in
[`evaluate_configuration`](../crates/rexafs/src/xafs/rmc/engine.rs).

The objective is

\[
F=\sum_d\frac{W_d}{N_d}\sum_{i=1}^{N_d}
\left[\left(\frac{k_{di}}{k_\mathrm{ref}}\right)^{w_d}
\frac{m_{di}-\chi^{\mathrm{obs}}_{di}}{\sigma_{di}}\right]^2,
\qquad k_\mathrm{ref}=1\ \text{Å}^{-1}.
\]

W is the positive relative dataset weight, w is `kweight`, observed χ is the
unweighted measurement, and σ is its positive dimensionless noise scale.
The mean over N prevents point count alone from scaling a dataset's contribution.
Increasing W, reducing σ, or emphasizing high k changes the numerical objective
and therefore acceptance. Duplicating a dataset still increases its influence.
No covariance between k points, Fourier window, independent-point correction,
or R-space filtering is applied. This project-specific score is not a reduced
chi-square, and σ need not imply a calibrated likelihood.

At every attempted step, one movable atom is chosen uniformly. Each Cartesian
component receives an independent uniform displacement in
`[-step_size, step_size)`, in Å. Thus the maximum vector length of a proposal is
√3 times `step_size`. Proposals are symmetric; hard constraints are tested by
rejection, without clipping or redrawing a move. Allowed moves with ΔF≤0 are
accepted; others are accepted with probability exp[−ΔF/(2T)]. T is the
dimensionless `temperature` setting. It controls uphill acceptance and has no
physical temperature unit. T=0 selects greedy descent. There is no automatic
annealing schedule or convergence-based stopping.

`RmcSettings` defaults to 100 attempted moves, seed 0, step size 0.05 Å per axis,
T=1, a global minimum distance of 1 Å, and a 0.5 Å displacement limit from each
atom's starting position. An empty `movable_atoms` list means all atoms. These
are numerical defaults, not validated chemistry. The displacement limit uses
unwrapped positions so crossing a periodic boundary does not reset displacement.
Pair-distance checks include periodic images of different atoms and of the same
atom. Image enumeration handles triclinic cells without assuming that fractional
coordinate rounding finds the shortest distance.

The [engine](../crates/rexafs/src/xafs/rmc/engine.rs) uses seeded ChaCha8 random
numbers and keeps each proposed state separate until acceptance. Rejection leaves
both coordinates and calculated spectra intact. The best state is stored
separately from the final state, which can be worse when T>0. Exact repeatability
requires identical inputs, seed, dependencies, ReFEFF settings and execution
environment; this is not a cross-platform bitwise reproducibility guarantee.

## ReFEFF calculation policy and cost

The [adapter](../crates/rexafs/src/xafs/rmc/refeff.rs) constructs a new absorber
cluster and runs a fresh ReFEFF EXAFS pipeline for every allowed trial and selected absorber.
It reads typed, fully assembled χ output, which already includes path
degeneracies. It does not multiply degeneracies again or update only path lengths
while retaining old scattering amplitudes. S₀² is one in ReFEFF input and applied
once by the engine. No DEBYE or SIG2 card is written: disorder is represented by
explicit coordinates and the selected absorber ensemble.

Default options are a 6 Å atom cluster, maximum half-path length 4 Å, maximum
four legs, EXAFS limit 16 Å⁻¹, no SCF card, orientational averaging, one worker,
and a 300-second cooperative timeout per absorber. Without an SCF card, ReFEFF's
non-self-consistent potential treatment is used; potentials still recalculate
for every geometry. Optional `scf_radius` enables self-consistency and optional
`polarization` supplies a Cartesian direction. `path_criteria` contains the
curved-wave and plane-wave screening percentages, default `[4.0, 2.5]` as in
ReFEFF. Use `[0.0, 0.0]` for small convergence tests that retain weaker paths;
this can substantially increase cost. A `max_legs` value above two is only an
upper limit and does not guarantee that multiple-scattering paths survive
screening. Scattering order, radii and screening require convergence checks for
the sample. Cluster membership and selected paths can change at cutoffs.
Coordinates are written to 12 decimal places in Å, but ReFEFF's internal
handoffs can round further; this is not a guarantee of spectral sensitivity
to displacements at that precision.

For A selected absorber entries across datasets and S allowed trial moves, cost
is approximately A(S+1) full calculations. Repeated absorbers across datasets
are currently recalculated. Hard-constraint rejections skip scattering. This is
intended for small pilots and as a future acceleration reference. It is not yet
a practical large-supercell production RMC engine. Limits of 10,000 explicit
atoms, 1,000 atoms per absorber cluster, and bounded image enumeration prevent
unbounded allocations/loops; these limits do not imply affordable runtime.

## Validation and remaining work

Run the focused tests:

```sh
cargo test --locked -p rexafs --features refeff-runner --test rmc --test rmc_refeff
cargo test --locked -p rexafs --features refeff-runner --lib rmc::
```

The [algorithm tests](../crates/rexafs/tests/rmc.rs) exercise analytic distance
recovery, deterministic repeat runs, serialized results, exact rejection behavior,
hard constraints, callback stopping, uphill acceptance, joint-dataset weighting,
absorber averaging, energy shifts and invalid-input rejection. Periodic tests
include boundary contacts, skew-cell contacts and self-image contacts.
The [ReFEFF tests](../crates/rexafs/tests/rmc_refeff.rs) calculate a changed geometry
and then the original geometry again, compare single and multiple scattering in
a triangle, check input precision and periodic cluster construction, and exercise
cancellation. The command-line demo separately runs the complete refinement with
real ReFEFF calculations. See the [dated validation record](rmc-validation.md)
for measured synthetic results and the checks performed on this branch.

Before scientific use, validate against independent experimental or established
reference calculations, calibrate noise/amplitude/alignment, converge the
scattering approximations, vary initial structures and seeds, and check chemical
plausibility. EXAFS alone does not uniquely determine thousands of coordinates;
an improved score is not evidence for a unique recovered structure or quantified
uncertainty. Finite clusters also require enough boundary atoms around every
chosen absorber.

Next extensions are element-pair distance windows and coordination restraints,
incremental calculations verified against this full-recalculation reference,
R-space objectives, PDF/Bragg constraints, checkpoint/resume and coordinate
trajectories, and desktop/binding integration. Current results are serializable
records, not exact-resume checkpoints or equilibrium ensembles. There is no
RMCProfile input/output adapter or EVAX evolutionary algorithm in this version.
