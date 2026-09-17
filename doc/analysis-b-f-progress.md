# Analysis milestones B–F (development)

Work proceeds on `feature/analysis-b-f`, based on the Phase A and Series/import
work at `8958681`. The [complete design](complete-analysis-design.md) remains
the scientific and workflow contract. No milestone below is released or complete
merely because its first module exists.

1. **B — Live acquisition:** integrate the earlier readiness/snapshot commits
   into a bounded folder coordinator, frozen processing, durable publication,
   paused recovery, and Series controls. Test synthetic incremental writers,
   changed files, multi-scan sources, retries and cancellation through computer use.
2. **C — XANES peak fitting:** native peak/step/baseline models, constrained fits,
   retained diagnostics and parameter trends; current, series and Live workflows.
3. **D — MBACK:** verify the atomic-data provider and licensing, implement the
   specified full objective and optional erfc term, then integrate normalization.
4. **E — Fluorescence correction:** implement the declared thick-sample XANES
   model, geometry/composition checks, separate internal/final normalization and
   preservation of the uncorrected branch.
5. **F — Wavelets:** explicit Cauchy kernel/order/grid, complex maps, linked views,
   comparisons, region trends and numerical export with bounded map memory.

Each numerical operation needs a simple Rust API, Python/TypeScript exposure,
documented units/defaults/assumptions, retained project results, and appropriate
runtime/editor checks. Reference implementations inform qualification; they are
not runtime dependencies. GUI checks use a separate application and generated
or already attributed public fixtures. Private experimental inputs are not copied
into this repository or public screenshots.

## Initial audit

The earlier `feature/live-acquisition` branch contained a tested readiness tracker
and immutable snapshots, but no running Live workspace. Its two commits were
preserved by cherry-picking them into this branch. Work on their original branch
was left unchanged. The remaining integration is still in progress.

## B: integrated development increment

The Live workspace now has a folder/filter preview, configurable quiet-file
observations (default three at one-second intervals), explicit existing-file
inclusion, frozen processing/metric recipes, bounded per-pass processing, and
SQLite publication records. Original bytes and converted spectra are retained;
embedded projects include both. Multi-scan sources retain separate frames, and
changed layouts require review. Recovery starts paused.

Computer use on an isolated macOS application and synthetic writer confirmed
partial-write intake, Pause with three arrivals held back, Resume from nine to
12 spectra, Follow latest off retaining frame 1 while frames 13 and 14 arrived,
and a visible trend spike at synthetic frame 7. GUI save/reopen identified cache artifacts being treated as extra catalog inputs;
these now have a separate project-file classification. Recovery counters and
retained-trend reconstruction are included in the final computer-use checks. No experimental input was used.

The focused run `cargo test --locked -p rexafs-gui --bin rexafs live` passed
26 selected tests. This includes existing tests whose names contain `live`, not
26 newly authored acquisition tests. New checks include exact source retention,
embedded relocation, distinct paths/revisions, failure rows, multi-scan sources,
reviewed mappings, cancellation, bounded batches and restart deduplication.

Native Windows/Linux and real network-share qualification remain outstanding.
C–F have not been implemented by this increment.

The full GUI test run passed **572 tests**, with **6 ignored**, before the final
QD regression and project-artifact classification changes. Subsequent focused
Live tests passed all 26 selected tests, and project compatibility tests passed
31 tests with one ignored. The new QD test reuses the already-attributed KEK
academic/nonmilitary fixture and checks angle-derived energy, absorption arrays
and original source retention. Release builds succeeded on this macOS host.

The [final computer-use record](validation/2026-09-17-live/README.md) confirms
embedded-project membership, paused recovery, retained trend reconstruction and
editable quiet-file timing. It includes only synthetic screenshots.

## C1: native peak-fit core

The unreleased [`PeakFit` API](xanes-peak-fitting.md) now provides Gaussian,
Lorentzian, pseudo-Voigt and true-Voigt peaks, error-function/arctangent steps,
and constant/linear baselines. Joint fitting supports fixed values, bounds,
restricted expression ties, native-point masks, selected-space errors, baseline
initialization, cancellation and independent batch fits. Result objects retain
initial/final definitions, component arrays, residuals and conditional covariance.

The full default-backend core unit suite passes all 208 tests. Ten focused peak
unit tests also pass on the ndarray compatibility backend, and two peak
integration tests pass on the default backend.
The integration tests compare all four peak shapes against pinned lmfit 1.3.4
references and check automatic preparation without mutating the source. A bound
regression test caught failure to refit other parameters after clamping; the new
projected optimizer passes the analytical constrained solution. Fixtures and
their integration test are excluded from the crate archive.

