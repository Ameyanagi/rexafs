---
title: "TypeScript · Spectrum"
description: "Spectrum declarations, defaults and API explanations."
audience: user
pagefind: true
---

**Stable 0.2.4.** These signatures match the released npm package. Explanations are maintained in source JSDoc and reviewed against this release.

[Installation and version guide](/docs/reference/) · [TypeScript tutorial](/docs/libraries/typescript/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/v0.2.4/js-rexafs/types.d.ts) · [JSDoc source](https://github.com/Ameyanagi/rexafs/blob/main/js-rexafs/types.d.ts)

Mutable absorption spectrum processed by the Rust engine in WebAssembly.

Construct from finite, equal-length Float64Arrays containing strictly increasing photon
energy in eV and absorption mu. The supplied absorption scale is retained until edge-step
normalization. Arrays and stage settings are copied, so callers keep ownership of their
inputs.

Stages run synchronously and return this same object for chaining. fft() computes missing
normalization and AUTOBK stages using the selected settings; ifft() additionally computes the
forward transform if absent. Changing or rerunning a stage clears dependent results. A stage
error throws; inspect or correct the inputs/settings before retrying.

Result getters never run processing. They return independent Float64Array copies, or
undefined before their stage runs and after invalidation. Copies remain valid after free().
Browser callers must await init() before constructing objects, and should use a Web Worker
for long calculations. Release every spectrum and settings/wrapper object with free() when
finished.

See [processing theory](https://rexafs.com/docs/science/processing/) for equations,
assumptions and interpretation.

## constructor

```typescript
constructor(energy: Float64Array, mu: Float64Array);
```

Copy measured photon energy in eV and absorption mu into a new, initially unprocessed
spectrum. Both inputs must be Float64Arrays of equal length, with at least two finite
samples and strictly increasing energy. Duplicate or decreasing energies are rejected.
Throws TypeError for other array types, and Error for invalid data or uninitialized browser
Wasm. Processing may need more samples than construction; free() the spectrum when
finished.

## from_arrays

```typescript
static from_arrays(energy: Float64Array, mu: Float64Array): Spectrum;
```

Create a new spectrum by copying measured energy in eV and absorption mu. Equivalent to new
Spectrum(energy, mu), including Float64Array type checks and the requirement for at least
two finite, strictly increasing energy samples of matching length. No processing runs
automatically at construction; free() the result when finished.

## free

```typescript
free(): void;
```

Release this object's Wasm allocation. Do not call methods, read fields or free it again
afterwards. Arrays and settings already copied elsewhere remain valid.

## set_spectrum

```typescript
set_spectrum(energy: Float64Array, mu: Float64Array): this;
```

Replace measured energy (eV) and absorption mu with independent copies. Uses the same
validation as the constructor and preserves the previous spectrum if input validation
fails. Clears E0 and all calculated results while retaining stage settings. Returns this
spectrum; recompute the desired stages after replacement.

## set_e0

```typescript
set_e0(e0: number): this;
```

Assign a finite edge energy in eV. Clears normalization, background, forward and inverse
results, and updates the selected normalization/background energy origins. The value is not
checked against the measured range until processing. Throws TypeError for a non-number or
RangeError for a nonfinite value. Returns this spectrum.

## set_normalization_method

```typescript
set_normalization_method(method?: NormalizationMethod | null): this;
```

Copy the selected normalization method and clear normalization, background, forward and
inverse results. A specified method E0 overrides the spectrum E0; otherwise the existing E0
is retained. The caller keeps ownership of the settings and wrapper and may free them after
assignment. Later edits require reassignment. Returns this spectrum.

Omitting the argument, undefined or null restores automatic pre/post-edge settings while
retaining the selected E0. These reset forms work in stable 0.2.4 and Next. For custom
settings, stable 0.2.4 accepts a NormalizationMethod wrapper; Next (the source checkout)
also accepts PrePostEdge directly.

## set_background_method

```typescript
set_background_method(method?: BackgroundMethod | null): this;
```

Copy the selected background method and clear background, forward and inverse results while
retaining normalization. The caller keeps ownership of the settings and wrapper and may
free them after assignment. Later edits require reassignment. Returns this spectrum.

Omitting the argument, undefined or null restores default AUTOBK settings. These reset
forms work in stable 0.2.4 and Next. For custom settings, stable 0.2.4 accepts a
BackgroundMethod wrapper; Next (the source checkout) also accepts AUTOBK directly. This
does not reset forward or inverse configuration values that were already resolved
automatically.

## set_fft

```typescript
set_fft(parameters: XrayFFTF): this;
```

Copy forward-transform settings and clear r(), chir_*(), kwin(), kwin_k(), q() and chiq().
Normalization and background k()/chi() are preserved. Settings can be freed after
assignment; later edits require reassignment. Invalid numerical settings are reported when
fft() runs. Returns this spectrum. Assign settings with kstep undefined to request fresh
spacing inference, for example after changing AUTOBK.kstep. Inverse settings are retained;
if their spacing was already resolved, they may also need replacement before ifft().

## e0

```typescript
e0(): number | undefined;
```

Return the selected or detected absorption-edge energy in eV, or undefined before
assignment/detection. Reading this value does not detect an edge or run any processing.

## find_e0

```typescript
find_e0(): this;
```

Estimate the absorption-edge energy from the derivative of measured mu(E), including local
smoothing/refinement. Clears normalization and all downstream results, then returns this
spectrum. Throws if the input cannot be used for edge detection. Inspect the result for
noisy, multiple-edge or unusual spectra and use set_e0() for an explicit choice.

## normalize

```typescript
normalize(): this;
```

Fit the selected pre/post-edge model, find E0 if needed, and calculate dimensionless
norm()/flat() plus pre_edge()/post_edge() baselines on the original energy grid. Clears
background and all Fourier results even when normalization was already present. Uses
automatic PrePostEdge defaults if no method was selected. Throws on unsupported methods or
failed baseline fits. Returns this spectrum.

## calc_background

```typescript
calc_background(): this;
```

Fit the selected smooth background and calculate unweighted, dimensionless chi(k) on k().
Runs missing normalization first and uses default AUTOBK if no background method was
selected. Clears forward and inverse results on every call. Throws on invalid parameters,
unsupported methods, insufficient data or a failed spline solve. Returns this spectrum.

## fft

```typescript
fft(): this;
```

Calculate complex chi(R) from chi(k), computing missing normalization and background first.
Defaults: k=2..15 inverse angstroms, kweight=2, KaiserBessel, nfft=2048 and kstep inferred
from the background grid. The unnormalized forward DFT is multiplied by kstep/sqrt(pi),
with no additional 1/N; XrayFFTF explains the formula and units. Clears inverse results and
throws on invalid settings or failed prerequisite stages. Returns this spectrum.

## ifft

```typescript
ifft(): this;
```

Calculate real chi(q) by windowing the retained complex Fourier bins in R and performing a
conjugate-symmetric inverse transform. Runs missing forward and prerequisite stages first.
Forward k-weighting and windowing remain in the output, so this is not generally unweighted
chi(k). Throws on inconsistent transform settings or failed prerequisite stages. Returns
this spectrum. At least two reported R samples are required even though filtering uses the
full internal Fourier bins; rmax_out=0 therefore fails. When reusing a spectrum with a
different forward grid, reset previously resolved inverse settings in Next, or create a
fresh spectrum in stable 0.2.4.

## invalidate_derived

```typescript
invalidate_derived(): this;
```

Clear normalization, background, forward and inverse calculated arrays without discarding
the measured inputs, selected E0 or stage settings. User-specified edge-step overrides are
retained; a previously estimated step is recomputed by the next normalization. Returns this
spectrum. Subsequent getters return undefined until their stages run again. Resolved
automatic settings, such as FFT kstep, are retained. Reassign the affected stage settings
to request fresh automatic values for changed input grids.

## k

```typescript
k(): Float64Array | undefined;
```

Return an independent copy of the uniform background k axis in inverse angstroms, beginning
at zero and paired with chi(). Its spacing is AUTOBK.kstep (default 0.05). Returns
undefined before background removal or after invalidation; this getter never runs a stage.

## chi

```typescript
chi(): Float64Array | undefined;
```

Return an independent copy of unweighted EXAFS chi(k) = (mu - smooth background) /
edge_step, dimensionless and paired with k(). The measured absorption and smooth background
are resampled according to the selected background method. Returns undefined before
background removal or after invalidation. Forward kweight and window settings do not change
this array.

## norm

```typescript
norm(): Float64Array | undefined;
```

Return an independent copy of dimensionless normalized absorption, (mu - pre_edge) /
edge_step, on the original input energy grid. Here pre_edge is the fitted baseline and
edge_step is the selected or fitted absorption jump. Returns undefined before normalization
or after invalidation; does not calculate missing results.

## flat

```typescript
flat(): Float64Array | undefined;
```

Return an independent copy of dimensionless flattened absorption on the input energy grid.
Above E0 this subtracts the fitted post-edge trend from norm(), with an offset that
preserves the value at the edge; below E0 it equals norm(). This is a
presentation/near-edge quantity, not the background chi(k). Returns undefined before
normalization or after invalidation.

## pre_edge

```typescript
pre_edge(): Float64Array | undefined;
```

Return an independent copy of the fitted pre-edge baseline, in the same units as input mu
and evaluated across the entire original energy grid. The fit uses the selected pre-edge
interval, and its extrapolation is subtracted during normalization. Returns undefined
before normalization or after invalidation.

## post_edge

```typescript
post_edge(): Float64Array | undefined;
```

Return an independent copy of the fitted post-edge baseline in input mu units, evaluated on
the original energy grid. It includes the pre-edge baseline plus the fitted polynomial for
pre-edge-subtracted absorption. It determines the edge step and flattening trend; it is not
the AUTOBK background. Returns undefined before normalization or after invalidation.

## r

```typescript
r(): Float64Array | undefined;
```

Return an independent copy of the reported Fourier R axis in angstroms, paired with
chir_mag(), chir_real() and chir_imag(). Its spacing is pi/(nfft*kstep), and its extent is
limited by rmax_out and the available positive-frequency bins. Peaks are not
phase-corrected bond lengths. Returns undefined before fft() or after invalidation.

## kwin

```typescript
kwin(): Float64Array | undefined;
```

Return an independent copy of the dimensionless forward window values, paired with
kwin_k(). These values exclude the k^kweight factor. The array can have a different
length/grid from background k()/chi() with grid=Larch. Returns undefined before fft() or
after invalidation.

## kwin_k

```typescript
kwin_k(): Float64Array | undefined;
```

Return an independent copy of the forward window axis in inverse angstroms, paired with
kwin(). Input uses the prepared background grid; Larch returns its resampled and possibly
extended window grid. This getter does not alter the background k()/chi() arrays. Returns
undefined before fft() or after invalidation.

## chir_mag

```typescript
chir_mag(): Float64Array | undefined;
```

Return an independent copy of the magnitude sqrt(real^2 + imag^2) of complex chi(R), paired
with r(). For dimensionless chi and forward kweight w, units are angstrom^(-(w+1)); at w=2
they are inverse cubic angstroms. No peak-height or window-area normalization is applied.
Returns undefined before fft() or after invalidation.

## chir_real

```typescript
chir_real(): Float64Array | undefined;
```

Return an independent copy of the real component of chi(R), paired with r(). Units are
angstrom^(-(w+1)), where w is the forward kweight and chi is dimensionless. Uses the
negative-exponent forward DFT with amplitude factor kstep/sqrt(pi). Returns undefined
before fft() or after invalidation.

## chir_imag

```typescript
chir_imag(): Float64Array | undefined;
```

Return an independent copy of the imaginary component of chi(R), paired with r(). Units are
angstrom^(-(w+1)), where w is the forward kweight and chi is dimensionless. Its sign
follows exp(-2*pi*i*j*m/nfft); reversing the Fourier convention changes that sign. Returns
undefined before fft() or after invalidation.

## q

```typescript
q(): Float64Array | undefined;
```

Return an independent copy of the real back-transform axis in inverse angstroms, beginning
at zero and paired with chiq(). Its spacing follows the inverse FFT settings and the
forward R grid; q distinguishes this possibly filtered/resized grid from background k().
Returns undefined before ifft() or after invalidation.

## chiq

```typescript
chiq(): Float64Array | undefined;
```

Return an independent copy of the real R-filtered signal, paired with q(). Forward
k-weighting and windowing remain, so this is not generally unweighted chi(k). With
dimensionless chi, forward kweight w and inverse rweight v, units are angstrom^(v-w);
ordinary v=0 retains the units of k^w*chi. Returns undefined before ifft() or after
invalidation.
