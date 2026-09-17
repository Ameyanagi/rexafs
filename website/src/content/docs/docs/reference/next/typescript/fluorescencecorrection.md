---
title: "TypeScript · FluorescenceCorrection"
description: "FluorescenceCorrection declarations, defaults and API explanations."
audience: user
pagefind: false
---

**Next API · unreleased.** This reference describes the source checkout. Compare with stable npm rexafs@0.2.9 before using it with an installed package.

[Installation and version guide](/docs/reference/) · [TypeScript tutorial](/docs/libraries/typescript/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts) · [JSDoc source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts)

Optically thick, homogeneous-sample XANES correction (unreleased).
new FluorescenceCorrection("CuO", "Cu", "K", {line:"Ka1", angles:[45,45]})
requires the complete sample formula, absorber, edge, detected emission and
measured geometry. Angles use the sample surface convention, not the normal.
Internal conventional normalization runs automatically; final normalization
of corrected mu is a separate operation. The fluo_elam_v1 model is not qualified
for EXAFS or finite-thickness samples. Offline atomic data load automatically.
See <https://xraypy.github.io/xraylarch/xafs_preedge.html#over-absorption-corrections.>

## constructor

```typescript
constructor(formula: string, element: string, edge: string, options: FluorescenceCorrectionOptions);
```

Copy explicit settings. Invalid types/options throw; scientific checks run
on calculation. Browser callers must await init() before construction.

## apply

```typescript
apply(energy: Float64Array, mu: Float64Array): FluorescenceCorrectionResult;
```

Correct original unnormalized fluorescence arrays on their energy grid (eV).
Copies matching finite Float64Arrays; energy must be positive and increasing.
Returns original/corrected mu in the same units. Invalid composition, geometry,
coverage, fitted step or singular denominator throw; nothing is clipped.
Uncertainty is unavailable. Prefer Spectrum.correct_fluorescence to retain
domain restrictions in later processing. This synchronous calculation should
run in a Worker for large browser workloads. Result needs no free().

## to_json

```typescript
to_json(): string;
```

Native settings JSON, including any pinned atomic identity.

## from_json

```typescript
static from_json(json: string): FluorescenceCorrection;
```

Restore settings; calculation checks scientific values and reference availability.

## free

```typescript
free(): void;
```

Release this model's Wasm allocation. Do not access it again; copied results remain valid.
