---
title: "Release history"
description: "Published rexafs versions, what changed for users, and migration guidance."
audience: user
---

Each stable release publishes desktop packages for macOS, Windows and Linux
together with the Python, npm and Rust packages. Every release also adds
saved-project fixtures to the regression suite, so a new release reads the
projects of every earlier release. Save and back up your project before
updating. Changes on the source branch are described in the [Next API
reference](/docs/reference/), separately from these published versions.

## Stable 0.2.4

[Download 0.2.4](/download/) or read the
[published release](https://github.com/Ameyanagi/rexafs/releases/tag/v0.2.4).

- The window fits the available display, and the empty workspace shows the
  import, project and Cu example actions directly.
- Platform monospace fonts, clipped long field text, a visible caret while
  editing and corrected hit testing in translated fields.
- Stage shortcuts keep working after a focused plot control disappears.
- Folder scans stop promptly when cancelled and use bounded memory.
- Explicit Larch FFT-grid selection and corrections to the legacy iterative
  AUTOBK derivative. The fixed-penalty default is unchanged.
- Existing format-1 `.rxs` projects remain supported.

## Earlier releases

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
- **Library APIs.** Stable 0.2.4 uses zero-argument settings constructors in
  Python and TypeScript with fields assigned afterward. Keyword and options
  constructors, direct settings assignment and `XrayFFTR` inverse configuration
  are documented as the unreleased [Next API](/docs/reference/).
