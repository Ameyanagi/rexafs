---
title: "Import and groups"
description: "Map detector signals, confirm energy units and organize spectra."
audience: user
---

## Import and review

Choose **Import** or drag files/folders into the workspace. Review the energy axis,
column assignment and plotted edge before accepting a mapping. A detected layout
is a suggestion; an ambiguous file waits for your review.

Select the main signal: **Transmission**, **Fluorescence**, **Reference** or
**μ column**. You can also import additional available channels. Confirm whether
the axis is eV, keV or monochromator angle. An angular axis needs the appropriate
monochromator spacing; do not relabel degrees as eV.

For transmission, absorption is $\mu(E)=\ln[I_0(E)/I_t(E)]$. Here $I_0$ and $I_t$
are incident and transmitted intensities in matching units; both must be positive.
For fluorescence, a common input is $I_f/I_0$. If the source already contains
absorption, choose its μ column. These conventions and their assumptions are
explained in [processing theory](/docs/science/processing/).

After changing a mapping, **Revalidate** it and inspect the preview. **Review later**
keeps a source pending. Failed or ambiguous imports should not be mistaken for
successfully processed spectra. [XDI](/docs/desktop/xdi/) supplies a structured
alternative with named columns and metadata.


[![Full import review with the selected Cu columns and confirmed eV units](/screenshots/import-mapping.jpg)](/screenshots/import-mapping.jpg)

*This measured example already contains μ(E); a second logarithm would be incorrect. Full application window, rexafs 0.2.4 on macOS. Select the image to view its full resolution.*


## Recipes and groups

The **Recipe** control records a reusable import choice for matching layouts.
A saved project's accepted mappings remain authoritative on reopen; a later
computer recipe is for new imports. Original source files are unchanged.

Select a group to inspect its spectrum. Mark groups for comparisons and multi-file
operations. Names, labels, locks, assignments and per-spectrum overrides help keep
large collections organized. A comparison plot does not merge observations.

Use **Re-map columns…** in the Data parameters to review an existing mapping.
Inspect derived spectra after changing source interpretation or processing inputs.

## Data treatment

Data tools include alignment, energy calibration, deglitching, truncation,
rebinning, smoothing, merging and difference spectra. Each changes a different
part of the analysis: use the [data-treatment guide](/docs/science/analysis/)
before applying a transformation across a collection.
