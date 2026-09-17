# rexafs for Python

[Published user guide](https://rexafs.com/docs/libraries/python/) ·
[Versioned API reference](https://rexafs.com/docs/reference/)

Process X-ray absorption spectra with Rust and work with the results as NumPy
arrays. Start with `Spectrum(energy, mu).fft()`; configure only what you need.
Energy is in **eV**, k/q in **Å⁻¹**, and R in **Å**.

## Install

We recommend [uv](https://docs.astral.sh/uv/getting-started/installation/) to
manage an analysis project's Python version, dependencies and commands.
CPython 3.10–3.14 is supported; start a Python 3.12 project with the stable
package:

```bash
uv init --python 3.12 rexafs-analysis
cd rexafs-analysis
uv add rexafs==0.2.5 numpy
uv run python -c "import rexafs; print(rexafs.__version__)"
```

`uv add` records dependencies in `pyproject.toml` and resolves their versions in
`uv.lock`. `uv run` uses the project's `.venv` automatically; no shell activation
is needed. Commit `pyproject.toml`, `.python-version` and `uv.lock` with your
analysis, and leave `.venv` out of version control. NumPy is listed explicitly
because the examples import it. See [uv's project guide](https://docs.astral.sh/uv/guides/projects/).

[PyPI](https://pypi.org/project/rexafs/) provides platform wheels. Rust is needed
only when building rexafs from source.

**Version note:** keyword constructors, direct configuration setters and
`XrayFFTR` were added in 0.2.5. The basic example also works in 0.2.4.

## Load and process a spectrum

Save this as `analyze.py` inside the project, beside your data file:

```python
import numpy as np
from rexafs import Spectrum

# Two whitespace-delimited columns: energy in eV and absorption mu.
data = np.loadtxt("spectrum.dat")
spectrum = Spectrum(data[:, 0], data[:, 1]).fft()
r, magnitude = spectrum.r(), spectrum.chir_mag()
print(spectrum.e0(), r, magnitude)
```

Run it with `uv run python analyze.py`. In your editor, select the Python
interpreter from this project's `.venv` for completion and hover help.

For QAS transmission files, `rexafs.io.read_qas_transmission(path)` returns a
Spectrum using `mu = ln(I0 / It)`, the natural logarithm of incident intensity
I0 divided by transmitted intensity It. Both intensities must be positive and
in matching units; this produces optical depth rather than an absolute absorption
coefficient. The first three whitespace-delimited columns are energy, I0 and
It; `#` starts a comment and additional columns are ignored. Do not use this
reader on a file that already contains mu. It accepts `str` or `pathlib.Path`:

```python
from rexafs import io
spectrum = io.read_qas_transmission("Ru_QAS.dat").fft()
```

File/parse failures raise `RuntimeError`. The reader sorts energy and calculated
mu together when needed, retaining duplicate energy rows. It does not enforce
positive intensities or finite ratios; processing rejects non-finite data, and
duplicates can require cleanup for the selected numerical stage. Check the raw
intensities before analysis. The array constructor instead rejects unordered
or duplicate energy values.

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

Some automatic values, including fit ranges and FFT spacings, are retained inside
the spectrum on subsequent calls; clearing results does not reset them to `None`.
AUTOBK's automatic `kmax` and `nknots` are instead calculated from each input.
The original settings
object remains unchanged. For example, after changing the background k spacing,
reassign forward and inverse settings so their automatic spacings are inferred
again:

```python
# Requires 0.2.5 or later; continue with the spectrum above.
spectrum.set_background_method(AUTOBK(kstep=0.1))
spectrum.set_fft(XrayFFTF(kstep=None))
spectrum.set_ifft(XrayFFTR(kstep=None)).ifft()
```

Apply the same principle to a new scan with a different normalization range:
reassign fresh automatic normalization settings instead of retaining the prior
scan's resolved bounds. The generated member help explains these choices.

## What the calculations mean

Normalization subtracts a fitted pre-edge baseline and divides absorption by
its edge step. AUTOBK estimates the smooth background to obtain the EXAFS
oscillations, chi(k). The Fourier transform weights and windows those oscillations
to display them against R; its peaks are not automatically phase-corrected bond
lengths. An inverse transform filters selected R contributions back into q space.

The forward code multiplies an unnormalized, negative-exponent FFT by
`kstep / sqrt(pi)`, with no extra division by FFT length. For dimensionless chi
and forward exponent `w`, chi(R) has units Å⁻⁽ʷ⁺¹⁾; the default `w=2` gives
Å⁻³. The real inverse retains the forward weighting and window, so with default
inverse `rweight=0`, its signal has units Å⁻ʷ and is generally not the original
unweighted chi(k).

The [processing theory guide](../doc/processing-theory.md) explains the
equations, symbols, units, assumptions and implementation choices, with scientific
references. Use the [fitting-statistics guide](../doc/fitting-statistics.md)
when interpreting structural fits and uncertainties.

## Completion, hover help and errors

The installed package includes `py.typed`, annotated `.pyi` files and native
runtime docstrings. Select the project's `.venv` interpreter in your editor
(Pylance/Pyright, for example). Hover over parameters for units, defaults and
behavior; `help(AUTOBK)` and `help(Spectrum.fft)` also work in a terminal.
`FTWindow`, `FFTGrid`, `AUTOBKSolver` and `AUTOBKClampScalePolicy` are Literal
type aliases, so editors suggest supported strings and flag invalid choices.

Invalid inputs/normalization raise `ValueError`; background/FFT failures raise
`RuntimeError`. Stages release the GIL during Rust computation. The public API
is `rexafs`; `_core` is an implementation detail.

## Build from source

**Packaging since 0.2.6:** the Python binding enables PyO3's `abi3-py310`
feature, sharing one wheel per platform across GIL-enabled CPython 3.10–3.14.
The published 0.2.5 wheels remain unchanged. The release workflow tests the same
wheel on every supported interpreter with its minimum available NumPy wheel
and the latest compatible NumPy. Free-threaded Python is not qualified.
See the [release checks](../doc/releasing.md#github-is-the-release-build-authority)
and [PyO3's stable ABI guide](https://pyo3.rs/v0.29.2/building-and-distribution.html#py_limited_apiabi3abi3t).

From the repository root, with the pinned Rust toolchain installed, use an
isolated build environment. This development workflow uses `uv venv` and
`uv run --no-project` so installing the local extension does not change the
library's dependency manifest with analysis-project dependencies:

```bash
uv venv --python 3.14
uv pip install maturin numpy
uv run --no-project maturin develop --release --locked
uv run --no-project python py-rexafs/tests/test_api.py
```

Fitting, groups, structures, plotting and direct ReFEFF calculation remain
Rust/desktop APIs. ILPBkg remains an unimplemented selector. The historical empty
MBACK selector lacks absorber/edge identity; the unreleased configured `MBack` API
below implements the full atomic match. See [AUTOBK defaults](../doc/autobk-fixed-penalty.md)
and [FFT grid compatibility](../doc/fft-grid-compatibility.md).
Licensed under MIT OR Apache-2.0.

## Universal measurement reader (since 0.2.6)

Version 0.2.6 adds content-detected beamline text, CSV, Athena, XTUNES
and HDF5 import through the shared Rust reader. Read a document, inspect its
scans, columns, units and warnings, then select a signal mapping. Ambiguous
channels require explicit selection; detector images require reduction before
creating an absorption spectrum. Reading performs no processing or corrections.
See the [reader guide](../doc/measurement-reader.md) for language examples,
unit conversions, dataset selection, fixture coverage and limits. This API is
not included in the published 0.2.5 packages.

Larix 1.0 session import is available in the shared reader since 0.2.6, including
stored absorption, complex saved arrays and inert session metadata. See the
[Larix import guide](../doc/larix-import.md).

Column selections accept exact, case-sensitive names as well as zero-based indices.
For a QAS file, use `measurement.arrays(energy="energy", i0="i0", it="it")`
for transmission, `iff="iff"` instead of `it` for fluorescence, or
`i0="it", it="ir"` for the reference. The same keywords work with `.spectrum()`.
Duplicate names require indices. Omit `energy_unit` to preserve detected axis
calibration, or explicitly override it with `"eV"` or `"keV"`.

## Development-only scalar measurements

The source checkout adds a one-call measurement API (newer than 0.2.9):

```python
result = spectrum.measure("mean", (-20.0, 30.0), space="flat")
print(result.value, result.unit)
```

Omit `space` for normalized absorption. Energy bounds are offsets from E₀;
`origin="absolute"` selects absolute energy in eV. Missing prerequisite stages
run on a private copy without changing the spectrum. Point, maximum, integral
and mean are supported, with strict coverage checks. See the
[measurement guide](../doc/full-frame-measurements.md) for k/R units, optional
independent errors, numerical meaning and the equivalent Rust/TypeScript calls.
This scalar operation is separate from `rexafs.io.Measurement`, the input-file
reader. The installed wheel includes result types and editor help.

## Full MBACK normalization (unreleased)

```python
from rexafs import MBack
model = MBack("Cu", "K", pre_edge=(-200, -50), post_edge=(100, 800))
result = model.fit(energy, mu)
norm, flat = result.norm, result.flat
spectrum.set_normalization_method(model).normalize()
```

Energy and E₀-relative ranges use eV; `norm` and `flat` are separate dimensionless
outputs. Degree 2 and no erfc are the defaults. Offline atomic data load
automatically. Inputs and model are unchanged by `fit`; assigning settings to a
spectrum copies them. `spectrum.mback_result()` retrieves its full result, or
`None` after invalidation. Inspect resolved ranges and warnings. `result.definition`
pins the original atomic reference for replay. See the
[MBACK guide](../doc/mback-normalization.md) for equations, assumptions and optional
background terms. Runtime atomic-data notices are included in `rexafs/licenses/atomic`.

## Cauchy wavelets (unreleased)

```python
from rexafs import Wavelet
wavelet_map = spectrum.wavelet(Wavelet((2, 12)))
image = wavelet_map.magnitude  # Independent NumPy matrix: R rows, k columns
region = wavelet_map.integral((4, 10), (1, 3))
print(region.value, region.unit)
```

The k interval uses Å⁻¹; R uses Å and is not phase-corrected. Missing normalization
and background stages run on a private copy. Defaults are weight 2, order 100,
k spacing 0.05 Å⁻¹, R up to 6 Å and no taper. Use named options to change them,
for example `Wavelet((2, 12), rmax=4)`. `model.calculate(k, chi)` also accepts
original unweighted χ(k). The [wavelet guide](../doc/wavelet-analysis.md) explains
native-grid integrals, slices, ownership, JSON replay and scientific limitations.
