# CI selection and release coverage

This checkout's PR and release workflow changes await post-merge measurement.
Historical timings appear below.

| Changed files or event | Checks |
|---|---|
| `website/**` | Complete Website build, generators, content/browser tests; lightweight Rust and release selectors |
| `doc/documentation-audience.csv` or allowed root/doc presentation files | Lightweight selectors; Website runs when its declared inputs change |
| Native code, dependencies, packages, fixtures, licenses, assets, scripts, CI or unknown paths | Full Rust checks and release qualification, including 20 Python wheels and six desktops |
| Manual release or reusable workflow | Full qualification by default; manual releases always remain full |

The [selector](../scripts/ci_scope.py) checks immutable commits: pull requests
use their merge-base diff; opted-in Rust pushes compare the previous and new
trees. Unknown paths, missing history and empty diffs require full checks.
Crate READMEs and benchmark data are package/test inputs. Both selectors check
coordinated versions and run selector/result self-tests. Reusable PR callers
must explicitly opt out of full qualification.

The stable aggregate jobs are **Rust checks** and **Release checks** in
the [Rust](../.github/workflows/rust.yml) and
[release](../.github/workflows/release-build.yml) workflows. Their
[result validator](../scripts/check-ci-results.py) rejects failed selectors,
missing results and unexpected skips. The release manifest requires every
qualification job to pass; reduced PR checks produce no partial manifest.

Rust, Release builds and Larch comparison cancel only superseded PR revisions.
Separate non-PR groups preserve manual builds. Website retains its existing
cancellation of superseded runs for the same ref.

Added caches cover core dependencies, compatible nightly macOS dependencies
and pinned `wasm-pack`. Native release jobs restore without saving caches;
workspace crates and package outputs are cleared before rebuilding. The shared
build tool is version-checked. Release artifacts are always rebuilt. Cold
dependency caches require main/nightly builds to populate them.

On 2026-09-13, [PR build 34751534452](https://github.com/Ameyanagi/rexafs/actions/runs/34751534452)
took **73m29s**. Intel macOS desktop queued **30m05s**, then ran **43m05s**.
The Windows regression step took **25m35s** including compilation; its tests
took **31.17s**. Python API tests took under one second. Compilation and queues
dominated. No speedup is claimed until a comparable run measures these changes.

The [0.2.5 tagged build](https://github.com/Ameyanagi/rexafs/actions/runs/34754751069)
retains its original workflow. Follow the [release runbook](releasing.md) for
source and artifact qualification.
