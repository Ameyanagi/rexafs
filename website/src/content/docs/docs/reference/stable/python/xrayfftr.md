---
title: "Python · XrayFFTR"
description: "XrayFFTR signatures, defaults and API explanations."
audience: user
pagefind: true
---

**Stable 0.2.5.** These signatures match the released Python package. Explanations are maintained in the source docstrings and reviewed against this release.

[Installation and version guide](/docs/reference/) · [Python tutorial](/docs/libraries/python/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/v0.2.5/py-rexafs/python/rexafs/__init__.pyi) · [Docstring source](https://github.com/Ameyanagi/rexafs/blob/main/py-rexafs/python/rexafs/__init__.pyi)

Configure a real inverse Fourier transform with an R-space window.

The code weights positive-R bins by R**rweight and a real window, then
reconstructs their conjugate negative-frequency partners to obtain a real
signal on q. q has the same physical meaning as k, in inverse angstroms;
the different name identifies a back-transformed, potentially filtered
signal. The forward k weighting and window remain in that signal.
This real inverse differs from Larch's complex, one-sided back-transform.

Defaults are rmin=0, rmax=20 angstroms, rweight=0, a KaiserBessel window,
dr=1 angstrom, qmax_out=10 inverse angstroms, nfft=2048, and automatic
kstep. Choose an R interval appropriate to your shells for deliberate
filtering. The uncorrected R axis includes scattering phase shifts.

Example: p = XrayFFTR(); p.rmin = 1.0; p.rmax = 3.0;
spectrum.set_ifft(p).ifft(). Settings are copied; reassign after edits.
These Python settings and set_ifft() were added in 0.2.5.

See the [implemented inverse convention](https://rexafs.com/docs/science/processing/)
for the scaling, and [Larch's Fourier guide](https://xraypy.github.io/xraylarch/xafs_fourier.html)
for windowing concepts rather than an assertion of identical inverse output.

Resolved automatic R limits, q spacing and numeric defaults are retained
inside the spectrum. Unset dr2 and window continue to select dr and
Hanning when the window is calculated.
Processing does not replace None fields in your original settings object.
Reassign fresh or reset settings when you want retained automatic choices
recalculated after changing the data or an earlier stage.

## XrayFFTR

```python
XrayFFTR(*, qmax_out: float | None=10.0, dr: float | None=1.0, dr2: float | None=None, rmin: float | None=0.0, rmax: float | None=20.0, rweight: float | None=0.0, nfft: int | None=2048, kstep: float | None=None, window: FTWindow | None='KaiserBessel')
```

Create inverse-transform settings with the recommended Rust defaults.

Set rmin/rmax to select an R region and use rweight=0 unless extra R
weighting is intended. Assign with spectrum.set_ifft(parameters).ifft().
The returned signal retains forward weighting and windowing; construction
alone does not filter a spectrum.

This settings class was added in 0.2.5; it is
not exported by the published 0.2.4 package. Python type conversion can raise TypeError, and an integer
outside the native field's representable range can raise OverflowError
before any numerical processing.

## qmax_out

```python
qmax_out: float | None
```

Largest returned q value, in inverse angstroms. Default: 10.0. None also
resolves to 10.0.

The q()/chiq() arrays stop at this bound or the available inverse samples,
whichever comes first. It must be finite and nonnegative. This limits
returned samples and does not change the R-space filter.

## dr

```python
dr: float | None
```

Low-R window taper parameter. Default: 1.0. None also resolves to 1.0.

For width-based windows the unit is angstroms. KaiserBessel also uses this
numeric value as its Bessel-function shape parameter. A wider taper smooths
the selected R boundary but mixes a broader range of distances into the
filtered signal.

FHanning instead uses a fractional taper parameter. Gaussian uses dr
as its standard-deviation scale in angstroms and has nonzero tails
beyond the nominal R interval.

## dr2

```python
dr2: float | None
```

High-R window taper parameter. Default None uses dr.

Use a separate value for asymmetric R-window geometry. The unit is
angstroms for width-based windows; KaiserBessel's Bessel-function shape
uses dr, so dr2 does not define an independent high-end shape.

Gaussian uses dr for its standard deviation; dr2 affects the domain
and center rather than providing a second Gaussian width. FHanning
uses a fractional taper parameter.

## rmin

```python
rmin: float | None
```

Lower R-window limit, in angstroms. Constructor default: 0.0.

Explicit None uses the first input R sample, which is zero. A larger
value removes lower-R contributions from the back-transform. The bound
must be finite, nonnegative and below rmax; the taper can extend below it.
R peaks have not been corrected for scattering phase shifts.

## rmax

```python
rmax: float | None
```

Upper R-window limit, in angstroms. Constructor default: 20.0.

Choose a shell range such as 1 to 3 angstroms only when appropriate for
your data. Explicit None uses the last reported input R sample, which
depends on forward rmax_out; an explicit bound instead acts on the full
internal Fourier bins. The taper can extend above the nominal bound.

## rweight

```python
rweight: float | None
```

Exponent applied as R**v before the inverse transform. Default: 0.0.

None resolves to 0.0; finite nonnegative values are floored to an integer
v. Leave it at zero for ordinary R filtering. A positive value emphasizes
higher-R contributions and changes the units of chiq(): with forward
kweight w the units are angstrom**(v - w).

## nfft

```python
nfft: int | None
```

Inverse transform length. Default: 2048. None also resolves to 2048;
minimum: 2.

Leave kstep automatic when changing this value. With fixed input R spacing,
a larger nfft gives a finer q grid by zero padding Fourier bins; a smaller
value discards bins beyond its representable range. Neither operation adds
experimental information.

## kstep

```python
kstep: float | None
```

Output q spacing, in inverse angstroms.

Default None uses pi / (nfft * delta_R), where delta_R is the difference
between the first two R samples in angstroms. An explicit positive value
must agree with that spacing or processing raises RuntimeError. Keep it
automatic when changing nfft so the physical Fourier grid stays consistent.

Automatic spacing is retained inside the spectrum after the first
inverse. If the forward R grid changes, assign fresh inverse settings
with kstep=None before calling ifft() again. The earlier resolved value
otherwise remains subject to the same consistency check.

## window

```python
window: FTWindow | None
```

R-space Fourier-window shape. Constructor default: "KaiserBessel".

Explicit None selects Hanning, unlike leaving the default unchanged.
The window selects and tapers R contributions before the real inverse;
it cannot undo the weighting or information lost in the forward window.
Accepted case-sensitive names are Hanning, Parzen, Welch, Gaussian,
Sine, KaiserBessel and FHanning. See the
[window reference](https://xraypy.github.io/xraylarch/xafs_fourier.html#ftwindow)
for the shape-dependent parameter conventions. Unknown names raise ValueError when assigned.
