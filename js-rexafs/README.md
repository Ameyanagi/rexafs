# rexafs for JavaScript and TypeScript

[Published user guide](https://rexafs.com/docs/libraries/typescript/) ·
[Versioned API reference](https://rexafs.com/docs/reference/)

Process X-ray absorption spectra in Node or a browser using the Rust engine
compiled to WebAssembly. TypeScript declarations are included.

## Install

We recommend [Bun](https://bun.sh/docs/installation) to manage the dependency:

```bash
bun add rexafs
```

Run the library in Node **22+**, or a browser with WebAssembly support. Bun can
install the package for either runtime; `npm install rexafs` and
`pnpm add rexafs` are also available. The
[npm package](https://www.npmjs.com/package/rexafs) includes the Wasm binaries;
Rust is not required to use it. Imports are ECMAScript modules.

**Version note:** options constructors, direct configuration arguments and
`XrayFFTR` were added in 0.2.5. The basic pipeline also works in 0.2.4.

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
const spectrum = new Spectrum(energy, mu);
try {
  spectrum.fft();
  const r = spectrum.r();
  const magnitude = spectrum.chir_mag();
  // Use the copied arrays here, or retain them outside this block.
} finally {
  spectrum.free(); // Copied arrays remain valid, including if processing failed.
}
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
Automatic values are resolved on the spectrum's copied settings. The original
settings object still reports the values you assigned, so its unset fields do
not become a record of the resolved calculation parameters.
Use `new PrePostEdge({ ... })` with `set_normalization_method()` for custom
normalization and `set_e0(eV)` to override the edge energy.

In 0.2.4 and later, calling `set_normalization_method()` or
`set_background_method()` without an argument restores that stage's recommended
defaults. `null` and `undefined` have the same effect. For custom settings,
0.2.4 accepts algorithm wrappers; 0.2.5 also accepts `PrePostEdge` and `AUTOBK`
settings directly.

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

Since 0.2.5, the package includes explanatory JSDoc for every exported
configuration field, constructor and spectrum method. Version 0.2.4 has shorter
editor help; the online stable reference preserves the published signatures.
Options interfaces (`AUTOBKOptions`, `XrayFFTFOptions`,
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

The forward transform multiplies the unnormalized, negative-exponent discrete
Fourier transform by `kstep / Math.sqrt(Math.PI)`. It applies no additional
`1 / nfft` or window-area correction. With dimensionless chi and k weight `w`,
`chir_real()`, `chir_imag()` and `chir_mag()` have units Å⁻⁽ʷ⁺¹⁾. Use
`kwin_k()` with `kwin()` because the window grid can differ from the background
grid when `grid` is `"Larch"`. The [processing theory](../doc/processing-theory.md)
explains this convention and its implementation sources.

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
# npm install /path/to/rexafs/js-rexafs/rexafs-0.2.5.tgz
```

The package includes Node/browser Wasm and declarations. Filesystem reading,
fitting, groups, structure downloads and FEFF execution remain Rust/desktop
APIs. ILPBkg remains unimplemented. The unreleased `MBack` API below adds full
atomic-reference normalization. TrustRegionDogLeg requires
a native Rust feature absent from Wasm. The recommended LinearDirect/FixedPenalty
path works in both bindings. See [FFT compatibility](../doc/fft-grid-compatibility.md).
Licensed under MIT OR Apache-2.0.

## Universal measurement reader (since 0.2.6)

Version 0.2.6 adds content-detected beamline text, CSV, Athena, XTUNES
and HDF5 import through the shared Rust reader. Read a document, inspect its
scans, columns, units and warnings, then select a signal mapping. Ambiguous
channels require explicit selection; detector images require reduction before
creating an absorption spectrum. Reading performs no processing or corrections.
See the [reader guide](../doc/measurement-reader.md) for language examples,
unit conversions, dataset selection, fixture coverage and limits. This API is
not included in the published 0.2.5 packages.

Larix 1.0 session import is available in the shared reader since 0.2.6, including
stored absorption, complex saved arrays and inert session metadata. See the
[Larix import guide](../doc/larix-import.md).

Column selections accept exact, case-sensitive names as well as zero-based indices.
For QAS data, use `measurement.arrays({energy: "energy", i0: "i0", it: "it"})`
for transmission, `iff: "iff"` instead of `it` for fluorescence, or
`i0: "it", it: "ir"` for the reference. Existing positional mappings remain valid.
Duplicate names require indices. Omit `energy_unit` to preserve detected axis
calibration, or explicitly override it with `"eV"` or `"keV"`.

## Development-only scalar measurements

The source checkout adds this call (newer than 0.2.9):

```typescript
const result = spectrum.measure("mean", [-20, 30], { space: "flat" });
console.log(result.value, result.unit);
```

The default is normalized absorption with E₀-relative energy bounds in eV.
Missing stages run on a private copy; existing arrays/settings remain unchanged.
Choose `origin: "absolute"` for absolute energy, or `space: "chi"`/`"fourier"`
for absolute k/R. Point, maximum, integral and mean share strict native-grid
coverage checks. See the [measurement guide](../doc/full-frame-measurements.md)
for units, optional independent errors and numerical assumptions. This scalar
operation is separate from the `Measurement` input-file reader. Type declarations
include options, result fields, completion choices and hover help.

## Full MBACK normalization (unreleased)

```ts
import { MBack } from "rexafs/node";
const model = new MBack("Cu", "K", {pre_edge: [-200,-50], post_edge: [100,800]});
const result = model.fit(energy, mu); // Float64Array inputs: eV and raw absorption
spectrum.set_normalization_method(model).normalize();
console.log(result.norm, result.flat);
model.free();
```

Ranges are E₀-relative eV. Degree 2 and no erfc are the defaults. Offline atomic
data load automatically. Results own their arrays; spectrum settings copy the
model. `spectrum.mback_result()` retrieves its saved result or `undefined` after
invalidation. `result.definition` creates a model pinned to the original reference;
free that model after use. Browser callers must await `init()` and should use a
Worker for large synchronous fits. See the [MBACK guide](../doc/mback-normalization.md)
for equations, assumptions and optional backgrounds. Atomic-data notices ship
in the npm package's `licenses/atomic` directory.

## Cauchy wavelets (unreleased)

```ts
import { Wavelet } from "rexafs/node";
const model = new Wavelet([2, 12]);
const map = spectrum.wavelet(model);
try {
  const image = map.magnitude; // Independent Float64Array: R rows, k columns
  const region = map.integral([4, 10], [1, 3]);
  console.log(image, region.value, region.unit);
} finally {
  map.free();
  model.free();
}
```

The k interval uses Å⁻¹; R uses Å and is not phase-corrected. Missing normalization
and background stages run on a private copy. Defaults are weight 2, order 100,
k spacing 0.05 Å⁻¹, R up to 6 Å and no taper. Named options override them, for
example `new Wavelet([2, 12], {rmax: 4})`. `model.calculate(k, chi)` also accepts
unweighted χ(k) in `Float64Array` inputs. Browser callers await `init()` first.
See the [wavelet guide](../doc/wavelet-analysis.md) for layout, native-grid integrals,
slices, ownership, JSON replay and scientific limitations.
