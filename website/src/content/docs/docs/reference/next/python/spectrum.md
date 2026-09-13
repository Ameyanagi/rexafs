---
title: "Python · Spectrum"
description: "Spectrum signatures, types and docstrings."
audience: user
pagefind: false
---


**Next API · unreleased.** These signatures describe the source checkout. They are not available from `pip install rexafs==0.2.4`.


[Installation and version guide](/docs/reference/) · [Python tutorial](/docs/libraries/python/)


Mutable Rust spectrum. Stages return the same object and release the GIL.

Invalid inputs raise ValueError; background/FFT failures raise RuntimeError.
Missing prerequisite stages use the configured methods and Rust defaults.


[Declaration source](https://github.com/Ameyanagi/rexafs/blob/main/py-rexafs/python/rexafs/__init__.pyi)



## Spectrum


```python
__init__(self, energy: ArrayLike, mu: ArrayLike) -> None
```

Copy energy (eV) and absorption mu into a spectrum. Inputs must be finite, one-dimensional, equal-length, with strictly increasing energy.


## from_arrays


```python
from_arrays(energy: ArrayLike, mu: ArrayLike) -> Spectrum
```

Create a spectrum from energy (eV) and absorption mu. Copies input arrays; equivalent to the constructor.


## set_spectrum


```python
set_spectrum(self, energy: ArrayLike, mu: ArrayLike) -> Spectrum
```

Replace energy (eV) and mu, copy the inputs and clear E0 and derived results. Returns this spectrum.


## set_e0


```python
set_e0(self, e0: float) -> Spectrum
```

Set edge energy in eV and invalidate normalization and downstream results. Returns this spectrum.


## set_normalization_method


```python
set_normalization_method(self, method: PrePostEdge | NormalizationMethod | None=None) -> Spectrum
```

Copy normalization settings and invalidate normalization and downstream results. Accepts PrePostEdge directly or a NormalizationMethod; omitted/None restores automatic pre/post-edge normalization.


## set_background_method


```python
set_background_method(self, method: AUTOBK | BackgroundMethod | None=None) -> Spectrum
```

Copy background settings and invalidate background and downstream results. Accepts AUTOBK directly or a BackgroundMethod; omitted/None restores default AUTOBK.


## set_ifft


```python
set_ifft(self, parameters: XrayFFTR) -> Spectrum
```

Copy inverse-transform settings; clear q and chi(q) while preserving forward results. Returns this spectrum.


## set_fft


```python
set_fft(self, parameters: XrayFFTF) -> Spectrum
```

Copy forward-transform settings; clear Fourier and inverse results while preserving normalization and chi(k). Returns this spectrum.


## e0


```python
e0(self) -> float | None
```

Edge energy in eV, or None before detection or assignment.


## find_e0


```python
find_e0(self) -> Spectrum
```

Detect edge energy from mu and invalidate dependent results. Returns this spectrum.


## normalize


```python
normalize(self) -> Spectrum
```

Run pre/post-edge normalization, finding E0 if needed. Returns this spectrum.


## calc_background


```python
calc_background(self) -> Spectrum
```

Run AUTOBK, computing missing normalization first. Returns this spectrum.


## fft


```python
fft(self) -> Spectrum
```

Compute chi(R), running missing normalization and AUTOBK first. Defaults: k=2..15 inverse angstroms, kweight=2, KaiserBessel, nfft=2048. Returns this spectrum.


## ifft


```python
ifft(self) -> Spectrum
```

Back-transform chi(R) to chi(q), running missing forward stages first. Configure the R window with set_ifft(XrayFFTR(...)). Returns this spectrum.


## invalidate_derived


```python
invalidate_derived(self) -> Spectrum
```

Clear all calculated results while retaining stage settings for recomputation. Returns this spectrum.


## k


```python
k(self) -> NDArray[np.float64] | None
```

Uniform background k axis in inverse angstroms; pairs with chi(). Returns an independent array copy, or None before its stage runs.


## chi


```python
chi(self) -> NDArray[np.float64] | None
```

Unweighted EXAFS chi(k) = (mu - smooth background) / edge_step; dimensionless and paired with k(). Returns an independent array copy, or None before its stage runs.


## norm


```python
norm(self) -> NDArray[np.float64] | None
```

Normalized absorption (mu - pre_edge) / edge_step on the input energy grid; dimensionless. Returns an independent array copy, or None before its stage runs.


## flat


```python
flat(self) -> NDArray[np.float64] | None
```

Normalized absorption with its fitted post-edge trend removed, preserving the edge value; dimensionless. Returns an independent array copy, or None before its stage runs.


## pre_edge


```python
pre_edge(self) -> NDArray[np.float64] | None
```

Fitted pre-edge baseline in mu units on the input energy grid. Returns an independent array copy, or None before its stage runs.


## post_edge


```python
post_edge(self) -> NDArray[np.float64] | None
```

Fitted post-edge baseline in mu units on the input energy grid. Returns an independent array copy, or None before its stage runs.


## r


```python
r(self) -> NDArray[np.float64] | None
```

Forward-transform R axis in angstroms; pairs with chir_mag/real/imag(). Peaks are not phase-corrected bond lengths. Returns an independent array copy, or None before its stage runs.


## kwin


```python
kwin(self) -> NDArray[np.float64] | None
```

Forward Fourier window values; use kwin_k() for the matching axis. Returns an independent array copy, or None before its stage runs.


## kwin_k


```python
kwin_k(self) -> NDArray[np.float64] | None
```

Forward Fourier window k axis in inverse angstroms; may differ from k() with grid=Larch. Returns an independent array copy, or None before its stage runs.


## chir_mag


```python
chir_mag(self) -> NDArray[np.float64] | None
```

Magnitude of chi(R); pairs with r(). Returns an independent array copy, or None before its stage runs.


## chir_real


```python
chir_real(self) -> NDArray[np.float64] | None
```

Real component of chi(R); pairs with r(). Returns an independent array copy, or None before its stage runs.


## chir_imag


```python
chir_imag(self) -> NDArray[np.float64] | None
```

Imaginary component of chi(R); pairs with r(). Returns an independent array copy, or None before its stage runs.


## q


```python
q(self) -> NDArray[np.float64] | None
```

Back-transform q axis in inverse angstroms; pairs with chiq(). Returns an independent array copy, or None before its stage runs.


## chiq


```python
chiq(self) -> NDArray[np.float64] | None
```

Real R-filtered signal on q(); forward k-weighting and windowing remain, so this is not generally the unweighted chi(k). Returns an independent array copy, or None before its stage runs.
