# rexafs 0.2.3 release qualification

Status: preparation in progress. Version 0.2.1 remains the published stable
release until this version's build and publication checks are complete.

## Source and issue/PR review

This release includes Windows fix `29a1191cd2bf016c09bdc430493998e8f0954395`
and a correction for the native accessibility adapter's return type. The
[unpublished 0.2.2 build](../2026-09-10-release-0.2.2/review.md) exposed the Linux
compile failure: the Unix adapter returns unit, while Mac/Windows return optional
queued events. The corrected dispatch handles both, preserves no-change skipping,
and keeps queued notifications outside the state borrow. Regular CI now compiles
the native Linux GUI, including its actual Unix accessibility dependency.

[PR #39](https://github.com/Ameyanagi/rexafs/pull/39) remains excluded. Its head
`3c631fb1061a138cf5ba2e9ba162114ef381140a` changes nalgebra 0.34.2 to 0.35.0,
which fails at the Levenberg–Marquardt 0.15 matrix trait boundary. The
[failed core job](https://github.com/Ameyanagi/rexafs/actions/runs/34300847335/job/102307221616)
reports `E0277` in `lcf.rs`. Compatible solver versions are retained.

[Issue #20](https://github.com/Ameyanagi/rexafs/issues/20) remains open for the
legacy dynamic-clamp Jacobian and output-FFT finite-grid window differences.
Earlier fixed-penalty/default solver improvements remain in place. This patch
changes no numerical processing code or scientific defaults.

## Local preparation

Verified locally on Apple Silicon macOS with Rust 1.98.1:

- Optimized desktop suite with ReFEFF and FEFF10: **458 passed, 0 failed,
  5 ignored**, including the shortcut and accessibility diff tests.
- The explicit 0.2.3 fixture writer saved, loaded and compared both storage modes.
  **28 retained samples** pass checksum/header validation. All 26 earlier sample
  bytes and hashes, including the unpublished 0.2.2 attempt, are preserved.
- Coordinated workspace/Python/npm version and `v0.2.3` name check passed.
- `cargo fmt --all -- --check`, `git diff --check`, and `actionlint` for the
  changed regular CI workflow passed.
- The lockfile changes only four workspace package versions. The 16 release
  helper tests passed during 0.2.2 preparation; those scripts are unchanged.
  The complete new release build will exercise them again.
- Native dependency source inspection confirms `accesskit_unix` returns unit,
  while the Mac and Windows adapters return optional queued events. The corrected
  dispatch retains that return type and its empty value on unchanged frames.

## Final build and publication

Pending: regular Linux GUI compilation; immutable `v0.2.3` tag; successful manual
release build for its exact commit; all four desktop targets, 20 Python wheels,
source distribution, Rust crate, npm package, licenses and the final manifest.

Pending: both Mac signatures, Apple notarization, ZIP/DMG provenance and temporary
installation checks; available-host final-download GUI checks; registry uploads
and hash comparisons; desktop asset checksums; public latest-release verification.

## Platform coverage

The [Windows development-build review](../2026-09-09-windows-gui/README.md)
records native interaction and software-renderer timing limits. These observations
are separate from final-download qualification. Available local hardware is
Apple Silicon macOS; Intel interaction uses Rosetta. Final-download native
Windows/Linux GUI checks, native Intel hardware and clean-machine graphical
qualification remain outside the available coverage. Windows/Linux remain previews.
