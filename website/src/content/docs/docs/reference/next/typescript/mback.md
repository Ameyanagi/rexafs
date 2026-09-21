---
title: "TypeScript · MBack"
description: "MBack declarations, defaults and API explanations."
audience: user
pagefind: false
---

**Next API · unreleased.** This reference describes the source checkout. Compare with stable npm rexafs@0.2.13 before using it with an installed package.

[Installation and version guide](/docs/reference/) · [TypeScript tutorial](/docs/libraries/typescript/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts) · [JSDoc source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts)

Full Chantler MBACK normalization (since 0.2.10). Example:
new MBack("Cu", "K", {pre_edge: [-200,-50], post_edge: [100,800]}).

Default degree 2 and erfc off. Automatic ranges respect neighboring edges;
inspect the returned intervals. Offline data are loaded automatically and
never shifted. fit() copies inputs and leaves them/model unchanged. Assign to
spectrum.set_normalization_method(model).normalize() for ordinary processing.
Invalid coverage, unsupported tables, unidentifiable fits and nonpositive
scale/step throw. Use a Worker for large browser fits; fit() is synchronous.
Call free() when finished. Settings copied into a spectrum remain independent.

## constructor

```typescript
constructor(element: string, edge: string, options?: MbackOptions);
```

Select absorber/edge and optional named settings; browser init() is required first.

## fit

```typescript
fit(energy: Float64Array, mu: Float64Array): MbackResult;
```

Fit matching finite raw absorption arrays, with strictly increasing energy in eV.

## to_json

```typescript
to_json(): string;
```

Versioned model JSON; no input arrays are added. Throws after free().

## from_json

```typescript
static from_json(json: string): MBack;
```

Restore a native definition. fit() checks scientific values and archived table identity.

## free

```typescript
free(): void;
```

Release this native model. Spectrum settings and owned results remain valid.
