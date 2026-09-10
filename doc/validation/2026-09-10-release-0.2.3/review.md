# rexafs 0.2.3 release qualification

[rexafs 0.2.3](https://github.com/Ameyanagi/rexafs/releases/tag/v0.2.3) was
published on **2026-09-09T23:56:19Z** and verified as GitHub's latest
stable release. PyPI, crates.io and npm also select 0.2.3 as their default stable
version. All public asset and package hashes match the qualified outputs.

## Source and issue/PR review

The immutable tag `v0.2.3` identifies source commit
`69607652a237ccc87ababb84cf1a765f5cde33b8`. It includes Windows fix
`29a1191cd2bf016c09bdc430493998e8f0954395` and corrects the native accessibility
adapter return type. The [unpublished 0.2.2 build](../2026-09-10-release-0.2.2/review.md)
exposed the Unix compile failure: the Unix adapter returns unit, while Mac and
Windows return optional queued events. The corrected dispatch handles both,
preserves no-change skipping, and keeps queued notifications outside the state
borrow. Regular CI now compiles the native Linux GUI with its actual Unix
accessibility dependency. The 0.2.2 tag remains unchanged; its release was withheld.

[PR #39](https://github.com/Ameyanagi/rexafs/pull/39) is excluded. Its reviewed head
`3c631fb1061a138cf5ba2e9ba162114ef381140a` updates nalgebra 0.34.2 to 0.35.0,
which fails at the Levenberg–Marquardt 0.15 matrix trait boundary. The
[failed core job](https://github.com/Ameyanagi/rexafs/actions/runs/34300847335/job/102307221616)
reports `E0277` in `lcf.rs`. Compatible solver versions are retained.

[Issue #20](https://github.com/Ameyanagi/rexafs/issues/20) remains open for the
legacy dynamic-clamp Jacobian and output-FFT finite-grid window differences.
Earlier fixed-penalty/default solver improvements remain in place. This patch
changes no numerical processing code or scientific defaults.

The concurrently prepared desktop quality changes in
[PR #48](https://github.com/Ameyanagi/rexafs/pull/48) and
[PR #49](https://github.com/Ameyanagi/rexafs/pull/49) target 0.2.4 and are outside
this immutable source tag.

## Local preparation

Verified locally on Apple Silicon macOS with Rust 1.98.1:

- Optimized desktop suite with ReFEFF and FEFF10: **458 passed, 0 failed,
  5 ignored**, including shortcut and accessibility diff tests.
- The explicit 0.2.3 fixture writer saved, loaded and compared both storage modes.
  **28 retained samples** pass checksum/header validation. All 26 earlier sample
  bytes and hashes, including the unpublished 0.2.2 attempt, are preserved.
- Coordinated workspace/Python/npm version and `v0.2.3` name check passed.
- `cargo fmt --all -- --check`, `git diff --check`, and `actionlint` for the
  changed regular CI workflow passed.
- The lockfile changes only four workspace package versions. Release helper
  suites were rerun by the successful tagged build.

## Tagged build and registry publication

[Manual release build 34411272061](https://github.com/Ameyanagi/rexafs/actions/runs/34411272061)
passed all **29 jobs** for the exact tagged commit. This includes all four desktop
targets, 20 installed/tested Python wheel targets, the rebuilt Python source
archive, Rust crate, npm consumers, license checks and the final manifest.
[Main Rust run 34410729175](https://github.com/Ameyanagi/rexafs/actions/runs/34410729175)
passed all **six jobs**, including the added native Linux GUI compilation,
strict checks, core plotting and the benchmark regression gate.

[Draft creation 34415852270](https://github.com/Ameyanagi/rexafs/actions/runs/34415852270),
[Rust publication 34415865416](https://github.com/Ameyanagi/rexafs/actions/runs/34415865416),
[PyPI publication 34415880909](https://github.com/Ameyanagi/rexafs/actions/runs/34415880909),
and [npm publication 34415895122](https://github.com/Ameyanagi/rexafs/actions/runs/34415895122)
all passed using that same source tag and build. Registry uploads used the
established trusted publishers. Rust reproduced and compared the crate before
upload; Python and npm published their checked build artifacts.

All 20 wheels and the source archive report the qualified build hashes. The
Rust crate checksum and downloaded npm tarball also match the original
[35-file build manifest](build-SHA256SUMS). All three default stable registry
endpoints selected 0.2.3 after their publication metadata propagated.

## Desktop artifact checks

The initial GitHub draft's 13 asset digests and 12 original desktop build hashes
matched the downloaded files. The Linux archive reports the expected version,
tag, commit, build run, both FEFF engines, ruviz/ruviz-gpui 0.14.1 and x86-64 ELF
architecture.

Windows archive and installer evidence identify the exact source and build.
The executable has the Windows GUI subsystem. Every portable archive payload
hash matches its installer record; the additional Microsoft runtime DLL hashes
and signer records match that record. Windows runner qualification passed
installation, reinstallation, uninstallation, all payload hashes, Start menu and
desktop shortcuts, uninstall registration, build identity, packaged calculations,
embedded ReFEFF, Unicode installation paths, and preservation of a user project.
These are native CI installer checks and local artifact inspection, not a local
interactive Windows check.

[Mac signing 34415836385](https://github.com/Ameyanagi/rexafs/actions/runs/34415836385)
passed for Apple Silicon and Intel without rebuilding the qualified executables.
Both signed ZIPs and DMGs were downloaded. Their metadata identifies version/tag
0.2.3, the source commit and build/signing runs above, and Developer ID team
`XXN44W8X56`. Recorded unsigned archive hashes match the original build manifest.
Installer evidence binds each DMG to its signed ZIP and executable hash. Both
extracted dependency inventories contain ruviz and ruviz-gpui **0.14.1**.

Both apps report the expected build identity and architecture. App signatures
have Hardened Runtime and a timestamp. Both apps and DMGs passed local signature
verification, stapling validation and Gatekeeper. Each DMG was mounted and copied
into a temporary installation; the installed executable hash matched its
installer evidence. Both temporary installations passed numerical, ReFEFF and
FEFF10 first-shell checks. Intel execution used Rosetta on the local host;
signing CI also checked temporary installations on both native runner architectures.

The ten signed Mac files replaced the four unsigned Mac draft files and added
both DMGs with their checksum/evidence sidecars. Windows and Linux files retain
their original qualified hashes. The public release contains **19** desktop
assets: 18 archives/installers/checksums/evidence files plus `SHA256SUMS`. All 19
GitHub digests, sizes and upload states matched before and after publication.
The [public desktop manifest](publication-SHA256SUMS) has SHA-256
`61d43e3260d073c0a638bafdc8f5e0d8579217a970bfb28fac5dc4181a7443e7`.

## Available-host interaction checks

Before signing, the exact Apple Silicon build opened a complete empty Data
workspace. Help's packaged Cu example imported as one 618-point group after
mu-column mapping and explicit eV confirmation. Cmd+3 and Cmd+4 changed stages.
Editing the background radius to 1.2, moving focus outside the editor, then using
Cmd+Z and Cmd+Shift+Z restored the default and edited values respectively. The
accessibility controls reflected each change. Plot scroll zoom, drag pan and
complete angstrom labels worked; dark and light themes were exercised.

The native Open dialog loaded a disposable embedded 0.2.3 project with seven
groups, two marks, a processing lock, pending-source record, palette assignments,
and the linked AUTOBK/FFT weight. The saved 300-DPI/two-point k-space publication
figure rendered its mixed Japanese/Latin title. Native embedded Save wrote
`/private/tmp/rexafs-023-unsigned-saved.rxs`. Retained import/recipe, embedded
payload, fitting, publication, Assistant and specimen records matched the input.
All original group state and processing overrides were preserved; saving also
materialized fallback override records and Cu parser evidence. The input still
matched the retained fixture bytes.

The exact unsigned Intel build launched under Rosetta and reopened that saved
project. The seven groups, marks, lock, processing plots, publication style and
title rendered correctly. Both disposable pre-signing app sessions were closed.

The final signed Intel app opened the disposable retained project under Rosetta.
It restored the seven groups, marks, lock, palette and pending-source record;
Background displayed the linked weight as 3 from FFT. Editing the radius from
1.2 to 1.3 and using Cmd+Z outside the editor restored 1.2. The publication figure
retained its 300-DPI/two-point style and mixed Japanese/Latin title. Cmd+S opened
the native save flow and wrote `/private/tmp/rexafs-023-native-intel-saved.rxs`.

The final signed Apple Silicon app reopened that Intel-saved project. Cmd+4
opened the restored Fourier plots, and the publication figure rendered the same
style, title and angstrom glyphs. Native Save wrote
`/private/tmp/rexafs-023-native-saved.rxs`.

Both final saves preserve the original imports, recipes, embedded payload,
marks, locks, linked and independent weights, palette assignments, fitting,
publication and Assistant state. Effective processing parameters are unchanged.
Undo removes a redundant group override when it equals the project baseline;
that expected representation change was checked against the production journal
and decoder defaults. The original fixture bytes remain unchanged. Both final
app sessions were closed after qualification.

## Platform coverage

Available local hardware is Apple Silicon macOS; Intel interaction uses Rosetta.
The [Windows development-build review](../2026-09-09-windows-gui/README.md)
records native interaction and software-renderer timing limits separately from
final-download qualification. Native Intel hardware, clean-machine graphical
installation, and interactive final Windows/Linux downloads remain outside this
release's available coverage. Windows/Linux retain preview labels.

Local evidence includes `/private/tmp/rexafs-023-gui-tests.log`,
`/private/tmp/rexafs-023-unsigned-interaction.json`,
`/private/tmp/rexafs-023-registries.log`, and
`/private/tmp/rexafs-023-draft-original-release.json`.
Final signed-download evidence is recorded in
`/private/tmp/rexafs-023-final-check.log`,
`/private/tmp/rexafs-023-native-save.log`, and
`/private/tmp/rexafs-023-public-release.json`. The source tag remains fixed;
this completed record and its two manifests are a documentation-only follow-up.
