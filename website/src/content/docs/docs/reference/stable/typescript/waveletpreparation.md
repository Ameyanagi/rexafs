---
title: "TypeScript · WaveletPreparation"
description: "WaveletPreparation declarations, defaults and API explanations."
audience: user
pagefind: true
---

**Stable 0.2.11.** These signatures match the released npm package. Explanations are maintained in source JSDoc and reviewed against this release.

[Installation and version guide](/docs/reference/) · [TypeScript tutorial](/docs/libraries/typescript/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/v0.2.11/js-rexafs/types.d.ts) · [JSDoc source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts)

Spectrum preparation retained with a map; original k/χ are its direct replay inputs.

## software_version

```typescript
software_version: string;
```

Rexafs version that prepared the spectrum.

## backend

```typescript
backend: string;
```

Numerical backend name.

## e0

```typescript
e0: number | null;
```

Resolved E₀ (eV), if available.

## edge_step

```typescript
edge_step: number | null;
```

Normalization edge step in input μ units, if available.

## normalization

```typescript
normalization: unknown;
```

Versioned native normalization settings, without large result arrays.

## background

```typescript
background: unknown;
```

Versioned native background settings, without large result arrays.
