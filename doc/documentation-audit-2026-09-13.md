# Documentation and API audit, 2026-09-13

This is a developer record, not part of the public manual. The audit compared
current source with user guides, editor-visible declarations, Rust API comments,
and current build/release instructions. The starting checkout was
`6ca9ed02d1a9cd037ed0784c723ce78e4e8564b2`; released examples were checked against
`v0.2.4` and the published Python, npm and Rust packages.

## Scope and method

Parallel reviews covered the following areas. Each reviewer traced factual
claims to the implementing functions and verified added scientific references
against primary papers or official documentation.

| Area | Source and documentation examined |
|---|---|
| Python | Stubs, native PyO3 help, array conversion, settings, stage methods, reader, package and website guides |
| TypeScript/Wasm | Declarations, JavaScript wrappers, Wasm bindings, initialization, ownership, package and website guides |
| Processing | Normalization, both AUTOBK backends, Fourier preparation/scaling, inverse transform, windows, scientific manuals |
| Analysis and fitting | LCF, PCA, fitting residuals/statistics, FEFF path evaluation, noise estimates, builders and templates |
| Structures | Lattice/site construction, formula/XYZ/CIF input, occupancy, cluster selection, path ranking and FEFF formats |
| Rust workflow and utilities | Crate/prelude, Spectrum, Group, getters/invalidation, I/O, plotting, numerical helpers, copying and rebin/merge behavior |
| Desktop | Import, marks/locks, processing, projects, publication, Series, structure/path controls, updates and assistant persistence |
| Maintainer workflows | Build/release scripts and workflows, development/dependency/profiling guides, normalization stability prototype |
| Website | Stable/Next generation, missing-help detection, citations, Rust backend selection and layout, homepage/API navigation |

Historical measurements and retained input/reference arrays were preserved.
Dated plans were labeled where their language could be mistaken for current
behavior. Third-party engine internals and every archived benchmark narrative
were not independently re-derived. This audit improves traceability; passing
software checks does not establish physical accuracy or statistical validity.

## Material corrections

- The public forward Fourier transform uses `kstep / sqrt(pi)` after an
  unnormalized negative-exponent FFT. The previous manual factor was already
  correct. The revised explanation states the convention explicitly, gives the
  NumPy equivalent, and separates it from AUTOBK's fixed internal scale.
- Automatic normalization and FFT parameters are now distinguished from fixed
  Rust defaults and desktop Auto values. Kaiser–Bessel shape parameters and
  `None`/`undefined` window behavior are described per setting.
- The default AUTOBK spline is cubic in k. The `ndarray-compat` backend has a
  different legacy objective/API and is not silently selected for the main Rust
  website reference.
- Fitting equations now specify the actual weights, noise conversions and
  information-count conventions. PCA second moments, LCF uncertainty limits,
  occupancy handling, path interpolation and multiple-scattering template
  assumptions are described without claiming stronger guarantees than the code.
- Merge spread is identified as the implemented finite-count correction, not a
  general unbiased weighted standard deviation or a standard error. Rebin
  endpoint coverage and Athena chi-only interchange limitations are explicit.
- Publication defaults are 300 DPI and 1920 × 1440 pixels. A new Series guide
  explains filename ordering, frame sampling, trend definitions and existing
  labeling limitations. Project undo is session state, not saved project data.
- Python/TypeScript stable signatures remain pinned to the release. Reviewed
  explanatory text comes from maintained docstrings/JSDoc; an undocumented public
  member now fails generation. Updated installed editor help remains a source
  build feature until the next package release.
- Rust references are generated separately for the published crate and the
  checkout. Both use the default numerical backend with optional capabilities.
  The guide banner lives inside the content area and inherits rustdoc colors.
- The homepage screenshot opens First analysis; Desktop and Libraries headings
  navigate to their guides. Direct Python, TypeScript and Rust API links and a
  task-based reference index reduce navigation steps. Installation recommends
  uv and Bun.

## Functional scope

Rust settings setters now accept `AUTOBK` and `PrePostEdge` directly through
`Into<Option<Method>>`. Existing enum, `Some(enum)` and unannotated `None` calls
remain supported on both backends. No numerical algorithm was changed to fit a
formula or a prose claim. One desktop template explanation string was corrected.

## Validation evidence

