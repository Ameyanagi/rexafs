---
title: "Processing controls"
description: "Use normalization, AUTOBK, forward and inverse transforms."
audience: user
---

Loading an absorption spectrum or changing its settings runs **normalization →
AUTOBK → forward Fourier transform → inverse transform**. Selecting a stage
changes the display and controls. The inverse transform runs even with its
controls collapsed, so invalid back-transform settings can fail processing.
See the [0.2.4 desktop pipeline](https://github.com/Ameyanagi/rexafs/blob/v0.2.4/crates/rexafs-gui/src/params.rs#L1589).

The inspector edits the **current** group. **Apply to N** copies only the selected
processing stage to eligible marked groups, excluding the current group and
processing-locked groups. **Reset** restores that stage from the project's
defaults. Copying all processing settings does not also copy column mappings or
reference calibration; copying a column mapping is a separate guarded action.
An Auto request is copied as Auto, so its resolved value can differ by spectrum.

## Normalization

Set $E_0$ or leave it automatic. Pre/post-edge range controls are offsets from $E_0$
in eV. Choose a meaningful baseline on either side of the edge. **norm** subtracts
the pre-edge line and divides by the edge step; **flat** also removes the fitted
post-edge trend. Inspect the curve rather than treating automatic settings as
proof of a valid baseline.

Blank fields select Auto; placeholders show the resolved values. New 0.2.4
desktop analyses start with these choices:

| Setting | Starting value |
|---|---|
| Pre-edge range, relative to $E_0$ | −200 to −30 eV |
| Post-edge range | $E_0+150$ eV to the measured energy endpoint |
| Post-edge polynomial order | 2 |
| Victoreen exponent | 0 |
| Edge step | Fitted, unless a positive measured override is entered |

Ranges adapt to available data; short spectra can need manual choices. Desktop
Auto applies these choices before calling the core library, whose unset fields
can resolve differently. Retain resolved settings when comparing interfaces or
other software.

## Background

AUTOBK estimates the smooth background and produces $\chi(k)$. Begin with
$R_\mathrm{bkg}=1$ Å, the fixed endpoint penalty 0.001 and the direct solver.
The background objective uses $k$ weight 1; display/forward-transform weighting is
separate. **Link to FFT** is an explicit choice to relate weights.

The default background grid spacing is 0.05 Å⁻¹ and its FFT length is 2048.
The background uses a Hanning window with $dk=0.1$ Å⁻¹; this is separate from the
forward-transform window. The fixed endpoint penalty is a rexafs choice described
in the [AUTOBK objective](/docs/science/autobk/). Older projects preserve their
saved clamp policy, so check it before comparing their results with a new project.

**Background options → Load standard…** accepts an optional χ(k) reference.
Supply exactly two numeric columns: increasing, nonnegative k in Å⁻¹ and matching
finite, unweighted, dimensionless χ(k). Separate columns with whitespace or
commas; begin comments with `#` or `*`. At least two points are required, and
extra columns or malformed rows produce an error. This input is a reference for
the background objective, not a new μ(E) spectrum or an already-windowed Fourier
curve. Its arrays are stored inside the project. Leave it unset for the normal
starting workflow; use **Clear standard** to remove it. See the [standard
reader](https://github.com/Ameyanagi/rexafs/blob/v0.2.4/crates/rexafs-gui/src/params.rs#L983)
and [AUTOBK objective](/docs/science/autobk/).


[![Full background-removal view showing weighted Cu EXAFS and AUTOBK settings](/screenshots/background.jpg)](/screenshots/background.jpg)

*Background settings control the spline fit; display weighting controls the plotted oscillations. rexafs 0.2.4 on macOS.*


## Forward transform

The recommended starting window is 2–15 Å⁻¹ with $k$ weight 2, taper parameter
$dk=1$ Å⁻¹ and a Kaiser–Bessel window. Shorten the range for a noisier or narrower
measurement. View **k**, **R**, or **k + R**, and enable real/imaginary components
where available. Keep enough samples in the fitted window for a meaningful fit.

The forward transform uses 2048 points and infers its k step from the background
grid unless you override it. Its explicit amplitude multiplier is
$\delta k/\sqrt{\pi}$ after an unnormalized forward FFT; increasing padding
refines the displayed R grid without increasing experimental resolution. For
Kaiser–Bessel, $dk$ also affects the window shape, so it is not only a taper width.
The [processing equations](/docs/science/processing/) define the units and scaling.

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
