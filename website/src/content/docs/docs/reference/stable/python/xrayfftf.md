---
title: "Python · XrayFFTF"
description: "XrayFFTF signatures, defaults and API explanations."
audience: user
pagefind: true
---

**Stable 0.2.4.** These signatures match the released Python package. Explanations are maintained in the source docstrings and reviewed against this release.

[Installation and version guide](/docs/reference/) · [Python tutorial](/docs/libraries/python/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/v0.2.4/py-rexafs/python/rexafs/__init__.pyi) · [Docstring source](https://github.com/Ameyanagi/rexafs/blob/main/py-rexafs/python/rexafs/__init__.pyi)

Configure the weighted Fourier transform from photoelectron k to distance R.

The code forms g = chi * k**w * W, with integer w=kweight and real window
W, then computes chi_R = (kstep / sqrt(pi)) * rfft(g, n=nfft) using an
unnormalized, negative-exponent FFT. There is no additional 1/nfft factor,
window-area normalization or phase rotation. On a zero-origin uniform
grid, R_m = pi * m / (nfft * kstep), where m is the frequency-bin index.
For dimensionless chi, chi_R has units angstrom**(-(w + 1)); R is in
angstroms. Zero padding refines the R sampling but adds no measured data.

Recommended defaults are kmin=2, kmax=15 inverse angstroms, kweight=2,
window="KaiserBessel", dk=1, nfft=2048 and grid="Input". Automatic kstep
uses the background grid, usually 0.05 inverse angstroms. Settings are
copied into a spectrum, so reassign them after editing.

Example: p = XrayFFTF(); p.kmax = 12.0; spectrum.set_fft(p).fft().
Use spectrum.kwin_k() with kwin() to inspect the window. Scattering phase
shifts mean that an uncorrected Fourier peak is not directly a bond length.

Implementation convention: [NumPy's DFT definition](https://numpy.org/doc/stable/reference/routines.fft.html#implementation-details).
Physical interpretation: [Rehr and Albers (2000)](https://doi.org/10.1103/RevModPhys.72.621).
See [processing theory](https://rexafs.com/docs/science/processing/) for the
equation and links to the implementing Rust functions.

Resolved automatic k limits, k spacing and numeric defaults are retained
inside the spectrum. An unset window still selects Hanning when used.
Processing does not replace None fields in your original settings object.
Reassign fresh or reset settings when you want retained automatic choices
recalculated after changing the data or an earlier stage.

## grid

```python
grid: str
```

Sampling/window convention. Default: "Input".

Input keeps the existing background samples and does not resample them
when kstep changes. Larch linearly interpolates onto a zero-origin grid
with the requested kstep and extends the window domain through the upper
taper. Both preserve Spectrum.k()/chi(); kwin_k() returns the matching
window axis. Unknown names raise ValueError when assigned.

## XrayFFTF

```python
XrayFFTF()
```

Create forward-transform settings with the recommended Rust defaults.

Start with k=2 to 15 inverse angstroms, kweight=2, a KaiserBessel window
and nfft=2048. Choose a useful k range for your measured data, then call
spectrum.set_fft(parameters).fft(). Construction does not run a transform;
numeric validation occurs when fft() processes the data.

Keyword arguments were added in 0.2.5. Published
0.2.4 settings use construction without arguments followed by field
assignment. Python type conversion can raise TypeError, and an integer
outside the native field's representable range can raise OverflowError
before any numerical processing.

## rmax_out

```python
rmax_out: float | None
```

Largest R shown by r() and chir_real/imag/mag(), in angstroms.

Default: 10.0. None also resolves to 10.0. It must be finite and
nonnegative. This limits returned display arrays; the complete internal
forward transform is retained for inverse filtering. Increasing it does not
improve spatial resolution or apply a structural shell filter.

Keep at least two returned R samples if you plan to call ifft(): its
grid validation uses r() even though filtering uses the full stored
Fourier coefficients. For example, rmax_out=0 permits a forward result
but makes a subsequent inverse fail with RuntimeError.

## dk

```python
dk: float | None
```

Low-k window taper parameter. Default: 1.0. None also resolves to 1.0.

For Hanning-like windows it describes a width in inverse angstroms.
KaiserBessel also uses this numeric value to set the Bessel-function shape,
so it is not a universally comparable taper width. Larger tapers generally
soften truncation at the cost of a broader R response. Use kwin_k()/kwin()
to inspect the actual window.

FHanning uses a fractional taper parameter. For Gaussian, dk is the
standard-deviation scale in inverse angstroms and the window has tails
beyond the nominal bounds. It is not a low-end-only width for those
families. See the [window reference](https://xraypy.github.io/xraylarch/xafs_fourier.html#ftwindow)
for their distinct conventions.

## dk2

```python
dk2: float | None
```

High-k window taper parameter. Default None uses dk.

It controls the upper-end geometry in inverse angstroms for the
width-based windows. Set it separately for an asymmetric taper.
Window families interpret taper parameters differently; KaiserBessel's
Bessel-function shape is controlled by dk, not an independent dk2 shape.

For Gaussian, dk2 affects the window domain and center but is not a
second standard deviation. FHanning interprets it as a fractional taper
parameter. See the [window reference](https://xraypy.github.io/xraylarch/xafs_fourier.html#ftwindow)
before comparing settings between families.

## kmin

```python
kmin: float | None
```

Lower Fourier-window limit, in inverse angstroms. Constructor default: 2.0.

Explicit None uses the first background k sample, usually zero. Raising
the limit suppresses low-k contributions but shortens the effective
transform range. The implementation requires finite bounds with kmin
below kmax; negative lower bounds are accepted, but the window is clipped
to its sampled domain. Use a nonnegative bound for the physical k range.
The taper can extend below the nominal limit.

## kmax

```python
kmax: float | None
```

Upper Fourier-window limit, in inverse angstroms. Constructor default: 15.0.

Explicit None uses the last background k sample. Lower it to exclude
a noisy high-k tail; this changes the transform without recomputing chi.
The taper can extend above the nominal bound. Input and Larch use
different window domains when the taper reaches beyond measured data.

## kweight

```python
kweight: float | None
```

Exponent applied as k**w before the transform. Default: 2.0. None also
resolves to 2.0.

Finite nonnegative values are floored to an integer w. A higher weight
emphasizes high-k oscillations and noise. It changes both amplitude and
units: dimensionless chi gives chi(R) in angstrom**(-(w + 1)). The
background chi() remains unweighted.

## nfft

```python
nfft: int | None
```

Total FFT length, including zero padding. Default: 2048. None also resolves
to 2048.

It must be at least 2. Choose a length that contains the prepared data:
Input keeps only the first nfft samples if the data are longer; Larch
requires its extended window grid to fit. Increasing nfft gives a finer R
grid with spacing pi / (nfft * kstep), but does not add structural
information or introduce an extra amplitude normalization.

## kstep

```python
kstep: float | None
```

k spacing used for FFT scaling and the R axis, in inverse angstroms.

Default None uses the difference between the first two input k values;
the default AUTOBK grid gives 0.05. An explicit value must be finite and
positive. Input does not resample, so keep this equal to its actual grid
spacing. Use grid="Larch" when requesting resampling at a different step.
The forward amplitude multiplier is kstep / sqrt(pi).

Once resolved, the spectrum retains this spacing on later fft() calls.
Changing the background k grid does not automatically reset it. Reassign
an XrayFFTF with kstep=None to infer the new spacing; the original
settings object remains unchanged by processing.

## window

```python
window: str | None
```

Forward Fourier-window shape. Constructor default: "KaiserBessel".

Explicit None selects Hanning, which differs from leaving the default
unchanged. The window reduces truncation ringing and broadens the R
response; it is not normalized by its area. Accepted case-sensitive names are Hanning, Parzen, Welch, Gaussian,
Sine, KaiserBessel and FHanning. See the
[window reference](https://xraypy.github.io/xraylarch/xafs_fourier.html#ftwindow)
for the shape-dependent parameter conventions. Unknown names raise ValueError when assigned.
