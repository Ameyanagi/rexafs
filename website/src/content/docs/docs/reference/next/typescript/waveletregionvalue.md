---
title: "TypeScript · WaveletRegionValue"
description: "WaveletRegionValue declarations, defaults and API explanations."
audience: user
pagefind: false
---

**Next API · unreleased.** This reference describes the source checkout. Compare with stable npm rexafs@0.2.11 before using it with an installed package.

[Installation and version guide](/docs/reference/) · [TypeScript tutorial](/docs/libraries/typescript/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts) · [JSDoc source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts)

Native covered-rectangle magnitude statistic; method identifies integral, mean or maximum. Units are those of k**weight * χ.

## value

```typescript
value: number;
```

Native region statistic; no experimental uncertainty is supplied.

## k_range

```typescript
k_range: [number, number];
```

Exact inclusive k bounds, in Å⁻¹.

## r_range

```typescript
r_range: [number, number];
```

Exact inclusive R bounds, in Å.

## unit

```typescript
unit: string;
```

Integral units including k weight.

## method

```typescript
method: string;
```

Numerical convention: bilinear_magnitude_v1 (integral), bilinear_magnitude_mean_v1 or bilinear_magnitude_maximum_v1.
