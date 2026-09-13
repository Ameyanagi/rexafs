# rexafs 0.2.5 release qualification

Status: preparation. No 0.2.5 tag, registry package or desktop release has been
published. This record distinguishes planned gates from completed evidence.

## Source and version propagation

[PR #58](https://github.com/Ameyanagi/rexafs/pull/58) contains the browser processing
preview, ARM64 release targets, documentation cleanup and coordinated 0.2.5
preparation. Cargo's workspace version supplies the four Rust package versions,
Python extension metadata and desktop identity. The npm manifest and lockfile
carry the same version. CI validates these sources, the workspace dependency,
Cargo.lock and the Python dynamic-version contract before building.

The public website remains on 0.2.4 until all 0.2.5 distributions are available.
Its later promotion must verify registry/archive hashes, update release metadata
and install examples, regenerate Stable API references from the new immutable
tag, and preserve screenshots' actual 0.2.4 capture labels. The browser preview
records its deployed source commit and WASM hash independently.

## Qualification gates

- Complete PR checks at the final source revision before merge.
- Retain linked and embedded 0.2.5 projects written and reopened by this version;
  preserve all historical fixture bytes and checksums.
- Create the immutable version tag only after the reviewed preparation is merged.
- Run the complete release workflow manually on that tag. Publication requires
  its exact commit, version, successful conclusion and original artifact hashes.
- Qualify native Linux and Windows ARM64 archives. Record native rexafs/ReFEFF
  separately from the Windows x64 FEFF10 helper's emulated execution.
- Sign and notarize the qualified Mac archives, build their DMGs and verify final
  outputs. Replace unsigned Mac assets in the draft and regenerate public hashes.
- Publish the GitHub-built Rust, Python and npm packages; verify versions and
  published bytes, then publish the complete desktop release.
- Promote website downloads and Stable API references only after those checks.

## Completed evidence

Local preparation on Apple Silicon macOS with Rust 1.98.1:

- The actual 0.2.5 fixture writer saved and reopened linked and embedded projects.
  All **32 retained samples** pass checksum/header validation; every historical
  sample retains its original hash. The focused project suite passed **26 tests**
  with the explicit fixture writer excluded from ordinary runs.
- Cargo.lock changes only the four workspace package versions. The coordinated
  version/tag check and Rust formatting check pass.
- **70 release-tooling tests across nine suites** passed, including mixed ARM64
  runtime selection, required normal/delayed imports, architecture rejection and
  version drift. Ruff, ty and workflow lint passed.
- Both WASM targets built. The npm package passed **13 runtime/editor tests**.
  A freshly installed CPython 3.12 wheel reports 0.2.5, passes **14 API tests**,
  and passes installed Pyright completion, hover and signature-help checks.
- Stable 0.2.4 and checkout 0.2.5 Rust references built. The production website
  build, **10 content/input tests**, **2 reference-generator tests**, and
  **10 browser/accessibility tests** passed at the custom-domain root. Astro
  reported zero errors, warnings or hints.

The preceding revision's [PR build 34749982035](https://github.com/Ameyanagi/rexafs/actions/runs/34749982035)
passed both native Linux desktop targets. Windows ARM64 compiled and passed its
desktop/engine tests but failed while packaging an x64-only Microsoft runtime
DLL found in the ARM64 redistributable directory. That failed revision cannot
qualify a release; the corrected runtime selection requires a fresh native run.
The fix omits only an unused x64 `vcruntime140_1.dll` after checking the native
app and selected runtime's normal and delayed imports. Other machine mismatches
still fail. Microsoft documents the companion's
[x64 exception-handling role](https://devblogs.microsoft.com/cppblog/making-cpp-exception-handling-smaller-x64/)
and [ARM64X's native default view](https://learn.microsoft.com/en-us/windows/arm/arm64x-pe).

The preceding browser/docs implementation passed production builds, numerical
browser/Node comparisons, content and accessibility checks. The ARM64 release
tooling passed its local regression suite. These results are recorded in the
[documentation audit](../../documentation-audit-2026-09-13.md); they do not replace
the final versioned build, native qualification or registry verification.

Build, signing, publication and live website evidence will be appended as those
steps complete. Local development packages are verification outputs only.
