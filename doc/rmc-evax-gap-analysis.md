# Historical gaps between the native RMC API and EVAX 6.16

**Historical audit:** the findings below describe commit `00b6ade`. The subsequent
implementation addresses the eight requested categories; see the [current guide](rmc.md)
and [performance/validation record](rmc-performance.md) for scope and remaining limits.
The original audit is retained without rewriting its findings.

Audited on 2026-09-16. Native implementation: `feature/rmc-refeff`, commit
`00b6ade`. Comparator: the supplied `/Users/ryuichi/Downloads/EVAX_src_6.16`
source tree. This is a source-based feature audit, not a numerical comparison of
two running programs. EVAX was not built or run for this audit. Its historical
`Copy` sources were excluded from the active implementation comparison.

The current Rust implementation provides a working coordinate-refinement
reference with ReFEFF. **It does not yet reproduce EVAX's evolutionary search,
accelerated scattering machinery, or full scientific workflow.** The most
consequential omission for practical system sizes is fast spectrum evaluation.

This document identifies missing work; it does not implement it. The requested
scope remains a Rust API and examples. Desktop controls and language bindings
are not requirements for closing these scientific and engine gaps.

## What is already present

The [Rust RMC module](../crates/rexafs/src/xafs/rmc/mod.rs) already has symmetric
single-atom moves, Metropolis acceptance, fixed/movable atoms, a minimum pair
distance, an initial-position displacement bound, finite and periodic geometry,
seeded runs, multiple EXAFS datasets/edges, and averaging over selected absorbers.
It retains initial, best and final states with their spectra, plus scalar move
history. Callers can stop between attempts; ReFEFF has a cancellation token.

The [ReFEFF adapter](../crates/rexafs/src/xafs/rmc/refeff.rs) already supports
multiple scattering through its leg-count and path-screening settings, optional
SCF and polarization. Multiple scattering itself is therefore **not a missing
feature**. The real tests cover a dimer and a small triangle; they do not establish
accuracy or performance for complex materials. See the [validation record](rmc-validation.md).

Likewise, multiple EXAFS datasets and periodic cells are implemented. They should
not be confused with multiple independently weighted structures, an evolutionary
population, or large-supercell validation, which remain separate gaps.

## Missing features and their practical effects

Priorities below are implementation recommendations for a useful Rust RMC engine,
not EVAX's own classification. Source links to EVAX refer to the local copy.

