# Cauchy wavelet analysis

Unreleased source-checkout feature. The native core implements `cauchy_v1`;
desktop and binding integration are still being qualified. This page does not
claim that a released GUI already provides the complete workflow.

A wavelet map localizes EXAFS oscillations jointly in photoelectron wave number
k (Å⁻¹) and Fourier-like distance R (Å). It helps compare contributions with
different k dependence. The scientific motivation is
[Muñoz, Argoul and Farges (2003), *Continuous Cauchy wavelet transform analyses of EXAFS spectra: a qualitative approach*](https://doi.org/10.2138/am-2003-0423).
R features are not phase-corrected bond lengths; color intensity is not a
concentration or coordination number. [Larch's documentation](https://xraypy.github.io/xraylarch/xafs_wavelets.html)
provides a related Cauchy implementation and examples.

## Common API

```rust,no_run
use rexafs::{Spectrum, Wavelet};
# fn example(spectrum: &Spectrum) -> Result<(), Box<dyn std::error::Error>> {
let map = spectrum.wavelet(&Wavelet::new(2.0..=12.0))?;
let magnitude = map.magnitude();
let region = map.integral(4.0..=10.0, 1.0..=3.0)?;
# Ok(()) }
```

The selected k interval must be fully measured. Missing normalization and AUTOBK
run automatically on a copy, using the spectrum's settings; the source remains
unchanged. Existing unweighted χ(k) is reused. The ordinary Fourier transform
and its display window do not run implicitly. A native fluorescence-corrected
XANES-only branch is rejected as unqualified for EXAFS.

`Wavelet::new` defaults to weight 2, order 100, k sampling 0.05 Å⁻¹, R up to 6 Å
and no taper. These are rexafs starting choices, not universal optimal settings.
Use `.kweight(0)`, `.order(50)`, `.kstep(0.025)`, `.rmax(4.0)` or `.taper(1.0)` to
change named settings. Larger order narrows the frequency response and broadens
localization in k. Weighting emphasizes high-k oscillations and noise. The taper
uses half-cosine ramps of the supplied width **inside** both selected endpoints;
it is explicitly distinct from the historical FFT window-width convention.

The array API is `settings.calculate(k, chi)`, where χ is unweighted and
dimensionless. Original arrays, requested settings, prepared grid, support mask,
window and complex result remain available. `map.real()` and `map.imaginary()`
borrow immutable flattened arrays, with **rows=R and columns=k**. Cell (j,i) is at
`j * map.k().len() + i`. `map.magnitude()` returns an independent array.
`map.phase(0.01)` returns radians, masking cells below 1% of the map maximum;
zero-magnitude cells are always masked. This changes neither complex data nor
region measurements. Magnitude slices use `slice_at_k(k)` and `slice_at_r(r)`.
Serde JSON preserves the scientific map; deserialization checks method, dimensions,
axes, finite values and resource limits. It does not independently prove that an
external producer's numerical values are correct.

## Numerical convention

The native implementation is
[`wavelet::calculate`](../crates/rexafs/src/xafs/wavelet/calculate.rs).
It linearly resamples onto a uniform zero-origin k grid with spacing δk. Only the
selected measured support contributes; values outside it are recorded zero
padding, not extrapolated observations. The grid ends at the last represented
uniform point within the original k extent. Define

```text
u_j = k_j^w χ(k_j) window(k_j)
U_l = sum_j u_j exp(−2π i j l / L)
ω_l = 2π l / (L δk)
a_R = m / (2R)
H_l(R) = [2π / Γ(m+1)] (a_R ω_l)^m exp(−a_R ω_l),  0 < l < L/2
W_j(R) = (1/L) sum_l U_l H_l(R) exp(+2π i j l / L)
```

Here j indexes k samples, l indexes discrete frequency, L is FFT length, w is k
weight and m is positive integer order. Γ(m+1)=m!. δk and a_R have Å⁻¹ units;
ω and R have Å units, so a_Rω is dimensionless. DC, Nyquist and negative-frequency
filter bins are zero. The filter peaks at ω=2R, matching the EXAFS 2kR coordinate.
The FFT has no forward multiplier and the inverse has factor 1/L. W therefore
has numerical units of k^wχ, unlike the ordinary XAFS Fourier transform's scaling.
The filter is evaluated in the log domain to avoid factorial/power overflow.
The existing easyfft/RustFFT dependency supplies Fourier transforms; no Python
runtime or new numerical dependency is required.

This discrete filter, explicit support and fixed order define rexafs
`cauchy_v1`. R=0 is excluded as singular. The default FFT is the smallest power
of two at least twice the prepared grid length. A requested shorter FFT fails;
measured data are never silently truncated. Default R spacing is π/(Lδk),
numerical sampling rather than independent spatial resolution. `.rstep(...)`
changes it; `.radii(vec![...])` supplies exact positive increasing coordinates
for comparisons. R must remain below the k-sampling Nyquist radius π/(2δk).
Extending R at fixed order/grid leaves overlapping values identical. Cropping,
color scaling and image resolution must operate on a separate display view.
Finite support and padding still produce boundary artifacts; inspect sensitivity
to the chosen support and taper.

The [pinned Larch implementation](https://github.com/xraypy/xraylarch/blob/e3c93284fed358c2c8979cba4c139430527433c6/larch/xafs/cauchy_wavelet.py)
couples order to the number of R rows and uses an actual FFT length twice its
`nfft` argument. Thus reference tests supply its exact order, coordinates and
actual length explicitly. They establish matched-convention agreement, not
identity between the two applications' defaults. The reference source preserves
historical Univ. Marne la Vallee/Hans–Argoul/Argoul–Muñoz/Farges attribution;
this is recorded with the fixture and upstream license.

## Region measurements and bounded calculation

`map.integral(k_range, r_range)` integrates the bilinear surface through **native
magnitude samples** over a fully covered rectangle. It inserts exact requested
boundaries and sums the integrals of clipped bilinear cells. k must also lie
within selected measured support; phase is not an integral metric. The value
has units of |W| times Å⁻¹ times Å, which reduce to those of k^wχ. No uncertainty
is supplied. [`wavelet::map`](../crates/rexafs/src/xafs/wavelet/map.rs) implements
integration, slices, immutable access and deserialization validation.

`settings.estimate(k)` checks dimensions and estimates buffer size before any
transform. Limits are 1 million original samples, 32768 prepared k samples/R rows,
262144 FFT points, 4 million complex cells and 64 million R-rows-times-FFT-points
of filtering work. Invalid grids fail before large allocation. Original-input
copies, optional preparation metadata, FFT scratch and serialization add overhead
to the reported scientific buffer size. A map is calculated one R row at a time;
`calculate_with_cancel` checks a callback between rows and discards partial output
on cancellation. `spectrum.wavelet_with_cancel` checks before/after prerequisite
processing too; it does not interrupt an ongoing AUTOBK solve.

For series use, process every requested frame with bounded map residency, retain
scalar region measurements and preserved inputs/settings, and recompute selected
maps on demand. This is the integration contract; a series/Live GUI workflow is
not completed by the core implementation alone.

## Qualification

Tests compare Fourier sign, axes and amplitude with independent direct DFT sums;
localize a synthetic packet near its known k/R position; verify identical values
under fixed-order R extension; and exercise irregular interpolation, explicit
padding/taper, cancellation, zero amplitude, invalid grids and memory limits.
An analytic bilinear surface checks region integration with non-grid boundaries.
The high-level spectrum test verifies automatic preparation, unchanged input,
retained preparation settings and replay from the original k/χ arrays.

The [synthetic Larch fixture](../crates/rexafs/tests/fixtures/analysis/wavelet/README.md)
contains two matched-grid cases with distinct weights/orders, original source
hash and package version. Test fixtures stay in Git and are excluded from crate
archives. No unpublished experimental data are used. Experimental interpretation,
platform GUI workflows and display/series integration require their own checks.
