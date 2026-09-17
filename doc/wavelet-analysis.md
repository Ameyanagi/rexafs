# Cauchy wavelet analysis

Unreleased source-checkout feature. The native core implements `cauchy_v1`;
the source desktop provides a single-spectrum map workspace, and Python and
TypeScript expose the same native calculation. Series/Live region tracking remains pending. This page does
not claim that a released GUI already provides the complete workflow.

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

## Python and TypeScript (unreleased)

The common call is `spectrum.wavelet(model)` in each language. Missing
normalization/background stages run on a private copy; users do not need to
prepare intermediate arrays. Specify the measured k interval in Å⁻¹, rather than
an energy interval relative to E₀.

```python
from rexafs import Wavelet

model = Wavelet((2, 12))
wavelet_map = spectrum.wavelet(model)
image = wavelet_map.magnitude       # NumPy matrix: R rows, k columns
region = wavelet_map.integral((4, 10), (1, 3))
print(region.value, region.unit)
```

Python options are named: `Wavelet((2, 12), kweight=2, rmax=4)`.
Array properties return independent NumPy copies. The transform and spectrum
preparation release the Python global interpreter lock. `phase()` returns a
matrix in radians, using `NaN` for zero/low-amplitude cells; its default floor is
1% of maximum magnitude. `wavelet_map.definition` returns independent settings,
and `preparation` describes automatic processing or is `None` for direct arrays.

```ts
import { Wavelet } from "rexafs/node";

const model = new Wavelet([2, 12]);
const map = spectrum.wavelet(model);
try {
  const image = map.magnitude;      // Flat Float64Array: R rows, k columns
  const [rows, columns] = map.shape;
  const region = map.integral([4, 10], [1, 3]);
  console.log(rows, columns, image[0], region.value, region.unit);
} finally {
  map.free();
  model.free();
}
```

TypeScript uses named options, for example `new Wavelet([2, 12], {rmax: 4})`.
Cell `(row, column)` is at `row * map.shape[1] + column`. Arrays are independent
copies and remain valid after the map is freed. Release every owned map and model
with `free()`, including a model obtained from `map.definition`. `preparation` is
`null` for array calculations. Browser applications import from `rexafs`, await
`init()` first, and should run synchronous calculations in a Worker.

Both bindings also provide `model.calculate(k, chi)` for original unweighted χ(k),
`model.estimate(k)` for dimensions/storage, `map.slice_at_k(k)` and
`map.slice_at_r(r)` for magnitude slices, and `to_json()`/`from_json()` for retained
definitions and maps. Python accepts one-dimensional numeric sequences or NumPy
arrays; TypeScript expects `Float64Array` inputs. Invalid coverage, settings,
dimensions or excessive resource requests fail with an error. Display sampling
and colors never enter a native region integral. All advanced settings have the
same meanings and defaults as the Rust API described above.

The adapters live in [`py-rexafs/src/wavelet.rs`](../py-rexafs/src/wavelet.rs),
[`crates/rexafs-wasm/src/wavelet.rs`](../crates/rexafs-wasm/src/wavelet.rs) and
[`js-rexafs/wavelet.js`](../js-rexafs/wavelet.js). They use the core calculation;
no separate Python or JavaScript numerical algorithm is introduced.

## Desktop workflow (unreleased)

Select a spectrum, then **Data → Analysis → Wavelet**. Set the fully measured k
interval and choose **Calculate**. Weight 2 and R up to 6 Å are starting values;
**Advanced** exposes order, sampling and taper. Missing normalization/background
processing uses the current spectrum settings on a copy. Confirmed unweighted
χ(k) groups reuse their original arrays without normalization or AUTOBK. Unknown
quantities must be confirmed first. A failed calculation
leaves the previous retained result intact; its fields must not be mistaken for
newly calculated data. The status text explains the outcome; hover to read a
long message.

The upper plot is the k–R map. **Spectra** shows the original k-weighted χ and the
ordinary Fourier magnitude below it. The linked Fourier calculation shares the
selected k interval and weight, and retains the current Fourier window/settings;
its window name is shown. That window does not change the wavelet's own taper.
**Slices**, or a click on the map, shows magnitude versus k at the selected R and
magnitude versus R at the selected k. The readout gives physical coordinates and
bilinearly interpolated native magnitude. R values are not phase-corrected.

Drag the two boundaries in each lower plot to select a k–R rectangle. The upper
map outlines that rectangle, and the inspector updates its full-native-grid
integral immediately. **Save region** retains the exact bounds, value, units,
integration convention and map identity. Saved region buttons restore the bounds.

**Colors** chooses a palette and direction. Magnitude has a zero-based scale;
real/imaginary views have symmetric scales. Phase uses radians and hides low
amplitude (below 1% of the native maximum); an entirely masked map has empty axes.
The display texture has at most 512 k columns and 256 R rows, interpolated in
physical coordinates, including nonuniform retained R grids. Magnitude uses
bilinear native magnitudes; phase uses the interpolated complex value. Neither
that texture nor display colors enter the region integral.

**Lock scale** keeps the numerical color limits across maps with identical
processing and wavelet definitions. A change of component, processing or transform
definition resets the lock. This is a color-limit control, not a qualified batch
comparison workflow. Recalculating a map resets its plot bounds; changing colors
preserves an interactive zoom. Switching between spectra and slices resets the
lower plots because their amplitude units differ.

**History** reopens retained maps for the current group and processing settings.
A retained result is labeled as such: it records the inputs used at calculation
time, and does not assert that a linked source file is unchanged. **Calculate**
reads the current input again. **Export map** writes JSON with the complete complex
map, original k/χ, preparation settings, linked Fourier result and saved regions.
It streams serialization without constructing a duplicate JSON value tree.

Project saves retain small receipts; **Include source files** embeds their
checksum-verified compressed artifacts too. Reopening restores the numerical map
and saved regions even if the original artifact cache is unavailable. The desktop
keeps one scientific map resident; this does not yet implement full-series map
retention or Live region trends. Desktop rendering requires at least two R rows.

See the [computer-use validation record](validation/2026-09-17-wavelet/README.md).

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
