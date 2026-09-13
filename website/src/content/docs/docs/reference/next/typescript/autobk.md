---
title: "TypeScript · AUTOBK"
description: "AUTOBK declarations and JSDoc."
audience: user
pagefind: false
---


**Next API · unreleased.** These signatures describe the source checkout, not npm rexafs@0.2.4.

[Installation and version guide](/docs/reference/) · [TypeScript tutorial](/docs/libraries/typescript/)

AUTOBK background settings. Recommended defaults use LinearDirect and FixedPenalty with lambda 0.001. Setters copy configurations; call free() when done.
Original AUTOBK method: [Newville et al. (1993)](https://doi.org/10.1103/PhysRevB.47.14126). The fixed endpoint penalty is a rexafs-specific choice, not part of that original objective.

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts)

## constructor

```typescript
constructor(options?: AUTOBKOptions);
```

Create settings, optionally overriding Rust defaults.

## free

```typescript
free(): void;
```

Release native memory. Do not use the object afterwards.

## ek0

```typescript
ek0: number | undefined;
```

Edge energy in eV. Default: use normalization E0.

## rbkg

```typescript
rbkg: number | undefined;
```

Background cutoff in angstroms. AUTOBK suppresses Fourier residuals below this R. Default: 1.0; increasing it can remove structural signal.

## nknots

```typescript
nknots: number | undefined;
```

Spline knot count. Default: determine from rbkg and the k range.

## kmin

```typescript
kmin: number | undefined;
```

Background fit lower k limit in inverse angstroms. Default: 0.0.

## kmax

```typescript
kmax: number | undefined;
```

Background fit upper k limit in inverse angstroms. Default: available data limit.

## kstep

```typescript
kstep: number | undefined;
```

Uniform output k spacing in inverse angstroms. Default: 0.05.

## nclamp

```typescript
nclamp: number | undefined;
```

Number of samples at each endpoint used by the clamp. Default: 3; 0 disables clamping.

## clamp_lo

```typescript
clamp_lo: number | undefined;
```

Low-k endpoint weight. Default: 0 (disabled).

## clamp_hi

```typescript
clamp_hi: number | undefined;
```

High-k endpoint weight. Default: 1.

## clamp_lambda

```typescript
clamp_lambda: number | undefined;
```

FixedPenalty strength. Recommended default: 0.001; 0 disables the endpoint penalty.

## nfft

```typescript
nfft: number | undefined;
```

FFT length for background removal. Default: 2048.

## kweight

```typescript
kweight: number | undefined;
```

Power of k used in the background objective. Default: 1.

## dk

```typescript
dk: number | undefined;
```

Background window taper width in inverse angstroms. Default: 0.1.

## linear_regularization

```typescript
linear_regularization: number | undefined;
```

Legacy direct-solver ridge strength. Default: 0.0001; unused by FixedPenalty.

## linear_condition_limit

```typescript
linear_condition_limit: number | undefined;
```

Maximum accepted linear-system condition number. Default: 1e8.

## linear_residual_ratio_limit

```typescript
linear_residual_ratio_limit: number | undefined;
```

Legacy direct-solver residual acceptance ratio. Default: 1.05; unused by FixedPenalty.

## linear_fallback_to_lm

```typescript
linear_fallback_to_lm: boolean | undefined;
```

Allow legacy solver fallback. Default: true; FixedPenalty never falls back.

## linear_workspace_cache

```typescript
linear_workspace_cache: boolean | undefined;
```

Reuse compatible spline/FFT geometry and SVD factors. Default: true; each spectrum has a new right-hand side and solution.

## window

```typescript
window: FTWindow | undefined;
```

Background Fourier window. Default: Hanning.

## solver

```typescript
solver: AUTOBKSolver | undefined;
```

Background solver. Recommended default: LinearDirect, required by FixedPenalty. TrustRegionDogLeg requires a Rust build with trust-region (included in Python, unavailable in Wasm).

## linear_fallback_solver

```typescript
linear_fallback_solver: AUTOBKSolver | undefined;
```

Legacy fallback solver. Default: TrustRegionDogLeg in Python, LegacyLm in Wasm; unused by FixedPenalty.

## clamp_scale_policy

```typescript
clamp_scale_policy: AUTOBKClampScalePolicy | undefined;
```

Endpoint model. Recommended default: FixedPenalty with LinearDirect; Fixed and TwoPass are legacy models.
