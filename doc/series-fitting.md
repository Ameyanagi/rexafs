# EXAFS fits and parameter trends in Series

This feature is **unreleased**. In Series, choose **Add trend… → EXAFS fit…**,
select a reviewed single-spectrum fit, and choose **Fit all frames**. The
**Fit** view shows the selected frame's data and model in k or R space, together
with a parameter trend. Frame navigation also changes the numerical readout.
The fit runs in the background when another stage or window is selected.

The saved model supplies enabled FEFF scattering paths, expressions, bounds,
starting values, fit ranges, and weighting. Every frame starts from the same
recorded initial values; results from the preceding frame do not become the
next frame's starting values. Joint fits are excluded. To change the model,
review a new single-spectrum fit in Fit, then select it for a new series run.

**Each spectrum** is the recommended processing choice. It uses each group's
effective processing settings. **Copy selected settings** copies the selected
spectrum's processing to every frame while preserving each frame's import
mapping. Normalization and background removal run as needed. A model configured
to follow the processing k-weight still follows the effective settings of each
frame; choose explicit fit weights in the model if all frames must use the
same weights.

All frames in the selected series are included, independently of the sampled
overview. Membership, ordering, coordinates, source revisions, processing,
and FEFF file bytes are frozen for the run. A source that changes after the
revision check fails explicitly. Missing or failed frames keep their place.
Stop takes effect between fits, retaining completed results. A new run retains
the previous run. Reopening a project never starts unfinished fits automatically.
New incoming Live scans require a new Series run; Live's separately configured
automatic EXAFS fitting continues to operate on its chosen outputs.

## Parameters and expressions

The trend selector includes the model's named variables, including fixed and
expression-defined variables, and each enabled path's fitted distance. Choose
**Expression…** to add a name, expression, display unit, and optional path.
Expressions use the same arithmetic and functions as the fit model. Names are
case sensitive. `reff` is the selected path's FEFF reference half-path length
in Å; `degen` is its dimensionless FEFF multiplicity. For a first-shell model
using `dr_1`, the expression `reff + dr_1` gives the fitted half-path length.
For a single-scattering path this is the absorber–scatterer distance. Multiple
scattering requires the half-path-length interpretation described in
[fitting statistics](fitting-statistics.md).

The unit field labels the plot; it does not perform conversion or dimensional
analysis. For example, `1000 * dr_1` can be labeled `mÅ`. Arbitrarily named
variables have no inferred physical unit. Selecting or adding a trend evaluates
retained results without rerunning the optimizer. An optional series coordinate
can replace frame number on the horizontal axis when coordinates are recorded.
Connecting lines guide the eye; they are not regressions or kinetic fits.

## Error bars

Error bars are on by default and show plus or minus one local standard error.
For an expression \(f(\boldsymbol\theta)\), rexafs calculates
\(u_f^2 = J C J^{\mathsf T}\). Here \(\boldsymbol\theta\) is the vector of
independent fit variables, \(C\) is their retained covariance in the corresponding
products of parameter units, and \(J_i=\partial f/\partial\theta_i\) is the
expression's sensitivity to variable \(i\). The result \(u_f\) has the units
of the expression. Off-diagonal covariance terms include parameter correlations.
This is first-order uncertainty propagation, as described by
[NIST Technical Note 1297, Appendix A](https://www.nist.gov/pml/nist-technical-note-1297/nist-tn-1297-appendix-law-propagation-uncertainty).

The rexafs implementation uses finite differences within parameter bounds and
reevaluates constrained variables at each step. It treats FEFF reference geometry
as exact, so `reff + dr_1` has the same fit standard error as `dr_1`. These bars
exclude FEFF/model errors, calibration errors, and systematic processing errors.
The local approximation can be unreliable for strongly nonlinear expressions,
poorly determined parameters, or solutions near bounds. Optimizer convergence
does not establish physical accuracy; see the covariance scaling and limitations
in [fitting statistics](fitting-statistics.md).

Unconverged or failed frames leave gaps. Converged values without usable covariance
remain visible with unavailable uncertainty; no zero uncertainty is invented.
Fixed expressions are labeled fixed. Full curves, solver reports, covariance,
and preparation records are checksum-verified artifacts included in portable
projects. Save the project to retain the run and custom expressions.

Implementation: [series_fits.rs](../crates/rexafs-gui/src/series_fits.rs) retains
fits and evaluates trends; [fit_details.rs](../crates/rexafs-gui/src/fit_details.rs)
propagates covariance; [Series fit UI](../crates/rexafs-gui/src/app/shell/series/fits.rs)
coordinates background work and plotting; [project storage](../crates/rexafs-gui/src/project/storage.rs)
embeds and relocates artifacts.

Right-click a spectrum or parameter plot and choose **Axis range…** to set
independent automatic or numeric bounds. For example, Y minimum can be `0`
while Y maximum follows new frames. See [plot axis ranges](plot-axis-ranges.md).
