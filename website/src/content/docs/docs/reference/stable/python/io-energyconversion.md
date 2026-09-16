---
title: "Python · io.EnergyConversion"
description: "io.EnergyConversion signatures, defaults and API explanations."
audience: user
pagefind: true
---

**Stable 0.2.9.** These signatures match the released Python package. Explanations are maintained in the source docstrings and reviewed against this release.

[Installation and version guide](/docs/reference/) · [Python tutorial](/docs/libraries/python/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/v0.2.9/py-rexafs/python/rexafs/io.pyi) · [Docstring source](https://github.com/Ameyanagi/rexafs/blob/main/py-rexafs/python/rexafs/io.pyi)

Axis conversion: ev, kev, offset_ev, or bragg (crystal spacing in Å).

offset_ev adds a finite energy origin in eV to each source value. FDMNES
detection selects its declared E_edge; source arrays remain unchanged.
bragg requires positive d_spacing and degrees_per_unit.

## kind

```python
kind: Literal['ev', 'kev', 'offset_ev', 'bragg']
```

Axis convention: ev, kev, offset_ev or bragg. Required at runtime.

## offset_ev

```python
offset_ev: float
```

Finite energy origin in eV; required for offset_ev. Zero leaves eV unchanged.

## d_spacing

```python
d_spacing: float
```

Positive crystal-plane spacing in angstroms; required for bragg.

## degrees_per_unit

```python
degrees_per_unit: float
```

Positive multiplier from source axis to degrees; required for bragg.
