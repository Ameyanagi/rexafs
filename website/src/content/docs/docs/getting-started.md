---
title: "Start with a spectrum"
description: "Choose desktop analysis or a library, and explore what rexafs can do."
audience: user
---

rexafs is a free, open-source toolkit for X-ray absorption spectroscopy (XAS).
It takes the spectra you measure at a beamline, normalizes them, extracts the
extended fine structure (EXAFS), transforms it to real space and fits
scattering-path models to recover interatomic distances, coordination numbers
and disorder. A single Rust calculation engine powers the desktop application
and the Python, TypeScript and Rust libraries, so a spectrum processed in a
script matches the desktop when the settings match. This manual targets
**stable 0.2.4**.

## Who this manual is for

- **Analysts and beamline users** who want to inspect, process, fit and
  publish without programming. Start with the desktop column below.
- **Programmers** who want the same processing inside NumPy scripts, Jupyter
  notebooks, browser tools or native applications. Start with the libraries
  column. A programmer using rexafs is a user; this manual documents the public
  API, installation, editor setup and defaults.
- **Readers new to XAS** who want to understand what each step calculates. Read
  [concepts and glossary](/docs/concepts/) and the [science
  overview](/docs/science/) alongside the workflow guides.

## Choose your path

| Desktop | Libraries |
|---|---|
| [Install the application](/docs/getting-started/install/) | [Choose Python, TypeScript or Rust](/docs/libraries/) |
| [Follow your first analysis](/docs/getting-started/first-analysis/) | [Run the spectrum pipeline](/docs/libraries/spectrum-api/) |
| [Build a structural fit](/docs/desktop/fitting/) | [Read the generated API reference](/docs/reference/) |

## A typical analysis

1. **Import** a measured file, confirm which columns hold energy and intensity
   or absorption, and confirm the energy unit. The desktop shows the parsed
   spectrum before you accept it.
2. **Normalize** the absorption so that the edge step is 1, after checking the
   estimated edge energy and the pre-edge and post-edge baselines.
3. **Remove the background** with AUTOBK to obtain the EXAFS oscillations
   χ(k), then **transform** them to χ(R) with a k weight and a window.
4. **Model and fit**: choose a structure, calculate scattering paths with
   ReFEFF or FEFF10, select the paths to include, and fit their parameters
   inside a k and R range. Inspect residuals, uncertainties and correlations.
5. **Save and publish**: keep everything in a portable `.rxs` project and
   export figures, data tables, captions and a methods draft.

The bundled Cu foil spectrum and built-in Cu structure let you complete every
step before importing your own data. [Your first
analysis](/docs/getting-started/first-analysis/) covers steps 1 to 3 and 5;
[fit your first coordination shell](/docs/desktop/fitting/) covers step 4.

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
