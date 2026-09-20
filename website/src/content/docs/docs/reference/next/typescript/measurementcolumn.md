---
title: "TypeScript · MeasurementColumn"
description: "MeasurementColumn declarations, defaults and API explanations."
audience: user
pagefind: false
---

**Next API · unreleased.** This reference describes the source checkout. Compare with stable npm rexafs@0.2.12 before using it with an installed package.

[Installation and version guide](/docs/reference/) · [TypeScript tutorial](/docs/libraries/typescript/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts) · [JSDoc source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts)

Original owned numeric channel in acquisition order.

## name

```typescript
name: string;
```

Source label, or column_N when absent (N is one-based).

## units

```typescript
units: string | null;
```

Declared source units; null means absent.

## values

```typescript
values: (number | null)[];
```

Raw samples. JSON snapshots encode nonfinite source values as null.
