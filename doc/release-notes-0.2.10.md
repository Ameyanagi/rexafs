# rexafs 0.2.10

Release candidate prepared on 18 September 2026. Publication and artifact checks
are tracked in the [qualification record](validation/2026-09-18-release-0.2.10/review.md).
The release contains the work merged into `dev` through PR 89.

## RMC refinement

**Fit → Fit mode: RMC** refines atomic coordinates from a processed Spectrum and
an explicit periodic structure. Guided setup includes a live supercell preview,
constraints, calibration and fit ranges. Results show experimental, initial and
best spectra, the best structure, statistics and recent residual trends. Pause,
stop-and-save, project persistence and checkpoint recovery support long runs.
Resume retains the saved preprocessing, inputs, settings and random state.

The desktop fits the normalized R-space real-plus-imaginary residual through the
existing native fitting transform. It respects saved AUTOBK Rbkg and starts R
minimum 0.15 Å above it. Background parameters remain fixed during coordinate
moves. A completed attempt budget does not establish convergence: the residual
trend is reported separately. See the [desktop workflow](rmc-desktop-workflow.md).

Exact cached ReFEFF is the default, using ReFEFF 0.4.0 and fixed reference
potentials. The Rust API also provides hybrid evolutionary/RMC search, weighted
structures, per-dataset settings, constraints, checkpointed Spectrum provenance,
structural summaries and convergence diagnostics. Adaptive scattering remains
experimental and explicitly opt-in; the Cu₂O comparison did not establish a
sustained speedup. The desktop currently exposes one spectrum and one structure;
EA and joint/weighted-structure controls remain future desktop work.

## Analysis and processing

- Native composite XANES peak/step/baseline fits support constraints, retained
  results, component curves and conditional covariance diagnostics. Desktop,
  Python and TypeScript interfaces use the same calculation. See
  [XANES peak fitting](xanes-peak-fitting.md).
- Full Chantler MBACK normalization uses versioned offline atomic references,
  retains its fitted curve and supports comparison with pre/post-edge processing.
  See [MBACK normalization](mback-normalization.md).
- **Transform → Wavelet** provides retained Cauchy maps, linked k/R plots, display
  components and region measurements. Python and TypeScript expose the native
  map. The RMC-compatible Morlet/STFT API remains a distinct transform convention;
  the earlier Morlet desktop preview is superseded. See [Wavelet analysis](wavelet-analysis.md).
- Fluorescence correction creates an independent retained group with explicit
  composition, geometry and emission settings. This homogeneous, optically thick
  model is limited to XANES; corrected groups cannot enter EXAFS processing or RMC.
  See [fluorescence correction](fluorescence-correction.md).
- Scalar measurements, saved recipes and full-frame Series runs retain processing
  state, source identities, results and recovery information. Series browsing,
  range controls and reference heatmaps are improved. Import batches reuse reviewed
  mappings; pending imports can be skipped with undo.

## Live acquisition and exports

Experimental Live acquisition watches a chosen folder using an explicit
completion policy, retains source snapshots and revisions, and updates Series
trends. It can apply a saved XANES peak model. Pause, retry and saved recovery
preserve committed results. Quiet-file completion is inferred from observations;
a writer pause can resemble completion. Network-share and physical Windows/Linux
acquisition workflows are not qualified by the current local review. See
[Live acquisition](live-acquisition.md).

Processing/comparison plots, analysis results, Series and Wavelet views export
CSV, PNG or SVG as appropriate. Retained Wavelet JSON also includes input arrays
and provenance. Plot zoom changes the view, not the recorded fitting interval.

## Compatibility and qualification

Project format 1 is retained. A new linked/embedded fixture pair is written and
reopened through this release's writer; historical fixture bytes remain unchanged.
The coordinated version covers Rust, Python, npm/WebAssembly and desktop builds.
RMC's new public workflow is a Rust API and desktop feature; Python/TypeScript
RMC bindings are not claimed by this release.

Publication requires the complete GitHub build of the immutable tag, registry
checksums and signed/notarized Mac downloads. Windows and Linux remain desktop
previews; automated checks do not establish physical interactive qualification.
The reviewed Cu₂O RMC run is saved at 5,396 attempts and still changing. Its score
is not evidence of a converged or unique structure.
