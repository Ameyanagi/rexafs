---
title: "Python · MBack"
description: "MBack signatures, defaults and API explanations."
audience: user
pagefind: true
---

**Stable 0.2.12.** These signatures match the released Python package. Explanations are maintained in the source docstrings and reviewed against this release.

[Installation and version guide](/docs/reference/) · [Python tutorial](/docs/libraries/python/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/v0.2.12/py-rexafs/python/rexafs/__init__.pyi) · [Docstring source](https://github.com/Ameyanagi/rexafs/blob/main/py-rexafs/python/rexafs/__init__.pyi)

Full Chantler MBACK normalization (since 0.2.10). Example:
MBack("Cu", "K", pre_edge=(-200, -50), post_edge=(100, 800)).

Ranges are eV offsets from E0. Degree defaults to 2; erfc is disabled. E0=None
uses the derivative detector. Automatic ranges use the outer 80% of measured
pre/post spans with neighboring-edge limits; inspect resolved result ranges.
The offline atomic table is loaded automatically and never energy shifted.
fit(energy, mu) leaves both inputs/model unchanged and returns owned norm/flat
arrays and full diagnostics. set_normalization_method(model) copies settings
into a Spectrum; normalize() invalidates its dependent background/FFT results.
Invalid ranges, unsupported data, nonidentifiability and nonpositive scale/step
raise ValueError. No experimental uncertainty is inferred from fit weights.

## MBack

```python
MBack(element: str, edge: str, *, e0: float | None=None, pre_edge: tuple[float, float] | None=None, post_edge: tuple[float, float] | None=None, degree: int=2, erfc: MbackErfc | None=None)
```

Create immutable settings; erfc requires an explicit MbackErfc object.

## fit

```python
fit(self, energy: NDArray[np.float64] | Sequence[float], mu: NDArray[np.float64] | Sequence[float]) -> MbackResult
```

Fit finite 1D arrays (energy eV, raw absorption), leaving inputs unchanged.

Copies input buffers and releases the GIL. Returns separate dimensionless
norm/flat and matched fpp in electron units. Invalid scientific inputs raise ValueError.

## to_json

```python
to_json(self) -> str
```

Serialize native settings without adding input arrays.

## from_json

```python
from_json(json: str) -> MBack
```

Restore native settings; fit validates scientific values and reference identity.
