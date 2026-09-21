---
title: "Synthetic copper: PCA, MCR-ALS and LCF"
description: "Follow a known CuO to Cu2O to Cu recipe, recover components, and compare fractions with ruviz figures."
audience: user
pagefind: false
---

This tutorial uses an **unreleased desktop preview** with 50 deterministic
synthetic spectra and three measured references: CuO, Cu₂O and Cu. The example
starts with CuO, passes through a Cu₂O-rich region and ends with Cu. It is a
teaching sequence with known fractions, not a measured reaction or kinetic model.
It is not included in the published 0.2.12 installer.

The spectra were mixed in **raw absorption μ(E)** before normalization. No random
fractions or additional noise were generated. The original measurements retain
their noise and calibration. The desktop opens this example with its normal
automatic processing settings and no per-spectrum overrides.

## Open the example

1. Choose **Synthetic copper reduction…** on the start screen or in **Help**.
   Alternatively, open `crates/rexafs-gui/data/examples/cu-reduction.rxs` from this
   source checkout. All input spectra are embedded in the project.
2. Check that there are **53 groups**: three references and 50 groups beginning
   with **Synthetic**. Only the 50 mixtures should be marked.
3. Keep the automatic normalization settings. Select **Data** to inspect the
   measured or processed spectra. Changing a plot's viewing range does not change
   an analysis calculation's range.
4. Save your work under a new project name. Group labels contain the known raw
   mixing percentages rounded to one decimal place; full-precision truth is
   retained in the project metadata and companion fraction table.

[![Known raw-absorption fractions across 50 synthetic frames](/figures/synthetic-copper/recipe.svg)](/figures/synthetic-copper/recipe.svg)

*Generated with ruviz. Frame 1 is 100% CuO; frame 50 is 100% Cu. Cu₂O reaches
85.07% at frames 25 and 26. Frame number has no experimental time unit. Solid,
dashed and dotted curves distinguish CuO, Cu₂O and Cu even without color.*

Download: [vector PDF](/figures/synthetic-copper/recipe.pdf),
[editable SVG](/figures/synthetic-copper/recipe.svg), or
[600 dpi PNG](/figures/synthetic-copper/recipe.png).

## Use PCA to inspect the component count

Principal component analysis (PCA) describes independent spectral directions.
It helps choose a component count, but its directions are not automatically
pure chemical species.

1. Leave only the **50 mixtures marked**; leave the references unmarked.
2. Open the command palette, find **Principal components**, and open that tool.
   In Data, the same tool is available in the analysis controls.
