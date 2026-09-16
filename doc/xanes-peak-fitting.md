# XANES peak fitting (unreleased)

The development core fits a sum of named peaks, absorption steps and baseline
terms on the original energy grid. It does not choose a peak count or identify
chemical species. GUI, Series/Live integration and bindings are being implemented
under [milestone C](analysis-b-f-progress.md); they are not implied by this core API.

```rust
use rexafs::{PeakFit, Spectrum};
use rexafs::prelude::Param;

fn fit(spectrum: &Spectrum) -> Result<(), rexafs::PeakFitError> {
    let model = PeakFit::new(-20.0..=40.0)
        .gaussian("white_line", 5.0, 2.0, 3.0)
        .linear_baseline(0.0, 0.0)
        .parameter(Param::new("white_line_center", 5.0).bounds(0.0, 10.0));
    let result = spectrum.fit_peaks(&model)?;
    println!("{:?}: {:?}", result.termination, result.components);
    Ok(())
}
```

`PeakFit` defaults to normalized absorption (`Norm`) and inclusive energy offsets
from each spectrum's E₀. Use `.flat()`, `.raw_mu()`, `.absolute()`, or
`.reference(9000.0)` explicitly when another representation or energy origin is
needed. Missing normalization is prepared on a copy using the source's settings.
Source arrays/settings and the model definition remain unchanged.

The Gaussian arguments above are a stable name, center in eV, whole-axis area in
signal units × eV, and full width at half maximum (FWHM) in eV. The starting area
is not the peak height. Areas default to nonnegative, centers to the fit interval,
and widths to positive values. Fixed values and restricted mathematical ties use
`Param::fixed("white_line_width", 3.0)` and
`Param::expr("second_width", "white_line_width")`. Unknown symbols, expression
cycles, inconsistent roles, invalid bounds and nonfinite values return errors.
Public component/parameter fields support advanced editors; names must remain
unique ASCII identifiers.

The models are implemented in [profiles.rs](../crates/rexafs/src/xafs/analysis/peakfit/profiles.rs).
Their area conventions follow the [official lmfit profile definitions](https://lmfit.github.io/lmfit-py/builtin_models.html),
with these explicit rexafs parameter conversions:

| Shape | Width and other parameters |
| --- | --- |
| Gaussian | FWHM = 2√(2 ln 2) σ, where σ is its standard deviation. |
| Lorentzian | FWHM = 2γ, where γ is its half-width at half maximum. |
| Pseudo-Voigt | Gaussian/Lorentzian mixture with one common FWHM and a Lorentzian fraction in [0,1]. |
| Voigt | Convolution with independent Gaussian and Lorentzian FWHM contributions; one may be fixed to zero. Combined FWHM is found numerically at half-height. |
| Error-function step | h[1 + erf((E−c)/τ)]/2. |
| Arctangent step | h[1/2 + atan((E−c)/τ)/π]. |
| Constant/linear baseline | b₀ or b₀ + b₁(E−Eref), with a recorded fixed reference. |

For steps, h is signed height in signal units, c is center in eV and τ is a
positive energy scale in eV; τ is not FWHM. Steps have no finite whole-axis area.
For linear baselines, b₀ has signal units and b₁ has signal units/eV. A broad peak
shape can be assigned `PeakRole::Baseline`; it is then excluded from the reported
area-weighted peak center. Signed peak areas for difference spectra are not
supported by this absorption model.

`initialize_baseline(&spectrum, &[peak_interval])` returns a new starting model
after fitting baseline-role terms outside the supplied peak intervals. Those
extra exclusions do not become final-fit masks. The subsequent composite fit
refines the baseline jointly with peaks. This supports the preliminary-baseline
workflow described in [Larix](https://xraypy.github.io/xraylarch/larix/preedge.html),
without loading or executing Python models.

## Fitting and interpreting uncertainty

The [solver](../crates/rexafs/src/xafs/analysis/peakfit/solver.rs) minimizes the
sum of squared native-point residuals. With supplied one-standard-deviation
errors σᵢ, the residual is `(dataᵢ − modelᵢ)/σᵢ`; otherwise it is unweighted.
Errors must already describe the selected representation. Rexafs does not infer
detector noise or propagate normalization uncertainty from raw μ automatically.

The deterministic [optimizer](../crates/rexafs/src/xafs/analysis/peakfit/optimizer.rs)
uses column-scaled, damped Gauss–Newton steps solved with nalgebra's singular-value
decomposition (SVD). This is a rexafs implementation. It projects proposed steps
onto bounds and refits free variables while outward-pointing bound variables are
held. It has numerical finite-difference derivatives, a default 200-iteration
budget and tolerance 10⁻¹⁰. No Python runtime or new numerical dependency is added.

The full requested interval must be covered. `.exclude(a..=b)` removes native
points inclusively. No interpolation, smoothing or extrapolation is introduced.
Fit results retain source indices, selected data, model, individual contributions,
unweighted residuals, supplied errors, initial/final parameters and termination.
Sampled component integrals use trapezoids between adjacent included native
points; excluded gaps are not bridged. They differ from analytic whole-axis area.

Local covariance uses the weighted Jacobian, with rank checked at a relative
singular-value cutoff of 10⁻¹⁰. For known independent errors its scale is absolute.
For unweighted fits it is scaled by residual sum of squares divided by N−p,
where N is the number of fitted energy samples and p is the number of independent
varying parameters. This does not reuse EXAFS's independent-point estimate.
Nonconvergence, cancellation, active bounds or deficient rank withhold covariance
and report a reason. Statistical interpretation remains conditional on the model
and noise assumptions; convergence is not proof of identifiability.

Peak center, area, height and FWHM errors use the full joint covariance when it
is available. The area-weighted center is Σ(Qⱼcⱼ)/ΣQⱼ for peak-role components j,
with area Qⱼ and center cⱼ; it excludes baseline/step roles and is unavailable
for negligible total area. It is a model summary, not the centroid of measured
data. Full covariance includes correlations with the jointly fitted baseline.

`fit_batch(&spectra)` fits each source independently from the same starting
definition and retains each error in input order. `fit_with_progress` allows
cancellation between optimizer iterations. A cancelled result retains the best
accepted model and is never reported as converged. Use `fit_prepared` only for
arrays already in the selected representation, with absolute energy in eV.
`result.fitted_model().evaluate(&energy, e0)` evaluates a copied fitted model on
another grid for display; it does not alter the historical result.

## Validation and remaining work

Analytical tests cover profile scaling, true-Voigt limits, weighted linear
covariance, overlapping peaks with tied widths, masks/irregular grids, active
bounds, movement away from initial bounds, rank deficiency, cancellation,
baseline initialization and invalid inputs. Four synthetic profile fits are
compared with pinned lmfit 1.3.4 / SciPy 1.16.1 references, including parameter
errors. [Fixture provenance and tolerances](../crates/rexafs/tests/fixtures/analysis/xanes-peaks/README.md)
are retained with the generated data. The fixtures are repository-only.

These tests establish the checked numerical cases, not physical validity for an
arbitrary experimental decomposition. Current/Series/Live GUI workflows, retained
GUI diagnostics, public Python/TypeScript APIs, native platform checks and a
documented public experimental example remain milestone-C work.
