---
title: "TypeScript · XrayFFTR"
description: "XrayFFTR declarations, defaults and API explanations."
audience: user
pagefind: false
---

**Next API · unreleased.** This reference describes the source checkout. Compare with stable npm rexafs@0.2.10 before using it with an installed package.

[Installation and version guide](/docs/reference/) · [TypeScript tutorial](/docs/libraries/typescript/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts) · [JSDoc source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts)

Settings for filtering selected R-space contributions and returning a real signal chi(q).

Choose rmin/rmax in angstroms to enclose the contributions of interest. Defaults are rmin=0,
rmax=20, dr=1, KaiserBessel, rweight=0, nfft=2048 and qmax_out=10 inverse angstroms.
Automatic kstep preserves consistency with the input R spacing.

rexafs windows and optionally R-weights the complex forward bins, supplies their conjugate
negative-frequency partners, and performs a real inverse DFT with scale
sqrt(pi)/(kstep*nfft). This implements a real filtered back-transform; forward k-weighting
and windowing remain in the result. It is not a general recovery of the original unweighted
chi(k), and is not Larch's complex, one-sided inverse representation. See
[inverse_fft.rs](https://github.com/Ameyanagi/rexafs/blob/main/crates/rexafs/src/xafs/inverse_fft.rs)
and the [DFT normalization
convention](https://numpy.org/doc/stable/reference/routines.fft.html#normalization).

set_ifft() copies settings. Reassign after editing, and free() this object when finished.

## constructor

```typescript
constructor(options?: XrayFFTROptions);
```

Create owned settings with the recommended defaults described below. Browser callers must
await init() first. Edit the fields, copy the settings into the appropriate spectrum stage,
and free() this object when finished.

Automatic fields are resolved on the spectrum's copy during processing; resolved values
are not written back into the original settings object. Resolved values stay in the
spectrum's settings until those settings are replaced.

This class was added in 0.2.5 and is not exported by npm 0.2.4.

## free

```typescript
free(): void;
```

Release this object's Wasm allocation. Do not call methods, read fields or free it again
afterwards. Arrays and settings already copied elsewhere remain valid.

## qmax_out

```typescript
qmax_out: number | undefined;
```

Largest reported back-transform q in inverse angstroms. Default: 10.0; undefined restores
this default. This crops q()/chiq() to the available output range without changing the
inverse calculation. Require a finite nonnegative value.

## dr

```typescript
dr: number | undefined;
```

Low-R window parameter. Default: 1.0; undefined restores this default. For taper windows it
controls transition geometry in angstroms. KaiserBessel also uses this numeric value as a
dimensionless shape parameter; different windows do not have equivalent shape for the same
dr.

## dr2

```typescript
dr2: number | undefined;
```

High-R window parameter. Default: undefined uses dr. It controls the upper transition
geometry in angstroms, with interpretation depending on the selected window family. Use
matching dr and dr2 for symmetric endpoint settings.

## rmin

```typescript
rmin: number | undefined;
```

Lower inverse-transform window bound in angstroms. Default: 0.0; explicitly assigning
undefined uses the first input R sample. Require 0 <= rmin < rmax. A nonzero lower bound
can exclude low-R background contributions.

## rmax

```typescript
rmax: number | undefined;
```

Upper inverse-transform window bound in angstroms. Default: 20.0; explicitly assigning
undefined uses the last reported input R sample. Require rmax > rmin. Choose the R interval
around the shell contribution of interest; uncorrected Fourier peaks are not directly bond
lengths.

## rweight

```typescript
rweight: number | undefined;
```

Power of R applied before the inverse transform. Default: 0.0; undefined restores this
default. Finite nonnegative values are floored to an integer. Leave at 0 for ordinary shell
filtering; a positive value additionally emphasizes larger-R contributions and changes the
signal units.

## nfft

```typescript
nfft: number | undefined;
```

Inverse FFT length N. Default: 2048; undefined restores this default. Require an integer of
at least 2. Keeping kstep automatic resolves output spacing from the existing R grid and N;
reducing N discards high-R bins and increasing it pads them with zeros.

## kstep

```typescript
kstep: number | undefined;
```

Output q spacing in inverse angstroms. Default: undefined computes pi / (nfft * delta_R),
where delta_R is the input R spacing in angstroms. An explicit value must agree with that
relationship or processing throws. Leave automatic when changing nfft. The resolved value
is retained in the spectrum's copy. After changing the forward R grid, reassign inverse
settings whose kstep is undefined to resolve it again.

## window

```typescript
window: FTWindow | undefined;
```

R-space window family. Constructor default: KaiserBessel; explicitly assigning undefined
selects Hanning. The window selects and tapers the R contributions retained in chiq(). The
forward k weight and window are not divided out by the inverse transform.
