# rexafs 0.2.14 qualification

Release preparation began on 24 September 2026 in Japan. This record separates
source checks from final artifact qualification and publication. Pending gates
below are requirements, not completed validation claims.

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

## Release gates

1. Prepare coordinated versions and new linked/embedded fixtures using the
   0.2.14 writer; retain all previous fixtures and hashes.
2. Verify preparation checks and promote `dev` to `main` through a pull request
   with the required checks and a merge commit.
3. Create the immutable `v0.2.14` tag and manually dispatch its complete release
   build. Use only that run's qualified outputs for publication.
4. Sign and notarize the Mac artifact; qualify its installed DMG and updater
   helper, and review the new Live/Series workflow in the signed application.
5. Publish qualified registry artifacts and the reviewed desktop release, then
   download public files and verify hashes against the build/signing manifests.
6. Advance the website's stable metadata and generated references only after
   those public artifacts are verified; retain build and publication evidence.

Physical beamline acquisition, network shares and end-to-end Google Drive
downloads remain outside the retained qualification. Automated platform builds
do not establish these operating conditions.
