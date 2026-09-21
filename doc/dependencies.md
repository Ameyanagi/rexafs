# Rust dependency update record

## Post-0.2.12 source update — 21 September 2026

The dependency PR review combines `rand 0.10.2` and `rand_chacha 0.10.0`.
Upgrading either alone mixes incompatible random-number traits. RMC imports now
use `RngExt`, following the [Rand migration guide](https://rust-random.github.io/book/update-0.10.html).
The engine retains explicit `rand_chacha::ChaCha8Rng` with `serde`, including
the saved seed, stream and word position; checkpoint formats are unchanged.

A [retained reference](../crates/rexafs/tests/fixtures/rmc_rng/README.md), generated
with the released 0.2.12 dependency versions (`rand 0.9.5`, `rand_chacha 0.9.0`),
checks seed expansion, deserialization, cloned streams, integer/float/Boolean
draws, shuffling and final serialized state. The four-seed reference and the
existing RMC/EA session tests passed with the paired upgrade. This tests software
continuation, not the physical accuracy or convergence of an RMC fit.

The same review selects [hdf5-pure 0.46.1](https://github.com/CramBL/hdf5-pure/releases/tag/v0.46.1)
and [ureq 3.4.2](https://github.com/algesten/ureq/blob/3.4.2/CHANGELOG.md), with
`ureq-proto 0.6.4` in the lockfile. Full platform/package CI qualifies the combined
change. These are source updates for the next release; published 0.2.12 artifacts
and their immutable tag retain their original dependencies.

Dependabot groups the two RNG packages so future updates can be qualified
together. Its Python scan excludes only `scripts/cu-mixtures-requirements.txt`:
that is the historical experiment environment, and `generate-cu-mixtures.py`
explicitly requires rexafs 0.2.8. Updating that pin alone would break reproduction.
Other Python dependencies remain in scope. See the
[Dependabot options](https://docs.github.com/en/code-security/reference/supply-chain-security/dependabot-options-reference)
and [experiment record](validation/2026-09-16-cu-mixtures/README.md).

## Original September 6 update

The release uses **Rust 1.98.1**, the latest stable verified on 2026-09-06 with
`rustup check` and the [Rust release announcements](https://blog.rust-lang.org/).
`rust-toolchain.toml` pins that verified toolchain; update it together with release
CI when adopting a newer stable. Nightly is not required.

Direct dependencies were checked against the crates.io API on that date and
updated across the Rust workspace, including the Python and Wasm binding crates.
The workspace lockfile records the resulting transitive versions. Unused
`serde_arrow` was removed instead of retaining an unused Arrow dependency tree.

## Compatibility constraints

| Dependency | Selected line | Reason |
|---|---|---|
| nalgebra | 0.34.2 | Latest Levenberg–Marquardt 0.15 exposes nalgebra 0.34 types; nalgebra 0.35 is not interchangeable at that trait boundary |
| nalgebra-apex | 0.33.3 | Latest apex-solver 1.4 exposes nalgebra 0.33 types; the solver adapter keeps this separate |
| rusqlite | 0.31 | apex-io 0.3 links SQLite through this version; Cargo rejects a second libsqlite3-sys `links = sqlite3` version |
| faer | 0.24 | Matches the latest apex-solver factor/Jacobian interface |
| GPUI / gpui_platform | Existing shared git revision | Latest registry GPUI remains 0.2.2; the desktop needs the matching platform entry point and macOS font-kit feature |

Other direct dependencies use the newest compatible resolution of the updated
requirements. This does not force every transitive crate to its newest major:
upstream public types, feature contracts and native-link constraints still apply.

## Published XrayDB dependency

The desktop uses [`xraydb` 0.4.1](https://crates.io/crates/xraydb), the published
library from [`Ameyanagi/xraydb-rs`](https://github.com/Ameyanagi/xraydb-rs).
This is the latest stable crates.io release verified on 2026-09-06. Cargo resolves
both `xraydb` and `xraydb-data` from crates.io; there is no local fork or path patch.
The unused local `xraydb-rs/` scaffold has been removed.

`SpectrumInterest` uses its embedded reference database to identify likely
absorption edges from E₀ when XDI metadata is absent. Explicit element/edge
metadata takes priority, and a database estimate never changes an existing model.
The desktop regression suite checks Cu/Ni edge identification, metadata priority
and invalid-energy handling. The crate is MIT OR Apache-2.0 licensed and is covered
by the release dependency inventory and license gate.

## Spline and license changes

`rusty-fitpack` was removed. Spline coefficients now use a direct QR solve with
the existing nalgebra dependency, with local evaluation and basis derivatives.
No replacement spline crate is needed. AUTOBK's direct optimizer is unchanged.
The Apache-licensed GPUI sum_tree component is patched to use tracing directly,
removing its GPL tracing wrappers. Details and the enforced non-GPL license policy
are in the [distribution notices](distribution-notices.md).

## Verification policy

Run core tests, the simple API tests, clippy, documentation builds, optional
feature checks, Python wheel installation, Wasm runtime tests and desktop builds.
Version resolution alone is not evidence of a working release. Record final
results and remaining platform qualification in [the release runbook](releasing.md).

## FEFF10 array-header correction — 19 September 2026

The 0.2.11 release candidate selects
[FEFF10 0.2.4](https://github.com/Ameyanagi/feff10-rs/releases/tag/v0.2.4).
Its native Fortran build initializes the format label and selector in all five
numeric-array writers. This corrects intermittent `gg.bin` parsing failures;
rebuilding the Rust wrapper with a 0.2.3 archive does not repair that archive.
The Windows helper is pinned to the matching 0.2.4 executable and verified
runtime DLLs. The original September 7 dependency selection is retained below.

All six upstream native builds and clean-runner smoke tests passed. All 18
downloaded release assets matched their manifest and GitHub digests. Rexafs's
package check also requires valid generated array headers. See the
[qualification record](validation/2026-09-19-release-0.2.11/review.md) for
downstream checks and publication status.

## Embedded FEFF engines — 7 September 2026

The September 7 dependency update selected [ReFEFF 0.3.0](https://github.com/Ameyanagi/refeff/releases/tag/v0.3.0)
(component crates 0.2.0) and [feff10 0.2.3](https://github.com/Ameyanagi/feff10-rs/releases/tag/v0.2.3),
verified against crates.io and upstream releases. Both use workspace dependency
requirements, and Cargo.lock records the exact registry artifacts. FEFF10 is the
existing Fortran-backed Rust wrapper shown as “FEFF-RS / FEFF10” in the GUI.

Published Mac Stable and Nightly apps include both engines. Select **Fit → Calculate → engine**
to run either on the same structure/input; ReFEFF is the default. Every calculation
gets a separate workspace and adds a source, whose label includes its engine.
The saved project retains each source's input and engine marker. Calculating a
second source preserves existing path edits; enable only the intended source's
paths when comparing fits, since enabling both adds both sets to the model.

The extracted package and copied installer app run `--self-check-feff`, checking
both engines against the analytical first-shell geometry of fcc Cu and finite,
nonzero amplitudes. FEFF10 is forced into worker mode in this check, exercising
re-execution before app initialization. The desktop regression suite additionally
compares Cu/Ni foil fits from identical inputs with both engines.

The published 0.2.4 Windows preview includes ReFEFF only: its GUI uses MSVC,
while the upstream FEFF10 archive targets MinGW. The current checkout runs
FEFF10 through a separate helper process; see the
[Windows development guide](desktop-development.md#windows). Windows/Linux preview publication began
with 0.1.3; the [release runbook](releasing.md#published-releases) links the
qualification records and installation guidance. Rust consumers can opt
into either backend; Python and Wasm packages keep their existing analysis APIs.

For a single-engine comparison, select that source in **Paths**, choose the desired
preset (for example **First shell**), then click **Deselect other sources**. This
retains the other calculations and their parameter edits for later comparison.

PR #39's isolated nalgebra 0.35 update is incompatible with the published
`levenberg-marquardt 0.15.0` (its manifest requires nalgebra 0.34). Dependabot
excludes only the 0.35 line until a compatible solver release can be upgraded
with it; 0.34 patch updates remain enabled. Upstream master has migrated to
0.35, but that unpublished change cannot be used by a registry release of rexafs.
