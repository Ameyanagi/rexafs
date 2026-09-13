---
title: "Science overview"
description: "How the scientific guides fit together, and what each explains."
audience: user
---

The science section explains what rexafs calculates and why. It follows the
[documentation baseline](https://github.com/Ameyanagi/rexafs/blob/main/CONTRIBUTING.md):
each equation states the question it answers, defines every symbol and unit,
explains its parameters, states its assumptions and cites a verified reference.
Where rexafs makes a project-specific choice, the guide labels it as such and
links to the implementing source for the documented release.

Read [concepts and glossary](/docs/concepts/) first if terms such as χ(k),
R_bkg or S₀² are unfamiliar.

## Guides

| Guide | Question it answers | Read it when |
|---|---|---|
| [How processing works](/docs/science/processing/) | How does rexafs go from measured intensities to normalized absorption, χ(k), χ(R) and χ(q)? Which sign, scale and window conventions does it use? | You want to label axes correctly, compare with another program, or understand a default. |
| [The AUTOBK objective](/docs/science/autobk/) | What does the background spline minimize, what does the fixed endpoint penalty do, and how do R_bkg and the spline count affect χ(k)? | Background removal looks wrong, or you need to justify the background settings. |
| [Fourier and Larch compatibility](/docs/science/fourier-compatibility/) | How do rexafs's Fourier grid and legacy AUTOBK options relate to XrayLarch, and which differences are expected? | You compare results with Larch or reproduce an older analysis. |
| [Fit statistics and uncertainties](/docs/science/fitting-statistics/) | How are residuals, independent points, chi-square, covariance, standard errors and the R-factor defined and reported? | You interpret a fit or write the uncertainty section of a paper. |
| [LCF, PCA and data treatment](/docs/science/analysis/) | How do linear combination fits and principal components work, and what do alignment, rebinning, merging and smoothing change? | You analyze a collection of spectra or preprocess a series. |
| [References and citations](/docs/science/references/) | Which papers, specifications and software underpin the methods, and what should a publication cite? | You prepare a methods section or check a claim. |

## The calculation chain in one paragraph

Normalization subtracts a fitted pre-edge baseline from μ(E) and divides by the
fitted edge step, so spectra of different thickness or concentration become
comparable. Converting energy above E₀ to wave number k and subtracting a smooth
AUTOBK background leaves χ(k), the EXAFS oscillations. Multiplying χ(k) by a
power of k, applying a window and Fourier transforming gives χ(R), whose peaks
lie near neighbor distances but are shifted by scattering phases. A structural
model sums scattering paths computed by ReFEFF or FEFF10, each with amplitude,
energy-shift, distance and disorder parameters, and a least-squares fit adjusts
those parameters until the model matches χ(R) or χ(k) inside the chosen ranges.
The fit reports standard errors from the local covariance and an R-factor for
the misfit; it does not decide whether the model is physically right.

## What the guides do not claim

- Passing regression tests establishes agreement with a reference
  implementation for the tested inputs, not physical accuracy for every sample.
- Automatic settings are starting points chosen for typical spectra. Inspect the
  windows, ranges and residuals for your own measurement.
- A Fourier peak position is not a bond length, and a converged fit is not a
  unique structure. The statistics guide lists what to inspect before
  interpreting a result.
