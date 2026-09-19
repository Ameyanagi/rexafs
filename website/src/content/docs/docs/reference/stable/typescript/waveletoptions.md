---
title: "TypeScript · WaveletOptions"
description: "WaveletOptions declarations, defaults and API explanations."
audience: user
pagefind: true
---

**Stable 0.2.11.** These signatures match the released npm package. Explanations are maintained in source JSDoc and reviewed against this release.

[Installation and version guide](/docs/reference/) · [TypeScript tutorial](/docs/libraries/typescript/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/v0.2.11/js-rexafs/types.d.ts) · [JSDoc source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts)

Optional Cauchy settings (since 0.2.10). Construction copies these values.

## kweight

```typescript
kweight?: number;
```

Integer exponent 0–6, default 2; emphasizes high-k signal and noise.

## order

```typescript
order?: number;
```

Cauchy order 1–4096, default 100. Larger order narrows frequency response and broadens localization in k.

## kstep

```typescript
kstep?: number;
```

Uniform numerical k spacing in Å⁻¹, default 0.05; interpolation adds no experimental resolution.

## rmax

```typescript
rmax?: number;
```

Maximum generated R in Å, default 6. R is not phase-corrected.

## rstep

```typescript
rstep?: number;
```

Numerical R spacing in Å; default π/(FFT length*kstep).

## taper

```typescript
taper?: number;
```

Half-cosine width inside the support endpoints in Å⁻¹. Default 0 means no taper.

## radii

```typescript
radii?: number[] | Float64Array;
```

Positive increasing R coordinates (Å), replacing the generated grid.

## nfft

```typescript
nfft?: number;
```

Power-of-two FFT length, at least twice the prepared k grid length; default automatic. No silent truncation.
