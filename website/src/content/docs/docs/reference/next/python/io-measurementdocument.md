---
title: "Python · io.MeasurementDocument"
description: "io.MeasurementDocument signatures, defaults and API explanations."
audience: user
pagefind: false
---

**Next API · unreleased.** This reference describes the source checkout. Compare with stable rexafs 0.2.5 before using it with an installed package.

[Installation and version guide](/docs/reference/) · [Python tutorial](/docs/libraries/python/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/main/py-rexafs/python/rexafs/io.pyi) · [Docstring source](https://github.com/Ameyanagi/rexafs/blob/main/py-rexafs/python/rexafs/io.pyi)

Independent snapshot of format, scans, saved arrays, session metadata and container diagnostics.

## format

```python
format: str
```

Content-detected format family.

## scans

```python
scans: list[MeasurementScan]
```

All recovered acquisition scans and project records.

## datasets

```python
datasets: list[MeasurementDataset]
```

HDF5 arrays and archived Larix/XTUNES results, including independent grids and complex components.

## metadata

```python
metadata: dict[str, str]
```

Container provenance. Larix session_text, command_history and symbol_order keys are prefixed larix.; commands and saved Python objects remain inert text.

## warnings

```python
warnings: list[str]
```

Container and encoding diagnostics, including partial HDF5 recovery.
