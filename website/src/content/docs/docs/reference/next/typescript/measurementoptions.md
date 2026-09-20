---
title: "TypeScript · MeasurementOptions"
description: "MeasurementOptions declarations, defaults and API explanations."
audience: user
pagefind: false
---

**Next API · unreleased.** This reference describes the source checkout. Compare with stable npm rexafs@0.2.12 before using it with an installed package.

[Installation and version guide](/docs/reference/) · [TypeScript tutorial](/docs/libraries/typescript/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts) · [JSDoc source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts)

Convenient scan and column selection (since 0.2.6). Names must be exact and unique.
Supply energy and exactly one of mu, it or iff; it/iff also require i0.
Conflicting/incomplete options throw. Omit column options to use automatic detection.

## scan

```typescript
scan?: number;
```

Zero-based scan index; defaults to 0.

## energy

```typescript
energy?: ColumnSelector;
```

Axis name or zero-based index; required with explicit signal roles.

## energy_unit

```typescript
energy_unit?: 'eV' | 'keV';
```

Override detected axis calibration; omitted retains detected/declared units. Unknown units require a choice.

## mu

```typescript
mu?: ColumnSelector;
```

Stored absorption column; cannot be combined with i0, it or iff.

## i0

```typescript
i0?: ColumnSelector;
```

Incident monitor, required for it or iff.

## it

```typescript
it?: ColumnSelector;
```

Transmitted intensity; produces ln(i0 / it), using the shared conversion checks.

## iff

```typescript
iff?: ColumnSelector | ColumnSelector[];
```

Fluorescence/yield detector or explicit list; summed then divided by i0, without corrections.
