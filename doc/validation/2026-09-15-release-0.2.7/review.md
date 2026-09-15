# rexafs 0.2.7 release qualification

Status: 0.2.7 packages and desktop downloads were published and verified on
15 September 2026. Updated website publication is tracked below.

The release combines the Codex startup correction in
[PR #70](https://github.com/Ameyanagi/rexafs/pull/70), the Assistant composer and
panel layout in [PR #71](https://github.com/Ameyanagi/rexafs/pull/71), and the
measurement reader corrections in [PR #72](https://github.com/Ameyanagi/rexafs/pull/72).
All three features passed their selected checks before merging into `dev`: 47
checks for #70, 47 for #71, and 48 for #72. The website deploy job was intentionally
skipped on each PR. Their benchmark logs reported no threshold regressions.

The merged `dev` tree at `8378ebac724e1ffb7d3e925d7ee97b52f79e85e6` exactly
matches the combined source used for the local tests below. The release promotion
uses a `dev` → `main` pull request with a merge commit.

## Qualification procedure

1. Coordinate Cargo and npm versions; Python derives its version from Cargo.
   Save and reopen new linked and embedded compatibility fixtures with the
   0.2.7 writer, preserving historical files and their checksums.
2. Review the release changes and promote `dev` to `main`. Create the immutable
   `v0.2.7` tag and manually dispatch the complete release build for that tag.
3. Require all package, interpreter and desktop jobs to pass. Publish only
   checksummed artifacts from that exact GitHub build.
4. Sign and notarize both Mac archives, qualify the resulting ZIPs and DMGs,
   and inspect the actual Assistant interface with computer use in an isolated
   test session. Capture versioned screenshots for the updated guide.
5. Publish crates.io, PyPI and npm packages and verify public bytes and fresh
   consumers. Complete and verify the desktop draft before making it public.
6. Update the public download metadata and Stable API references only after
   publication, verify the deployed guide and retain screenshot provenance.

The [release runbook](../../releasing.md) defines the commands and platform
limits. Passing numerical tests does not establish experimental accuracy, and
archive self-checks do not replace graphical review.

## Local preparation evidence

On Apple Silicon macOS, the combined source from PRs #70–72 passed
`cargo test --locked -p rexafs -p rexafs-gui`: **821 passed, 0 failed, 10 ignored**.
This includes 478 GUI tests and the core suites with the GUI's enabled features.
The combined check used 0.2.6 metadata before the coordinated version change.

After the version change, the 0.2.7 maintainer writer saved and reopened new
linked and embedded project fixtures. The project suite passed **26 tests**,
with its explicit fixture writer ignored. All **36 retained samples** passed
checksum and header verification; historical file bytes and hashes are unchanged.
The coordinated `v0.2.7` version check, formatting, and Rustdoc with
missing-documentation and broken-link errors enabled passed. Python and
TypeScript references regenerated with 130 and 182 documented members per channel;
Stable continues to describe published 0.2.6 until publication is verified.

These local checks do not qualify a release tag, public packages or signed
desktop downloads. Their results will be supplemented by the exact-tag build
and graphical review below.

## Local Assistant interface review

Computer use exercised an optimized 0.2.7 review app on Apple Silicon macOS,
with a separate settings file and the bundled public Cu example. It connected
to the installed Codex model catalog with `PATH=/usr/bin:/bin`. No assistant
message was sent. The following workflows passed visual and accessibility checks:

- Model, Reasoning and Access menus display their current choices and descriptions.
- Escape, Tab, Enter and Space navigate and activate menus once.
- Parameters closes and reopens with `⌘J` while a composer control has focus.
- A widened Assistant makes room for the most recently opened Groups or Parameters
  panel; normalization parameters remain visible and usable.
- Pop out retains the composer and model catalog, its menu stays within the
  separate window, and Dock returns the same session to the analysis.

The unedited local review capture and receipt are under
`/tmp/rexafs-release-0.2.7/gui-candidate/`. This is a source-build review; final
public screenshots must come from the qualified signed release download.

## Stable source promotion

[Release PR #73](https://github.com/Ameyanagi/rexafs/pull/73) passed 65 checks,
with three intentional skips and no unresolved review threads. Both benchmark
reports found no threshold regressions. The selected release-build matrix,
all 20 Python runtime combinations, Rust checks, website checks and the `dev`
nightly completed successfully before promotion.

The PR was merged with a merge commit at
`23e909b36c9595e3de47edafc10010bac88e4bb6` on 15 September 2026. Its tree is
identical to the reviewed release head
`fcb7e8e52b680587a9d7752fc4475a2e18c5fb87`. The immutable `v0.2.7` tag points
to that merge commit. [Manual tag build 34915881034](https://github.com/Ameyanagi/rexafs/actions/runs/34915881034)
is the publication source; the successful PR and nightly artifacts are not
used as stable release uploads.

The post-promotion [Rust workflow](https://github.com/Ameyanagi/rexafs/actions/runs/34915838420)
and [website workflow](https://github.com/Ameyanagi/rexafs/actions/runs/34915838435)
also passed on the tagged source commit. All four measured benchmarks were
within the existing regression thresholds. This records the measured workloads,
not a general performance guarantee.

## Documentation preview review

The revised Assistant guide was inspected through computer use in a local
browser preview. It covers the compact menus, keyboard navigation, workspace
panels, connection diagnostics and existing access controls. Full-size image
links preserve the original captures. The local review passed the website
build, Astro checks (zero errors, warnings or hints), eight generator tests,
22 content/runtime tests and 25 browser tests. These checks used review images;
final publication requires replacement with the signed-release captures below.

## Published artifacts and installed packages

All **37 jobs** in [manual tag build 34915881034](https://github.com/Ameyanagi/rexafs/actions/runs/34915881034)
passed. This includes six desktop targets, four ABI3 wheels exercised across
20 CPython/platform combinations, the source distribution, Rust and npm packages,
and the final release manifest. All 29 original build-artifact checksums passed
local verification.

The publication jobs for [crates.io](https://github.com/Ameyanagi/rexafs/actions/runs/34919027072),
[PyPI](https://github.com/Ameyanagi/rexafs/actions/runs/34919029089) and
[npm](https://github.com/Ameyanagi/rexafs/actions/runs/34919031316) passed with trusted
publishing. The seven publicly downloaded distribution files match the original
manifest and registry checksums. All three registries report 0.2.7 as latest.
The original measurement/session fixture bundles are absent from the crate,
Python source distribution and npm tarball.

Fresh public-package consumers passed:

- Rust: energy units, named and mixed column selectors, transmission and
  fluorescence arithmetic, ambiguous roles, distinct stored signals and malformed rows.
- Python: **24 tests and 30 subtests**, plus targeted checks of the three reader
  corrections and installed-wheel completion, hover and signature checks.
- TypeScript/JavaScript: **21 runtime and four editor tests**, plus targeted
  checks of the three reader corrections. Editor checks cover root, Node and
  browser exports, TypeScript 7 checking and TypeScript 6 language services.

Initial npm/PyPI metadata responses lagged behind successful publication and
then refreshed. Final checks used publicly available 0.2.7 files and metadata.
An initial local npm editor check lacked its expected downloaded tarball; it
passed after that public tarball was downloaded and checksum-verified.

## Signed desktops and graphical review

[Signing run 34919035253](https://github.com/Ameyanagi/rexafs/actions/runs/34919035253)
passed for Mac ARM64 and Intel. Both signed ZIPs and DMGs retain source commit
`23e909b36c9595e3de47edafc10010bac88e4bb6` and build run `34915881034`.
Local verification checked their original/signed checksums, Developer ID team
`XXN44W8X56`, hardened-runtime signatures, stapled notarization, Gatekeeper,
read-only mounted installation, copied app identity and both calculation engines.
ARM64 ran natively on macOS 26.5.1; Intel ran under Rosetta. Native Intel graphical
hardware was not tested locally. Windows and Linux remain desktop previews;
their installer and X11 qualification comes from the tagged CI matrix.

Computer use then reviewed the actual signed ARM64 app with isolated settings,
`PATH=/usr/bin:/bin`, and the bundled 618-point `cu_150k.xmu` example. It connected
to the installed Codex model catalog. The plotted import preview, normalization
parameters beside the Assistant, all three composer menus, Escape/Tab/Enter/Space,
arrow navigation, Parameters shortcuts with composer focus, panel resizing and
pop-out/dock passed. No assistant message was sent; Review remained selected and
Workspace commands remained off. User projects and settings were not used.

The [public desktop release](https://github.com/Ameyanagi/rexafs/releases/tag/v0.2.7)
contains **27 assets**. Their GitHub digests matched the qualified staged files
before publication; all 27 were downloaded without credentials after publication
and matched those same checksums. Public Mac archives are the signed outputs.

## Versioned Assistant screenshots

The three full, unedited JPEG window captures are **1192 × 768** pixels. They
were made through computer use on 15 September 2026 from the signed ARM64 app;
its executable SHA-256 is
`607e1a62d94090e7c62d7d858cd9c8f3fbaa69f338e6738173dd337193c275d6`.
They replace the temporary local review images, and their guide links open the
original image bytes. The available models depend on the connected installation.
The bundled public Cu measurement retains its [source and attribution](https://rexafs.com/licenses/).

| Capture | Bytes | SHA-256 |
| --- | ---: | --- |
| `assistant-layout.jpg` | 119811 | `11d89b391595969a8767d1d1e38adda73db96f50619f3455f99f7a6a2113206b` |
| `assistant-models.jpg` | 124002 | `8755dab6197411d2b72b92aa757780eef5fb3ad940f17159e53b7a2a159156e9` |
| `assistant-access.jpg` | 129128 | `bbc288be6e5e1f06bd441d0ec0d7928059058d230acd61a84d3fd6c6cdeee661` |

Local receipts and logs are retained under `/tmp/rexafs-release-0.2.7/` for this
maintainer session. That temporary directory is not distributed. Published
build/signing runs, asset checksums and the versioned screenshots provide the
public evidence.

The maintainer subsequently approved a correction to branch commit metadata.
The corrected `main` source tree at `95f002f41b6d3592e7e5acb16c90b52c5ae0721e`
is byte-identical to the release tag. All release tags, package bytes and signing
identities remain unchanged; historical build links above identify their actual
original commits. Documentation work starts from the corrected branch history.

## Final documentation checks

The final signed-release captures and 0.2.7 metadata passed the complete site
build, Astro checks (zero errors, warnings or hints), eight generator tests,
22 content/runtime tests and 25 browser tests. Stable Rustdoc was generated
from the published crate with its verified checksum; Python and TypeScript
references contain 130 and 182 documented members per channel. Computer use
reviewed the rendered Assistant guide and opened its model image at its original
1192 × 768 size. Website deployment is verified separately after promotion.
