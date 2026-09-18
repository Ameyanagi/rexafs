---
title: "Python · WaveletSize"
description: "WaveletSize signatures, defaults and API explanations."
audience: user
pagefind: false
---

**Next API · unreleased.** This reference describes the source checkout. Compare with stable rexafs 0.2.10 before using it with an installed package.

[Installation and version guide](/docs/reference/) · [Python tutorial](/docs/libraries/python/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/main/py-rexafs/python/rexafs/__init__.pyi) · [Docstring source](https://github.com/Ameyanagi/rexafs/blob/main/py-rexafs/python/rexafs/__init__.pyi)

Checked dimensions and buffer estimate; input copies/scratch/serialization add overhead.

## k_points

```python
k_points: int
```

Number of prepared k columns, including padding.

## r_points

```python
r_points: int
```

Number of positive R rows.

## nfft

```python
nfft: int
```

Internal FFT length, in samples.

## cells

```python
cells: int
```

Number of complex map cells.

## bytes

```python
bytes: int
```

Estimated scientific buffer bytes; input copies and scratch add overhead.
