# What fit statistics and uncertainties mean

This guide describes the current Rust/desktop EXAFS fitting implementation.
Python and TypeScript currently expose spectrum processing, not these fitting
classes. Processing produces chi(k); fitting adjusts a scattering-path model
to that signal. A converged optimizer has satisfied numerical stopping rules.
It has not established that the chosen structural model is unique or correct.

## Residuals and shared parameters

For dataset $d$, let $\boldsymbol\theta$ be the vector of independent fitted
parameters and let $\mathbf r_d(\boldsymbol\theta)$ be its real residual vector.
The residual includes the selected fit space, windows, k-weights and noise
scales. Complex R-space residuals include real and imaginary components
in that vector. Each element is a scaled data–model discrepancy. The optimizer
minimizes

$$
S(\boldsymbol\theta)=\sum_d\|\mathbf r_d(\boldsymbol\theta)\|_2^2.
$$

The squared Euclidean norm means the sum of the squared entries. A global
parameter appears in several datasets' models; a local parameter appears only
where assigned. Concatenating the residuals couples datasets through their
shared parameters. Independent batch fits instead solve separate problems.
The implementation is in
[solver.rs](../crates/rexafs/src/xafs/fitting/solver.rs) and
[transform.rs](../crates/rexafs/src/xafs/fitting/transform.rs); the
[multiple-spectrum guide](joint-fitting.md) describes the user controls.

## Independent information and reported chi-square

Denser k/R grids do not create more independent experimental information.
rexafs uses the following bandwidth estimate for each dataset:

$$
N_{\mathrm{idp},d}=1+\frac{2\Delta k_d\Delta R_d}{\pi},
\qquad N_{\mathrm{idp}}=\sum_dN_{\mathrm{idp},d}.
$$

$\Delta k_d=k_{\max,d}-k_{\min,d}$ is the selected k span in Å⁻¹, and
$\Delta R_d=R_{\max,d}-R_{\min,d}$ is the selected R span in Å. Their product
and $N_{\mathrm{idp}}$ are dimensionless. The additive **1** is the convention
implemented by `compute_n_idp`; it is not a claim that every published counting
convention uses that offset. The underlying information-counting issue is
examined by [Stern, *Physical Review B* 48, 9825–9827 (1993)](https://doi.org/10.1103/PhysRevB.48.9825).

Let $M$ be the number of scalar entries in the combined residual and $p$ the
number of independently varying parameters. For $M>0$ and $N_{\mathrm{idp}}>p$,
rexafs reports

$$
\chi^2_{\mathrm{reported}}=S\frac{N_{\mathrm{idp}}}{M},
\qquad
\chi^2_{\mathrm{reduced}}=
\frac{\chi^2_{\mathrm{reported}}}{N_{\mathrm{idp}}-p}.
$$

These are EXAFS reporting conventions, not simply the optimizer's raw sum $S$.
The code floors the denominator at $10^{-12}$ to avoid division by zero.
That numerical guard does not make a model with too many parameters
statistically identifiable. Noise estimates, overlapping windows, repeated
measurements and model inadequacy affect interpretation. See
[Larch's EXAFS fitting reference](https://xraypy.github.io/xraylarch/xafs_feffit.html)
for related reporting conventions; the equations above are traced to rexafs.

## Covariance and standard errors

The Jacobian has entries
$J_{ij}=\partial r_i/\partial\theta_j$: each column describes how residuals
change when one parameter changes. It has $M$ rows and $p$ columns. rexafs
approximates parameter covariance at the fitted point by

$$
C\approx\frac{S}{N_{\mathrm{idp}}-p}(J^TJ)^{-1},
\qquad
\operatorname{stderr}(\theta_j)=\sqrt{C_{jj}}.
$$

This is a local linear approximation, with the same denominator guard as above.
The implementation scales the inverse normal matrix by **raw** $S$, not by
$\chi^2_{\mathrm{reported}}$. Covariance element $C_{ij}$ has units equal to
the product of the units of parameters $i$ and $j$; a standard error has the
same units as its parameter. It is not automatically a 95% confidence interval.
The [least-squares derivation](../supportinginfo/uncertainty.md) explains the
Jacobian approximation and its assumptions. SciPy likewise distinguishes an
inverse-curvature estimate from variance-scaled covariance in its
[`leastsq` documentation](https://docs.scipy.org/doc/scipy/reference/generated/scipy.optimize.leastsq.html).

For nonzero variances, the correlation coefficient is

$$
\rho_{ij}=\frac{C_{ij}}{\sqrt{C_{ii}C_{jj}}}.
$$

It is dimensionless. A value near +1 or −1 means that the two parameter
estimates can compensate for each other locally. In rexafs, rows and columns
follow `varying_names`. Covariance can be absent when the inverse cannot be
formed; a finite matrix can still be poorly conditioned. Bounds, nonlinear
parameter dependence and systematic errors can make local symmetric errors
misleading. See [likelihood and limitations](../supportinginfo/uncertainty2.md).

## R-factor

In the same transformed/scaled residual representation, let $\mathbf d$ be
the data vector and $\mathbf m$ the model vector. rexafs uses

$$
R_{\mathrm{factor}}=
\frac{\|\mathbf d-\mathbf m\|_2^2}{\|\mathbf d\|_2^2}.
$$

The combined value sums the dataset numerators and denominators before taking
the ratio. It measures a relative squared discrepancy, not a probability that
the model is correct. The code returns zero for a numerically zero denominator;
that special value must not be interpreted as a good fit to an absent signal.
Compare values only with compatible fit spaces, weighting and data selection.

## What to inspect before interpreting a fit

Inspect residual structure, fit ranges, solver status, parameter bounds and
correlations as well as the displayed statistics. Test whether reasonable
processing or model choices change the conclusions. Include the scattering
backend, paths, parameter constraints and uncertainty convention when reporting
results. A small numerical residual does not account for an incorrect phase,
missing paths, sample heterogeneity or detector effects.
