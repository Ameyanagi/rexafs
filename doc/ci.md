# CI selection and release coverage

CI selects checks from changed files while retaining full release coverage.
Historical full-build timings appear below.

| Changed files or event | Checks |
|---|---|
| `website/**` | Complete Website build, generators, content/browser tests; lightweight Rust and release selectors |
| `doc/documentation-audience.csv` or allowed root/doc presentation files | Lightweight selectors; Website runs when its declared inputs change |
| Native code, dependencies, packages, fixtures, licenses, assets, scripts, CI or unknown paths | Full Rust checks and release qualification, including three Python wheels, 15 Python runtime environments and five desktops |
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
Windows installer checks run for pull requests, relevant pushes to `main`, and
manual requests. Feature-branch pushes no longer duplicate their PR checks.

Added caches cover core dependencies, compatible nightly macOS dependencies
and pinned `wasm-pack`. Native release jobs restore without saving caches;
workspace crates and package outputs are cleared before rebuilding. The shared
build tool is version-checked. Release artifacts are always rebuilt. Cold
dependency caches require main/nightly builds to populate them.

Website builds reuse Rust and WebAssembly compilation dependencies. Only
successful `main` builds save the large shared dependency cache, keyed by the
toolchain and dependency inputs, with a compatible fallback when those inputs
change. This avoids a new multi-gigabyte entry for every commit or PR.
The browser engine's workspace crates, generated wrappers and provenance
manifest are rebuilt from the current checkout.

Stable and Next Rust reference HTML have separate caches. Their identities
include compiler versions, the builder, the header and feature policy. Stable
uses the published crate checksum; Next hashes the current source, embedded
data, manifests, lockfile and local dependencies. Restores verify all retained
file hashes and replace public output completely. Missing, altered or extra
files cause regeneration. Copy-only edits can reuse both references; the full
website content, browser and accessibility checks still run.

The source workflow builds one ABI3 wheel for each of the three supported Python
platforms. [ABI3 is CPython's stable binary interface](https://pyo3.rs/v0.29.2/building-and-distribution.html#py_limited_apiabi3abi3t):
the same platform wheel can serve Python 3.10–3.14. All 15 platform/interpreter
combinations verify the installed wheel's bytes and run the API checks against
both the minimum and latest compatible NumPy. From 0.2.12, macOS desktop and
Python builds support Apple Silicon only. The published 0.2.6–0.2.11 inventories
retain four wheels, including Intel macOS; 0.2.5 retains its per-interpreter ABI.

On 2026-09-13, [PR build 34751534452](https://github.com/Ameyanagi/rexafs/actions/runs/34751534452)
took **73m29s**. Intel macOS desktop queued **30m05s**, then ran **43m05s**.
The Windows regression step took **25m35s** including compilation; its tests
took **31.17s**. Python API tests took under one second. Compilation and queues
dominated. These baseline timings do not measure cache speedups.

The [0.2.5 tagged build](https://github.com/Ameyanagi/rexafs/actions/runs/34754751069)
retains its original workflow. Follow the [release runbook](releasing.md) for
source and artifact qualification.
