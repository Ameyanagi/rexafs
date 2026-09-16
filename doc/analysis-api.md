# LCF, PCA and MCR-ALS APIs

This describes **rexafs 0.2.9**, including native MCR-ALS and
prepared-component processing. LCF means linear combination fitting; PCA means
principal component analysis; MCR-ALS means multivariate curve resolution by
alternating least squares.

The desktop uses these Rust calculations. **LCF, PCA and MCR-ALS do not currently
have public Python or TypeScript bindings.** Their examples below are Rust, not
proposed binding signatures. See the [Cu workflow](cu-mixture-analysis.md) for
the GUI and the [validation record](validation/2026-09-16-cu-mixtures/README.md)
for measured recovery results.

## Shared inputs and ownership

Use `use rexafs::prelude::*;`. `Spectrum` is the short name for `XASSpectrum`;
these are the same type. The three analyses accept borrowed collections, such
as `Vec<Spectrum>`, slices of spectrum references, or `Arc<Spectrum>` values.
They copy the selected arrays into their results, so later edits to an input do
not change a retained analysis. **Missing processing arrays are calculated on a
temporary copy**, using each spectrum's existing settings or the stage defaults
when unset. Norm, Flat and Deriv require normalization; Chi requires background
subtraction, which normalizes if needed. Existing selected arrays are reused.
No smoothing or energy alignment is introduced. Failed preparation names the
input and retains the processing error; no source arrays or caches are changed.

For example, raw `Spectrum::from_arrays(...)` inputs can go directly to `lcf`,
`pca_train` or `mcr_als`. Set normalization/background settings first when the
scientific workflow requires them; the analysis then calculates the missing
stages. `AnalysisSpace::arrays(&spectrum)` provides the same preparation behavior
when only copied arrays are needed.

A recovered component declared as Norm has no original Flat representation.
Select Norm, or explicitly set its normalization method to request pre/post-edge
refitting before asking for Flat. The API does not silently relabel Norm as Flat.
A prepared Flat component is reused as supplied unless refitting was requested.

`AnalysisSpace` selects the actual values used in the calculation:

| Variant | Values | Available analyses |
| --- | --- | --- |
| `Norm` | Normalized absorption, dimensionless | LCF, PCA, MCR |
| `Flat` | Flattened normalized absorption, dimensionless | LCF, PCA, MCR |
| `Deriv` | Energy derivative of normalized absorption, eV⁻¹ | LCF, PCA |
| `Chi { kweight }` | k-weighted EXAFS | LCF, PCA |

All three core configs currently default to `Norm`. The Cu desktop workflow
starts on `Flat`. Specify `Flat` explicitly to reproduce that workflow.

For energy spaces, `range: Some((a, b))` is in **eV relative to E₀**, not absolute
energy. LCF uses the target's E₀; PCA and MCR use the first input's E₀. `None`
selects −20 to +30 eV. For `Chi`, the bounds are absolute k in Å⁻¹ and the default
is 3 to 12 Å⁻¹. k weighting changes spectral units by a factor of Å⁻ᵏʷᵉⁱᵍʰᵗ.

The analyses interpolate onto selected samples of the target grid (LCF) or first
input grid (PCA/MCR). **Every input must cover the entire requested interval.**
All three reject nonfinite, equal or reversed bounds, insufficient points and
incomplete coverage; no bounds are swapped and no constant tails are added.
For LCF shifts, references must additionally cover the configured shift margin
on both sides of the interval. PCA target transformation requires coverage of
the retained model grid. The GUI uses this same validation, including batch LCF;
a half-entered interval is an error, while clearing both fields selects defaults.
The shared implementation is in
[`analysis/mod.rs`](../crates/rexafs/src/xafs/analysis/mod.rs).

## Linear combination fitting

The entry points are `lcf(&target, &standards, &config)` and
`lcf_combinatorial(&target, &standards, &config, max_standards)`. The latter tries
subsets, subject to `config.max_combinations`, and returns ranked results.

```rust
use rexafs::prelude::*;
let config = LcfConfig {
    space: AnalysisSpace::Flat,
    range: Some((-29.0, 171.0)),
    ..Default::default()
};
let fit = lcf(target, standards, &config)?;
for coefficient in &fit.weights {
    println!("{}: {}", coefficient.name, coefficient.weight);
}
// fit.x, fit.data, fit.fit and fit.residual share the selected grid.
```

Defaults constrain each weight to 0–1, require their sum to equal one, and do not
fit energy shifts. Optional shifts are per reference, bounded by
`max_e0_shift = 5.0` in axis units. `max_combinations` defaults to 1000.

