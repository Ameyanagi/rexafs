# rexafs 0.2.2 release qualification

Status: **not published**. The immutable `v0.2.2` tag points to
`dc993fe55dea9889b0eab3b1668f1bce1e2bd8c4`. Its
[manual release build](https://github.com/Ameyanagi/rexafs/actions/runs/34408472161)
failed Linux desktop compilation and was cancelled. No release assets or registry
packages were published from this attempt. The
[0.2.3 qualification record](../2026-09-10-release-0.2.3/review.md) tracks the fix.

The Linux [failed job](https://github.com/Ameyanagi/rexafs/actions/runs/34408472161/job/102657037257)
reported `E0308` at `accessibility.rs:305`: `Option::and_then` required the
adapter call to return `Option`, while `accesskit_unix` returns `()`.
The Mac and Windows adapters return optional queued events. The correction
preserves each platform's return type and adds a regular Linux GUI compile gate.

## Source and issue/PR review

The release starts from Windows fix
`29a1191cd2bf016c09bdc430493998e8f0954395`. It changes Windows console behavior,
platform command shortcuts and selection modifiers, accessibility tree updates,
and package/installer diagnostics. It does not change numerical processing code.

- [Main Rust CI](https://github.com/Ameyanagi/rexafs/actions/runs/34406693214)
  passed all five jobs, including core plotting, strict checks and the benchmark
  regression gate.
- [Windows installer CI](https://github.com/Ameyanagi/rexafs/actions/runs/34406693188)
  passed its helper, install, reinstall and uninstall checks. This workflow uses
  an existing release archive; it does not qualify a newly compiled 0.2.2 GUI.
  The final tag's complete release matrix must build and qualify that executable.
- [PR #39](https://github.com/Ameyanagi/rexafs/pull/39), head
  `3c631fb1061a138cf5ba2e9ba162114ef381140a`, changes nalgebra 0.34.2 to 0.35.0.
  Its [core build](https://github.com/Ameyanagi/rexafs/actions/runs/34300847335/job/102307221616)
  fails with `E0277` in the `LeastSquaresProblem` implementation in `lcf.rs`:
  nalgebra 0.35 storage types do not implement the nalgebra 0.34 traits expected
  by Levenberg–Marquardt 0.15. The release excludes the PR and retains the
  compatible versions documented in [dependency notes](../../dependencies.md).
- [Issue #20](https://github.com/Ameyanagi/rexafs/issues/20) remains open. Its
  comments record the earlier default fixed-penalty, interpolation, knot/cutoff,
  and endpoint work. The legacy dynamic clamp Jacobian still omits the derivative
  of its coefficient-dependent scale; the public output FFT still constructs its
  window on the provided finite grid. Those are separate numerical compatibility
  tasks. This patch keeps the established defaults and saved-project behavior.

## Local preparation

Verified locally on Apple Silicon macOS with Rust 1.98.1:

- Optimized desktop suite with ReFEFF and FEFF10: **458 passed, 0 failed,
  5 ignored**. This includes the platform shortcut and accessibility diff tests.
- Explicit 0.2.2 fixture writer: both linked and embedded projects saved,
  loaded and compared successfully. The retained manifest verifies **26 samples**;
  all 24 historical project/input bytes and checksums are unchanged.
- Coordinated workspace/Python/npm version and `v0.2.2` tag-name check passed.
- Release maintenance (3), Windows installer (7), desktop download staging (2),
  and registry artifact (4) helper tests passed: **16 tests**.
- `cargo fmt --all -- --check` and `git diff --check` passed.
- The lockfile changes only the four workspace package versions. No dependency
  resolution changed. The project format remains version 1.

## Final build and publication

The manual build did not produce a qualified final manifest. Signing, registry
publication, and GitHub release publication were not started. The existing
0.2.1 public release and packages were left intact.

## Platform coverage

The [Windows development-build review](../2026-09-09-windows-gui/README.md)
records no-console launches, native Ctrl shortcuts, Undo, plots, import and
structure interactions. Its release timing sample used Microsoft Basic Render
Driver software rendering and does not establish accelerated Windows frame rates.
These observations are separate from final-download qualification.

Available local hardware is Apple Silicon macOS. Intel app interaction can be
checked under Rosetta. Final-download native Windows/Linux GUI checks, native
Intel hardware and clean-machine graphical qualification remain outside the
available-host coverage; Windows/Linux remain previews.
