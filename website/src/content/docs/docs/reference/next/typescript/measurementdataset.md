---
title: "TypeScript · MeasurementDataset"
description: "MeasurementDataset declarations, defaults and API explanations."
audience: user
pagefind: false
---

**Next API · unreleased.** This reference describes the source checkout. Compare with stable npm rexafs@0.2.6 before using it with an installed package.

[Installation and version guide](/docs/reference/) · [TypeScript tutorial](/docs/libraries/typescript/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts) · [JSDoc source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts)

Numeric dataset or archived Larix/XTUNES result; its quantity may not be absorption.

## path

```typescript
path: string;
```

HDF5 path, XTUNES table path, or Larix /symbol/attribute path (JSON Pointer escaping: ~0 for ~ and ~1 for /).

## shape

```typescript
shape: number[];
```

Dimensions in HDF5 order; empty means scalar.

## values

```typescript
values: (number | null)[];
```

Row-major values; nonfinite values appear as null in snapshots.

## imaginary

```typescript
imaginary: (number | null)[] | null;
```

Imaginary components matching values and shape for a complex Larix array;
null for real data. Values contains the real components.

## attributes

```typescript
attributes: Record<string, string>;
```

Source attributes. Larix retains exact numeric bytes in larix.bytes_base64
with NumPy dtype in larix.dtype, including integers rounded by the f64 view.
