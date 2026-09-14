# rexafs 0.2.6 release qualification

Status: published on 14 September 2026. Rust, Python and npm packages are public;
the desktop release became public at **11:08:18 UTC** with 27 verified assets.
The preparation history below is followed by the publication evidence.

The shared reader was squash-merged in
[PR #64](https://github.com/Ameyanagi/rexafs/pull/64) at
`6d2f53ab96f98d97c06b3b4fdd0b91c62bb71cf8`. All checks on its final revision
passed, including Windows fixture checks, benchmarks and six desktop targets.
Those PR artifacts carry 0.2.5 metadata and must not be published as 0.2.6.

## Source preparation history

Cargo's workspace and local lockfile entries, the workspace rexafs dependency,
and npm's manifest/lockfile advance together to 0.2.6. Python derives its version
from Cargo. Reader API help identifies 0.2.6 as the introduction version. Website
Stable metadata stays at 0.2.5 until actual publication is verified.

Generate and reopen linked and embedded projects through the 0.2.6 writer;
preserve all historical project and beamline fixture bytes. Review the release
notes, generated Next API help, package exclusions and coordinated-version check.

## Release procedure

1. Complete preparation checks and merge the reviewed release commit.
2. Create an immutable `v0.2.6` tag, then manually dispatch the full release
   build on that exact tag. Only its successful, checksummed artifacts may be
   promoted; local builds and PR artifacts remain verification outputs.
3. Qualify the four ABI3 wheels on all 20 interpreter/platform combinations,
   the source archives, npm package and all six native desktop targets.
4. Sign and notarize both qualified Mac archives, verify the resulting ZIPs and
   DMGs, and replace unsigned Mac assets in the draft desktop release.
5. Publish GitHub-built packages to crates.io, PyPI and npm. Verify public bytes
   and fresh consumer installations, then publish the complete desktop release.
6. Promote website downloads and Stable references to the verified tag and crate
   checksum. Check the deployed site and preserve versioned screenshot evidence.

The [release runbook](../../releasing.md) defines the commands and platform
limits. Numerical defaults and project format remain unchanged; qualification
does not establish experimental accuracy or compatibility with every GPU.

## Local preparation evidence

On macOS ARM64, the 0.2.6 maintainer writer saved and reopened both new project
fixtures. The focused project suite passed 26 tests with the explicit fixture
writer ignored; all 34 retained project/source samples passed their checksum and
header checks. Historical fixture hashes were preserved.

The coordinated `v0.2.6` version check and strict Rustdoc passed. Python and
TypeScript Next references were regenerated with 130 and 182 documented members.
At preparation time, Stable still used the unchanged 0.2.5 declarations; release
CI and publication were separate gates, as listed above.

## Exact-tag qualification and package publication

[PR #65](https://github.com/Ameyanagi/rexafs/pull/65) prepared the coordinated
release. Immutable tag `v0.2.6` resolves to
`8ba48385ca00273bd8bd2bf7e2c46d291e6e5ad1`. All **37 jobs** in the manually
dispatched [build 34830810935](https://github.com/Ameyanagi/rexafs/actions/runs/34830810935)
succeeded, including the 20 Python interpreter/platform combinations, source
distribution rebuild, npm checks and six native desktop targets. The original
manifest covers 29 build artifacts and has SHA-256
`204bb912df97f84b1458b8dd73256a3ab7618288067c9eb6227a4e4f8f47f261`.

Trusted publication used that same tag and build for every registry:

| Channel | Successful publication run | Public package SHA-256 |
| --- | --- | --- |
| crates.io | [34835390430](https://github.com/Ameyanagi/rexafs/actions/runs/34835390430) | `e1db444140a8ec96c09ebefabde85596a2265f2e371ee6fbc9f71593fd94e0a1` |
| PyPI | [34835392098](https://github.com/Ameyanagi/rexafs/actions/runs/34835392098) | Source: `5a7d78e8e7c64ff31852d6cc2fd129a64724c1d18c4caad92dd7f76100f70685` |
| npm | [34835393888](https://github.com/Ameyanagi/rexafs/actions/runs/34835393888) | `86c6c19a728dc55955b33c03a26bc47fad48eaf13624b0f40d1898155997f3b7` |

At **11:02:41 UTC**, fresh downloads of all seven registry files matched the
tagged build: the crate, four ABI3 wheels, Python source distribution and npm
tarball. Registry checksums and npm's SHA-512 integrity also matched. All three
registries identified 0.2.6 as their latest stable version. The retained beamline
and session test bundles were absent from the published source packages.

The npm publisher initially reported that its accepted upload was processing;
metadata and the tarball became available afterward. No duplicate upload or
version replacement was used. Local receipt:
`/tmp/rexafs-release-0.2.6/registry-verification.json`.

Fresh consumers on Apple Silicon macOS passed:

- Rust: installed the public crate and checked named/mixed column selection,
  keV-to-eV conversion, transmission and fluorescence arithmetic, and unknown
  column errors.
- Python 3.12 with NumPy 2.5.3: **24 tests and 30 subtests**, plus installed-wheel
  completion, signatures, property/method hover help and keyword choices.
- npm: **25 tests**, including Node/browser entry points, measurement/session
  readers, processing, installed TypeScript 7 consumers and editor checks.

The local Python test environment initially lacked pytest; installing that test
dependency allowed the complete suite to run. This was test-harness setup, not
a package runtime failure. Cross-platform qualification is supplied by the
tagged CI matrix, not by these local consumer checks.

## Signed desktop artifacts and graphical review

[Signing run 34835395867](https://github.com/Ameyanagi/rexafs/actions/runs/34835395867)
signed and notarized both Mac archives and installers with Apple team
`XXN44W8X56`, preserving the original build identity. Local checks required
Developer ID signatures, hardened runtime, timestamps, stapling and Gatekeeper
acceptance. Both DMGs were mounted read-only and their installed copies passed
architecture/build checks, processing and both packaged Cu engine self-checks.

| Installer | SHA-256 |
| --- | --- |
| Mac ARM64 DMG | `40e845e7f01f4b74abfe819dc8ec707db6b01760ad49952f801cb44826914620` |
| Mac Intel DMG | `78fcf739d2c579b9f9d2d7e8992ff16bab9cd107803b0682d9ced676ccdf4c45` |

Computer use exercised the actual signed binaries, with executable hashes and
isolated `REXAFS_SETTINGS` paths checked against the running processes:

- ARM64: previewed the retained QAS Mo foil reference, inspected source headers,
  imported transmission and reference together, undid/redid the complete import,
  processed the reference through the Fourier transform, and saved/reopened an
  embedded project. Both groups and the plotted result survived reopening.
- Intel under Rosetta: opened that project and reprocessed/rendered the reference
  in k and R. Fresh file import and native Intel GPU hardware were not exercised
  in this graphical check; both packaged engines passed separate binary checks.

The project contains two imported spectra and two copies of the original source
bytes in their operation provenance; both decoded payloads match the retained
measurement exactly. During save-dialog automation, macOS treated a full path
entered in the filename field as a colon-separated filename. The newly generated
test file was moved into the isolated verification directory and reopened from
there. No original measurement or pre-existing project was changed.

The [0.2.6 import screenshot](../../../website/public/screenshots/0.2.6/import-preview.png)
is an unmodified full-window capture; [its record](import-capture.md) identifies
the executable, source measurement, attribution and hashes. At 1192 × 768,
expanding source details reduced the visible signal-list area; collapsing it
restored all choices. Keeping every signal row visible with expanded headers is
a remaining layout improvement. Import counts and mappings remained correct.

The release draft's inventory, uploaded digests and all 27 freshly downloaded
files matched the qualified desktop files before publication. The desktop-only
`SHA256SUMS` has SHA-256
`ddd29956000859d236075019774d81bfb171d9e8f5a97a2e0b83468b172833e9`.
The original complete build manifest remains separate. Local records and
screenshots are under `/tmp/rexafs-release-0.2.6/`; only the versioned import
capture and this concise qualification record are committed.

Windows installers are unsigned previews. Windows ARM64 runs rexafs and ReFEFF
natively with an emulated x64 FEFF10 helper. Linux qualification uses Ubuntu
24.04, X11 and software Vulkan; native Wayland, physical GPU combinations and
interactive Windows checks remain outside this evidence.

## Public downloads and Stable documentation

After publication, unauthenticated HTTP downloads of all **27 public desktop
assets** matched the qualified files and the GitHub upload digests. This completed
at **2026-09-14T11:09:07.316365+00:00**; the local receipt is
`/tmp/rexafs-release-0.2.6/public-desktop-verification.json`.

Stable Python and TypeScript references were regenerated from `v0.2.6` with
130 and 182 documented members respectively. Rust references were built from the
published crate after verifying its SHA-256. The import guide includes the new
signed-app screenshot, preserving earlier capture versions and attribution.
Local documentation validation passed:

- Full website and WebAssembly build, including Stable Rustdoc from the
  checksum-verified published crate and strict Next Rustdoc.
- Eight reference-generator tests and 22 website tests covering links, anchors,
  images, citations and API documentation coverage.
- Astro checks across 37 files, with no errors, warnings or hints.
- All 25 browser tests against this worktree's fresh production build, using an
  isolated preview server. Desktop and mobile home-page captures and the Python
  API capture were also visually reviewed.

Deployment is verified after the documentation promotion reaches `main`;
the local checks above do not establish that the public website has updated.
