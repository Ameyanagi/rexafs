---
title: "Python · Spectrum"
description: "Spectrum signatures, types and docstrings."
audience: user
pagefind: true
---


**Stable 0.2.4.** Install the stable package to use these signatures.


[Installation and version guide](/docs/reference/) · [Python tutorial](/docs/libraries/python/)



[Declaration source](https://github.com/Ameyanagi/rexafs/blob/v0.2.4/py-rexafs/python/rexafs/__init__.pyi)



## Spectrum


```python
__init__(self, energy: ArrayLike, mu: ArrayLike) -> None
```

Initialize self.  See help(type(self)) for accurate signature.


## from_arrays


```python
from_arrays(energy: ArrayLike, mu: ArrayLike) -> Spectrum
```


## set_spectrum


```python
set_spectrum(self, energy: ArrayLike, mu: ArrayLike) -> Spectrum
```


## set_e0


```python
set_e0(self, e0: float) -> Spectrum
```


## set_normalization_method


```python
set_normalization_method(self, method: NormalizationMethod | None=None) -> Spectrum
```


## set_background_method


```python
set_background_method(self, method: BackgroundMethod | None=None) -> Spectrum
```


## set_fft


```python
set_fft(self, parameters: XrayFFTF) -> Spectrum
```


## e0


```python
e0(self) -> float | None
```


## find_e0


```python
find_e0(self) -> Spectrum
```


## normalize


```python
normalize(self) -> Spectrum
```


## calc_background


```python
calc_background(self) -> Spectrum
```


## fft


```python
fft(self) -> Spectrum
```


## ifft


```python
ifft(self) -> Spectrum
```


## invalidate_derived


```python
invalidate_derived(self) -> Spectrum
```


## k


```python
k(self) -> NDArray[np.float64] | None
```


## chi


```python
chi(self) -> NDArray[np.float64] | None
```


## norm


```python
norm(self) -> NDArray[np.float64] | None
```


## flat


```python
flat(self) -> NDArray[np.float64] | None
```


## pre_edge


```python
pre_edge(self) -> NDArray[np.float64] | None
```


## post_edge


```python
post_edge(self) -> NDArray[np.float64] | None
```


## r


```python
r(self) -> NDArray[np.float64] | None
```


## kwin


```python
kwin(self) -> NDArray[np.float64] | None
```


## kwin_k


```python
kwin_k(self) -> NDArray[np.float64] | None
```


## chir_mag


```python
chir_mag(self) -> NDArray[np.float64] | None
```


## chir_real


```python
chir_real(self) -> NDArray[np.float64] | None
```


## chir_imag


```python
chir_imag(self) -> NDArray[np.float64] | None
```


## q


```python
q(self) -> NDArray[np.float64] | None
```


## chiq


```python
chiq(self) -> NDArray[np.float64] | None
```
