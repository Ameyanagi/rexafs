---
title: "TypeScript · PeakContribution"
description: "PeakContribution declarations, defaults and API explanations."
audience: user
pagefind: false
---

**Next API · unreleased.** This reference describes the source checkout. Compare with stable npm rexafs@0.2.11 before using it with an installed package.

[Installation and version guide](/docs/reference/) · [TypeScript tutorial](/docs/libraries/typescript/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts) · [JSDoc source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts)

One peak, step or baseline contribution on retained native points.

## name

```typescript
name: string;
```

Stable component identity from the initial definition.

## role

```typescript
role: "Peak" | "Baseline" | "Edge";
```

Scientific role, independent of mathematical shape.

## shape

```typescript
shape: "Gaussian" | "Lorentzian" | "PseudoVoigt" | "Voigt" | "ErfStep" | "ArctanStep" | "Constant" | "Linear";
```

Mathematical shape used for evaluation.

## curve

```typescript
curve: number[];
```

Component values at the result's absolute-energy points, in signal units.

## center_ev

```typescript
center_ev: number | null;
```

Peak/step center in absolute eV; None for polynomial baselines.

## area

```typescript
area: number | null;
```

Whole-axis model area in signal units × eV; None for steps/polynomials.

## height

```typescript
height: number | null;
```

Peak contribution at its center, excluding all other components.

## fwhm_ev

```typescript
fwhm_ev: number | null;
```

Peak FWHM in eV; true Voigt uses a numerical half-height root.

## center_standard_error_ev

```typescript
center_standard_error_ev: number | null;
```

Conditional errors propagated with the full joint covariance; absent when
local uncertainty is unavailable or the quantity does not apply.

## area_standard_error

```typescript
area_standard_error: number | null;
```

Conditional whole-axis area error, in signal units × eV.

## height_standard_error

```typescript
height_standard_error: number | null;
```

Conditional peak-height error, in signal units.

## fwhm_standard_error_ev

```typescript
fwhm_standard_error_ev: number | null;
```

Conditional FWHM error, in eV, including both true-Voigt width parameters.

## sampled_integral

```typescript
sampled_integral: number;
```

Trapezoidal component integral over included native-grid segments only.
Masked gaps are not bridged; this is not its whole-axis analytic area.
