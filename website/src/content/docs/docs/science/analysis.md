---
title: "LCF, PCA and data treatment"
description: "Analyze collections while keeping preprocessing assumptions explicit."
audience: user
---

These tools are available in the desktop and Rust in 0.2.4; Python and TypeScript
expose spectrum processing. In the desktop, use **Data → Parameters** or action search.

## Linear combination fitting

Linear combination fitting (LCF) models an unknown spectrum as a weighted sum of references:

$$\min_{\mathbf w}\sum_{j=1}^{M}\left[U(x_j)-\sum_{i=1}^{S}w_iS_i(x_j)\right]^2.$$

$U$ is the unknown, $S_i$ the $i$th standard, $w_i$ its dimensionless weight,
$M$ the number of fitted samples and $S$ the number of standards. The axis $x$
is energy in eV or k in Å⁻¹, depending on the analysis space. All spectral arrays
must use the same units and normalization. Standards are interpolated onto the
unknown's grid in the selected range. Outside a standard's measured coverage,
the interpolator holds its endpoint value. Choose an interval measured for
every spectrum so that these constant tails do not affect the fit. The Rust
analysis functions require the relevant normalization/background arrays to
exist; they do not run processing stages automatically or modify their inputs.

The default bounds are $0\le w_i\le1$ with $\sum_iw_i=1$. Energy-space ranges
start at −20 to +30 eV relative to $E_0$; χ-space ranges start at 3–12 Å⁻¹.
Optional shifts move each standard's axis before fitting; they are off by default.
The shift is added to the standard's axis: a positive value moves a feature to
higher energy or k. `max_e0_shift=5` means 5 eV for energy spaces but 5 Å⁻¹ for
χ-space, despite its historical name; choose a meaningful bound for the selected
axis. These are constant axis shifts, not a full physical recalibration model.
Weights only support composition claims when the standards and measurement model
justify that interpretation. Similar standards can give ambiguous weights.

