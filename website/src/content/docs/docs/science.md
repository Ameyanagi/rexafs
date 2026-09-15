---
title: "Science overview"
description: "How the scientific guides fit together, and what each explains."
audience: user
---

Explore the calculations, assumptions and numerical conventions in rexafs 0.2.7.
Each guide links the theory to its implementation.

Read [concepts and glossary](/docs/concepts/) first if terms such as $\chi(k)$,
$R_{\mathrm{bkg}}$ or $S_0^2$ are unfamiliar.

## Guides

| Guide | Covers |
|---|---|
| [How processing works](/docs/science/processing/) | Absorption, normalization, $\chi(k)$ and Fourier transforms: equations, units and defaults. |
| [The AUTOBK objective](/docs/science/autobk/) | Background splines, $R_{\mathrm{bkg}}$ and the fixed endpoint penalty. |
| [Fourier and Larch compatibility](/docs/science/fourier-compatibility/) | Sampling grids, expected differences and legacy behavior. |
| [Fit statistics and uncertainties](/docs/science/fitting-statistics/) | Residuals, independent points, chi-square, covariance and R-factor. |
| [LCF, PCA and data treatment](/docs/science/analysis/) | Reference mixtures, principal components and preprocessing a collection. |
| [References and citations](/docs/science/references/) | Sources for the methods and guidance for citing your analysis. |

## The calculation chain in one paragraph

Normalization subtracts a fitted pre-edge baseline from $\mu(E)$ and divides by the
fitted edge step, so spectra of different thickness or concentration become
comparable. Converting energy above $E_0$ to wave number k and subtracting a smooth
AUTOBK background leaves $\chi(k)$, the EXAFS oscillations. Multiplying $\chi(k)$ by a
power of k, applying a window and Fourier transforming gives $\chi(R)$, whose peaks
lie near neighbor distances but are shifted by scattering phases. A structural
model sums scattering paths computed by ReFEFF or FEFF10, each with amplitude,
energy-shift, distance and disorder parameters, and a least-squares fit adjusts
those parameters until the model matches $\chi(R)$ or $\chi(k)$ inside the chosen ranges.
The fit reports standard errors from the local covariance and an R-factor for
the misfit; it does not decide whether the model is physically right.

## What the guides do not claim

- Regression tests check numerical agreement on tested inputs, not physical
  accuracy for every sample.
- Automatic settings are starting points. Inspect your windows, ranges and residuals.
- Fourier peaks are phase shifted; converged fits may describe nonunique
  structures. Check the [fit interpretation guidance](/docs/science/fitting-statistics/#what-to-inspect-before-interpreting-a-fit).
