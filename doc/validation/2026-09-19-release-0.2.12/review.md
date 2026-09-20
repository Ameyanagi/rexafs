# 0.2.12 release qualification

Status: **published on 20 September 2026**. The immutable
`v0.2.12` tag identifies `2c9ce950694dca25153ba248d5f4fe19c2a66735`.
Existing 0.2.11 assets remain unchanged.

## Scope

This patch repairs the signed macOS updater helper and keeps the installation
action available after download on all desktop platforms. The
[source qualification](../2026-09-19-updater/review.md) records the reproduced
failure, native tests, full-bundle fix and disposable-app computer-use check.
The source check used an ad hoc signed candidate receiving an official signed
Nightly app; it does not replace final Developer ID qualification.

The final screenshot refresh uses the signed 0.2.12 app and unchanged public
room-temperature Cu data. Eight full, unedited computer-use windows document
fresh RMC results, structural histories, Auto values and publication controls.
Earlier 0.2.11 and source-candidate captures retain their original provenance.
No private original measurement project is included.

## Preparation and required publication gates

The release scope was expanded on 20 September to include PRs #103–#106:
desktop workflow corrections, adaptive RMC resources, structural history,
population search and the catalogue-capacity fix. Dev commit
`6b1af79c568472af8d2ee3b7bad5033e970dbcd2` passed its Rust, Website and
[Nightly desktop](https://github.com/Ameyanagi/rexafs/actions/runs/35495161287)
workflows. These development checks do not replace the exact-tag release build.
The [RMC source review](../2026-09-20-rmc-followup/review.md) retains the measured
cache comparison and its numerical scope. A separate 256-site native check
prepared 2,246,144 paths and evaluated four k points; it did not assess fit quality.

On 20 September, the release scope was further updated to remove Intel Mac
desktop packages, Python wheels and nightly builds. The final release must qualify
five desktop targets and three Python wheels across 15 CPython runtime
combinations. Historical 0.2.11 downloads and evidence retain their original
platform inventories. Earlier checks with Intel Mac entries are historical and
do not replace qualification of the revised source.

Completed locally: coordinated version validation and its seven regression tests;
new linked/embedded projects written and reopened through the optimized 0.2.12
writer; all 46 retained fixture hashes verified; 29 project tests passed (the
explicit fixture writer remains ignored in the ordinary test run). These checks
do not substitute for the target-specific release matrix.

- Complete the reviewed source and version pull requests into dev, then promote
  dev to main after its selected checks pass.
- Build the immutable version tag using the release workflow; require every
  package, desktop, installer and runtime qualification to pass.
- Sign and notarize the Apple Silicon Mac package. Check the new helper self-test
  in the signed ZIP and installed DMG, then exercise that signed app locally.
- Verify registry package bytes and final desktop asset hashes before announcing
  publication or advancing the website's stable release metadata.

The old Mac helper cannot update itself. Users of affected old versions need
one initial installation of the corrected version; later updates use its repaired
helper. The final signed checks are recorded below; they do not repair the
helper inside an older installed app.

## Source review before tagging

The documentation and original source-candidate captures were merged through
[PR #107](https://github.com/Ameyanagi/rexafs/pull/107). The RMC API version
comments were corrected through [PR #109](https://github.com/Ameyanagi/rexafs/pull/109).
These changes do not relabel the earlier captures as signed release images.

[PR #115](https://github.com/Ameyanagi/rexafs/pull/115) removed Intel Mac desktop,
Python-wheel and Nightly builds. All **30 release-build jobs** in
[run 35510070738](https://github.com/Ameyanagi/rexafs/actions/runs/35510070738)
passed, including all five desktop targets and all 15 Python runtime combinations.
The PR passed 41 checks, with the expected website deployment skip. Its merge
`59f314e2c7133ef2d9e1569d67fe2d08ab8c1dfe` has the same tree as tested candidate
`90c3fe63558b1dfb5bca3be38f62a6f8686fda69`.
The platform validators reject Intel artifacts from 0.2.12 onward and retain
historical desktop and wheel inventories. Existing public artifacts were not
changed. These PR outputs were preliminary; publication additionally required the
exact-tag build, signing and public-byte verification recorded below.

The reduced Nightly workflow also passed for that dev merge and published
[the Apple Silicon-only prerelease](https://github.com/Ameyanagi/rexafs/releases/tag/nightly-20260920-35512345927)
on 20 September. Its six assets contain the signed ZIP, notarized DMG, checksums
and installation evidence for `aarch64-apple-darwin`; no Intel asset is present.
These Nightly artifacts are not used for the stable release.

## Promotion and immutable tag

[PR #108](https://github.com/Ameyanagi/rexafs/pull/108) promoted `dev` to `main`
with a merge commit on 20 September 2026. All 56 selected checks passed;
three expected skips were the Nightly dispatcher and two website deployments.
All 30 jobs in [the promotion release build](https://github.com/Ameyanagi/rexafs/actions/runs/35512348758)
passed, including Windows installation, reinstallation and removal.

The merged main tree exactly matches the qualified dev tree. The annotated
`v0.2.12` tag points to that merge, and `main` was merged back into `dev` by
fast-forward. [The manually dispatched exact-tag build](https://github.com/Ameyanagi/rexafs/actions/runs/35514851752)
is the source of release artifacts; PR and Nightly outputs are not substituted.

## Exact-tag build

[Build 35514851752](https://github.com/Ameyanagi/rexafs/actions/runs/35514851752)
passed all **30 jobs** on its first attempt. This includes the Rust crate and
optional numerical backends, npm/WebAssembly package, Python source distribution,
three ABI3 wheels and all 15 CPython 3.10–3.14 runtime combinations. All five
desktop archives passed their selected tests. Both Linux packages passed their
GUI checks; both Windows installers passed installation, reinstallation and
removal. The Mac installer preview passed before Developer ID signing.

The final complete-build manifest and aggregate release gate passed. The source
validator confirmed `v0.2.12`, the merged commit, the manual workflow event and
the `abi3-py310` wheel profile. The merged commit's
[Rust checks](https://github.com/Ameyanagi/rexafs/actions/runs/35514820954)
also passed. The separate signed-app and public-download checks follow.

## Signed Mac and fresh native checks

[Signing run 35516779212](https://github.com/Ameyanagi/rexafs/actions/runs/35516779212)
produced the Apple Silicon ZIP and DMG from the qualified build. The installed
DMG passed signature, stapling, Gatekeeper, ARM64 architecture, package and both
FEFF engine checks. The signed updater-helper self-test passed. Build metadata
identifies the exact tag, clean source, original build and signing runs; the
installed executable hash matches the installation evidence. The app was not
modified or ad hoc signed for these checks.

Computer use ran the installed app on an **Apple M4**, macOS **26.5.1**, with ten
automatic workers. The unchanged CC0 room-temperature Cu measurement contains
408 points. Its source and hash are in the
[capture manifest](../../../website/public/screenshots/0.2.12/capture.json).
This is different from the bundled 150 K Cu example used for package self-checks.

| Native check | Small structural-history case | Large catalogue regression |
|---|---:|---:|
| Periodic fcc Cu repeats / absorbing sites | 2 × 2 × 2 / 32 | 4 × 4 × 4 / 256 |
| Cluster / path radius | 6 / 4 Å | 8 / 6 Å |
| Maximum path legs | 4 | 4 |
| Completed attempts | 10 | 1 |
| Preparation | 1.0249 s | 93.1778 s |
| Total active time, including preparation | 2.1073 s | 97.3738 s |
| Initial / best objective | 6.464109 / 5.334100 | 6.477012 / 6.458291 |
| Captured catalogue capacity | 2,584,576 | 2,531,328 |
| Retained absorber snapshots | 32 | 256 |
| Snapshot hits / cold misses / repeat misses | 320 / 32 / 0 | 256 / 256 / 0 |
| Evictions / oversized snapshots | 0 / 0 | 0 / 0 |

Both checks used k = 2–12 Å⁻¹, R = 1.5–3.5 Å, k weight 2, fixed S₀² = 1 and
ΔE₀ = 0 eV, and seed 20260918. Auto resource settings were retained. The larger
case used 910.09 MiB of retained snapshot payload under a 1284 MiB cache limit;
the limit is not total application memory. Preparation displayed live stages
and counts, and the interface responded to view changes. All 256 sites completed
without the former one-million-path error. Catalogue capacity is a guard, not a
measurement of enumerated paths. The additional 4.20 s in that one-attempt check
includes move and completion work; it is not a repeated performance benchmark.

The small result was saved, exported and reopened without starting another
calculation. Its checkpoint agrees with exported results. All eleven structural
samples (steps 0–10) were retained; CSV and JSON values agree. Independent checks
confirmed initial fcc coordination 12, mean distance a/√2 = 2.55646 Å, and
g(r) normalization with density 4/a³ and spherical shell volumes. Coordination,
mean and variance views rendered. These are software checks: ten attempts do
not establish convergence, calibration, fit uncertainty or physical accuracy.
Amplitude mismatch remains visible.

The complete report export produced six PNG/SVG/CSV figure sets, captions,
methods, references and an embedded project whose RMC checkpoint matches the
saved run. The double-column χ(k) figure is 2100 × 1440 pixels (7 × 4.8 inches
at 300 DPI). A subsequent grid edit changed the header to “Changes since last
publish” and preserved the previous export. Compact presets expose names and
dimensions through their native hover/accessibility descriptions.

## Publication and public-byte verification

Publication workflows for [crates.io](https://github.com/Ameyanagi/rexafs/actions/runs/35516789999),
[PyPI](https://github.com/Ameyanagi/rexafs/actions/runs/35516791738),
[npm](https://github.com/Ameyanagi/rexafs/actions/runs/35516793287) and
[GitHub draft staging](https://github.com/Ameyanagi/rexafs/actions/runs/35516794596)
passed. The signed Mac ZIP replaced the unsigned draft ZIP; the notarized DMG,
installation evidence and regenerated checksums were included before publication.
The stable [GitHub release](https://github.com/Ameyanagi/rexafs/releases/tag/v0.2.12)
was published at **14:57:42 UTC on 20 September 2026**.

At **14:57:57 UTC**, all **22 desktop files** were downloaded from their public
release URLs and matched against the final qualified bytes and GitHub SHA-256
digests. All **six registry files** (crate, npm archive, Python source archive
and three wheels) matched the original exact-tag manifest. The crate SHA-256 is
`ece951c6e1f613645bf029c1138b4379fe87b447da8effec12a07493d69f7f23`.
Installed public Python and npm packages passed the documented Cu processing
examples; both reported E₀ = 8977.493 eV. Desktop and Python inventories contain
no Intel Mac binaries. Historical 0.2.11 Intel desktop files and wheel remain
available. Website stable metadata was advanced only after these checks.

Raw artifacts, public downloads, native projects, exports and verification logs
are retained locally under `target/release-0.2.12-refresh/`, outside source
control. Public workflow logs, asset checksums, installation evidence and the
committed capture manifest provide the published provenance. A draft-by-tag API
lookup returned 404 before publication; the complete public asset inventory and
bytes were verified immediately afterward. No failed lookup is counted as an
asset verification.

## Documentation refresh checks

Stable Python and TypeScript pages were regenerated from the immutable tag, and
the Stable Rust reference was built from the checksum-verified published crate.
Next remains separately labeled. Version validation, all eight generator tests,
Astro diagnostics (zero errors/warnings), the production build and all 23
content tests passed. The initial browser run passed 24 of 25 checks; its download
test still expected six targets. That expectation was updated to require five
from 0.2.12, including exactly one Apple Silicon Mac package, while retaining
historical four- and six-target rules. Both download/homepage checks then passed.
The local browser preview used port 4399 because another project owned 4321.

Eight new image hashes were rechecked against their original capture bytes.
Historical images and measurement attribution were preserved. The website
metadata identifies the published tag and five verified desktop downloads.