3. Keep **flat**, **Subtract mean off**, and the default calculation range
   (**Auto**, −20 to +30 eV relative to the first spectrum's E₀).
4. Run the calculation. Inspect **Scree**, **Cumulative**, and **Error vs count**.
   The preview starts with **Y axis → Linear**. Log remains an optional display.
5. Inspect three retained components in Reconstruction. The reconstruction count
   does not limit how many principal components the training calculation finds.

The first three components retain **99.9998568% of the squared signal** in this
run. Their contributions are 99.70213%, 0.22785% and 0.06987%; the fourth contributes
0.0001431%. These are uncentered squared-signal percentages, not centered variance.
Automatic preprocessing adds small extra directions, so the reported numerical
rank can exceed three. Three dominant components are consistent with the known
recipe; numerical rank alone does not establish chemical identity.

[![Uncentered PCA on a linear scale, with a separate linear detail of components two through six](/figures/synthetic-copper/pca.svg)](/figures/synthetic-copper/pca.svg)

*Both panels use linear axes. The detail panel makes the small contributions
visible without changing the calculation or replacing the default scale.
Symbols show the calculated components; connecting lines guide the eye.*

Download: [vector PDF](/figures/synthetic-copper/pca.pdf),
[editable SVG](/figures/synthetic-copper/pca.svg), or
[600 dpi PNG](/figures/synthetic-copper/pca.png).

With **Subtract mean** enabled, an exactly linear, sum-to-one mixture of three
independent references has two varying directions plus its mean. Automatic
preprocessing can add further small directions. Do not interpret two dominant
centered directions as evidence that only two species are present.

## Recover spectra and fractions with MCR-ALS

Multivariate curve resolution by alternating least squares (MCR-ALS) estimates
component spectra and their weights together. This run is blind: neither the
reference spectra nor the known fractions enter the fit.

1. Keep the **50 mixtures marked**, with all references unmarked.
2. Open **Resolve components (MCR-ALS)** from the command palette or Data tools.
3. Keep the desktop defaults: **flat**, **3 components**, **Fractions sum to 1**
   enabled, **Nonnegative spectra** disabled, **500 maximum iterations**, and
   **initialization seed 0**. Signed spectra allow small baseline-subtracted
   negative values; the coefficients remain nonnegative.
4. Leave both range endpoints **Auto**, with **Common full range** selected.
   The preview resolves the measured overlap after loading the selected spectra.
   Here this uses all **517 points from 8780.206 to 9768.204 eV**.
5. Choose **Resolve components**. Inspect **Spectra**, **Fractions**,
   **Convergence**, and **Residuals**. This run converged in **110 iterations**.
6. To identify the estimated spectra afterward, use **Group actions → Clear
   marks**, mark only the three **Reference** groups, and choose **Compare marked
   standards** in the MCR result. This compares the saved components without
   supplying references to the completed fit.
7. Save the project to retain the result arrays and settings.

Component numbers are arbitrary. In this run, components 1, 3 and 2 matched CuO,
Cu₂O and Cu, respectively. Matching is a comparison with known references, not
independent proof of identity. MCR can reconstruct spectra very accurately while
its component spectra and fractions still differ from the generating recipe.

## Compare with known-reference LCF

Linear combination fitting (LCF) holds the reference spectra fixed and estimates
their weights. Use the three measured references supplied with the example.

For a single spectrum:

1. Clear the mixture marks and **mark only the three Reference groups**. Select a
   synthetic spectrum as the current group without marking it.
2. Open **Linear combination fit**. Choose **flat**, keep **Σ = 1** (sum to one) enabled,
   and leave **E₀ shifts** disabled.
3. After the target and references load, choose **Common full range**, then
   **Fit**. Inspect the coefficients, fitted curve and residual.
4. When selecting another target, choose **Common full range** again: LCF stores
   energy offsets relative to that target's E₀, which can change between frames.

For an interactive trend across the series, keep just the references marked,
set the LCF range in Data, then choose **Series → series → All frames → Run LCF
trend**. The Series display representation does not change the LCF fitting space.
Check completion and failed rows before exporting. The saved series includes one
weight per reference and a spectral R-factor for each successful frame.

**Range distinction:** Series LCF currently reuses one pair of offsets relative
to each frame's E₀. With automatic E₀, that is not one fixed absolute energy
interval. A default XANES Series trend therefore does not reproduce the
full-range table below. The reproducible native command below calculates the
full-range offsets separately for every target and verifies identical LCF/MCR
input arrays. Do not copy one target's full-range offsets into the whole series.

## Read the comparison correctly

The labels give **raw μ mixing weights**, while MCR and LCF here fit flattened,
edge-step-normalized spectra. Comparing those two kinds of coefficients directly
can be misleading. For this validation, each fitted weight was divided by the
corresponding pure-reference edge step, then the three weights were rescaled to
sum to one. The automatically determined reference steps were:

| Reference | Edge step in source absorption units |
| --- | ---: |
| CuO | 0.637679294 |
| Cu₂O | 1.120257832 |
| Cu | 1.050925701 |

This is an **approximate conversion** with automatic preprocessing: the baseline
fits and E₀ can vary between spectra. It is performed by the comparison helper,
not automatically by the desktop Fractions view. The resulting numbers remain
raw-signal multipliers, not calibrated mass or atomic fractions.

The local optimized macOS preview was checked on 2026-09-22. The full-range LCF
calculation used the native rexafs engine with the same 50 processed spectra,
517-point energy grid, nonnegative sum-to-one coefficients and no fitted energy
shifts. References were supplied to LCF and withheld from MCR.

| Check against the known raw recipe | Blind MCR | Known-reference LCF |
| --- | ---: | ---: |
| Mean absolute error across all 150 coefficients | 1.051 percentage points | **0.047 percentage points** |
| Maximum absolute error | 2.800 percentage points | **0.533 percentage points** |
| Cu estimate at the 100% Cu endpoint | 97.200% | **100.000%** |
| Relative squared spectral residual | 2.95608 × 10⁻⁷ | 3.22965 × 10⁻⁷ |

[![MCR and LCF fractions compared with the known recipe, with fraction errors below on matching axes](/figures/synthetic-copper/comparison.svg)](/figures/synthetic-copper/comparison.svg)

*Generated with ruviz from the retained numerical results. In panels a–b, solid
lines connect the raw recipe values; open symbols show the recovered fractions
at all 50 frames after the approximate edge-step conversion. Circles identify
CuO, squares Cu₂O and triangles Cu. Panels c–d show estimate minus recipe on
identical vertical scales; connecting lines guide the eye. Summary errors cover
all 150 coefficients. A percentage point (pp) is an absolute difference between
percentages: 97.2% versus 100% differs by 2.8 points.*

Download the publication figure: [vector PDF](/figures/synthetic-copper/comparison.pdf),
[editable SVG](/figures/synthetic-copper/comparison.svg), or
[600 dpi PNG](/figures/synthetic-copper/comparison.png).
The figure is 180 × 146.05 mm; the PNG is 4252 × 3450 pixels. The PDF embeds its
fonts. All formats preserve the same data and panel scales.

LCF recovered the fractions more closely in this example because it used the
known reference spectra. MCR had a slightly lower spectral residual while its
fraction errors were larger. The residual is the sum of squared spectral errors
divided by the sum of squared input values across all 50 frames; it measures
reconstruction, not composition accuracy or statistical confidence.

At frame 25, the raw recipe is **8.882% CuO, 85.069% Cu₂O and 6.049% Cu**.
After conversion, LCF gives **8.872%, 85.052% and 6.077%**, while MCR gives
**8.009%, 86.295% and 5.695%**. These are measured results for this teaching
example, not general error guarantees.

The [retained result data](/figures/synthetic-copper/results.json)
include every raw truth value, normalized fit weight, converted estimate,
reference step and PCA contribution used in the figures. An independent
constrained least-squares check agreed with native LCF weights within
1.9 × 10⁻¹². The native reproduction also matches the earlier desktop results.

## Browse the evolution and differences in Series

1. Open **Series → Select series or scan → series**. This selects the 50-frame
   mixture scan; the separate `references` scan contains the three standards.
2. Choose **flat μ(E)** for absorption, **k²χ(k)** for weighted EXAFS, or
   **|χ(R)|** for Fourier magnitude. Click a heatmap row, use Previous/Next, or
   enter **Frame** and press Enter to move through the sequence.
3. Turn on **Difference** and leave **Ref: 1** selected to subtract the pure-CuO
   starting frame. Frame 1 becomes zero; frame 50 shows the Cu endpoint minus CuO.
   With the automatic palette, red is positive and blue is negative.
4. Use **Add trend…** to define a metric, such as a maximum or integral, and
   **Calculate all 50 frames** to evaluate it. Review saved runs in **Results…**.

For **|χ(R)|**, Difference subtracts the two magnitudes; it is not the magnitude
of a complex Fourier difference. The lower edge-energy or saved measurement
trend keeps its original definition. Frame number has no assigned time unit,
and white-line height is a signal metric rather than a concentration.

## Reproduce the figures

The figures were rendered with **ruviz 0.14.2** from the retained numerical
arrays. They were not traced from screenshots. The compact
[result data](/figures/synthetic-copper/results.json) include all 50 recipe rows,
MCR and LCF weights, approximate converted fractions, reference edge steps and
PCA contributions. All three figures use 7.5–10 point Helvetica text at a fixed
180 mm width. Species are distinguished by color and line style or marker shape.
SVG files stay sharp when enlarged; the 600 dpi PNGs include physical resolution
metadata. The vector PDFs contain embedded fonts.

From a source checkout containing this preview, these commands extract the
bundled raw data, repeat the calculations, and render the figures. They require
Python 3 and the repository's Rust toolchain; the original Athena file is not
needed. Choose an output directory that does not already exist.

```sh
python3 scripts/extract-cu-reduction.py \
  crates/rexafs-gui/data/examples/cu-reduction.rxs /tmp/cu-tutorial
cargo run --locked --release -p rexafs --example cu_reduction_check -- \
  /tmp/cu-tutorial --desktop-defaults
cargo run --locked --release -p rexafs --features plotting \
  --example cu_reduction_figures -- \
  /tmp/cu-tutorial/tutorial-results.json /tmp/cu-tutorial/figures
```

Extraction verifies the stored checksums and copies the spectra without
regenerating or processing them. The calculation helper uses the desktop's
automatic preprocessing and explicitly resolves the same absolute energy bounds
for each LCF target. The renderer reads the resulting `tutorial-results.json`;
it performs no further fitting or smoothing.

To export all three figures as vector PDFs and losslessly compress the PNGs,
install Cairo and run the export helper (which uses CairoSVG 2.8.2):

```sh
uv run --no-project --python 3.12 scripts/export-cu-figures.py \
  /tmp/cu-tutorial/figures
```

On macOS with Homebrew Cairo, prefix that command with
`DYLD_FALLBACK_LIBRARY_PATH="$(brew --prefix)/lib"` if Cairo is not found.

## Data and methods

The measured references come from the user-supplied group measurement project
`Cu oxides.prj`, using labels `cuo_abs`, `cu2o_abs`, and `cufoil_abs`.
Its SHA-256 is `18684eb2c4776d5d3661f6ad6fdfae6fb81d91571bda00611ec33bd686c67dc6`.
Raw references were linearly interpolated onto the foil's shared measured grid,
without energy shifts or extrapolation, then mixed. The historical 100-mixture
example is a separate dataset; the present tutorial uses 50 deterministic raw-μ
mixtures. Additional experimental conditions are not inferred from filenames.

For a raw mixture coefficient $f_j$ and reference edge step $\Delta_j$, shared
linear baseline processing gives normalized coefficient
$g_j = f_j\Delta_j / \sum_k f_k\Delta_k$. The comparison approximately reverses
that scaling with $f_j = (g_j/\Delta_j) / \sum_k(g_k/\Delta_k)$.
Here $j$ and $k$ index the three references, $f$ and $g$ are dimensionless, and
$\Delta$ retains the input absorption units. Automatic per-spectrum baseline
choices need not preserve this exact relation. See the
[normalization explanation](https://xraypy.github.io/xraylarch/xafs_preedge.html)
and [rexafs processing guide](/docs/science/processing/).

For the MCR model and alternating least-squares method, see
[Camp (2019)](https://doi.org/10.6028/jres.124.018) and
[official pyMCR documentation](https://pages.nist.gov/pyMCR/).
rexafs uses its own Rust implementation, initialization and stopping defaults.
The [source-checkout guide](https://github.com/Ameyanagi/rexafs/blob/dev/doc/synthetic-copper-reduction.md)
links the calculation, normalization, range-selection and ruviz rendering code.
