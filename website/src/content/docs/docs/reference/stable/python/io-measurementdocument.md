---
title: "Python · io.MeasurementDocument"
description: "io.MeasurementDocument signatures, defaults and API explanations."
audience: user
pagefind: true
---

**Stable 0.2.14.** These signatures match the released Python package. Explanations are maintained in the source docstrings and reviewed against this release.

[Installation and version guide](/docs/reference/) · [Python tutorial](/docs/libraries/python/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/v0.2.14/py-rexafs/python/rexafs/io.pyi) · [Docstring source](https://github.com/Ameyanagi/rexafs/blob/main/py-rexafs/python/rexafs/io.pyi)

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
