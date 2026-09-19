---
title: "Python · io.MeasurementColumn"
description: "io.MeasurementColumn signatures, defaults and API explanations."
audience: user
pagefind: true
---

**Stable 0.2.11.** These signatures match the released Python package. Explanations are maintained in the source docstrings and reviewed against this release.

[Installation and version guide](/docs/reference/) · [Python tutorial](/docs/libraries/python/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/v0.2.11/py-rexafs/python/rexafs/io.pyi) · [Docstring source](https://github.com/Ameyanagi/rexafs/blob/main/py-rexafs/python/rexafs/io.pyi)

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
