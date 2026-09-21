---
title: "TypeScript · SpectrumMapping"
description: "SpectrumMapping declarations, defaults and API explanations."
audience: user
pagefind: true
---

**Stable 0.2.13.** These signatures match the released npm package. Explanations are maintained in source JSDoc and reviewed against this release.

[Installation and version guide](/docs/reference/) · [TypeScript tutorial](/docs/libraries/typescript/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/v0.2.13/js-rexafs/types.d.ts) · [JSDoc source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts)

Explicit, copied selection for converting one original scan.
Column defaults to ColumnSelector (string | number). Detected candidates use
SpectrumMapping<number>, since the reader has already resolved their indices.

## energy_column

```typescript
energy_column: Column;
```

Exact name or zero-based index of the energy or calibrated angle column.

## energy

```typescript
energy: EnergyConversion;
```

Source axis conversion to electronvolts.

## signal

```typescript
signal: SignalConversion<Column>;
```

Selected detector arithmetic, without normalization or background removal.
