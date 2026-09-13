---
title: "LCF, PCA and data treatment"
description: "Analyze collections while keeping preprocessing assumptions explicit."
audience: user
---

These operations are available in the desktop and Rust. The stable Python and
TypeScript bindings currently expose spectrum processing rather than these tools.
In the desktop, find data treatment and analysis tools in **Data → Parameters**
or through the action search.

## Linear combination fitting

LCF models an unknown spectrum as a weighted sum of reference spectra:

$$\min_{\mathbf w}\sum_{j=1}^{M}\left[U(x_j)-\sum_{i=1}^{S}w_iS_i(x_j)\right]^2.$$

$U$ is the unknown, $S_i$ the $i$th standard, $w_i$ its dimensionless weight,
$M$ the number of fitted samples and $S$ the number of standards. The axis $x$
is energy in eV or k in Å⁻¹, depending on the analysis space. All spectral arrays
must use the same units and normalization. Standards are interpolated onto the
unknown's grid in the selected range.

The default bounds are $0\le w_i\le1$ with $\sum_iw_i=1$. Energy-space ranges
start at −20 to +30 eV relative to $E_0$; χ-space ranges start at 3–12 Å⁻¹.
Optional shifts move each standard's axis before fitting; they are off by default.
Weights only support composition claims when the standards and measurement model
justify that interpretation. Similar standards can give ambiguous weights.

rexafs uses bounded least squares, with a nonlinear outer fit when shifts are
allowed. The generated [Rust API](/api/rust/rexafs/xafs/analysis/index.html) documents
`LcfConfig`, `lcf` and combination searches. For the general XANES use of reference
mixtures, see [Larch's linear-analysis guide](https://xraypy.github.io/xraylarch/xafs_xanes.html).

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

The [Rust PCA API](/api/rust/rexafs/xafs/analysis/pca/index.html) includes the actual
configuration, variance convention, indicator function and target transform.
[Larch's PCA explanation](https://xraypy.github.io/xraylarch/xafs_xanes.html#principal-component-analysis)
is useful background; its default centering differs from rexafs.

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
