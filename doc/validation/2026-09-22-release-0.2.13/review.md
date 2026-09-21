# rexafs 0.2.13 qualification

Published on 22 September 2026 in Japan, at 20:28:26 UTC on 21 September.
The [0.2.13 release](https://github.com/Ameyanagi/rexafs/releases/tag/v0.2.13)
contains the qualified artifacts described below. The preparation gates are
retained to distinguish source review, binary qualification and publication.

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

## Source qualification

[PR #122](https://github.com/Ameyanagi/rexafs/pull/122) passed all 41 selected
checks, with only the expected website deployment skip. Its
[release matrix](https://github.com/Ameyanagi/rexafs/actions/runs/35632913943)
passed all five desktop targets, both Windows installers and all 15 Python
runtime combinations. Rust, benchmark, Larch comparison and website gates also
passed. No failed job was bypassed or retried.

The merge into `dev`, `17a6af3fa2ca8d213903bf5bba2255550526a670`, has exactly the
same tree as the tested head, `2e8bf761e83af207a1e4207fc9306dac3606153e`.

[PR #123](https://github.com/Ameyanagi/rexafs/pull/123) also passed all 41 selected
checks on its first attempt, with the expected website deployment skip. Its
[release matrix](https://github.com/Ameyanagi/rexafs/actions/runs/35633934713)
qualified the coordinated 0.2.13 versions and the new compatibility fixtures.
The resulting `dev` merge, `cad85ea9a4dd0102ff5d053cb8003c1fe27cf9c0`, has the same
tree as tested head `2a2bb0eeed99e10e85fe7a9f0167c99df2229cbd`.

[PR #124](https://github.com/Ameyanagi/rexafs/pull/124) promoted this prepared
source from `dev` to `main` with a merge commit after all 56 selected checks
passed. Three expected skips were the Nightly dispatcher and the two website
deployments. All 30 jobs in the
[promotion release matrix](https://github.com/Ameyanagi/rexafs/actions/runs/35639187238)
passed on their first attempt. The merged main tree exactly matches the
qualified `dev` tree.

## Immutable tag and exact-tag build

The annotated `v0.2.13` tag identifies the main merge,
`9b7e74c47edcff8ef64cdfdf6ecabb40323b4a11`. Main was then merged back into `dev`
by fast-forward. The
[manual exact-tag build](https://github.com/Ameyanagi/rexafs/actions/runs/35644510582)
was dispatched on `v0.2.13`; its event, branch and source commit were verified.
All 30 jobs passed on their first attempt, including both Windows installer and
updater checks, Linux GUI smoke checks, all five desktop targets and all 15
Python runtime combinations. PR and Nightly artifacts are not substituted for
these release outputs.

## Tagged-source tutorial reproduction

The documented extraction and calculation commands were run from the immutable
0.2.13 source in an optimized local build. Extraction verified all 53 embedded
spectra and both metadata files. The fresh PCA, blind MCR and known-reference
LCF calculations used the desktop defaults and the same 50 mixtures on the
517-point grid. All **828 numerical values** in the resulting tutorial record
matched the retained figure data exactly. MCR converged in 110 iterations; the
first three uncentered PCA components retained 99.999856843% of squared signal.

The [reproduction record](tutorial-reproduction.json) identifies source, input
record hashes, comparison scope and tolerance. Software version and release-status
labels were excluded from the numerical comparison. The original figure data
remain unchanged and retain their source-preview provenance. This local check is
separate from qualification of the signed desktop artifact.

## Signed installer and native workflow

[Signing run 35649662534](https://github.com/Ameyanagi/rexafs/actions/runs/35649662534)
signed, notarized and stapled the original Apple Silicon artifact with Developer
ID team `XXN44W8X56`. It passed on its first attempt. The downloaded signed ZIP
and DMG checksums matched their sidecars; the DMG's executable hash matched the
installed app. Installer verification, Gatekeeper, strict signatures and stapling
passed before and after installation into a fresh directory.

On an Apple M4 running macOS 26.5.1, the installed app passed `--self-check`,
`--self-check-feff` and `--self-check-updater`. Its source, feature flags and
version matched the tag and signing metadata. Existing user applications were
not replaced.

Computer Use opened the bundled example with 53 groups and 50 marked mixtures,
then ran PCA and MCR through ordinary default controls. PCA started in Linear
with no mean subtraction and Auto −20 to +30 eV. MCR independently used the full
517-point common range, three components, coefficient closure, signed spectra,
500 maximum iterations and seed zero. It converged in 110 iterations.

The saved project's PCA model and MCR result matched all **92,709 numerical
values** in the tagged-source calculation exactly; label strings were excluded.
The [signed-analysis record](signed-analysis.json) retains the saved project hash,
comparison tolerance and settings. Series selected all 50 marked frames and
displayed frame 50 minus frame 1. Storage completed its background scan and
listed 24 managed candidates totaling 1.26 GiB; no files were deleted.

Six full, unedited 1192 × 768 windows and their
[capture manifest](../../../website/public/screenshots/0.2.13/capture.json)
identify the exact installer, executable, input and image hashes. The approved
ruviz figures and their original numerical record remain unchanged.

## Publication

All four workflows passed using the same exact-tag build:

- [crates.io](https://github.com/Ameyanagi/rexafs/actions/runs/35650559826).
- [PyPI](https://github.com/Ameyanagi/rexafs/actions/runs/35650562748).
- [npm](https://github.com/Ameyanagi/rexafs/actions/runs/35650565914).
- [GitHub draft](https://github.com/Ameyanagi/rexafs/actions/runs/35650569384).

The draft's unsigned Mac archive was replaced with the qualified signed archive,
and the signed DMG, installer evidence and new checksum manifest were added.
All 22 final draft asset sizes and SHA-256 digests were checked before publication.
No published asset or immutable tag was replaced.

The documented copper examples passed with a freshly installed PyPI wheel and
npm package, both reporting version 0.2.13 and E₀ = 8977.493 eV. Both produced
326 finite Fourier-magnitude points. npm initially returned 404 while processing
the accepted publication; installation succeeded after the version became public.
The [consumer record](published-consumers.json) identifies both exact examples;
their Fourier magnitudes agreed within 4.94 × 10⁻¹³ absolute difference.

All **six registry files and 22 desktop files** were then downloaded through their
public URLs. Each size and SHA-256 digest matched the qualified build or signed
replacement. The [public artifact record](published-artifacts.json) retains every
URL, byte count and checksum. Stable website metadata now points to this verified
tag, source commit, crate checksum and platform inventory. Intel Mac desktop
archives and Python wheels are absent, as required by the retained platform policy.

## Website preparation and local review

Stable metadata was advanced only after the public verification above. Python
and TypeScript references and citations were regenerated; Stable Rust HTML was
built from the published crate after verifying its recorded SHA-256. Next Rust
HTML was restored from a source-identity and content-hash-verified cache.

Astro reported no errors, warnings or hints. All eight generator tests, 23 content
tests and 25 browser tests passed. The initial browser server could not use port
4321 because another workspace owned it. The unchanged suite passed with the
same timeouts and zero retries against an isolated preview on port 4340.

The stable copper tutorial was separately reviewed at desktop and 390-pixel
mobile widths: all images and figure downloads loaded, equations rendered,
and no console errors, accessibility violations or horizontal overflow were
found. The six original signed-app screenshots are linked at full resolution.
The old Next tutorial URL remains as a link to the stable guide. Historical
images, fixtures, recipe data and approved ruviz figure bytes are retained.
