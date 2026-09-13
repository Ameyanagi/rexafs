---
title: "Structures and scattering paths"
description: "Choose structures, inspect an absorber and build a FEFF path model."
audience: user
---

The **Fit** workspace separates **Structure**, **Calculate**, **Paths**, **Model**
and **Results**. A structure supplies the reference geometry for scattering
calculations; measured EXAFS supplies the observations used in fitting.

## Find a structure

Choose a built-in **Curated** structure for an offline example, import CIF/XYZ,
or search an available database. The interface offers Materials Project, AMCSD
and COD; connectivity, downloaded catalogs and credentials depend on the source.
Materials Project credentials belong to computer settings, not the saved project.
Retain the structure's database identifier and attribution.

Select the absorber and edge deliberately. The inspection center in a 3D view
can differ from the calculation absorber. A visual selection does not itself
change the scattering calculation. Check the lattice, coordination and occupancy
against what is known about your sample.

The desktop starts with an 8 Å cluster radius and accepts 2–12 Å. For periodic
CIF structures, it uses the majority species at a partially occupied site and
excludes hydrogen from the calculation cluster. These defaults do not simulate
a distribution of disordered or vacant sites. The separate display controls can
show a different neighborhood; confirm the calculated cluster before fitting.
See the [desktop cluster
construction](https://github.com/Ameyanagi/rexafs/blob/v0.2.4/crates/rexafs-gui/src/app/shell/structure_view.rs#L1088).

## Calculate and select paths

Choose the engine and cluster radius, then **Calculate paths**. ReFEFF and FEFF10
are different backends; availability depends on the installed package. Windows
0.2.4 uses ReFEFF. Record the actual engine and settings in your methods.

In Paths, use **First shell**, **To fit R max** or the importance filter as a
starting selection, then inspect individual paths. Single and multiple scattering
can contribute in the same R region. A large path count does not justify a large
number of independently fitted variables.

The **Importance ≥ 10 %** shortcut selects paths whose estimated amplitude is
at least 10% of the largest estimate in the collection. Importance comes from
the mean estimated amplitude over 3–12 Å⁻¹, scaled to 0–100. It is a path-selection
heuristic, not a fitted fraction, probability or statistical significance.
**To fit R max** includes paths with reference half-length up to the fit's R
maximum plus 0.3 Å; that margin is a UI choice, not a phase correction. Presets
apply to the visible calculation source and preserve selections from other
sources. Inspect multiple-scattering contributions explicitly where relevant.
See the [path presets](https://github.com/Ameyanagi/rexafs/blob/v0.2.4/crates/rexafs-gui/src/app/shell/path_picker.rs#L60).


[![Full path selection view showing Cu geometry and shell-grouped scattering paths](/screenshots/paths.jpg)](/screenshots/paths.jpg)

*The first-shell shortcut selects a starting model; it does not validate that model for every sample. Full application window, rexafs 0.2.4 on macOS. Select the image to view its full resolution.*


## Path parameters

A model adds the selected path contributions,

$$\chi_{\mathrm{model}}(k)=\sum_{j=1}^{P}\chi_j(k;\boldsymbol\theta_j).$$

Here $P$ is the number of paths, $k$ is wave number in Å⁻¹, and
$\boldsymbol\theta_j$ contains the adjustable parameters for path $j$. Each
$\chi_j$ and their sum are dimensionless before k weighting. The contributions
include scattering amplitudes and phases from the chosen engine. The theory is
reviewed by [Rehr and Albers (2000)](https://doi.org/10.1103/RevModPhys.72.621).

| Parameter | Meaning | Units |
|---|---|---|
| $S_0^2$ | Amplitude reduction factor | Dimensionless |
| $\Delta E_0$ | Energy shift in the path model | eV |
| $\Delta R$ | Change from the path's reference half-length | Å |
| $\sigma^2$ | Mean-square relative displacement / disorder term | Å² |
| Path degeneracy | Number of equivalent paths in the reference calculation | Dimensionless |

For a single-scattering pair, the reported physical distance is
$R=R_{\mathrm{eff}}+\Delta R$, where $R_{\mathrm{eff}}$ is the reference path
half-length. A multiple-scattering half-length is not a single interatomic bond.
Expressions can share variables or constrain them across paths; use the full
covariance when propagating uncertainty through an expression. See
[fitting statistics](/docs/science/fitting-statistics/).

## Parameter templates

The Model stage offers four starting arrangements. Inspect the generated values,
bounds and expressions before fitting; the template cannot establish which
constraints your material obeys.

| Desktop choice | Parameters it creates or preserves |
|---|---|
| **Share by shell** (default) | One distance change and disorder variable per selected single-scattering shell; amplitude and energy shift are shared within each structure. |
| **Separate by path** | Each selected path has its own distance/disorder pair; amplitude and energy shift remain shared within each structure. This increases the number of fitted variables. |
| **Nearest shell** | Only the nearest selected shell's distance/disorder pair varies. Other paths use $\Delta R=0$ Å and $\sigma^2=0.003$ Å². The nearest selected shell need not be the first shell in the complete structure. |
| **Custom expressions** | Preserve the current model and edit the path expressions. Reusing a variable name shares that parameter. |

For **Share by shell**, a multiple-scattering path's distance change is the mean
of the shell distance-change variables associated with its recognized scatterers.
Its disorder term uses the outermost recognized shell's variable, rather than
the largest fitted disorder value. These are rexafs starting constraints; they
are not a physical derivation of correlated multiple-scattering motion. Examine
the generated expressions and replace them when your model needs different
constraints. **Apply template (replaces edits)** can overwrite manual model edits.
See the [desktop template chooser](https://github.com/Ameyanagi/rexafs/blob/v0.2.4/crates/rexafs-gui/src/app/shell/fit.rs#L650)
and [template implementation](https://github.com/Ameyanagi/rexafs/blob/v0.2.4/crates/rexafs/src/xafs/fitting/template.rs).

## Inspect the structure

**Structure display** contains atom/bond styles, absorber highlighting and view
effects. Center focus and depth cue help expose the neighborhood. Slice/depth
controls change what is visible; they do not remove atoms from the calculation.
A display bond is a geometric heuristic, not a bond-order measurement.

The [first-fit tutorial](/docs/desktop/fitting/) shows the complete sequence with
one Cu spectrum and its full application screenshots.
