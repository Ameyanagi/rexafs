---
title: "Multiple spectra and batches"
description: "Share parameters across spectra or fit a collection independently."
audience: user
---

**Model & fit → Fit multiple spectra** fits several spectra simultaneously.
The browser groups paths under their spectrum. Selecting a spectrum shows its
range and every assigned path's parameter values; selecting a path focuses it.

For the mathematical objective and the meaning of shared parameters,
chi-square, covariance and R-factor, see [fitting statistics](/docs/science/fitting-statistics/).
A joint fit combines residuals under shared parameter constraints; it does not
make duplicated spectra independent physical measurements.

## Workflow

1. Configure the needed paths in **Paths**, including paths from different
   calculated structures where needed.
2. Choose **Fit multiple spectra**. Add the current file, or mark several files
   in the file browser and choose **+ Marked**.
3. Use **± Paths** beside a spectrum to change its assignments. Each path shows
   its calculation directory, reference distance, and number of legs.
4. Edit the displayed initial values. **Global** uses one variable across spectra;
   **This spectrum** gives that spectrum its own value and Fit toggle. Paths
   referencing the same name within a spectrum still share that variable.
   New multi-spectrum setups start distance/disorder variables per spectrum.
5. Set each spectrum's k/R range. **Transform (k)** follows that file's processing
   k-weight, including per-file overrides. Number buttons select manual fit
   weights. New models follow the transform; old projects keep saved weights.
6. **Advanced** exposes expressions for each path in the selected spectrum.
   Editing these does not change other spectra or the source path template.
   Undefined variable names have an **Add parameter** action. A numeric constant
   can become a variable with **Fit this value**.
7. Run the fit. Select a spectrum above the result plots to inspect its model,
   residuals, contributions, and R-factor. The result panel includes global
   statistics and each fitted variable's uncertainty. Re/Im χ(R) are selectable.

Assignments use path file identities rather than catalog indices. Projects and
history preserve scopes, local starting values, fit/fixed choices, per-spectrum
ranges, and per-path expressions. History includes physical path distances with
uncertainties propagated through the full covariance matrix.

A global constrained parameter cannot depend on a local parameter. Undefined
variables, invalid ranges, missing paths, empty assignments, and duplicate spectra
prevent fitting. Failed preprocessing reports the file name; it never silently
omits a dataset.

## Independent batches

**Results & batch → Batch** fits each frame independently, using its effective
processing parameters. Automatic fit weights also follow each frame's transform.
Select **Single spectrum** before starting a batch. Each row has solver status
and uncertainties; failures appear in Problems. CSV exports values and errors.