- Core Rust suite: 246 passed, 3 existing ignored; strict all-targets Clippy passed.
- Direct-settings compatibility: 14 processing tests passed on each backend,
  including exact agreement between direct and wrapped configuration, stage
  invalidation and inferred `None` resets.
- Focused FFT/inverse tests: 6 passed. An independent dense DFT comparison using
  published Python 0.2.4 agreed within `4e-14`; zero-padding preserved amplitudes
  at shared R samples.
- Fitting/analysis/structure checks: 31 passed, 2 existing ignored. These overlap
  the core suite and must not be added to its count.
- Python: rebuilt wheel, 11 runtime tests, installed Pyright completion/hover/
  signature checks, all 83 public native member docstrings, and constructor
  defaults checked against instances.
- TypeScript: rebuilt browser/Node Wasm, 12 runtime/editor tests, and published
  Cu processing under both Node and Bun.
- Website: Astro checks, 5 content checks and 6 browser checks, covering links,
  declared signatures, missing help, math/citations, screenshots, search,
  accessibility, homepage destinations and Rust desktop/mobile layout.
- Rust documentation builds: published and checkout references; plotting-enabled
  doctests and broken intra-doc links checked separately.
- Installation: published Python package installed through uv; offline wheel
  installation and Bun tarball installation verified in clean environments.
- Maintainer tooling: all 8 suites passed. The normalization prototype reproduced
  its retained summary at relative tolerance `1e-10`, absolute tolerance `1e-12`.

Browser-generated test screenshots are local build artifacts in
`website/test-results/`. Computer use also verified the corrected Rust layout
in a full browser window. Existing full desktop application screenshots retain
their original capture provenance and are not cropped or regenerated by code.

## Second pass: source-owned documentation

A second review started from `b2e74079c31db959194020db4afe9e17d563b56c`
after the user requested another complete audit and emphasized that API help
must originate in source documentation. Eight reviewers covered the areas in
the scope table above, including an independent integration review. This section
records the second pass separately from the completed first-pass evidence.

The maintained Rust comments, Python native docstrings/stubs, and TypeScript
declarations now own the added explanations. The website generators copy that
help instead of keeping a second handwritten reference. Stable declarations
remain pinned to 0.2.4; Rust's expanded source help appears in Next until a crate
release. No new package version is implied by these documentation changes.

Additional findings and corrections:

- The QAS reader sorts energy and absorption together but retains duplicate
  rows. This differs from the checked array constructors, which reject unsorted
  or duplicate energy. Published Python probes confirmed the sorting behavior.
- Normalization and Fourier settings can retain inferred values after results
  are invalidated. Guides explain how to request fresh FFT/inverse spacing.
  AUTOBK's automatic `kmax` and `nknots` are exceptions: they remain unset and
  are calculated locally from each input.
- Fixed-penalty AUTOBK uses an internal `0.05 / sqrt(pi)` scale, while the
  legacy iterative objective uses the actual `kstep / sqrt(pi)`. Endpoint
  penalty strength is described as a numerical balance under those conventions.
- Source help now explains accepted negative finite Fourier lower bounds,
  inverse output-grid prerequisites, immediate binding conversion errors,
  window-specific parameters, interpolation/extrapolation, and low-level
  numerical validation limits.
- Fitting equations and uncertainty definitions are present on Rust result
  types as well as in the scientific guides. Previously bare error variants,
  payloads, structure/database operations and setting fields now have help.
  Alignment shift signs, smoothing widths, symmetry-setting limitations and
  metadata retained after data treatment are stated explicitly.
- Desktop and maintenance operations gained source comments describing
  import validation, recipe persistence, project replacement, publication
  limits, update actions, release provenance and script side effects.
- Python installation now follows the official uv project workflow:
  `uv init`, `uv add`, and `uv run`. The public guide includes the matching
  editor environment, Jupyter kernel, recorded dependencies and offline setup.
- Generator tests edit source help in temporary tagged repositories and verify
  that both channels inherit it, Stable excludes new signatures, and missing
  descriptions fail generation. Citation extraction includes TypeScript
  entry-point declarations and native Wasm help. Rust builds clear generated
  HTML between channels to prevent obsolete pages from carrying over.

