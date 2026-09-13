# rexafs 0.2.5 release qualification

Status: the tagged build and native ARM64 qualification passed; crates.io, PyPI
and npm publication and installed-package verification are complete. PR #58 is
merged and `v0.2.5` identifies `50d59e2147e12a98ace9a9e17df872a910e9ca24`.
Mac signing, installer and graphical checks passed. The public desktop release
contains all 27 verified assets. Website metadata and references are prepared
and locally verified for deployment; this record precedes the live-site check.

## Source and version propagation

[PR #58](https://github.com/Ameyanagi/rexafs/pull/58) contains the browser processing
preview, ARM64 release targets, documentation cleanup and coordinated 0.2.5
preparation. Cargo's workspace version supplies the four Rust package versions,
Python extension metadata and desktop identity. The npm manifest and lockfile
carry the same version. CI validates these sources, the workspace dependency,
Cargo.lock and the Python dynamic-version contract before building.

Website metadata and install examples now identify the published 0.2.5 release.
Stable API references were regenerated from the immutable tag and verified
published crate; screenshots retain their actual 0.2.4 capture labels. The browser preview records its deployed source commit and WASM
hash independently.

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

The final PR revision `eb663a878d4c8c10271c0690b3fdbee5bda02380` passed all
42 checks; deployment was correctly skipped for the pull request. PR #58 merged
at 2026-09-13 11:33:11 UTC. The merge commit's tree matches that tested revision.
The annotated `v0.2.5` tag points to the merge commit, and the coordinated version
check passed before tagging. The exact-tag manual
[release build 34754751069](https://github.com/Ameyanagi/rexafs/actions/runs/34754751069)
is the sole artifact source for this release. The main-branch Rust, Website and
Windows installer workflows also passed at the tagged commit.

### Tagged build and native ARM64

Release build `34754751069`, attempt 1, completed with **all 31 jobs successful**
at the exact tagged commit. Its `release-manifest/SHA256SUMS` contains 45 unique
entries and has SHA-256
`44a91fefe03f0feeb781fd20bcd2a9d41b389b63de89b7f135a2641ae0fdcc7f`.
The native artifact review matched all nine ARM64 distribution, installer,
sidecar and GUI-evidence files to that manifest and rechecked the immutable tag.

- **Linux ARM64:** the archive contains an AArch64 ELF64 executable and native
  ReFEFF/FEFF10. The job passed 462 desktop tests (5 ignored), 5 engine integration
  tests, extracted-package and both Cu engine checks, and all 10 GUI checks.
  Opened-project, transform, fit and 960 × 640 screenshots were reviewed.
  Qualification used Ubuntu 24.04, X11/Xvfb and Mesa software Vulkan; native
  Wayland and physical GPU behavior were not exercised.
- **Windows ARM64:** the desktop and all nine bundled Microsoft CRT DLLs report
  native ARM64 (`0xAA64`). Runtime hashes match the CI-recorded Microsoft-signed
  provenance. All 1,320 installer payload hashes match the archive, and all 14
  native-host installation checks passed, including installed engine execution,
  reinstall, uninstall and preserved user data. The job passed 458 desktop tests
  (5 ignored), 5 engine integration tests, 9 archive tests and 27 installer tests.
  ReFEFF is native; FEFF10's five pinned x64 helper/runtime files use Windows 11
  emulation. The installer remains unsigned, and the hosted runner did not
  qualify interactive Windows GUI behavior.

### Published packages

Registry verification at **2026-09-13 13:47:40 UTC** confirmed 0.2.5 as the latest
version on crates.io, PyPI and npm. Published bytes match the qualified build
manifest, including all **21 Python distributions**: 20 CPython 3.10–3.14 wheels
for macOS ARM64/x64 and Linux/Windows x64, plus the source distribution.

| Published artifact | SHA-256 |
| --- | --- |
| Rust crate | `711e1749fe9418ac1b187584a48707cb6970d4a14dddd78c5fc5e36f7bd50679` |
| npm tarball | `a4ee304ea4b752b91a40e6fd9fecf2141488eaa91cbd64a1280f16233d80d102` |
| CPython 3.12 macOS ARM64 wheel used below | `4bbbb497c21d540978f1fc5a87324f4a5b66fedcd854f1daffa835d5f2b44981` |

Fresh public PyPI/npm installations passed on macOS ARM64 with CPython 3.12.12,
NumPy 2.5.3, Node 24.19.0, TypeScript 5.9.3, Pyright 1.1.414 and Chromium
153.0.8010.12. All six installed Python files and 26 npm files matched their
public archives; npm's registry SHA-512 integrity also matched. No package was
rebuilt or repacked for these consumer checks.

- All 14 Python API tests and the installed-wheel Pyright contract passed.
- All 10 npm runtime tests and 3 TypeScript editor tests passed, along with the
  authored Python, Node and browser guide examples, both strict TypeScript
  compilation modes, and two real-browser scenarios.
- All 326 browser Fourier rows matched Node exactly. Python/Node maximum
  absolute differences were `4.93e-13` for `chir_mag()` and `1.51e-14` for
  `chiq()`, in each output's units, passing `atol=rtol=1e-10`. Grid arrays matched
  exactly; ownership and inverse-result invalidation checks passed.

These clean consumer checks cover the stated local environment and published
package bytes. Platform coverage comes from the tagged CI jobs; the checks did
not exercise the public website.

Raw registry identities and hashes are retained locally in
`/tmp/rexafs-0.2.5-registries/verification.json`. Consumer commands, results and
logs are in `/tmp/rexafs-0.2.5-published-consumers/`, with `review.md` and
`verification.json` as indexes. Native job logs, artifact metadata, manifest and
tag checks are in `/tmp/rexafs-0.2.5-native-review/`, indexed by `REVIEW.md`.
These temporary local records are not public release attachments.

### Signed Mac installers and draft assets

[Signing run 34760514667](https://github.com/Ameyanagi/rexafs/actions/runs/34760514667)
passed both architectures. Each signed ZIP records its original qualified ZIP
hash, tagged source, build run and signing run. Local verification required
Apple team `XXN44W8X56`, Developer ID signatures, hardened runtime, notarization,
stapling and Gatekeeper acceptance. Both DMGs were mounted read-only; temporary
installed copies passed architecture/build-identity checks, the processing
self-check and both packaged Cu calculation-engine checks. Intel execution used
Rosetta on the Apple Silicon review machine.

| Signed installer | SHA-256 |
| --- | --- |
| macOS ARM64 DMG | `b2c539baff9d00acc1bb70c12c1d19446db4044a65c35b90245b8375ac7b114e` |
| macOS Intel DMG | `542d2c09f3f782bd02ac534ed4c16c88d864374357d673b2c3c9f9428bb31b6e` |

Before publication, the GitHub draft contained exactly 26 desktop assets plus
`SHA256SUMS`. Its inventory, uploaded digests and fresh downloaded bytes matched
the locally qualified files. The final desktop manifest has SHA-256
`0bf7346fa76065f7e8c3bd61eb59d0a8fc5fc9c54effb044c03c007fc0e8b5be`.
Local receipts are `/tmp/rexafs-0.2.5-signed-artifacts/verification.json` and
`/tmp/rexafs-0.2.5-draft-verification/verification.json`.

### Signed Mac graphical checks

Graphical checks completed at **2026-09-13 14:21:41 UTC** using the signed,
notarized extracted apps with verified identities and executable hashes.

- **ARM64:** the complete bundled Cu workflow passed: import, normalization,
  AUTOBK, Fourier transform, structure construction, ReFEFF paths, first-shell
  fitting, embedded-project save and figure/data/analysis-folder exports. After
  removing access to the original FEFF workspace, the portable project reopened
  with its model and fit history, and refitting passed.
- **Intel under Rosetta:** the actual translated x64 process opened that portable
  project, rendered processing results, refitted the restored model and exported
  its analysis folder. Both architectures produced 46 export files, including
  11 figures; all SVGs parsed and all CSV numbers were finite.

Portable projects retain source bytes, paths, model and fit history; live curve
arrays are recomputed by refitting. Intel's export correctly reported unavailable
cached arrays for the archived first fit. Fresh Intel import and path calculation
were not exercised through the GUI; separate exact-binary checks qualified both
packaged engines. This is workflow qualification, not validation of a unique
physical interpretation.

Computer-use attachment briefly opened two extra empty instances without a
confirmed settings override; both were closed. The real settings file's mtime
and size stayed unchanged, with no observed content write. Permission metadata
changed consistently with read-time `chmod 0600`; this is not evidence of fully
isolated settings access. No qualification app processes remained. Screenshots,
exports and these limitations are indexed by
`/tmp/rexafs-0.2.5-signed-gui-jyotnnto/gui-checks.json`.

### Public desktop release

[Version 0.2.5](https://github.com/Ameyanagi/rexafs/releases/tag/v0.2.5) became
public at **2026-09-13 14:22:21 UTC**. Post-publication verification completed at
**14:22:32 UTC**: all 27 downloaded public asset hashes matched the qualified
release files, including the final desktop `SHA256SUMS` hash above. The receipt is
`/tmp/rexafs-0.2.5-public-verification/verification.json`.
This completes desktop publication; website promotion and live browser checks
remain separate pending gates.

### Website promotion prepared for deployment

After publication, unauthenticated downloads of all six platform packages and
six checksum sidecars matched the qualified files. The local `release.json`
records 0.2.5, the immutable source commit and the published Rust crate checksum.
Python references contain 89 documented members and TypeScript 144 in each
Stable/Next channel. Stable declarations come from `v0.2.5`; Rust HTML was built
from the checksum-verified published crate, separately from the checkout API.

Both root and `/rexafs` production builds passed all 22 content/input/staging
tests and all 21 browser tests. The two generator tests passed; Astro reported
zero errors, warnings or hints across 35 files. Desktop and 390-pixel screenshots
show three OS cards, six unique downloads, readable Windows ARM64 requirements
and 0.2.5 library commands. All 146 checked relative documentation file links
resolve. Scientific browser evidence is detailed in the
[ReFEFF review](../2026-09-13-browser-refeff/review.md).

Local receipts are `/tmp/rexafs-0.2.5-public-download-verification.json`,
`/tmp/rexafs-0.2.5-final-rustdoc.log` and
`/tmp/rexafs-0.2.5-final-website-qa/verification.json`. These are pre-deployment
checks. The final PR must pass CI before merge; production identity, downloads
and calculations are checked against the resulting deployment.

### Earlier local preparation

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
desktop/engine tests but failed while packaging a Microsoft runtime companion
whose PE header reports x64 (`0x8664`) in the ARM64 redistributable directory.
That failed revision did not qualify a release; the successful tagged native run
above qualifies the corrected runtime selection.
The fix omits only an unused x64-compatible `vcruntime140_1.dll` after checking
the native app and selected runtime's normal and delayed imports. Other machine
mismatches still fail. Microsoft documents the companion's
[x64 exception-handling role](https://devblogs.microsoft.com/cppblog/making-cpp-exception-handling-smaller-x64/)
and [ARM64X's native default view](https://learn.microsoft.com/en-us/windows/arm/arm64x-pe).

The preceding browser/docs implementation passed production builds, numerical
browser/Node comparisons, content and accessibility checks. The ARM64 release
tooling passed its local regression suite. These results are recorded in the
[documentation audit](../../documentation-audit-2026-09-13.md); they do not replace
the final versioned build, native qualification or registry verification.

Live website results belong to the final website PR and its deployment review.
Earlier local development packages remain verification outputs only; the
consumer checks above used published packages.
