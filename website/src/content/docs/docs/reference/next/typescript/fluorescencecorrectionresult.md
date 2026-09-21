---
title: "TypeScript · FluorescenceCorrectionResult"
description: "FluorescenceCorrectionResult declarations, defaults and API explanations."
audience: user
pagefind: false
---

**Next API · unreleased.** This reference describes the source checkout. Compare with stable npm rexafs@0.2.13 before using it with an installed package.

[Installation and version guide](/docs/reference/) · [TypeScript tutorial](/docs/libraries/typescript/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts) · [JSDoc source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts)

Independent historical correction (since 0.2.10), with copied arrays/dictionaries.
Editing these values never alters the spectrum, to_json() record or replay
definition. No free() is required for this JavaScript result. Inspect warnings
and amplification; numerical success does not establish physical validity.

## energy

```typescript
readonly energy: Float64Array;
```

Original measured energy in eV, without resampling.

## original_mu

```typescript
readonly original_mu: Float64Array;
```

Original uncorrected absorption, in supplied units.

## corrected_mu

```typescript
readonly corrected_mu: Float64Array;
```

Corrected absorption, same grid/units; final normalization is separate.

## factor

```typescript
readonly factor: Float64Array;
```

Dimensionless alpha/denominator, without clipping.

## denominator

```typescript
readonly denominator: Float64Array;
```

Dimensionless alpha+1-internal_norm, without clipping.

## method

```typescript
readonly method: string;
```

Named numerical convention, fluo_elam_v1.

## input_mode

```typescript
readonly input_mode: AbsorptionMode;
```

Original acquisition interpretation; unknown records a caller assumption.

## alpha

```typescript
readonly alpha: number;
```

Dimensionless attenuation/geometry constant.

## geometry_ratio

```typescript
readonly geometry_ratio: number;
```

sin(incidence)/sin(exit) with surface angles; dimensionless.

## minimum_denominator

```typescript
readonly minimum_denominator: number;
```

Smallest dimensionless denominator on the whole input grid.

## maximum_amplification

```typescript
readonly maximum_amplification: number;
```

Largest dimensionless factor; high values amplify noise.

## singularity_threshold

```typescript
readonly singularity_threshold: number;
```

Numerical rejection limit, 64*epsilon*max(1, alpha+1).

## warnings

```typescript
readonly warnings: string[];
```

Domain, interpretation and numerical diagnostics; not confidence intervals.

## definition

```typescript
readonly definition: FluorescenceCorrection;
```

Fresh independent settings pinned to resolved ranges/E0 and atomic data; call free() after use.

## internal

```typescript
readonly internal: FluorescenceInternalNormalization;
```

Internal conventional fit of original mu, with independent lists.

## atomic

```typescript
readonly atomic: Record<string, unknown>;
```

Edge, emission and compound attenuation records: eV energies, mass fractions,
cm²/g values, table identities and checksums.

## to_json

```typescript
to_json(): string;
```

Original native record with full inputs/assumptions/atomic evidence.
