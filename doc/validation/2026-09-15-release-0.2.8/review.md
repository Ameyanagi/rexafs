# rexafs 0.2.8 release qualification

Status: release preparation. No 0.2.8 artifacts have been published.

This release adds one-button macOS updates and extends GUI folder discovery to
the formats already handled by the core measurement reader. Feature development
started from `dev` at `e5cbccb1060c56ce9bb6b18bf2e9bb4a02ef0608`.
The [release runbook](../../releasing.md) governs source promotion, exact-tag
builds, signing, publication and public artifact verification.

## Local preparation evidence

Apple Silicon macOS checks on 15 September 2026 used the pinned Rust 1.98.1
toolchain. Before the coordinated version change:

- The default GUI suite passed 483 tests with five ignored. A later focused
  suite passed 482 tests, with five ignored and four previously passed expensive
  FEFF tests filtered out. These runs overlap and must not be added together.
- The final updater suite passed ten tests covering signature rejection before
  execution, archive link rejection, concurrent installation locks, replacement
  and launch rollback, cancellation, and reuse of a cancelled cached download.
- Folder discovery covered 203 readable original fixtures. All 322 valid GUI
  signal previews matched the shared core conversions.
- The website passed Astro checking with no errors, warnings or hints, a
  139-page build and 22 content/runtime tests. Existing generated binding and
  reference artifacts were reused only after checking that their source matched.
- The normal optimized desktop build passed. No local artifact is a release
  publication input.

## Interactive update and import review

Computer use exercised disposable app copies with isolated settings and public
KEK Cu data plus a synthetic Larix session. KEK `.qd` displayed 5,135 points,
the recorded Bragg-angle conversion and transmission arithmetic. Both Larix
scans could be selected and plotted. Accepting the chosen signals cleared their
pending entries, and workspace shortcuts worked after the preview closed.

An explicitly old, locally built nightly identity allowed the updater to offer
the official signed `nightly-20260915-34921472793` release. One click completed
download, verification, replacement and automatic launch. The installed signed
app then reopened the saved recovery with exactly two accepted spectra and their
plots; no extra folder entries appeared. The transaction retained the old bundle
and the recovery file. A separate cancellation check left the test app unchanged.

These tests used temporary copies under `/tmp/rexafs-updates-review/`, not the
user's installed app or analysis. Their receipt paths are temporary local evidence.
The 0.2.8 signed release must be checked separately after its GitHub build.

## Pending release gates

The 0.2.8 maintainer writer saved and reopened both linked and embedded fixtures.
All 38 retained samples passed checksum/header validation; historical bytes and
hashes are unchanged. The 29 project tests passed with the explicit writer
ignored. The coordinated version check and formatting passed. Core strict Clippy
and the default core suite also passed before the metadata-only version change.

1. Retain the reviewed 0.2.8 linked and embedded fixture pair with this source.
2. Pass and merge the feature PR into `dev`; review its nightly and selected CI.
3. Pass the `dev` → `main` release PR and merge with a merge commit.
4. Tag the merged main commit `v0.2.8` and pass the full manually dispatched
   release matrix for that immutable source.
5. Sign and notarize both Mac targets, inspect actual downloads, and publish
   only the qualified checksummed desktop and registry artifacts.
6. Verify public package bytes and fresh consumers, update public release
   metadata and Stable references, and merge the released state back to `dev`.
