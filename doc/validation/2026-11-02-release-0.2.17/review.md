# rexafs 0.2.17 qualification

Release preparation began on 2 November 2026 in Japan. This record separates
source checks from final artifact qualification and publication. Pending gates
below are requirements, not completed validation claims.

## Source and scope

Version 0.2.17 collects the work merged into `dev` after 0.2.16:

- Linked projects embed their FEFF calculation; Fit › Paths marks sources with
  missing FEFF files, blocks single and joint fits that use them, re-links a
  matching recalculation in place and removes sources with their unused
  variables ([#228](https://github.com/Ameyanagi/rexafs/pull/228), issue
  [#227](https://github.com/Ameyanagi/rexafs/issues/227)). Before/after desktop
  screenshots of a development build are attached to the pull request; the
  review fixes merged with it were checked by tests only;
- Published 0.2.16 downloads and release evidence in the documentation
  ([#226](https://github.com/Ameyanagi/rexafs/pull/226)).

Each pull request passed its selected checks before merging. The TypeScript 7
update ([#214](https://github.com/Ameyanagi/rexafs/pull/214)) is still not
included.

## Local preparation

On Apple Silicon macOS, the 0.2.17 writer generated new linked and embedded
fixtures through the ignored `write_release_compatibility_fixtures` test.
Compared with the 0.2.16 pair, the embedded fixture differs only in the
recorded software version and save time. The linked fixture additionally
records an `archive_path` for its FEFF path file and carries that file's
compressed payload, which is the intended format change. The fixture checker
verified all 56 samples and their recorded hashes. Earlier fixture files were
not regenerated.

## Pending release gates

1. Verify preparation checks and promote `dev` to `main` through a pull request
   with the required checks and a merge commit.
2. Create the immutable `v0.2.17` tag and manually dispatch its complete release
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
