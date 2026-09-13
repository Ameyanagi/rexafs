---
title: "Python · XrayFFTR"
description: "XrayFFTR signatures, types and docstrings."
audience: user
pagefind: false
---


**Next API · unreleased.** These signatures describe the source checkout. They are not available from `pip install rexafs==0.2.4`.


[Installation and version guide](/docs/reference/) · [Python tutorial](/docs/libraries/python/)


Inverse Fourier-transform settings. Set rmin/rmax to select an R-space shell. Settings are copied when assigned to a spectrum.

See the [XrayLarch Fourier guide](https://xraypy.github.io/xraylarch/xafs_fourier.html) for XAFS conventions. Filtering retains the forward weighting and window.


[Declaration source](https://github.com/Ameyanagi/rexafs/blob/main/py-rexafs/python/rexafs/__init__.pyi)



## XrayFFTR


```python
__init__(self, *, qmax_out: float | None=10.0, dr: float | None=1.0, dr2: float | None=None, rmin: float | None=0.0, rmax: float | None=20.0, rweight: float | None=0.0, nfft: int | None=2048, kstep: float | None=None, window: FTWindow | None='KaiserBessel') -> None
```

Create settings; override only the parameters you need. Defaults match Rust new().


## qmax_out


```python
qmax_out: float | None
```

Maximum back-transform q in inverse angstroms. Default: 10.0.


## dr


```python
dr: float | None
```

Low-R taper width in angstroms. Default: 1.0.


## dr2


```python
dr2: float | None
```

High-R taper width in angstroms. Default: use dr.


## rmin


```python
rmin: float | None
```

Lower inverse-transform window limit in angstroms. Default: 0.0.


## rmax


```python
rmax: float | None
```

Upper inverse-transform window limit in angstroms. Default: 20.0; choose a shell range for R filtering.


## rweight


```python
rweight: float | None
```

Power of R applied before IFFT. Default: 0.0; nonnegative values are floored to an integer.


## nfft


```python
nfft: int | None
```

Inverse FFT length. Default: 2048; leave kstep automatic when changing this.


## kstep


```python
kstep: float | None
```

Output q spacing in inverse angstroms. Default: infer from input R and nfft; an explicit value must match that spacing.


## window


```python
window: FTWindow | None
```

Inverse Fourier window shape. Default: KaiserBessel; explicitly unset uses Hanning.
