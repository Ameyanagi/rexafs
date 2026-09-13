---
title: "Import and groups"
description: "Map detector signals, confirm energy units and organize spectra."
audience: user
---

## Import and review

Choose **Import** or drag files/folders into the workspace. Check the energy axis,
columns and plotted edge before accepting the suggested mapping. Ambiguous files
wait for review.

Select the main signal: **Transmission**, **Fluorescence**, **Reference** or
**μ column**. You can also import additional available channels. Confirm whether
the axis is eV, keV or monochromator angle. An angular axis needs the appropriate
monochromator spacing; do not relabel degrees as eV.

Enter angular axes as the Bragg angle $\theta$, not the full scattering angle
$2\theta$. rexafs uses first-order diffraction:

$$E=\frac{hc}{2d\sin\theta}.$$

Here $E$ is energy in eV, $d$ is the reflecting-plane spacing in Å, and
$hc=12398.419843320026$ eV Å is the conversion factor used in the code. The angle
must be greater than zero and at most 90° (or $\pi/2$ radians); $d$ must be finite
and positive. Use the spacing for the actual monochromator reflection. This
conversion does not fit a motor offset or calibrate the instrument. See
[Bragg's law](https://dictionary.iucr.org/Bragg%27s_law) and the [desktop axis
converter](https://github.com/Ameyanagi/rexafs/blob/v0.2.4/crates/rexafs-gui/src/import_mapping.rs#L35).

For transmission, absorption is $\mu(E)=\ln[I_0(E)/I_t(E)]$. Here $I_0$ and $I_t$
are incident and transmitted intensities in matching units; both must be positive.
For fluorescence, a common input is $I_f/I_0$. If the source already contains
absorption, choose its μ column. These conventions and their assumptions are
explained in [processing theory](/docs/science/processing/).

In fluorescence mode, rexafs sums the selected region-of-interest (ROI) columns
before dividing by $I_0$. Each selected detector column is counted once. Select
only compatible detector signals, with any required detector corrections already
applied. Reference mode uses a precomputed reference μ column when assigned;
otherwise it calculates $\ln(I_t/I_r)$, where $I_r$ is the intensity after the
reference foil. These are the actual [desktop import
operations](https://github.com/Ameyanagi/rexafs/blob/v0.2.4/crates/rexafs-gui/src/params.rs#L913).

After changing a mapping, **Revalidate** and inspect the preview. **Review later**
keeps the source pending. [XDI](/docs/desktop/xdi/) provides named columns and
metadata for import.

Inspect the point count and parser warnings. The ordinary text reader can skip
malformed or short rows and truncate wider rows to the established column width;
diagnostics report these changes with example source lines. Converted points with
non-finite energy or μ are excluded. The remaining energy/μ pairs are sorted by
energy without averaging duplicates. Fewer than two finite points is an import
error; later processing can reject an inadequate axis or range. XDI rejects
malformed numeric rows before signal conversion. Finite values alone do not
establish physically valid detector intensities or column assignments.


[![Full import review with the selected Cu columns and confirmed eV units](/screenshots/import-mapping.jpg)](/screenshots/import-mapping.jpg)

*This measured example already contains μ(E); do not apply a second logarithm. rexafs 0.2.4 on macOS.*


## Recipes and groups

**Recipe** saves import choices for matching layouts. Reopening a project keeps
its accepted mappings; later computer recipes apply to new imports. Source files
are unchanged.

Recipes match column names/order, units, parser dialect and conversion metadata
within their saved folder or instrument scope.
Matching project recipes take priority over computer recipes. Conflicting
interpretations need review. Unnamed columns require one representative
confirmation in each new batch, even when a previous recipe supplies a suggested
mapping. A familiar layout with changed units also requires a new review.

Editing a named recipe creates a revision; existing groups keep their original
interpretation. Disabling reuse stops automatic selection for future imports
without deleting earlier groups. See the
[recipe dispatcher](https://github.com/Ameyanagi/rexafs/blob/v0.2.4/crates/rexafs-gui/src/import_recipes.rs#L317).

Select a group to make it **current** and edit its settings in the inspector.
**Marks** select groups for comparisons and bulk actions without changing the
current group. Filtering or collapsing rows can hide marked groups without
unmarking them; check the marked/hidden count before bulk actions. Comparison
plots show at most 12 sampled traces and do not merge observations. Use
[Series](/docs/desktop/series/) to inspect individual scan frames.

Right-click a group to rename it, duplicate it, add an available channel, or
**Lock processing**. A duplicate has independent settings but can read the same
source file. A processing lock prevents processing edits; it does not copy source
data. **Remove group** removes the group without deleting its source file and can
be undone in the current session. Derived results keep their saved arrays when an
input group is removed; their input notice records the missing source.

Use **Re-map columns…** in the Data parameters to review an existing mapping.
Inspect derived spectra after changing source interpretation or processing inputs.

Derived results, such as normalized differences, are labelled by their stored
quantity. They can be plotted and exported but cannot enter normalization/AUTOBK
as raw absorption. Older projects ask you to confirm unlabelled arrays' quantity
before processing. Choose what the values represent; a label does not convert
the arrays.

## Data treatment

See [data treatment](/docs/science/analysis/) for alignment, energy calibration,
deglitching, truncation, rebinning, smoothing, merging and difference spectra
before applying these tools across a collection.
