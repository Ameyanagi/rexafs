---
title: "TypeScript · XrayFFTFOptions"
description: "XrayFFTFOptions declarations and JSDoc."
audience: user
pagefind: false
---


**Next API · unreleased.** These signatures describe the source checkout, not npm rexafs@0.2.4.

[Installation and version guide](/docs/reference/) · [TypeScript tutorial](/docs/libraries/typescript/)

Forward Fourier-transform settings. Defaults: k=2..15 inverse angstroms, kweight=2, KaiserBessel window.

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts)

## grid

```typescript
grid?: FFTGrid;
```

Sampling/window domain. Default: Input (existing k grid). Larch resamples on the extended FFT window grid.

## rmax_out

```typescript
rmax_out?: number | undefined;
```

Maximum displayed R in angstroms. Default: 10.0; does not truncate the inverse-transform filter.

## dk

```typescript
dk?: number | undefined;
```

Low-k taper width in inverse angstroms. Default: 1.0.

## dk2

```typescript
dk2?: number | undefined;
```

High-k taper width in inverse angstroms. Default: use dk.

## kmin

```typescript
kmin?: number | undefined;
```

Lower Fourier window limit in inverse angstroms. Default: 2.0; undefined uses the first k sample.

## kmax

```typescript
kmax?: number | undefined;
```

Upper Fourier window limit in inverse angstroms. Default: 15.0; undefined uses the last k sample.

## kweight

```typescript
kweight?: number | undefined;
```

Power of k applied before FFT. Default: 2.0; nonnegative values are floored to an integer.

## nfft

```typescript
nfft?: number | undefined;
```

Forward FFT length. Default: 2048.

## kstep

```typescript
kstep?: number | undefined;
```

FFT k spacing in inverse angstroms. Default: infer from input k.

## window

```typescript
window?: FTWindow | undefined;
```

Fourier window shape. Default: KaiserBessel; explicitly unset uses Hanning.
