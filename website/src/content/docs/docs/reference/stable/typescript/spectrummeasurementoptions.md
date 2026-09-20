---
title: "TypeScript · SpectrumMeasurementOptions"
description: "SpectrumMeasurementOptions declarations, defaults and API explanations."
audience: user
pagefind: true
---

**Stable 0.2.12.** These signatures match the released npm package. Explanations are maintained in source JSDoc and reviewed against this release.

[Installation and version guide](/docs/reference/) · [TypeScript tutorial](/docs/libraries/typescript/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/v0.2.12/js-rexafs/types.d.ts) · [JSDoc source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts)

Scalar spectrum measurement options (since 0.2.10); defaults prepare Norm on a copy.

## space

```typescript
space?: "mu" | "norm" | "flat" | "chi" | "fourier";
```

Selected signal. Default: norm. mu retains original units; flat is dimensionless.

## origin

```typescript
origin?: "e0" | "absolute";
```

Default: e0 for energy, absolute for k/R. E0 means offsets in eV. k/R require absolute.

## kweight

```typescript
kweight?: number;
```

Nonnegative integer exponent on k, 0–255. Default: 0. Applies only to chi.

## errors

```typescript
errors?: Float64Array;
```

Independent standard deviations on the selected signal's native grid, in its units.
Raw-count errors are NOT propagated through normalization or Fourier transforms.
Requires finite nonnegative values matching that grid. Supports point, mean and
integral; maximum rejects this model. Axis, E0 and settings are treated as exact;
no correlations or confidence intervals are inferred. Omit for unknown errors.
