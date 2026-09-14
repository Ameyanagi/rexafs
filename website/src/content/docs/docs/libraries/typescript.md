---
title: "TypeScript and JavaScript"
description: "Use rexafs in Node and the browser with typed arrays."
audience: user
---

## Install

Install with [Bun](https://bun.sh/docs/installation):

```sh
bun add rexafs@0.2.6
```

The package runs in Node 22+ and browsers with WebAssembly support. You can also
install it with `npm install rexafs@0.2.6` or `pnpm add rexafs@0.2.6`.
Use ECMAScript modules and TypeScript `moduleResolution: "NodeNext"` for Node or
`"Bundler"` for browser bundlers. Typed declarations are included; see the
[stable API reference](/docs/reference/stable/typescript/spectrum/).

The package exposes single-spectrum processing. See
[WebAssembly support](/docs/libraries/webassembly/) for other APIs and ReFEFF status.

## Node: load and transform a spectrum

Download [cu_150k.xmu](/examples/cu_150k.xmu). This example reads its first two
columns, energy in eV and absorption:

```javascript
import { readFileSync } from "node:fs";
import { Spectrum } from "rexafs/node";

const rows = readFileSync("cu_150k.xmu", "utf8")
  .split(/\r?\n/)
  .filter((line) => line.trim() && !line.trim().startsWith("#"))
  .map((line) => line.trim().split(/\s+/).map(Number));
const energy = Float64Array.from(rows, (row) => row[0]);
const mu = Float64Array.from(rows, (row) => row[1]);
const spectrum = Spectrum.from_arrays(energy, mu);
try {
  spectrum.fft();
  console.log(spectrum.e0(), spectrum.r(), spectrum.chir_mag());
} finally {
  spectrum.free();
}
```

Node initializes the packaged Wasm synchronously.

## Read measurement files

The shared reader accepts text or bytes, including a Node `Buffer`:

```typescript
import { readFileSync } from "node:fs";
import { read_measurement } from "rexafs/node";

const measurement = read_measurement(readFileSync("measurement.dat"));
try {
  console.log(measurement.document.scans[0].signals);
  const { energy, mu } = measurement.arrays();
  console.log(energy, mu);
} finally {
  measurement.free();
}
```

Automatic conversion requires exactly one detected signal. Select ambiguous
columns by name or index, for example
`measurement.arrays({energy: "energy", i0: "i0", it: "it"})`. Returned arrays
are independent copies and retain acquisition order. The reader does not run
processing or detector corrections. See [reading measurements](/docs/reference/stable/measurement-reading/)
for angles, HDF5, Athena, Larix, XTUNES and explicit signal selection.

## Browser initialization

Initialize Wasm before constructing browser objects:

```typescript
import init, { Spectrum } from "rexafs/browser";
await init();
// Supply your measured energy and absorption as Float64Arrays.
const spectrum = Spectrum.from_arrays(energy, mu);
try {
  spectrum.fft();
  const magnitude = spectrum.chir_mag();
} finally {
  spectrum.free();
}
```

Serve the packaged `.wasm` asset with your bundler. A failed initialization or
malformed input throws an error. Array getters return independent copies, or
`undefined` before the corresponding stage runs.

<span id="configure-stable-024"></span>

## Configure processing

Use options constructors and pass settings directly. Apply this configuration
before freeing the spectrum in the Node example above:

```typescript
import { AUTOBK, XrayFFTF, XrayFFTR } from "rexafs/node";
const background = new AUTOBK({ rbkg: 1.0 });
const transform = new XrayFFTF({ kweight: 2 });
const inverse = new XrayFFTR({ rmin: 1.0, rmax: 3.0 });
try {
  spectrum.set_background_method(background).set_fft(transform).set_ifft(inverse).ifft();
  console.log(spectrum.q(), spectrum.chiq());
} finally {
  background.free();
  transform.free();
  inverse.free();
}
```

`ifft()` calculates missing forward stages and filters R=1–3 Å in this example.
With forward `kweight=2` and default inverse `rweight=0`, `chiq()` has units Å⁻²;
it retains the forward weighting and window. Choose the R interval from your
data; it is not a phase-corrected bond-distance range.

Settings are copied into the spectrum. Release settings and the spectrum when
finished; never use an object after `free()`.

The defaults match the [Python guide](/docs/libraries/python/#recommended-defaults)
for the main processing path. Legacy trust-region optimization is unavailable in
Wasm; use the recommended fixed-penalty/direct-solver combination.

See [processing theory](/docs/science/processing/) for weighting, units and citations.

## Read results and editor help

The [stable reference](/docs/reference/stable/typescript/spectrum/) and installed
editor help document field units, defaults, automatic values and errors.

`chi()` returns the unweighted, dimensionless background residual on `k()`.
The `chir_real()`, `chir_imag()` and `chir_mag()` getters return the transform on
`r()`, with units Å⁻⁽ʷ⁺¹⁾ for forward k weight `w`. The code multiplies an
unnormalized forward discrete Fourier transform by `kstep / Math.sqrt(Math.PI)`;
it adds no `1 / nfft` or window-area correction. The
[Fourier explanation](/docs/science/processing/#4-transform-from-k-to-r) defines
the sign, scaling and assumptions.

To plot the forward window, pair `kwin()` with `kwin_k()`. The `"Larch"` grid can
resample and extend the window domain while the background `k()` and `chi()` stay
unchanged. Getter calls do not run processing, and their copied arrays remain
valid after the spectrum is freed. After changing a stage's settings, run that
stage again before reading its results.

## Automatic settings when reusing a spectrum

Processing resolves automatic values on the spectrum's copied settings, leaving
your original settings object unchanged. Resolved values such as FFT `kstep`
persist when results are invalidated. AUTOBK's automatic `kmax` and `nknots`
remain unset and are recalculated for each input.

For example, after changing the background `kstep`, assign forward settings with
`kstep = undefined` using `set_fft()` before calling `fft()`. This requests new
spacing inference for the changed background grid. If an inverse transform has
already resolved its spacing, reassign inverse settings with automatic `kstep`
through `set_ifft()` after changing the forward R spacing or inverse FFT length.
Reapply the desired window and range settings to each new configuration.

Calling `set_normalization_method()` or
`set_background_method()` with no argument, `undefined` or `null` restores that
stage's default settings; the selected E0 is retained. Custom settings accept
`PrePostEdge` and `AUTOBK` directly. The older method wrappers and field assignments
remain supported.

The [Spectrum reference](/docs/reference/stable/typescript/spectrum/) documents
which stages each operation invalidates.
