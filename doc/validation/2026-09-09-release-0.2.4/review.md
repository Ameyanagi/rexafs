# rexafs 0.2.4 release qualification

Status: preparation in progress. Publication requires a successful build of the
exact immutable release tag, signing and verified distribution artifacts.

## Source

[PR #48](https://github.com/Ameyanagi/rexafs/pull/48) improves Linux and Windows
desktop usability, folder scanning and repository validation. A separate release
PR coordinates the four workspace package versions, npm version, release notes
and retained project fixtures. Existing version tags and fixture bytes remain
unchanged. Numerical algorithms, defaults and dependency versions are unchanged.

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
public release artifacts.

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
remains open for the legacy clamp Jacobian and output-FFT window-domain differences.
