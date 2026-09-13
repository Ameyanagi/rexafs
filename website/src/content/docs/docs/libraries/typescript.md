---
title: "TypeScript and JavaScript"
description: "Use rexafs in Node and the browser with typed arrays."
audience: user
---

## Install

Install with [Bun](https://bun.sh/docs/installation):

```sh
bun add rexafs@0.2.4
```

The package runs in Node 22+ and browsers with WebAssembly support. You can also
install it with `npm install rexafs@0.2.4` or `pnpm add rexafs@0.2.4`.
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

## Configure stable 0.2.4

In this release, assign fields after construction and use the background wrapper:

```typescript
import { AUTOBK, BackgroundMethod, XrayFFTF } from "rexafs/node";
const background = new AUTOBK();
background.rbkg = 1.0;
const method = BackgroundMethod.AUTOBK(background);
const transform = new XrayFFTF();
transform.kweight = 2;
try {
  spectrum.set_background_method(method).set_fft(transform).fft();
} finally {
  method.free();
  background.free();
  transform.free();
}
```

Settings are copied into the spectrum. Release settings and the spectrum when
finished; never use an object after `free()`. Apply this configuration before
freeing the spectrum in the complete example above.

The defaults match the [Python guide](/docs/libraries/python/#recommended-defaults)
for the main processing path. Legacy trust-region optimization is unavailable in
Wasm; use the recommended fixed-penalty/direct-solver combination. The option-object
constructors and inverse settings in [Next API](/docs/reference/) are not in npm 0.2.4.

See [processing theory](/docs/science/processing/) for weighting, units and citations.

## Read results and editor help

The [reference](/docs/reference/) documents field units, defaults and automatic
values. Stable pages keep 0.2.4 signatures; expanded installed editor help belongs
to Next.

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
already resolved its spacing, Next also requires reassigning inverse settings with
automatic `kstep` through `set_ifft()`. Stable 0.2.4 does not expose inverse settings;
use a fresh spectrum when changing those grids after a back-transform.

Calling `set_normalization_method()` or
`set_background_method()` with no argument, `undefined` or `null` restores that
stage's default settings in both stable 0.2.4 and Next; the selected E0 is retained.
For custom settings, stable 0.2.4 uses algorithm wrappers as shown above, while Next
also accepts `PrePostEdge` and `AUTOBK` settings objects directly.

The source JSDoc for [Spectrum](/docs/reference/next/typescript/spectrum/) documents
these version differences and which stages each operation invalidates.
