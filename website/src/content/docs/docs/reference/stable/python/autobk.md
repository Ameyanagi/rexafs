---
title: "Python · AUTOBK"
description: "AUTOBK signatures, defaults and API explanations."
audience: user
pagefind: true
---

**Stable 0.2.5.** These signatures match the released Python package. Explanations are maintained in the source docstrings and reviewed against this release.

[Installation and version guide](/docs/reference/) · [Python tutorial](/docs/libraries/python/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/v0.2.5/py-rexafs/python/rexafs/__init__.pyi) · [Docstring source](https://github.com/Ameyanagi/rexafs/blob/main/py-rexafs/python/rexafs/__init__.pyi)

Fit a smooth atomic background and extract the EXAFS oscillation chi(k).

AUTOBK fits a cubic spline in k to suppress low-R Fourier content of the
background-subtracted signal. Subtracting that background and dividing
by the normalization edge step gives dimensionless chi. rbkg controls the
low-R region treated as background; a value that reaches the first
structural shell can remove the signal you want to measure.

Recommended starting values are rbkg=1.0 angstrom, kstep=0.05 inverse
angstroms, kweight=1, window="Hanning", solver="LinearDirect", and
clamp_scale_policy="FixedPenalty" with clamp_lambda=0.001. The fixed
penalty discourages large endpoint oscillations; its strength is a
rexafs convention, not a universal statistical regularization parameter.
None resolves optional fields when processing, as described per field.

Example: p = AUTOBK(); p.rbkg = 1.2. Assign p to the spectrum's background
stage, then call calc_background() or a later stage.
Settings are copied; assigning them again is required after later edits.
Invalid ranges, an incompatible solver, or a failed solve raise RuntimeError
during processing. A solved background still needs scientific inspection.

Original AUTOBK method: [Newville et al. (1993)](https://doi.org/10.1103/PhysRevB.47.14126).
The fixed endpoint penalty and linear solution are rexafs-specific choices;
see the [implemented AUTOBK objective](https://rexafs.com/docs/science/autobk/).

Resolved scalar defaults and ek0 are retained inside the spectrum.
Automatic kmax and nknots remain unset in the settings and are calculated
locally from the current input on each background call.
Processing does not replace None fields in your original settings object.
Reassign fresh or reset settings when you want retained automatic choices
recalculated after changing the data or an earlier stage.

## AUTOBK

```python
AUTOBK(*, ek0: float | None=None, rbkg: float | None=1.0, nknots: int | None=None, kmin: float | None=0.0, kmax: float | None=None, kstep: float | None=0.05, nclamp: int | None=3, clamp_lo: int | None=0, clamp_hi: int | None=1, clamp_lambda: float | None=0.001, nfft: int | None=2048, kweight: int | None=1, dk: float | None=0.1, linear_regularization: float | None=0.0001, linear_condition_limit: float | None=100000000.0, linear_residual_ratio_limit: float | None=1.05, linear_fallback_to_lm: bool | None=True, linear_workspace_cache: bool | None=True, window: FTWindow | None='Hanning', solver: AUTOBKSolver | None='LinearDirect', linear_fallback_solver: AUTOBKSolver | None='TrustRegionDogLeg', clamp_scale_policy: AUTOBKClampScalePolicy | None='FixedPenalty')
```

Create AUTOBK settings with the recommended Rust defaults.

Begin with rbkg=1.0 angstrom and the LinearDirect/FixedPenalty pair.
Edit only the fields your data require, then assign the settings to the
spectrum's background stage. Construction does not fit a spectrum;
numeric range and solver compatibility checks occur during processing.

Keyword arguments were added in 0.2.5. Published
0.2.4 settings use construction without arguments followed by field
assignment. Python type conversion can raise TypeError, and an integer
outside the native field's representable range can raise OverflowError
before any numerical processing.

## ek0

```python
ek0: float | None
```

Edge energy used to convert energy to k, in eV.

Default None uses the normalization E0. Above the edge, k is approximately
sqrt((E - E0) / 3.81), with E in eV and k in inverse
angstroms; the conversion constant has units eV * angstrom**2.
Prefer Spectrum.set_e0() when changing the edge for all stages
together; an independent background edge can differ from normalization.

## rbkg

```python
rbkg: float | None
```

Positive background cutoff in angstroms. Default: 1.0; None resolves to 1.0.

AUTOBK suppresses low-R Fourier residuals up to a cutoff derived from this
value, the k range, and the discrete R grid; it is not an exact continuous
boundary. Increasing rbkg allows a more flexible spline and can remove
real short-distance structure. Start near 1.0 and keep it below the first
physical shell of interest.

## nknots

```python
nknots: int | None
```

Requested number of spline coefficients/anchor points.

Default None uses 1 + floor(2 * rbkg * (kmax - kmin) / pi), clamped to
5 through 128; explicit values are clamped to the same range. rbkg is in
angstroms and k limits in inverse angstroms, so the count is dimensionless.
More coefficients increase background flexibility. Leave this automatic
unless testing a justified spline model.

## kmin

```python
kmin: float | None
```

Lower k bound of the background fit and Fourier window, in inverse
angstroms.

Default: 0.0. None also resolves to 0.0. Raising it excludes low-k data from
the Fourier objective and changes the spline geometry. It must be below the
effective kmax. The returned k() array still starts at zero.

## kmax

```python
kmax: float | None
```

Upper k bound of the background fit, in inverse angstroms.

Default None uses the available data limit; an explicit larger value is
clipped to that limit. Lower it to exclude a noisy high-k tail. This also
changes the spline count and the extent of the returned k()/chi() arrays;
it is independent of the later XrayFFTF.kmax setting.

## kstep

```python
kstep: float | None
```

Spacing of the uniform output k grid, in inverse angstroms.

Default: 0.05. None also resolves to 0.05. Values must be finite and
positive. The grid begins at zero and is used by Spectrum.k() and chi().
Smaller steps produce more interpolated samples without adding measured
information. This usually supplies the automatic spacing of the later
forward transform.

For FixedPenalty, the internal objective FFT uses a fixed amplitude
reference of 0.05 / sqrt(pi); kstep still sets the physical R spacing
and low-R cutoff. Legacy clamp policies use kstep / sqrt(pi) instead.
This internal convention is separate from the public forward transform.
If that transform has already run, assign fresh XrayFFTF settings when
changing this step so its automatic spacing is resolved again.

## nclamp

```python
nclamp: int | None
```

Number of endpoint samples included at each enabled end of the clamp.

Default: 3. None also resolves to 3; 0 disables endpoint penalties.
FixedPenalty uses the first and last nclamp samples, limited to the
available length, and requires a nonnegative value. Increasing the count
constrains a wider endpoint region. clamp_lo/clamp_hi select which ends
contribute.

## clamp_lo

```python
clamp_lo: int | None
```

Relative weight of the low-k endpoint penalty. Default: 0. None also
resolves to 0.

Zero disables this end. For FixedPenalty, the absolute integer weight
multiplies the endpoint residual before squaring; doubling it gives four
times the squared contribution for a fixed residual. Start at zero so the
near-edge oscillation is not forced toward zero.

## clamp_hi

```python
clamp_hi: int | None
```

Relative weight of the high-k endpoint penalty. Default: 1. None also
resolves to 1.

Zero disables this end. For FixedPenalty, the absolute integer weight
multiplies endpoint residuals before squaring. Larger values discourage
large high-k endpoint oscillations more strongly but may bias a real
oscillation; clamp_lambda controls the overall penalty strength.

## clamp_lambda

```python
clamp_lambda: float | None
```

Strength of the FixedPenalty mean-square endpoint term. Default: 0.001.

None resolves to 0.001; 0 disables both endpoint penalties regardless of
their weights. Values must be finite and nonnegative. Increasing this
value favors smaller endpoint oscillations over the low-R objective.
Its numerical meaning depends on the implemented Fourier scaling and
weights; it is not an uncertainty estimate. Unused by Fixed and TwoPass.

FixedPenalty evaluates its low-R residual with an internal FFT factor
of 0.05 / sqrt(pi). Changing the background k weight or window changes
the numerical balance against the endpoint penalty, even with unchanged
lambda. See the [implemented objective](https://rexafs.com/docs/science/autobk/).

## nfft

```python
nfft: int | None
```

Number of samples in the FFT used inside background removal.

Default: 2048. None also resolves to 2048. Use a positive length large
enough to contain the output k grid: the underlying FFT otherwise keeps only
its first nfft samples. A larger length makes the R grid finer through zero
padding; it does not improve experimental resolution. This setting is
independent of the later XrayFFTF.nfft.

## kweight

```python
kweight: int | None
```

Integer exponent of k in the background Fourier objective. Default: 1.

None resolves to 1. Larger nonnegative weights emphasize higher-k
oscillations and their noise while the spline is fitted. Use nonnegative
values on the zero-origin grid; negative powers are singular at k=0.
This weight does not remain in the returned chi() and is independent
of the exponent used by the later forward transform.

## dk

```python
dk: float | None
```

Window taper parameter for the background objective. Default: 0.1.

None resolves to 0.1. For the default Hanning window this sets the taper
width at each k bound, in inverse angstroms. A broader taper softens
truncation but reduces the strongly weighted range. Other window families
interpret the parameter differently; KaiserBessel also uses it as a
shape parameter. Inspect the window when changing families.

## linear_regularization

```python
linear_regularization: float | None
```

Ridge strength for legacy Fixed/TwoPass direct solves. Default: 0.0001.

None resolves to 0.0001. It adds a scaled diagonal regularization to the
legacy normal equations, trading closeness to the unregularized solution
for stability. FixedPenalty does not use this parameter; changing it
does not regularize the recommended column-scaled SVD solve.

## linear_condition_limit

```python
linear_condition_limit: float | None
```

Largest accepted condition measure for a direct solve. Default: 1e8.

None resolves to 1e8. FixedPenalty requires a finite value at least 1
and checks the largest/smallest singular-value ratio of its column-scaled
design matrix. Legacy solves use a different condition proxy. Lowering
the limit rejects more unstable systems; raising it can accept sensitive
solutions. FixedPenalty failures raise RuntimeError without a fallback.

## linear_residual_ratio_limit

```python
linear_residual_ratio_limit: float | None
```

Maximum solved/base residual-norm ratio for legacy direct solves.

Default: 1.05. None also resolves to 1.05; legacy processing clamps it to at
least 1. Exceeding the limit rejects the direct solution and may trigger the
configured fallback. FixedPenalty does not use this legacy acceptance test;
its rank, condition and stationarity checks are separate.

## linear_fallback_to_lm

```python
linear_fallback_to_lm: bool | None
```

Allow a rejected legacy direct solve to use linear_fallback_solver.

Default: True. None also resolves to True. Despite its historical name, the
selected fallback need not be Levenberg-Marquardt. False makes a rejected
legacy direct solve raise an error. FixedPenalty never falls back: it
returns a failed solve as RuntimeError regardless of this setting.

## linear_workspace_cache

```python
linear_workspace_cache: bool | None
```

Reuse compatible background-solver geometry and factorization. Default: True.

None resolves to True. FixedPenalty caches spline/FFT design matrices,
column scaling and singular-value decomposition factors for matching
geometry; each spectrum still supplies its own data and receives a new
solution. False disables this reuse, mainly for comparisons or profiling;
the cache does not reuse a previous spectrum's chi().

## window

```python
window: FTWindow | None
```

Fourier window used inside the background objective. Default: "Hanning".

None selects Hanning. A window reduces artifacts from abrupt k truncation;
its shape changes the objective and can change the extracted background.
Accepted case-sensitive names are Hanning, Parzen, Welch, Gaussian,
Sine, KaiserBessel and FHanning. See the
[window reference](https://xraypy.github.io/xraylarch/xafs_fourier.html#ftwindow)
for the shape-dependent parameter conventions.
An unsupported name raises ValueError when assigned.

## solver

```python
solver: AUTOBKSolver | None
```

Solver used for the spline coefficients. Recommended default: "LinearDirect".

None resolves to LinearDirect, which is required by FixedPenalty.
TrustRegionDogLeg and LegacyLm are iterative solvers for legacy objectives;
select Fixed or TwoPass explicitly to use them. TrustRegionDogLeg requires
the trust-region Rust feature, included in Python packages. Unknown names
raise ValueError; incompatible solver/objective pairs fail during processing.

## linear_fallback_solver

```python
linear_fallback_solver: AUTOBKSolver | None
```

Solver used after a rejected legacy LinearDirect solve.

The Python constructor default is "TrustRegionDogLeg". Explicit None
normally resolves to "LegacyLm" when fallback is enabled; it is therefore
different from leaving the constructor argument unchanged. LinearDirect
cannot be its own fallback. FixedPenalty ignores this setting and never
falls back. Unknown names raise ValueError when assigned.

## clamp_scale_policy

```python
clamp_scale_policy: AUTOBKClampScalePolicy | None
```

Endpoint model for background removal. Recommended default: "FixedPenalty".

None resolves to FixedPenalty, which requires LinearDirect and uses
clamp_lambda as a fixed mean-square penalty strength. Fixed and TwoPass
retain older residual-dependent clamp models; their results need not
match the recommended objective. See the [AUTOBK objective](https://rexafs.com/docs/science/autobk/) for
the distinction. Unknown names raise ValueError when assigned.
