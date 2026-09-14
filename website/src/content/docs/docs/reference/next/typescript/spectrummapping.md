---
title: "TypeScript · SpectrumMapping"
description: "SpectrumMapping declarations, defaults and API explanations."
audience: user
pagefind: false
---

**Next API · unreleased.** This reference describes the source checkout. Compare with stable npm rexafs@0.2.5 before using it with an installed package.

[Installation and version guide](/docs/reference/) · [TypeScript tutorial](/docs/libraries/typescript/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts) · [JSDoc source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts)

Explicit, copied selection for converting one original scan.

## energy_column

```typescript
energy_column: number;
```

Zero-based energy or calibrated angle column.

## energy

```typescript
energy: EnergyConversion;
```

Source axis conversion to electronvolts.

## signal

```typescript
signal: SignalConversion;
```

Selected detector arithmetic, without normalization or background removal.
