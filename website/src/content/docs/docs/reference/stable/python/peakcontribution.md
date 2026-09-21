---
title: "Python · PeakContribution"
description: "PeakContribution signatures, defaults and API explanations."
audience: user
pagefind: true
---

**Stable 0.2.13.** These signatures match the released Python package. Explanations are maintained in the source docstrings and reviewed against this release.

[Installation and version guide](/docs/reference/) · [Python tutorial](/docs/libraries/python/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/v0.2.13/py-rexafs/python/rexafs/__init__.pyi) · [Docstring source](https://github.com/Ameyanagi/rexafs/blob/main/py-rexafs/python/rexafs/__init__.pyi)

One component curve and derived metrics (since 0.2.10). Curve getters return copies.

## name

```python
name(self) -> str
```

Stable component identity from the initial definition.

## role

```python
role(self) -> Literal['Peak', 'Baseline', 'Edge']
```

Scientific role, independent of mathematical shape.

## shape

```python
shape(self) -> Literal['Gaussian', 'Lorentzian', 'PseudoVoigt', 'Voigt', 'ErfStep', 'ArctanStep', 'Constant', 'Linear']
```

Mathematical shape used for evaluation.

## curve

```python
curve(self) -> NDArray[np.float64]
```

Component values at the result's absolute-energy points, in signal units.

## center_ev

```python
center_ev(self) -> float | None
```

Peak/step center in absolute eV; None for polynomial baselines.

## area

```python
area(self) -> float | None
```

Whole-axis model area in signal units × eV; None for steps/polynomials.

## height

```python
height(self) -> float | None
```

Peak contribution at its center, excluding all other components.

## fwhm_ev

```python
fwhm_ev(self) -> float | None
```

Peak FWHM in eV; true Voigt uses a numerical half-height root.

## center_standard_error_ev

```python
center_standard_error_ev(self) -> float | None
```

Conditional errors propagated with the full joint covariance; absent when
local uncertainty is unavailable or the quantity does not apply.

## area_standard_error

```python
area_standard_error(self) -> float | None
```

Conditional whole-axis area error, in signal units × eV.

## height_standard_error

```python
height_standard_error(self) -> float | None
```

Conditional peak-height error, in signal units.

## fwhm_standard_error_ev

```python
fwhm_standard_error_ev(self) -> float | None
```

Conditional FWHM error, in eV, including both true-Voigt width parameters.

## sampled_integral

```python
sampled_integral(self) -> float
```

Trapezoidal component integral over included native-grid segments only.
Masked gaps are not bridged; this is not its whole-axis analytic area.