| Priority | Missing capability | Evidence in EVAX 6.16 | Current Rust behavior and consequence |
| --- | --- | --- | --- |
| Critical | **Cached and incremental scattering evaluation** | [`genEXAFS`](</Users/ryuichi/Downloads/EVAX_src_6.16/calculation.cpp:745>), [mutation](</Users/ryuichi/Downloads/EVAX_src_6.16/calculation.cpp:2066>) and [periodic basis update](</Users/ryuichi/Downloads/EVAX_src_6.16/calculation.cpp:2310>) distinguish basis construction, path clustering, amplitude updates and spectrum assembly. | Every geometrically allowed trial starts fresh ReFEFF calculations for every selected absorber. There is no affected-environment detection, path dependency map, scattering table cache or incremental spectrum update. This dominates runtime. |
| High | **Element-specific constraints and soft energy terms** | [`define_potential`](</Users/ryuichi/Downloads/EVAX_src_6.16/structure.cpp:1498>) and [`getEnergy`](</Users/ryuichi/Downloads/EVAX_src_6.16/calculation.cpp:5456>) configure and dispatch element/pair-specific hard constraints and Lennard–Jones, Sutton–Chen, displacement and path-distribution penalties. [`hybrid`](</Users/ryuichi/Downloads/EVAX_src_6.16/calculation.cpp:449>) combines residual and energy. | One global minimum distance and one global displacement limit; no species-pair rules, element-specific displacement bounds, restraint callback or energy contribution to the objective. Different chemical pairs cannot currently have different limits. |
| High | **R-, q- and wavelet-space objectives** | [`calculateResidual`](</Users/ryuichi/Downloads/EVAX_src_6.16/subCalculation.cpp:545>) selects k, back-transformed q, complex R or wavelet comparison with windows. | The RMC objective is k-space only. It cannot fit a selected radial shell or use a wavelet mask. Existing rexafs R/q transform code is available but is not connected to RMC. |
| High | **Checkpoint/resume and partial-state recovery** | [Restart writer](</Users/ryuichi/Downloads/EVAX_src_6.16/calculation.cpp:274>) and [reader](</Users/ryuichi/Downloads/EVAX_src_6.16/calculation.cpp:353>) save/load coordinates, spectra, iteration and other state, with additional cluster artifacts. | JSON results are records, not resumable sessions. A backend error returns an error without the accumulated best state; the progress callback only receives scalar move records. Long runs cannot safely resume through this API. EVAX's exact random-stream continuation was not established either. |
| High for EVAX algorithm parity | **Evolutionary populations, selection and crossover** | [`selectionT`, `selection`, `crossover`](</Users/ryuichi/Downloads/EVAX_src_6.16/calculation.cpp:1497>), [diversity/hypermutation](</Users/ryuichi/Downloads/EVAX_src_6.16/calculation.cpp:1159>) and [main iteration](</Users/ryuichi/Downloads/EVAX_src_6.16/calculation.cpp:2272>). Selection is used when there is more than one state. | One current configuration plus a saved best state. No population, breeding, family selection or diversity control. Independent seeds can be run manually, but that is not EVAX's evolutionary algorithm. |
| High for mixtures | **Multiple weighted structures** | [`new_basis_EXAFS`](</Users/ryuichi/Downloads/EVAX_src_6.16/subCalculation.cpp:1502>) combines subcalculations; [structure weights](</Users/ryuichi/Downloads/EVAX_src_6.16/subCalculation.cpp:1555>) scale their spectra. [`make_move`](</Users/ryuichi/Downloads/EVAX_src_6.16/calculation.cpp:664>) optionally changes and normalizes weights. | `RmcProblem` owns one `Configuration`. Absorber averaging within it cannot express independently weighted phases/configurations or refinement of mixture weights. |
| High for polarized joint fits | **Forward-model settings per dataset** | [Spectrum parameter fields](</Users/ryuichi/Downloads/EVAX_src_6.16/subParameters.h:12>) and [polarization input](</Users/ryuichi/Downloads/EVAX_src_6.16/subCalculation.cpp:2744>) belong to individual spectrum subcalculations. | Edge, S₀² and energy shift are per dataset, but polarization, SCF radius, scattering radii, leg count and screening belong to one calculator. Two datasets of the same edge with different polarization cannot be described to the built-in calculator. |
| Medium | **Acceptance schedule and automatic spectrum-weight adjustment** | [Mutation](</Users/ryuichi/Downloads/EVAX_src_6.16/calculation.cpp:2012>) changes the acceptance parameter when enabled; its subsequent update block can adjust spectrum weights. | Numerical tolerance, step size and dataset weights remain constant throughout a call. There is no annealing/acceptance schedule or plateau stopping policy. EVAX's declared step-adaptation functions have commented-out calls in the inspected ordinary path, so automatic step adaptation is not counted as verified active behavior. |
| Medium | **Collective and specialized move strategies** | [`make_move`](</Users/ryuichi/Downloads/EVAX_src_6.16/calculation.cpp:664>) supports one/all-atom choices; [`set_move_type`](</Users/ryuichi/Downloads/EVAX_src_6.16/parameters.cpp:1112>) recognizes both. The source also has a separate one-dimensional path. | Only one atom is displaced per proposal, with one global step size. There is no move-strategy interface. EVAX also halves some violating displacements; our engine rejects without modifying/redrawing the proposal. These are different algorithms, not merely different option names. |
| Medium | **Additional residual norms and EVAX-compatible score reporting** | [Residual norm switch](</Users/ryuichi/Downloads/EVAX_src_6.16/subCalculation.cpp:647>) provides L1, an angular comparison and default normalized root least squares; [`hybrid`](</Users/ryuichi/Downloads/EVAX_src_6.16/calculation.cpp:449>) adds its own scaling. | Weighted mean squared, noise-scaled residual only. EVAX residuals, stopping thresholds and acceptance settings cannot be transferred numerically without matching these definitions. Exact EVAX normalization need not be the native default. |
| Medium | **Structural distribution analysis** | [`GenRDFAll`](</Users/ryuichi/Downloads/EVAX_src_6.16/calculation.cpp:2924>) exports absorber/element-resolved radial distance histograms. | No RMC result helpers for distance distributions, mean/variance or coordination analysis, and no coordinate trajectory. XYZ export requires callers to do analysis separately. EVAX's histogram normalization should be matched explicitly rather than automatically calling it density-normalized g(r). |
| Medium | **Path-level output and approximation diagnostics** | [Partial EXAFS output](</Users/ryuichi/Downloads/EVAX_src_6.16/subCalculation.cpp:1258>) and [`Check_EXAFS`](</Users/ryuichi/Downloads/EVAX_src_6.16/calculation.cpp:3062>) expose path contributions and check approximations. | The adapter retains assembled χ and warning strings, but discards typed path output and stage reports. It cannot report retained leg counts, individual contributions or whether all multiple-scattering paths were screened out. |
| Medium | **Calibration workflow connected to RMC** | [`one_shell_fit`](</Users/ryuichi/Downloads/EVAX_src_6.16/calculation.cpp:1744>) performs a separate first-shell fit with distance scale, disorder, energy shift and amplitude; it contains its own stated fit-space limitations. | RMC accepts fixed S₀² and ΔE₀. rexafs already has ordinary path fitting, but no example/API workflow carries a calibrated model into RMC with provenance. This does not mean nuisance parameters must be refined simultaneously with every atomic move. |
| Lower / specialized | **Experimental amplitude/phase overrides** | [Loader](</Users/ryuichi/Downloads/EVAX_src_6.16/calculation.cpp:580>) and [reader](</Users/ryuichi/Downloads/EVAX_src_6.16/subCalculation.cpp:678>) expose a specialized override; the inspected setup applies it to the first subcalculation. | The built-in backend uses ReFEFF spectra only. An alternative calculator can implement another model, but no ready-made experimental amplitude/phase backend exists. |
| Lower / preparation | **Doping/initial-disorder setup helpers and EVAX interchange** | [Geometry construction](</Users/ryuichi/Downloads/EVAX_src_6.16/structure.cpp:941>) includes supercell/displacement/doping setup; EVAX uses its own parameter and restart formats. | Supercell creation exists, but callers must construct explicit dopant/disorder realizations. Mixed occupancy is rejected. No EVAX parameter, P1-job or restart adapter exists. This is a convenience/compatibility gap, not evidence that EVAX supports arbitrary composition-changing RMC moves. |

