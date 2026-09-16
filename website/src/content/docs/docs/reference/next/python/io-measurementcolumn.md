---
title: "Python · io.MeasurementColumn"
description: "io.MeasurementColumn signatures, defaults and API explanations."
audience: user
pagefind: false
---

**Next API · unreleased.** This reference describes the source checkout. Compare with stable rexafs 0.2.9 before using it with an installed package.

[Installation and version guide](/docs/reference/) · [Python tutorial](/docs/libraries/python/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/main/py-rexafs/python/rexafs/io.pyi) · [Docstring source](https://github.com/Ameyanagi/rexafs/blob/main/py-rexafs/python/rexafs/io.pyi)

Original channel: source name, optional units and samples; None marks nonfinite raw cells.

## name

```python
name: str
```

Original label or generated column_N name.

## units

```python
units: str | None
```

Original unit declaration, or None when undeclared.

## values

```python
values: list[float | None]
```

Owned samples in source order; None denotes nonfinite cells in snapshots.
