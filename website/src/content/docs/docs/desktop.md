---
title: "Desktop analysis"
description: "Follow the measurement-to-fit workflow in the rexafs desktop."
audience: user
---

The desktop is a complete analysis environment: it imports measured spectra,
processes them, builds and fits structural models, browses measurement series
and exports publication-ready results, all inside one saved project. It runs
on macOS, Windows and Linux without a Python or Rust installation.

## The workspace

The window has three areas. The **Groups** panel on the left lists imported
spectra and derived results. The center shows plots for the selected stage. The
**Parameters** inspector on the right holds the controls for that stage on the
current group. The **stage bar** across the top follows the analysis order:

| Stage | What it does |
|---|---|
| **Data** | Import files, review column mappings, organize groups, and apply data treatment such as alignment, rebinning or merging. |
| **Normalize** | Estimate the edge energy, fit the pre-edge and post-edge baselines and scale the absorption to a unit edge step. |
| **Background** | Remove the smooth background with AUTOBK to obtain χ(k). |
| **Transform** | Weight and window χ(k), Fourier-transform it to χ(R), and back-transform a selected R range to χ(q). |
| **Fit** | Choose a structure, calculate scattering paths, select paths, set up a model and run single or joint fits. Its steps are **Structure → Calculate → Paths → Model → Results**. |
| **Series** | Browse a folder of scans as ordered frames and inspect trends across them. |
| **Publish** | Set figure size, labels, limits and captions, then export PNG, SVG, CSV or a complete analysis folder. |

Processing runs automatically when a spectrum loads or a setting changes, so
selecting a stage changes what you see and edit, not what is calculated.

| Task | Guide |
|---|---|
| Read files, assign columns, compare channels and manage groups | [Import and groups](/docs/desktop/import/) |
| Normalize, remove background and Fourier-transform EXAFS | [Processing controls](/docs/desktop/processing/) |
| Choose an absorber, structure and scattering paths | [Structures and paths](/docs/desktop/structures/) |
| Fit one spectrum and inspect uncertainty | [First structural fit](/docs/desktop/fitting/) |
| Share parameters or process a batch | [Multiple spectra](/docs/desktop/multiple-spectra/) |
| Browse a scan and inspect frame-by-frame trends | [Series and trends](/docs/desktop/series/) |
| Move or recover a project | [Projects](/docs/desktop/projects/) |
| Export figures, data, captions and methods | [Publication](/docs/desktop/publication/) |
| Use the optional analysis assistant | [Assistant](/docs/desktop/assistant/) |

**Groups** and **Parameters** toggle the side panels. Use the action search to
find a stage or processing tool. Per-spectrum processing settings and group
assignments belong to the project; application layout preferences belong to
your computer.

## Keyboard navigation

Use **Cmd** on macOS and **Ctrl** on Windows/Linux for the modifier below. These
are the shortcuts in [rexafs
0.2.4](https://github.com/Ameyanagi/rexafs/blob/v0.2.4/crates/rexafs-gui/src/app.rs#L484).
Most workspace shortcuts yield while the Assistant has focus, and text fields
retain their normal editing shortcuts.

| Action | Shortcut |
|---|---|
| Open a project | Cmd/Ctrl+O |
| Import files or folders | Cmd/Ctrl+Shift+O |
| Save the project | Cmd/Ctrl+S |
| Search actions | Cmd/Ctrl+K |
| Data through Publish, in stage-bar order | Cmd/Ctrl+1 through Cmd/Ctrl+7 |
| Toggle Groups / Parameters | Cmd/Ctrl+B / Cmd/Ctrl+J |
| Focus the group filter | Cmd/Ctrl+P |
| Undo / redo an analysis edit | Cmd/Ctrl+Z / Cmd/Ctrl+Shift+Z |
| Show the action journal | Cmd/Ctrl+Shift+J |

When the Groups list has focus, use Up/Down to navigate, Space to toggle a mark,
and F2 or Enter to rename a group. Cmd/Ctrl+A marks the displayed groups;
Cmd/Ctrl+Shift+A clears marks. **Help** opens the bundled Cu example, the offline
license reader and Updates. Undo history belongs to the current session; save a
project or export an analysis record before closing it.

For a complete start-to-finish example, begin with
[your first analysis](/docs/getting-started/first-analysis/).


[![The complete Data stage displaying the measured Cu absorption spectrum](/screenshots/data.jpg)](/screenshots/data.jpg)

*The selected group determines the current spectrum and its parameters. Full application window, rexafs 0.2.4 on macOS. Select the image to view its full resolution.*
