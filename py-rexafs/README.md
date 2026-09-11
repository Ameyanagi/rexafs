# rexafs for Python

Process X-ray absorption spectra with Rust and work with the results as NumPy
arrays. Start with `Spectrum(energy, mu).fft()`; configure only what you need.
Energy is in **eV**, k/q in **Å⁻¹**, and R in **Å**.

## Install

Use CPython 3.10–3.14 in a virtual environment:

```bash
python -m venv .venv
# macOS/Linux:
source .venv/bin/activate
# Windows PowerShell: .venv\Scripts\Activate.ps1
python -m pip install --upgrade rexafs
```

[PyPI](https://pypi.org/project/rexafs/) provides platform wheels. Rust is needed
only when building from source. NumPy is installed as a dependency.

**Version note:** keyword constructors, direct configuration setters and
`XrayFFTR` below are additions after 0.2.4. Until released, install this checkout
using [Build from source](#build-from-source). The basic example works in 0.2.4.

## Load and process a spectrum

```python
import numpy as np
from rexafs import Spectrum

# Two whitespace-delimited columns: energy in eV and absorption mu.
data = np.loadtxt("spectrum.dat")
spectrum = Spectrum(data[:, 0], data[:, 1]).fft()
r, magnitude = spectrum.r(), spectrum.chir_mag()
print(spectrum.e0(), r, magnitude)
```

For QAS transmission files, `rexafs.io.read_qas_transmission(path)` returns a
Spectrum using `mu = ln(I0 / It)`, the natural logarithm of incident intensity
I0 divided by transmitted intensity It. Both intensities must be positive and
in matching units; this produces optical depth rather than an absolute absorption
coefficient. It accepts `str` or `pathlib.Path`:

```python
from rexafs import io
spectrum = io.read_qas_transmission("Ru_QAS.dat").fft()
```

`normalize()`, `calc_background()`, `fft()` and `ifft()` return the same spectrum.
Missing prerequisite stages run automatically. Results are copied NumPy float64
arrays; a result is `None` until its stage has run. Check for `None` when using a
result in typed code. Input lists, arrays and strided views are accepted and
copied; input energy must be finite and strictly increasing, with matching mu.

## Configure only the parameters you need

```python
from rexafs import AUTOBK, PrePostEdge, XrayFFTF, XrayFFTR

spectrum.set_normalization_method(PrePostEdge(pre_edge_end=-30.0))
spectrum.set_background_method(AUTOBK(rbkg=1.0))
spectrum.set_fft(XrayFFTF(kmin=2.0, kmax=12.0, kweight=2.0)).fft()
# Optional: isolate an R range and back-transform it.
spectrum.set_ifft(XrayFFTR(rmin=1.0, rmax=3.0, dr=0.5)).ifft()
q, filtered_chi = spectrum.q(), spectrum.chiq()
```

These ranges are examples; choose them for your data. Constructors take named
arguments, which editors can complete. Fields remain mutable, so existing code
such as `background = AUTOBK(); background.rbkg = 1.2` still works.
`BackgroundMethod.AUTOBK(background)` and
`NormalizationMethod.PrePostEdge(parameters)` remain available for Rust-style
algorithm selection; passing settings directly is the shorter equivalent.

| Settings | Recommended starting values |
|---|---|
| `PrePostEdge()` | Automatically choose E0, fit ranges and polynomial order from the spectrum |
| `AUTOBK()` | `rbkg=1.0`, `kstep=0.05`, `kweight=1`, `window="Hanning"`, `solver="LinearDirect"`, `clamp_scale_policy="FixedPenalty"`, `clamp_lambda=0.001` |
| `XrayFFTF()` | `kmin=2.0`, `kmax=15.0`, `kweight=2.0`, `dk=1.0`, `window="KaiserBessel"`, `nfft=2048`, `grid="Input"` |
| `XrayFFTR()` | `rmin=0.0`, `rmax=20.0`, `dr=1.0`, `rweight=0.0`, `qmax_out=10.0`, `nfft=2048`, automatic `kstep` |

Omitting an argument preserves its constructor default. `None` requests automatic
resolution for optional fields; this can differ from the constructor default
(for example, `XrayFFTF(kmax=None)` uses the measured upper k limit).
`PrePostEdge()` follows Rust `PrePostEdge::new()`, whose ranges are automatic,
rather than the fixed ranges in Rust `PrePostEdge::default()`.

Setters copy settings and clear affected results. Editing the original settings
later requires calling the setter again. `set_e0(eV)` clears normalization and
all later results; `set_fft()` preserves chi(k); `set_ifft()` preserves chi(R).
Calling a stage explicitly recomputes it. See the [shared API guide](../doc/api.md).

## What the calculations mean

Normalization subtracts a fitted pre-edge baseline and divides absorption by
its edge step. AUTOBK estimates the smooth background to obtain the EXAFS
oscillations, chi(k). The Fourier transform weights and windows those oscillations
to display them against R; its peaks are not automatically phase-corrected bond
lengths. An inverse transform filters selected R contributions back into q space.

The [processing theory guide](../doc/processing-theory.md) explains the
equations, symbols, units, assumptions and implementation choices, with scientific
references. Use the [fitting-statistics guide](../doc/fitting-statistics.md)
when interpreting structural fits and uncertainties.

## Completion, hover help and errors

The installed package includes `py.typed`, annotated `.pyi` files and native
runtime docstrings. Select the environment containing rexafs in your editor
(Pylance/Pyright, for example). Hover over parameters for units, defaults and
behavior; `help(AUTOBK)` and `help(Spectrum.fft)` also work in a terminal.
`FTWindow`, `FFTGrid`, `AUTOBKSolver` and `AUTOBKClampScalePolicy` are Literal
type aliases, so editors suggest supported strings and flag invalid choices.

Invalid inputs/normalization raise `ValueError`; background/FFT failures raise
`RuntimeError`. Stages release the GIL during Rust computation. The public API
is `rexafs`; `_core` is an implementation detail.

## Build from source

From the repository root, with the pinned Rust toolchain installed:

```bash
uv venv --python 3.14
uv pip install maturin numpy
uv run --no-project maturin develop --release --locked
uv run --no-project python py-rexafs/tests/test_api.py
```

Fitting, groups, structures, plotting and direct ReFEFF calculation remain
Rust/desktop APIs. MBack and ILPBkg selectors are unimplemented placeholders and
raise errors when processed. See [AUTOBK defaults](../doc/autobk-fixed-penalty.md)
and [FFT grid compatibility](../doc/fft-grid-compatibility.md).
Licensed under MIT OR Apache-2.0.
