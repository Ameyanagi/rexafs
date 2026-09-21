---
title: "TypeScript · StepOptions"
description: "StepOptions declarations, defaults and API explanations."
audience: user
pagefind: false
---

**Next API · unreleased.** This reference describes the source checkout. Compare with stable npm rexafs@0.2.13 before using it with an installed package.

[Installation and version guide](/docs/reference/) · [TypeScript tutorial](/docs/libraries/typescript/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts) · [JSDoc source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts)

Initial values for an absorption-edge step; a step has no finite peak area.

## center

```typescript
center: number;
```

Center in eV under the selected origin, E0 offsets by default.

## height

```typescript
height: number;
```

Change between asymptotes in the selected signal units.

## scale

```typescript
scale: number;
```

Positive eV scale in erf((E-center)/scale) or atan((E-center)/scale); not a peak FWHM.
