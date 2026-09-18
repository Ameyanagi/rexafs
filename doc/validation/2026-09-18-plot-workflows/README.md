# Plot workflow checks — 18 September 2026

This local, unreleased macOS release build was checked with computer use in an
isolated rexafs test app. The user's working application and unpublished ReGe
measurements were not used. The two experimental Cu measurements are the public
XAS Data Library fixtures retained in the repository; see the
[fixture provenance and licenses](../../../crates/rexafs/tests/fixtures/xas/README.md).

- Opening Transform → Wavelet displays its map without Calculate. Changing R
  maximum from 6 to 4 Å updates the grid from 97 to 65 R rows. Changing weight
  from 2 to 3 updates the map, scale and linked spectra without another action.
  Switching from Cu at 10 K to room-temperature Cu keeps the chosen settings and
  automatically displays the new source's map.
- An invalid k interval keeps the last valid map and reports the error. Restoring
  the interval recalculates automatically. Colors is in the top-right toolbar;
  its palette menu also contains Lock scale. History is absent.
- The source menu in the 340-pixel Structure library panel exposes Curated, CIF
  folder, Materials Project, AMCSD and COD. COD remains selectable and visible;
  a public `ReO2` search returned one result.
- Comparison CSV export was performed through the GUI. It contained both full
  arrays: 612 points from `cu_metal_10K.xdi` (8786.204–11362.47 eV), and 408 from
  `cu_metal_rt.xdi · scan_1` (8779–10145.86 eV). Labels and separate energy grids
  were retained. No curve was replaced by the active spectrum.
- PNG and SVG exports were saved through the GUI. The PNG was visually inspected:
  both comparison curves and their full legend labels are present. The SVG was
  parsed as XML and checked for both labels. The exported PNG is linked below.
- A subsequent toolbar update places Magnitude, Real, Imaginary and Phase
  immediately beside Wavelet. Computer-use checks in the rebuilt release app
  confirmed all four selections update the map without moving the controls.
  Returning from R restores the selected Wavelet display. Colors and Export
  remain at the right of the toolbar. Formatting and the release build passed.
- The subsequent header cleanup removes the sample name and routine grid/order
  message above the map. Progress and errors share the Spectra/Slices caption
  row. In the rebuilt app, setting k maximum to 1 Å⁻¹ while k minimum was
  2 Å⁻¹ displayed an error with the previous map and plot position preserved.
  Restoring 12 Å⁻¹ recalculated successfully and cleared the error. JSON export
  notices now use the application status bar.

Automated checks: the desktop suite passed 598 tests with 6 ignored before the
last export presentation adjustments. Subsequent focused Wavelet checks passed
10 tests, including cancellation, automatic-preview retention, project
round-tripping and CSV/native-grid agreement on experimental Cu data. Both CSV
export tests passed, followed by 12 plotting tests after the final presentation
changes. The final release build and website checks succeeded;
the website reported no errors, warnings or hints. Existing desktop dead-code
warnings remain. These are local checks, not a claim of Windows/Linux GUI
qualification.

Screenshots and output:

- [Automatic Wavelet update](wavelet-auto.jpg)
- [Updated Wavelet component toolbar](wavelet-toolbar.jpg)
- [Wavelet with the redundant header removed](wavelet-clean-header.jpg)
- [Comparison Export menu](comparison-export.jpg)
- [Structure source menu](structure-sources.jpg)
- [COD search result](cod-result.jpg)
- [Exported comparison PNG](comparison.png)

CSV reflects the plotted preview subset and any display offsets; it does not
silently expand a sampled comparison to all loaded groups. Wavelet CSV uses the
full native numerical grid, including explicitly flagged padding. See the
[desktop guide](../../../website/src/content/docs/docs/desktop/processing.md)
for export formats and conventions.
