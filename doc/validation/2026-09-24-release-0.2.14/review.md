# rexafs 0.2.14 qualification

Version 0.2.14 was published on 24 September 2026 in Japan (23 September UTC).
This record separates source checks, final artifact qualification and publication.

## Source and scope

[PR #127](https://github.com/Ameyanagi/rexafs/pull/127) adds the compact Live
monitor, simultaneous signals and averages, reference processing, automatic EXAFS
fits, Series parameter expressions, plot ranges and optional Assistant app/file
access. It was squash-merged into `dev` as
`c402471b556613670f476d9097ee82e1994c8164` after its selected checks passed.

The tested head was `e9c6c56428231fbf489a26596c983d6fab9920a3`.
The [release matrix](https://github.com/Ameyanagi/rexafs/actions/runs/35818439041)
passed all five desktop targets, both Windows installers and all 15 Python
runtime combinations. The [Rust checks](https://github.com/Ameyanagi/rexafs/actions/runs/35818439056)
and [website build](https://github.com/Ameyanagi/rexafs/actions/runs/35818439045)
also passed. Website deployment was intentionally skipped for the pull request.

The initial Windows run exposed a test fixture using the Unix-only absolute
path `/tmp`. The corrected test uses a native temporary directory and verifies
the specific batch-size error. The subsequent Windows x64 and ARM64 jobs passed;
no check was disabled. All 18 Live tests and repository hooks also passed locally.

The [Live monitor review](../2026-09-23-live-monitor/README.md) and
[Series/replay review](../2026-09-23-series-fit-live/README.md) retain original
computer-use captures and numerical checks. Those captures keep their original
0.2.12 preview identities. They are not presented as measurements of the final
signed 0.2.14 application. The ten-minute replay used repeated Cu fixture bytes,
not independent beamline measurements or evidence of improved signal-to-noise.

## Local preparation

On Apple Silicon macOS, the optimized 0.2.14 writer generated new linked and
embedded fixtures with both FEFF features enabled. The project suite passed
31 tests with two intentional ignores, including load/save/reopen of the entire
retained collection. The fixture checker verified all 50 samples and their
recorded hashes. Earlier fixture files were not regenerated.

Coordinated version validation, all seven version-check regressions and all
12 release-tooling suites passed. Python and TypeScript references and the
citation index were regenerated without changing the published 0.2.13 metadata.
The first TypeScript generation attempt needed this isolated worktree's website
dependencies; generation succeeded after making the installed dependencies
available. This environment setup did not change the package lockfile.

## Release preparation and promotion

[PR #128](https://github.com/Ameyanagi/rexafs/pull/128) coordinates the 0.2.14
versions, new project fixtures, release notes and source-guide availability
labels. Its final head, `63d2e8010abaf2cfb22f5435bdd8316eb1e60cdd`, passed all
41 active checks, with only the expected website deployment skip. The
[release matrix](https://github.com/Ameyanagi/rexafs/actions/runs/35910265737)
passed every desktop, Windows installer/update, package and Python runtime job.
[Rust](https://github.com/Ameyanagi/rexafs/actions/runs/35910265635),
[Larch comparison](https://github.com/Ameyanagi/rexafs/actions/runs/35910265798)
and [website](https://github.com/Ameyanagi/rexafs/actions/runs/35910265721)
checks also passed. An earlier preparation run was cancelled after the final
documentation update; it is not counted as successful qualification.

The `dev` merge, `d1973d2cb05f3086543980578e6efd2a9b5da2ab`, has exactly the
tested head's tree. [PR #129](https://github.com/Ameyanagi/rexafs/pull/129)
promoted this prepared source from `dev` to `main` after all 56 active checks
passed, with three expected skips. The
[promotion release matrix](https://github.com/Ameyanagi/rexafs/actions/runs/35914730652)
passed all 30 jobs, including both Windows installer and updater checks.
The merge commit, `4925e34b735ec8908e5084e797110aba7c235fe7`, has exactly the
qualified `dev` tree. Main was then fast-forwarded back into `dev`.

## Immutable tag and release build

The annotated `v0.2.14` tag identifies that main merge. The
[manual exact-tag build](https://github.com/Ameyanagi/rexafs/actions/runs/35920006031)
was dispatched on this tag; its event, tag and source commit were verified.
All 30 jobs passed on their first attempt, including five desktop targets,
both Windows installer and updater checks, Linux GUI smoke checks, the Rust
and npm packages, the Python source archive and all 15 Python runtime combinations.
Pull-request and Nightly artifacts are not substituted for the release outputs.

## Signed desktop qualification

[Signing run 35924216636](https://github.com/Ameyanagi/rexafs/actions/runs/35924216636)
passed Developer ID signing, notarization and installer checks. A fresh local DMG
installation passed strict signature, Gatekeeper, core, signed updater-helper and
FEFF checks. The [signed workflow record](signed-workflow.md) retains fresh Cu
Series fits, covariance-derived expression bars, independent display limits,
save/reopen, three-signal averages, pause/resume and Assistant control inspection.

The Computer Use connector rejected authentication. The final GUI review used
existing native macOS Accessibility permissions and unedited native window PNGs;
the record and eight-image manifest distinguish that fallback from the earlier
connector captures. All 102 compared Cu fit-summary values agreed within the
recorded tolerance; they were not bit-identical. Three 401-point synthetic signal
averages matched their arithmetic means to at most 5.56 × 10⁻¹⁷ absolute error.

## Publication and public downloads

[crates.io](https://github.com/Ameyanagi/rexafs/actions/runs/35925786902),
[PyPI](https://github.com/Ameyanagi/rexafs/actions/runs/35925789769) and
[npm](https://github.com/Ameyanagi/rexafs/actions/runs/35925792542) publication
succeeded using the qualified exact-tag artifacts. The
[draft workflow](https://github.com/Ameyanagi/rexafs/actions/runs/35924219554)
staged desktop files. Signed Mac outputs replaced only draft assets, and all
22 draft asset sizes and GitHub SHA-256 digests matched before publication.
The immutable source tag and historical public assets were not replaced.

All 22 public desktop files and six registry files were then downloaded and
compared with the original build/signing outputs. Registry metadata also matched
the crate checksum, PyPI digests and npm integrity value. The
[public artifact table](published-artifacts.md) retains URLs, byte counts and hashes.
[Fresh Python and npm consumers](published-consumers.md) ran the unmodified
documented Cu example and compared complete finite Fourier arrays.

Stable website metadata was advanced only after those public-download checks.
The publication change regenerates Stable references from the verified release,
retains historical screenshots and adds the signed 0.2.14 workflow captures.

Local publication checks passed: Astro checked 38 files with zero errors,
warnings or hints; all eight generator tests, 23 content/engine tests and 25
browser/accessibility tests passed. The production build completed, Stable Rust
reference used the verified published crate, and Next restored a hash-verified
source cache. New qualification links and all eight original PNG hashes were
checked. Browser checks used a separate local preview port, leaving existing
development servers untouched. GitHub's Website workflow independently verifies
the committed change before deployment.

Physical beamline acquisition, network shares and end-to-end Google Drive
downloads remain outside the retained qualification. Automated platform builds
do not establish these operating conditions.
