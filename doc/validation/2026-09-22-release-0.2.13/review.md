# rexafs 0.2.13 qualification

Prepared on 22 September 2026 in Japan. This is a release preparation record;
it does not establish publication. Version 0.2.12 remains the published stable
release until the checks below complete.

## Scope and retained evidence

[PR #122](https://github.com/Ameyanagi/rexafs/pull/122) contains the deterministic
copper teaching project, PCA and MCR-ALS defaults, storage cleanup, tutorial and
ruviz figures. The [tutorial validation](../2026-09-22-cu-reduction/README.md)
retains numerical results, provenance, input hashes and local checks. Its
measurements describe the original 0.2.12 source preview and are not relabeled
as measurements of a signed 0.2.13 application.

The coordinated release also includes dependency and support-link changes
already merged after v0.2.12. The release notes explain the user-visible scope.
Project format remains 1; all historical compatibility fixtures remain intact.

## Local preparation checks

The optimized 0.2.13 desktop writer generated new linked and embedded projects
with both FEFF features enabled. The project persistence suite passed 30 tests
with two intentional ignores, including load/save/reopen of the full retained
fixture collection. The manifest checker verified all 48 retained samples and
the new release headers without changing older files.

Coordinated version validation and all seven version-check regression tests
passed. Repository commit checks passed, including fixture integrity and release
tooling regressions. Python and TypeScript references and the citation index
were regenerated without changing the stable 0.2.12 metadata or generated pages.
Local links in the new release documents were checked.

## Release gates

- Merge PR #122 only after its selected checks succeed.
- Prepare coordinated 0.2.13 manifests, release notes, and new linked/embedded
  compatibility fixtures written by the release's own project writer.
- Pass the release-preparation checks, then promote `dev` to `main` through a
  reviewed pull request with a merge commit and the required aggregate checks.
- Create the immutable v0.2.13 tag on the qualified main commit and manually
  dispatch the complete release build for that exact tag.
- Sign and notarize the qualified Apple Silicon artifact. Verify the signed app,
  installed DMG, updater helper and the new teaching workflow on macOS.
- Publish the qualified registry files and reviewed desktop release, then verify
  public download hashes against the build and signing manifests.
- Update the website's stable release metadata and references only after the
  corresponding downloads and registry packages have been verified.

Source commits, build/signing run identifiers, installation results, publication
checksums and website deployment evidence will be recorded here as they complete.
