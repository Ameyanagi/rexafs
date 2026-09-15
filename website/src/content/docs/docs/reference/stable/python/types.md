---
title: "Python · types and version"
description: "types and version signatures, defaults and API explanations."
audience: user
pagefind: true
---

**Stable 0.2.7.** These signatures match the released Python package. Explanations are maintained in the source docstrings and reviewed against this release.

[Installation and version guide](/docs/reference/) · [Python tutorial](/docs/libraries/python/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/v0.2.7/py-rexafs/python/rexafs/__init__.pyi) · [Docstring source](https://github.com/Ameyanagi/rexafs/blob/main/py-rexafs/python/rexafs/__init__.pyi)

## __version__

```python
__version__: str
```

Version of the installed Python package, for example "0.2.4". Include this value when reporting results or requesting help; a source build can contain changes beyond the published package with the same version.

## FFTGrid

```python
FFTGrid: TypeAlias = Literal['Input', 'Larch']
```

Sampling conventions for the forward transform: "Input" or "Larch".

"Input" preserves the background k grid. "Larch" linearly resamples onto a
zero-origin grid and extends the window domain to cover its upper taper.
Neither changes Spectrum.k() or Spectrum.chi(); use kwin_k() with kwin().
See [FFT grid compatibility](https://rexafs.com/docs/science/fourier-compatibility/).

## FTWindow

```python
FTWindow: TypeAlias = Literal['Hanning', 'Parzen', 'Welch', 'Gaussian', 'Sine', 'KaiserBessel', 'FHanning']
```

Names of the supported real Fourier windows.

Hanning and FHanning use cosine tapers; Parzen is linear, Welch is parabolic,
Gaussian is bell-shaped, Sine uses a sine profile, and KaiserBessel uses a
modified Bessel function. Windows reduce truncation ringing but broaden peaks.
Taper parameters are shape-dependent: KaiserBessel also uses dk/dr to control
its shape, so equal parameter values do not make the windows equivalent.
Use the default for each processing stage as a starting point and inspect
the resulting window. See [Larch's window reference](https://xraypy.github.io/xraylarch/xafs_fourier.html#ftwindow).

Names are case-sensitive. FHanning interprets its taper parameters as
fractions of the selected interval rather than ordinary absolute widths.
Gaussian uses dk/dr as its standard-deviation scale and has nonzero tails
outside the nominal interval; dk2/dr2 affects the domain geometry rather
than defining a separate Gaussian width. Inspect the generated window
when choosing any family; there is no area normalization.

## AUTOBKSolver

```python
AUTOBKSolver: TypeAlias = Literal['TrustRegionDogLeg', 'LegacyLm', 'LinearDirect']
```

Names of the AUTOBK spline solvers.

"LinearDirect" solves a linear least-squares problem and is required by the
recommended "FixedPenalty" objective. "TrustRegionDogLeg" and "LegacyLm"
are iterative alternatives for the legacy "Fixed" or "TwoPass" objectives.
TrustRegionDogLeg requires the trust-region Rust feature, included in Python
packages. Changing the solver does not select a compatible objective for you.

## AUTOBKClampScalePolicy

```python
AUTOBKClampScalePolicy: TypeAlias = Literal['FixedPenalty', 'Fixed', 'TwoPass']
```

Names of the AUTOBK endpoint-penalty models.

"FixedPenalty" is recommended: a fixed mean-square penalty discourages large
endpoint oscillations and uses LinearDirect. "Fixed" and "TwoPass" retain
legacy residual-dependent clamp scaling; TwoPass updates that scale after
an initial direct solve. The models solve different objectives and need not
produce identical backgrounds. See the [AUTOBK objective](https://rexafs.com/docs/science/autobk/).
