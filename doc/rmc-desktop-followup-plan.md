# RMC desktop follow-up

Unreleased implementation work requested on 20 September 2026. This plan extends
the six desktop usability fixes in PR #104. Items below are acceptance criteria,
not claims that the features have been completed.

## Required behavior

- Automatically size the exact-scattering cache from available physical memory,
  retain operating-system headroom, and reassess the budget during a run. Offer
  an explicit memory override and explain the resolved automatic value.
- Distinguish cold cache misses from repeated requests without a usable cached
  snapshot. Show eviction counts, retained payload, estimated working-set size,
  and an actionable warning when capacity causes repeated calculations.
- Report preparation and calculation stages before the first completed move.
  Check cancellation during path enumeration, including large catalogues.
- Preserve numerical results, deterministic checkpoints, and historical saved
  projects when the runtime memory budget changes.
- Add a Structural evolution view with element-pair curves; initial, current,
  and best overlays; a distance-versus-step heatmap; and histories of coordination
  number, mean distance, and distance variance in a declared radial interval.
- Normalize periodic bulk distributions by species number density and spherical
  bin volume. Explicitly label finite-cluster neighbor counts. Document periodic
  images, self exclusion, selected centers, units, and finite-cell limitations.
- Calculate structural diagnostics periodically on a background worker with
  bounded storage. Preserve actual attempt/generation coordinates, settings,
  sampling gaps, and exportable history across pause, save, and resume.
- Expose genetic/evolutionary and hybrid EA–RMC search through desktop controls,
  with population/generation/local-move settings, correct progress, and exact
  checkpoint recovery. Explain the current incompatibility with energy-shift
  refinement. Do not silently change a user's scientific settings.

The follow-up also includes the requested compact publication template picker:
small layout previews, names and dimensions on hover, and retained accessible
names and keyboard focus.

## Evidence and verification

The motivating local 108-site copper calculation uses an 8 Å cluster, 6 Å maximum
half-path length, and four path legs. A controlled initial evaluation plus one
seeded attempt took 44.611 s per attempt with 256 MiB and 3.314 s with 512 MiB.
The original probe reported equal initial, current, and best states. Initialization remained
about 44 s. This is one local performance diagnostic, not a whole-refinement
speedup or a converged structure. Raw data and hardware metadata remain in the
original checkout's ignored `target/rmc-cache-comparison-20260920.json`.

Required verification includes controlled cache-pressure/recovery tests,
cancellation inside enumeration, periodic and finite structural examples,
background sampling and serialization, evolutionary/hybrid save-and-resume,
legacy project loading, and a computer-use walkthrough. Repeat the motivating
performance comparison with the automatic policy and retain the numerical
equality check. Run the repository's required core and desktop checks.

The implementation must retain the historical constructor identity of
`PreparedRefeffCalculator`: that identity currently serializes the constructor's
cache setting. Runtime memory adjustment must preserve the identity, so old
checkpoints remain resumable.

## References

- [Existing desktop worker](../crates/rexafs-gui/src/rmc_fitting.rs).
- [Prepared scattering and cache](../crates/rexafs/src/xafs/rmc/accelerated.rs).
- [Path catalogue](../crates/rexafs/src/xafs/rmc/paths.rs).
- [Distance histograms](../crates/rexafs/src/xafs/rmc/analysis.rs).
- [Evolutionary search](../crates/rexafs/src/xafs/rmc/evolution.rs).
- [GROMACS radial-distribution normalization](https://manual.gromacs.org/current/onlinehelp/gmx-rdf.html).
- [Linux available-memory estimate](https://docs.kernel.org/filesystems/proc.html).
- [Windows physical-memory counters](https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/ns-sysinfoapi-memorystatusex).
- [Apple virtual-memory counters](https://github.com/apple-oss-distributions/xnu/blob/main/osfmk/mach/vm_statistics.h).

Automatic fractions, headroom, sampling intervals, and warning thresholds are
rexafs resource policies. They must be documented as such, not attributed to the
scientific references.
