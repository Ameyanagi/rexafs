---
title: "Python and Jupyter"
description: "Install rexafs and process real Cu data with NumPy."
audience: user
---

## Install

Use CPython 3.10–3.14 in a virtual environment:

```sh
python -m venv .venv
# macOS/Linux:
source .venv/bin/activate
# Windows PowerShell: .venv\Scripts\Activate.ps1
python -m pip install rexafs==0.2.4 numpy
```

In VS Code, select this environment with **Python: Select Interpreter**. In
Jupyter, select its kernel. The package includes `py.typed` and type stubs.
The [generated reference](/docs/reference/stable/python/spectrum/) contains the
released signatures; [Next API](/docs/reference/next/python/spectrum/) adds richer
editor help and configuration constructors for a future package release.

## Load and transform a measured spectrum

Download [cu_150k.xmu](/examples/cu_150k.xmu) and save it beside your script.
This file already contains energy in eV and absorption in its first two columns.

```python
import numpy as np
from rexafs import Spectrum

energy, mu = np.loadtxt("cu_150k.xmu", comments="#", usecols=(0, 1), unpack=True)
spectrum = Spectrum(energy, mu).fft()
print("E0 (eV):", spectrum.e0())
print("R (angstrom):", spectrum.r())
print("Fourier magnitude:", spectrum.chir_mag())
```

`fft()` calculates missing normalization and background stages. Output arrays are
independent copies. Before a stage runs, an unavailable output is `None`. Invalid
inputs and numerical failures raise exceptions.

## Configure stable 0.2.4

Construct a settings object, assign only the fields you need, and pass it to the
appropriate setter. In 0.2.4, background/normalization settings use their wrappers:

```python
from rexafs import AUTOBK, BackgroundMethod, XrayFFTF

background = AUTOBK()
background.rbkg = 1.0
spectrum.set_background_method(BackgroundMethod.AUTOBK(background))
transform = XrayFFTF()
transform.kweight = 2.0
spectrum.set_fft(transform).fft()
```

Settings are copied. Reassign them after editing to apply the change. In this
release, `.ifft()` uses the available inverse defaults; the `XrayFFTR` constructor
and `.set_ifft()` binding are unreleased additions.

For QAS transmission files with energy, incident intensity and transmitted intensity
in the first three columns, use `rexafs.io.read_qas_transmission(path)`. It computes
the natural logarithm of the intensity ratio. Do not use that reader on a file
that already contains μ, such as this Cu example.

## Recommended defaults

AUTOBK starts with `rbkg=1.0` Å, a fixed endpoint penalty of `0.001`, the direct
solver and background `kweight=1`. The forward transform starts with k=2–15 Å⁻¹,
`kweight=2`, a Kaiser–Bessel window and 2048 FFT points. Automatic normalization
ranges adapt to the data. These are starting points; inspect the windows and noise.

[Processing theory](/docs/science/processing/) explains the equations and references.
For repeated spectra, loop over `Spectrum` objects; the Python package does not
currently expose Rust's Group, LCF/PCA or structural fitting APIs.
