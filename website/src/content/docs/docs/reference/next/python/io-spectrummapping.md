---
title: "Python · io.SpectrumMapping"
description: "io.SpectrumMapping signatures, defaults and API explanations."
audience: user
pagefind: false
---

**Next API · unreleased.** This reference describes the source checkout. Compare with stable rexafs 0.2.6 before using it with an installed package.

[Installation and version guide](/docs/reference/) · [Python tutorial](/docs/libraries/python/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/main/py-rexafs/python/rexafs/io.pyi) · [Docstring source](https://github.com/Ameyanagi/rexafs/blob/main/py-rexafs/python/rexafs/io.pyi)

Explicit axis and detector mapping; energy converts to eV without inferred corrections.

## energy_column

```python
energy_column: ColumnSelector
```

Exact source axis name or zero-based index. Names must be unique within the scan.

## energy

```python
energy: EnergyConversion
```

Declared energy unit or explicit Bragg calibration.

## signal

```python
signal: SignalConversion
```

Detector arithmetic; no corrections are inferred.
