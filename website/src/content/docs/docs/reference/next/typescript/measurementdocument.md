---
title: "TypeScript · MeasurementDocument"
description: "MeasurementDocument declarations, defaults and API explanations."
audience: user
pagefind: false
---

**Next API · unreleased.** This reference describes the source checkout. Compare with stable npm rexafs@0.2.9 before using it with an installed package.

[Installation and version guide](/docs/reference/) · [TypeScript tutorial](/docs/libraries/typescript/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts) · [JSDoc source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts)

Independent snapshot of a universal import; editing it does not change Rust data.

## format

```typescript
format: string;
```

Content-detected format family.

## scans

```typescript
scans: MeasurementScan[];
```

All recovered scans, including those requiring manual mapping.

## datasets

```typescript
datasets: MeasurementDataset[];
```

HDF5 datasets and saved Larix/XTUNES arrays, retaining shapes and independent grids.

## metadata

```typescript
metadata: Record<string, string>;
```

Container provenance. Larix uses larix.session_text, larix.command_history
and larix.symbol_order; commands and saved Python objects remain inert text.

## warnings

```typescript
warnings: string[];
```

Encoding, container and unreadable HDF5 alias-group diagnostics.
