---
title: "TypeScript · PeakOptions"
description: "PeakOptions declarations, defaults and API explanations."
audience: user
pagefind: false
---

**Next API · unreleased.** This reference describes the source checkout. Compare with stable npm rexafs@0.2.13 before using it with an installed package.

[Installation and version guide](/docs/reference/) · [TypeScript tutorial](/docs/libraries/typescript/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts) · [JSDoc source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts)

Initial Gaussian/Lorentzian values; units follow the selected representation.

## center

```typescript
center: number;
```

Initial center in eV under the chosen origin: E0 offsets by default.

## area

```typescript
area: number;
```

Whole-axis analytic area, signal units times eV; nonnegative by default.

## fwhm

```typescript
fwhm: number;
```

Full width at half maximum in eV; strictly positive.