## Implementation details that make the first gaps larger

The [current evaluator](../crates/rexafs/src/xafs/rmc/engine.rs:89) loops through
every dataset and selected absorber on every allowed trial. It recalculates
duplicate absorber/edge requests even when only their k grid or loss differs.
The [calculator interface](../crates/rexafs/src/xafs/rmc/mod.rs:72) receives a full
configuration, absorber, edge and k grid, but no moved-atom description,
accept/reject notification, dataset identity or polarization. Efficient reuse
and dataset-specific settings need an explicit request/cache contract or a
carefully validated configuration-keyed cache.

The [pair-check loop](../crates/rexafs/src/xafs/rmc/geometry.rs:194) scans all index
pairs and skips unaffected ones inside the loop. It has no neighbor list. The
engine also clones the configuration for each trial. These costs should be
profiled after reducing scattering cost; replacing safe state separation with
in-place edits requires rejection/rollback tests.

The [run loop](../crates/rexafs/src/xafs/rmc/engine.rs:162) holds its random
generator, original displacement reference and current/best states as local
variables. Starting a new run from `final_state` changes the displacement origin
and resets the random generator. That is not continuation of the old run.
A proper session snapshot needs the original constraint reference, random state,
accepted and best configurations, score/spectrum state, counters, settings and
backend/problem identity. On restoration, any scattering cache needs validation
or recomputation.

