# rexafs 0.2.16 qualification

Release preparation began on 30 October 2026 in Japan. This record separates
source checks from final artifact qualification and publication. Pending gates
below are requirements, not completed validation claims.

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

## Pending release gates

1. Verify preparation checks and promote `dev` to `main` through a pull request
   with the required checks and a merge commit.
2. Create the immutable `v0.2.16` tag and manually dispatch its complete release
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
