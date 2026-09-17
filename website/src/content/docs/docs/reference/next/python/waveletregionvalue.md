---
title: "Python · WaveletRegionValue"
description: "WaveletRegionValue signatures, defaults and API explanations."
audience: user
pagefind: false
---

**Next API · unreleased.** This reference describes the source checkout. Compare with stable rexafs 0.2.9 before using it with an installed package.

[Installation and version guide](/docs/reference/) · [Python tutorial](/docs/libraries/python/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/main/py-rexafs/python/rexafs/__init__.pyi) · [Docstring source](https://github.com/Ameyanagi/rexafs/blob/main/py-rexafs/python/rexafs/__init__.pyi)

Immutable native magnitude statistic. dk times dR cancels for integrals; units equal
k**weight * chi. This is a descriptive transform metric, not concentration.

## value

```python
value(self) -> float
```

Native region statistic, without an experimental uncertainty estimate.

## k_range

```python
k_range(self) -> tuple[float, float]
```

Exact inclusive k bounds in inverse angstroms.

## r_range

```python
r_range(self) -> tuple[float, float]
```

Exact inclusive R bounds in angstroms.

## unit

```python
unit(self) -> str
```

Units of k**weight * chi for all three region statistics.

## method

```python
method(self) -> str
```

Method: bilinear_magnitude_v1 (integral), bilinear_magnitude_mean_v1
or bilinear_magnitude_maximum_v1.

## to_json

```python
to_json(self) -> str
```

Value, exact bounds, units and method as JSON.
