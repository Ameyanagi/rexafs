---
title: "WebAssembly support"
description: "What runs in a browser, and which capabilities require the native packages."
audience: user
---

[Open the browser workspace](/app/) to import a spectrum, adjust processing
settings, inspect plots and export results. This preview uses the source-checkout
engine; it does not change the published 0.2.4 packages. Files stay in the browser.

The TypeScript package already runs the Rust spectrum-processing engine in
WebAssembly (WASM). Install `rexafs`; the package includes the compiled module.

| Capability | Browser package, 0.2.4 |
|---|---|
| Normalization, AUTOBK, forward and inverse transforms | Available |
| NumPy/Python interface | Separate native Python package |
| Groups, LCF/PCA, structures and path fitting | Not exposed in TypeScript |
| ReFEFF or FEFF10 calculations | Not available |
| Desktop interface | Native application |

Follow the [browser quick start](/docs/libraries/typescript/#browser-initialization) for
initialization, input arrays and memory cleanup. Calculations are synchronous;
use a Web Worker for long processing so the page remains responsive. Load input
through the browser and pass arrays to `Spectrum`.

## Can ReFEFF run in WASM?

ReFEFF WASM support is being handled upstream. The current ReFEFF dependency fails a browser-target
compile check, and its `run_in_memory` implementation uses temporary files.
Rust's [browser target](https://doc.rust-lang.org/rustc/platform-support/wasm32-unknown-unknown.html)
does not provide native filesystem or thread operations. See the repository's
[build results and porting plan](https://github.com/Ameyanagi/rexafs/blob/main/doc/webassembly.md).

Use [Rust](/docs/libraries/rust/) or the [desktop](/docs/desktop/fitting/)
for scattering calculations and fitting today.
