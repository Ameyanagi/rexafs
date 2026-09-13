# WebAssembly build assessment

**Implementation update:** `/app/` provides local spectrum processing, plots and
CSV/provenance export. `/app/scattering/` runs the published ReFEFF 0.4.0 WASI
engine in a separate Worker. Native rexafs 0.2.5 retains ReFEFF 0.3.0; the
browser integration does not change that release's source or dependencies.

## ReFEFF 0.4.0 integration

[ReFEFF 0.4.0](https://github.com/Ameyanagi/refeff/releases/tag/v0.4.0)
adds serial WASI execution and a browser adapter with a virtual filesystem,
progress and cancellation. The website stages its published archive after
checking the pinned SHA-256, retains its licenses and source identity, and loads
the engine only when a scattering calculation starts. The Worker receives
user-selected input bytes; it does not upload calculation files.

The processing engine remains a separate `wasm-bindgen` module. ReFEFF uses
`wasm32-wasip1` and the upstream `browser_wasi_shim` adapter; it is not linked
into the npm processing package. The [upstream guide](https://github.com/Ameyanagi/refeff/blob/v0.4.0/wasm/README.md)
documents the runtime and native/browser ZnSe comparison. Browser calculations
use one thread and memory-backed intermediate files. Compiling the full CLI
does not qualify every FEFF workflow in a browser.

The [browser qualification record](validation/2026-09-13-browser-refeff/review.md)
retains the engine and native-reference identities, comparison of all 4,010
numeric values in the ZnSe test outputs, and interface checks at both deployment
paths. The unchanged historical input contains a Kr scatterer; this is a
software-agreement case, not a validated model of pristine ZnSe.

The earlier probes below apply to the recorded 0.3.0 dependency. They explain
why changing only rexafs's compilation target was insufficient; they do not
describe the newer WASI release.

## Historical assessment

Assessed on 13 September 2026 from rexafs commit
`0e3572f5a7931244214b9f92bbd0046cc6f5f504`, with Rust 1.98.1 and the committed
lockfile. This developer record distinguishes a successful compile from a
tested JavaScript API. The [public support guide](../website/src/content/docs/docs/libraries/webassembly.md)
describes what users can run now.

<a id="build-results"></a>

## Historical build results

| Build | Result | What it establishes |
|---|---|---|
| `rexafs-wasm`, release, browser and Node | Passed | The existing processing bindings produce usable WASM modules. |
| Core, `--no-default-features`, `wasm32-unknown-unknown` | Passed | Core modules type-check for the target; this does not expose every operation to JavaScript or test native I/O paths. |
| Core with default `trust-region` feature | Failed | Native dependencies require further target configuration or separation; this checkout failed in `ring`, `zstd-sys` and `getrandom`. |
| Core, `--no-default-features --features refeff-runner` | Failed | `atomic-wait 1.1.0` has no selected platform implementation for this target. |

Reproduce from the repository root:

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-pack --locked --version 0.15.0
npm --prefix js-rexafs ci
npm --prefix js-rexafs run build
npm --prefix js-rexafs test

cargo check --locked -p rexafs --no-default-features --target wasm32-unknown-unknown

# Historical diagnostic probes with the recorded lockfile, not supported recipes.
cargo check --locked -p rexafs --target wasm32-unknown-unknown
cargo check --locked -p rexafs --no-default-features --features refeff-runner --target wasm32-unknown-unknown
```

The package build writes `js-rexafs/dist/web/` and `js-rexafs/dist/node/`.
All 13 Node runtime/editor checks passed, including loading the browser glue
from bytes and comparisons with retained native processing fixtures. These
checks cover the exported processing pipeline, not a ReFEFF simulation.
The separate Chromium check also loaded the packaged WASM over HTTP and matched
the Ru fixture (`E0 = 22118.8` eV, 315 k samples, 326 R samples).
The default-feature C compilation failures were observed with this Mac's
compiler; they do not prove those libraries are fundamentally unportable.

<a id="why-refeff-needs-more-work"></a>

## ReFEFF 0.3.0 barriers

The first observed compile failure follows this dependency chain:

```text
refeff 0.3.0 → refeff-linalg 0.2.0 → faer 0.24.4
            → spindle 0.2.6 → atomic-wait 1.1.0
```

`refeff-linalg` enables faer's default features. Disabling rexafs's own default
features cannot disable features requested by another dependency. Inspect the
resolved chain with:

```sh
cargo tree --locked -p rexafs --no-default-features --features refeff-runner --target wasm32-unknown-unknown -i atomic-wait
```

Compilation is only the first obstacle. In the source shipped with ReFEFF 0.3.0:

- [`Runner::run_in_memory_inner`](https://github.com/Ameyanagi/refeff/blob/6e260d7ea4d9869f6456c48ed06815cd0c240213/crates/refeff/src/lib.rs#L559)
  creates a temporary directory, writes input artifacts, runs the file pipeline
  and reads output files. “In memory” describes the caller's interface.
- The [engine scheduler](https://github.com/Ameyanagi/refeff/blob/6e260d7ea4d9869f6456c48ed06815cd0c240213/crates/refeff-engine/src/lib.rs)
  uses filesystem handoffs, thread-pool configuration and `Instant` timing.
  Selecting one thread still creates a worker pool. A serial build also needs
  changes to ReFEFF's unconditional `faer::Par::rayon` configuration.
- rexafs's own [runner adapter](../crates/rexafs/src/xafs/fitting/runner.rs)
  reads/writes paths and wraps native execution; it also needs an adapter for
  browser-owned data and progress.

The [`wasm32-unknown-unknown` target](https://doc.rust-lang.org/rustc/platform-support/wasm32-unknown-unknown.html)
does not supply working `std::fs` or native thread creation. JavaScript glue
cannot transparently replace those standard-library operations. A successful
`cargo check` alone would therefore be insufficient.

## Recommended porting sequence

This was the recommendation before ReFEFF 0.4.0. Upstream subsequently chose
WASI with a virtual filesystem and serial scheduling for browser execution;
the processing and scattering modules remain separate. Advanced analysis
bindings and broader numerical qualification are still useful next steps.

1. **Keep the existing processing module small.** Preserve numerical regression
   checks and provide a Worker example before adding long calculations.
2. **Expose advanced analysis independently of engine execution.** Add text/byte
   FEFF path parsing, path evaluation, fitting with supported solvers, LCF/PCA
   and structure operations. Rust already has
   [text CIF/XYZ parsing](../crates/rexafs/src/xafs/structure/mod.rs) and
   [fitting separate from running FEFF](../crates/rexafs/src/xafs/fitting/mod.rs).
   Each exported operation needs browser runtime tests, not only a core compile.
3. **Port ReFEFF's EXAFS pipeline upstream.** Make sequential linear algebra
   selectable, replace file handoffs with owned buffers or an explicit storage
   interface, and supply browser-compatible timing/progress. Test a single-worker
   path before considering shared-memory threading. Keep native file runners.
   An Emscripten prototype can instead retain the file handoffs through its
   virtual filesystem, after fixing the dependency and scheduling barriers.
4. **Qualify the full calculation.** Run identical retained inputs natively and
   in browser WASM; compare path identities, grids, amplitudes, phases and final
   χ(k) under stated tolerances. Measure binary size, memory and runtime. Test
   failures, cancellation and limits before expanding input/module coverage.
5. **Load scattering separately.** Use an optional package/module so basic
   processing users need not download the larger scattering engine.

The original browser workspace implemented the first step with a cancellable
Worker. The corresponding historical upstream plan is the
[embedding roadmap](https://github.com/Ameyanagi/refeff/blob/6e260d7ea4d9869f6456c48ed06815cd0c240213/docs/EMBEDDING_ROADMAP.md).

## Other meanings of “everything”

| Component | Required work |
|---|---|
| FEFF10 | The [dependency](../Cargo.toml) uses native prebuilt Fortran artifacts. Those archives cannot be linked into browser WASM. A source/toolchain port and equivalent numerical qualification are separate work. |
| Python | Current wheels are native PyO3 extensions. A browser Python package needs a separate Pyodide/Emscripten build with matching Python, NumPy and ABI versions; it is not the npm WASM module. See the official [Rust-extension tutorial](https://pyodide-build.readthedocs.io/en/latest/tutorials/rust.html). No rexafs Pyodide wheel was built here. |
| Desktop | The pinned GPUI includes a web backend, but rexafs's [application](../crates/rexafs-gui/Cargo.toml), file dialogs, project storage, update client and assistant process still need browser-specific integrations. The numerical module does not provide a web desktop. |
| Structure services and files | Native HTTP/SQLite and filesystem adapters need browser equivalents. Keep service credentials out of a public client bundle; preserve user-selected input/output and provenance. |

[WASI](https://doc.rust-lang.org/rustc/platform-support/wasm32-wasip1.html)
provides host filesystem interfaces; ordinary `wasip1` does not support native
thread/process spawning. [Emscripten's virtual filesystem](https://emscripten.org/docs/porting/files/file_systems_overview.html)
can preserve legacy file handoffs. Changing targets alone did not resolve the
recorded `atomic-wait` dependency failure, and neither target is a drop-in change
to the processing package. ReFEFF 0.4.0 supplies the required upstream changes
and uses WASI through its separate browser adapter.
