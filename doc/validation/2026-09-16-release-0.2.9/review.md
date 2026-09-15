# rexafs 0.2.9 release qualification

Status on 16 September 2026: preparation; not published. This record will be
extended with the exact source commit, immutable-tag build, signing and
publication evidence. No local build is an input to public release uploads.

## Scope and source

The candidate combines bounded Assistant context and group analysis workflows
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

## Remaining release gates

- Green feature and release pull requests, with a merge commit for `dev` → `main`.
- Successful manual `release-build.yml` run for the immutable `v0.2.9` tag.
- Signing, notarization, installer and extracted-app checks for both Mac targets.
- Verification and publication of the exact GitHub-built registry artifacts.
- Qualified desktop assets and public checksums, followed by publication.
- Published-byte verification, Stable documentation metadata and branch synchronization.
