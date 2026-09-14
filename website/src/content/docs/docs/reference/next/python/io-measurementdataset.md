---
title: "Python · io.MeasurementDataset"
description: "io.MeasurementDataset signatures, defaults and API explanations."
audience: user
pagefind: false
---

**Next API · unreleased.** This reference describes the source checkout. Compare with stable rexafs 0.2.5 before using it with an installed package.

[Installation and version guide](/docs/reference/) · [Python tutorial](/docs/libraries/python/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/main/py-rexafs/python/rexafs/io.pyi) · [Docstring source](https://github.com/Ameyanagi/rexafs/blob/main/py-rexafs/python/rexafs/io.pyi)

Numeric dataset or saved Larix/XTUNES array with original dimensions and row-major values; None marks nonfinite cells.

## path

```python
path: str
```

HDF5 path, XTUNES table path, or Larix /symbol/attribute path; Larix escapes ~ and / as ~0 and ~1.

## shape

```python
shape: list[int]
```

Original dimensions in row-major order; an empty list denotes a scalar.

## values

```python
values: list[float | None]
```

Owned row-major values; None represents nonfinite source cells.

## imaginary

```python
imaginary: list[float | None] | None
```

Imaginary components matching values and shape for complex Larix arrays; None for real arrays. Values contains the real components.

## attributes

```python
attributes: dict[str, str]
```

Source attributes and quantity descriptions. Larix retains exact numeric bytes in larix.bytes_base64 with NumPy dtype in larix.dtype, even when f64 views round large integers.
