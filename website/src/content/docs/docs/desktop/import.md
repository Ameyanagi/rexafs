---
title: "Import and groups"
description: "Map detector signals, confirm energy units and organize spectra."
audience: user
---

## Import and review

Choose **Import…** or drop a measurement file into the workspace. The plotted
preview handles beamline text, numeric tables, Athena projects, Larix sessions,
XTUNES saves and supported HDF5 arrays. Choose **Scan** for a file with several
records. File content determines the reader; an extension alone does not
establish the units or detector arithmetic.

1. Inspect the plotted signal and **Energy axis**. The reader preserves declared
   eV/keV, relative-energy and monochromator-angle calibration. Confirm an
   assumed unit or provide the missing crystal-plane spacing for an angular axis.
2. Check the signals to import: stored **μ**, **Transmission**, **Fluorescence**
   or **Reference**, when the source provides those channels. Each row shows its
   source-column formula. **Preview** changes the displayed curve without
   changing the checked outputs.
3. Use **Columns** to correct detector roles or **Custom mapping…** for a manual
   output. **Source details** reveals original headers and rows. Read any parser
   warnings before accepting the data.
4. Choose **Import N spectra**. Every checked conversion must succeed before
   groups are added. Each signal becomes a separate spectrum, the Data view
   starts on raw μ(E), and one undo removes the entire import.

![rexafs 0.2.6 import preview with transmission, fluorescence and reference choices](/screenshots/0.2.6/import-preview.png)

Captured from the signed 0.2.6 Mac app. The QAS Mo foil measurement is from
[Ryuichi Shimogawa and contributors, xasref](https://github.com/Ameyanagi/xasref/blob/74d1e795855055c7731da406b276bd50b27aafff/foil_QAS_sample_position/Mo%20foil%200001-r0003.dat),
distributed under its MIT repository notice.

For transmission, rexafs computes `ln(I0 / It)` from incident and transmitted
intensities in matching units. Reference uses `ln(It / Ir)`, with `Ir` measured
after the reference foil. Fluorescence uses the selected detector sum divided by
`I0`. Choose stored μ when absorption is already calculated. The reader does
not repeat dark-current, dead-time or other detector corrections.

Angles are Bragg angles θ, not full scattering angles 2θ. Conversion uses
`E = hc / (2 d sin θ)`, where energy E is in eV, reflecting-plane spacing d is
in Å and `hc = 12398.419843320026 eV Å`. Use the spacing for the actual
monochromator reflection. This applies first-order Bragg diffraction; it does
not fit an instrument offset. The [reader guide](/docs/reference/stable/measurement-reading/#units-and-signals)
explains units, intensity requirements, defaults and scientific references.

For HDF5, **Scan → Select datasets…** combines explicitly chosen, equal-length
real vectors. Images need a detector reduction; warnings identify partial group
recovery. A readable container does not mean every channel is available. The
[format guide](/docs/reference/stable/measurement-reading/) describes Athena,
Larix, XTUNES, 9809/KEK `.qd` and other supported formats and their limits.

Folders and multiple-file selections retain the batch workflow below. Import
containers individually. Source files are unchanged; saved desktop projects
retain original measurement bytes and accepted signal mappings.

**Reader corrections in development:** duplicate detector-role headings require
explicit column selection, and each stored absorption column appears as its own
signal choice. Malformed leading numeric rows produce an error even when comments
separate them from later data. These corrections are not part of the 0.2.6 download.

## Recipes and groups

Batch imports use a column review and saved recipes. Check the plotted edge,
units and parser diagnostics; **Revalidate** checks an edited mapping and
**Review later** keeps the source pending.

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
