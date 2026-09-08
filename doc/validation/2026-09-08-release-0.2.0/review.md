# Release 0.2.0 qualification

The Groups/import integration and release preparation merged as
[PR #41](https://github.com/Ameyanagi/rexafs/pull/41) and
[PR #42](https://github.com/Ameyanagi/rexafs/pull/42). PR #42 passed all 34 checks:
[Release builds 34198489189](https://github.com/Ameyanagi/rexafs/actions/runs/34198489189),
[Rust 34198489183](https://github.com/Ameyanagi/rexafs/actions/runs/34198489183), and
[Larch comparison 34198489209](https://github.com/Ameyanagi/rexafs/actions/runs/34198489209).
The reviewed merge `7278be4ce91b80de235ce785782b46e7f5647447` is tagged `v0.2.0`;
its tree exactly matches the fully tested candidate `8a317ff`.
[Final manual build 34204231697](https://github.com/Ameyanagi/rexafs/actions/runs/34204231697)
passed all 29 jobs. It is the source for [Mac signing 34208785388](https://github.com/Ameyanagi/rexafs/actions/runs/34208785388)
and [GitHub draft creation 34208788450](https://github.com/Ameyanagi/rexafs/actions/runs/34208788450).
Signing and final downloaded-artifact qualification passed as recorded below.
All three registry publications passed, their package hashes match the qualified
build, and [GitHub 0.2.0](https://github.com/Ameyanagi/rexafs/releases/tag/v0.2.0)
is public and latest. All three registries select 0.2.0 as the default stable version.

## Final signed artifacts and available-host qualification

The signing run passed for both architectures without rebuilding the original
executables. Both final ZIPs and DMGs were downloaded afresh. Archive and
installer provenance identifies build 34204231697, signing run 34208785388,
source commit `7278be4ce91b80de235ce785782b46e7f5647447`, and Developer ID team
`XXN44W8X56`. The unsigned archive hashes match the original build manifest;
installer evidence matches the signed archives and their executable hashes.

Both extracted apps report the expected version, commit, channel, tag, features,
and architecture. App signatures include Hardened Runtime, a timestamp, and the
expected Developer ID. Both apps and DMGs passed signature verification,
stapling validation, and Gatekeeper. Both apps passed the packaged numerical
self-check. Apple Silicon ReFEFF and FEFF10 first-shell checks passed.

The signed Apple Silicon app rendered its bundled Cu example and opened a
portable copy of the retained embedded project without adjacent source files.
It restored seven groups, two marks, the processing lock, pending source,
Reference channel and plot, and `Retained Cu mapping · v1`. Reviewing the saved
application captured exactly one compatible source and rendered all 618 raw
points; Cancel kept its mapping. Source Details displayed nine malformed rows
and example lines 620–624. Both saved Assistant conversations were present;
opening one rendered its historical recorded receipt and transcript.

The signed Intel app launched and rendered the Cu example under Rosetta, then
restored the same seven-group project, two marks, lock, pending source, version-1
receipt, and 645-point Reference plot. Intel qualification used Rosetta on Apple
Silicon. Native Intel hardware remains unqualified. The portable input's bytes
remain identical to the retained fixture.

Both signed ZIP/checksum pairs replaced their unsigned draft assets, and both
signed DMGs and qualification sidecars were added. At initial publication, all
**42** GitHub asset digests matched the final local files, including `SHA256SUMS`
(which covered the other 41 assets). The original unsigned build manifest is
retained separately. The [initial publication manifest](publication-SHA256SUMS)
has SHA-256
`94a3225c39a673cc7372b6d3cf5799890c2e20034db051ab73d15dab18f2b90d`.

Final evidence: `/tmp/rexafs-020-final-check.log`,
`/tmp/rexafs-020-final-arm-feff.log`,
`/tmp/rexafs-020-draft-asset-digests.json`, and
`/tmp/rexafs-020-final-project.rxs`. Public release identity, notes, and all asset
digests were checked again after publication; the response is retained in
`/tmp/rexafs-020-public-asset-digests.json`.

## Registry publication

The original publisher's two attempts for each registry stopped while fetching
unrelated Mac ARM/Linux desktop artifacts, before any registry upload. The
package artifacts remained downloadable and matched the original manifest.
[PR #43](https://github.com/Ameyanagi/rexafs/pull/43) makes each registry require
only its exact package set plus that manifest, including every Python wheel and
the source archive. Missing, unexpected, duplicate, wrong-version, and modified
packages are rejected. Four registry tests, three maintenance tests, workflow
lint, the new Linux CI checks, and fresh downloads of the real 1/1/21 registry
packages passed.

Publication resumed from the reviewed tooling tag `v0.2.0-publish-tools.1`
(`ace3ed785d9900259cbe42c4f6f9ee205924f4fa`) with explicit `release_tag=v0.2.0`.
The source validator binds the original release tag to build 34204231697. Rust
is checked out at the original source commit before package reproduction and
comparison. Trusted publishers and the tag-restricted release environment are
preserved; the source tag and qualified release binaries remain fixed.

[Rust publication 34211846925](https://github.com/Ameyanagi/rexafs/actions/runs/34211846925),
[npm publication 34211849827](https://github.com/Ameyanagi/rexafs/actions/runs/34211849827), and
[PyPI publication 34211852979](https://github.com/Ameyanagi/rexafs/actions/runs/34211852979)
all passed using the same final manual build. Rust reproduced and compared the
original crate before publishing. PyPI's first tooling attempt stopped before
upload on a wheel connection reset; its second attempt succeeded.

The published Rust checksum and downloaded npm tarball's SHA-256 match the
original build manifest. All 20 PyPI wheels and the source archive report the
same SHA-256 hashes as that build. The default stable endpoints on crates.io,
npm, and PyPI all select 0.2.0. Verification output is retained in
`/tmp/rexafs-020-registries.log`. Only after these checks passed was the GitHub
draft made public and marked latest on 2026-09-08 at 09:55 UTC.

## Desktop download cleanup

The public asset list was subsequently reduced to **19** files: the same 18
desktop archives/installers/checksums/evidence files plus a desktop-only
`SHA256SUMS`. The 23 duplicate registry packages were removed from GitHub after
rechecking their published registry hashes; all remain available through PyPI,
crates.io, and npm. No desktop binary, installer, package version, or source tag
changed. Offline package installation is documented in
[installing.md](../../installing.md).

All 19 public asset digests match the desktop files. The desktop manifest's
SHA-256 is `c5ff5fe9dbc4215f96ab03d7668639084438d61499d65a46f8130325cb1c6e8c`.
The original 41-file manifest above preserves the full publication evidence.
Before/after API records are retained in
`/tmp/rexafs-020-before-download-cleanup.json` and
`/tmp/rexafs-020-desktop-public-assets.json`.

## Local release gates

The coordinated versions and retained fixture manifest verify. All **22**
samples are checksummed, with every previously released byte preserved. The new
linked and embedded projects were saved by the 0.2.0 writer and reopened through
the production decoder; project tests passed **25 tests**, zero failures, one
ignored maintainer writer. The complete GUI suite passed **427 tests**, zero
failures, four ignored.

Core/default, ndarray, and trust-region tests passed, as did strict core clippy,
optional plotting/FEFF/structure-provider integration, rustdoc, license policy,
the Apache sum_tree patch tests, and verified Cargo packaging. The packaged
crate contains both declared licenses and intact, decodable compressed structure
databases. Formatting, release/desktop/archive/installer/benchmark script tests,
and actionlint passed. GUI builds retain existing warnings.

The release desktop build and freshly extracted package passed numerical,
ReFEFF, and FEFF10 self-checks. The local package was built from preparation
commit `bfc166b`; it is a qualification output, not a public release artifact.

Python wheels passed installation and API tests in fresh CPython 3.10–3.14
environments. The source archive's declared license paths verified; a fresh source rebuild
and its API tests passed under the declared Rust toolchain. JavaScript/Wasm
build, Node tests, real Chromium pipeline, and installed tarball/TypeScript
consumer checks passed.

## Native Apple Silicon candidate

The packaged app launched and rendered its Cu example. Opening the retained
embedded 0.2.0 project restored seven groups, two marks, a processing lock, the
pending missing source, and the existing Reference channel and plot. The import
receipt and Data inspector identified `Retained Cu mapping · v1`. Reviewing its
application captured exactly one compatible source and showed the full 618-point
raw preview; Cancel retained the original mapping. Version 2 remains available
for future reuse without rewriting the version-1 application.

The diagnostics source displayed all nine malformed rows and example lines
620–624. The synthetic legacy result retained its quantity-confirmation state.

A disposable embedded copy was used for the Normalize-copy acceptance workflow.
After unlocking the marked diagnostics sample, the other marked sample's
pre-edge start was changed to −180 eV and Apply Normalize copied it to exactly
one other group. The Reference channel retained its original −200 eV default
and independent edge/background settings. Native Save wrote
`/tmp/rexafs-020-native-after.rxs`. Inspection confirmed both sample settings,
unchanged import mappings and marks, the identical Reference group ID and all
Reference parameters, retained version-1 application membership, and two saved
Assistant conversations. All 22 retained fixture hashes remained unchanged.

## Evidence and remaining qualification

Local logs and exit statuses: `/tmp/rexafs-020-gates/`. Fixture writer and project
logs: `/tmp/rexafs-020-fixture-writer.log` and
`/tmp/rexafs-020-project-tests.log`. Native disposable input/output:
`/tmp/rexafs-020-native.rxs` and `/tmp/rexafs-020-native-after.rxs`.

Native Intel hardware, clean-machine installation, and interactive Windows/Linux
qualification remain outstanding. Windows/Linux retain preview labels. Current
upstream dependency advisories remain without suppressions; the license gate
passes. Series/frame workflows and deferred Assistant features are outside this
release's implementation scope.