Independent review corrected two new wording errors before publication:
TypeScript default-reset calls already work in 0.2.4, and automatic AUTOBK
limits must not be described as retained resolved values. Historical input data,
measurement records and original application screenshots remain unchanged.

Second-pass verification:

- Core Rust: 250 tests passed, including five doctests; three existing tests
  remain ignored. Strict all-targets Clippy passed.
- Plotting: 23 tests passed, including component/unit labels at integer and
  fractional k weights; six plotting-enabled doctests passed. These suites
  overlap the core checks and are not an additional total.
- Both Stable and Next Rust website references built successfully. Next denies
  missing public documentation and broken intra-doc links. One narrowly scoped
  exception covers Pest's generated `Rule::all_rules()` method; its contract is
  documented in the grammar. The public `Rule` re-export remains available and
  all six expression tests pass. The secondary numerical backend also passed
  documentation-link and HTML checks.
- Python: rebuilt wheel, 14 runtime checks, installed Pyright hover/completion/
  signatures, native/stub documentation parity and all 49 constructor defaults
  passed. The final help-only corrections were rebuilt and parity/editor checks
  repeated. The clean published-package uv project ran the Cu example; Jupyter
  kernel setup and fresh-cache offline installation passed.
- TypeScript: both Wasm targets rebuilt; all 13 runtime/editor checks passed.
  All six omitted/undefined/null reset cases were separately checked against
  published npm 0.2.4 after correcting the version wording.
- Website: two generator regression tests, Astro checks, five content checks
  and six browser/accessibility checks passed. Safari computer use inspected
  the uv quickstart, download commands and source-generated Rust statistics.
  The longer download command block is keyboard-scrollable. All 47 generated
  reference/citation artifacts reproduce without drift; the citation inventory
  contains 52 authored links, including implementation and service references.
- Maintenance: all eight tooling suites passed and the normalization prototype
  reproduced its retained summary (`rtol=1e-10`, `atol=1e-12`). All 11 changed
  Python maintenance/prototype files retain the same executable AST. All ten
  edited GUI Rust files contain comment-only changes.

The sole change to user-visible calculated output is the Rust plotting label:
Fourier amplitudes are now labeled Å⁻⁽ʷ⁺¹⁾ for k-weight w. Array values and
numerical algorithms are unchanged. The parser derive was isolated solely to
scope the generated-documentation lint exception; its public rule type is
re-exported at the existing path. External FEFF executables and live structure
services were reviewed from source/contracts, not exercised end to end.

## Third pass: website messaging and onboarding

A third review started from `f7a947e` after the user judged the public site's
visual design good but its explanation of the application too sparse. This pass
changed presentation and added user pages; it made no numerical or API changes
and did not alter the release manifest. The manual still targets published 0.2.4.

Homepage and download page:

- The homepage now states what rexafs is and who it is for in plain terms,
  with four key facts, an eight-item capability grid linked to the guides, the
  desktop stage bar in analysis order, a four-figure tour using the existing
  unedited 0.2.4 screenshots, an interface comparison table that keeps the
  Python/TypeScript processing-only scope explicit, an eight-question FAQ and
  a fuller footer. The design language, palette, entry panels and tested
  entry-point links are unchanged.
- The download page gained a consistent header and footer, an explanation of
  what each desktop package contains, per-package scope notes and a social
  preview image derived from the existing brand banner.

Manual:

- New pages: Concepts and glossary, Science overview, and FAQ. Overview,
  Desktop overview and Release history were expanded with audience, workflow
  and per-release summaries. Troubleshooting links the FAQ and glossary.
- Starlight now shows edit links and last-updated dates. The docs header links
  Science and Download on wide screens.
- One wording correction: FEFF10 ships in the macOS and Linux 0.2.4 packages,
  not only macOS, as recorded in the release-build workflow.

Verification: Astro check, five content checks and six browser/accessibility
checks passed; the new pages were reviewed in full-page desktop and mobile
captures. A separate change in the same session bundles FEFF10 with Windows
builds through the upstream helper process; the public manual keeps describing
the published 0.2.4 Windows package until that change is released.

## Fourth pass: concise website, API priorities and WASM feasibility

