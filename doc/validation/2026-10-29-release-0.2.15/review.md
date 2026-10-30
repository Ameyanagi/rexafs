# rexafs 0.2.15 qualification

Release preparation began on 29 October 2026 in Japan. This record separates
source checks from final artifact qualification and publication. 0.2.15 was
published on 29 October 2026 in Japan (28 October UTC).

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

## Release gates

1. **Promotion.** [#210](https://github.com/Ameyanagi/rexafs/pull/210) promoted
   `dev` to `main` with a merge commit after its required checks passed. The
   merge commit `1f74544213c3e1f4ac58ca61f2ae50a80b4f91fc` has the same tree as
   the tested `dev`.
2. **Tag build.** The annotated `v0.2.15` tag names that commit. All 30 jobs of
   the manually dispatched
   [exact-tag build](https://github.com/Ameyanagi/rexafs/actions/runs/40763005974)
   passed. Publication used only that run's outputs.
3. **Mac signing.** The
   [signing run](https://github.com/Ameyanagi/rexafs/actions/runs/40772009151)
   applied Developer ID signing and an accepted notarization submission
   (`125b1ec4-78d7-4386-996d-716ed11618ad`). Its unsigned input matched the build
   archive hash `2131c875…ee680`. On Apple Silicon macOS, an app copied from the
   signed DMG passed `codesign --verify --deep --strict`, Gatekeeper
   (`source=Notarized Developer ID`), stapler validation of the app and DMG,
   `--version` (0.2.15), `--self-check` (E₀ = 8977.493 eV, 501 k points,
   326 R points) and `--self-check-updater`. No interactive signed-GUI review
   or new screenshot capture was run for this release, and the FEFF calculation
   path was not exercised in the installed app.
4. **Publication.** [crates.io](https://github.com/Ameyanagi/rexafs/actions/runs/40772063506),
   [PyPI](https://github.com/Ameyanagi/rexafs/actions/runs/40772074623) and
   [npm](https://github.com/Ameyanagi/rexafs/actions/runs/40772086127) used trusted
   publishing; npm recorded a signed provenance statement. The
   [GitHub draft](https://github.com/Ameyanagi/rexafs/actions/runs/40772052695)
   received the signed Mac ZIP, DMG and their records in place of the unsigned
   ZIP, with a regenerated `SHA256SUMS`, before it was made public. All
   [22 desktop files and six registry files](published-artifacts.md) were
   downloaded from public URLs and matched the build or signing outputs. Fresh
   [Python and npm consumers](published-consumers.md) ran the documented Cu
   example with the published packages.
5. **Website.** The stable website metadata, versioned guides and generated
   references advanced to 0.2.15 only after those checks.

Physical beamline acquisition, network shares and end-to-end Google Drive
downloads remain outside the retained qualification. Automated platform builds
do not establish these operating conditions.
