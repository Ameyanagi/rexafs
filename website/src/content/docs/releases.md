---
title: "Release history"
description: "Published rexafs versions, what changed for users, and migration guidance."
audience: user
---

Published changes and migration notes are listed below. Back up projects before
updating. Source-checkout APIs are documented in the [Next API
reference](/docs/reference/).

## Stable 0.2.14

[Download 0.2.14](/download/) or read the
[published release](https://github.com/Ameyanagi/rexafs/releases/tag/v0.2.14).

- A compact [Live monitor](/docs/desktop/series/) keeps acquisition visible while
  other views remain usable. Capture processing from a chosen reference, import
  transmission, fluorescence and reference together, and choose cumulative or
  fixed-size averages for each signal.
- Automatic EXAFS fits and [Series parameter trends](/docs/desktop/series/)
  include error bars and expressions such as `reff + dr_1`. Fit results remain
  available alongside the measured spectra.
- [Plot axis ranges](/docs/desktop/processing/) accept separate automatic or
  explicit bounds, including zero for a minimum while the maximum stays automatic.
- Optional [Assistant connected-app access and file import](/docs/desktop/assistant/)
  use the connected Codex account. The default model is GPT-6 Sol when available.

Rust, Python, npm and desktop packages share version 0.2.14. Mac downloads are
signed and notarized for Apple Silicon; Windows and Linux remain desktop previews.
Project format remains 1, with historical projects and source provenance retained.
Fit error bars describe local covariance estimates, and repeated or cumulative
averages are not independent measurements. See the
[release notes](https://github.com/Ameyanagi/rexafs/blob/main/doc/release-notes-0.2.14.md)
and [qualification record](https://github.com/Ameyanagi/rexafs/blob/main/doc/validation/2026-09-24-release-0.2.14/review.md).

<span id="stable-0213"></span>

## Previous stable 0.2.13

[Download 0.2.13](https://github.com/Ameyanagi/rexafs/releases/tag/v0.2.13) or read the
[published release](https://github.com/Ameyanagi/rexafs/releases/tag/v0.2.13).

- A bundled [synthetic copper tutorial](/docs/desktop/synthetic-copper/) with
  three measured references and 50 deterministic mixtures of raw absorption.
  Known fractions are retained for validating PCA, MCR-ALS and LCF.
- PCA starts on a linear scale. MCR-ALS has independent range controls, with
  Auto covering the full common energy interval of the selected spectra.
- [Storage cleanup](/docs/desktop/storage/) reviews managed installer downloads
  and inactive updater app copies, with direct links to their hidden folders.
- Reproducible tutorial results and ruviz figures in SVG, vector PDF and
  600 dpi PNG, with default styling and small comparison markers.
- Reviewed dependency updates preserve historical random-number streams and
  project compatibility.

Rust, Python, npm and desktop packages share version 0.2.13. macOS desktop and
Python packages require Apple Silicon; Windows and Linux desktop packages remain
previews. Project format remains 1. Historical projects and reference provenance
are preserved. See the
[release notes](https://github.com/Ameyanagi/rexafs/blob/main/doc/release-notes-0.2.13.md)
and [qualification record](https://github.com/Ameyanagi/rexafs/blob/main/doc/validation/2026-09-22-release-0.2.13/review.md).

<span id="preparing-0212"></span>
<span id="stable-0212"></span>

## Previous stable 0.2.12

[Download 0.2.12](https://github.com/Ameyanagi/rexafs/releases/tag/v0.2.12) or read the
[published release](https://github.com/Ameyanagi/rexafs/releases/tag/v0.2.12).

- Apple Silicon only on macOS for desktop downloads and Python wheels. Version
  0.2.11 remains the last Intel Mac release; new nightlies also require Apple Silicon.
- RMC Auto memory and catalogue capacity, preparation progress and cache warnings.
- Element-pair structural distributions and histories, with saved/exported data.
- Desktop genetic and hybrid EA–RMC search with population checkpoint recovery.
- Compact publication presets, complete report publishing and change tracking.
- Readable Auto fields, recoverable numeric edits and direct Series navigation.
- The signed macOS updater-helper correction. Affected 0.2.10/0.2.11 Mac apps
  require one manual installation of the corrected app before later in-app updates.

See the [new RMC controls](/docs/desktop/rmc/#new-in-0212)
and [publication workflow](/docs/desktop/publication/). Windows and Linux remain
desktop previews. Short software checks do not establish RMC convergence.

All package channels share version 0.2.12. Mac downloads are signed and notarized.
Historical projects retain their scientific settings; the file format remains 1.
The [release notes](https://github.com/Ameyanagi/rexafs/blob/main/doc/release-notes-0.2.12.md)
and [qualification record](https://github.com/Ameyanagi/rexafs/blob/main/doc/validation/2026-09-19-release-0.2.12/review.md)
identify the immutable source, build and public packages.

<span id="stable-0211"></span>

## Previous stable 0.2.11

[Download 0.2.11](https://github.com/Ameyanagi/rexafs/releases/tag/v0.2.11) or read the
[published release](https://github.com/Ameyanagi/rexafs/releases/tag/v0.2.11).

- [RMC fitting](/docs/desktop/rmc/) inherits Transform ranges and expands theory
  coverage for the window and energy shift. Larger cells retain every selected
  absorber, with electronic preparation shared only for identical inputs.
- **CPU workers** allocates available logical CPUs for new desktop jobs. Absorbers
  run concurrently first; optional **Parallel paths** uses spare workers within
  the same bounded pool. Saved runs retain their captured settings.
- Calibration preview, bounded automatic moves and optional local coordinate
  refinement help assess and improve the starting fit. **ΔE₀ → Refine** updates
  the theoretical energy shift periodically while S₀² stays fixed. These controls
  do not establish a unique structure or physical convergence.
- FEFF10 0.2.4 corrects intermittent native array-header failures. Every desktop
  package uses the corrected library or matching Windows helper, with generated
  headers checked during qualification.

Rust, Python, npm and all six desktop targets share version 0.2.11. Mac downloads
are signed and notarized; Windows and Linux remain previews. RMC is available in
Rust and the desktop; Python and TypeScript RMC bindings remain planned.
Historical project fixtures are retained. See the
[release notes](https://github.com/Ameyanagi/rexafs/blob/main/doc/release-notes-0.2.11.md)
and [qualification record](https://github.com/Ameyanagi/rexafs/blob/main/doc/validation/2026-09-19-release-0.2.11/review.md).

<span id="stable-0210"></span>

## Previous stable 0.2.10

[Download 0.2.10](https://github.com/Ameyanagi/rexafs/releases/tag/v0.2.10) or read the
[published release](https://github.com/Ameyanagi/rexafs/releases/tag/v0.2.10).

- [RMC fitting](/docs/desktop/rmc/) adds guided structure/supercell setup, exact
  cached ReFEFF, real-plus-imaginary R-space fitting, live curves and coordinates,
  residual-trend diagnostics and saved-run recovery. Saved preprocessing is
  retained on resume. Adaptive scattering stays experimental and opt-in in Rust.
- [Processing](/docs/desktop/processing/) adds MBACK normalization, retained
  Cauchy wavelet maps in Transform, energy-offset controls and explicit
  fluorescence correction limited to XANES. Corrected groups cannot enter EXAFS.
- [XANES peak fitting](/docs/science/analysis/#xanes-peak-fitting) provides named
  peaks, steps and baselines with constraints and retained results. Python and
  TypeScript expose the same native calculation, MBACK and Cauchy wavelets.
- [Series](/docs/desktop/series/) adds full-frame measurements, saved recipes,
  region trends and recovery. Experimental Live acquisition watches completed
  source revisions; network-share and physical Windows/Linux acquisition remain
  unqualified.
- [Import](/docs/desktop/import/) can reuse a reviewed mapping across matching
  files and skip unwanted pending files with undo. Plot CSV/PNG/SVG exports cover
  additional processing, analysis, Series and Wavelet views.

All package channels share version 0.2.10. Mac downloads are signed and notarized;
Windows and Linux remain desktop previews. The desktop RMC workflow currently
uses one spectrum and structure; evolutionary and weighted-structure workflows
are Rust APIs. Project format 1 and historical compatibility fixtures are retained.
See the [release notes](https://github.com/Ameyanagi/rexafs/blob/main/doc/release-notes-0.2.10.md)
and [qualification record](https://github.com/Ameyanagi/rexafs/blob/main/doc/validation/2026-09-18-release-0.2.10/review.md).

## Previous stable 0.2.9

[Download 0.2.9](https://github.com/Ameyanagi/rexafs/releases/tag/v0.2.9) or read the
[published release](https://github.com/Ameyanagi/rexafs/releases/tag/v0.2.9).

- Native [MCR-ALS, LCF and PCA workflows](/docs/science/analysis/) prepare missing
  processed arrays on copies, validate shared coverage and retain results.
  Rust defaults to norm; the desktop defaults to flat and allows choosing norm.
- PCA shows reconstruction error versus component count, numerical-rank and
  indicator diagnostics, with linear or logarithmic axes. These are numerical
  diagnostics, not proof of a chemical species count.
- Recovered MCR components become ordinary calculated groups for subsequent
  background subtraction, Fourier transformation and fitting, with provenance.
- Collection plots offer **Plot all**, a sampled preview and color gradients.
  [Series](/docs/desktop/series/) follows the selected frame and retains all-frame
  LCF results. Plot presets and calculation ranges are separate.
- [Project import](/docs/desktop/import/#import-a-whole-project) selects spectra
  across Athena, Larch-compatible and Larix project scans. The plotted preview
  retains each scan's mapping and signal choices.
- The [Assistant](/docs/desktop/assistant/) receives a short overview and retrieves
  relevant headers, settings and analysis details when needed.
- [Update and restart](/docs/getting-started/updates/) supports macOS, Windows
  installers and supported Linux portable installations. Versions through 0.2.7
  need one manual upgrade to obtain this updater.

Rust, Python, npm and all six desktop targets are published. Windows and Linux
remain previews. Python and TypeScript LCF/PCA/MCR bindings remain planned.
Version 0.2.8's registry packages were published separately; its desktop draft
was not promoted. Version 0.2.9 includes those updater and import-discovery changes.

## Previous stable 0.2.7

[Download 0.2.7](https://github.com/Ameyanagi/rexafs/releases/tag/v0.2.7) or read the
[published release](https://github.com/Ameyanagi/rexafs/releases/tag/v0.2.7).

- The [Assistant](/docs/desktop/assistant/) has compact Model, Reasoning and
  Access menus. Parameters and Groups stay accessible beside it, with the
  Assistant narrowing when space is tight.
- Codex launchers opened from Finder can find their installed runtimes.
  Connection failures show a startup diagnostic and a Retry action.
- Ambiguous detector labels require explicit column selection. Distinct stored
  absorption columns remain separate signal choices, and malformed leading
  numeric rows produce an error rather than dropping samples.
- Edited Athena optional arrays survive save/reopen when the original project
  contains historical absence markers. The original markers remain retained.
- Public API signatures, numerical defaults, project format and Assistant
  permissions are unchanged. All package channels and six desktop targets
  remain available; Windows and Linux remain desktop previews.

## Previous stable 0.2.6

[Download 0.2.6](https://github.com/Ameyanagi/rexafs/releases/tag/v0.2.6) or read the
[published release](https://github.com/Ameyanagi/rexafs/releases/tag/v0.2.6).

- A shared reader across Rust, Python, TypeScript, the desktop and browser
  workspace detects beamline text, numeric tables, Athena, Larix, XTUNES and
  supported HDF5 datasets. Select columns by name or index, retain raw arrays
  and headers, and review ambiguous units or detector roles explicitly.
- The desktop import preview plots the selected signal and can import
  transmission, fluorescence and reference together. Expand source details when
  needed; the Data view opens on raw μ(E). See [import and groups](/docs/desktop/import/).
- Declared eV/keV, relative-energy and Bragg-angle calibrations are retained.
  Japanese 9809 and tested KEK `.qd` files use recorded angles and crystal
  spacing. See [reading measurements](/docs/reference/stable/measurement-reading/)
  for format coverage and limits.
- Four platform wheels support GIL-enabled CPython 3.10–3.14 using the stable
  Python ABI. The same wheel bytes are tested across those interpreters with
  minimum and latest compatible NumPy. Free-threaded Python remains unqualified.
- Test measurements retain their original bytes, source attribution and license
  records in the repository; fixture bundles are excluded from published packages.
- Existing format-1 projects and numerical defaults are unchanged. All six
  desktop targets remain available, with Windows and Linux labeled as previews.

## Previous stable 0.2.5

[Download the previous 0.2.5 release](https://github.com/Ameyanagi/rexafs/releases/tag/v0.2.5).

- ARM64 desktop previews for Windows and Linux, alongside the existing x64
  packages. Windows ARM64 requires Windows 11.
- FEFF10 in both Windows packages. rexafs and ReFEFF run natively on Windows
  ARM64; the x64 FEFF10 helper runs under emulation. Linux ARM64 includes native
  ReFEFF and FEFF10. See [platform requirements](/docs/getting-started/install/).
- Python keyword constructors and TypeScript options constructors, direct
  normalization/background settings, and `XrayFFTR`/`Spectrum.set_ifft()` for
  inverse configuration. Existing construction and setter forms remain valid.
- Expanded editor help and shorter workflow guides. Rust Fourier plot-axis
  labels now include the integration measure; calculated amplitudes are unchanged.
- Existing format-1 `.rxs` projects remain supported. Numerical defaults are
  unchanged.

## Website previews

- [Spectrum processing](/app/): local text/CSV import, processing, plots and
  CSV/provenance exports using the source-checkout engine.
- [Scattering calculations](/app/scattering/): ReFEFF 0.4.0 in a cancellable
  browser Worker, with generated FEFF files and a provenance record.

These follow website deployment, separately from the versioned npm API. Native
rexafs 0.2.14 also uses ReFEFF 0.4.0. See [WASM scope](/docs/libraries/webassembly/).

## Earlier releases

<span id="stable-024"></span>

### 0.2.4

[Release 0.2.4](https://github.com/Ameyanagi/rexafs/releases/tag/v0.2.4).

- The window fits the available display, and the empty workspace shows the
  import, project and Cu example actions directly.
- Platform monospace fonts, clipped long field text, a visible caret while
  editing and corrected hit testing in translated fields.
- Stage shortcuts keep working after a focused plot control disappears.
- Folder scans stop promptly when cancelled and use bounded memory.
- Explicit Larch FFT-grid selection and corrections to the legacy iterative
  AUTOBK derivative. The fixed-penalty default is unchanged.
- Existing format-1 `.rxs` projects remain supported.

| Release | What changed for users |
|---|---|
| [0.2.3](https://github.com/Ameyanagi/rexafs/releases/tag/v0.2.3) | Windows sessions open without a console window; Ctrl shortcuts and multi-selection on Windows and Linux; faster accessibility updates. Numerical defaults unchanged from 0.2.1. |
| [0.2.1](https://github.com/Ameyanagi/rexafs/releases/tag/v0.2.1) | Starts in the empty Data workspace; simplified processing controls and contextual Fit steps; explicit main and optional import channels; undoable Remove marked; optional AUTOBK weight link to the FFT; persistent color cycles and publication presets with 300 DPI figures and CSV export; structure center focus and depth cues. |
| [0.2.0](https://github.com/Ameyanagi/rexafs/releases/tag/v0.2.0) | Unified Groups with durable identities, marks and processing locks; per-file import receipts and pending-layout review; a focused column-mapping editor with axis conversion; saved import recipes; XDI edge identities and parser diagnostics. |
| [0.1.4](https://github.com/Ameyanagi/rexafs/releases/tag/v0.1.4) | Coordination number N for each path; fit reports copied as Markdown; the optional assistant with saved conversations in projects. |
| [0.1.3](https://github.com/Ameyanagi/rexafs/releases/tag/v0.1.3) | ReFEFF and FEFF10 bundled in macOS and Linux builds, ReFEFF on Windows; signed and notarized Mac installers; Windows and Linux desktop previews with a per-user Windows installer. |
| [0.1.2](https://github.com/Ameyanagi/rexafs/releases/tag/v0.1.2) | Common $\chi(k)$ display weight for overlays; exposed background $E_0$, χ standards, solver settings and inverse-transform controls; corrected inverse FFT windowing; Stable and Nightly update discovery with verified Mac downloads; multi-file and folder import with separate sample, fluorescence and reference channels; draggable AUTOBK and inverse R limits; flattened $\mu(E)$ publication figure and CSV export. |
| [0.1.1](https://github.com/Ameyanagi/rexafs/releases/tag/v0.1.1) | Fixed duplicate spectra during fast plot pans; floor-based automatic AUTOBK spline count; the single-solve fixed endpoint penalty (`clamp_lambda = 0.001`) for new AUTOBK analyses. |
| [0.1.0](https://github.com/Ameyanagi/rexafs/releases/tag/v0.1.0) | First rexafs release. |

Version 0.2.2 was tagged but withheld after a Linux qualification failure; its
changes shipped in 0.2.3. See each release page for its exact platform scope
and checksums.

## Migration notes

- **Projects.** `.rxs` format 1 has been the only project format since 0.1.0.
  New releases read earlier projects; an older executable may not preserve
  newer features when saving. Keep the original file when moving between
  versions. See [projects and recovery](/docs/desktop/projects/).
- **AUTOBK defaults.** Since 0.1.1, new analyses use the fixed endpoint
  penalty. Older projects preserve their saved clamp policy, so check it before
  comparing results with a new project. See the [AUTOBK
  objective](/docs/science/autobk/).
- **Library APIs.** Version 0.2.5 adds Python keyword constructors, TypeScript
  options constructors and direct normalization/background settings. It also
  exposes `XrayFFTR` and `Spectrum.set_ifft()` in Python/TypeScript and adds the
  spectrum setter in Rust. Version 0.2.4 needs zero-argument Python/TypeScript
  settings constructors followed by field assignment; those forms remain valid
  in later releases. See the [Stable API reference](/docs/reference/).
