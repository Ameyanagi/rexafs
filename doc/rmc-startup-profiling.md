# RMC startup profiling method

This is an **unreleased** implementation and validation note from September 2026,
based on `dev` commit `d3a6b6f`. It does not describe the published 0.2.10
application's performance. Experimental inputs, timing reports and derived
numerical summaries remain in a local research archive; they are not included
in this repository.

## Startup work

Preparing the first EXAFS calculation includes electronic potential/phase
preparation and subsequent path evaluation for each absorbing site. Equivalent
translated environments previously generated different FEFF input strings because
equal-distance scatterers appeared in different row orders. This prevented reuse
of the same electronic calculation.

The implementation avoids repeated work. Parallelizing electronic preparation
would also encounter ReFEFF's
[scoped global linear-algebra policy lock](https://github.com/Ameyanagi/refeff/blob/91b23364309c3daca871b748ae45bf2426c60b8e/crates/refeff-linalg/src/lib.rs#L81).
The optimization does not change how a supercell is built or displayed.

## Change and scientific limits

With `AccelerationSettings::reuse_electronic_inputs = true`, scatterer rows are
sorted at the existing twelve-decimal coordinate serialization precision. The
absorber remains first. The complete input string and complete options must
match before immutable potentials and phase tables can be shared. Coordinates
are not rotated, binned or additionally rounded. Different orientations remain distinct.

Every absorbing atom still has its own topology, path catalogue and exact GENFMT
path calculations. A moved atom affects those paths as before. This optimization
adds no frozen-path approximation, absorber sampling or weaker path screening.
It retains the existing fixed-reference-potential model. An irregular or
disordered reference may have few matching inputs and therefore less benefit.

Sorting can change numerical summation and the representative atom chosen for a
potential when nearest atoms tie in distance. ReFEFF still selects a nearest atom:
its [geometry ordering and model-atom selection](https://github.com/Ameyanagi/refeff/blob/91b23364309c3daca871b748ae45bf2426c60b8e/crates/refeff-io/src/rdinp/geometry.rs)
sort the coordinates by distance before selecting the first atom of each
potential. This is consistent with FEFF's
[nearest representative-atom prescription](https://feff.phys.washington.edu/feff/Docs/feff8/feff85/feff85/POT.html).
Changing a tied representative can matter when those atoms have different
neighborhoods. Agreement cannot be assumed to be bitwise or attributed solely to floating-point
rounding. Compare both ordering modes, especially with self-consistent-field settings.

Accordingly, the mode is part of calculator identity. New desktop requests enable
it; historical requests retain their original ordering and identity on resume.
The Rust default remains false for compatibility. To enable it for a new job:

```rust
let acceleration = AccelerationSettings {
    reuse_electronic_inputs: true,
    ..Default::default()
};
```

The implementation is in
[`PreparedRefeffCalculator::ensure`](../crates/rexafs/src/xafs/rmc/accelerated.rs),
[`PreparedRefeffContext`](../crates/rexafs/src/xafs/rmc/prepared.rs), and the
[desktop request/worker](../crates/rexafs-gui/src/rmc_fitting.rs).

## Benchmark and validation method

The [benchmark source](../crates/rexafs/examples/rmc_startup_benchmark.rs) accepts
an experimental Athena project supplied locally. It compares `legacy` and `reuse`
ordering modes for Cu, Cu₂O or CuO, using the bundled crystal structures. The
crystal data's attribution remains in the
[bundled catalog](../crates/rexafs/data/builtin_cifs/catalog.json).

```sh
cargo run --release --locked -p rexafs --features refeff-runner \
  --example rmc_startup_benchmark -- \
  '/path/to/Cu oxides.prj' cu 2 reuse /tmp/cu-reuse-new.json
```

Use a new output path for each run. The source project is not modified. The JSON
contains experimental arrays and must remain subject to the input's sharing
restrictions. No experimental source file or benchmark result is distributed
with this example.

For a comparison, use the same source, processing settings, structure, seed,
compiler and backend versions. Run fresh calculators in alternating mode order
and repeat measurements without concurrent builds. Record hardware, worker counts,
path criteria and all calculation settings. The startup timer covers
`RmcSession::new`, including absorber preparation, initial paths and the objective;
it excludes preprocessing, calculator construction and file I/O. A second timer
covers one coordinate proposal. Cold-cache checkpoint resume is verified outside
the timed regions.

Compare initial and proposed χ(k) arrays, objectives, active paths and catalogue
counts as well as elapsed time. A timing improvement does not establish structural
convergence. Summation order and representative-atom selection can affect spectra;
agreement for one structure does not bound errors for another.

The native regressions cover independent preparation of a distinct local geometry,
polarization separation, retained path reports, moved atoms, cancellation and
reversed absorber visitation with cold caches. Desktop regressions cover allocating
all requested absorber contexts and preserving historical calculator identities.
See [the refinement validation record](rmc-refinement-plan.md#energy-refinement-validation)
for current test and computer-use coverage. Detailed unpublished experimental
records are retained outside the repository.
