# rexafs 0.2.11 qualification record

Version 0.2.11 was published on 19 September 2026 and is marked Latest on
[GitHub](https://github.com/Ameyanagi/rexafs/releases/tag/v0.2.11). Packages and
desktop bytes were verified independently after publication.

## Source and scope

[PR 94](https://github.com/Ameyanagi/rexafs/pull/94) contains RMC transform-range
inheritance, larger absorber-context allocation, identical-input preparation
reuse, CPU controls, calibration, coordinate refinement and optional periodic
ΔE₀ optimization with fixed S₀². Its final feature candidate is
`f38b5a931ffa9e5212a0f125debee82f3a67cec8`.

All 48 feature checks passed, including the release workflow's second attempt.
PR 94 was squash-merged into `dev` at
`cbb6b6cb0269911799edbd8ddc51d403001cfd5b`. The merged tree equals the tested
feature tree. Release preparation is based on that merged commit and includes
the FEFF10 correction below.

The candidate passes 61 focused native tests, strict ReFEFF-enabled core Clippy,
formatting, an optimized desktop build and website diagnostics locally. Native
computer-use checks with the public Cu example verify CPU validation, a ten-worker
run, save/reload and checkpoint continuation. These are source-checkout checks;
they do not qualify future tagged artifacts or establish physical convergence.

## Release preparation

Cargo and npm advance together to 0.2.11; Python inherits Cargo’s version.
New linked and embedded format-1 fixtures retain explicit worker and energy
controls. Historical fixture bytes are preserved. Source API documentation
identifies the introduced version. The maintainer writer passed; all 29 project
round-trip tests passed, with the maintainer operation ignored in the ordinary
suite. All 44 retained samples passed manifest validation. Coordinated-version,
formatting and whitespace checks passed, as did seven version-tool tests, three
release-maintenance tests, 19 CI-scope tests and eight documentation-generator
tests. Website diagnostics reported zero errors, warnings or hints.

The release matrix now runs the dedicated RMC scattering integration suite on
all six desktop targets, alongside the existing FEFF runner/fitting checks.
The same release-mode command passed locally: seven RMC, three ReFEFF-runner
and two FEFF10 amplitude/parity tests.
Website Stable metadata was retained at 0.2.10 during preparation and advanced
only after packages and desktop downloads were verified publicly.

## FEFF10 dependency failure

The first [feature build](https://github.com/Ameyanagi/rexafs/actions/runs/35406642227)
failed in the Linux ARM64 archive's `--self-check-feff` operation. FEFF10's
`mkgtr` stage reported an unexpected end of record in `gg.bin`; the preceding
624 desktop tests, two FEFF10 integration tests and three ReFEFF integration
tests had passed. This matches the error retained in the
[0.2.10 signing record](../2026-09-18-release-0.2.10/review.md#signing-and-desktop-qualification).
Five consecutive checks of the unchanged feature executable passed locally on
an ARM64 Mac, and one unchanged CI retry was requested after all other jobs
passed. These passing samples do not fix the defect.
The unchanged retry passed all Linux ARM64 tests, archive/self-check operations
and GUI smoke checks. Its manifest and final release aggregate also passed.

Further isolated stage runs reproduced the exact error and retained the failed
`gg.bin`. FEFF10's `WriteComplex2D` writes an uninitialized four-byte format label
when its optional format argument is absent. A newline among those bytes splits
the header and makes MKGTR read its remainder as array data. Replacing only those
labels recovered the complete calculation, with a first-shell path file
byte-for-byte identical to the successful baseline.

[feff10-rs PR 4](https://github.com/Ameyanagi/feff10-rs/pull/4) initializes the
numeric-array format defaults in the copied Fortran build sources. Its new
header assertion fails against the existing 0.2.3 archive. The corrected native
build passed four Cu worker/automatic-isolation runs, the four active integration
tests (including Cu EXAFS and Cu/BN XANES reference comparisons), strict Clippy
and formatting locally. Twelve extended integration cases remained ignored.
Rexafs's release-mode FEFF10 amplitude-stability and ZnSe fitting comparisons
also passed against the corrected local native archive (two tests).
The desktop package self-check now validates every generated `gg.bin` array
header, so an older archive cannot pass merely because its undefined bytes
happen to be readable. Both focused regression tests passed in release mode,
including malformed labels and valid LF/CRLF line endings.
The fix required new native archives and a rexafs dependency/helper update.
No check or tolerance was disabled.

All seven checks on FEFF10's versioned candidate
`bbec441d493b1d1cfb5d211af728c243a61a43d5` passed, including macOS, Windows
and Intel-compiler Linux tests, installed Python wheels and review. PR 4 merged
at `343744557e5acbd029811db8fc41e22b5f95def0`, with the same tree as the checked
candidate. The immutable `v0.2.4` tag started
[release build 35411760951](https://github.com/Ameyanagi/feff10-rs/actions/runs/35411760951).
All six native builds and their clean-runner smoke tests passed. GitHub published
the 18 release assets on 19 September 2026; every downloaded file matched the
release manifest and GitHub digest. All 20 jobs in the release workflow passed.
The three Rust packages and five Python wheels published successfully; public
wheels matched the original CI artifacts, and downloaded crates matched the
registry checksums.

The rexafs candidate now selects FEFF10 0.2.4 and pins the matching Windows
executable by its verified SHA-256. The four runtime DLL checksums are unchanged.
With no native-library override, all 12 release-mode native integration tests
passed against the published dependency: seven RMC, three ReFEFF-runner and two
FEFF10 amplitude/fitting tests. The nine archive-tool tests also passed.
Both focused desktop header tests passed against the published archive. The
optimized desktop executable passed `--self-check-feff` for both engines,
including the new header guard. The Windows helper downloader independently
verified all five public files and their x64 PE headers. Cross-platform rexafs
qualification is recorded in the promotion and exact-tag release sections below.

## Promotion and tagged build

[PR 95](https://github.com/Ameyanagi/rexafs/pull/95) passed all 67 checks with no
failures before its merge into `main`. The merge commit is
`70f61e81262f9ac593f813c895ed843aa23ee640`; its tree equals the checked
`441e3baca805aecda53ada0579822eed146a824d` candidate. The immutable `v0.2.11`
tag points to that merge. `dev` was fast-forwarded to retain the shared history.

The manually dispatched [tagged build 35415279391](https://github.com/Ameyanagi/rexafs/actions/runs/35415279391)
uses that exact commit. All 37 jobs passed: source/package checks, four Python
wheels and all 20 CPython runtime combinations, npm, six native desktop targets,
the complete artifact manifest and the final aggregate. Desktop jobs include
project compatibility, FEFF10/ReFEFF/RMC integration, extracted-archive checks
and platform installer/update qualification. Main and dev Rust CI, the website
workflow, Windows installer workflow and the same-source nightly also passed.

All 33 original GitHub-built files were downloaded and matched the complete
build manifest. The crate's retained VCS metadata identifies the exact tag
commit and a clean checkout. Registry packages retain this original manifest;
the final desktop manifest is regenerated after replacing the unsigned Mac
archives with signed outputs.

[Mac signing run 35418251336](https://github.com/Ameyanagi/rexafs/actions/runs/35418251336)
uses the successful tagged build's archives without rebuilding the executables.
Both architecture jobs and their source gate passed. Both archives and DMGs
were signed, notarized and stapled, and passed installed-app checks. The final
manifest includes the signed archives, DMGs and signing evidence. On an ARM64
Mac, a fresh DMG copy passed strict codesign, stapler, Gatekeeper, build-info,
`--self-check` and `--self-check-feff` checks for both scattering engines.

## Public packages and downloads

The [crates.io](https://github.com/Ameyanagi/rexafs/actions/runs/35418495756),
[PyPI](https://github.com/Ameyanagi/rexafs/actions/runs/35418496993) and
[npm](https://github.com/Ameyanagi/rexafs/actions/runs/35418498418) publication
workflows all succeeded. All seven files downloaded from those public registries
matched the original CI manifest: the crate, four ABI3 wheels, Python source
archive and npm archive. The crate SHA-256 is
`79de6625f6067ef2d7c5676303797d467e234036cbb03da4e8d0a96d90aea108`.
Fresh Python 3.12 and Node installations ran the website's Cu examples.

The desktop release was published at 03:37:42 UTC on 19 September 2026.
All 27 assets were then downloaded without authentication; every digest and
size matched the staged signed release and GitHub metadata. The final
`SHA256SUMS` covers the other 26 assets, including sidecars and signing evidence.
No source tag was moved and no binary was rebuilt during signing or publication.

## Signed desktop and documentation captures

Computer use controlled a separate process installed from the signed ARM64 DMG,
with isolated preferences and the public CC0 room-temperature Cu measurement.
The executable SHA-256 is
`970e72651b78a740cfdda24086baa286654fcd7896493a9c429f2ced6bcbd162`.
Three original 1192 × 768 JPEG captures and their full source, input and image
provenance are retained in
[the capture manifest](../../../website/public/screenshots/0.2.11/capture.json).

The check imported the stored transmission signal, processed the spectrum and
set Transform k = 2–12 Å⁻¹ and Back FT R = 1.5–3.5 Å. A 2 × 2 × 2 curated Cu
cell contained 32 atoms. RMC inherited the ranges, offered Auto = 10 CPU workers
and automatic 0.05 Å moves. With two workers, ten attempts, ΔE₀ bounds ±1 eV and
an update interval of five attempts, it completed without errors and retained
S₀² = 1.000. The best shift was +1.00 eV; the bound warning was visible. Run
details retained two workers, parallel paths, one calculated and 31 shared
electronic preparations. Saving and reopening the portable project retained
the curves, ten-attempt checkpoint and matching energy/amplitude values without
starting a calculation.

The initial default ±15 eV refinement bounds correctly blocked k minimum 2 Å⁻¹
because they would remove the Fourier taper; the interface requested a higher
k minimum or revised calibration. Narrow bounds were chosen only for this short
software check. Its best objective decreased from 6.4641092 to 5.0267694, but
**convergence was not assessed** and the amplitude mismatch remained. These
captures demonstrate controls and persistence, not a calibrated or converged Cu
structure. The existing scientific reference tests provide separate numerical
evidence.

The updated website passed Astro diagnostics with zero errors, warnings or hints,
eight generator tests, 23 content/runtime tests and 25 browser tests. Stable
Rust reference pages were regenerated from the checksum-verified published
crate; Python and TypeScript pages retain the exact released declarations.
Computer use verified the rendered RMC guide and new screenshot captions.

Experimental research inputs, derived results and detailed timing records stay
outside the repository. Windows/Linux preview and physical-convergence limits
remain explicit in the release notes.
