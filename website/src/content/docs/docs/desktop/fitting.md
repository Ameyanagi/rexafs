---
title: "Fit your first coordination shell"
description: "Build and inspect a one-shell Cu EXAFS model."
audience: user
---

This example continues the [Cu processing walkthrough](/docs/getting-started/first-analysis/).
It illustrates how to construct a fit, not how to establish a unique structural
interpretation. Use ranges and constraints appropriate to your actual sample.

## 1. Choose a structure and calculate paths

Open **Fit → Structure**. In **Curated**, choose the built-in **Cu / Copper**
structure. Choose **Use structure →**. In Calculate, select Cu as absorber, the
K edge and a calculation radius of 8 Å. Use **ReFEFF** for this walkthrough and
choose **Calculate paths**. Wait for the calculation to complete.

The available backend depends on the package/platform. The Windows build uses
ReFEFF; the Mac package also offers FEFF10. Record the engine actually used.


[![Full structure and calculation setup for the built-in Cu model](/screenshots/structure.jpg)](/screenshots/structure.jpg)

*The absorber, edge and cluster radius define the scattering calculation. Full application window, rexafs 0.2.4 on macOS. Select the image to view its full resolution.*


## 2. Select a model

Choose **First shell** in Paths, then **Set up model →**. The Cu example selects
the first Cu–Cu single-scattering path. The calculated reference distance and path
degeneracy describe the reference structure, not fitted results.


[![Full Paths stage with first-shell Cu scattering selected](/screenshots/paths.jpg)](/screenshots/paths.jpg)

*One path is selected from the calculated Cu path collection. Full application window, rexafs 0.2.4 on macOS. Select the image to view its full resolution.*


Inspect the model's amplitude, energy shift, distance change and disorder
parameters. Their meanings and units are described in
[structures and paths](/docs/desktop/structures/). Keep the initial template for
this example. In **Fit ranges**, inspect $k=2$–12 Å⁻¹ and $R=1$–3 Å. The transform
weight is 2. The fit is in R space; the visible k-space curves can extend beyond
the selected fitting range.


[![Full model view with k and R fitting limits](/screenshots/fit-ranges.jpg)](/screenshots/fit-ranges.jpg)

*The chosen fit domain and independent-variable count determine how to interpret the statistics. Full application window, rexafs 0.2.4 on macOS. Select the image to view its full resolution.*


## 3. Run and inspect

Choose **Run fit**. In Results, inspect the measured/model curves, residuals,
parameter uncertainties, correlations where reported, and physical path distances.
A small R-factor and a converged optimizer do not establish that the model is unique
or physically complete.


[![Full Cu first-shell fit result with curves, residuals, parameters and statistics](/screenshots/fit-result.jpg)](/screenshots/fit-result.jpg)

*Illustrative ReFEFF first-shell fit: the higher-shell signal is outside this model’s selected R range. Full application window, rexafs 0.2.4 on macOS. Select the image to view its full resolution.*


The screenshot's noise scale is 1. Its uncertainties therefore depend on that
scale and the local covariance approximation; they are not a full experimental
error budget. Read [fitting statistics](/docs/science/fitting-statistics/) before
interpreting the reported standard errors.

Save the project to retain the model/history, and use
[publication exports](/docs/desktop/publication/) to preserve data and methods.
For shared variables across spectra, continue with
[multiple-spectrum fitting](/docs/desktop/multiple-spectra/).
