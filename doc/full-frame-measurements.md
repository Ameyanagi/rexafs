# Full-frame measurements (development version)

This increment is newer than 0.2.9. It starts Phase A of the
[complete analysis design](complete-analysis-design.md); it does not implement
the later Live, peak-fit, MBACK, correction, wavelet, regression or confidence
milestones. Python/TypeScript exposure is a tracked follow-up after this Rust
contract stabilizes.

## Simple Rust API

```rust
use rexafs::prelude::Measurement;

// Normalized absorption, −20 to +30 eV relative to the resolved E₀.
let result = spectrum.measure(&Measurement::mean(-20.0..=30.0))?;
println!("{} {}", result.value, result.unit);

// Choose flattened absorption explicitly.
let height = spectrum.measure(&Measurement::maximum(0.0..=30.0).flat())?;
// Absolute energy; retain the input's original absorption units.
let point = spectrum.measure(&Measurement::point(9000.0).raw_mu().absolute())?;
// Absolute k in Å⁻¹, weight 2. No Fourier transform is required.
let chi = spectrum.measure(&Measurement::mean(3.0..=10.0).chi(2))?;
// Absolute uncorrected R in Å, using the spectrum's FFT settings.
let ft = spectrum.measure(&Measurement::integral(1.0..=3.0).fourier())?;
```

`Measurement::point`, `mean`, `maximum` and `integral` take coordinates; their
default representation is **Norm** and their default origin is **E₀-relative**.
`absolute()` changes energy coordinates to eV on the absolute energy axis.
`chi(weight)` and `fourier()` select their own absolute axes. The result records
the definition, finite scalar, units, actual bounds and resolved E₀. Maximum
also returns its absolute position; ties choose the lowest coordinate.

The operation borrows the input, reuses available stages, and calculates missing
prerequisites on a copy. It does not alter settings or create a project, worker,
cache, file or desktop session. Prepared Norm/Flat components retain their
scientific type. Raw, normalized and Fourier inputs cannot be freely relabeled.
Configured core normalization defaults still apply; provide suitable pre/post
intervals for a short XANES scan. No automatic fallback to EXAFS processing is used.

For already prepared arrays, use
`rexafs::xafs::analysis::metrics::measure(x, y, Metric::Mean { start, end })`.
`measure_masked` additionally accepts excluded intervals; intersecting any one
returns a typed missing-data error. Neither route sorts, extrapolates, clips,
fills gaps or drops nonfinite rows. Axes must increase strictly, lengths must
match and every input value must be finite. A region requires positive width.

## Numerical meaning

Between native samples the signal is linear. A point is linear interpolation;
a region integral sums trapezoids after interpolating its exact boundaries.
For a segment of width Δx with endpoint signals y₀ and y₁, its contribution is
Δx(y₀+y₁)/2. Δx uses axis units and y uses the selected signal units. This is
exact for the linear interpolant, not necessarily for an unknown continuous
experimental signal. The [SciPy trapezoid reference](https://docs.scipy.org/doc/scipy/reference/generated/scipy.integrate.trapezoid.html)
documents the composite rule; strict coverage and increasing axes are rexafs
choices implemented in `xafs/analysis/metrics.rs`.

The mean is the integral divided by region width, rather than the arithmetic
mean of unevenly spaced samples. No baseline is subtracted. An integral of
absorption therefore is not automatically a peak area or chemical concentration.
The advanced array operator also supports a nonnegative finite-region centroid,
integrating x·y(x) analytically and rejecting a zero area or negative samples.
No uncertainty is inferred; `standard_error` is absent until a documented
input-error propagation method is supplied.

Energy uses eV, k uses Å⁻¹ and R uses Å. Norm/Flat are dimensionless. For χ(k)
weighted by kᵖ, signal units are Å⁻ᵖ. Forward-transform magnitude has units
Å⁻⁽ᵖ⁺¹⁾ under the existing `kstep / sqrt(pi)` convention. Integrals multiply the
signal unit by axis units. R is not a phase-corrected bond length.

## Desktop workflow

Open **Series → Measurements**. Create a series from marked groups, the current
folder scan or all groups. Membership is explicit and includes reviewed imports
and materialized components. New imports do not silently enter that series.
The first increment retains current group order and labels it as a sequence;
it does not claim that filename or arrival order is elapsed acquisition time.

Choose Point, Maximum, Integral, Mean or absolute E₀. Select the representation
and coordinate origin, then **Preview** a named frame. Dashed lines show the
resolved point/region on its spectrum. **Calculate all N frames** calculates
every member, independently of the older sampled overview. The default maximum
range is −20…+50 eV; a conventional white-line definition can be entered as
0…+30 eV in an explicitly selected Norm or Flat representation.

The worker first records input revisions, then prepares one spectrum at a time.
Normalized metrics stop after normalization; k metrics stop after background
subtraction and R metrics after the forward FFT. Unrelated invalid later-stage
settings do not block an earlier measurement. Each row has an explicit outcome;
missing numbers are not zero. The display cache is not used as the scientific
input and the result table displays 50 rows at a time without limiting export.

**Cancel** retains finished rows. **Resume unfinished** uses the saved definition
and checks input/settings revisions before calculating unfinished rows. Changed
inputs fail with a reason; start a new run to analyze the new data. Earlier runs
remain available. **Check input revisions** compares current sources/settings
without changing historical values. This first increment checks linked-file
changes explicitly; automatic filesystem-driven invalidation belongs to Live.

Saving a project retains series, frame/group identities, definitions, settings,
resolved preparation, source digests, outcomes and run history. Portable replay
still requires embedded inputs or unchanged linked sources. **Export CSV**
includes all statuses and values; **Export definition and results** writes the
full JSON metadata. A figure alone is not a replay record. Old projects still
open and the older **Scan overview** keeps its historical sampled definitions.

## Remaining Phase A gates

Natural/acquisition-time ordering with membership editing, physical-coordinate
metadata, standalone named measurement presets, automatic revision checking,
recovery-journal UI, plot-gesture authoring, independent-error propagation and
full resource/platform qualification remain tracked work. Do not label this
increment as the complete eight-milestone roadmap or as a released feature.
