---
title: "Python · PeakFitResult"
description: "PeakFitResult signatures, defaults and API explanations."
audience: user
pagefind: false
---

**Next API · unreleased.** This reference describes the source checkout. Compare with stable rexafs 0.2.12 before using it with an installed package.

[Installation and version guide](/docs/reference/) · [Python tutorial](/docs/libraries/python/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/main/py-rexafs/python/rexafs/__init__.pyi) · [Docstring source](https://github.com/Ameyanagi/rexafs/blob/main/py-rexafs/python/rexafs/__init__.pyi)

Owned native fit (since 0.2.10). Array getters return independent copies.
Result energies/centers are absolute eV; parameter values retain model coordinates.
Covariance/errors are conditional on the model/noise, not model-selection confidence.

## definition

```python
definition(self) -> PeakFit
```

Independent copy of the initial definition.

## fitted_model

```python
fitted_model(self) -> PeakFit
```

Copy fitted values for explicit reuse without changing the initial definition.

## parameters

```python
parameters(self) -> dict[str, float]
```

Named final values; parameter centers use the chosen model coordinates.

## parameter_errors

```python
parameter_errors(self) -> dict[str, float | None]
```

Named conditional local errors; None means unavailable or not independently estimated.

## components

```python
components(self) -> list[PeakContribution]
```

Component curves and summaries in model order; independent copies.

## to_json

```python
to_json(self) -> str
```

Full native result with initial/final constraints, masks and diagnostics.

## origin_ev

```python
origin_ev(self) -> float
```

Resolved energy origin in eV, added to parameter centers/reference energies.

## energy

```python
energy(self) -> NDArray[np.float64]
```

Absolute energy in eV, only the native points used by this fit.

## source_indices

```python
source_indices(self) -> list[int]
```

Original zero-based point indices; preserves masks and sampling provenance.

## data

```python
data(self) -> NDArray[np.float64]
```

Selected representation's measured values, in its signal units.

## model

```python
model(self) -> NDArray[np.float64]
```

Joint baseline + peaks + steps, in the same signal units.

## residual

```python
residual(self) -> NDArray[np.float64]
```

Unweighted data minus model, in signal units (also for weighted fits).

## standard_deviation

```python
standard_deviation(self) -> list[float] | None
```

Supplied selected-space standard deviations on the fitted points, if any.

## objective

```python
objective(self) -> float
```

Sum of squared residuals, divided by supplied standard deviations if present.

## points

```python
points(self) -> int
```

Number of fitted native data points (not EXAFS independent-point estimates).

## free_parameters

```python
free_parameters(self) -> int
```

Number of independent varying parameters; expression ties are excluded.

## degrees_of_freedom

```python
degrees_of_freedom(self) -> int
```

points − free_parameters. Fits with fewer points than variables are rejected.

## jacobian_rank

```python
jacobian_rank(self) -> int
```

Weighted numerical Jacobian rank under a 1e-10 relative singular-value cutoff.

## covariance_names

```python
covariance_names(self) -> list[str]
```

Sorted independent parameter names defining covariance/correlation axes.

## covariance

```python
covariance(self) -> list[list[float]] | None
```

Local covariance; absolute-error scaling when standard deviations were given,
otherwise multiplied by objective/degrees_of_freedom.

## correlation

```python
correlation(self) -> list[list[float]] | None
```

Dimensionless correlations corresponding to covariance_names. Absent when
any conditional variance is zero; the warning explains that case.

## uncertainty_unavailable

```python
uncertainty_unavailable(self) -> str | None
```

Why covariance/standard errors were withheld, rather than replaced by zero.

## peak_center_ev

```python
peak_center_ev(self) -> float | None
```

Model peak-area-weighted center in absolute eV, excluding baseline/steps.

## peak_center_standard_error_ev

```python
peak_center_standard_error_ev(self) -> float | None
```

Conditional error in that center, using full parameter covariance.

## termination

```python
termination(self) -> Literal['FixedModel', 'Converged', 'NotConverged', 'Cancelled']
```

Explicit numerical termination category.

## termination_detail

```python
termination_detail(self) -> str
```

Solver-specific termination detail, retained verbatim for diagnosis.

## evaluations

```python
evaluations(self) -> int
```

Number of residual-vector evaluations during optimization, including numerical derivatives.

## warnings

```python
warnings(self) -> list[str]
```

Active bounds and other model/uncertainty limitations.
