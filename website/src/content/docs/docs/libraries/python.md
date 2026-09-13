---
title: "Python and Jupyter"
description: "Install rexafs and process real Cu data with NumPy."
audience: user
---

## Install

We recommend [uv](https://docs.astral.sh/uv/getting-started/installation/) to
manage your analysis as a Python project. CPython 3.10–3.14 is supported;
this example creates a Python 3.12 project:

```sh
uv init --python 3.12 rexafs-analysis
cd rexafs-analysis
uv add rexafs==0.2.4 numpy
uv run python -c "import rexafs; print(rexafs.__version__)"
```

`uv add` records the dependencies in `pyproject.toml`, saves exact resolved
versions in `uv.lock`, and installs them in the project's `.venv`. `uv run`
uses that environment automatically without shell activation. Commit
`pyproject.toml`, `.python-version` and `uv.lock` with your analysis; exclude
`.venv` from version control. NumPy is listed explicitly because the example
imports it. See [uv's project guide](https://docs.astral.sh/uv/guides/projects/).

In VS Code, select the project's `.venv` with **Python: Select Interpreter**. The
package includes `py.typed` and type stubs. The
[stable reference](/docs/reference/stable/python/spectrum/) preserves the released
signatures and adds explanations reviewed against the implementation. Improved
installed hover help and keyword constructors appear in the
[Next API](/docs/reference/next/python/spectrum/) until a new package release.
Updating the website does not replace the stubs in an existing installation.

For Jupyter, add the kernel as a development dependency of this same project:

```sh
uv add --dev ipykernel
uv run python -m ipykernel install --user --name rexafs-analysis --display-name "Python (rexafs analysis)"
```

Select **Python (rexafs analysis)** in your notebook's kernel menu. In VS Code,
you can also select the project's `.venv` directly as the notebook kernel.
See [uv's Jupyter guide](https://docs.astral.sh/uv/guides/integration/jupyter/).

## Load and transform a measured spectrum

Download [cu_150k.xmu](/examples/cu_150k.xmu) into the project, and save the
following script as `analyze.py` beside it.
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

Run the script from the project directory:

```sh
uv run python analyze.py
```

`fft()` calculates missing normalization and background stages. Output arrays are
independent NumPy float64 copies. Before a stage runs, or after invalidation,
an unavailable output is `None`. Input and normalization errors raise
`ValueError`; background and Fourier failures raise `RuntimeError`. The default
Fourier magnitude has units Å⁻³ because `kweight=2` and the amplitude factor is
$\delta k/\sqrt{\pi}$ for the k step $\delta k$ (`kstep`), with no additional
FFT-length normalization.

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

Settings are copied. Reassign them after editing to apply the change. Some
automatic values, including fit ranges and FFT spacings, are retained inside
the spectrum for later calculations; AUTOBK's automatic `kmax` and `nknots` are
instead calculated from each input. Processing does not update the original
settings object. If you change the background
k spacing after a transform, reassign an `XrayFFTF` with `kstep=None` before
calling `fft()` so it infers the new spacing. In this release, `.ifft()` uses
the available inverse defaults; the `XrayFFTR` constructor and `.set_ifft()`
binding are unreleased additions. After changing the forward R spacing, use a
fresh spectrum for the stable inverse or use the configurable Next inverse.

For QAS transmission files with energy, incident intensity and transmitted intensity
in the first three columns, use `rexafs.io.read_qas_transmission(path)`. It computes
the natural logarithm of the intensity ratio. Intensities should be positive
and in matching units. The reader ignores extra columns, treats `#` as a comment
and returns an unprocessed spectrum. It sorts energy and calculated absorption
together when needed, retaining duplicate energy rows. It does not enforce
positive intensities or finite ratios; processing rejects non-finite data, and
duplicates may need cleanup for the selected stage. The array constructor
instead rejects unordered and duplicate energy inputs. Do not use this reader
on a file that already contains μ, such as this Cu example.

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
