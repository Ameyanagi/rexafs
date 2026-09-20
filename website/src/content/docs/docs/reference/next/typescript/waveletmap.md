---
title: "TypeScript · WaveletMap"
description: "WaveletMap declarations, defaults and API explanations."
audience: user
pagefind: false
---

**Next API · unreleased.** This reference describes the source checkout. Compare with stable npm rexafs@0.2.12 before using it with an installed package.

[Installation and version guide](/docs/reference/) · [TypeScript tutorial](/docs/libraries/typescript/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts) · [JSDoc source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts)

Owned Cauchy map (since 0.2.10). All array getters return independent typed-array
copies. Complex/magnitude/phase arrays are flat, row-major: index r*shape[1]+k.
shape is [R rows, k columns]; W units are those of k**weight * χ, distinct from
ordinary Fourier scaling. Display colors and sampling do not define a metric.

## shape

```typescript
readonly shape: [number, number];
```

Matrix dimensions in [R, k] order, including explicit k padding.

## k

```typescript
readonly k: Float64Array;
```

Independent k coordinates, in Å⁻¹.

## r

```typescript
readonly r: Float64Array;
```

Independent R coordinates, in Å; not phase-corrected distances.

## input_k

```typescript
readonly input_k: Float64Array;
```

Original measured k before resampling.

## input_chi

```typescript
readonly input_chi: Float64Array;
```

Original unweighted χ, unchanged.

## prepared_chi

```typescript
readonly prepared_chi: Float64Array;
```

Resampled unweighted χ; zero outside selected support.

## window

```typescript
readonly window: Float64Array;
```

Support/taper multipliers applied before weighting.

## support

```typescript
readonly support: Uint8Array;
```

One for measured support, zero for padding.

## real

```typescript
readonly real: Float64Array;
```

Flat native real values.

## imaginary

```typescript
readonly imaginary: Float64Array;
```

Flat native imaginary values.

## magnitude

```typescript
readonly magnitude: Float64Array;
```

Flat native magnitude, without display normalization/resampling.

## phase

```typescript
phase(relative_floor?: number): Float64Array;
```

Radians, with NaN for zero amplitude or values below a fraction of the maximum
(default 1%). Fraction must lie in [0,1]. The native map remains unchanged.

## slice_at_r

```typescript
slice_at_r(r: number): Float64Array;
```

Native magnitude versus k at a covered R coordinate (Å).

## slice_at_k

```typescript
slice_at_k(k: number): Float64Array;
```

Native magnitude versus R at a covered k coordinate (Å⁻¹).

## integral

```typescript
integral(k_range: [number, number], r_range: [number, number]): WaveletRegionValue;
```

Integrate native bilinear magnitude over a covered k/R rectangle. Display
sampling never participates; invalid bounds throw. No uncertainty is inferred.

## mean

```typescript
mean(k_range: [number, number], r_range: [number, number]): WaveletRegionValue;
```

Area-weighted mean of native bilinear magnitude (since 0.2.10), not an average
of cells. k is Å⁻¹ and R is Å. Increasing, fully covered ranges are required;
invalid bounds throw. Returns units/method without inferred uncertainty.

## maximum

```typescript
maximum(k_range: [number, number], r_range: [number, number]): WaveletRegionValue;
```

Maximum native bilinear magnitude, including rectangle boundaries
(since 0.2.10). k is Å⁻¹ and R is Å; increasing, fully covered ranges are
required. Returns units/method without inferred uncertainty.

## definition

```typescript
readonly definition: Wavelet;
```

Fresh independent settings; release them with free() after use.

## preparation

```typescript
readonly preparation: WaveletPreparation | null;
```

Original processing metadata, or null for a direct array calculation.

## warnings

```typescript
readonly warnings: string[];
```

Interpretation and boundary diagnostics, not confidence intervals.

## to_json

```typescript
to_json(): string;
```

Full native map, original inputs and preparation provenance as JSON.

## from_json

```typescript
static from_json(json: string): WaveletMap;
```

Restore checked method, dimensions, axes, finite values and budgets. This does
not independently prove an external producer's numerical correctness.

## free

```typescript
free(): void;
```

Release the native map. Previously returned array copies remain valid.
