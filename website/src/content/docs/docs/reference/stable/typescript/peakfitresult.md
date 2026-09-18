---
title: "TypeScript · PeakFitResult"
description: "PeakFitResult declarations, defaults and API explanations."
audience: user
pagefind: true
---

**Stable 0.2.10.** These signatures match the released npm package. Explanations are maintained in source JSDoc and reviewed against this release.

[Installation and version guide](/docs/reference/) · [TypeScript tutorial](/docs/libraries/typescript/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/v0.2.10/js-rexafs/types.d.ts) · [JSDoc source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts)

Owned numerical result (since 0.2.10). Arrays are ordinary JavaScript copies.
Editing the displayed values never changes the retained JSON or the input spectrum.
Local errors are conditional, not model-selection confidence intervals.

## definition

```typescript
readonly definition: PeakFit;
```

Initial model; each access returns a new native copy. Release it with free() when finished.

## fitted_model

```typescript
fitted_model(): PeakFit;
```

Copy fitted values for explicit reuse; caller owns the returned model.

## parameters

```typescript
parameters: Record<string, number>;
```

Named final values; parameter centers retain the model's coordinate origin.

## parameter_errors

```typescript
parameter_errors: Record<string, number | null>;
```

Conditional local errors, null when unavailable or not independently estimated.

## components

```typescript
components: PeakContribution[];
```

Component curves/summaries in model order.

## to_json

```typescript
to_json(): string;
```

Full native snapshot with initial/final constraints, masks and diagnostics.

## origin_ev

```typescript
origin_ev: number;
```

Resolved energy origin in eV, added to parameter centers/reference energies.

## energy

```typescript
energy: number[];
```

Absolute energy in eV, only the native points used by this fit.

## source_indices

```typescript
source_indices: number[];
```

Original zero-based point indices; preserves masks and sampling provenance.

## data

```typescript
data: number[];
```

Selected representation's measured values, in its signal units.

## model

```typescript
model: number[];
```

Joint baseline + peaks + steps, in the same signal units.

## residual

```typescript
residual: number[];
```

Unweighted data minus model, in signal units (also for weighted fits).

## standard_deviation

```typescript
standard_deviation: number[] | null;
```

Supplied selected-space standard deviations on the fitted points, if any.

## objective

```typescript
objective: number;
```

Sum of squared residuals, divided by supplied standard deviations if present.

## points

```typescript
points: number;
```

Number of fitted native data points (not EXAFS independent-point estimates).

## free_parameters

```typescript
free_parameters: number;
```

Number of independent varying parameters; expression ties are excluded.

## degrees_of_freedom

```typescript
degrees_of_freedom: number;
```

points − free_parameters. Fits with fewer points than variables are rejected.

## jacobian_rank

```typescript
jacobian_rank: number;
```

Weighted numerical Jacobian rank under a 1e-10 relative singular-value cutoff.

## covariance_names

```typescript
covariance_names: string[];
```

Sorted independent parameter names defining covariance/correlation axes.

## covariance

```typescript
covariance: number[][] | null;
```

Local covariance; absolute-error scaling when standard deviations were given,
otherwise multiplied by objective/degrees_of_freedom.

## correlation

```typescript
correlation: number[][] | null;
```

Dimensionless correlations corresponding to covariance_names. Absent when
any conditional variance is zero; the warning explains that case.

## uncertainty_unavailable

```typescript
uncertainty_unavailable: string | null;
```

Why covariance/standard errors were withheld, rather than replaced by zero.

## peak_center_ev

```typescript
peak_center_ev: number | null;
```

Model peak-area-weighted center in absolute eV, excluding baseline/steps.

## peak_center_standard_error_ev

```typescript
peak_center_standard_error_ev: number | null;
```

Conditional error in that center, using full parameter covariance.

## termination

```typescript
termination: "FixedModel" | "Converged" | "NotConverged" | "Cancelled";
```

Explicit numerical termination category.

## termination_detail

```typescript
termination_detail: string;
```

Solver-specific termination detail, retained verbatim for diagnosis.

## evaluations

```typescript
evaluations: number;
```

Number of residual-vector evaluations during optimization, including numerical derivatives.

## warnings

```typescript
warnings: string[];
```

Active bounds and other model/uncertainty limitations.
