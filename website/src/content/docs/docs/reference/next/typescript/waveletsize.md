---
title: "TypeScript · WaveletSize"
description: "WaveletSize declarations, defaults and API explanations."
audience: user
pagefind: false
---

**Next API · unreleased.** This reference describes the source checkout. Compare with stable npm rexafs@0.2.9 before using it with an installed package.

[Installation and version guide](/docs/reference/) · [TypeScript tutorial](/docs/libraries/typescript/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts) · [JSDoc source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts)

Checked output dimensions and approximate scientific buffer storage.

## k_points

```typescript
k_points: number;
```

Prepared k columns, including padding.

## r_points

```typescript
r_points: number;
```

Positive R rows.

## nfft

```typescript
nfft: number;
```

Internal FFT length.

## cells

```typescript
cells: number;
```

Complex cell count.

## bytes

```typescript
bytes: number;
```

Estimated bytes, excluding input copies, FFT scratch and serialization.
