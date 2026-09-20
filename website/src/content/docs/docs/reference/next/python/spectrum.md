---
title: "Python · Spectrum"
description: "Spectrum signatures, defaults and API explanations."
audience: user
pagefind: false
---

**Next API · unreleased.** This reference describes the source checkout. Compare with stable rexafs 0.2.12 before using it with an installed package.

[Installation and version guide](/docs/reference/) · [Python tutorial](/docs/libraries/python/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/main/py-rexafs/python/rexafs/__init__.pyi) · [Docstring source](https://github.com/Ameyanagi/rexafs/blob/main/py-rexafs/python/rexafs/__init__.pyi)

Own a measured absorption spectrum and its calculated processing stages.

Energy is in eV; absorption mu may be an absorption coefficient or a
consistently scaled measurement such as transmission optical depth.
Input arrays are copied to Rust-owned float64 storage. Returned NumPy
arrays are independent copies, so editing them does not change this
spectrum. Each result getter returns None until its stage has succeeded.

Start with Spectrum(energy, mu).fft() to run automatic normalization,
AUTOBK background removal and the k-to-R transform. Explicit processing
calls recompute their stage and return this same object; missing
prerequisites run automatically with the selected settings. Calculation
methods release Python's global interpreter lock while Rust runs.

Changing input data or settings clears dependent results. A failed stage
does not substitute another algorithm. Invalid input and normalization
errors raise ValueError; background and Fourier errors raise RuntimeError.
See [processing theory](https://rexafs.com/docs/science/processing/) for
equations, interpretation and limitations. Groups and structural fitting
are not currently exposed by this Python Spectrum API.

## correct_fluorescence

```python
correct_fluorescence(self, model: FluorescenceCorrection) -> Spectrum
```

Correct into an independent unnormalized Spectrum (since 0.2.10).
Internal conventional normalization runs automatically; the source stays
unchanged. Unknown acquisition provenance is explicitly interpreted as
fluorescence. Known transmission, prepared norm/flat and repeated correction
raise ValueError. Supply line and measured surface angles in the model.
Releases the GIL. Call normalize() for separate final polynomial/MBACK
normalization. History survives edits; this XANES-only branch rejects
background/FFT/wavelets. Corrected-array uncertainty is unavailable.

## fluorescence_correction

```python
fluorescence_correction(self) -> FluorescenceCorrectionResult | None
```

Owned historical correction record, or None. Later edits/normalization
never rewrite its original inputs or remove the XANES-only restriction.

## absorption_mode

```python
absorption_mode(self) -> AbsorptionMode
```

Acquisition interpretation: unknown, transmission or fluorescence.

## set_absorption_mode

```python
set_absorption_mode(self, mode: AbsorptionMode) -> Spectrum
```

Explicitly revise acquisition interpretation and return this Spectrum.
Arrays/caches stay unchanged. Correction history and restrictions survive.

## wavelet

```python
wavelet(self, model: Wavelet) -> WaveletMap
```

Since 0.2.10: spectrum.wavelet(Wavelet((2, 12))) prepares missing
normalization/AUTOBK on a copy, reusing existing chi. The interval uses
inverse angstroms. Arrays/settings/caches are unchanged; Rust releases
the GIL. Result matrices are owned (R rows, k columns). Invalid coverage,
grids and unqualified corrected XANES input raise ValueError. R is not
phase-corrected; color intensity is not a concentration.

## fit_peaks

```python
fit_peaks(self, model: PeakFit, *, errors: NDArray[np.float64] | Sequence[float] | None=None) -> PeakFitResult
```

Fit a composite XANES model, preparing missing normalization on a copy (since 0.2.10).

Example: spectrum.fit_peaks(PeakFit((-20, 40)).gaussian("p1", 5, 2, 3)).
Defaults are Norm, E0-relative eV and 200 iterations. Source arrays, settings,
caches and initial model remain unchanged; Rust releases the GIL. Results
contain data/model/residual arrays on the retained native points. No smoothing
or interpolation occurs. Invalid models, coverage or preparation raise ValueError.
A result can be nonconverged: inspect termination, warnings and uncertainty_unavailable.

Optional errors are positive independent standard deviations in the SELECTED
signal representation on the original native grid, including excluded points.
Raw detector errors are not propagated through normalization. Without errors,
covariance uses residual-based variance. Active bounds, deficient rank and
nonconvergence withhold conditional local uncertainty; this is not model confidence.

## measure

```python
measure(self, operation: Literal['point'], coordinates: float, *, space: Literal['mu', 'norm', 'flat', 'chi', 'fourier']='norm', origin: Literal['e0', 'absolute'] | None=None, kweight: int=0, errors: NDArray[np.float64] | Sequence[float] | None=None) -> MeasurementResult
```

Measure a region on a private copy (since 0.2.10).

Recommended: spectrum.measure("mean", (-20, 30)). Defaults to normalized
mu and E0-relative energy offsets in eV; select space="flat" explicitly.
k is in inverse angstroms and R in angstroms, without phase correction;
these require absolute coordinates (selected automatically when origin=None).
kweight defaults to zero and applies only to chi. Bounds must increase.
Mean is the piecewise-linear integral divided by interval width, not an
arithmetic sample mean. Integral uses trapezoids and interpolated endpoints.
Missing stages run automatically using this spectrum's settings, with the
GIL released. Arrays, settings and cached results remain unchanged. No
extrapolation or display sampling occurs. Invalid input, missing coverage,
or unavailable preparation raises ValueError. Results own their values.

errors supplies independent standard deviations of the SELECTED signal on
its native grid, in its signal units, finite and nonnegative. Raw-count
errors are not propagated through normalization or transforms. Point,
integral and mean support this model; maximum rejects it. Axis, E0 and
settings are exact. No correlations or confidence intervals are inferred;
standard_error is None when errors are omitted.

## measure

```python
measure(self, operation: Literal['mean', 'integral', 'maximum'], coordinates: tuple[float, float], *, space: Literal['mu', 'norm', 'flat', 'chi', 'fourier']='norm', origin: Literal['e0', 'absolute'] | None=None, kweight: int=0, errors: NDArray[np.float64] | Sequence[float] | None=None) -> MeasurementResult
```

Measure a region on a private copy (since 0.2.10).

Recommended: spectrum.measure("mean", (-20, 30)). Defaults to normalized
mu and E0-relative energy offsets in eV; select space="flat" explicitly.
k is in inverse angstroms and R in angstroms, without phase correction;
these require absolute coordinates (selected automatically when origin=None).
kweight defaults to zero and applies only to chi. Bounds must increase.
Mean is the piecewise-linear integral divided by interval width, not an
arithmetic sample mean. Integral uses trapezoids and interpolated endpoints.
Missing stages run automatically using this spectrum's settings, with the
GIL released. Arrays, settings and cached results remain unchanged. No
extrapolation or display sampling occurs. Invalid input, missing coverage,
or unavailable preparation raises ValueError. Results own their values.

errors supplies independent standard deviations of the SELECTED signal on
its native grid, in its signal units, finite and nonnegative. Raw-count
errors are not propagated through normalization or transforms. Point,
integral and mean support this model; maximum rejects it. Axis, E0 and
settings are exact. No correlations or confidence intervals are inferred;
standard_error is None when errors are omitted.

## Spectrum

```python
Spectrum(energy: ArrayLike, mu: ArrayLike)
```

Create a spectrum by copying energy and mu into float64 storage.

energy contains X-ray energies in eV; mu contains the corresponding
absorption values in consistent units. Lists, NumPy arrays and strided
views are accepted. Inputs must be one-dimensional, finite, equal-length,
have at least two samples, and have strictly increasing energy.
Invalid shapes or data raise ValueError. No processing runs here;
call fft() for the default pipeline or normalize() for just normalization.

The two-sample minimum only permits storage. Automatic edge detection
requires at least three samples, and baseline/spline fitting needs enough
points on the appropriate sides of the edge. Supply real numeric input;
NumPy conversion errors for unsupported objects propagate to the caller.

## from_arrays

```python
from_arrays(energy: ArrayLike, mu: ArrayLike) -> Spectrum
```

Create a spectrum from energy in eV and corresponding absorption mu.

Equivalent to Spectrum(energy, mu): it accepts array-like inputs and
copies them to owned float64 storage. Inputs must be finite, one-dimensional,
equal-length, with at least two samples and strictly increasing energy.
Invalid data raise ValueError. Derived results are initially unavailable.

As with the constructor, two samples are enough to create the object
but not enough for automatic edge detection or a useful EXAFS pipeline.
Array conversion follows NumPy's float64 conversion rules.

## set_spectrum

```python
set_spectrum(self, energy: ArrayLike, mu: ArrayLike) -> Spectrum
```

Replace measured energy/mu, copy the inputs and clear E0 and derived results.

Input units and validation match the constructor: energy in eV, finite
matching one-dimensional arrays with strictly increasing energy and at
least two samples. Invalid input raises ValueError before replacing data.
Stage settings are retained, but old calculated edge values and arrays
are discarded. Returns this spectrum; call a processing stage to recompute.

Unlike the QAS reader, this method rejects unordered input rather than
sorting it. Previously resolved automatic fit ranges and FFT spacings
are retained with the other settings. Reassign automatic settings if
the new scan needs those choices inferred again.

## set_e0

```python
set_e0(self, e0: float) -> Spectrum
```

Set the absorption edge energy in eV and clear all dependent results.

The value is propagated to configured normalization and AUTOBK settings,
so subsequent energy-to-k conversion uses the new edge. Use a finite
value strictly inside the measured energy range; validation occurs when
processing runs, not at this setter. Returns this spectrum without
performing normalization or recalibrating the input energy axis.

## set_normalization_method

```python
set_normalization_method(self, method: PrePostEdge | MBack | NormalizationMethod | None=None) -> Spectrum
```

Copy the selected normalization method and clear normalization and later results.

Since 0.2.5, PrePostEdge settings are accepted directly; omitted/None restores
automatic normalization. Published 0.2.4 uses
NormalizationMethod.PrePostEdge(parameters) or new_prepostedge().
An explicit E0 in those parameters becomes the spectrum E0; otherwise
an existing spectrum E0 is retained. Edits to the original settings do
not propagate: assign again to apply them. Returns this spectrum without
processing data; call normalize() or a later stage to recompute.

## set_background_method

```python
set_background_method(self, method: AUTOBK | BackgroundMethod | None=None) -> Spectrum
```

Copy the selected background method and clear background and Fourier results.

Since 0.2.5, AUTOBK settings are accepted directly; omitted/None restores
default AUTOBK. Published 0.2.4 uses BackgroundMethod.AUTOBK(parameters)
or BackgroundMethod.new_autobk() for recommended defaults. Existing
normalization results are retained. Later edits to the original settings
do not propagate: assign them again to apply changes. Returns this
spectrum without fitting; call calc_background() or a later stage.

If changing kstep after fft() has already run, also reassign XrayFFTF
settings with kstep=None. Clearing the Fourier results does not reset
its previously resolved automatic spacing.

## set_ifft

```python
set_ifft(self, parameters: XrayFFTR) -> Spectrum
```

Copy inverse settings and clear q()/chiq(), preserving forward results.

Use XrayFFTR to choose the R window and output q range. Editing the
original settings later does not change this spectrum; assign again
to apply changes. Returns this spectrum without filtering. Call ifft()
to compute the new result. This binding was added in 0.2.5.

## set_fft

```python
set_fft(self, parameters: XrayFFTF) -> Spectrum
```

Copy forward settings and clear Fourier and inverse results.

Normalization and the background k()/chi() arrays are retained. Use
XrayFFTF to choose k weights, the window and grid convention. Editing
the original settings later has no effect until you assign them again.
Returns this spectrum without transforming; call fft() to recompute.

If ifft() has already resolved its automatic kstep and this change
alters the R-grid spacing, reassign inverse settings with kstep=None
before the next inverse. This setter clears results, not the resolved
parameters of the inverse stage.

## e0

```python
e0(self) -> float | None
```

Return the detected or assigned edge energy E0 in eV, or None if unset.

This getter does not run edge detection. Call find_e0(), normalize() or
a later processing stage to resolve an automatic edge. E0 sets the
energy origin for k conversion; it is not an independent calibration
measurement or a fitted structural energy shift.

## find_e0

```python
find_e0(self) -> Spectrum
```

Estimate the absorption edge from the energy derivative of mu.

The detector seeks a strong rise with neighboring high-derivative
samples and refines its search around the candidate edge. It returns
this spectrum, stores E0 in eV and clears normalization and all later
results. Inspect the result for noisy spectra or multiple edges;
automatic detection is not energy calibration. Invalid data raise ValueError.

At least three energy/mu samples are required, even though the
constructor can store two. An insufficient scan raises ValueError.

## normalize

```python
normalize(self) -> Spectrum
```

Fit pre/post-edge baselines and compute dimensionless normalized absorption.

The implemented method computes norm = (mu - pre_edge) / edge_step
and flat with its fitted post-edge trend removed. Missing E0 and
automatic parameters are resolved from the data. Call norm(), flat(),
pre_edge() and post_edge() to retrieve independent result arrays.
This recomputes normalization, clears background/Fourier results and
returns this spectrum. Invalid ranges, failed fits and an empty MBack selector raise ValueError.

## mback_result

```python
mback_result(self) -> MbackResult | None
```

Copy the latest full MBACK result, or None when absent/invalidated.

Arrays and diagnostics remain independent after further processing.

## calc_background

```python
calc_background(self) -> Spectrum
```

Fit the selected smooth background and calculate dimensionless chi(k).

Missing normalization runs first. The default AUTOBK method suppresses
low-R Fourier content with rbkg=1.0 angstrom and the fixed endpoint
penalty; k() is a zero-origin grid with default step 0.05 inverse angstroms.
The returned chi() is unweighted. This recomputes the background, clears
forward/inverse results and returns this spectrum. Background failures
or ILPBkg raise RuntimeError; prerequisite normalization can raise ValueError.

## fft

```python
fft(self) -> Spectrum
```

Compute the weighted k-to-R Fourier transform, running missing prerequisites.

Defaults are k=2 to 15 inverse angstroms, kweight=2, a KaiserBessel
window and nfft=2048. The amplitude multiplier is kstep / sqrt(pi)
after an unnormalized negative-exponent FFT; there is no extra 1/nfft.
For dimensionless chi the default output units are inverse cubic
angstroms. XrayFFTF documents the full convention and sampling choices.

Use r() with chir_real(), chir_imag() or chir_mag(), and kwin_k() with
kwin(). This recomputes the forward transform, clears inverse results
and returns this spectrum. Background and FFT failures raise RuntimeError;
prerequisite normalization can raise ValueError.

## ifft

```python
ifft(self) -> Spectrum
```

Back-transform the stored Fourier signal to a real signal on q().

Missing forward stages run first. The inverse applies an R window and
reconstructs a real signal using conjugate negative-frequency bins.
The forward k weighting and window remain: chiq() is generally not
the original unweighted chi(k). With default forward kweight=2 and
inverse rweight=0, chiq() has units inverse square angstroms.
This recomputes the inverse and returns this spectrum. Invalid inverse
grids or settings raise RuntimeError; prerequisite errors propagate.

At least two entries must be present in the displayed r() array;
an overly small forward rmax_out can therefore prevent inversion.
After changing the forward R spacing, reassign inverse settings with
automatic kstep to resolve the new grid instead of retaining an old
resolved spacing. Configurable inverse settings were added in 0.2.5;
use a fresh spectrum in 0.2.4 after changing the forward grid.

## invalidate_derived

```python
invalidate_derived(self) -> Spectrum
```

Clear normalization, background and Fourier results, keeping stage settings.

The measured inputs and spectrum E0 are retained, as are explicit
settings needed for recomputation. Getters for cleared arrays return
None until their stages run again. Ordinary setters already invalidate
the affected results; use this method when you need a full recomputation.
Returns this spectrum without running any calculations.

Resolved fit ranges and FFT spacings are retained along with explicit
parameters. They are not restored to their original None values; reassign
fresh settings if you want automatic ranges or spacings inferred again.

## k

```python
k(self) -> NDArray[np.float64] | None
```

Uniform background k axis in inverse angstroms, paired with chi().

It begins at zero with AUTOBK.kstep spacing. It remains the background
grid even when the forward transform uses grid="Larch".

Returns an independent NumPy float64 array copy, or None before
calc_background() (or a dependent stage) succeeds or after invalidation.
Reading this result does not run processing; editing the copy does
not change the spectrum.

## chi

```python
chi(self) -> NDArray[np.float64] | None
```

Unweighted, dimensionless EXAFS oscillation on k().

chi is (mu - smooth background) / edge_step, resampled on the background
k grid. Neither the AUTOBK objective weight nor the later forward FFT
k weight is stored in this array.

Returns an independent NumPy float64 array copy, or None before
calc_background() (or a dependent stage) succeeds or after invalidation.
Reading this result does not run processing; editing the copy does
not change the spectrum.

## norm

```python
norm(self) -> NDArray[np.float64] | None
```

Dimensionless normalized absorption on the original energy grid.

norm = (mu - pre_edge) / edge_step, with the baseline and edge step in
the same units as mu. Use this for a normalized edge; flat() additionally
removes the fitted post-edge trend.

Returns an independent NumPy float64 array copy, or None before
normalize() (or a dependent stage) succeeds or after invalidation.
Reading this result does not run processing; editing the copy does
not change the spectrum.

## flat

```python
flat(self) -> NDArray[np.float64] | None
```

Dimensionless normalized absorption with the fitted post-edge trend removed.

Above the sample nearest E0, the fitted baseline difference divided
by edge_step is subtracted and its value at E0 added back. Below that
sample, flat equals norm. Values use the original energy grid; this
flattening is separate from AUTOBK background subtraction.

Returns an independent NumPy float64 array copy, or None before
normalize() (or a dependent stage) succeeds or after invalidation.
Reading this result does not run processing; editing the copy does
not change the spectrum.

## pre_edge

```python
pre_edge(self) -> NDArray[np.float64] | None
```

Fitted pre-edge baseline on the original energy grid, in mu units.

The line fitted to mu * E**n_victoreen is divided by E**n_victoreen.
It is extrapolated across the scan so normalization can subtract it.
Inspect this baseline against the measured pre-edge region.

Returns an independent NumPy float64 array copy, or None before
normalize() (or a dependent stage) succeeds or after invalidation.
Reading this result does not run processing; editing the copy does
not change the spectrum.

## post_edge

```python
post_edge(self) -> NDArray[np.float64] | None
```

Fitted post-edge baseline on the original energy grid, in mu units.

This is the fitted post-edge polynomial plus the pre-edge baseline.
Its difference from pre_edge near E0 estimates the normalization edge
step. It is not the AUTOBK smooth atomic background.

Returns an independent NumPy float64 array copy, or None before
normalize() (or a dependent stage) succeeds or after invalidation.
Reading this result does not run processing; editing the copy does
not change the spectrum.

## r

```python
r(self) -> NDArray[np.float64] | None
```

Forward-transform R axis in angstroms, paired with chir_real/imag/mag().

For FFT length N and k spacing delta_k, adjacent bins are separated
by pi / (N * delta_k); rmax_out limits the returned range. Zero padding
only refines sampling. Scattering phase shifts mean that an uncorrected
R peak is not directly a bond length.

Returns an independent NumPy float64 array copy, or None before
fft() (or a dependent stage) succeeds or after invalidation.
Reading this result does not run processing; editing the copy does
not change the spectrum.

## kwin

```python
kwin(self) -> NDArray[np.float64] | None
```

Dimensionless forward Fourier-window values, paired with kwin_k().

These are the window W before multiplying by chi and k**kweight.
Use the matching kwin_k() axis: with grid="Larch" the window can extend
beyond, and have a different length from, the background k() grid.

Returns an independent NumPy float64 array copy, or None before
fft() (or a dependent stage) succeeds or after invalidation.
Reading this result does not run processing; editing the copy does
not change the spectrum.

## kwin_k

```python
kwin_k(self) -> NDArray[np.float64] | None
```

k axis of the forward Fourier window, in inverse angstroms.

Pair it with kwin(). For grid="Input" it matches the background grid;
for grid="Larch" it uses the resampled, extended window domain. The
background k()/chi() arrays themselves are unchanged by this choice.

Returns an independent NumPy float64 array copy, or None before
fft() (or a dependent stage) succeeds or after invalidation.
Reading this result does not run processing; editing the copy does
not change the spectrum.

## chir_mag

```python
chir_mag(self) -> NDArray[np.float64] | None
```

Magnitude of the complex forward transform, paired with r().

It equals sqrt(chir_real()**2 + chir_imag()**2). For dimensionless chi
and integer forward kweight w, units are angstrom**(-(w + 1)); the
default w=2 gives inverse cubic angstroms. Magnitude discards phase
information and is not a probability density.

Returns an independent NumPy float64 array copy, or None before
fft() (or a dependent stage) succeeds or after invalidation.
Reading this result does not run processing; editing the copy does
not change the spectrum.

## chir_real

```python
chir_real(self) -> NDArray[np.float64] | None
```

Real component of the complex forward transform, paired with r().

The convention uses a negative-exponent FFT followed by kstep/sqrt(pi),
without additional FFT-length normalization. For dimensionless chi and
integer kweight w, units are angstrom**(-(w + 1)); the default w=2
gives inverse cubic angstroms.

Returns an independent NumPy float64 array copy, or None before
fft() (or a dependent stage) succeeds or after invalidation.
Reading this result does not run processing; editing the copy does
not change the spectrum.

## chir_imag

```python
chir_imag(self) -> NDArray[np.float64] | None
```

Imaginary component of the complex forward transform, paired with r().

Its sign follows the negative-exponent forward FFT, without an extra
phase rotation. For dimensionless chi and integer kweight w, units are
angstrom**(-(w + 1)); the default w=2 gives inverse cubic angstroms.
Keep the complex components when phase matters.

Returns an independent NumPy float64 array copy, or None before
fft() (or a dependent stage) succeeds or after invalidation.
Reading this result does not run processing; editing the copy does
not change the spectrum.

## q

```python
q(self) -> NDArray[np.float64] | None
```

Back-transform q axis in inverse angstroms, paired with chiq().

q describes the same physical variable as k, but labels a reconstructed
signal after R filtering. Its spacing is determined by the inverse FFT
length and the input R spacing; qmax_out limits the returned range.

Returns an independent NumPy float64 array copy, or None before
ifft() (or a dependent stage) succeeds or after invalidation.
Reading this result does not run processing; editing the copy does
not change the spectrum.

## chiq

```python
chiq(self) -> NDArray[np.float64] | None
```

Real R-filtered signal on q(), retaining the forward weighting and window.

This is generally not the original unweighted chi(k). For dimensionless
input chi, integer forward kweight w and inverse rweight v, its units
are angstrom**(v - w); defaults w=2 and v=0 give inverse square
angstroms. A selected R window removes contributions outside its taper.

Returns an independent NumPy float64 array copy, or None before
ifft() (or a dependent stage) succeeds or after invalidation.
Reading this result does not run processing; editing the copy does
not change the spectrum.
