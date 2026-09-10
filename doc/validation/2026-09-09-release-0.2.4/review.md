# rexafs 0.2.4 release qualification

Status: preparation in progress. Publication requires a successful build of the
exact immutable release tag, signing and verified distribution artifacts.

## Source

[PR #48](https://github.com/Ameyanagi/rexafs/pull/48) improves Linux and Windows
desktop usability, folder scanning and repository validation. A separate release
PR coordinates the four workspace package versions, npm version, release notes
and retained project fixtures. Existing version tags and fixture bytes remain
unchanged. The expanded release PR also resolves issue #20 with a corrected
legacy derivative and explicit FFT grid compatibility; numerical defaults and
dependency versions are unchanged.

## Available-host validation

The [Linux desktop report](../2026-09-09-linux-desktop/README.md) retains screenshots
and check results from Ubuntu 24.04.4 ARM64, X11 and Mesa llvmpipe. The feature
branch passed 461 optimized desktop tests (5 ignored), core/default, ndarray and
plotting suites, strict Clippy, Python API and Node/browser Wasm checks. Packaged
launch, both FEFF engines and ten graphical smoke checks passed. The report
separates manual UI checks from automated coverage.

The 0.2.4 writer saved, reopened and compared both new fixtures. The manifest
verifies all 30 retained samples; all previous fixture bytes and hashes remain
unchanged. The coordinated v0.2.4 version check and optimized desktop suite
passed (461 tests, 5 ignored). Local packages are verification outputs, not
public release artifacts. The final 0.2.4 Linux ARM64 package also passed both
FEFF engine diagnostics and all ten X11 smoke checks using the new embedded
fixture.

The first release-PR strict job passed Clippy and formatting, then its hook tried
to install the repository-pinned toolchain over a partially installed runner
copy. The workflow now explicitly runs hooks with the stable toolchain it already
installed, matching the job's other checks. Local hooks still use the repository
pin. The corrected revision passed that strict job.

The strengthened graphical check selects Normalize's raw μ(E) control and
requires a nonzero rendered-frame count for that instrumented plot. The earlier
idle diagnostic line alone did not establish rendering. This interaction exposed
a reproducible focus regression: Normalize → Background removed the focused
control, so the subsequent Transform shortcut could no longer reach the root.
Stage shortcuts now return focus to the workspace before switching. The same
packaged click-and-shortcut sequence then passed all ten
[checks](gui-checks.json), including the
[Transform screenshot after clicking a control](stage-shortcut-after-click.png).
The final
revision requires a complete CI pass.

## Numerical compatibility follow-up

The expanded PR adds the [issue #20 corrections](../../fft-grid-compatibility.md).
Both array backends pass independent derivative and FFT regressions. The
[measured comparison](../2026-09-10-numerical-compat/README.md) retains complete
before/after Cu/Ni/Ru arrays; fixed-λ defaults are exactly unchanged. The desktop
suite now contains 462 passing optimized tests (5 ignored), including persistence
and cache invalidation for the FFT grid choice. All nine Python API tests pass.
The final revision requires refreshed CI and packaging checks below.

## Final build and publication

Pending: green feature and release PRs; immutable v0.2.4 tag; successful manual
release build for that exact commit, including all four desktop targets,
20 Python wheels, source distribution, Rust crate, npm package, licenses,
installer checks, Linux graphical smoke and complete manifest.

Pending: both Mac signatures, notarization and ZIP/DMG provenance; registry
publication and byte comparisons; public desktop checksums and latest release.

## Platform limits

Windows and Linux remain desktop previews. Linux GUI evidence uses X11 software
rendering; physical GPU and native Wayland qualification are outside this host.
Windows release CI validates core tests, native compilation, desktop diagnostics
and installation/reinstallation/uninstallation. The retained native Windows
interaction report covers the earlier development build, not new text editing
and empty-workspace controls. Mac signing CI checks signed downloads and temporary
installation; interactive Mac qualification is outside this Linux host.

[PR #39](https://github.com/Ameyanagi/rexafs/pull/39) remains excluded due to its
solver dependency incompatibility. [Issue #20](https://github.com/Ameyanagi/rexafs/issues/20)
is addressed by the [numerical compatibility fixes](../../fft-grid-compatibility.md).
