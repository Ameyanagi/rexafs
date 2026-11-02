# rexafs 0.2.16 qualification

Release preparation began on 30 October 2026 in Japan. This record separates
source checks from final artifact qualification and publication. 0.2.16 was
published on 31 October 2026 in Japan (30 October UTC).

## Source and scope

Version 0.2.16 collects the work merged into `dev` after 0.2.15:

- Process plot focus, click-to-focus, dμ/dE on measured μ(E), the spline k
  range on normalized μ(E), the back-transform R window, and a XANES view that
  stays still while E₀ is dragged
  ([#223](https://github.com/Ameyanagi/rexafs/pull/223)). Before/after desktop
  screenshots are attached to the pull request;
- Rust dependency updates: pyo3 0.29.3, rusqlite 0.41, encoding_rs 0.8.42 and
  AccessKit for macOS and Windows
  ([#216](https://github.com/Ameyanagi/rexafs/pull/216)–[#220](https://github.com/Ameyanagi/rexafs/pull/220));
- GitHub Actions and website tooling updates
  ([#211](https://github.com/Ameyanagi/rexafs/pull/211)–[#213](https://github.com/Ameyanagi/rexafs/pull/213),
  [#215](https://github.com/Ameyanagi/rexafs/pull/215),
  [#221](https://github.com/Ameyanagi/rexafs/pull/221)).

Each pull request passed its selected checks before merging. The TypeScript 7
update ([#214](https://github.com/Ameyanagi/rexafs/pull/214)) is not included:
the website's TypeScript reference generator uses the compiler API that
TypeScript 7 no longer provides.

## Local preparation

On Apple Silicon macOS, the 0.2.16 writer generated new linked and embedded
fixtures through the ignored `write_release_compatibility_fixtures` test. Compared
with the 0.2.15 pair, only the recorded software version, save time and source
file modification times differ. The fixture checker verified all 54 samples and
their recorded hashes. Earlier fixture files were not regenerated.

## Release gates

1. **Promotion.** The preparation pull request
   [#224](https://github.com/Ameyanagi/rexafs/pull/224) passed 41 checks, with 2
   skipped. [#225](https://github.com/Ameyanagi/rexafs/pull/225) then promoted
   `dev` to `main` with a merge commit after all its checks passed (61 passed,
   4 skipped). The merge commit `a4b3436789cb23cf7b0cb1ad50ced8f12b4139d3` has
   the same tree as the tested `dev`.
2. **Tag build.** The annotated `v0.2.16` tag (tag object
   `3953b082106bd36c1c05cafedd9e5b1bd6a3b9d5`) names that commit. All 30 jobs of
   the manually dispatched
   [exact-tag build](https://github.com/Ameyanagi/rexafs/actions/runs/41070595063)
   passed. Publication used only that run's outputs.
3. **Mac signing.** The
   [signing run](https://github.com/Ameyanagi/rexafs/actions/runs/41078442580)
   applied Developer ID signing (team `XXN44W8X56`) and an accepted notarization
   submission (`da07d53c-a916-4cdb-83fd-dc74430a24d4`). `sign-macos-release.py`
   checked its unsigned input against the build manifest
   (`8033cc28…619189`). It produced the signed ZIP `6c4512f2…e03707` and DMG
   `b8d6ab65…e23d45`. On Apple Silicon macOS, an app copied from the signed
   DMG passed `codesign --verify --deep --strict`, Gatekeeper
   (`source=Notarized Developer ID`), stapler validation of the app and DMG,
   `--version` (0.2.16), `--self-check` (E₀ = 8977.493 eV, 501 k points,
   326 R points) and `--self-check-updater`. The signed app received these
   command-line checks only; no interactive signed-GUI review or new screenshot
   capture was run. The new Process behavior was reviewed interactively on a
   development build, with before/after screenshots in
   [#223](https://github.com/Ameyanagi/rexafs/pull/223). The FEFF calculation
   path was not exercised in the installed app.
4. **Publication.** [crates.io](https://github.com/Ameyanagi/rexafs/actions/runs/41079203378),
   [PyPI](https://github.com/Ameyanagi/rexafs/actions/runs/41079240963) and
   [npm](https://github.com/Ameyanagi/rexafs/actions/runs/41079277568) used trusted
   publishing; npm recorded a signed provenance statement. The
   [GitHub draft](https://github.com/Ameyanagi/rexafs/actions/runs/41079165464)
   received the signed Mac ZIP, DMG, `dmg.json` and SHA-256 sidecars in place of
   the unsigned ZIP, with a regenerated `SHA256SUMS`, before it was made public
   at 2026-10-30T15:24:25Z. All
   [22 desktop files and six registry files](published-artifacts.md) were
   downloaded from public URLs and matched the build or signing outputs. Fresh
   [Python and npm consumers](published-consumers.md) ran the documented Cu
   example with the published packages.
5. **Website.** The stable website metadata, versioned guides and generated
   references advanced to 0.2.16 only after those checks.

Physical beamline acquisition, network shares and end-to-end Google Drive
downloads remain outside the retained qualification. Automated platform builds
do not establish these operating conditions.
