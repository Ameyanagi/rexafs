---
title: "TypeScript · FluorescenceInternalNormalization"
description: "FluorescenceInternalNormalization declarations, defaults and API explanations."
audience: user
pagefind: true
---

**Stable 0.2.14.** These signatures match the released npm package. Explanations are maintained in source JSDoc and reviewed against this release.

[Installation and version guide](/docs/reference/) · [TypeScript tutorial](/docs/libraries/typescript/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/v0.2.14/js-rexafs/types.d.ts) · [JSDoc source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts)

Internal conventional fit of original mu (since 0.2.10), distinct from final
normalization. Lists are independent copies on the original energy grid.

## e0

```typescript
e0: number;
```

Edge energy E0 used by the internal normalization, in electronvolts.

## pre_edge

```typescript
pre_edge: [number, number];
```

Resolved pre-edge eV offsets from E0.

## post_edge

```typescript
post_edge: [number, number];
```

Resolved post-edge eV offsets from E0.

## degree

```typescript
degree: number;
```

Internal post-edge polynomial degree; pre-edge is linear.

## edge_step

```typescript
edge_step: number;
```

Positive fitted jump in original mu units, before numerical flooring.

## pre_curve

```typescript
pre_curve: number[];
```

Pre-edge line in original mu units.

## post_curve

```typescript
post_curve: number[];
```

Pre-edge line plus post-edge polynomial, in original mu units.

## norm

```typescript
norm: number[];
```

Dimensionless internal n0 in alpha+1-n0.
