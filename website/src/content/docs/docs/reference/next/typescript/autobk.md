---
title: "TypeScript · AUTOBK"
description: "AUTOBK declarations, defaults and API explanations."
audience: user
pagefind: false
---

**Next API · unreleased.** This reference describes the source checkout. Compare with stable npm rexafs@0.2.14 before using it with an installed package.

[Installation and version guide](/docs/reference/) · [TypeScript tutorial](/docs/libraries/typescript/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts) · [JSDoc source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts)

Settings for extracting extended X-ray absorption fine structure, chi(k), with a cubic
spline background in photoelectron wavenumber k.

AUTOBK separates slowly varying atomic absorption from oscillations associated with
neighboring atoms by suppressing low-R Fourier residuals. Recommended starting values are
rbkg=1 angstrom, kstep=0.05 inverse angstroms, kweight=1, Hanning, and LinearDirect with
FixedPenalty and clamp_lambda=0.001. Choose rbkg below structural signal; excessive
background flexibility can remove that signal.

The original method is [Newville et al. (1993)](https://doi.org/10.1103/PhysRevB.47.14126).
The fixed endpoint penalty and its default strength are rexafs-specific choices, implemented
in
[background/fixed.rs](https://github.com/Ameyanagi/rexafs/blob/main/crates/rexafs/src/xafs/background/fixed.rs);
they are not part of that paper's objective.

The spectrum copies assigned background settings. Reassign after editing, and release your
settings with free() when finished. A processing failure, including an ill-conditioned
fixed-penalty fit, throws an Error without silently switching objectives.

## constructor

```typescript
constructor(options?: AUTOBKOptions);
```

Create owned settings with the recommended defaults described below. Browser callers must
await init() first. Edit the fields, copy the settings into the appropriate spectrum stage,
and free() this object when finished.

Automatic fields are resolved on the spectrum's copy during processing; resolved values
are not written back into the original settings object. Resolved scalar defaults and ek0
are retained in the spectrum. Automatic kmax and nknots remain unset in stored settings
and are calculated locally for each input.

Named options were added in 0.2.5. Version 0.2.4 uses this constructor without
arguments, followed by property assignment.

## free

```typescript
free(): void;
```

Release this object's Wasm allocation. Do not call methods, read fields or free it again
afterwards. Arrays and settings already copied elsewhere remain valid.

## ek0

```typescript
ek0: number | undefined;
```

Edge energy used for the energy-to-k conversion, in eV. Default: undefined uses
normalization E0. An in-range override changes the background k grid without redefining the
pre/post-edge fitting regions. Prefer a consistent E0 across both stages; an out-of-range
override is discarded.

## rbkg

```typescript
rbkg: number | undefined;
```

Background cutoff in angstroms. Default: 1.0; undefined restores this default. AUTOBK
minimizes low-R Fourier components of the residual below a cutoff derived from this value.
Increasing rbkg allows a more flexible background and can remove real first-shell signal;
start below the first structural peak.

## nknots

```typescript
nknots: number | undefined;
```

Number of spline control points used to represent the smooth background. Default: undefined
derives the count from rbkg and the fitted k interval; the resolved count is clamped to 5
through 128. More points add flexibility and can overfit structure. This is not the length
of the repeated endpoint knot vector.

## kmin

```typescript
kmin: number | undefined;
```

Lower background fit/window bound in inverse angstroms. Default: 0.0; undefined restores
this default. Require kmin < the usable kmax. Raising this bound excludes the near-edge
region from the Fourier objective; the returned k() grid still begins at zero.

## kmax

```typescript
kmax: number | undefined;
```

Upper background fit/window bound in inverse angstroms. Default: undefined uses the
available data limit and explicit values are capped at that limit. Reducing it can exclude
noisy high-k data and shortens the returned k()/chi() arrays.

## kstep

```typescript
kstep: number | undefined;
```

Spacing of the uniform output k() grid in inverse angstroms. Default: 0.05; undefined
restores this default. Must be finite and positive. Smaller spacing interpolates the same
measured data more densely and changes the internal R sampling; it does not add
experimental resolution.

## nclamp

```typescript
nclamp: number | undefined;
```

Number of samples at each enabled endpoint used to discourage large residual chi values.
Default: 3; undefined restores this default. FixedPenalty requires a nonnegative integer,
caps the count at the available samples, and includes the last high-k sample. Use 0 to
disable endpoint clamping.

## clamp_lo

```typescript
clamp_lo: number | undefined;
```

Integer multiplier for the low-k endpoint residuals. Default: 0, which disables that
endpoint. In FixedPenalty the absolute value multiplies each residual, so its square
weights the objective. The active low-k samples begin at k=0 on the output grid.

## clamp_hi

```typescript
clamp_hi: number | undefined;
```

Integer multiplier for the high-k endpoint residuals. Default: 1. In FixedPenalty the
absolute value multiplies each residual, so doubling it quadruples that endpoint
contribution before averaging. Use 0 to disable the high-k endpoint penalty.

## clamp_lambda

```typescript
clamp_lambda: number | undefined;
```

Numerical strength of the FixedPenalty endpoint term. Recommended default: 0.001; undefined
restores this default. The objective adds lambda times the mean squared active, weighted
endpoint chi residual to the mean squared low-R residual. Require a finite nonnegative
value; 0 disables the endpoint term.

This empirical balance is tied to the implemented residual convention: the FixedPenalty
Fourier residual uses the fixed numerical factor 0.05/sqrt(pi), while the endpoint residual
uses unweighted, edge-step-normalized chi. Changing kweight or the window changes the
balance at a fixed lambda. The parameter is unused by legacy endpoint policies; it is not a
universal physical constant. See [the fixed-penalty
objective](https://rexafs.com/docs/science/autobk/).

## nfft

```typescript
nfft: number | undefined;
```

FFT length used inside background removal. Default: 2048; undefined restores this default.
Use a positive integer large enough to contain the prepared k grid. It sets the internal R
spacing together with kstep; increasing zero-padding refines that grid without adding
measured information.

## kweight

```typescript
kweight: number | undefined;
```

Integer power of k applied in the background Fourier objective. Default: 1; undefined
restores this default. Larger powers emphasize high-k residuals and their noise. The
returned chi() remains unweighted; the independent XrayFFTF.kweight controls the later
displayed transform.

## dk

```typescript
dk: number | undefined;
```

Background window parameter. Default: 0.1; undefined restores this default. For the default
Hanning window it controls endpoint taper widths in inverse angstroms. KaiserBessel also
uses this numeric value as a dimensionless shape parameter; equal dk does not make
different window families equivalent.

## linear_regularization

```typescript
linear_regularization: number | undefined;
```

Ridge strength for the legacy LinearDirect objective. Default: 0.0001; undefined restores
this default. Unused by the recommended FixedPenalty model, which solves the specified
endpoint-penalized least-squares problem without this additional ridge term. Change only
when reproducing a legacy calculation.

## linear_condition_limit

```typescript
linear_condition_limit: number | undefined;
```

Largest accepted condition number of the linear system. Default: 1e8; undefined restores
this default. FixedPenalty applies this limit to the column-scaled design matrix and
requires a finite value of at least 1. An ill-conditioned or rank-deficient solve throws
instead of silently changing the objective.

## linear_residual_ratio_limit

```typescript
linear_residual_ratio_limit: number | undefined;
```

Acceptance threshold for comparing the legacy direct solution residual against its
reference residual. Default: 1.05; undefined restores this default. Unused by FixedPenalty.
This is a numerical fallback criterion, not a statistical uncertainty or goodness-of-fit
probability.

## linear_fallback_to_lm

```typescript
linear_fallback_to_lm: boolean | undefined;
```

Allow the legacy direct solver to retry with linear_fallback_solver if its checks fail.
Default: true; undefined restores this default. Despite the historical name, the chosen
fallback need not be Levenberg-Marquardt. FixedPenalty never falls back and ignores this
flag.

## linear_workspace_cache

```typescript
linear_workspace_cache: boolean | undefined;
```

Reuse compatible spline geometry, Fourier operators and matrix factorization. Default:
true; undefined restores this default. Each spectrum still supplies new data and receives a
new solution, with condition checks repeated. Disabling this changes reuse and runtime, not
the specified objective.

## window

```typescript
window: FTWindow | undefined;
```

Window family used inside the background Fourier objective. Default: Hanning; undefined
restores Hanning. The taper reduces ringing at the selected k boundaries. This setting is
independent of the later XrayFFTF window; see FTWindow for the accepted names.

## solver

```typescript
solver: AUTOBKSolver | undefined;
```

Algorithm used to determine background spline coefficients. Recommended default:
LinearDirect, required by FixedPenalty; undefined restores it. LegacyLm solves the legacy
objective iteratively. TrustRegionDogLeg requires a native Rust feature and throws when
processing in the published Wasm package.

## linear_fallback_solver

```typescript
linear_fallback_solver: AUTOBKSolver | undefined;
```

Solver used only when an enabled legacy LinearDirect fallback is needed. Default in Wasm:
LegacyLm; undefined also resolves to LegacyLm with the default fallback flag. FixedPenalty
ignores this option. TrustRegionDogLeg is unavailable in Wasm, and LinearDirect cannot be
its own fallback.

## clamp_scale_policy

```typescript
clamp_scale_policy: AUTOBKClampScalePolicy | undefined;
```

Endpoint penalty model. Recommended default: FixedPenalty; undefined restores it.
FixedPenalty uses a fixed mean-square endpoint term and requires LinearDirect. Fixed and
TwoPass preserve older residual-dependent scaling choices for reproducing historical
results; they are different objectives.
