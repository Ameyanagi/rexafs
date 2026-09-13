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

## Calculate and select paths

Choose the engine and cluster radius, then **Calculate paths**. ReFEFF and FEFF10
are different backends; availability depends on the installed package. Windows
0.2.4 uses ReFEFF. Record the actual engine and settings in your methods.

In Paths, use **First shell**, **To fit R max** or the importance filter as a
starting selection, then inspect individual paths. Single and multiple scattering
can contribute in the same R region. A large path count does not justify a large
number of independently fitted variables.


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

## Inspect the structure

**Structure display** contains atom/bond styles, absorber highlighting and view
effects. Center focus and depth cue help expose the neighborhood. Slice/depth
controls change what is visible; they do not remove atoms from the calculation.
A display bond is a geometric heuristic, not a bond-order measurement.

The [first-fit tutorial](/docs/desktop/fitting/) shows the complete sequence with
one Cu spectrum and its full application screenshots.
