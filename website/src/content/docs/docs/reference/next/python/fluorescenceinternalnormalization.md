---
title: "Python · FluorescenceInternalNormalization"
description: "FluorescenceInternalNormalization signatures, defaults and API explanations."
audience: user
pagefind: false
---

**Next API · unreleased.** This reference describes the source checkout. Compare with stable rexafs 0.2.14 before using it with an installed package.

[Installation and version guide](/docs/reference/) · [Python tutorial](/docs/libraries/python/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/main/py-rexafs/python/rexafs/__init__.pyi) · [Docstring source](https://github.com/Ameyanagi/rexafs/blob/main/py-rexafs/python/rexafs/__init__.pyi)

Internal fit of original mu, distinct from final normalization (since 0.2.10).
All arrays are independent Python lists on the original energy grid.

## e0

```python
e0: float
```

Measured edge energy in eV.

## pre_edge

```python
pre_edge: list[float]
```

Resolved pre-edge eV offsets from E0, [start, end].

## post_edge

```python
post_edge: list[float]
```

Resolved post-edge eV offsets from E0, [start, end].

## degree

```python
degree: int
```

Internal post-edge polynomial degree; pre-edge is linear.

## edge_step

```python
edge_step: float
```

Positive fitted jump in original absorption units, before numerical flooring.

## pre_curve

```python
pre_curve: list[float]
```

Pre-edge line in original absorption units.

## post_curve

```python
post_curve: list[float]
```

Pre-edge line plus post-edge polynomial, in original absorption units.

## norm

```python
norm: list[float]
```

Dimensionless internal n0 used in alpha+1-n0.
