---
title: "Python · WaveletMap"
description: "WaveletMap signatures, defaults and API explanations."
audience: user
pagefind: true
---

**Stable 0.2.14.** These signatures match the released Python package. Explanations are maintained in the source docstrings and reviewed against this release.

[Installation and version guide](/docs/reference/) · [Python tutorial](/docs/libraries/python/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/v0.2.14/py-rexafs/python/rexafs/__init__.pyi) · [Docstring source](https://github.com/Ameyanagi/rexafs/blob/main/py-rexafs/python/rexafs/__init__.pyi)

Owned native Cauchy map (since 0.2.10), with independent NumPy array properties.

Matrices have shape (R rows, k columns), including explicit k padding. W has
units of k**weight * chi, distinct from ordinary Fourier scaling. No color
normalization changes the scientific data; to_json retains full provenance.

## shape

```python
shape(self) -> tuple[int, int]
```

Matrix dimensions: R rows, k columns.

## k

```python
k(self) -> NDArray[np.float64]
```

Independent inverse-angstrom coordinates, including padded columns.

## r

```python
r(self) -> NDArray[np.float64]
```

Independent angstrom coordinates; not phase-corrected distances.

## input_k

```python
input_k(self) -> NDArray[np.float64]
```

Original measured k before resampling.

## input_chi

```python
input_chi(self) -> NDArray[np.float64]
```

Original unweighted dimensionless chi, unchanged.

## prepared_chi

```python
prepared_chi(self) -> NDArray[np.float64]
```

Resampled unweighted chi; values outside support are padding zeros.

## window

```python
window(self) -> NDArray[np.float64]
```

Support/taper multipliers applied before k weighting.

## support

```python
support(self) -> NDArray[np.bool_]
```

True for selected measured support; False for padding.

## real

```python
real(self) -> NDArray[np.float64]
```

Independent real matrix, rows=R and columns=k.

## imaginary

```python
imaginary(self) -> NDArray[np.float64]
```

Independent imaginary matrix, rows=R and columns=k.

## magnitude

```python
magnitude(self) -> NDArray[np.float64]
```

Independent full-native-grid magnitude matrix.

## phase

```python
phase(self, relative_floor: float=0.01) -> NDArray[np.float64]
```

Radians; NaN masks zero amplitude and values below a fraction of the map
maximum (default 1%). Fraction is in [0,1]. Native data stay unchanged.

## slice_at_r

```python
slice_at_r(self, r: float) -> NDArray[np.float64]
```

Native magnitude versus k at a covered R coordinate (angstroms).

## slice_at_k

```python
slice_at_k(self, k: float) -> NDArray[np.float64]
```

Native magnitude versus R at a covered k coordinate (inverse angstroms).

## integral

```python
integral(self, k_range: tuple[float, float], r_range: tuple[float, float]) -> WaveletRegionValue
```

Integrate native bilinear magnitude over a fully covered rectangle.
k_range uses inverse angstroms and r_range angstroms. Returns exact bounds,
value, units and method without experimental uncertainty. Releases the GIL;
display sampling never participates. Invalid coverage raises ValueError.

## mean

```python
mean(self, k_range: tuple[float, float], r_range: tuple[float, float]) -> WaveletRegionValue
```

Area-weighted mean of native bilinear magnitude (since 0.2.10).
k is inverse angstroms; R is angstroms. Requires increasing, fully covered
ranges. Returns units and method without uncertainty; releases the GIL.

## maximum

```python
maximum(self, k_range: tuple[float, float], r_range: tuple[float, float]) -> WaveletRegionValue
```

Maximum native bilinear magnitude, including rectangle boundaries
(since 0.2.10). k is inverse angstroms; R is angstroms. Invalid coverage
raises ValueError. Returns units/method without uncertainty; releases the GIL.

## definition

```python
definition(self) -> Wavelet
```

Independent transform definition, retaining automatic and explicit choices.

## preparation

```python
preparation(self) -> dict[str, object] | None
```

Original spectrum preparation metadata; None for direct array calculations.

## warnings

```python
warnings(self) -> list[str]
```

Interpretation/boundary diagnostics, not confidence intervals.

## to_json

```python
to_json(self) -> str
```

Complete native map, original inputs and processing provenance.

## from_json

```python
from_json(json: str) -> WaveletMap
```

Restore checked method, dimensions, axes, finite values and budgets.
Validation does not independently prove external numerical results.
