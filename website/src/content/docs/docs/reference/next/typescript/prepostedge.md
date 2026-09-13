---
title: "TypeScript · PrePostEdge"
description: "PrePostEdge declarations and JSDoc."
audience: user
pagefind: false
---


**Next API · unreleased.** These signatures describe the source checkout, not npm rexafs@0.2.4.

[Installation and version guide](/docs/reference/) · [TypeScript tutorial](/docs/libraries/typescript/)

Pre/post-edge normalization settings. Defaults adapt to the measured energy range. Setters copy configurations; call free() when done.
For measurement conventions, see [Newville, Fundamentals of XAFS](https://docs.xrayabsorption.org/tutorials/XAFS_Fundamentals.pdf).

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts)

## constructor

```typescript
constructor(options?: PrePostEdgeOptions);
```

Create settings, optionally overriding Rust defaults.

## free

```typescript
free(): void;
```

Release native memory. Do not use the object afterwards.

## pre_edge_start

```typescript
pre_edge_start: number | undefined;
```

Pre-edge fit start relative to E0, in eV. Default: infer from the measured range.

## pre_edge_end

```typescript
pre_edge_end: number | undefined;
```

Pre-edge fit end relative to E0, in eV. Default: infer from the pre-edge start.

## norm_start

```typescript
norm_start: number | undefined;
```

Post-edge fit start relative to E0, in eV. Default: infer from the available range (at most 25 eV).

## norm_end

```typescript
norm_end: number | undefined;
```

Post-edge fit end relative to E0, in eV. Default: measured upper energy limit.

## norm_polyorder

```typescript
norm_polyorder: number | undefined;
```

Post-edge polynomial degree, 0 through 5. Default: 0, 1 or 2 for fit spans below 50, below 350, or at least 350 eV.

## n_victoreen

```typescript
n_victoreen: number | undefined;
```

Victoreen energy exponent for the pre-edge fit. Default: 0.

## e0

```typescript
e0: number | undefined;
```

Edge energy in eV. Default: detect from the spectrum.

## edge_step

```typescript
edge_step: number | undefined;
```

Absorption edge-step override in mu units. Default: estimate from the fitted baselines.
