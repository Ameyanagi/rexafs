---
title: "WebAssembly support"
description: "What runs in a browser, and which capabilities require the native packages."
audience: user
---

[Process a spectrum](/app/) or [calculate scattering with ReFEFF](/app/scattering/).
Both previews run locally in the browser. The processing engine follows the
website's source checkout; scattering uses the published ReFEFF 0.4.0 engine.

The TypeScript package already runs the Rust spectrum-processing engine in
WebAssembly (WASM). Install `rexafs`; the package includes the compiled module.

| Capability | Browser package, 0.2.9 |
|---|---|
| Normalization, AUTOBK, forward and inverse transforms | Available |
| Beamline text, HDF5, Athena, Larix and XTUNES import | Shared reader; explicit detector/dataset selection where needed |
| NumPy/Python interface | Separate native Python package |
| Groups, LCF/PCA, structures and path fitting | Not exposed in TypeScript |
| ReFEFF calculations | Separate browser workspace; not part of the npm API |
| FEFF10 calculations | Native packages only |
| Desktop interface | Native application |

Follow the [browser quick start](/docs/libraries/typescript/#browser-initialization) for
initialization, input arrays and memory cleanup. Calculations are synchronous;
use a Web Worker for long processing so the page remains responsive. Load input
through the browser and pass arrays to `Spectrum`.

## Can ReFEFF run in WASM?

Yes. [ReFEFF 0.4.0](https://github.com/Ameyanagi/refeff/releases/tag/v0.4.0)
provides a browser Worker with an in-memory filesystem through WASI, the
WebAssembly System Interface. It runs on one calculation thread. Its compiled
engine loads separately when you start a scattering calculation.

1. Open the [scattering workspace](/app/scattering/) and load the ZnSe test input
   or your `feff.inp`.
2. Check the input cards, then choose **Run calculation**. Progress appears
   while the page stays responsive; **Cancel** stops the Worker.
3. Inspect the calculated $\chi(k)$ and download the generated files and
   provenance record. Editing the input requires a new calculation.

The plot shows dimensionless EXAFS $\chi$ against photoelectron wave number
$k$ in Å⁻¹. Generated FEFF files retain their original columns and headers.
Record the input and engine version when using calculated paths in a fit.

The unchanged upstream test input includes a krypton (Kr) scatterer; it is not
a pure ZnSe material model. Its EXAFS outputs have native/browser comparison
evidence; this does not qualify every FEFF workflow. Inputs, intermediate files and
outputs share the browser's memory, so large calculations may exceed its limits.
The [ReFEFF guide](https://github.com/Ameyanagi/refeff/blob/v0.4.0/wasm/README.md)
describes the adapter and upstream validation. See [licenses and example
provenance](/licenses/) before redistributing its assets.

ReFEFF's WASI adapter is separate from rexafs's `wasm-bindgen` processing module.
Native rexafs 0.2.9 still uses ReFEFF 0.3.0. Structural fitting, the full desktop
interface and the native Python extension remain outside these browser previews.
Use [Rust](/docs/libraries/rust/) or the [desktop](/docs/desktop/fitting/) for
those workflows. The [build assessment](https://github.com/Ameyanagi/rexafs/blob/main/doc/webassembly.md)
retains the earlier 0.3.0 compile probes and current integration details.
