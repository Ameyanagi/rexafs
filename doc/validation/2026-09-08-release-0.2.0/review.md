# Release 0.2.0 qualification

Release candidate preparation is based on the Groups/import integration in
[PR #41](https://github.com/Ameyanagi/rexafs/pull/41), including its Windows path
correction. Release 0.2.0 has not yet been tagged or published. Final build,
signing, registry, and downloaded-artifact evidence will be recorded separately
from the local candidate checks below.

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
