---
title: "TypeScript and JavaScript"
description: "Use rexafs in Node and the browser with typed arrays."
audience: user
---

## Install

Use Node 22 or newer:

```sh
npm install rexafs@0.2.4
```

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