`LcfResult.weights` contains `LcfComponent` records with input index, name, weight,
optional standard error and fitted shift. `components` contains **weighted
reference contributions**, not recovered pure spectra. `r_factor` is squared
residual divided by squared data norm; `chi_square` is unweighted residual sum
of squares, not a noise-calibrated statistic. Retain the config separately: this
result does not store the entire config. The desktop's saved analysis does.

`lcf_batch(&targets, &standards, &config)` returns an ordered
`Vec<Result<LcfResult, AnalysisError>>`. One failed target does not discard other
fits. Standards are prepared once per batch; the same offset interval is resolved
from each target's E₀. An invalid standard yields a contextual error for every
row. `lcf_batch_with_progress(..., |index, result| true)` reports each completed
row, including failures. Returning false stops before the next target and returns
the completed prefix. Both run on the calling thread. The desktop Series worker
uses this API in bounded batches and preserves frame identities, failed rows
and cancellation in the project. Implementation:
[`lcf.rs`](../crates/rexafs/src/xafs/analysis/lcf.rs).

## Principal component analysis

```rust
use rexafs::prelude::*;
let model = pca_train(spectra, &PcaConfig {
    space: AnalysisSpace::Flat,
    range: Some((-29.0, 171.0)),
    center: true,
})?;
let reconstruction = model.reconstruct_training(0, 2)?;
let suggestion = model.component_count_suggestion(); // count and diagnostic basis, or None
let errors = model.reconstruction_errors(); // count, SSE, relative error for the plot
// reconstruction.fit and residual describe the first input using two directions.
```

`center` defaults to **false** in the core. It controls mean subtraction, so
changing it changes the interpretation of the model. For these three-standard,
sum-to-one synthetic mixtures, centering leaves two varying directions plus the
mean. It does not turn the chemistry into a two-species system.

`PcaModel` retains the grid, labels, input matrix, mean, component directions,
scores, eigenvalues, variance fractions, cumulative fractions and IND indicator.
`target_transform(&spectrum, count)` projects another prepared spectrum onto the
model. `reconstruct(&values_on_model_grid, count)` accepts a `DVector<f64>` with
exactly the model grid length. `reconstruct_training(index, count)` avoids that
array preparation for an existing row. All return owned `PcaFit` arrays.

`suggested_components_variance(0.999)` selects a cumulative-fraction threshold.
`component_count_suggestion()` is the recommended count diagnostic shared with
the GUI. It returns a count and `PcaCountBasis` (numerical rank or interior IND
minimum), or `None` for zero data or a boundary minimum. `numerical_rank()` uses
the rexafs tolerance σ > ε max(n, p) ‖D‖F, where ε is f64 precision and D is the
original training matrix. This is a roundoff check, not an experimental noise
estimate. The historical `suggested_components_ind()` remains available for
compatibility and only searches finite IND minima.

`reconstruction_errors()` returns counts zero through all available directions,
SSE and relative squared error. SSE is n times the sum of discarded eigenvalues;
relative error divides by the original data's squared norm, also when centered.
Zero input has undefined relative error (`NaN`). No display floor or logarithm
changes these results. The GUI uses these exact values and applies Linear/Log
scaling only while plotting. None of these diagnostics proves the number of
chemical species. `n_components()` reports available SVD directions, **not** a
recommended count. Implementation and algorithm references:
[`pca.rs`](../crates/rexafs/src/xafs/analysis/pca.rs).

## Multivariate curve resolution

```rust
use rexafs::prelude::*;
let result = mcr_als(spectra, &McrConfig {
    space: AnalysisSpace::Flat,
    range: Some((-150.0, 700.0)), // every input must cover this interval
    components: 3,
    max_iterations: 2000,
    ..Default::default()
})?;
let mut component = result.component_spectrum(0)?;
component.calc_background()?.fft()?;
// The ordinary EXAFS fitting API accepts component.k() and component.chi().
```

MCR estimates `data ≈ concentrations × spectra`. Coefficients are always
nonnegative; sum-to-one is enabled by default. Component spectra are signed by
default because baseline-subtracted data can contain small negative values.
`nonnegative_spectra: true` adds a nonnegativity constraint to those spectra.

Other defaults are three components, 500 iterations, relative improvement
tolerance 10⁻⁸, deterministic seed 0, no supplied initialization and no anchors.
`initial_spectra` optionally supplies component rows on the selected grid.
`anchors: Vec<McrAnchor>` fixes explicitly known sample compositions. Using
references for initialization or constraints must be described as such, rather
than as blind recovery.

