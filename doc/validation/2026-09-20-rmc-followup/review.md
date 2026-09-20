# Desktop RMC follow-up qualification

This record covers unreleased source changes based on dev commit
`5e2f4cfe63e152ea08b9ef298dc5c58b80e8e12f` (rexafs 0.2.12), dated
20 September 2026. It extends [PR #104](https://github.com/Ameyanagi/rexafs/pull/104)
with the behavior described in the [user guide](../../rmc-structural-evolution.md).
It does not qualify physical accuracy or convergence of a refined structure.

## Cache measurement

The user's locally retained saved input contains a 108-site periodic copper cell,
a 10.84614 Å cubic cell edge, an 8 Å cluster radius, a 6 Å maximum half-path
length, four path legs, and 108 selected absorbers. Ten workers, parallel path
updates and shared electronic inputs were enabled. The retained path catalogue
contained 947,592 entries. Its numerical arrays are not redistributed; the input
SHA-256 and sanitized counters are in [cache-comparison.json](cache-comparison.json).

The [measurement harness](cache_probe.rs) loads the saved request, sets one seeded
attempt, creates a fresh calculator for each policy, and times session
initialization and that attempt separately. The constructor remains at its
historical 256 MiB setting; a runtime monitor applies either the fixed limit or
the actual desktop automatic-memory policy, resampled every two seconds. The
comparison asserts equality of the serialized initial, current and best states,
including spectra and scores. These bytes matched exactly. This establishes
agreement for this input and attempt, not physical correctness.

Hardware was Mac16,12 with 10 logical CPUs and 32 GiB physical RAM, running macOS
26.5.1 (25F80). The harness used rustc 1.98.1, optimization enabled, and the release
core library with ReFEFF and FEFF10 features. Compilation was excluded, and no
build or test compilation ran during the retained measurement. Other desktop
applications remained open. An earlier overlapping-build trial is kept locally
but is excluded from the table.

| Policy | Initialization | One attempt | Retained snapshots | Repeated misses after attempt |
| --- | ---: | ---: | ---: | ---: |
| Fixed 256 MiB | 40.542 s | 38.670 s | 57 | 108 |
| Auto, initially 1,505 MiB | 40.885 s | 3.153 s | 108 | 0 |

Auto settled at 1,588 MiB during preparation and retained about 479 MiB of snapshot
payload. It reused 201,480 active paths on the measured attempt; the fixed policy
reused none. The measured attempt was 12.26 times faster. Initialization remains
about 41 seconds; this change reports preparation progress and permits cancellation
but does not remove that work. This is one sequential local comparison, without
replicates or confidence intervals, and is not a whole-refinement speedup claim.
Memory limits exclude electronic tables, catalogues and transient results.

The original raw report, executable, compile logs and source checkpoint remain in
the original checkout's ignored `target/` and local recovery storage. The retained
harness has only its source import path made relative and Rust formatting applied.
It takes an input checkpoint and a new output JSON filename. Building it requires
linking the same release `rexafs`, `serde`, `serde_json`, and `libc` libraries used
by the desktop, including their transitive dependency search directory.

## Automated checks

- Full release desktop suite: 647 passed, 7 intentionally ignored. This includes
  every retained release fixture loading, saving and reopening without losing
  state, and both GA and hybrid runs compared against split/cold-resumed runs.
- New coverage checks finite-cluster counts, periodic shell/density normalization,
  periodic self images, cancellation during catalogue enumeration, cache shrinking
  and recovery without changed spectra, bounded structural history/export, and
  exact evolutionary population/random-state recovery with a changed memory limit.
- Default core suite: 475 passed, 3 ignored, including documentation tests.
- Strict core Clippy passed with warnings denied.
- Strict Rust API documentation passed with broken links and missing public
  documentation denied, using the website's optional-feature policy. All eight
  reference-generator tests passed, and the website type/content check reported
  no errors or warnings. The generated citation index includes the g(r) reference.
- ReFEFF/FEFF10 focused tests: 47 passed across path/cache, radial distributions,
  session/evolutionary continuation and acceleration regressions.
- All repository pre-commit checks passed, including release tooling and fixture
  integrity. Release version consistency and the 46-sample compatibility manifest
  also passed.
- Local links in the changed guides were checked. Scientific wording and the
  density/self-image conventions were reviewed against the linked implementation
  and authoritative references in the guide.

The desktop suite was rerun after the cancellation fix: 647 passed, 7 ignored.
The cold-resume regression stops reconstruction immediately, verifies a retained
result event, and compares the recovery checkpoint with the original population
or chain state. A user stop is distinguished from a backend or persistence error.

## Computer-use review

A separate review application and project/settings copies were used. Original
user input and the stable/nightly applications were left intact. The publication
preset controls were checked at 1,100 × 700 points: the three icons retain visible
selection, correct dimensions after selection, hover names/dimensions, and full
accessible names. The retained synthetic finite Cu–O hybrid checkpoint was used
for structural overlays, the heatmap and distance histories. It is a test fixture,
not a scientifically refined experimental structure.

On a copy of the actual large copper checkpoint, the release app showed live
preparation/scattering counts before reconstruction finished. At a fixed 256 MiB
limit it reported 57 retained snapshots, an estimated 479 MiB needed, and repeated
misses. The warning wrapped within a narrow window. A debug-build cancellation
check exposed the loss of the displayed prior result during cold reconstruction;
that was corrected and covered by the regression above. Debug-build times are not
used as performance evidence.

The final release build also preserved the displayed fit and its completed-attempt
count when cold reconstruction was stopped, with no backend-error banner. Clearing
the memory field selected Auto and showed a resolved 1,470 MiB limit at that moment.
The original input checkpoint SHA-256 was unchanged. Review-only recovery folders
were moved into the local review archive after the app closed.

The structural captions were rechecked after wrapping changes. At 1,100 × 700
points the plot remained reachable by scrolling, including its axes. Retained
unedited screenshots are checksummed in [screenshots-sha256.json](screenshots-sha256.json):

- [Template hover and selection](05-template-hover-1100.jpg).
- [Capacity warning in a narrow window](11-cache-warning-small.jpg).
- [Initial/current/best overlays](12-structural-overlays-final.jpg).
- [Distance-versus-generation heatmap](13-structural-heatmap-final.jpg).
- [Scrolled heatmap and axes in a smaller window](14-structural-small-scroll-final.jpg).
- [Mean-distance history at actual generations](15-structural-mean-final.jpg).
- [Stopped cold resume with its prior result intact](17-stopped-preserved.jpg).
- [Resolved automatic cache limit](18-auto-memory.jpg).

The last two captures use the final cancellation implementation. Other captures
cover unchanged controls or plotting behavior; earlier screenshots and raw
accessibility transcripts remain in the ignored local review directory. Platform
qualification outside macOS is delegated to the PR's native CI matrix; local
results do not establish those platform outcomes.
