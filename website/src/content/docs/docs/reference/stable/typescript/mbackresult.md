---
title: "TypeScript · MbackResult"
description: "MbackResult declarations, defaults and API explanations."
audience: user
pagefind: true
---

**Stable 0.2.14.** These signatures match the released npm package. Explanations are maintained in source JSDoc and reviewed against this release.

[Installation and version guide](/docs/reference/) · [TypeScript tutorial](/docs/libraries/typescript/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/v0.2.14/js-rexafs/types.d.ts) · [JSDoc source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts)

Owned full-MBACK result (since 0.2.10). Arrays are independent JavaScript copies.
norm and fpp are different quantities. Region balancing is not inverse-variance
weighting; objective/convergence alone do not establish experimental uncertainty.

## method

```typescript
method: string;
```

Named profile, currently mback_chantler_v1.

## definition

```typescript
readonly definition: MBack;
```

A fresh native replay model pinned to the original table; free() it after use.

## to_json

```typescript
to_json(): string;
```

Original result snapshot including all settings/provenance. Editing copied arrays does not alter it.

## reference

```typescript
reference: AtomicReference;
```

Exact offline reference and interpolation identity.

## e0

```typescript
e0: number;
```

Fixed resolved edge origin in eV.

## tabulated_edge_ev

```typescript
tabulated_edge_ev: number;
```

Tabulated edge in eV; not shifted to measured E0.

## pre_edge

```typescript
pre_edge: [number, number];
```

Resolved inclusive pre-edge offsets in eV.

## post_edge

```typescript
post_edge: [number, number];
```

Resolved inclusive post-edge offsets in eV.

## scale

```typescript
scale: number;
```

Positive atomic conversion scale from input mu units.

## edge_step

```typescript
edge_step: number;
```

Positive fitted absorption step in input mu units.

## objective

```typescript
objective: number;
```

Sum of squared balanced residuals, in squared f2 units.

## condition

```typescript
condition: number;
```

Column-scaled Jacobian condition number.

## rank

```typescript
rank: number;
```

Final Jacobian rank, including erfc width when enabled.

## evaluations

```typescript
evaluations: number;
```

Number of linear solves during fitting.

## erfc_width

```typescript
erfc_width: number | null;
```

Width in eV, or null when erfc is disabled.

## erfc_amplitude

```typescript
erfc_amplitude: number;
```

Amplitude in f2 units; zero when disabled.

## energy_scale

```typescript
energy_scale: number;
```

Polynomial coordinate scale in eV.

## energy

```typescript
energy: number[];
```

Original energy grid, in eV.

## f2

```typescript
f2: number[];
```

Atomic scattering factor in electron units.

## fpp

```typescript
fpp: number[];
```

Matched scale*mu-background in electron units; distinct from norm.

## norm

```typescript
norm: number[];
```

Dimensionless normalized absorption (scale*mu-pre_curve)/Delta.

## flat

```typescript
flat: number[];
```

Dimensionless flattened absorption using the auxiliary post-edge trend.

## background

```typescript
background: number[];
```

Smooth background in f2 units.

## pre_curve

```typescript
pre_curve: number[];
```

Auxiliary pre-edge line on f2+background, in f2 units.

## post_curve

```typescript
post_curve: number[];
```

Auxiliary quadratic post-edge curve, in f2 units.

## residual

```typescript
residual: number[];
```

Unweighted f2+background-scale*mu on every input point.

## coefficients

```typescript
coefficients: number[];
```

Increasing polynomial powers of (energy-e0)/energy_scale, in f2 units.

## fit_indices

```typescript
fit_indices: number[];
```

Original zero-based indices included in the objective.

## weights

```typescript
weights: number[];
```

1/sqrt(region count), in fit_indices order.

## warnings

```typescript
warnings: string[];
```

Nonfatal boundary/conditioning diagnostics.
