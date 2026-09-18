---
title: "Series and trends"
description: "Browse scan frames and distinguish sampled overviews from complete calculations."
audience: user
---

This guide describes **rexafs 0.2.10**. Captions identify older screenshots.

Version **0.2.10** keeps the heatmap and frame browser as
the main Series view, including named series of project-stored spectra.
**Add trend… → choose metric and range → Calculate all N frames** adds a saved
trend. New desktop trends use Flat, 0–30 eV from each frame's E₀; the core API
still defaults to Norm. Drag the shaded preview’s boundaries to update the
range fields and value, or type the bounds directly. **Left/Right** browses
preview spectra; arrows in a focused field still edit text. K-space spectrum plots
center zero with symmetric vertical limits; glitches remain visible.
**Difference** subtracts a fixed reference frame from the heatmap and cursor
spectrum. **Reference… / Ref: N ▾** changes it. **Colors ▾** opens a popup menu
to select a palette or reverse it, keeping the plots and controls in place.
Auto uses blue–red with a zero-centered scale for differences. These temporary
view controls leave original groups and calculations unchanged.
**Results…** contains saved runs, per-frame failures and CSV/JSON export.
**Advanced** holds ordering, coordinates, presets, recipes and recovery.

The source selector lists named series and folder scans. **Use loaded groups**
starts from spectra already in a project. The overview samples at most 192
available frames, while custom trends calculate every member. Saved trends retain
their original settings; changing the overview representation does not recalculate
them. The older batch-fit and LCF controls still require a folder scan.
See the [full-frame workflow](https://github.com/Ameyanagi/rexafs/blob/v0.2.10/doc/full-frame-measurements.md)
for details.

[![Development Series overview with a saved trend from 513 synthetic spectra](/screenshots/next/series-trend-overview.jpg)](/screenshots/next/series-trend-overview.jpg)

Development screenshot captured through computer use on 2026-09-16, using the
project's synthetic series generator. It contains no experimental data.

[![Development difference heatmap with the color popup open](/screenshots/next/series-difference-colors.jpg)](/screenshots/next/series-difference-colors.jpg)

Captured through computer use on 2026-09-17 with the same synthetic source.
Frame 258 is shown relative to frame 1. Opening the palette menu leaves the
plot sizes and controls in place; the saved trend is unchanged.
The overview and older batch controls described below remain available. The LCF screenshot retains its 0.2.9 provenance.

Import a folder of related spectra, then open **Series → Select scan**.
The heatmap shows the scan; adjacent plots show the selected frame and trend.
Choose **flat μ(E)** (the initial selection), **norm μ(E)**, weighted **χ(k)**
or **|χ(R)|**. The energy plot uses the selected representation.

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
frame loading](https://github.com/Ameyanagi/rexafs/blob/v0.2.9/crates/rexafs-gui/src/app.rs)
and [Series controls](https://github.com/Ameyanagi/rexafs/blob/v0.2.9/crates/rexafs-gui/src/app/shell/series.rs).

## What the trends mean

The **$E_0$ trend** reports the absolute estimated or requested edge energy in eV,
despite its “E₀ shift” label. It does not subtract a reference or calibrate the
beamline. Noise and normalization choices can move automatic estimates, as can
sample changes.

The **white-line trend** is the maximum processed absorption from $E_0$ to
$E_0+30$ eV, inclusive. This trend uses **flattened absorption when available**,
falling back to normalized absorption, independently of the energy-plot selector.
Its historical UI label is “white line (norm. μ).” The height is dimensionless after
edge-step normalization. It is a local maximum, not an integrated peak area,
concentration or oxidation-state calibration. A glitch can dominate it. These
definitions come from `frame_sample` and `trend_snapshot` in the
[Series implementation](https://github.com/Ameyanagi/rexafs/blob/v0.2.9/crates/rexafs-gui/src/app.rs).
See [processing theory](/docs/science/processing/) for the meaning of normalized
and flattened absorption.

The $E_0$ and white-line trends cover sampled overview frames. Independent fits
can supply fitted-parameter trends; linear-combination fitting (LCF) supplies
standard weights. Choose **Sampled frames / All frames** for those calculations.
**All frames** calculates every surviving frame without increasing the heatmap's
192-frame limit. Missing or failed results appear as gaps, not zero-valued
parameters.

For LCF, mark at least two standards and review their processing settings, the
LCF range and its constraints in the data tools; Series uses those settings.
The core prepares missing selected arrays on copies. Canceling keeps completed
rows, so check completion status and per-frame errors before treating an exported
trend as the whole scan. Project saves retain the batch coefficients and settings;
publication exports include `data/lcf-series.csv` and JSON.

[![All 100 Cu-mixture LCF results and the selected frame 51 in rexafs 0.2.9](/screenshots/0.2.9/series-lcf.jpg)](/screenshots/0.2.9/series-lcf.jpg)

Captured through computer use from the signed 0.2.9 Mac app after all 100
synthetic mixtures completed LCF against the Cu foil, Cu₂O and CuO references.
The plot uses flattened absorption. Selecting the heatmap and pressing Right
changed the selected frame, spectrum and trend cursor together; the capture
shows frame 51. See [data and capture provenance](/licenses/#documentation-screenshots).

Inspect a subset before a complete calculation, then review failures and
per-frame uncertainties. See [multiple spectra and batches](/docs/desktop/multiple-spectra/)
for fit setup and [LCF theory](/docs/science/analysis/) for standard weights.
Processing choices and model inadequacy can create apparent trends; smoothness
alone does not validate a model.


## Experimental Live acquisition

In 0.2.10, **Series → Live…** watches a selected folder and retains completed
spectra and trends. Preview the filename filter, detector mapping and frozen
processing or saved recipe before starting. The default quiet-file policy uses
three matching observations one second apart. A writer pause can look complete;
choose a policy suitable for the acquisition and inspect retained revisions.
**Pause**, retry and **Open paused** preserve committed results and permit review
before continuation. A saved XANES peak model can run as frames arrive.

Live acquisition remains experimental. Physical Windows/Linux acquisition and
network-share behavior are not qualified by the current local review. See the
[Live guide](https://github.com/Ameyanagi/rexafs/blob/v0.2.10/doc/live-acquisition.md)
for completion policies, recovery and source-revision handling.
