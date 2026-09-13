---
title: "Python · XrayFFTF"
description: "XrayFFTF signatures, types and docstrings."
audience: user
pagefind: false
---


**Next API · unreleased.** These signatures describe the source checkout. They are not available from `pip install rexafs==0.2.4`.


[Installation and version guide](/docs/reference/) · [Python tutorial](/docs/libraries/python/)


Forward Fourier-transform settings. Defaults: k=2..15 inverse angstroms, kweight=2, KaiserBessel window. Settings are copied when assigned to a spectrum.

XAFS and scattering theory: [Rehr and Albers (2000)](https://doi.org/10.1103/RevModPhys.72.621). Fourier peaks are not automatically phase-corrected bond distances.


[Declaration source](https://github.com/Ameyanagi/rexafs/blob/main/py-rexafs/python/rexafs/__init__.pyi)



## XrayFFTF


```python
__init__(self, *, grid: FFTGrid='Input', rmax_out: float | None=10.0, dk: float | None=1.0, dk2: float | None=None, kmin: float | None=2.0, kmax: float | None=15.0, kweight: float | None=2.0, nfft: int | None=2048, kstep: float | None=None, window: FTWindow | None='KaiserBessel') -> None
```

Create settings; override only the parameters you need. Defaults match Rust new().


## grid


```python
grid: FFTGrid
```

Sampling/window domain. Default: Input (existing k grid). Larch resamples on the extended FFT window grid.


## rmax_out


```python
rmax_out: float | None
```

Maximum displayed R in angstroms. Default: 10.0; does not truncate the inverse-transform filter.


## dk


```python
dk: float | None
```

Low-k taper width in inverse angstroms. Default: 1.0.


## dk2


```python
dk2: float | None
```

High-k taper width in inverse angstroms. Default: use dk.


## kmin


```python
kmin: float | None
```

Lower Fourier window limit in inverse angstroms. Default: 2.0; None/undefined uses the first k sample.


## kmax


```python
kmax: float | None
```

Upper Fourier window limit in inverse angstroms. Default: 15.0; None/undefined uses the last k sample.


## kweight


```python
kweight: float | None
```

Power of k applied before FFT. Default: 2.0; nonnegative values are floored to an integer.


## nfft


```python
nfft: int | None
```

Forward FFT length. Default: 2048.


## kstep


```python
kstep: float | None
```

FFT k spacing in inverse angstroms. Default: infer from input k.


## window


```python
window: FTWindow | None
```

Fourier window shape. Default: KaiserBessel; explicitly unset uses Hanning.
