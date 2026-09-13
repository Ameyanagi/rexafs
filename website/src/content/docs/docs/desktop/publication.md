---
title: "Publication and exports"
description: "Create figures, captions, data tables and reproducible analysis exports."
audience: user
---

Open **Publish** after processing a spectrum or completing a fit. Select a figure,
adjust its controls, and save **PNG** or **SVG**. The preview preserves the image's
aspect ratio and uses the same rendered PNG bytes as the individual PNG save.

Blank controls use ruviz's native defaults, including its 6.4 × 4.8 inch canvas at
100 DPI (640 × 480 pixels with the current dependency). Width and height use
inches; font and line widths use points. Set DPI to the journal's requested raster
resolution, or use the vector SVG for scalable line art. Changing the preview
window does not alter export dimensions. Reset restores the selected figure's
defaults.

Controls include labels/title, paired axis limits, legend, grid, processing guides
and individual visible curves. Fit figures expose model/data components and path
contributions without vertical offsets. R-space residuals explicitly represent
the difference of magnitudes, rather than a complex residual. All scientific
curves come from the processing/fit results; branding images are separate assets.

Enter a figure caption and processing/parameter/path-table captions in the editor;
press **Enter** to apply text. Blank captions use factual descriptions of the
selected curves or table definitions. **Copy caption** transfers the current
figure's caption. Settings and captions are saved in `.rxs` files and apply to
the same figure/table type across a multi-spectrum export. For different samples,
use general captions here and complete individual exported captions in the report.

**Export analysis folder** creates a new directory containing:

- `report.html`: a report with vector figures, numbered figure captions and
  semantic tables with captions, units and uncertainty definitions; printable
  through a browser, with image aspect ratios retained;
- `README.md`, `analysis.md` and `captions.md`: linked figures, a captioned analysis
  record and captions ready to edit into a manuscript;
- PNG/SVG figure pairs, with descriptive caption metadata in the SVG;
- `manifest.json`: figure/table numbering, captions, presentation settings,
  source/fit associations and any unavailable or stale results;
- processed arrays, available fit arrays, requested/resolved settings, fit history,
  the project, a methods draft and reference files.

Captions identify data and conventions; they do not infer sample preparation,
temperature, beamline conditions or scientific conclusions. Complete those from
the experimental record. Standard errors represent the fit covariance, not all
experimental/model uncertainty. R-space coordinates are not phase corrected.
Review unavailable arrays and stale-fit notices before selecting results for a
paper. The export preserves evidence for that review; it does not certify a fit's
scientific validity or compliance with every journal's submission rules.

## Analysis folder contents

Use **Publish** to adjust figures and captions, save individual PNG/SVG files, or choose **Export analysis folder…**. See the [publication editor guide](/docs/desktop/publication/) for controls, defaults and caption conventions. A new local folder contains:

| Asset | Contents |
|---|---|
| `analysis.md` | Requested settings, source comments, current model, historical fit inputs, values, uncertainties, path distances and journal |
| `resolved.md` | Per-spectrum processing outputs recomputed at export time |
| `figures/*.png`, `figures/*.svg` | Spectra, fit overlays and residuals using the saved dimensions/style; ruviz defaults when unset |
| `report.html`, `captions.md` | Vector figures and tables with numbered captions, plus manuscript caption text |
| `data/*.json` | Processed arrays and available full fit results |
| `methods.md` | Editable methods draft with missing experimental details identified |
| `references.md`, `references.bib` | Algorithm references and reminders to cite the actual data, structures and FEFF backend |
| `state.json`, `project.rxs` | Structured analysis context and project |
| `batch-results.csv` | Batch results when available; the manifest flags stale results |
| `README.md`, `manifest.json` | Figure index and any incomplete exports |

Scope is the current spectrum, marked spectra, assigned fit spectra and recorded results. **Copy Markdown** copies the analysis record without exporting figures. These assets also provide context that an external LLM can read.

An existing destination is never overwritten. The project uses the selected raw-data mode: relative links by default, or losslessly compressed original spectra and referenced FEFF inputs with **Raw: embedded**. Its metadata header records sources and checksums. Full processed arrays are included. Archived fit statistics remain exportable when their plot arrays are unavailable; the manifest reports the missing figures. Auto requests and historical settings are explicitly distinguished from current, resolved values. The methods text is a draft, not an invented experimental record.


[![Full publication editor showing a vector Cu fit figure and output controls](/screenshots/publication.jpg)](/screenshots/publication.jpg)

*Review the selected curves, axis units and caption before exporting. Full application window, rexafs 0.2.4 on macOS. Select the image to view its full resolution.*
