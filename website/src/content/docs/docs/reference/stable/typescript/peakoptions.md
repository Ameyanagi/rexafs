---
title: "TypeScript · PeakOptions"
description: "PeakOptions declarations, defaults and API explanations."
audience: user
pagefind: true
---

**Stable 0.2.13.** These signatures match the released npm package. Explanations are maintained in source JSDoc and reviewed against this release.

[Installation and version guide](/docs/reference/) · [TypeScript tutorial](/docs/libraries/typescript/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/v0.2.13/js-rexafs/types.d.ts) · [JSDoc source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts)

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
