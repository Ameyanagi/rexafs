---
title: "Python · FluorescenceCorrectionResult"
description: "FluorescenceCorrectionResult signatures, defaults and API explanations."
audience: user
pagefind: true
---

**Stable 0.2.13.** These signatures match the released Python package. Explanations are maintained in the source docstrings and reviewed against this release.

[Installation and version guide](/docs/reference/) · [Python tutorial](/docs/libraries/python/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/v0.2.13/py-rexafs/python/rexafs/__init__.pyi) · [Docstring source](https://github.com/Ameyanagi/rexafs/blob/main/py-rexafs/python/rexafs/__init__.pyi)

Historical correction (since 0.2.10), with independent arrays and dictionaries.
Later spectrum edits do not rewrite this record. Inspect amplification/warnings;
a finite result does not prove physical validity. No uncertainty is claimed.

## energy

```python
energy(self) -> NDArray[np.float64]
```

Original measured energy in eV, without resampling.

## original_mu

```python
original_mu(self) -> NDArray[np.float64]
```

Original uncorrected absorption, in supplied units.

## corrected_mu

```python
corrected_mu(self) -> NDArray[np.float64]
```

Corrected absorption, same grid/units; final normalization is separate.

## factor

```python
factor(self) -> NDArray[np.float64]
```

Dimensionless alpha/denominator, without clipping.

## denominator

```python
denominator(self) -> NDArray[np.float64]
```

Dimensionless alpha+1-internal_norm, without clipping.

## method

```python
method(self) -> str
```

Named convention, fluo_elam_v1.

## input_mode

```python
input_mode(self) -> AbsorptionMode
```

Original acquisition interpretation; unknown records a caller assumption.

## alpha

```python
alpha(self) -> float
```

Dimensionless attenuation/geometry constant.

## geometry_ratio

```python
geometry_ratio(self) -> float
```

sin(incidence)/sin(exit) using measured surface angles; dimensionless.

## minimum_denominator

```python
minimum_denominator(self) -> float
```

Smallest dimensionless denominator on the whole input grid.

## maximum_amplification

```python
maximum_amplification(self) -> float
```

Largest dimensionless factor; high values amplify noise.

## singularity_threshold

```python
singularity_threshold(self) -> float
```

Numerical rejection limit, 64*epsilon*max(1, alpha+1).

## warnings

```python
warnings(self) -> list[str]
```

Domain, interpretation and numerical diagnostics; not confidence intervals.

## definition

```python
definition(self) -> FluorescenceCorrection
```

Independent settings pinned to resolved ranges/E0 and atomic data.

## internal

```python
internal(self) -> FluorescenceInternalNormalization
```

Internal conventional fit of original mu, with independent Python lists.

## atomic

```python
atomic(self) -> dict[str, object]
```

Edge, emission and compound attenuation records: energies (eV), mass
fractions, cm²/g values, table identities and checksums.

## to_json

```python
to_json(self) -> str
```

Full native record with original inputs, assumptions and atomic evidence.
