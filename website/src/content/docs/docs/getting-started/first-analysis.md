---
title: "Your first analysis"
description: "Process the bundled Cu spectrum and save a reproducible project."
audience: user
---

Use the bundled **Cu foil at 150 K**, measured at NSLS X-11A in September 1992.
This is a teaching example; use a suitable reference for experimental calibration.
See [data provenance](/licenses/). Select a screenshot for the full application
window.

## 1. Open and verify the data

Choose **Open Cu example** in the empty workspace. In the import review, choose
**μ column**, energy column 1 and absorption column 2. Select **eV** and confirm
the units when asked. Choose **Revalidate** if the mapping has changed, then
**Import 1 files → 1 groups**. This file already contains absorption: do not apply
a second logarithm.

[![Full import dialog showing the Cu absorption column, eV units and validated preview](/screenshots/import-mapping.jpg)](/screenshots/import-mapping.jpg)

*Check the Cu edge and column mapping before importing. rexafs 0.2.4, macOS.*

## 2. Inspect normalization

Open **Normalize**. Inspect the estimated $E_0$ and the blue pre-edge and yellow
post-edge fit windows. A blank numeric field means **Auto**; its placeholder shows
the current resolved value. Select **norm** for edge-step-normalized absorption,
or **flat** to remove the fitted post-edge trend from that display.

Start with the defaults. When processing your own sample, choose baseline windows
that exclude the edge structure and remain inside the measured range. Automatic
$E_0$ is an estimate, not an energy calibration.

[![Full normalization window showing the Cu edge and baseline fitting regions](/screenshots/normalize.jpg)](/screenshots/normalize.jpg)

*Normalization divides pre-edge-subtracted absorption by the fitted edge step. rexafs 0.2.4, macOS.*

## 3. Extract EXAFS and transform it

Open **Background**, select **χ(k)** and inspect the oscillations. The recommended
AUTOBK cutoff is $R_\mathrm{bkg}=1$ Å; the background fit uses $k$ weight 1 by default.
A higher cutoff can remove structural signal as well as background.

Open **Transform**, then **k + R**. The default forward window is 2–15 Å⁻¹,
with weight 2 and a Kaiser–Bessel window. Choose a useful upper limit for the
signal-to-noise range of your actual spectrum. The lower plot displays the
magnitude of the Fourier transform. Its peak locations are not phase corrected.

[![Full transform window showing weighted EXAFS and Fourier magnitude together](/screenshots/transform.jpg)](/screenshots/transform.jpg)

*The Cu spectrum in k and R, with forward-transform controls. rexafs 0.2.4, macOS.*

## 4. Save and export

Choose **Save project**. **Link source files** keeps references to your inputs;
**Include source files** makes a portable project with embedded original bytes.
Choose a new `.rxs` filename. See [projects and recovery](/docs/desktop/projects/).

Open **Publish**, choose a figure and PNG or SVG, then **Export…**. Choose
**Analysis folder** as the format to export figures, data, captions, methods and
references together.
Review the figure labels and experimental details before sharing them.

Next, [fit a first coordination shell](/docs/desktop/fitting/) or read
[what each calculation means](/docs/science/processing/).
