# rexafs for JavaScript and TypeScript

Process X-ray absorption spectra in Node or a browser using the Rust engine
compiled to WebAssembly. TypeScript declarations are included.

## Install

```bash
npm install rexafs
```

Use Node **22+**, or a browser with WebAssembly support. The
[npm package](https://www.npmjs.com/package/rexafs) includes the Wasm binaries;
Rust is not required to use it. Imports are ECMAScript modules.

**Version note:** options constructors, direct configuration setters and
`XrayFFTR` below are additions after 0.2.4. Until released, install a tarball from
this checkout using [Build from source](#build-from-source). The basic pipeline
works in 0.2.4.

## Node quick start

Save as `analyze.mjs` and run `node analyze.mjs`:

```javascript
import { readFileSync } from "node:fs";
import { Spectrum } from "rexafs/node";

// Whitespace-delimited energy (eV) and absorption mu; # starts a comment.
const rows = readFileSync("spectrum.dat", "utf8")
  .split(/\r?\n/).map(line => line.split("#")[0].trim()).filter(Boolean)
  .map(line => line.split(/\s+/).map(Number));
const energy = Float64Array.from(rows, row => row[0]);
const mu = Float64Array.from(rows, row => row[1]);
const spectrum = new Spectrum(energy, mu);
try {
  spectrum.fft();
  console.log(spectrum.e0(), spectrum.r(), spectrum.chir_mag());
} finally {
  spectrum.free();
}
```

Node loads the packaged Wasm automatically. The optional default `init()` export
is a no-op in Node, useful for code shared with browsers.

## Browsers

```typescript
import init, { Spectrum } from "rexafs/browser";
await init(); // Required before constructing spectra or settings.
// energy and mu are Float64Arrays loaded by your application.
const spectrum = new Spectrum(energy, mu).fft();
const r = spectrum.r();
const magnitude = spectrum.chir_mag();
spectrum.free(); // The copied result arrays remain valid.
```

Serve the packaged Wasm asset alongside its glue module. `init(wasmUrl)` or
`init(wasmBytes)` allows an explicit asset location when a bundler relocates it.
The root `rexafs` import selects the runtime through package export conditions;
use `rexafs/browser` or `rexafs/node` when you want to choose explicitly.
Calculations are synchronous after initialization; run long work in a Web Worker.

## Configure only what you need

```typescript
import { AUTOBK, XrayFFTF, XrayFFTR } from "rexafs/node";

const background = new AUTOBK({ rbkg: 1.0 });
const forward = new XrayFFTF({ kmin: 2.0, kmax: 12.0, kweight: 2.0 });
const inverse = new XrayFFTR({ rmin: 1.0, rmax: 3.0, dr: 0.5 });
try {
  spectrum.set_background_method(background).set_fft(forward).fft();
  spectrum.set_ifft(inverse).ifft();
  console.log(spectrum.q(), spectrum.chiq());
} finally {
  background.free(); forward.free(); inverse.free();
}
```

This fragment assumes a live spectrum; the ranges are examples to adapt to your
data. Settings are copied by setters, so they can be freed after assignment.
Changing a settings object later requires assigning it again. Rust-style
`BackgroundMethod.AUTOBK(settings)` and
`NormalizationMethod.PrePostEdge(settings)` remain available. Scalar field
names and stage names match Rust; no additional pipeline object is needed.

| Settings | Recommended starting values |
|---|---|
| `new PrePostEdge()` | Automatic E0, pre/post-edge ranges and polynomial order |
| `new AUTOBK()` | `rbkg: 1`, `kstep: 0.05`, `kweight: 1`, `window: "Hanning"`, `solver: "LinearDirect"`, `clamp_scale_policy: "FixedPenalty"`, `clamp_lambda: 0.001` |
| `new XrayFFTF()` | `kmin: 2`, `kmax: 15`, `kweight: 2`, `dk: 1`, `window: "KaiserBessel"`, `nfft: 2048`, `grid: "Input"` |
| `new XrayFFTR()` | `rmin: 0`, `rmax: 20`, `dr: 1`, `rweight: 0`, `qmax_out: 10`, `nfft: 2048`, automatic `kstep` |

Omitted options retain the constructor defaults. Explicit `undefined` requests
automatic resolution for optional fields; for example `{ kmax: undefined }`
uses the available k range rather than the forward default of 15 Å⁻¹.
Use `new PrePostEdge({ ... })` with `set_normalization_method()` for custom
normalization and `set_e0(eV)` to override the edge energy.

## What the calculations mean

Normalization subtracts a fitted pre-edge baseline and divides absorption by
its edge step. AUTOBK estimates the smooth background to obtain the EXAFS
oscillations, chi(k). The Fourier transform weights and windows those oscillations
to display them against R; its peaks are not automatically phase-corrected bond
lengths. An inverse transform filters selected R contributions back into q space.

The [processing theory guide](../doc/processing-theory.md) explains the
equations, symbols, units, assumptions and implementation choices, with scientific
references. Use the [fitting-statistics guide](../doc/fitting-statistics.md)
when interpreting structural fits and uncertainties.

## Completion and hover help

Every exported configuration field, constructor and spectrum method has typed
signatures and JSDoc help. Options interfaces (`AUTOBKOptions`, `XrayFFTFOptions`,
etc.) and string unions (`FTWindow`, `FFTGrid`, `AUTOBKSolver`,
`AUTOBKClampScalePolicy`) are exported from all entry points. Editors can suggest
field names and valid string choices, show units/defaults on hover and flag typos.
Use TypeScript `moduleResolution: "NodeNext"` for Node or `"Bundler"` for a
bundler project so package export conditions resolve correctly.

Inputs must be finite, equal-length `Float64Array`s with strictly increasing
energy in eV. k/q use Å⁻¹ and R uses Å. Result getters return independent arrays,
or `undefined` before the corresponding stage runs. Narrow that optional result
before indexing it. Stages return the same spectrum and throw on failure.
Changing parameters invalidates dependent stages; see the
[shared API guide](../doc/api.md) for details.

## Build from source

From the repository root with Rust and Node installed:

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-pack --locked
npm --prefix js-rexafs ci
npm --prefix js-rexafs run build
npm --prefix js-rexafs test
cd js-rexafs
npm pack
# From your application directory, install the resulting file:
# npm install /path/to/rexafs/js-rexafs/rexafs-0.2.4.tgz
```

The package includes Node/browser Wasm and declarations. Filesystem reading,
fitting, groups, structure downloads and FEFF execution remain Rust/desktop
APIs. MBack and ILPBkg are unimplemented placeholders; TrustRegionDogLeg requires
a native Rust feature absent from Wasm. The recommended LinearDirect/FixedPenalty
path works in both bindings. See [FFT compatibility](../doc/fft-grid-compatibility.md).
Licensed under MIT OR Apache-2.0.
