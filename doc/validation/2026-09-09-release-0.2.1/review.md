# Release 0.2.1 qualification

[rexafs 0.2.1](https://github.com/Ameyanagi/rexafs/releases/tag/v0.2.1) was
published on **2026-09-09T07:37:29Z** and verified as GitHub's latest stable
release. PyPI, crates.io, and npm also select 0.2.1 as their default stable
version. All public asset and package hashes match the qualified outputs.

## Preparation

The release starts from `411a92f32ba1b135d64db48d4a29aa69ba96bac5`, which
contains the merged startup/Help changes, UI and structure improvements, and
ruviz/ruviz-gpui 0.14.1 update. PR #47 passed all 35 checks, and the merged
main Rust run passed. The preparation changes coordinated versions, release
documentation, and the retained-project writer/tests; production behavior is
unchanged from that merged source.

The 0.2.1 writer generated linked and embedded projects from the retained 0.2.0
sample. They preserve the earlier import and Assistant state and add the
AUTOBK/FFT weight link, its independent background value, reversed Viridis group
assignments, and publication style with a mixed Japanese/Latin title. Both were
reopened through the production decoder. All **22** previous sample hashes are
unchanged; the manifest now checks **24** samples.

The complete optimized GUI suite passed **450 tests**, zero failed, five
ignored, with ReFEFF and FEFF10 enabled. The new fixture regression checks that
unlinking restores the independent weight and that palette identities and
publication settings survive reading both storage modes. Existing tests cover
full retained-state round trips and historical defaults.

Coordinated version validation, formatting, whitespace checks, three release
maintenance tests, four registry-artifact tests, and two desktop-download tests
passed. The dependency lock changes only the four workspace package versions.
Local logs: `/private/tmp/rexafs-021-fixture-writer.log` and
`/private/tmp/rexafs-021-gui-tests.log`.

## Tagged build and publication

The immutable tag `v0.2.1` identifies release preparation commit
`67ea97f68073123f10cb4fcf5acb816a171e7639`.
[Manual release build 34318676399](https://github.com/Ameyanagi/rexafs/actions/runs/34318676399)
passed all **29 jobs** at that exact commit, including 20 installed/tested Python
wheel targets, a rebuilt source archive, the Rust crate, npm consumers, and four
desktop packages. Windows installer install/reinstall/uninstall checks passed.
[Main Rust run 34318657718](https://github.com/Ameyanagi/rexafs/actions/runs/34318657718)
passed all five jobs, including core plotting and the benchmark regression gate.

[Mac signing 34323372368](https://github.com/Ameyanagi/rexafs/actions/runs/34323372368)
passed for Apple Silicon and Intel without rebuilding the original executables.
[Draft creation 34323375062](https://github.com/Ameyanagi/rexafs/actions/runs/34323375062),
[Rust publication 34323378024](https://github.com/Ameyanagi/rexafs/actions/runs/34323378024),
[PyPI publication 34323381496](https://github.com/Ameyanagi/rexafs/actions/runs/34323381496),
and [npm publication 34323384421](https://github.com/Ameyanagi/rexafs/actions/runs/34323384421)
all passed using the same tag and manual build. All registry channels used their
established trusted publishers. Rust reproduced and compared its crate before
upload; Python and npm published their checked build artifacts.

All 20 Python wheels and the source archive report the qualified build hashes.
The Rust crate checksum and the downloaded npm tarball also match the original
[35-file build manifest](build-SHA256SUMS). All three default stable registry
endpoints were checked after their publication metadata had propagated.

## Final desktop downloads

Both signed ZIPs and DMGs were downloaded afresh. Their build metadata identifies
version/tag 0.2.1, the source commit and build/signing runs above, and Developer ID
team `XXN44W8X56`. The recorded unsigned archive hashes match the original build.
Installer evidence binds each DMG to its signed ZIP and executable hash. Both
extracted dependency inventories contain ruviz and ruviz-gpui **0.14.1**.

Both apps report the expected version, commit, channel, tag, calculation engines,
and architecture. App signatures have Hardened Runtime and a timestamp. Both apps
and DMGs passed local signature verification, stapling validation, and Gatekeeper.
Both apps passed the packaged numerical check; Apple Silicon additionally passed
local ReFEFF and FEFF10 first-shell checks. Signing CI also checked fresh installer
copies and their packaged calculations on both native runner architectures.

The ten final signed Mac files replaced the four unsigned Mac draft files and
added both DMGs with their evidence/checksum sidecars. Windows and Linux files
retain their original qualified hashes. The public release contains **19** desktop
assets: 18 archives/installers/checksums/evidence files plus `SHA256SUMS`.
All 19 GitHub asset digests were compared with the local files before and after
publication. The [public desktop manifest](publication-SHA256SUMS) has SHA-256
`97491d4718d2b3dd7de798c1c9053f14921af341c655889f3660d1ffe3638594`. Registry packages remain available through their registries.

## Available-host interaction checks

The signed Apple Silicon app opened in the complete, empty Data workspace. Help
opened its packaged Cu example; explicit mu-column/eV confirmation imported one
618-point group. Processing, Fourier plots, pan/zoom, and complete angstrom labels
worked in dark and light themes. Publication previews rendered clean four-point
strokes without the previous sharp join spikes.

Opening a disposable embedded 0.2.1 project restored seven groups, two marks,
the processing lock, pending-source record, group palette assignments, and the
AUTOBK weight link (3 from FFT, with independent value 1 retained). The saved
300-DPI/two-point publication style and mixed Japanese/Latin title rendered.
Saving to `/private/tmp/rexafs-021-native-saved.rxs` preserved the link, colors,
marks, lock, publication settings, recipe version-1 application, and both Assistant
conversations. The original disposable input and retained fixture bytes remained
unchanged.

The signed Intel app launched under Rosetta and reopened the project saved by
the Apple Silicon app. It restored the seven groups, marks/lock, processed Cu
spectrum, and saved publication style/title. This is an Apple Silicon host check;
native Intel hardware, clean-machine installation, and interactive Windows/Linux
qualification remain outstanding. Windows/Linux retain preview labels. The earlier
UI/renderer reviews retain their broader interaction-coverage limits.

Local evidence: `/private/tmp/rexafs-021-final-check.log`,
`/private/tmp/rexafs-021-final-arm-feff.log`,
`/private/tmp/rexafs-021-registries.log`,
`/private/tmp/rexafs-021-draft-final-assets.json`, and
`/private/tmp/rexafs-021-public-release.json`. The source tag remains fixed; this
completed qualification record is a documentation-only follow-up.