`mcr_als_with_progress` additionally accepts a callback
`FnMut(iteration, relative_error) -> bool`; returning false cancels and retains
the best completed result. It runs on the calling thread.

`McrResult` owns all input, fitted and residual arrays, coefficients, component
spectra, settings, E₀, selected initialization rows, objective history and an
explicit termination status. Check `termination`: reaching `IterationLimit` or
being `Cancelled` is not convergence. `relative_error` is residual sum of squares
divided by input sum of squares. A small residual does not establish unique
chemical components. The native implementation and its relation to the
pyMCR method are documented in
[`mcr.rs`](../crates/rexafs/src/xafs/analysis/mcr.rs).

`component_spectrum(index)` is the new **0.2.9** bridge to the normal
spectrum API. It copies one component, preserves its declared norm/flat values,
uses E₀ from the calculation and a unit edge step, and retains that input type
through edits and serialization. No second pre-edge fit or flattening occurs
unless you explicitly request it.
Keep the `McrResult` alongside it for complete source and calculation provenance;
the GUI stores this in the calculated group's Operation metadata and project.
Historical MCR JSON without E₀ requires an explicit value through
`Spectrum::from_prepared(energy, values, space, e0)`.

You can apply pre/post-edge normalization to a recovered flattened component.
For example, this explicitly opts into refitting:

```rust
use rexafs::prelude::*;
component.set_normalization_method(PrePostEdge {
    e0: Some(8979.0),
    pre_edge_start: Some(-150.0),
    pre_edge_end: Some(-75.0),
    norm_start: Some(150.0),
    norm_end: Some(650.0),
    ..PrePostEdge::new()
})?.normalize()?;
```

The original input values and their declared
flat representation remain stored; `norm()` and `flat()` return the newly fitted
outputs. `preserves_prepared_values()` distinguishes the default unit-step path
from this explicit correction. The choice and settings survive serialization.
These example intervals require sufficient measured coverage.

Only the fitted energy interval is recovered. The previous 8950–9148 eV Cu
calculation is a XANES result; wider EXAFS requires repeating MCR over wider
measured coverage. AUTOBK then operates directly on that declared component
representation. This unit-step adapter is a rexafs choice, consistent with
the [AUTOBK definition](https://xraypy.github.io/xraylarch/xafs_autobk.html)
χ = (μ − μ₀) / edge step; it does not restore the original unflattened signal.

## Result shapes

Let n be the number of spectra, p the number of selected energy points, c the
requested MCR components and r the available PCA directions. All indices are
zero-based. Matrices are `nalgebra::DMatrix<f64>` and vectors are
`DVector<f64>` or `Vec<f64>` as declared in the source.

| Result field | Shape | Meaning |
| --- | --- | --- |
| LCF `x`, `data`, `fit`, `residual` | p | One target and reconstruction |
| LCF `components[j]` | p | Reference j multiplied by its fitted weight |
| PCA `data`, MCR `data`, `fit`, `residual` | n × p | One spectrum per row |
| PCA `components` | r × p | Orthonormal mathematical directions |
| PCA `scores` | n × r | Coordinates along those directions |
| MCR `spectra` | c × p | Estimated component spectra |
| MCR `concentrations` | n × c | Component coefficients per input |

For 100 spectra and three MCR components, concentrations has 100 rows and three
columns. Rows of the recovered-spectra matrix are the three spectra to compare
with Cu foil, Cu₂O and CuO, after resolving their arbitrary component order.

## Binding todo list and follow-up work

- [ ] Add public Python LCF, batch LCF, PCA and MCR-ALS functions with NumPy
  result arrays, result-to-spectrum conversion and completion-visible docs.
- [ ] Add equivalent TypeScript bindings with typed options and explicit matrix
  dimensions, result-to-spectrum conversion and JSDoc.
- [ ] Test both bindings against the core for automatic preparation, E₀-relative
  intervals, coverage failures, ordered batch errors, cancellation and PCA
  diagnostics. Frontends must call these core implementations rather than copy
  their algorithms. Keep defaults Norm; require explicit Flat.
- [ ] Consider typed energy-offset/k ranges and a shared top-level error conversion.
  Analysis returns `AnalysisError`; processing returns `rexafs::Error`. A function
  chaining both can currently use `Result<_, Box<dyn std::error::Error>>`.

The preparation, common validation, batch LCF and core PCA diagnostics described
above are implemented in 0.2.9. The binding tasks remain open;
no Python or TypeScript analysis API is being claimed here.
