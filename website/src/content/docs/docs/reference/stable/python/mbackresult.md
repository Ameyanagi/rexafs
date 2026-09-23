---
title: "Python · MbackResult"
description: "MbackResult signatures, defaults and API explanations."
audience: user
pagefind: true
---

**Stable 0.2.14.** These signatures match the released Python package. Explanations are maintained in the source docstrings and reviewed against this release.

[Installation and version guide](/docs/reference/) · [Python tutorial](/docs/libraries/python/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/v0.2.14/py-rexafs/python/rexafs/__init__.pyi) · [Docstring source](https://github.com/Ameyanagi/rexafs/blob/main/py-rexafs/python/rexafs/__init__.pyi)

Owned full-MBACK output (since 0.2.10). Arrays are returned as independent copies.

norm=(scale*mu-pre_curve)/Delta is dimensionless; fpp=scale*mu-background
remains in f2 units. flat separately removes the auxiliary post-edge trend.
objective uses balanced pre/post sample counts, not inverse measurement
variances. Inspect warnings/condition and resolved ranges. No covariance or
experimental confidence interval is implied. to_json retains all provenance.

## to_json

```python
to_json(self) -> str
```

Complete result JSON, including reference identity, settings, weights and curves.

## definition

```python
definition(self) -> MBack
```

Immutable replay definition pinned to the original table.

Requested automatic E0/ranges remain automatic; explicit settings remain explicit.

## reference

```python
reference(self) -> AtomicReference
```

Independent dictionary with provider, data checksum and table identity.

## reference_json

```python
reference_json(self) -> str
```

Reference identity JSON, including the actual data checksum.

## pre_edge

```python
pre_edge(self) -> tuple[float, float]
```

Resolved inclusive pre-edge offsets in eV from E0.

## post_edge

```python
post_edge(self) -> tuple[float, float]
```

Resolved inclusive post-edge offsets in eV from E0.

## fit_indices

```python
fit_indices(self) -> list[int]
```

Original zero-based indices included in the objective.

## warnings

```python
warnings(self) -> list[str]
```

Nonfatal boundary/conditioning diagnostics. Empty does not establish physical validity.

## erfc_width

```python
erfc_width(self) -> float | None
```

Positive erfc width in eV, or None when disabled.

## e0

```python
e0(self) -> float
```

Resolved fixed energy origin in eV.

## edge_step

```python
edge_step(self) -> float
```

Positive fitted absorption step in input mu units.

## scale

```python
scale(self) -> float
```

Positive conversion from input absorption to f2.

## objective

```python
objective(self) -> float
```

Sum of squared region-balanced residuals, in squared f2 units.

## condition

```python
condition(self) -> float
```

Condition number of the column-scaled final Jacobian.

## rank

```python
rank(self) -> int
```

Rank of the final Jacobian, including erfc width when enabled.

## evaluations

```python
evaluations(self) -> int
```

Number of linear solves used by the fit.

## erfc_amplitude

```python
erfc_amplitude(self) -> float
```

Fitted erfc amplitude in f2 units; zero when disabled.

## energy_scale

```python
energy_scale(self) -> float
```

Polynomial coordinate scale in eV.

## energy

```python
energy(self) -> NDArray[np.float64]
```

Original energy grid in eV. Returns a copy.

## f2

```python
f2(self) -> NDArray[np.float64]
```

Unshifted atomic scattering factor, in electron units. Returns a copy.

## fpp

```python
fpp(self) -> NDArray[np.float64]
```

Matched scale*mu-background, in electron units; distinct from norm. Returns a copy.

## norm

```python
norm(self) -> NDArray[np.float64]
```

Dimensionless normalized absorption. Returns a copy.

## flat

```python
flat(self) -> NDArray[np.float64]
```

Dimensionless flattened absorption using the auxiliary post-edge trend. Returns a copy.

## background

```python
background(self) -> NDArray[np.float64]
```

Fitted smooth background in f2 units. Returns a copy.

## pre_curve

```python
pre_curve(self) -> NDArray[np.float64]
```

Auxiliary pre-edge line on f2+background, in f2 units. Returns a copy.

## post_curve

```python
post_curve(self) -> NDArray[np.float64]
```

Auxiliary post-edge curve on f2+background, in f2 units. Returns a copy.

## residual

```python
residual(self) -> NDArray[np.float64]
```

Unweighted f2+background-scale*mu on every energy point. Returns a copy.

## coefficients

```python
coefficients(self) -> NDArray[np.float64]
```

Increasing polynomial powers of (energy-e0)/energy_scale, in f2 units. Returns a copy.

## weights

```python
weights(self) -> NDArray[np.float64]
```

1/sqrt(region sample count), in fit_indices order. Returns a copy.
