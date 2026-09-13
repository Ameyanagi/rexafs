---
title: "Desktop analysis"
description: "Follow the measurement-to-fit workflow in the rexafs desktop."
audience: user
---

Use the stage bar to move between **Data**, **Normalize**, **Background**,
**Transform**, **Fit**, **Series** and **Publish**. Inside Fit, the steps are
**Structure → Calculate → Paths → Model → Results**.

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
