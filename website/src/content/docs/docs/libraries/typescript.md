---
title: "TypeScript and JavaScript"
description: "Use rexafs in Node and the browser with typed arrays."
audience: user
---

## Install

We recommend [Bun](https://bun.sh/docs/installation) to install the library:

```sh
bun add rexafs@0.2.4
```

Run it in Node 22 or newer, or in a browser with WebAssembly support. Bun manages
the dependency for either runtime. If you already use another package manager,
`npm install rexafs@0.2.4` or `pnpm add rexafs@0.2.4` installs the same package.
The package includes TypeScript declarations. Use an ESM project and a TypeScript
module-resolution mode appropriate to your runtime, such as `NodeNext` for Node
or `Bundler` for a browser bundler. See the
[generated reference](/docs/reference/stable/typescript/spectrum/) for the stable API.

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

Node initializes the packaged Wasm synchronously. Browser applications initialize
it explicitly before constructing objects:

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

The generated reference explains each configuration field's purpose, units,
default and effect. It also states which settings are automatic when assigned
`undefined`. These explanations come from the checked declarations; stable pages
retain the 0.2.4 signatures. The installed 0.2.4 package has shorter editor help,
while the source checkout includes the expanded JSDoc used by the site.

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

Automatic values are resolved on the spectrum's copied settings during processing.
The calculation does not write them back into your original settings object.
Stored automatic values, such as FFT kstep, remain in the spectrum when calculated
arrays are invalidated; they are not automatically inferred again for each call.
AUTOBK's automatic kmax and nknots are different: they remain unset in stored
settings and are calculated locally for each input.

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
