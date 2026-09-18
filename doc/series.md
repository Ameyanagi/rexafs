# Series and trends

Open **Series** after importing a folder of related spectra, then choose **Select
scan**. The heatmap shows changes across the scan; the adjacent plots show the
selected frame and a chosen trend. Switch between **norm μ(E)**, weighted
**χ(k)** and **|χ(R)|** to examine different parts of the measurement.

## Frames and overview sampling

Folder scans are ordered by filename, within each directory. Frame number is an
index in that order, not a measured time in seconds. Use consistent, zero-padded
filenames when their ordering represents acquisition order, and retain the actual
timestamps separately. Check the selected scan and file names before interpreting
a progression as a physical time series.

The overview samples at most **192 active frames**, spread through the scan.
Each sampled frame uses its effective processing settings, including per-file
overrides. The heatmap uses 256 display points along its energy, k or R axis.
Linear interpolation supplies the common display grid, with zeros outside an
individual frame's available range. Consequently, an empty end region need not
represent a measured zero, and a short-lived feature can be absent from the
sampled overview. Failed sampled frames are reported in **Problems**.

The overview k axis spans 0–15 Å⁻¹ and the R axis spans 0–6 Å. The energy axis
starts 40 eV below the first successfully processed frame's $E_0$ and ends at
the smaller of that frame's measured endpoint and $E_0+200$ eV. These are display
ranges. They do not set each spectrum's normalization or transform limits.
For amplitude comparisons, use consistent processing and k weights across
frames. The Series header reports the project k weight, while a frame can have
a different per-file override.

Click a heatmap row or use the scrubber to select a frame. With the Series plot
focused, Left/Right moves one frame, Shift+Left/Right moves approximately one
percent of the scan, and Home/End selects the first/last position. The frame plot
can initially show the nearest sampled frame while the selected frame loads;
wait for processing before interpreting its details. **Refresh overview**
rebuilds the scan display after changes.

These display choices are rexafs-specific, implemented in the [overview and
frame loading](https://github.com/Ameyanagi/rexafs/blob/v0.2.4/crates/rexafs-gui/src/app.rs#L4853)
and [Series controls](https://github.com/Ameyanagi/rexafs/blob/v0.2.4/crates/rexafs-gui/src/app/shell/series.rs).

## What the trends mean

The initial $E_0$ trend reports the **absolute estimated or requested edge energy in
eV**, even though its selector says “E₀ shift.” It does not subtract a reference
energy or calibrate the beamline. Automatic edge estimates can move because of
noise or normalization choices as well as sample changes.

The white-line trend is the maximum processed absorption between $E_0$ and
$E_0+30$ eV, inclusive. In 0.2.4, both this trend and the energy overview use
**flattened absorption when available**, falling back to normalized absorption;
the UI labels this “norm μ(E).” The white-line height is dimensionless after
edge-step normalization. It is a local maximum, not an integrated peak area,
concentration or oxidation-state calibration. A glitch can dominate it. These
definitions come from [frame_sample](https://github.com/Ameyanagi/rexafs/blob/v0.2.4/crates/rexafs-gui/src/app.rs#L677)
and [trend_snapshot](https://github.com/Ameyanagi/rexafs/blob/v0.2.4/crates/rexafs-gui/src/app.rs#L7844).
See [processing theory](processing-theory.md) for the meaning of normalized
and flattened absorption.

The $E_0$ and white-line trends describe the sampled overview frames. Completed
independent fits can supply fitted-parameter trends, while linear-combination
fitting (LCF) can supply the weights of selected standards. Those calculations
use the **Sampled frames / All frames** scope control. **All frames** processes
each surviving frame for the calculation; it does not increase the heatmap's
192-frame sampling limit. Missing or failed results remain gaps, rather than
zero-valued fitted parameters.

For an LCF trend, mark at least two standard spectra and load their processed
curves before starting. Only marked standards with results for their current
settings are available to the calculation. Review the LCF range and constraints
in the data tools, because the Series calculation uses those settings. Canceling
a running calculation keeps the rows already completed; inspect its completion
status before treating an exported trend as the whole scan.

Inspect a representative subset before running a complete calculation, then
review failures and the reported per-frame uncertainties. Use
[multiple spectra and batches](joint-fitting.md) for fit setup and
[LCF theory](https://rexafs.com/docs/science/analysis/) to interpret standard weights. Processing
choices and model inadequacy can create apparent trends; a smooth trend alone
does not validate a model.

## Live acquisition (unreleased)

**Live…** watches completed files and applies a frozen processing and trend recipe.
See [Live acquisition](live-acquisition.md) for the configurable quiet-file check,
include-existing preview, review, pause/resume and recovery workflow.
