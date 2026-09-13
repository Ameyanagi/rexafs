---
title: "Processing controls"
description: "Use normalization, AUTOBK, forward and inverse transforms."
audience: user
---

Processing follows **normalization → AUTOBK → forward Fourier transform → optional
inverse transform**. The display and settings refer to the selected spectrum.
A stage computes the prerequisites it needs; changing an earlier setting makes
later results dependent on that change.

## Normalization

Set $E_0$ or leave it automatic. Pre/post-edge range controls are offsets from $E_0$
in eV. Choose a meaningful baseline on either side of the edge. **norm** subtracts
the pre-edge line and divides by the edge step; **flat** also removes the fitted
post-edge trend. Inspect the curve rather than treating automatic settings as
proof of a valid baseline.

Desktop initial choices can differ from the library's automatic range selection.
Blank fields select Auto, and placeholders report the resolved values. Keep the
resolved settings with your analysis when comparing interfaces or other software.

## Background

AUTOBK estimates the smooth background and produces $\chi(k)$. Begin with
$R_\mathrm{bkg}=1$ Å, the fixed endpoint penalty 0.001 and the direct solver.
The background objective uses $k$ weight 1; display/forward-transform weighting is
separate. **Link to FFT** is an explicit choice to relate weights.


[![Full background-removal view showing weighted Cu EXAFS and AUTOBK settings](/screenshots/background.jpg)](/screenshots/background.jpg)

*Background settings control the spline fit, while display weighting controls the plotted oscillations. Full application window, rexafs 0.2.4 on macOS. Select the image to view its full resolution.*


## Forward transform

The recommended starting window is 2–15 Å⁻¹ with $k$ weight 2, taper parameter
$dk=1$ Å⁻¹ and a Kaiser–Bessel window. Shorten the range for a noisier or narrower
measurement. View **k**, **R**, or **k + R**, and enable real/imaginary components
where available. Keep enough samples in the fitted window for a meaningful fit.

**Advanced → Larch grid** selects an explicit alternative sampling convention.
It does not change the physical model. See
[Fourier compatibility](/docs/science/fourier-compatibility/).

## Back transform

Expand **Back FT R → q** and choose an R window to isolate a contribution.
The automatic q step follows the R sampling and inverse FFT length. Changing the
maximum q display extent does not independently set its spacing.

The filtered signal retains the weighting/window applied before the forward
transform; it is not generally the original unweighted $\chi(k)$.
[Processing theory](/docs/science/processing/) defines the normalization and
Fourier signs, scales, units and limitations.