This pass started from `0e3572f5a7931244214b9f92bbd0046cc6f5f504`. At the
user's request, eight subagents divided the homepage, onboarding, desktop,
libraries, science, repository review and portability assessment. A further
integration review checked the combined edits. The starting inventory contained
182 prose files; it now covers all 185, including the new recommendations and
WASM guides. Generated reference explanations remain sourced from declarations.

The editorial goal was to remove repeated promotional copy while retaining
theory and practical instructions. Homepage capability/FAQ duplication was
removed; the download page now groups architectures under three operating-system
cards, states missing Windows/Linux ARM64 packages, and links detailed setup.
Manual sections were shortened mainly in introductions, navigation, repeated
instructions and screenshot captions. All science display equations, original
screenshot assets, historical measurements and input fixtures are unchanged.

Source-checked corrections include the FAQ's automatic startup update checks,
AUTOBK's locally resolved `kmax`/`nknots`, released versus checkout Windows
FEFF10 support, the already-shipped offline license reader, and figure CSV
exports. JavaScript examples now establish `try`/`finally` before processing so
failed stages do not skip `free()`.

Validation in this pass:

- Astro check: no errors, warnings or hints; production build passed.
- Both reference-generator regression tests, all five content tests and all six
  browser/accessibility tests passed at the `/rexafs/` deployment base.
- Python/TypeScript reference and citation regeneration produced no drift.
- Desktop/mobile download captures were reviewed; each published package appears
  once, each operating system has one heading, and both missing ARM64 packages
  are explicit. Existing released assets were verified through the GitHub API.
- All non-website prose local file targets were checked; the public content test
  checks rendered site destinations and anchors. The inventory covers every
  tracked/new prose document, including generated references.
- Both release WASM package targets built; all 13 Node runtime/editor tests and
  the separate Chromium processing/asset-loading check passed. The core without
  default features also passed its WASM compile check.
- Default-core and ReFEFF-enabled WASM probes failed. These are recorded failures,
  not new supported configurations. See the [build assessment](webassembly.md).

The [prioritized recommendations](documentation-api-roadmap.md) cover source-owned
documentation, API reproducibility, batches, advanced bindings, ARM64 distribution
and ReFEFF portability. No numerical implementation, package release or website
deployment was performed. This review did not re-derive every archived scientific
claim or run every native desktop workflow again.

## Browser workspace and next-release ARM64 implementation

The follow-up request added a browser processing workspace at `/app/` and native
Windows/Linux ARM64 jobs to the next-release pipeline. ReFEFF WASM support is
being handled upstream; this change does not modify its source or dependencies.

The workspace imports numeric text/CSV with explicit columns and eV/keV units,
then runs normalization, AUTOBK and a forward transform in a cancellable Worker.
It plots full processing results and exports full-resolution CSV and a JSON
record containing requested settings, resolved E0, source-text and WASM hashes,
source commit and build status. Inferred background settings that the binding
does not expose are explicitly identified as unavailable. Display reduction
retains sample extrema; exported arrays are not reduced. Browser-specific input,
FFT coverage and spline-workspace limits reject unsupported work before AUTOBK
instead of silently truncating the transform. The Rust algorithms are unchanged.

Release packaging now validates six native desktop architectures. Windows ARM64
uses native rexafs/ReFEFF and a separately identified x64 FEFF10 helper through
Windows 11 emulation. Windows ZIPs include signed, architecture-checked Microsoft
runtime DLLs, and installer builds revalidate their recorded provenance. Versions
after 0.2.4 require both new archives and Windows installer evidence; historical
release checks remain compatible. The public download page retains actual 0.2.4
assets and links to the next-release plans.

Validation:

- Astro check reports no errors, warnings or hints; the production root-path
  build and Stable/Next Rust reference generation pass.
- All ten content/input tests, two reference-generator tests and ten browser
  tests pass at `SITE_BASE=/`. Browser tests compare the Cu Fourier CSV with
  Node results, exercise transmission/keV import, cancellation, invalid input,
  stale exports and short-FFT rejection/recovery. Accessibility checks pass.
- The earlier `/rexafs/` build also passed the browser processing/import/export
  checks. Desktop/mobile captures were reviewed, including axis-label spacing.
- All 58 release-tooling tests across eight suites pass. Workflow lint and
  `git diff --check` pass.

Native ARM64 builds, packaged engine execution and installer qualification still
require their CI jobs. No release or website deployment was performed locally.
