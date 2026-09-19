---
title: "Python · PeakFit"
description: "PeakFit signatures, defaults and API explanations."
audience: user
pagefind: true
---

**Stable 0.2.11.** These signatures match the released Python package. Explanations are maintained in the source docstrings and reviewed against this release.

[Installation and version guide](/docs/reference/) · [Python tutorial](/docs/libraries/python/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/v0.2.11/py-rexafs/python/rexafs/__init__.pyi) · [Docstring source](https://github.com/Ameyanagi/rexafs/blob/main/py-rexafs/python/rexafs/__init__.pyi)

Immutable composite XANES peak definition (since 0.2.10).

PeakFit((-20, 40)).gaussian("p1", 5, 2, 3).linear_baseline(0, 0)
starts with Norm and E0-relative eV. Each builder returns a NEW model.
Peak arguments are center, whole-axis area (signal units times eV), and
FWHM (eV). Default bounds keep centers in the fit range, areas nonnegative,
and widths positive. Missing normalization runs on a copy; inputs stay intact.
No smoothing or automatic chemical/component-count assignment is performed.
Inspect termination and warnings; covariance is conditional on the chosen model.

## PeakFit

```python
PeakFit(range: tuple[float, float])
```

Create an empty Norm model over inclusive E0-relative eV. Add components before fitting.

## flat

```python
flat(self) -> PeakFit
```

Use dimensionless flattened mu; prerequisites run on a copy. Returns a new model.

## raw_mu

```python
raw_mu(self) -> PeakFit
```

Use the original mapped absorption signal and its units. Returns a new model.

## absolute

```python
absolute(self) -> PeakFit
```

Interpret ranges, centers and baseline references as absolute eV. Returns a new model.

## reference

```python
reference(self, energy_ev: float) -> PeakFit
```

Use offsets from this fixed reference energy in eV. Returns a new model.

## gaussian

```python
gaussian(self, name: str, center: float, area: float, fwhm: float) -> PeakFit
```

Add a Gaussian: center/FWHM in eV, whole-axis area in signal units times eV. Returns a new model.

## lorentzian

```python
lorentzian(self, name: str, center: float, area: float, fwhm: float) -> PeakFit
```

Add a Lorentzian with whole-axis area and FWHM in eV. Returns a new model.

## pseudo_voigt

```python
pseudo_voigt(self, name: str, center: float, area: float, fwhm: float, fraction: float) -> PeakFit
```

Add a common-FWHM mixture; fraction is the Lorentzian share from zero to one. Returns a new model.

## voigt

```python
voigt(self, name: str, center: float, area: float, gaussian_fwhm: float, lorentzian_fwhm: float) -> PeakFit
```

Add a true Voigt with independent Gaussian/Lorentzian FWHM in eV. Returns a new model.

## erf_step

```python
erf_step(self, name: str, center: float, height: float, scale: float) -> PeakFit
```

Add height*(1+erf((E-center)/scale))/2; scale is positive eV. Returns a new model.

## arctan_step

```python
arctan_step(self, name: str, center: float, height: float, scale: float) -> PeakFit
```

Add height*(1/2+atan((E-center)/scale)/pi); scale is positive eV. Returns a new model.

## constant_baseline

```python
constant_baseline(self, offset: float=0) -> PeakFit
```

Add a fitted constant named baseline, in signal units. Returns a new model.

## linear_baseline

```python
linear_baseline(self, offset: float=0, slope: float=0) -> PeakFit
```

Add baseline = offset+slope*E_offset; slope is signal units/eV. Returns a new model.

## exclude

```python
exclude(self, range: tuple[float, float]) -> PeakFit
```

Exclude an inclusive interval in model coordinates. Masked gaps are not integrated.

## parameter

```python
parameter(self, name: str, value: float, *, vary: bool=True, bounds: tuple[float | None, float | None]=(None, None), expression: str | None=None) -> PeakFit
```

Replace an EXISTING parameter, returning a new model.

Names use component_parameter, for example p1_center, p1_area, p1_width.
Bounds default to unbounded; pass them explicitly to retain restrictions.
An expression is a restricted tie, not executable code; it overrides vary.
Dependencies, physical domains and finite values are checked at fit/evaluation.
Unknown names raise ValueError immediately. Use vary=False to fix a value.

## as_baseline

```python
as_baseline(self, name: str) -> PeakFit
```

Make a named peak part of the baseline, excluding it from the area-weighted center.
Shapes are unchanged; steps must remain edges. Returns a new model.

## solver

```python
solver(self, *, max_iterations: int=200, tolerance: float=1e-10) -> PeakFit
```

Set positive iteration/tolerance limits on a new model; validated when fitting.

## evaluate

```python
evaluate(self, energy: NDArray[np.float64] | Sequence[float], *, e0: float | None=None) -> NDArray[np.float64]
```

Evaluate without fitting or masking at absolute energy in eV.
E0-relative models require e0 in eV. Returns a new NumPy array;
invalid definitions and nonfinite arrays raise ValueError.

## initialize_baseline

```python
initialize_baseline(self, spectrum: Spectrum, peak_intervals: Sequence[tuple[float, float]]) -> PeakFit
```

Initialize only baseline-role variables outside the given peak intervals.
Intervals use model coordinates; final masks and input spectra stay unchanged.
Returns a new starting model for a joint final fit. Rust releases the GIL.

## fit_batch

```python
fit_batch(self, spectra: Sequence[Spectrum]) -> list[PeakFitOutcome]
```

Independent unweighted fits from this frozen start, one outcome per input.
Bad frames keep an error row and do not stop later frames. Inputs are unchanged.
Rust releases the GIL. Use spectrum.fit_peaks for supplied point errors.

## to_json

```python
to_json(self) -> str
```

Serialize the complete initial definition, constraints and masks.

## from_json

```python
from_json(json: str) -> PeakFit
```

Restore and validate a complete definition; invalid input raises ValueError.