The measured debug demo needed 242.085 seconds for its initial model plus 12
trial evaluations of a two-atom example, with other checks running concurrently.
This is evidence of the present cost, not an isolated benchmark. There is no
measured EVAX/native speed ratio. The fresh-pipeline architecture avoids our own
cached-basis approximation, but does not establish that ReFEFF is more physically
accurate than EVAX's configured FEFF calculation.

## Validation still missing

These are evidence gaps, distinct from absent APIs:

1. **A matching EVAX reference run.** Compare identical geometries and settings,
   first for spectra/path contributions and then for objective values. Changing
   both optimizer and scattering engine at once prevents attributing differences.
2. **Independent scattering validation on representative systems.** Add a metal,
   a multi-element oxide with substantial multiple scattering, and displaced
   snapshots. The current target and fitted model use the same ReFEFF engine.
3. **Real joint-edge and polarized cases.** Analytic tests establish averaging,
   weighting and energy-shift arithmetic; they do not validate real multi-edge
   spectra or polarized structural recovery.
4. **Periodic refinement at useful sizes.** Existing tests check periodic geometry
   and cluster construction; they do not benchmark or validate a realistic
   disordered supercell refinement, cutoff crossings or boundary invariance of
   calculated spectra.
5. **Approximation convergence.** Vary atom/path radii, scattering order,
   screening, potential treatment and interpolation resolution. The triangle test
   showed that default screening can remove its multiple-scattering contribution.
6. **Robustness across seeds, starting models and data noise.** The small recovery
   example cannot establish structural uniqueness, confidence intervals or
   chemically useful recovery from experimental data.
7. **Isolated performance measurements.** Measure preparation, allowed/rejected
   moves, each absorber calculation, cache rebuilds and peak memory separately,
   with exact software versions and consistent hardware/thread settings.

The EVAX source contains diagnostics and richer methods, but its presence alone
is not a validation certificate for that source distribution either.

## Recommended development order

| Stage | Concrete next deliverable | Acceptance evidence |
| --- | --- | --- |
| 1 | Add per-absorber calculation reports and representative fixtures; profile full recalculation. | Recorded path counts/settings, finite spectra, geometry reversibility, independent reference comparisons. |
| 2 | Introduce a reusable calculator/session contract; reuse unchanged absorber environments before implementing more aggressive path approximations. Keep ReFEFF as the primary engine. | Cached and fresh spectra agree within stated tolerances after both accepted and rejected moves, including cutoff changes. |
| 3 | Add species-pair distance rules, per-species displacement bounds and a restraint interface. Add snapshot/resume with partial-result recovery. | Chemical rules act on the intended pairs; resumed and uninterrupted runs follow the same proposals under a fixed backend/environment. |
| 4 | Connect existing rexafs R/q transforms; add per-dataset forward settings and a calibration example. Implement wavelets separately with a documented convention. | Transform, weighting and normalization comparisons; polarized joint-dataset tests. |
| 5 | Add weighted configurations and population/evolutionary search as separate concepts; add structural-distribution analysis. | Mixture averaging before residuals, reproducible population operations and meaningful multi-seed recovery cases. |

The [existing fitting transform](../crates/rexafs/src/xafs/fitting/transform.rs:441)
already supports k/R/q residuals, so those spaces should reuse its validated
operations where conventions match. The current RMC loss is a different
normalization; connecting the code requires an explicit numerical policy rather
than replacing a function call blindly.

Coordination/angle restraints, diffraction/PDF fitting, atom insertion/deletion,
and statistical uncertainty inference may be useful extensions, but they are not
claimed here as verified features of the inspected EVAX workflow. In particular,
RMCProfile-style diffraction/PDF constraints are a separate integration target.
Desktop work remains outside the requested scope.
