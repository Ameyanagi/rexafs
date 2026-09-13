---
title: "Python · AUTOBK"
description: "AUTOBK signatures, types and docstrings."
audience: user
pagefind: false
---


**Next API · unreleased.** These signatures describe the source checkout. They are not available from `pip install rexafs==0.2.4`.


[Installation and version guide](/docs/reference/) · [Python tutorial](/docs/libraries/python/)


AUTOBK background settings. Recommended defaults use LinearDirect and FixedPenalty with lambda 0.001. Settings are copied when assigned to a spectrum.

Original AUTOBK method: [Newville et al. (1993)](https://doi.org/10.1103/PhysRevB.47.14126). The fixed endpoint penalty is a rexafs-specific choice, not part of that original objective.


[Declaration source](https://github.com/Ameyanagi/rexafs/blob/main/py-rexafs/python/rexafs/__init__.pyi)



## AUTOBK


```python
__init__(self, *, ek0: float | None=None, rbkg: float | None=1.0, nknots: int | None=None, kmin: float | None=0.0, kmax: float | None=None, kstep: float | None=0.05, nclamp: int | None=3, clamp_lo: int | None=0, clamp_hi: int | None=1, clamp_lambda: float | None=0.001, nfft: int | None=2048, kweight: int | None=1, dk: float | None=0.1, linear_regularization: float | None=0.0001, linear_condition_limit: float | None=100000000.0, linear_residual_ratio_limit: float | None=1.05, linear_fallback_to_lm: bool | None=True, linear_workspace_cache: bool | None=True, window: FTWindow | None='Hanning', solver: AUTOBKSolver | None='LinearDirect', linear_fallback_solver: AUTOBKSolver | None='TrustRegionDogLeg', clamp_scale_policy: AUTOBKClampScalePolicy | None='FixedPenalty') -> None
```

Create settings; override only the parameters you need. Defaults match Rust new().


## ek0


```python
ek0: float | None
```

Edge energy in eV. Default: use normalization E0.


## rbkg


```python
rbkg: float | None
```

Background cutoff in angstroms. AUTOBK suppresses Fourier residuals below this R. Default: 1.0; increasing it can remove structural signal.


## nknots


```python
nknots: int | None
```

Spline knot count. Default: determine from rbkg and the k range.


## kmin


```python
kmin: float | None
```

Background fit lower k limit in inverse angstroms. Default: 0.0.


## kmax


```python
kmax: float | None
```

Background fit upper k limit in inverse angstroms. Default: available data limit.


## kstep


```python
kstep: float | None
```

Uniform output k spacing in inverse angstroms. Default: 0.05.


## nclamp


```python
nclamp: int | None
```

Number of samples at each endpoint used by the clamp. Default: 3; 0 disables clamping.


## clamp_lo


```python
clamp_lo: int | None
```

Low-k endpoint weight. Default: 0 (disabled).


## clamp_hi


```python
clamp_hi: int | None
```

High-k endpoint weight. Default: 1.


## clamp_lambda


```python
clamp_lambda: float | None
```

FixedPenalty strength. Recommended default: 0.001; 0 disables the endpoint penalty.


## nfft


```python
nfft: int | None
```

FFT length for background removal. Default: 2048.


## kweight


```python
kweight: int | None
```

Power of k used in the background objective. Default: 1.


## dk


```python
dk: float | None
```

Background window taper width in inverse angstroms. Default: 0.1.


## linear_regularization


```python
linear_regularization: float | None
```

Legacy direct-solver ridge strength. Default: 0.0001; unused by FixedPenalty.


## linear_condition_limit


```python
linear_condition_limit: float | None
```

Maximum accepted linear-system condition number. Default: 1e8.


## linear_residual_ratio_limit


```python
linear_residual_ratio_limit: float | None
```

Legacy direct-solver residual acceptance ratio. Default: 1.05; unused by FixedPenalty.


## linear_fallback_to_lm


```python
linear_fallback_to_lm: bool | None
```

Allow legacy solver fallback. Default: True; FixedPenalty never falls back.


## linear_workspace_cache


```python
linear_workspace_cache: bool | None
```

Reuse compatible spline/FFT geometry and SVD factors. Default: True; each spectrum has a new right-hand side and solution.


## window


```python
window: FTWindow | None
```

Background Fourier window. Default: Hanning.


## solver


```python
solver: AUTOBKSolver | None
```

Background solver. Recommended default: LinearDirect, required by FixedPenalty. TrustRegionDogLeg requires a Rust build with trust-region (included in Python, unavailable in Wasm).


## linear_fallback_solver


```python
linear_fallback_solver: AUTOBKSolver | None
```

Legacy fallback solver. Default: TrustRegionDogLeg in Python, LegacyLm in Wasm; unused by FixedPenalty.


## clamp_scale_policy


```python
clamp_scale_policy: AUTOBKClampScalePolicy | None
```

Endpoint model. Recommended default: FixedPenalty with LinearDirect; Fixed and TwoPass are legacy models.
