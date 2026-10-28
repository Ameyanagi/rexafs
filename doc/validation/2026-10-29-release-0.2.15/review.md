# rexafs 0.2.15 qualification

Release preparation began on 29 October 2026 in Japan. This record separates
source checks from final artifact qualification and publication. Pending gates
below are requirements, not completed validation claims.

## Source and scope

Version 0.2.15 collects the work merged into `dev` after 0.2.14, including:

- the multi-panel publication composer and its follow-ups: named recipes,
  multi-sample stacking, insets, matplotlib export, report integration and
  above-curve labels ([#190](https://github.com/Ameyanagi/rexafs/pull/190),
  [#192](https://github.com/Ameyanagi/rexafs/pull/192)–[#197](https://github.com/Ameyanagi/rexafs/pull/197),
  [#208](https://github.com/Ameyanagi/rexafs/pull/208));
- FEFF fit windows that follow Process settings
  ([#205](https://github.com/Ameyanagi/rexafs/pull/205));
- the CVE-2026-111497 and RustSec dependency updates with advisory checks in CI
  ([#204](https://github.com/Ameyanagi/rexafs/pull/204));
- the apex-solver 1.5 update ([#202](https://github.com/Ameyanagi/rexafs/pull/202)),
  which stops passing variable bounds to apex-solver. 1.5 rejects bounds it
  cannot enforce; 1.4 stored and ignored them. rexafs clamps bounded variables
  in the residual and when applying the solution, so fit results are unchanged.
  All 248 `rexafs` library tests passed with 1.5, including the three fitting
  tests that failed before the change.

Each pull request passed its selected checks before merging.

## Local preparation

On Apple Silicon macOS, the 0.2.15 writer generated new linked and embedded
fixtures through the ignored `write_release_compatibility_fixtures` test. Compared
with the 0.2.14 pair, they add only the explicit fit-window, taper and
follow-Process range fields. The project suite passed 48 tests with two
intentional ignores, and the fixture checker verified all 52 samples and their
recorded hashes. Earlier fixture files were not regenerated.

The coordinated version check reports 0.2.15 for Cargo, Python and npm.
Formatting, `cargo clippy -D warnings` for `rexafs` with and without the
`trust-region` feature, and the release version, archive, download and
maintenance tooling tests passed.

## Pending release gates

1. Verify preparation checks and promote `dev` to `main` through a pull request
   with the required checks and a merge commit.
2. Create the immutable `v0.2.15` tag and manually dispatch its complete release
   build. Use only that run's qualified outputs for publication.
3. Sign and notarize the Mac artifact; qualify its installed DMG and updater
   helper.
4. Publish qualified registry artifacts and the reviewed desktop release, then
   download public files and verify hashes against the build/signing manifests.
5. Advance the website's stable metadata and generated references only after
   those public artifacts are verified; retain build and publication evidence.

Physical beamline acquisition, network shares and end-to-end Google Drive
downloads remain outside the retained qualification. Automated platform builds
do not establish these operating conditions.
