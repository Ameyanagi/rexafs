---
title: "Fit your first coordination shell"
description: "Build and inspect a one-shell Cu EXAFS model."
audience: user
---

Continue the [Cu processing walkthrough](/docs/getting-started/first-analysis/)
with a one-shell model. For your own samples, choose appropriate ranges and
constraints; this example does not establish a unique structural interpretation.

## 1. Choose a structure and calculate paths

Open **Fit → Structure → Curated**, select **Cu / Copper**, then
**Use structure →**. In Calculate, select Cu as absorber, the K edge, an 8 Å
radius and **ReFEFF**. Choose **Calculate paths** and wait for completion.

Every 0.2.7 desktop package offers ReFEFF and FEFF10; see the
[Windows ARM64 requirement](/docs/getting-started/install/#arm64-availability).
Record the engine used.


[![Full structure and calculation setup for the built-in Cu model](/screenshots/structure.jpg)](/screenshots/structure.jpg)

*Cu scattering setup. Full window, rexafs 0.2.4 on macOS; select to enlarge.*


## 2. Select a model

Choose **First shell** in Paths, then **Set up model →**. The Cu example selects
the first Cu–Cu single-scattering path. The calculated reference distance and path
degeneracy describe the reference structure, not fitted results.


[![Full Paths stage with first-shell Cu scattering selected](/screenshots/paths.jpg)](/screenshots/paths.jpg)

*First-shell Cu path. Full window, rexafs 0.2.4 on macOS; select to enlarge.*


Inspect the model's amplitude, energy shift, distance change and disorder
parameters. Their meanings and units are described in
[structures and paths](/docs/desktop/structures/). Keep the initial template for
this example. In **Fit ranges**, inspect $k=2$–12 Å⁻¹ and $R=1$–3 Å. The transform
weight is 2. The fit is in R space; the visible k-space curves can extend beyond
the selected fitting range.


[![Full model view with k and R fitting limits](/screenshots/fit-ranges.jpg)](/screenshots/fit-ranges.jpg)

*R-space fit limits. Full window, rexafs 0.2.4 on macOS; select to enlarge.*


## 3. Run and inspect

Choose **Run fit**. In Results, inspect the measured/model curves, residuals,
parameter uncertainties, correlations where reported, and physical path distances.
A small R-factor and a converged optimizer do not establish that the model is unique
or physically complete.


[![Full Cu first-shell fit result with curves, residuals, parameters and statistics](/screenshots/fit-result.jpg)](/screenshots/fit-result.jpg)

*ReFEFF first-shell fit; higher shells lie outside the selected R range. Full window, rexafs 0.2.4 on macOS; select to enlarge.*


The screenshot's noise scale is 1. Its uncertainties therefore depend on that
scale and the local covariance approximation; they are not a full experimental
error budget. Read [fitting statistics](/docs/science/fitting-statistics/) before
interpreting the reported standard errors.

Save the project to retain the model/history, and use
[publication exports](/docs/desktop/publication/) to preserve data and methods.
For shared variables across spectra, continue with
[multiple-spectrum fitting](/docs/desktop/multiple-spectra/).
