# rexafs 0.2.9 release qualification

Status on 16 September 2026: **0.2.9 is published as Latest** on
[GitHub](https://github.com/Ameyanagi/rexafs/releases/tag/v0.2.9), and its Rust,
Python and npm packages are public. All seven public registry files and all
27 public desktop assets match their qualified inputs. No local build is an
input to public release uploads.

## Scope and source

The release combines bounded Assistant context and group analysis workflows
(PR #78), selection of spectra across imported project scans (PR #79), and
native MCR-ALS with shared LCF/PCA preparation, validation and desktop workflows
(PR #80). See the [release notes](../../release-notes-0.2.9.md),
[API guide](../../analysis-api.md) and
[Cu validation evidence](../2026-09-16-cu-mixtures/README.md).

Version 0.2.8 is already present on crates.io, PyPI and npm. Its GitHub release
was still a draft when this candidate was prepared. The next coordinated
version is 0.2.9; no existing tag or registry artifact is replaced.

## Local preparation evidence

PR #80's local checks passed: 353 core tests with three ignored, 373 tests with
ndarray compatibility with three ignored, and 512 desktop tests with five
ignored. Strict core Clippy, formatting, Rust documentation, four compiled API
examples, the website source check, an optimized desktop build and crate
fixture-exclusion checks passed. These counts describe the feature checkout,
not the eventual immutable-tag build.

The computer-use checks retained in the Cu record include all 100 flattened
mixtures in batch LCF, PCA count/error plots, visible range errors, and saved
results. Numerical recovery values and scientific limitations are recorded
there rather than inferred from screenshots.

The 0.2.9 writer saved and reopened a new linked/embedded fixture pair with
synthetic LCF, batch LCF, centered PCA and MCR results. All 28 project persistence
tests passed (one maintainer writer ignored in the normal run). The manifest
check verified 40 samples and unchanged historical hashes. Coordinated version
checks, formatting, generated API references and the website source check
(37 files, no errors or warnings) also passed.

During PR #80 qualification, the website generation gate detected a missing
MCR citation entry; regenerating the index fixed it. The Python source archive
job then encountered a Rustup `cargo-clippy` file conflict while implicitly
installing toolchain-file extras. That job now explicitly selects its installed
Rust 1.98.1 toolchain for Cargo metadata and the isolated source rebuild. Local
archive creation and source-license/ABI3 checks passed with that setting; the
GitHub run remains the cross-platform qualification gate.

## Pull-request qualification

[Feature PR #80](https://github.com/Ameyanagi/rexafs/pull/80) passed all 48
selected checks before its merge into `dev` at
`b243ddfd413ea31b16901a355c2df9e186fdd244`. Its
[release build](https://github.com/Ameyanagi/rexafs/actions/runs/35035346691)
successfully rebuilt and tested the Python source archive with the explicit
toolchain setting. All four measured benchmarks were below their baseline
times; the existing 25% regression threshold was unchanged.

[Preparation PR #81](https://github.com/Ameyanagi/rexafs/pull/81) also passed
all 48 selected checks, including all six desktop targets, all 20 Python
runtime combinations and the source rebuild. Its
[release build](https://github.com/Ameyanagi/rexafs/actions/runs/35036236918)
and benchmark gate passed before the squash merge into `dev` at
`c2cb56b1ab82494c88c1a7a552dfe9e71c81b188`. Website deployment was intentionally
skipped on both pull requests. Their successful build artifacts support review;
they are not public release inputs.

[Promotion PR #82](https://github.com/Ameyanagi/rexafs/pull/82) targets the
protected `main` branch. Its branch update incorporated the previous main
release merge at `69256fb6cdb9664712c0e29a4f57c9241103b011`; the source tree
remained identical to the qualified preparation head. Main protection requires
up-to-date `Rust checks` and `Release checks`, resolved conversations and pull
requests, including for administrators. Force pushes and deletion are blocked.
All **64 checks passed**, with two intentional skips and no unresolved review
threads. The [current-head nightly](https://github.com/Ameyanagi/rexafs/actions/runs/35039524581)
also completed its Mac builds, signing and publication before promotion.

The protected pull request merged with a merge commit on 16 September 2026 at
`97f8557f37ba8040707e61f62ef8a720e6c8ff06`. Its source tree is identical to
the reviewed head. The immutable `v0.2.9` tag points to that commit.
[Manual tag build 35043376060](https://github.com/Ameyanagi/rexafs/actions/runs/35043376060)
passed all 37 jobs and is the source of the published registry packages.

The promotion's [Rust workflow](https://github.com/Ameyanagi/rexafs/actions/runs/35039527179)
and its separate [dev-push workflow](https://github.com/Ameyanagi/rexafs/actions/runs/35039524626)
passed. Both benchmark reports found no threshold regressions, with all four
measured medians below baseline. The
[Larch comparison](https://github.com/Ameyanagi/rexafs/actions/runs/35039527203)
validated 966 production fits; its reported worst prototype-relative error was
1.33 × 10⁻¹². These checks cover their measured numerical workloads, not every
experimental dataset or interactive workflow.

## Immutable build and registry publication

The exact-tag build passed all six desktop targets, four ABI3 wheel builds and
20 Python runtime combinations, the Python source rebuild, npm package checks
and the license policy. Its complete original manifest contains 33 artifacts;
all 33 downloaded files matched locally. The separate main-push Rust workflow
also passed its blocking benchmark gate with no threshold regressions.

Trusted publication passed for
[crates.io](https://github.com/Ameyanagi/rexafs/actions/runs/35047111122),
[PyPI](https://github.com/Ameyanagi/rexafs/actions/runs/35047113008) and
[npm](https://github.com/Ameyanagi/rexafs/actions/runs/35047114915).
All seven publicly downloaded package files matched the original build manifest.
The published crate SHA-256 is
`a8d7f0f6362455f57f2e613ffcc06580c85b67351cdb63eba0e4a7efb4e5ed92`.
Archive inspection confirmed that the measurement/session/analysis fixture
collections, their dedicated tests and validation screenshots are excluded from
the crate, Python source archive and npm package.

Fresh consumers passed against the publicly downloaded packages:

- Rust: automatic preparation without mutating raw inputs, 100 flattened
  two-standard mixture fits, centered PCA rank and reconstruction diagnostics,
  anchored MCR, component Fourier transformation and invalid-range rejection.
  The consumer lockfile identifies the crates.io source and the published
  checksum, with no local source patch.
- Python 3.12: 24 tests and 30 subtests, plus installed-wheel completion,
  parameter/method hover and signature checks.
- JavaScript/TypeScript: all 25 runtime/editor tests. The editor harness used
  the public tarball directly; root, Node and browser exports were checked with
  the release's pinned TypeScript tools. Only the test harness was adapted;
  published runtime files were unchanged.

The Rust consumer uses analytic synthetic mixtures. The separate Cu experiment
and its physical interpretation limits remain in the linked Cu validation record.
Python and TypeScript analysis bindings remain planned; their package tests cover
the existing reader and spectrum-processing APIs.

## Signed desktop qualification

[Signing run 35047109319](https://github.com/Ameyanagi/rexafs/actions/runs/35047109319)
passed for both Mac architectures, including notarization, installer checks and
calculation checks on extracted signed applications. It signs the exact-tag
build without recompiling executables. Both signed ZIPs and both signed DMGs
were downloaded and verified locally. Developer ID team `XXN44W8X56`, hardened
runtime, strict code-signature verification, stapled notarization and Gatekeeper
acceptance passed. DMGs were mounted read-only at dedicated temporary locations;
their applications were copied to separate QA directories and verified again.
Version, calculation self-check and both FEFF engine checks passed on macOS
26.5.1: ARM64 natively, Intel through Rosetta. Only the owned temporary mounts
were detached; existing user apps and mounted volumes were left alone.

[Desktop draft creation](https://github.com/Ameyanagi/rexafs/actions/runs/35047116507)
passed. Its unsigned Mac ZIPs and checksum sidecars were replaced with the
qualified signed outputs, and the two signed DMGs, sidecars and provenance
records were added. The regenerated desktop manifest covers 26 files; the
manifest itself is the 27th asset. All GitHub asset digests matched before the
release was published. Every asset was then downloaded without authentication
and matched again. The public Latest endpoint returned `v0.2.9`.
The [public download inventory](../../../website/public/releases/0.2.9/verification.json) records exact URLs, sizes
and hashes. The original complete build manifest is retained separately from
the desktop-only public manifest.

## Signed-release computer-use checks

The signed ARM64 app used isolated settings and a new copy of the Cu validation
project. The original project was unchanged. Eight full, unedited 1192 × 768
JPEG captures are retained under `website/public/screenshots/0.2.9/`; the
[capture manifest](../../../website/public/screenshots/0.2.9/capture.json) records their hashes, executable identity,
private QA project checksum and exact analysis settings.

- Centered PCA used all 100 flattened mixtures over −29 to +171 eV relative
  to E₀. The GUI reported rank two, 75.91%/24.09% contributions and target
  relative squared residual 1.16 × 10⁻²⁹. The linear error-versus-count plot
  was inspected and captured.
- Native MCR used all 100 flattened mixtures over common full measured coverage,
  three components, coefficient closure, seed zero, 2,000 maximum iterations
  and no spectral nonnegativity. It converged after 870 iterations with displayed
  relative squared residual 9.661 × 10⁻²⁵. Adding the three components changed
  the group count from 114 to 117. A newly added component produced finite
  k-space and R-space plots in Transform without processing errors.
- Series LCF completed 100/100 frames against the three prepared standards.
  Clicking the heatmap and pressing Right moved the selected frame to 51 and
  changed the spectrum and trend cursor together. A new portable QA project
  retained these results and the component groups.
- Both Larch `FeFoil_QXAFS_Compare.prj` and `json_unzipped.prj` selected all four
  records by default and imported all four together. The group count rose
  117 → 121 → 125. Accidentally selected `.license` companions were skipped;
  they were not treated as measurements. The published import image shows
  `json_unzipped.prj` with four checked records.
- Assistant opened beside Normalize parameters. Its model and access menus
  were inspected and captured without sending a message or changing access.

These GUI observations qualify the tested macOS workflows, not every device or
scientific interpretation. Intel interactive operation was not exercised, and
Windows/Linux remain desktop previews supported by their CI checks rather than
physical interactive testing in this session. Centered PCA directions are not
chemical identities, and blind MCR's small residual does not prove unique pure
components. The Cu fixture record retains its attribution limits.

## Documentation publication

Stable metadata, package installation pins and the Rust/Python/TypeScript
references now target the verified 0.2.9 artifacts. Updated guides cover input
preparation, common-range validation, collection plots, calculated-component
processing, project import and the bounded Assistant context. Python and
TypeScript collection-analysis bindings remain explicitly planned.

Website deployment follows the normal protected documentation PR promotion
through `dev` to `main`. Publishing a GitHub release alone does not regenerate
the website's curated metadata or screenshots.

Local publication checks passed: coordinated version validation, regenerated
Python/TypeScript references and citation index, Stable and Next Rustdoc,
37-file Astro check with no errors or warnings, eight generator tests, the
custom-domain website build, 22 content/runtime tests and 25 browser tests.
The eight new app captures were inspected at their original dimensions.
The first local website build lacked `wasm-pack` on its shell path; rerunning
with the documented Cargo binary directory completed the unchanged build.
