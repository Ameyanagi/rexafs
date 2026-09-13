---
title: "TypeScript · Spectrum"
description: "Spectrum declarations and JSDoc."
audience: user
pagefind: false
---


**Next API · unreleased.** These signatures describe the source checkout, not npm rexafs@0.2.4.

[Installation and version guide](/docs/reference/) · [TypeScript tutorial](/docs/libraries/typescript/)

Mutable Rust spectrum. Stages run synchronously and return the same object.
Missing prerequisites use the selected algorithms and their defaults.
Array getters return independent copies, or undefined before their stage runs.

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts)

## constructor

```typescript
constructor(energy: Float64Array, mu: Float64Array);
```

Copy energy (eV) and absorption mu into a spectrum. Inputs must be finite, one-dimensional, equal-length, with strictly increasing energy.

## from_arrays

```typescript
static from_arrays(energy: Float64Array, mu: Float64Array): Spectrum;
```

Create a spectrum from energy (eV) and absorption mu. Copies input arrays; equivalent to the constructor.

## free

```typescript
free(): void;
```

Release native memory. Do not use the object afterwards.

## set_spectrum

```typescript
set_spectrum(energy: Float64Array, mu: Float64Array): this;
```

Replace energy (eV) and mu, copy the inputs and clear E0 and derived results. Returns this spectrum.

## set_e0

```typescript
set_e0(e0: number): this;
```

Set edge energy in eV and invalidate normalization and downstream results. Returns this spectrum.

## set_normalization_method

```typescript
set_normalization_method(method?: PrePostEdge | NormalizationMethod | null): this;
```

Copy normalization settings and invalidate normalization and downstream results. Accepts PrePostEdge directly or a NormalizationMethod; omitted/undefined restores automatic pre/post-edge normalization.

## set_background_method

```typescript
set_background_method(method?: AUTOBK | BackgroundMethod | null): this;
```

Copy background settings and invalidate background and downstream results. Accepts AUTOBK directly or a BackgroundMethod; omitted/undefined restores default AUTOBK.

## set_ifft

```typescript
set_ifft(parameters: XrayFFTR): this;
```

Copy inverse-transform settings; clear q and chi(q) while preserving forward results. Returns this spectrum.

## set_fft

```typescript
set_fft(parameters: XrayFFTF): this;
```

Copy forward-transform settings; clear Fourier and inverse results while preserving normalization and chi(k). Returns this spectrum.

## e0

```typescript
e0(): number | undefined;
```

Edge energy in eV, or undefined before detection or assignment.

## find_e0

```typescript
find_e0(): this;
```

Detect edge energy from mu and invalidate dependent results. Returns this spectrum.

## normalize

```typescript
normalize(): this;
```

Run pre/post-edge normalization, finding E0 if needed. Returns this spectrum.

## calc_background

```typescript
calc_background(): this;
```

Run AUTOBK, computing missing normalization first. Returns this spectrum.

## fft

```typescript
fft(): this;
```

Compute chi(R), running missing normalization and AUTOBK first. Defaults: k=2..15 inverse angstroms, kweight=2, KaiserBessel, nfft=2048. Returns this spectrum.

## ifft

```typescript
ifft(): this;
```

Back-transform chi(R) to chi(q), running missing forward stages first. Configure the R window with set_ifft(new XrayFFTR(...)). Returns this spectrum.

## invalidate_derived

```typescript
invalidate_derived(): this;
```

Clear all calculated results while retaining stage settings for recomputation. Returns this spectrum.

## k

```typescript
k(): Float64Array | undefined;
```

Uniform background k axis in inverse angstroms; pairs with chi(). Returns an independent array copy, or undefined before its stage runs.

## chi

```typescript
chi(): Float64Array | undefined;
```

Unweighted EXAFS chi(k) = (mu - smooth background) / edge_step; dimensionless and paired with k(). Returns an independent array copy, or undefined before its stage runs.

## norm

```typescript
norm(): Float64Array | undefined;
```

Normalized absorption (mu - pre_edge) / edge_step on the input energy grid; dimensionless. Returns an independent array copy, or undefined before its stage runs.

## flat

```typescript
flat(): Float64Array | undefined;
```

Normalized absorption with its fitted post-edge trend removed, preserving the edge value; dimensionless. Returns an independent array copy, or undefined before its stage runs.

## pre_edge

```typescript
pre_edge(): Float64Array | undefined;
```

Fitted pre-edge baseline in mu units on the input energy grid. Returns an independent array copy, or undefined before its stage runs.

## post_edge

```typescript
post_edge(): Float64Array | undefined;
```

Fitted post-edge baseline in mu units on the input energy grid. Returns an independent array copy, or undefined before its stage runs.

## r

```typescript
r(): Float64Array | undefined;
```

Forward-transform R axis in angstroms; pairs with chir_mag/real/imag(). Peaks are not phase-corrected bond lengths. Returns an independent array copy, or undefined before its stage runs.

## kwin

```typescript
kwin(): Float64Array | undefined;
```

Forward Fourier window values; use kwin_k() for the matching axis. Returns an independent array copy, or undefined before its stage runs.

## kwin_k

```typescript
kwin_k(): Float64Array | undefined;
```

Forward Fourier window k axis in inverse angstroms; may differ from k() with grid=Larch. Returns an independent array copy, or undefined before its stage runs.

## chir_mag

```typescript
chir_mag(): Float64Array | undefined;
```

Magnitude of chi(R); pairs with r(). Returns an independent array copy, or undefined before its stage runs.

## chir_real

```typescript
chir_real(): Float64Array | undefined;
```

Real component of chi(R); pairs with r(). Returns an independent array copy, or undefined before its stage runs.

## chir_imag

```typescript
chir_imag(): Float64Array | undefined;
```

Imaginary component of chi(R); pairs with r(). Returns an independent array copy, or undefined before its stage runs.

## q

```typescript
q(): Float64Array | undefined;
```

Back-transform q axis in inverse angstroms; pairs with chiq(). Returns an independent array copy, or undefined before its stage runs.

## chiq

```typescript
chiq(): Float64Array | undefined;
```

Real R-filtered signal on q(); forward k-weighting and windowing remain, so this is not generally the unweighted chi(k). Returns an independent array copy, or undefined before its stage runs.
