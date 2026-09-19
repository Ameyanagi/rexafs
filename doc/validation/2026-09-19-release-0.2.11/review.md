# rexafs 0.2.11 qualification record

Release preparation is in progress. This record does not yet establish public
availability. The preceding published version is 0.2.10.

## Source and scope

[PR 94](https://github.com/Ameyanagi/rexafs/pull/94) contains RMC transform-range
inheritance, larger absorber-context allocation, identical-input preparation
reuse, CPU controls, calibration, coordinate refinement and optional periodic
ΔE₀ optimization with fixed S₀². Its final feature candidate is
`f38b5a931ffa9e5212a0f125debee82f3a67cec8`.

All 48 feature checks passed, including the release workflow's second attempt.
PR 94 was squash-merged into `dev` at
`cbb6b6cb0269911799edbd8ddc51d403001cfd5b`. The merged tree equals the tested
feature tree. Release preparation is based on that merged commit; promotion to
`main` remains pending the FEFF10 correction below.

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
Website Stable metadata remains unchanged until packages and desktop downloads
have been verified publicly.

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
The fix requires new native archives and a rexafs dependency/helper update.
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
qualification still requires the promotion and exact-tag release matrix.

## Pending qualification

- Dev → main promotion checks and merge identity.
- Immutable tag and successful manually dispatched full release build.
- Package checksums and trusted publication to crates.io, PyPI and npm.
- Signing, notarization and installed Mac checks using the exact build archives.
- Public desktop asset verification, release publication and Stable documentation.

Experimental research inputs, derived results and detailed timing records stay
outside the repository. Windows/Linux preview and physical-convergence limits
remain explicit in the release notes.
