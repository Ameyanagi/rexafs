---
title: "Python · PrePostEdge"
description: "PrePostEdge signatures, types and docstrings."
audience: user
pagefind: false
---


**Next API · unreleased.** These signatures describe the source checkout. They are not available from `pip install rexafs==0.2.4`.


[Installation and version guide](/docs/reference/) · [Python tutorial](/docs/libraries/python/)


Pre/post-edge normalization settings. Defaults adapt to the measured energy range. Settings are copied when assigned to a spectrum.

For measurement conventions, see [Newville, Fundamentals of XAFS](https://docs.xrayabsorption.org/tutorials/XAFS_Fundamentals.pdf).


[Declaration source](https://github.com/Ameyanagi/rexafs/blob/main/py-rexafs/python/rexafs/__init__.pyi)



## PrePostEdge


```python
__init__(self, *, pre_edge_start: float | None=None, pre_edge_end: float | None=None, norm_start: float | None=None, norm_end: float | None=None, norm_polyorder: int | None=None, n_victoreen: int | None=None, e0: float | None=None, edge_step: float | None=None) -> None
```

Create settings; override only the parameters you need. Defaults match Rust new().


## pre_edge_start


```python
pre_edge_start: float | None
```

Pre-edge fit start relative to E0, in eV. Default: infer from the measured range.


## pre_edge_end


```python
pre_edge_end: float | None
```

Pre-edge fit end relative to E0, in eV. Default: infer from the pre-edge start.


## norm_start


```python
norm_start: float | None
```

Post-edge fit start relative to E0, in eV. Default: infer from the available range (at most 25 eV).


## norm_end


```python
norm_end: float | None
```

Post-edge fit end relative to E0, in eV. Default: measured upper energy limit.


## norm_polyorder


```python
norm_polyorder: int | None
```

Post-edge polynomial degree, 0 through 5. Default: 0, 1 or 2 for fit spans below 50, below 350, or at least 350 eV.


## n_victoreen


```python
n_victoreen: int | None
```

Victoreen energy exponent for the pre-edge fit. Default: 0.


## e0


```python
e0: float | None
```

Edge energy in eV. Default: detect from the spectrum.


## edge_step


```python
edge_step: float | None
```

Absorption edge-step override in mu units. Default: estimate from the fitted baselines.
