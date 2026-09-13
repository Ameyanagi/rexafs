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

For an angular axis, enter the Bragg angle $\theta$, rather than the full
scattering angle $2\theta$. rexafs uses first-order diffraction:

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

After changing a mapping, **Revalidate** it and inspect the preview. **Review later**
keeps a source pending. Failed or ambiguous imports should not be mistaken for
successfully processed spectra. [XDI](/docs/desktop/xdi/) supplies a structured
alternative with named columns and metadata.

Inspect the point count and parser warnings as well as the curve. For ordinary
text files, the reader can skip malformed or short rows and truncate wider rows
to the established column width; the import diagnostics report those decisions
and example source lines. Points whose converted energy or calculated μ is
non-finite are excluded. The remaining energy and μ arrays are sorted together
by increasing energy. Sorting does not average duplicate energies. Fewer than
two finite points is an import error, and later processing can reject an
inadequate axis or range. XDI rejects malformed numeric rows before this signal
conversion. A successfully parsed file still needs a physically sensible mapping:
the finite-value check does not establish that detector intensities are valid.


[![Full import review with the selected Cu columns and confirmed eV units](/screenshots/import-mapping.jpg)](/screenshots/import-mapping.jpg)

*This measured example already contains μ(E); a second logarithm would be incorrect. Full application window, rexafs 0.2.4 on macOS. Select the image to view its full resolution.*


## Recipes and groups

The **Recipe** control records a reusable import choice for matching layouts.
A saved project's accepted mappings remain authoritative on reopen; a later
computer recipe is for new imports. Original source files are unchanged.

Recipes match the complete column layout, including names/order, units, parser
dialect and conversion metadata, within their saved folder or instrument scope.
Matching project recipes take priority over computer recipes. Conflicting
interpretations need review. Unnamed columns require one representative
confirmation in each new batch, even when a previous recipe supplies a suggested
mapping. A familiar layout with changed units also requires a new review.

Editing a named recipe creates a new revision; existing imported groups keep
their original interpretation. Disabling reuse prevents automatic selection for
future imports without deleting earlier groups. These rules are defined in the
[recipe dispatcher](https://github.com/Ameyanagi/rexafs/blob/v0.2.4/crates/rexafs-gui/src/import_recipes.rs#L317).

Select a group to inspect its spectrum. Mark groups for comparisons and multi-file
operations. Names, labels, locks, assignments and per-spectrum overrides help keep
large collections organized. A comparison plot does not merge observations.

The **current** group supplies the inspector and editable settings. **Marks**
select the recipients of comparison and bulk actions; they do not change which
group is current. Filtering or collapsing rows can hide marked groups without
unmarking them; inspect the marked/hidden count before a bulk operation. Large
comparison selections are sampled to at most 12 plotted traces. Use
[Series](/docs/desktop/series/) to browse a scan and inspect its individual frames.

Right-click a group to rename it, duplicate it, add an available channel, or
**Lock processing**. A duplicate has independent processing settings but can
still read the same original file. A processing lock prevents processing edits;
it is not a copy of the original data. **Remove group** removes it from the
project, without deleting the original file, and can be undone in the current
session. Derived results retain their saved arrays when an input group is removed;
their input notice records the missing source relationship.

Use **Re-map columns…** in the Data parameters to review an existing mapping.
Inspect derived spectra after changing source interpretation or processing inputs.

Results such as normalized differences are labelled by their stored quantity.
They remain available for plotting and export, but cannot be passed through
normalization/AUTOBK as though they were raw absorption. Older projects with
unlabelled derived arrays ask you to confirm their quantity before processing.
Confirm what the stored values actually represent; choosing a label does not
convert the arrays.

## Data treatment

Data tools include alignment, energy calibration, deglitching, truncation,
rebinning, smoothing, merging and difference spectra. Each changes a different
part of the analysis: use the [data-treatment guide](/docs/science/analysis/)
before applying a transformation across a collection.
