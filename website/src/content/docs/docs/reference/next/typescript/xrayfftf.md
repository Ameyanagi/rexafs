---
title: "TypeScript · XrayFFTF"
description: "XrayFFTF declarations, defaults and API explanations."
audience: user
pagefind: false
---

**Next API · unreleased.** This reference describes the source checkout. Compare with stable npm rexafs@0.2.10 before using it with an installed package.

[Installation and version guide](/docs/reference/) · [TypeScript tutorial](/docs/libraries/typescript/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts) · [JSDoc source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts)

Settings for converting weighted, windowed chi(k) into complex chi(R).

Recommended starting values: kmin=2, kmax=15 inverse angstroms, kweight=2, KaiserBessel,
dk=1 and nfft=2048. Adapt the k interval to the useful measured data. The default Input grid
preserves the background grid; automatic kstep normally resolves to AUTOBK's 0.05 inverse
angstroms.

For a uniform zero-origin grid `k[j]=j*kstep`, prepare
`g[j] = chi(k[j]) * k[j]^w * window[j]`. The code computes `chiR[m] = (kstep /
sqrt(pi)) * sum_j g[j] * exp(-2*pi*i*j*m/N)`, with N=nfft, w=kweight, i^2=-1 and
`R[m]=pi*m/(N*kstep)`. Here j indexes prepared k samples and m indexes nonnegative
Fourier bins. The sum uses the first N prepared samples and zeros for missing
samples. There is no 1/N forward normalization or window-area correction. For dimensionless
chi, chi(R) has units angstrom^(-(w+1)).

This is the [NumPy unnormalized forward DFT
convention](https://numpy.org/doc/stable/reference/routines.fft.html#implementation-details)
with the explicit factor in
[xftf_fast_nalgebra](https://github.com/Ameyanagi/rexafs/blob/main/crates/rexafs/src/xafs/xrayfft.rs).
Scattering phases shift the peaks, so R is not automatically a phase-corrected bond
distance; see [Rehr and Albers (2000)](https://doi.org/10.1103/RevModPhys.72.621).

set_fft() copies settings. Reassign after editing, and free() this object when finished.

## constructor

```typescript
constructor(options?: XrayFFTFOptions);
```

Create owned settings with the recommended defaults described below. Browser callers must
await init() first. Edit the fields, copy the settings into the appropriate spectrum stage,
and free() this object when finished.

Automatic fields are resolved on the spectrum's copy during processing; resolved values
are not written back into the original settings object. Resolved values stay in the
spectrum's settings until those settings are replaced.

Named options were added in 0.2.5. Version 0.2.4 uses this constructor without
arguments, followed by property assignment.

## free

```typescript
free(): void;
```

Release this object's Wasm allocation. Do not call methods, read fields or free it again
afterwards. Arrays and settings already copied elsewhere remain valid.

## grid

```typescript
grid: FFTGrid;
```

Sampling and window-construction convention. Default: Input preserves the prepared
background k grid. Larch linearly resamples onto a zero-origin grid and extends the window
domain as needed. Neither changes the returned background k()/chi(); always pair kwin()
with kwin_k().

## rmax_out

```typescript
rmax_out: number | undefined;
```

Maximum reported R in angstroms. Default: 10.0; undefined restores this default. This
limits the r() and chir_*() output arrays, not the internally retained Fourier bins used by
ifft(). It does not change the transform amplitude or frequency resolution. ifft()
nevertheless needs at least two reported R samples to infer/validate their spacing, so
rmax_out=0 is insufficient for a back-transform.

## dk

```typescript
dk: number | undefined;
```

Low-k window parameter. Default: 1.0; undefined restores this default. For taper windows it
controls transition geometry in inverse angstroms. KaiserBessel also uses the same numeric
value as a dimensionless shape parameter, so it cannot be compared as a universal taper
width across all windows.

## dk2

```typescript
dk2: number | undefined;
```

High-k window parameter. Default: undefined uses dk. It controls the upper transition
geometry in inverse angstroms; interpretation depends on the window family. Use the same
value as dk for symmetric endpoint settings.

## kmin

```typescript
kmin: number | undefined;
```

Lower Fourier window bound in inverse angstroms. Default: 2.0; explicitly assigning
undefined uses the first prepared k sample. Both bounds must be finite with kmin < kmax;
negative bounds are accepted. Select this above the region where the EXAFS approximation or
background subtraction is unreliable.

## kmax

```typescript
kmax: number | undefined;
```

Upper Fourier window bound in inverse angstroms. Default: 15.0; explicitly assigning
undefined uses the last prepared k sample. Require kmax > kmin. Select this within the
useful measured range; high-k noise can dominate after k weighting.

## kweight

```typescript
kweight: number | undefined;
```

Power of k applied before the forward transform. Default: 2.0; undefined restores this
default. Finite nonnegative values are floored to an integer w. Larger w emphasizes high-k
oscillations and noise; for dimensionless chi, the transformed amplitude has units
angstrom^(-(w + 1)).

## nfft

```typescript
nfft: number | undefined;
```

Forward FFT length N. Default: 2048; undefined restores this default. Require an integer of
at least 2. The R spacing is pi / (N * kstep), in angstroms. Use N at least as large as the
prepared data: Input truncates excess samples, whereas Larch rejects a window grid that
does not fit. Larger zero-padding does not improve experimental resolution.

## kstep

```typescript
kstep: number | undefined;
```

k spacing used to scale the transform and label R, in inverse angstroms. Default: undefined
infers the first spacing of the prepared k grid (normally 0.05 from AUTOBK defaults). Larch
also uses this spacing to resample chi. Input does not resample, so keep it consistent with
the input grid. Require a finite positive value. The inferred value is retained in the
spectrum's copied FFT settings. After changing the background grid, reassign FFT settings
with kstep undefined to infer the new spacing.

## window

```typescript
window: FTWindow | undefined;
```

Fourier window family. Constructor default: KaiserBessel; explicitly assigning undefined
selects the window routine's Hanning fallback. The window reduces truncation ringing and
changes amplitude. No correction for window area or coherent gain is applied; see FTWindow
for valid names.
