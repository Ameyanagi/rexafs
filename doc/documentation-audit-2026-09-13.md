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
