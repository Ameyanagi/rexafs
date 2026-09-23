---
title: "TypeScript · XrayFFTROptions"
description: "XrayFFTROptions declarations, defaults and API explanations."
audience: user
pagefind: true
---

**Stable 0.2.14.** These signatures match the released npm package. Explanations are maintained in source JSDoc and reviewed against this release.

[Installation and version guide](/docs/reference/) · [TypeScript tutorial](/docs/libraries/typescript/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/v0.2.14/js-rexafs/types.d.ts) · [JSDoc source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts)

Named field overrides for new XrayFFTR(options). Omitted fields retain constructor defaults;
explicitly assigning undefined uses the resolution described for each field. An unknown key
or a non-object options argument throws TypeError. Numerical range and data-dependent checks
generally run when the configured processing stage executes.

## qmax_out

```typescript
qmax_out?: number | undefined;
```

Largest reported back-transform q in inverse angstroms. Default: 10.0; undefined restores
this default. This crops q()/chiq() to the available output range without changing the
inverse calculation. Require a finite nonnegative value.

## dr

```typescript
dr?: number | undefined;
```

Low-R window parameter. Default: 1.0; undefined restores this default. For taper windows it
controls transition geometry in angstroms. KaiserBessel also uses this numeric value as a
dimensionless shape parameter; different windows do not have equivalent shape for the same
dr.

## dr2

```typescript
dr2?: number | undefined;
```

High-R window parameter. Default: undefined uses dr. It controls the upper transition
geometry in angstroms, with interpretation depending on the selected window family. Use
matching dr and dr2 for symmetric endpoint settings.

## rmin

```typescript
rmin?: number | undefined;
```

Lower inverse-transform window bound in angstroms. Default: 0.0; explicitly assigning
undefined uses the first input R sample. Require 0 <= rmin < rmax. A nonzero lower bound
can exclude low-R background contributions.

## rmax

```typescript
rmax?: number | undefined;
```

Upper inverse-transform window bound in angstroms. Default: 20.0; explicitly assigning
undefined uses the last reported input R sample. Require rmax > rmin. Choose the R interval
around the shell contribution of interest; uncorrected Fourier peaks are not directly bond
lengths.

## rweight

```typescript
rweight?: number | undefined;
```

Power of R applied before the inverse transform. Default: 0.0; undefined restores this
default. Finite nonnegative values are floored to an integer. Leave at 0 for ordinary shell
filtering; a positive value additionally emphasizes larger-R contributions and changes the
signal units.

## nfft

```typescript
nfft?: number | undefined;
```

Inverse FFT length N. Default: 2048; undefined restores this default. Require an integer of
at least 2. Keeping kstep automatic resolves output spacing from the existing R grid and N;
reducing N discards high-R bins and increasing it pads them with zeros.

## kstep

```typescript
kstep?: number | undefined;
```

Output q spacing in inverse angstroms. Default: undefined computes pi / (nfft * delta_R),
where delta_R is the input R spacing in angstroms. An explicit value must agree with that
relationship or processing throws. Leave automatic when changing nfft. The resolved value
is retained in the spectrum's copy. After changing the forward R grid, reassign inverse
settings whose kstep is undefined to resolve it again.

## window

```typescript
window?: FTWindow | undefined;
```

R-space window family. Constructor default: KaiserBessel; explicitly assigning undefined
selects Hanning. The window selects and tapers the R contributions retained in chiq(). The
forward k weight and window are not divided out by the inverse transform.
