---
title: "Python · FluorescenceCorrection"
description: "FluorescenceCorrection signatures, defaults and API explanations."
audience: user
pagefind: true
---

**Stable 0.2.11.** These signatures match the released Python package. Explanations are maintained in the source docstrings and reviewed against this release.

[Installation and version guide](/docs/reference/) · [Python tutorial](/docs/libraries/python/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/v0.2.11/py-rexafs/python/rexafs/__init__.pyi) · [Docstring source](https://github.com/Ameyanagi/rexafs/blob/main/py-rexafs/python/rexafs/__init__.pyi)

Optically thick, homogeneous-sample XANES correction (since 0.2.10).

FluorescenceCorrection("CuO", "Cu", "K", line="Ka1", angles=(45,45))
requires the complete sample formula, absorber, edge, detected emission and
measured incident/exit angles. Angles are degrees FROM THE SAMPLE SURFACE,
each in (0,90]; geometry is never inferred. family=True selects an unresolved
within-shell family such as Ka. Internal conventional normalization runs
automatically (degree 1, no Victoreen term). Final normalization is separate.
The fluo_elam_v1 model is not qualified for EXAFS or finite-thickness samples.
See <https://xraypy.github.io/xraylarch/xafs_preedge.html#over-absorption-corrections.>

## FluorescenceCorrection

```python
FluorescenceCorrection(formula: str, element: str, edge: str, *, line: str, angles: tuple[float, float], family: bool=False, e0: float | None=None, pre_edge: tuple[float, float] | None=None, post_edge: tuple[float, float] | None=None, degree: int=1)
```

Copy settings. e0=None detects the edge. Internal pre/post ranges are
inclusive eV offsets from E0; None uses available low endpoint to -30 eV
and +100 eV to available high endpoint. Complete coverage is required.
degree=1 is the internal post-edge degree (0–5). No final normalization runs.

## apply

```python
apply(self, energy: NDArray[np.float64] | Sequence[float], mu: NDArray[np.float64] | Sequence[float]) -> FluorescenceCorrectionResult
```

Copy original unnormalized fluorescence arrays and release the GIL.
Energy must be positive, strictly increasing eV; arrays are matching finite
one-dimensional values. Output keeps original grid/units. Invalid geometry,
composition, coverage, fitted step or singular denominator raise ValueError;
nothing is clipped. Array uncertainty is unavailable. Prefer
Spectrum.correct_fluorescence to preserve restrictions in further processing.

## to_json

```python
to_json(self) -> str
```

Native settings JSON, including any pinned atomic identity.

## from_json

```python
from_json(json: str) -> FluorescenceCorrection
```

Restore settings; calculation checks scientific values and data availability.