GUI parameter editing, retained current/series diagnostics, Live peak recipes,
Python/TypeScript bindings and full milestone-C qualification remain in progress.
No GUI peak-fitting completion is claimed by this core increment.

## C2: retained desktop peak fits

Data analysis now has a peak-model workspace with initial components/residuals,
dragged fit limits, optional baseline initialization, final masks, constraints,
and current/marked/Series fitting. Curves and full diagnostics are retained in
checksum-verified disk artifacts, while batch summaries remain resident. Embedded
projects carry those artifacts without importing them as extra spectra.

The result workspace offers fit/residual, correlation and unit-bearing trends;
left/right navigation preserves text editing. Saved models are immutable revisions.
Exports retain failed frames, input digests, masks, origins and captured series
coordinates. The [computer-use record](validation/2026-09-17-peaks/README.md)
shows synthetic masked fits, all 14 batch outcomes, project recovery and exports.
The [guide](xanes-peak-fitting.md#desktop-workflow) describes the workflow.

Core tests now pass 209 unit tests and two reference integrations. The desktop
suite passed 577 tests with six ignored before final presentation/navigation
adjustments, followed by six peak-filtered tests and release-build computer use.
Strict core Clippy passed. Live peak recipes, Python/TypeScript bindings, native
platform qualification and the remaining milestone acceptance checks are still
in progress. D–F have not been implemented by this increment.

## C3: Live peak-model recipes

Live now accepts an immutable saved peak-model revision alongside its scalar
measurement recipe. Preview fits a representative completed source before Start.
Every committed frame retains its peak outcome, including failures, independently
of the scalar outcome. The session ledger refers to checksum-verified fit artifacts;
recovery does not need to refit a source whose producer file has disappeared.

The macOS computer-use check fitted 14 synthetic scans, then added frame 15 while
viewing the peak-area trend. The chart updated without changing the selected
frame. This check caught a stale result counter, which was corrected. Acquisition
was paused before saving the embedded QA project. See the
[peak validation record](validation/2026-09-17-peaks/README.md) for the final
recovery check and test counts. Python/TypeScript peak APIs and the remaining
milestone-C acceptance checks are still in progress.

## C4: simple Python and TypeScript peak APIs

Both bindings expose an immutable `PeakFit` builder and the single-call
`spectrum.fit_peaks(model)` operation. Python accepts named keyword arguments;
TypeScript peak shapes take named options for center, area and width. Baselines
can start from zero without additional arguments. Missing normalization runs on
a copy, and `model.fit_batch(spectra)` preserves one outcome per input, including
failures. Constraints, error weighting, baseline initialization, serialization
and fitted-model reuse remain explicit options.

Installed Python/Node/browser packages compare all four peak profiles with the
same pinned lmfit fixture, including masks and conditional parameter errors.
Automatic Norm/Flat preparation, unchanged inputs, initialization, constraints,
batch failure rows and owned result arrays are covered. Generated Next API pages
and installed-package completion/hover/signature checks cover both languages;
stable reference signatures remain unchanged. See the
[binding guide](xanes-peak-fitting.md#python-and-typescript) for short examples.

The final installed-wheel run passed 29 tests and 34 subtests; npm passed all
30 tests, including installed root/Node/browser TypeScript contracts. The Python
language-server check passed, as did strict Clippy for both binding crates,
eight documentation-generator tests, and the website type/content check.

This increment does not complete native platform or experimental qualification,
and D–F remain pending. It does not add the separately tracked LCF/PCA/MCR bindings.

## D1: identified offline atomic reference provider

The core now uses pinned xraydb 0.4.1 and its embedded XrayDB 9.2 resource, with
explicit provider/table identities and an actual decoded-data SHA-256. Chantler
f₂, Elam total attenuation, explicit edge/line selections and stoichiometric
compound mass fractions share checked energy coverage. Formula errors retain
byte positions. No runtime network or Python is involved.

The [data record](../crates/rexafs/data/atomic/README.md) retains the exact size,
checksums, source licenses and the upstream qualification about original-source
copyright statements. Desktop, Python and npm packaging retain the notices.
Three core tests and a pinned Python-XrayDB comparison pass, covering four edge
regions and four compounds at relative tolerance 2×10⁻¹⁰. Strict core Clippy
passes. This establishes a reference-data layer, not completed MBACK or
fluorescence correction; their implementations and GUI qualification follow.

## D2: full MBACK core

`MBack::for_edge("Cu", "K").fit(energy, mu)` now evaluates the named full-MBACK
Chantler objective. The same settings work through the ordinary spectrum setter.
It offers strict or recorded automatic intervals, degree 0–5, positive atomic
scale, optional bounded erfc background, rank/conditioning diagnostics, matched
fpp and separate norm/flat outputs. Historical empty placeholders remain readable;
recalculation needs an explicit absorber/edge. Successful spectrum settings pin
the actual atomic reference for future replay.

Seven synthetic checks cover known scale/background, input-unit invariance,
irregular grids, independently balanced regions, excluded structure, active erfc
bounds, neighboring edges, invalid/unidentifiable fits and failed-cache clearing.
The full default core suite passes 219 tests, plus an independent integration
comparison with pinned Larch match_f2 and preedge functions in both erfc modes.
The seven focused checks also pass on ndarray-compat; strict core Clippy passes.
The [guide](mback-normalization.md) records the objective, auxiliary normalization,
automatic range policy, solver differences and data attribution. Desktop method
comparison/history, series/Live, bindings and experimental qualification remain
open; E–F are not yet implemented.

## D3: simple Python and TypeScript MBACK APIs

Both bindings now expose `MBack("Cu", "K", …)` with named range/settings options,
`model.fit(energy, mu)` and direct assignment to spectrum normalization. Results
expose norm/flat/fpp, diagnostics, copied arrays and an immutable replay definition
pinned to the original atomic identity. Optional `MbackErfc` settings require
explicit width/amplitude bounds. Missing/invalid identity and ranges fail through
the same native core. Spectrum recalculation invalidates its cached MBACK result.

Installed Python passes 32 tests and 36 subtests, including atomic-data notice
packaging. npm passes 32 tests with Node/browser native reference comparisons
and installed root/Node/browser type/editor checks. Python's installed-wheel
language-server test verifies constructor options, completion and hover help.
Strict binding Clippy, eight documentation-generator tests and the website
check all pass. Next reference pages include the new APIs; stable signatures
remain those of the release, with the historical no-argument selector documented.
Computer use inspected the current Normalize layout before the forthcoming
method-selector/range/preview integration. No GUI MBACK completion is claimed yet.

## D4: desktop MBACK, independent comparisons and retained history

Normalize now offers Polynomial, MBACK and Compare methods. Explicit source-header
identity supplies an editable suggestion. Preview and application run off the UI
thread; changing the selected group invalidates the comparison. The existing
range fields and draggable boundaries use resolved MBACK intervals, with 0.1 eV
display precision. Switching methods after editing one endpoint preserves the
opposite visible bound. Polynomial remains the default.

Atomic match and residual views show the fitting intervals. Independent results
retain original inputs, settings, complete normalization outputs and atomic data
identities in bounded immutable artifacts. Embedded projects restore that history;
JSON export includes it. Common preparation supplies identical values and resolved
provenance to desktop, Series and Live. A missing historical atomic version blocks
recomputation without replacing the archived result.

The release executable builds. The full serial GUI suite passes 585 tests with
6 intentionally ignored (391.15 seconds). A preceding parallel run passed 583,
failed one unrelated updater lock-reacquisition test, and ignored 6; the isolated
lock test passes. The final serial run also includes the new range-switch test.
The parallel test interference is not claimed to be diagnosed or fixed. Strict
GUI Clippy still reports pre-existing lint findings; no blanket suppression was
added. Core and binding qualification is recorded in D2/D3.

Computer use verified synthetic and attributed Cu inputs, source suggestions,
range editing, Norm/Flat comparison, JSON export and embedded save/reopen after
moving the original cache aside. Only synthetic screenshots are retained in the
[desktop validation record](validation/2026-09-17-mback/README.md). Native
Windows/Linux GUI and broader experimental qualification remain outstanding.
E and F remain in progress, not completed by this increment.

## E1: native fluorescence correction and explicit assumptions

`FluorescenceCorrection::new(formula, absorber, edge).line(line).angles(in, out)`
and `spectrum.correct_fluorescence(&settings)` provide the common path. Internal
conventional normalization runs automatically; the call returns a new spectrum,
preserving the original. Measured geometry is mandatory. Original arrays, formula
mass fractions, line/edge/attenuation tables, internal fit, denominator, factor
and diagnostics remain attached. Final polynomial or MBACK normalization is a
separate ordinary stage. Unknown-mode input is explicitly interpreted by the call;
native transmission imports, prepared norm/flat data and already corrected
lineages are rejected. The corrected branch cannot run unqualified EXAFS stages.

The named `fluo_elam_v1` calculation matches three synthetic cases from pinned
Larch (CuO, dilute Cu/SiO₂, Fe₂O₃), including explicit line/family choices,
geometry and internal intervals. The comparison passes on both array backends.
Five focused tests pass on each backend: model recovery, unit scaling, the valid
weak-correction limit, near-singularity diagnostics, invalid science, replay,
source immutability, acquisition interpretation and final normalization. The full
core suite passed 223 tests before the final acquisition-interpretation check was
added; the final focused run includes that additional check. Strict core Clippy
passes. No physical accuracy claim follows from reference agreement.

The [guide](fluorescence-correction.md) explains the numerical profile, explicit
defaults and its limitation for extremely dilute inputs whose ±10 eV net
attenuation jump is nonpositive. Fixtures are synthetic, retain attribution and
are excluded from the crate archive. Desktop, Series/Live and binding integration
are not completed by E1; they need to retain the same correction lineage.

## F1: native Cauchy wavelets and exact region measurements

`spectrum.wavelet(&Wavelet::new(2.0..=12.0))` prepares missing normalization and
AUTOBK on a copy. Direct `settings.calculate(k, chi)` accepts the same scientific
definition. Defaults are fixed order 100, weight 2, k step 0.05 Å⁻¹, R extent
6 Å and no taper. The result retains original/prepared arrays, support/window,
exact axes, immutable complex values, settings and optional spectrum preparation
metadata. Magnitude, phase masks and linked slices are calculated independently
of display choices. `map.integral(k_range, r_range)` uses the complete bilinear
magnitude surface, not a sampled image.

The named `cauchy_v1` convention fixes order independently of R extent, explicitly
uses inverse-FFT scaling 1/L and disallows silent truncation/extrapolation. Grid,
map-size and FFT-work budgets are checked before large allocation. Cancellation
between R rows discards partial output. Prepared inputs and exact settings allow
later map recomputation while series workflows retain only scalar measurements.

Six focused tests pass on both backends, covering a direct-DFT oracle, synthetic
k/R localization, unchanged values under R extension, irregular resampling,
support/taper, cancellation, analytic region integration, validated save/reload,
memory limits and automatic spectrum preparation. Two pinned-Larch cases match
when order, exact R coordinates and actual FFT length are supplied explicitly;
this is not default-for-default identity. Strict core Clippy passes. The public
wavelet and fluorescence examples both compile as Rust documentation tests.
See the [wavelet guide](wavelet-analysis.md) for equations, units and qualification.

F1 does not complete the linked desktop view, bounded series/Live map cache,
region trends or Python/TypeScript bindings. E/F integration remains outstanding;
no release or public merge is implied by these local core commits.


## F2a: retained desktop maps, linked inspection and region exports

The source desktop now has **Data → Analysis → Wavelet**, with automatic
normalization/background preparation, explicit k/R controls, a magnitude map,
real/imaginary/masked-phase views and a palette menu. Clicking the map updates
physical cursor coordinates and both magnitude slices. Original χ and ordinary
Fourier magnitude remain available. Their different transform/window conventions
are labeled and retained.

Shared range handles select a rectangle; its full-native-grid integral updates
without rebuilding the texture. Saved regions retain map identity, bounds, units
and numerical convention. A bounded physical-coordinate texture is display-only.
Color limits can be locked across compatible settings; changed definitions reset
that lock. New calculations reset map bounds, while color changes preserve zoom.

Immutable compressed artifacts retain complete numerical results. History,
streamed JSON export and embedded-project restore preserve the map and regions.
A saved map is explicitly historical; recalculation rereads the current source.
The project carries receipts and the desktop holds one map, rather than every
historical map in GPU memory.

Validation: 587 desktop tests passed, six ignored in the serial full suite;
focused map resampling and embedded-history tests passed after later UI edits.
The strict desktop Clippy run still reports the 52 existing findings elsewhere;
no new wavelet finding was reported. The optimized macOS app was exercised with
computer use on a generated EXAFS packet: map calculation, dragged k/R bounds,
slices, phase/colors, export, history and embedded reopen without the old cache.
The [validation record](validation/2026-09-17-wavelet/README.md) separates numerical
checks from observed UI behavior.

This increment does **not** complete F2's full-series/Live region trends, an
explicitly qualified comparison workflow, or Python/TypeScript wavelet bindings.
Fluorescence desktop/Series/Live/binding integration also remains outstanding.

## F3: Python and TypeScript wavelet APIs

Both installed packages expose `Wavelet` settings and
`spectrum.wavelet(model)`, with automatic prerequisites on a private copy.
The minimal constructor needs only the measured k interval; weight, order,
sampling and taper retain core defaults and have named overrides. Direct
`model.calculate(k, chi)` is available for original unweighted EXAFS arrays.
Owned results provide physical axes, complex values, magnitude, masked phase,
native-grid slices and rectangle integrals, original inputs, preparation metadata
and JSON replay. Python returns NumPy matrices; TypeScript exposes row-major
typed arrays and explicit Wasm ownership. See the
[binding examples](wavelet-analysis.md#python-and-typescript-unreleased).

Installed-wheel tests passed (35), including both pinned Larch wavelet cases,
array independence, spectrum preparation, replay, masked zero phase and invalid
coverage/allocation requests. All 35 npm tests passed, including Node/browser
wavelet paths and installed-tarball TypeScript types, hover help and completion.
The Python language-server check passed against the installed wheel. Strict
Python/Wasm Clippy, eight reference-generator tests and the website check
(zero errors/warnings) passed. Stable API pages keep their released signatures;
only Next reference pages gained the new API.

This completes the initial wavelet binding increment. Full-series/Live region
trends and qualified comparison workflows remain in progress; fluorescence
desktop/Series/Live/binding work and cross-platform milestone qualification are
also outstanding. F2a's computer-use evidence covers the unchanged desktop code.

## E2: Python and TypeScript fluorescence correction

Both packages now expose `FluorescenceCorrection` with required composition,
emission and measured surface angles, plus `spectrum.correct_fluorescence(model)`.
The latter returns an independent unnormalized spectrum. Internal conventional
normalization is automatic; final polynomial/MBACK normalization remains a separate
ordinary `normalize()` call. Original arrays, internal fit, atomic data, numerical
diagnostics and exact-replay settings remain available in the correction record.
The native spectrum carries that record through edits and prevents accidental
repeated correction or unqualified EXAFS/wavelet processing. Both APIs also expose
explicit acquisition interpretation. The Python reader preserves known transmission
evidence when creating a spectrum. See the
[short examples](fluorescence-correction.md#python-and-typescript-unreleased).

Installed-wheel tests passed (38) and all npm tests passed (38). New cases cover
all three pinned Larch synthetic corrections, array/result independence, replay,
geometry/coverage/singularity failures, source immutability, polynomial and MBACK
final normalization, retained history after edits, EXAFS/repeat-correction rejection,
and native object lifetimes in Node and browser adapters. Python and installed
TypeScript editor checks passed, including named angles and acquisition choices.
Strict Python/Wasm Clippy, eight reference-generator tests and the website check
(zero errors/warnings) passed. Only Next API references gained new signatures.

Desktop/Series/Live correction workflow integration and experimental/platform
qualification remain outstanding. No private measurements were added, and no
remote publication is implied by this local increment.

## Integration with dev PR #87

The feature branch incorporates dev revision `01e64d0` (PR #87), including native
RMC refinement and ReFEFF 0.4.0. The merge retains both the atomic-data dependency
and the new random-number dependencies, and exports RMC alongside the B–F APIs.
Existing feature commits remain in the branch history.

The new `RmcDataset::from_spectrum` entry point now shares the core EXAFS domain
check. A regression test first demonstrated that a corrected XANES-only spectrum
with legacy EXAFS buffers was accepted; it is now rejected without changing its
arrays or history. The same check protects retained-source validation. RMC
algorithms, defaults and its explicit prerequisite-processing contract are unchanged.
See [the input guide](rmc-spectrum-input.md) and
[`rmc_spectrum.rs`](../crates/rexafs/tests/rmc_spectrum.rs).

Fresh Python and Node/browser packages pass all 38 runtime tests in each suite and
their installed editor checks. The full desktop suite passes 588 tests, with six
existing ignored tests. Ten targeted alternate-backend tests pass for RMC input,
fluorescence and wavelet references. Strict core/Python/Wasm Clippy, formatting,
eight documentation-generator tests and the website check pass. Validation uses
local builds and synthetic or already retained attributed fixtures.
The complete `cargo test --locked -p rexafs --features refeff-runner` run also
passes: 469 core, integration and documentation tests, with three existing ignored
tests. This includes the 100-spectrum LCF/PCA/MCR cases and the RMC/scattering
regression suites. No GUI layout or existing analysis defaults change in this merge.
