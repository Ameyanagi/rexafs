---
title: "Start with a spectrum"
description: "Choose desktop analysis or a library, and explore what rexafs can do."
audience: user
---

rexafs analyzes X-ray absorption spectra with a shared Rust calculation engine.
Use the desktop for interactive analysis or the Python, TypeScript and Rust
libraries for scripts and applications. This manual targets **stable 0.2.4**.

| Desktop | Libraries |
|---|---|
| [Install the application](/docs/getting-started/install/) | [Choose Python, TypeScript or Rust](/docs/libraries/) |
| [Follow your first analysis](/docs/getting-started/first-analysis/) | [Run the spectrum pipeline](/docs/libraries/spectrum-api/) |
| [Build a structural fit](/docs/desktop/fitting/) | [Read the generated API reference](/docs/reference/) |

## Features by interface

“Rust” includes optional features where indicated. The smaller bindings expose
spectrum processing; additional Rust functionality is not automatically available
in Python or TypeScript.

| Feature | Desktop | Python | TypeScript / JS | Rust |
|---|:---:|:---:|:---:|:---:|
| Energy/absorption arrays, normalization, AUTOBK, FFT and IFFT | Yes | Yes | Yes | Yes |
| Normalization/background/forward-FFT settings | Yes | Yes | Yes | Yes |
| Custom inverse-transform settings | Yes | Next API | Next API | Yes |
| Text mapping, XDI and multiple imported channels | Yes | QAS reader / NumPy | Supply arrays | Readers |
| Groups, marks, processing overrides and comparisons | Yes | Loop over spectra | Loop over spectra | Group API |
| Alignment, calibration, deglitching, truncation, rebinning, smoothing, merging and differences | Yes | — | — | Yes |
| Linear combination fits and principal components | Yes | — | — | Yes |
| CIF/XYZ structures, built-in structures and online structure sources | Yes | — | — | Yes |
| FEFF path calculation and structural fitting | Yes | — | — | Optional engines |
| Simultaneous fits, shared/local parameters and independent batches | Yes | — | — | Yes |
| Portable projects, backups and fit history | Yes | — | — | Desktop-specific format |
| Publication figures, captions, data and methods exports | Yes | Use plotting tools | Use plotting tools | Optional plotting |
| Optional analysis assistant | Yes | — | — | — |

Next API means documented source additions that have **not** been published in
0.2.4. Stable examples below use the released interfaces. See the
[API version guide](/docs/reference/) before using a next-version signature.

## Know your data

Photon energy is in **eV**, wave number $k$ and back-transform coordinate $q$ are
in **Å⁻¹**, and Fourier coordinate $R$ is in **Å**. Supply finite arrays with a
strictly increasing energy axis. Construct absorption from your detector signals
before passing arrays to a library.

The [processing guide](/docs/science/processing/) explains the equations and units.
A Fourier peak is not automatically a phase-corrected bond distance. A converged
fit still needs inspection of its residuals, parameter correlations and model.
