---
title: "Python · MeasurementResult"
description: "MeasurementResult signatures, defaults and API explanations."
audience: user
pagefind: true
---

**Stable 0.2.14.** These signatures match the released Python package. Explanations are maintained in the source docstrings and reviewed against this release.

[Installation and version guide](/docs/reference/) · [Python tutorial](/docs/libraries/python/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/v0.2.14/py-rexafs/python/rexafs/__init__.pyi) · [Docstring source](https://github.com/Ameyanagi/rexafs/blob/main/py-rexafs/python/rexafs/__init__.pyi)

Owned result from Spectrum.measure (since 0.2.10). No processing state is mutated.

## value

```python
value(self) -> float
```

Finite scalar in unit.

## unit

```python
unit(self) -> str
```

Signal unit for point/mean/maximum; signal times axis unit for integral.

## range

```python
range(self) -> tuple[float, float]
```

Resolved absolute bounds: eV, inverse angstroms, or angstroms.

## position

```python
position(self) -> float | None
```

Absolute point/maximum position, otherwise None.

## standard_error

```python
standard_error(self) -> float | None
```

Independent propagated standard error when supplied; not a confidence interval.

## e0_ev

```python
e0_ev(self) -> float | None
```

Resolved absorption edge in eV, or None when unnecessary.

## to_json

```python
to_json(self) -> str
```

Serialize the result and full native measurement definition without changing inputs.
