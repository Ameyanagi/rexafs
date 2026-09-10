# rexafs 0.2.4 release qualification

This report records source and candidate qualification. Consult the
[versioned release](https://github.com/Ameyanagi/rexafs/releases/tag/v0.2.4)
for the final immutable source commit, tagged build, signing and publication
results, together with the public artifact checksums.

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
The candidate CI report also passed all 966 fixed-objective fits and 30 endpoint
checks at the existing tolerances. Its measured benchmark comparisons passed the
25% regression threshold.

The [manual Larch-grid screenshot](larch-grid.png) records the selected sampling
mode and rendered plots. Undo restored Input grid and the app quit cleanly.

## Linux ARM64 and NVIDIA GB10

The local optimized 0.2.4 candidate passed all ten GUI checks with an NVIDIA
GB10 selected as a hardware device (driver 580.142). The
[checks](nvidia-gui-checks.json), [renderer log](nvidia-renderer.txt) and
[Transform screenshot](nvidia-transform.png) retain that evidence. The test used
Xvfb at 1366 × 768, with `VK_DRIVER_FILES` selecting the NVIDIA ICD. The renderer
reported `is_software_emulated: false`; the same candidate had already passed
with Mesa software rendering. This verifies that local ARM64 candidate and
driver combination, rather than a published ARM64 download or physical-monitor
session.

The Linux x86-64 CI archive also passed all ten GUI checks. The Windows x86-64
installer passed installation, reinstallation, Unicode-path, shortcut,
registration, packaged-calculation and uninstall checks, preserving a user
project. The downloaded installer checksum and all 1,299 portable payload files
matched its build record.

One ARM64 Mac [candidate run](https://github.com/Ameyanagi/rexafs/actions/runs/34421683944/attempts/1) reported an unexpected end of record while FEFF10
read `gg.bin`; the second test using the same input passed. Both fitting tests
passed ten consecutive repetitions on Linux ARM64. The failed Mac log was
retained and the unchanged native job rerun; no test or tolerance was disabled.
Final native Mac and tagged-build results belong to the release evidence below.

## Final build and publication

The [release page](https://github.com/Ameyanagi/rexafs/releases/tag/v0.2.4) records
the immutable tag commit and exact manually dispatched build. The
[release workflow](https://github.com/Ameyanagi/rexafs/actions/workflows/release-build.yml)
qualifies all four desktop targets, 20 Python wheels, the source distribution,
Rust crate, npm package, licenses, installer checks, Linux graphical smoke and
complete manifest. Desktop executables and bindings use optimized release builds.

The release body also identifies the Mac signing run, ZIP/DMG provenance and
registry hash comparisons. Its `SHA256SUMS` covers the public desktop downloads.
Signing consumes the qualified Mac archives without rebuilding their executables.
Local development packages and pull-request artifacts are verification outputs;
publication requires the successful build of the exact release tag.

## Platform limits

Windows and Linux downloads remain x86-64 desktop previews. Linux x86-64 CI
uses X11 software rendering; the additional local ARM64 candidate uses the
GB10 hardware device through a virtual X11 display. Physical monitors, native
Wayland and clean-machine graphical setup remain unqualified. Linux archives
use the Ubuntu 24.04 baseline and reference glibc 2.39. No native Linux or Windows
ARM64 download is included; Windows ARM64 has not been built or tested.
Windows release CI validates core tests, native compilation, desktop diagnostics
and installation/reinstallation/uninstallation. The retained native Windows
interaction report covers the earlier development build, not new text editing
and empty-workspace controls. Mac signing CI checks signed downloads and temporary
installation; interactive Mac qualification is outside this Linux host.

[PR #39](https://github.com/Ameyanagi/rexafs/pull/39) remains excluded due to its
solver dependency incompatibility. [Issue #20](https://github.com/Ameyanagi/rexafs/issues/20)
is addressed by the [numerical compatibility fixes](../../fft-grid-compatibility.md).
