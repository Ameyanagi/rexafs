---
title: "Python and Jupyter"
description: "Install rexafs and process real Cu data with NumPy."
audience: user
---

## Install

We recommend [uv](https://docs.astral.sh/uv/getting-started/installation/) to
install rexafs in an isolated environment. CPython 3.10–3.14 is supported;
this example uses Python 3.12:

```sh
uv venv --python 3.12
uv pip install rexafs==0.2.4
# macOS/Linux:
source .venv/bin/activate
# Windows PowerShell: .venv\Scripts\Activate.ps1
python -c "import rexafs; print(rexafs.__version__)"
```

NumPy is installed as a dependency. See [uv's environment guide](https://docs.astral.sh/uv/pip/environments/)
for environment selection. An existing pip workflow also works: activate its
environment and run `python -m pip install rexafs==0.2.4`.

In VS Code, select this environment with **Python: Select Interpreter**. The
package includes `py.typed` and type stubs. The
[stable reference](/docs/reference/stable/python/spectrum/) preserves the released
signatures and adds explanations reviewed against the implementation. Improved
installed hover help and keyword constructors appear in the
[Next API](/docs/reference/next/python/spectrum/) until a new package release.
Updating the website does not replace the stubs in an existing installation.

For Jupyter, install and register a kernel from the same activated environment:

```sh
uv pip install ipykernel
python -m ipykernel install --user --name rexafs --display-name "Python (rexafs)"
```

Select **Python (rexafs)** in your notebook's kernel menu. The
[IPython kernel guide](https://ipython.readthedocs.io/en/stable/install/kernel_install.html)
explains how to register separate environments.

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
independent NumPy float64 copies. Before a stage runs, or after invalidation,
an unavailable output is `None`. Input and normalization errors raise
`ValueError`; background and Fourier failures raise `RuntimeError`. The default
Fourier magnitude has units Å⁻³ because `kweight=2` and the amplitude factor is
`kstep / sqrt(pi)`, with no additional FFT-length normalization.

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
the natural logarithm of the intensity ratio. Intensities should be positive
and in matching units. The reader ignores extra columns, treats `#` as a comment
and returns an unprocessed spectrum. It does not check positive intensities or
sort energies; invalid derived data are rejected when processing runs. Do not
use that reader on a file that already contains μ, such as this Cu example.

## Recommended defaults

AUTOBK starts with `rbkg=1.0` Å, a fixed endpoint penalty of `0.001`, the direct
solver and background `kweight=1`. The forward transform starts with k=2–15 Å⁻¹,
`kweight=2`, a Kaiser–Bessel window and 2048 FFT points. Automatic normalization
ranges adapt to the data. The FFT's automatic `kstep` uses the background grid,
whose default is 0.05 Å⁻¹. Increasing the FFT length adds zero padding and refines
the displayed R spacing without adding experimental resolution. These are
starting points; inspect the windows and noise, and remember that uncorrected
R peaks are not directly bond distances.

[Processing theory](/docs/science/processing/) explains the equations and references.
For repeated spectra, loop over `Spectrum` objects; the Python package does not
currently expose Rust's Group, LCF/PCA or structural fitting APIs.
