---
title: "TypeScript · VoigtOptions"
description: "VoigtOptions declarations, defaults and API explanations."
audience: user
pagefind: true
---

**Stable 0.2.11.** These signatures match the released npm package. Explanations are maintained in source JSDoc and reviewed against this release.

[Installation and version guide](/docs/reference/) · [TypeScript tutorial](/docs/libraries/typescript/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/v0.2.11/js-rexafs/types.d.ts) · [JSDoc source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts)

True convolution with separate Gaussian/Lorentzian widths.

## center

```typescript
center: number;
```

Center in eV under the selected origin, E0 offsets by default.

## area

```typescript
area: number;
```

Whole-axis analytic area in signal units times eV, nonnegative by default.

## gaussian_fwhm

```typescript
gaussian_fwhm: number;
```

Gaussian FWHM in eV. One width may be fixed to zero, not both.

## lorentzian_fwhm

```typescript
lorentzian_fwhm: number;
```

Lorentzian FWHM in eV. Combined FWHM is computed in the result.
