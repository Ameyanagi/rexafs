# Notebook review and analysis figures

Reviewed on 16 September 2026. These user-supplied notebooks are design and
algorithm references. Their code and saved figures were inspected without
executing installation, download or file-renaming cells. The original notebooks
remain unchanged and are not redistributed with rexafs.

| Notebook | SHA-256 |
| --- | --- |
| `PCA_Analysis_composition_dependence.ipynb` | `c0dc9933b87982ffabf28c7d3c920d8b9cfee2b55154586dec801bd91063ebc8` |
| `MCR_Implementation_Cu.ipynb` | `43e2cf1c86452c38ca3c3d8b8d6a92d46fdeec9450de64d410dca84b044ad056` |

## PCA notebook

The notebook reads ten normalized Ti spectra from
[`pkrouth/XAFS2023`](https://github.com/pkrouth/XAFS2023), selects −20 to +80 eV
relative to E₀ = 4971.230 eV, and fits scikit-learn PCA with spectra as rows.
This PCA subtracts the mean spectrum. Imported scaling classes are not applied
in the demonstrated calculation. Its figures are:

- Normalized spectra overlaid in the chosen energy interval (cells 19–25).
- Explained variance ratio against component number, a scree plot (cell 29).
- PC1/PC2 sample scores and a three-dimensional PC1/PC2/PC3 view (cells 33–34).
- Cosine similarity of the full PCA score vectors as a heatmap (cells 40–49).

The heatmap is a **similarity** matrix, not a distance matrix. Negative values
are legitimate for mean-centered score vectors. It does not establish a
chemical component count. A zero score vector has undefined cosine similarity.
The cell labeled missing-value checking uses `all(isna())`, which detects an
entirely missing column, not an individual missing value.

## MCR notebook

The worked example uses 100 mixtures, 125 spectral points and three components.
It uses pyMCR 0.5.1 with nonnegative least-squares updates for concentrations and
spectra, nonnegativity constraints and concentration normalization. Its figures
show initial concentrations (cell 45), fitted concentrations across samples,
recovered spectra, and side-by-side true and recovered spectra (cell 48).

The notebook uses random initial concentrations. That branch does not apply the
`RANDOM_STATE` setting, so rerunning it is not reproducible. Methods refer to the
global `fit_params` rather than consistently to `self.fit_params`. Optional
standardization prepares a separate matrix, but the solver still receives the
original matrix. These prototype behaviors should not become rexafs behavior.

The saved comparison plots use point indices rather than measured energy and do
not match recovered component permutations to the references. Consequently,
the same curve color denotes different components on the two sides. The
concentration truth file is loaded but no fitted-versus-true fraction plot is
shown. The printed final error is the last iteration; the plotted arrays are
pyMCR's best iteration. These should be distinguished in a results view.

## rexafs results workflow

The following figures belong in the analysis workspace, with compact view
selectors rather than additional parameter panels:

| Analysis | Views | Interpretation |
| --- | --- | --- |
| PCA | Scree, cumulative fraction, IND, scores, loadings, similarity, reconstruction | Estimate useful directions, inspect clusters, check reconstruction. |
| MCR-ALS | Component spectra, fractions, convergence, residuals | Inspect recovered factors and whether the model describes every sample. |
| Validation against references | Matched spectral overlays and fraction parity | Compare known truth with estimates; do not infer chemical names from curve order. |

The PCA notebook's two-dimensional score plot is the primary grouping view.
Pairwise PC1/PC2, PC1/PC3 and PC2/PC3 views provide the three projections without
requiring a new three-dimensional rendering dependency. A cumulative fraction
plot and logarithmic scree plot supplement its linear scree plot: common edge
shape can dominate uncentered spectra and obscure smaller independent features.

Mean centering is an explicit choice. For the noise-free copper experiment,
three independent standard spectra produce an uncentered rank of three.
Fractions that sum to one leave only two independent composition changes after
subtracting the mean. Two centered principal components therefore do not imply
two chemical species. See the [core PCA implementation](../../../crates/rexafs/src/xafs/analysis/pca.rs)
and [Larch's centered PCA example](https://xraypy.github.io/xraylarch/xafs_xanes.html#pca-example).

The GUI must not interpret an indicator minimum deep in floating-point roundoff
as dozens of chemical components. Numerical rank is a precision diagnostic;
Malinowski IND is a heuristic; a cumulative fraction threshold is a user-chosen
reconstruction criterion. None supplies a chemical-species count by itself.

Native rexafs MCR uses the existing Rust matrix and constrained least-squares
facilities. pyMCR remains an independent development comparison, as described
in the [experiment plan](../../cu-mixture-recovery-plan.md). Its exact constrained
concentration solve differs from the notebook's NNLS followed by normalization;
the latter is generally not the minimizer of the sum-to-one least-squares problem.
See [pyMCR documentation](https://pages.nist.gov/pyMCR/) and
[Camp (2019)](https://doi.org/10.6028/jres.124.018).
