---
title: "TypeScript · MeasurementResult"
description: "MeasurementResult declarations, defaults and API explanations."
audience: user
pagefind: true
---

**Stable 0.2.12.** These signatures match the released npm package. Explanations are maintained in source JSDoc and reviewed against this release.

[Installation and version guide](/docs/reference/) · [TypeScript tutorial](/docs/libraries/typescript/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/v0.2.12/js-rexafs/types.d.ts) · [JSDoc source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts)

Owned native scalar result. JSON serialization preserves its complete definition.

## value

```typescript
value: number;
```

Finite scalar in unit.

## unit

```typescript
unit: string;
```

Signal unit for mean/maximum/point; signal times axis unit for integral.

## range

```typescript
range: [number, number];
```

Resolved absolute native-axis bounds: eV, inverse angstroms, or angstroms.

## position

```typescript
position: number | null;
```

Absolute point/maximum position, otherwise null.

## standard_error

```typescript
standard_error: number | null;
```

Propagated independent standard error, or null. Not a confidence interval.

## e0_ev

```typescript
e0_ev: number | null;
```

Resolved absorption edge in eV, or null when unnecessary.

## measurement

```typescript
measurement: {
    metric: { Point: { x: number } } | { Mean: { start: number; end: number } }
      | { Integral: { start: number; end: number } } | { Maximum: { start: number; end: number } };
    space: "Mu" | "Norm" | "Flat" | "Fourier" | { Chi: { kweight: number } };
    origin: "E0" | "Absolute";
  };
```

Core definition for reproducible storage; coordinates retain their requested origin.
