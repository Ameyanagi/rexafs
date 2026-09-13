---
title: "TypeScript · XrayFFTROptions"
description: "XrayFFTROptions declarations and JSDoc."
audience: user
pagefind: false
---


**Next API · unreleased.** These signatures describe the source checkout, not npm rexafs@0.2.4.

[Installation and version guide](/docs/reference/) · [TypeScript tutorial](/docs/libraries/typescript/)

Inverse Fourier-transform settings. Set rmin/rmax to select an R-space shell.

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts)

## qmax_out

```typescript
qmax_out?: number | undefined;
```

Maximum back-transform q in inverse angstroms. Default: 10.0.

## dr

```typescript
dr?: number | undefined;
```

Low-R taper width in angstroms. Default: 1.0.

## dr2

```typescript
dr2?: number | undefined;
```

High-R taper width in angstroms. Default: use dr.

## rmin

```typescript
rmin?: number | undefined;
```

Lower inverse-transform window limit in angstroms. Default: 0.0.

## rmax

```typescript
rmax?: number | undefined;
```

Upper inverse-transform window limit in angstroms. Default: 20.0; choose a shell range for R filtering.

## rweight

```typescript
rweight?: number | undefined;
```

Power of R applied before IFFT. Default: 0.0; nonnegative values are floored to an integer.

## nfft

```typescript
nfft?: number | undefined;
```

Inverse FFT length. Default: 2048; leave kstep automatic when changing this.

## kstep

```typescript
kstep?: number | undefined;
```

Output q spacing in inverse angstroms. Default: infer from input R and nfft; an explicit value must match that spacing.

## window

```typescript
window?: FTWindow | undefined;
```

Inverse Fourier window shape. Default: KaiserBessel; explicitly unset uses Hanning.
