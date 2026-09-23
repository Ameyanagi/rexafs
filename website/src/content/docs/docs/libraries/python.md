---
title: "Python and Jupyter"
description: "Install rexafs and process real Cu data with NumPy."
audience: user
---

## Install

Use [uv](https://docs.astral.sh/uv/getting-started/installation/) to create an
analysis project. rexafs supports CPython 3.10–3.14; this example uses 3.12:

```sh
uv init --python 3.12 rexafs-analysis
cd rexafs-analysis
uv add rexafs==0.2.14 numpy
uv run python -c "import rexafs; print(rexafs.__version__)"
```

Three stable-ABI wheels cover Linux x64, Windows x64 and Apple Silicon macOS.
From **0.2.12**, macOS wheels support **Apple Silicon only**. Intel Mac wheels
remain available for 0.2.11; new Intel source builds are not qualified.
Each platform wheel supports GIL-enabled CPython 3.10–3.14; free-threaded Python
is not qualified. NumPy is installed separately for the selected interpreter. For other
architectures, see [source builds](https://github.com/Ameyanagi/rexafs/tree/main/py-rexafs#build-from-source).

`uv run` uses the project's `.venv` automatically. Commit `pyproject.toml`,
`.python-version` and `uv.lock` to preserve dependencies; exclude `.venv`.
NumPy is explicit because the example imports it. See
[uv's project guide](https://docs.astral.sh/uv/guides/projects/).

In VS Code, select `.venv` with **Python: Select Interpreter** for completion and
hover help. Installed docstrings and the
[stable reference](/docs/reference/stable/python/spectrum/) explain settings,
units, defaults and errors.

### Jupyter

Add a notebook kernel to the same project:

```sh
uv add --dev ipykernel
uv run python -m ipykernel install --user --name rexafs-analysis --display-name "Python (rexafs analysis)"
```

Select **Python (rexafs analysis)** in your notebook, or `.venv` in VS Code.
See [uv's Jupyter guide](https://docs.astral.sh/uv/guides/integration/jupyter/).

## Load and transform a measured spectrum

Download [cu_150k.xmu](/examples/cu_150k.xmu) and save this script as `analyze.py`
beside it. The first two data columns contain energy in eV and absorption.

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

<span id="configure-stable-024"></span>

## Configure processing

Use keyword arguments and pass settings directly. This example filters the
R=1–3 Å interval and back-transforms it to q:

```python
from rexafs import AUTOBK, XrayFFTF, XrayFFTR

background = AUTOBK(rbkg=1.0)
transform = XrayFFTF(kweight=2.0)
inverse = XrayFFTR(rmin=1.0, rmax=3.0)
spectrum.set_background_method(background).set_fft(transform).set_ifft(inverse).ifft()
print("q (inverse angstrom):", spectrum.q())
print("Filtered chi(q):", spectrum.chiq())
```

`ifft()` calculates missing forward stages. With forward `kweight=2` and default
inverse `rweight=0`, `chiq()` has units Å⁻²: it retains the forward weighting and
window. The R interval is not a phase-corrected bond-distance range. Inspect the
data before choosing it. Normalization similarly accepts `PrePostEdge` directly;
the older method wrappers and field assignments remain supported.

Settings are copied. Reassign them after editing to apply the change. Some
automatic values, including fit ranges and FFT spacings, are retained inside
the spectrum for later calculations; AUTOBK's automatic `kmax` and `nknots` are
instead calculated from each input. Processing does not update the original
settings object. If you change the background
k spacing after a transform, reassign an `XrayFFTF` with `kstep=None` before
calling `fft()` so it infers the new spacing. After changing the forward R
spacing or inverse FFT length, also reassign an `XrayFFTR` with `kstep=None`
before `ifft()`. Reapply the desired window and range settings.

## Read measurement files

Use the shared reader for beamline text and supported containers:

```python
from rexafs.io import read_measurement

measurement = read_measurement("measurement.dat")
print(measurement.document["scans"][0]["signals"])
energy_ev, mu = measurement.arrays()
```

Automatic conversion requires exactly one detected signal. When a file has
several channels, select names or indices explicitly, for example
`measurement.arrays(energy="energy", i0="i0", it="it")` for transmission.
Arrays retain acquisition order. Reading does not run processing or detector
corrections. See [reading measurements](/docs/reference/stable/measurement-reading/)
for stored absorption, fluorescence, angles, containers and warnings.

### Specialized QAS reader

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
For repeated spectra, loop over `Spectrum` objects. See [available operations](/docs/libraries/#available-operations) for the binding scope.


## Analysis added in 0.2.10

Use [MBack](/docs/reference/stable/python/mback/) for configured atomic-reference
normalization, [Wavelet](/docs/reference/stable/python/wavelet/) for Cauchy maps
and region measurements, and [PeakFit](/docs/reference/stable/python/peakfit/)
for composite XANES peaks, steps and baselines.
[FluorescenceCorrection](/docs/reference/stable/python/fluorescencecorrection/)
requires explicit composition, emission and geometry; its applicability is
limited to XANES. Each reference documents units, defaults, retained results
and errors. RMC remains available through Rust and the desktop.