rexafs solves the bounded linear subproblem to numerical tolerances with an
active-set method: coefficients on bounds are held fixed while the free
coefficients are solved, then bounds are activated or released as needed.
When shifts are allowed, an outer nonlinear solve varies the shifts and solves
the bounded weights again at each step. See
[Nocedal and Wright, *Numerical Optimization*](https://doi.org/10.1007/978-0-387-40065-5)
for active-set constrained optimization. This description is traced to
[lcf.rs](https://github.com/Ameyanagi/rexafs/blob/v0.2.4/crates/rexafs/src/xafs/analysis/lcf.rs).
The generated [Rust API](/api/rust/rexafs/xafs/analysis/index.html) documents
`LcfConfig`, `lcf` and combination searches. For the general XANES use of reference
mixtures, see [Larch's linear-analysis guide](https://xraypy.github.io/xraylarch/xafs_xanes.html).

The reported `chi_square` is the unweighted squared residual sum, in squared
spectral units. `reduced_chi_square` divides it by
`max(n_data - n_vary, 1)`. Here `n_vary` counts one coefficient per standard,
subtracts one for the sum constraint, and adds each fitted shift; coefficients
on bounds do not reduce that reporting count. This differs from the
[EXAFS path fitter's statistics](/docs/science/fitting-statistics/).
Coefficient errors use a local linear covariance with the fitted shifts held
fixed; shift errors come from the projected nonlinear problem. They are not a
joint coefficient/shift covariance or a confidence interval. Similar references,
active bounds, interpolation, and an uncertain reference set limit interpretation.
The R-factor is the squared discrepancy divided by the data's squared norm;
it is NaN for an all-zero signal.

## Principal component analysis

PCA arranges $n$ spectra on a common grid of $m$ points in a matrix
$D\in\mathbb R^{n\times m}$ and computes a singular value decomposition:

$$D=U\Sigma V^\mathsf T.$$

Rows of $V^\mathsf T$ are orthonormal spectral components; the diagonal singular
values $\sigma_i$ indicate their magnitudes. The fraction attributed to component
$i$ is $\sigma_i^2/\sum_j\sigma_j^2$. Matrix entries have the chosen spectral
units; components are normalized and scores carry the spectral scale.

rexafs defaults to **no mean subtraction** (`center=false`). If centering is
enabled, $D$ in this equation is the mean-subtracted matrix. Centering changes the
question answered by the decomposition and the interpretation of rank; record it
when comparing software. A component is a mathematical direction, not necessarily
a pure chemical species. Noise and inconsistent normalization can create
additional apparent components. Target transformation tests reconstruction using
selected components, not chemical uniqueness.

See the [Rust PCA API](/api/rust/rexafs/xafs/analysis/pca/index.html) for configuration
and [Larch's PCA explanation](https://xraypy.github.io/xraylarch/xafs_xanes.html#principal-component-analysis)
for background; its default centering differs from rexafs.

The API's `eigenvalues` are $\lambda_i=\sigma_i^2/n$, in squared spectral units.
Without centering these are eigenvalues of the second-moment matrix
$D^\mathsf T D/n$, not a covariance about the mean. With centering they use a
population denominator $n$, not the sample-variance denominator $n-1$.
The reported variance fraction is therefore a fraction of squared signal when
uncentered, or of squared deviations when centered. It is zero for every
component of an all-zero input matrix.

To reconstruct a target spectrum $\mathbf y$ on the model grid using $c$
components, rexafs computes

$$
\mathbf a=C_c(\mathbf y-\boldsymbol\mu),\qquad
\widehat{\mathbf y}=\boldsymbol\mu+C_c^\mathsf T\mathbf a.
$$

$C_c$ contains the first $c$ orthonormal component rows, $\boldsymbol\mu$ is
the stored mean (zero without centering), and $\mathbf a$ contains projection
scores in the input spectral units. The reconstructed $\widehat{\mathbf y}$
has those same units. Scores can be negative and are not composition fractions.
Zero retained components reconstructs the mean alone. `target_transform()`
interpolates onto the training grid using the same endpoint-held convention as
LCF; `reconstruct()` instead requires values already on that grid.

The indicator used to suggest a retained count $c$ is

$$
\operatorname{IND}(c)=
\frac{1}{(n-c)^2}
\sqrt{\frac{\sum_{j=c+1}^{\min(n,m)}\lambda_j}{m(n-c)}},
\qquad 0\le c<n.
$$

Here $n$ is the number of training spectra, $m$ the number of common grid
points, and the eigenvalue index starts at one. This is the precise scaling
implemented in [pca.rs](https://github.com/Ameyanagi/rexafs/blob/v0.2.4/crates/rexafs/src/xafs/analysis/pca.rs).
The method is based on [Malinowski (1977)](https://doi.org/10.1021/ac50012a027);
its minimum suggests a numerical rank rather than proving a number of chemical
species. rexafs searches finite minima from $c=1$ and limits its suggestion to
available components. When $n>m$, the missing tail eigenvalues count as zero
and the resulting minima can be uninformative. Inspect the spectrum shapes,
noise, and reconstruction error along with either rank suggestion.

PCA reconstruction `chi_square` is also a raw squared residual sum; its reduced
value divides by `max(m - c, 1)`. No experimental noise variance is supplied to
this calculation, so this number is not a calibrated chi-square test.

## Data-treatment choices

| Tool | What changes | What to inspect |
|---|---|---|
| Calibration / alignment | Energy origin or relative energy shift | A suitable reference and common interval |
| Deglitch | Selected bad samples | Real features must not be mistaken for glitches |
| Truncate | Available energy extent | Enough pre/post-edge and EXAFS range remains |
| Rebin | Sampling on pre-edge/XANES/k regions | Bin widths and lost resolution |
| Smooth | Local spectral variation | Noise reduction can also suppress signal |
| Merge | Several spectra become a mean/combined spectrum | Alignment and physically comparable measurements |
| Difference | A reference is subtracted | Common units, normalization and energy registration |

Keep original inputs and record the transformations. More interpolated or
zero-padded points do not create independent physical information. The
[generated tools documentation](/api/rust/rexafs/xafs/tools/index.html) defines the
implemented methods and numerical defaults.

### Rebinning and the spread of merged spectra

The default rebin grid uses 10 eV pre-edge steps up to $E_0-30$ eV,
0.5 eV steps through the near-edge region up to $E_0+50$ eV, then
0.05 Å⁻¹ steps in k. Boxcar bins average absorption and retain the nominal grid
energy. Centroid bins also replace that energy with the mean measured energy in
the bin. Empty bins use linear interpolation. The finite outer bin boundaries
can omit points near the ends of the measured interval; inspect the returned
grid before interpreting endpoint features. Within-bin standard deviations
measure variation among the included samples, not propagated measurement errors.
See [`rebin` in the source](https://github.com/Ameyanagi/rexafs/blob/v0.2.4/crates/rexafs/src/xafs/tools.rs).

For a merge, rexafs first interpolates each spectrum onto the chosen grid within
the shared measured energy interval. At each energy it calculates

$$
\bar\mu=\frac{\sum_{i=1}^{N}w_i\mu_i}{W},\qquad
s=\sqrt{\frac{\sum_{i=1}^{N}w_i(\mu_i-\bar\mu)^2}{W(N-1)/N}},\qquad
W=\sum_{i=1}^{N}w_i.
$$

Here $N$ is the number of selected spectra, $\mu_i$ is interpolated absorption,
and $w_i$ is its finite, nonnegative relative weight, with $W>0$. The mean
$\bar\mu$ and reported spread $s$ retain absorption units. Equal weights give
the usual sample standard deviation; see the
[NIST definition of sample spread](https://www.itl.nist.gov/div898/handbook/eda/section3/eda356.htm).
For unequal weights, the denominator is a rexafs choice, not a general unbiased
weighted variance estimator. Zero-weight members still count in $N$; a single
member receives zero spread. This is the formula implemented by
[`merge_spectra`](https://github.com/Ameyanagi/rexafs/blob/v0.2.4/crates/rexafs/src/xafs/tools.rs),
not the standard error of the merged mean. It also does not estimate correlations
introduced by interpolation. Keep the individual scans when assessing whether
variation reflects noise, energy drift or an actual change in the sample.
