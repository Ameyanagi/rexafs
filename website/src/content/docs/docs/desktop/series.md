---
title: "Series and trends"
description: "Browse scan frames and distinguish sampled overviews from complete calculations."
audience: user
---

Import a folder of related spectra, then open **Series → Select scan**.
The heatmap shows the scan; adjacent plots show the selected frame and trend.
Choose **norm μ(E)**, weighted **χ(k)** or **|χ(R)|**.

## Frames and overview sampling

Scans follow filename order within each directory. Frame numbers are indices,
not elapsed seconds. Use zero-padded filenames for acquisition order, retain
timestamps separately, and check the scan and file names before interpreting a
time series.

The overview samples at most **192 active frames** across the scan, each using
its effective settings, including per-file overrides. The heatmap interpolates
linearly onto 256 energy, k or R display points, filling zeros outside each
frame's range. An empty end region need not be a measured zero, and sampling can
miss short-lived features. Failed sampled frames appear in **Problems**.

Display ranges are 0–15 Å⁻¹ for k and 0–6 Å for R. The energy axis runs from
$E_0-40$ eV to the smaller of the measured endpoint and $E_0+200$ eV, using the
first successfully processed frame. These ranges do not set normalization or
transform limits. Use consistent processing and k weights to compare amplitudes;
the header shows the project k weight, which per-file overrides can differ from.

Select a frame with the heatmap or scrubber. With the plot focused, Left/Right
moves one frame, Shift+Left/Right moves approximately one percent of the scan, and
Home/End selects the first/last position. The plot can show the nearest sampled
frame while the selected frame loads; wait for processing before interpreting it.
Use **Refresh overview** after changes.

These display choices are rexafs-specific, implemented in the [overview and
frame loading](https://github.com/Ameyanagi/rexafs/blob/v0.2.4/crates/rexafs-gui/src/app.rs#L4853)
and [Series controls](https://github.com/Ameyanagi/rexafs/blob/v0.2.4/crates/rexafs-gui/src/app/shell/series.rs).

## What the trends mean

The **$E_0$ trend** reports the absolute estimated or requested edge energy in eV,
despite its “E₀ shift” label. It does not subtract a reference or calibrate the
beamline. Noise and normalization choices can move automatic estimates, as can
sample changes.

The **white-line trend** is the maximum processed absorption from $E_0$ to
$E_0+30$ eV, inclusive. In 0.2.4, this trend and the energy overview use
**flattened absorption when available**, falling back to normalized absorption;
the UI labels this “norm μ(E).” The white-line height is dimensionless after
edge-step normalization. It is a local maximum, not an integrated peak area,
concentration or oxidation-state calibration. A glitch can dominate it. These
definitions come from [frame_sample](https://github.com/Ameyanagi/rexafs/blob/v0.2.4/crates/rexafs-gui/src/app.rs#L677)
and [trend_snapshot](https://github.com/Ameyanagi/rexafs/blob/v0.2.4/crates/rexafs-gui/src/app.rs#L7844).
See [processing theory](/docs/science/processing/) for the meaning of normalized
and flattened absorption.

The $E_0$ and white-line trends cover sampled overview frames. Independent fits
can supply fitted-parameter trends; linear-combination fitting (LCF) supplies
standard weights. Choose **Sampled frames / All frames** for those calculations.
**All frames** calculates every surviving frame without increasing the heatmap's
192-frame limit. Missing or failed results appear as gaps, not zero-valued
parameters.

For LCF, mark at least two standards and load results for their current processing
settings. Review the LCF range and constraints in the data tools; Series uses
those settings. Canceling keeps completed rows, so check completion status before
treating an exported trend as the whole scan.

Inspect a subset before a complete calculation, then review failures and
per-frame uncertainties. See [multiple spectra and batches](/docs/desktop/multiple-spectra/)
for fit setup and [LCF theory](/docs/science/analysis/) for standard weights.
Processing choices and model inadequacy can create apparent trends; smoothness
alone does not validate a model.
