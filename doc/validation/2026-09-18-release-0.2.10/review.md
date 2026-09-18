# rexafs 0.2.10 qualification record

Published on 18 September 2026. All seven registry files and 27 desktop assets
were downloaded and verified against the qualified build and signed Mac outputs.
This record distinguishes source preparation, immutable-tag build, signing and
public-artifact checks.

## Source and scope

The release starts from `dev` commit
`4d211b973297b23dbbf7baaa6787720322374cdd` (PR 89), incorporating PRs 85–89.
Main's existing promotion commit is merged back before preparation to retain
shared branch history. The [release notes](../../release-notes-0.2.10.md) describe
RMC, analysis, processing, Series and experimental Live workflows.

The preceding RMC integration passed 48 CI checks. On its combined source,
457 core tests and 618 desktop tests passed locally, along with strict core
Clippy and an optimized build. Native macOS accessibility interaction opened the
saved 5,396-attempt Cu₂O checkpoint and the current Cauchy Wavelet workspace.
Those checks identify the preceding development source, not the release tag.
The Cu₂O residual remained StillChanging; no converged refinement is claimed.

## Release preparation

Coordinated Cargo/npm versions advance to 0.2.10; Python inherits Cargo's version.
API source documentation identifies the introduced version. Website Stable
metadata remained on 0.2.9 during preparation and publication checks.
The maintainer writer produces new linked and embedded format-1 fixtures without
rewriting historical files. The maintainer writer passed; all 29 project tests
passed, including loading, saving and reopening every retained release fixture.
The fixture manifest validates 42 samples. Coordinated version, formatting and
whitespace checks passed. Python/TypeScript references and citations were
regenerated; all eight documentation-generator tests passed. Stable Rustdoc
was restored from its verified published-crate cache, Next Rustdoc built
successfully, and the website source check reported zero errors or warnings.

## Promotion qualification

[Promotion PR 90](https://github.com/Ameyanagi/rexafs/pull/90) targets protected
main from dev. Its final candidate is
`4deec10eef7f7e8efef0f21389fe38a5803d6633`, including corrected package README
scope. An earlier documentation-only candidate was superseded before tagging.
The [candidate Larch comparison](https://github.com/Ameyanagi/rexafs/actions/runs/35312120809)
passed 966 production fits with worst prototype-relative error 1.33 × 10⁻¹².
Both candidate website workflows passed. All 65 current-head checks passed,
with three intentional skips, no unresolved review threads and no source change
after qualification. Both Rust workflows passed; all four processing benchmark
medians were below baseline, with the existing 25% regression threshold unchanged.
All six desktop targets and all 20 Python runtime combinations passed.

The protected PR merged with a merge commit on 18 September 2026 at
`0b3aca3331e38ac4a34364824a74e96271f5ca1d`; its tree is identical to the reviewed
candidate. The immutable `v0.2.10` tag identifies that commit.
[Manual exact-tag build 35316272629](https://github.com/Ameyanagi/rexafs/actions/runs/35316272629)
passed all 37 jobs. All 33 original build artifacts matched its SHA-256 manifest.
PR and nightly artifacts were not public release inputs.

## Signing and desktop qualification

[Signing run 35320715055](https://github.com/Ameyanagi/rexafs/actions/runs/35320715055)
signed and notarized the original Apple Silicon and Intel archives without
rebuilding their executables. Both ZIPs and copied DMG applications passed
strict signature verification, stapling, Gatekeeper, version, spectrum and
ReFEFF/FEFF10 self-checks. Team ID: `XXN44W8X56`; Hardened Runtime is enabled.
Local execution was native ARM64 and Intel through Rosetta.

The first ARM64 signing attempt passed notarization but FEFF10 reported an
unexpected end of record reading `gg.bin`. This matches the intermittent error
retained in the [0.2.4 record](../2026-09-09-release-0.2.4/review.md).
The original downloaded executable passed locally, including five additional
consecutive FEFF checks. The unchanged failed signing job passed on retry;
no test or tolerance was disabled. The final signed ZIP and installed app each
passed again locally. Both attempts remain in the signing run history.

Native macOS accessibility interaction with the signed, DMG-installed application
opened a private copy of the existing Cu₂O project. It retained 5,396 attempts,
best normalized objective 0.03488986888 and the StillChanging diagnosis. The
k/R plots, recent convergence history and retained Cauchy Wavelet workspace
rendered without errors. The saved-input warning correctly identifies that
resume uses the frozen problem. These are checkpoint/release checks, not a new
converged scientific result; the original project was preserved.

The GitHub draft's two unsigned Mac archives were replaced by the qualified
signed outputs, both DMGs and evidence were added, and the desktop manifest was
regenerated. All 27 draft assets matched staged sizes and SHA-256 hashes before
[desktop publication](https://github.com/Ameyanagi/rexafs/releases/tag/v0.2.10).
All 27 public assets were then downloaded without authentication and matched
the qualified bytes and final manifest. GitHub Latest resolves to `v0.2.10`.

## Published packages and consumer checks

The [Rust](https://github.com/Ameyanagi/rexafs/actions/runs/35320737785),
[Python](https://github.com/Ameyanagi/rexafs/actions/runs/35320740051),
[npm](https://github.com/Ameyanagi/rexafs/actions/runs/35320742145) and
[desktop draft](https://github.com/Ameyanagi/rexafs/actions/runs/35320744160)
publication workflows passed. npm initially held the accepted upload for registry
processing, then made it available. All seven files—one crate, four ABI3 wheels,
one sdist and one npm tarball—were downloaded without registry credentials and
matched the exact-tag build manifest.

A fresh Rust consumer resolved rexafs 0.2.10 from crates.io, with the published
checksum and ReFEFF 0.4.0. The prepared RMC example completed 80 synthetic trials,
checkpoint/cold resume and report exports; objective 0.016840 → 0.000029. This is
an API smoke test, not a converged experimental fit.

A fresh Python 3.12 environment installed the public ARM64 wheel with NumPy
2.5.3. All 38 tests and 41 subtests passed, covering processing, measurement,
MBACK, peaks, fluorescence and wavelets. Installed-package editor hovers,
completion, signatures and intentional error diagnostics passed. The website's
public Cu example ran with finite Fourier and inverse-transform arrays.

All 38 Node/browser package tests passed against the downloaded npm bytes,
including installed TypeScript consumer/editor checks. The public Cu Node guide
example also ran successfully.

## Stable documentation

Stable metadata and install links advance to 0.2.10 only after those public
checks. The Rust reference uses the downloaded crate and its verified checksum.
The desktop manual adds a complete RMC setup, result and checkpoint guide, with
updated processing, Wavelet, peak fitting and experimental Live documentation.
The website built 230 pages and indexed 1,889 HTML files. Source checking
reported no errors or warnings; all eight generator tests, 23 content/engine
tests and 25 browser tests passed. The Python and Node guide examples ran against
the verified published packages. Documentation is promoted through a separate
protected dev → main pull request.

Windows/Linux desktop and Live acquisition limits remain explicit. Native macOS
qualification does not establish behavior on physical Windows/Linux graphics,
network shares or every experimental dataset. Adaptive RMC remains experimental
and opt-in.
