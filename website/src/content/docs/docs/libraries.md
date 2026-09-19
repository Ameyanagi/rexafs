---
title: "Choose a library"
description: "Use the same spectrum workflow in Python, TypeScript and Rust."
audience: user
---

[Try the browser workspace](/app/) for local processing without installation.

All three libraries use the Rust numerical engine. These guides target
**published 0.2.11**.

| Library | Best for | Install |
|---|---|---|
| [Python](/docs/libraries/python/) | NumPy, Jupyter and analysis scripts | `uv add rexafs==0.2.11` |
| [TypeScript / JavaScript](/docs/libraries/typescript/) | Node applications and browser tools | `bun add rexafs@0.2.11` |
| [Rust](/docs/libraries/rust/) | Native applications, parallel groups, structures and fitting | `cargo add rexafs@0.2.11` |

The commands assume an existing project. Follow the language guide for project
setup, editor configuration and a measured Cu example.

## Available operations

Python and TypeScript expose single-spectrum normalization, AUTOBK background
removal, Fourier and Cauchy wavelet transforms, MBACK, fluorescence correction,
scalar measurements and XANES peak fits. Rust also exposes collections, data treatment,
linear combination fitting, principal component analysis and structural fitting;
see the
[feature map](/docs/getting-started/#features-by-interface).
The TypeScript package already uses WebAssembly. See
[WebAssembly support](/docs/libraries/webassembly/) for its scope and ReFEFF status.

## Shared workflow

Start with energy/absorption arrays and a mutable `Spectrum`. Request only
the stage you need: `normalize()`, `calc_background()`, `fft()` or `ifft()`.
Missing prerequisites run automatically.

Python returns NumPy copies; TypeScript returns `Float64Array` copies. Release
TypeScript/Wasm objects with `free()` when finished. Rust ownership depends on the
getter's return type. Supply energy in eV and absorption μ; file readers have
format-specific validation rules.

The [spectrum workflow](/docs/libraries/spectrum-api/) explains units, errors and
settings. The [API reference](/docs/reference/) separates the published package
from the source checkout.
