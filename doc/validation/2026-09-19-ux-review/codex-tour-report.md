<!-- Retained tester notes from the Codex computer-use session that produced the captures in this folder. Written by the tester, lightly path-scrubbed; not edited for content. These are impressions, not verified findings; see ../../ux-review-0.2.11.md for the reviewed conclusions. -->

# rexafs desktop UI screenshot tour

Tour date: 19 September 2026, approximately 13:28–13:44 JST. All 14 requested steps were attempted. The tour produced **73 original PNG screenshots** in this folder (re-encoded as JPEG; see [README.md](README.md)), including intermediate failures and extra views. Numbers reflect capture order, not the numbered task steps. No screenshot was cropped, annotated, or edited.

The most useful starting captures are **01, 12–18, 20–24, 28–30, 33–47, 51–60, 62–70, and 72–73**. Captures 04–11 mostly document native-dialog automation difficulties rather than application defects.

## Environment, scope, and completion

- Executable: `<repository>/target/release/rexafs`, launched from a shell in the repository with `REXAFS_SETTINGS` pointing to this artifact directory's `settings.json`. The application was not rebuilt.
- The executable's window title is **rexafs Nightly**. Updates identifies it as **0.2.11 · nightly-local-updater-test · Nightly**. These observations concern this exact supplied executable, not an independently verified stable application bundle.
- The original window measured **1440 × 932 macOS points**. Its PNG captures are 2880 × 1864 pixels. The narrow-window test measured **1100 × 700 points**, with 2200 × 1400 pixel captures.
- All three supplied fixture files were copied to this directory. The copied `cu_150k.xmu` was imported, then the copied `rexafs-0.2.11-embedded.rxs` was opened. `cu_metal_rt.xdi` was prepared but not opened; no numbered step required its import.
- Original and copied fixtures remain byte-identical. See [fixture-checksums.json](fixture-checksums.json). Screenshot dimensions and SHA-256 hashes are in [screenshots.json](screenshots.json).
- No fit, path calculation, RMC run, PCA training, LCF fit, batch fit, or live acquisition was started. Existing results and automatically displayed plots were inspected. No assistant message was sent, account action taken, update downloaded, or export committed.
- The UI displays extraction/workspace paths under `~/.rexafs/project-data/` despite the separate settings path. I did not open, modify, or clean those locations manually. The settings override should not be described as proof of complete application-storage isolation.
- The requested process, PID 58797, was quit with **⌘Q**. It exited with status 0, its window disappeared, and **no save prompt appeared**. The theme was returned to dark before quitting.
- This tour wrote only to the artifact directory. Repository status was initially clean; the final status contained an untracked `doc/ux-review-0.2.11.md`, which this tour did not create or edit. The before/after status files are preserved here. No repository change was reverted.

Several rexafs copies shared the same application identifier. Initial background launch attempts did not remain available, and an app-name-based computer-use selector returned the older installed instance. That selector was abandoned before sending clicks or keys through it. The successful tour used the requested executable in a persistent shell session, followed by process-specific macOS accessibility activation, keyboard input, mouse input, and native window capture. An accessibility button press was also used to expand Wavelet settings after coordinate-click attempts did not expand it. These automation difficulties are not classified as confirmed UI bugs.

Most captures contain only the requested app window. **05 and 48 are full-desktop captures** taken to inspect the detached native dialog and macOS menu bar; other applications are visible in their backgrounds. Captures 06–10 contain only the native file-dialog window. No other application's content was intentionally clicked or edited during the tour.

## 1. Empty workspace

Captured the fresh workspace before opening fixtures. The default theme was dark. Seven stages run across the top: Data, Normalize, Background, Transform, Fit, Series, and Publish. Groups occupies the left column, the empty plot area the center, and Source/processing controls the right.

The central **Start an analysis** card provides Import spectra…, Open project…, and Open Cu example. This is a useful entry point. However, many scientific tools are already visible in the right column without input data, and small toolbar icons and stage-status dots have little visible explanation. A **Stable update available — open Updates to review it** notice appeared immediately at the bottom.

Screenshot: [01-empty-workspace.png](01-empty-workspace.jpg).

## 2. Command palette

Opened ⌘K without typing, then searched for `fit`, and escaped. The unfiltered list combines stage navigation, group operations, parameter copying, fitting, and export. It includes keyboard hints and a short navigation footer.

The filtered list includes Go to Fit, Fit the current group, Export batch-fit results as CSV, Check for updates · Stable / Nightly, and Linear combination fit…. The updates entry is surprising in this search. Later, a `linear` search placed **Apply all processing settings to marked groups** above **Linear combination fit…**. I explicitly selected the second row; the parameter-copying action was not run.

Screenshots: [02-command-palette.png](02-command-palette.jpg), [03-command-palette-fit.png](03-command-palette-fit.jpg), [45-tools-palette-lcf.png](45-tools-palette-lcf.jpg).

## 3. Raw-data import

Used ⌘⇧O and the native file dialog. Entered the copied file's absolute path using the Go to Folder flow. Initial input/capture attempts failed to show or navigate the detached dialog correctly. The successful native selection is capture 10, and the actual import-mapping preview is capture 12.

The mapping modal identifies **cu_150k.xmu**, **text · 618 points**, and the selected **Stored signal · mu**. It provides a substantial preview plot with Energy (eV), Detected energy, Columns, Source details, Reset mapping, Cancel, and **Import 1 spectrum**. The preview is helpful, but column assignments are hidden behind disclosure controls. The sentence **μ column: μ(E) = 2: mu** is difficult to parse without knowing the numbering convention. Both Close and Cancel are offered.

Clicked Import 1 spectrum. The group becomes **cu_150k.xmu · scan_1**, with the plot occupying most of the center. The Source panel states that the original measurement and mapping are retained. A single ordinary imported spectrum appears beneath a **Results** heading, which is surprising for raw input.

Screenshots:

- [04-import-attempt-main-window.png](04-import-attempt-main-window.jpg): main window only; detached dialog absent from this capture.
- [05-import-native-dialog.png](05-import-native-dialog.jpg): full desktop, showing the detached dialog and focus difficulty.
- [06-import-native-dialog-window.png](06-import-native-dialog-window.jpg): native dialog before successful path navigation.
- [07-import-path-not-yet-applied.png](07-import-path-not-yet-applied.jpg), [08-native-dialog-navigation-attempt.png](08-native-dialog-navigation-attempt.jpg): the intended fixture path was not yet applied.
- [09-import-path-entry.png](09-import-path-entry.jpg): Go to Folder sheet.
- [10-import-fixture-selected.png](10-import-fixture-selected.jpg): copied fixture selected.
- [11-import-confirm-attempt-main-window.png](11-import-confirm-attempt-main-window.jpg): main window during the confirmation attempt; not the mapping preview.
- [12-import-mapping-preview.png](12-import-mapping-preview.jpg): successful mapping preview before confirmation.
- [13-import-confirmed.png](13-import-confirmed.jpg): imported spectrum.

## 4. Fixture project and Data stage

Opened the copied project with ⌘O and a native path dialog. Data displayed **Groups 7**, a pending `unavailable-pending.dat`, a folded Ru_QAS.dat entry, cu_150k.xmu, two marked fixture groups, second.xmu, and a synthetic result with an unconfirmed quantity. The current group was cu_150k.xmu.

The top receipt reads **Added 2 files → 2 groups · 1 needs mapping · 1 warnings** even though this action opened a saved project. This appears to be restored fixture state; it is not evidence that the tour just imported two additional files. One group shows warning and lock icons. Long names are ellipsized, making similar fixture groups hard to distinguish. **Compare 3** is shown while the center primarily presents one recognizable green trace; the relationship among current, marked, and plotted groups requires interpretation.

The Source panel includes **eV assumed; units absent** and **Mapping: detected or manually assigned · no reusable recipe**. It is detailed, but its length pushes processing tools below the fold.

Screenshot: [14-project-open.png](14-project-open.jpg).

## 5. Normalize, Background, Transform, and Wavelet

Used ⌘2, ⌘3, and ⌘4 with the right parameter panel visible throughout.

- **Normalize:** Polynomial and MBACK choices, E0, edge step, pre-edge limits, normalization limits, and polynomial order are visible. Blue and amber guides mark ranges on the plot. Automatic values use blue text and often include their resolved values, which helps. The fit-maximum value is clipped, and the current-group name in the panel header is squeezed by Apply to 1 and its explanatory text.
- **Background:** Two stacked plots show μ(E) with an AUTOBK spline and an R-space magnitude. The panel exposes rbkg, k limits, a background k-weight linked to FFT, and collapsed options. A thin negative spike in the orange spline expands the upper plot's vertical range substantially. I did not assess the scientific correctness of that curve.
- **Transform:** The k + R view has two plots, shaded windows, and Forward FT parameters. Abbreviations such as dk and R max out are compact but require domain knowledge. Back FT and other options are collapsed.
- **Wavelet:** Clicked Wavelet. A heatmap, an R-space side plot, and a lower k-space trace appear, with Magnitude/Real/Imaginary/Phase and Spectra/Slices controls. The side plot's horizontal tick labels are very dense. The heatmap relies on its adjacent plots to explain the axes. Expanded Wavelet settings later: it has its own k limits, k-weight, and R max, plus a note that R is not phase-corrected and colors affect only display. The coexistence of Forward FT k-weight 3 and Wavelet k-weight 2 is visible, but their relationship is not explained in the immediate view.

Screenshots: [15-normalize.png](15-normalize.jpg), [16-background.png](16-background.jpg), [17-transform.png](17-transform.jpg), [18-transform-wavelet.png](18-transform-wavelet.jpg), [71-wavelet-heading-click-attempt.png](71-wavelet-heading-click-attempt.jpg), [72-wavelet-settings.png](72-wavelet-settings.jpg). Capture 71 records an unsuccessful coordinate-click expansion; 72 shows the expanded panel.

## 6. Fit steps and RMC

⌘5 initially restored **Model**, **Fit multiple spectra**, and **Spectra & paths**. I then clicked every step in the requested order.

| View | What was visible | Screenshot |
| --- | --- | --- |
| Initial Fit | Multiple-spectrum model, one spectrum, undefined parameters, disabled Run fit. | [19-fit-initial.png](19-fit-initial.jpg) |
| Structure | Curated offline structure list, Cu filter, import CIF/XYZ, and a two-atom structure preview. The instruction says Choose a structure even while a structure preview is visible. | [20-fit-structure.png](20-fit-structure.jpg) |
| Calculate | ReFEFF versus FEFF-RS / FEFF10; local-workspace path; Calculate paths button. Not activated. | [21-fit-calculate.png](21-fit-calculate.jpg) |
| Paths | Geometry, FEFF curves, selected Cu–Cu first shell, and path table with Reff, N, legs, and amplitude. | [22-fit-paths.png](22-fit-paths.jpg) |
| Model | Copper spectrum, fit ranges, k-weights, global versus per-spectrum scope, S₀², N, and undefined de0/dr/ss. | [23-fit-model.png](23-fit-model.jpg) |
| Results | Processed-data plots, Result/History/Batch controls, and a message to run the fit for model/residual contributions. No completed fit was displayed here. | [24-fit-results.png](24-fit-results.jpg) |

The fixture's multiple-spectrum setup has only one spectrum. The instruction **Add at least two spectra in Spectra & paths** explains one blocker, but the form simultaneously displays undefined raw parameter identifiers and **0 free variables**, despite a checked fit control for S₀². The wording and priority of these blockers are difficult for a new tester to reconcile. **Fixed** wraps as **Fixe / d** at the N parameter, even at the original window width. These are observations of the restored fixture state; I did not repair the model or attempt fitting.

The top-right **Fit mode: Path fitting** menu exposes **RMC**. Its terminology overlaps with the separate Single spectrum / Fit multiple spectra controls also labeled Fit mode.

RMC provides numbered Structure, Supercell, Fit settings, and Results steps. I visited all four without choosing/building a new structure or running a calculation. The draft shows repeat counts of 2, attempt budget 10000, 4 CPU workers, auto moves, move size 0.05 Å, k/R limits, k-weight, and amplitude/energy options. **R min** renders a long **1.349999999…** value that does not fit comfortably in the field. Scrolling exposes S₀², ΔE0 refinement, Estimate calibration…, and Advanced settings.

Results and Run details were reachable, but both showed **Your RMC results will appear here**. The draft has no completed run available in this view. I did not use Recover latest run because it could load an unrelated local run, nor did I start a run to populate results.

Screenshots: [25-fit-mode-menu.png](25-fit-mode-menu.jpg), [26-rmc-structure.png](26-rmc-structure.jpg), [27-rmc-supercell.png](27-rmc-supercell.jpg), [28-rmc-fit-settings.png](28-rmc-fit-settings.jpg), [29-rmc-results.png](29-rmc-results.jpg), [30-rmc-run-details.png](30-rmc-run-details.jpg), [73-rmc-settings-lower.png](73-rmc-settings-lower.jpg).

## 7. Series and Publish

Series initially showed a largely empty center with **Live acquisition…** and **Select series or scan**. Later I opened the selector and chose the available **data · 5 frames** row. This populated a heatmap, a current-frame plot, an edge-energy trend, a frame cursor, and LCF/batch-fit controls. No Run button was used.

The first frame was Ru_QAS.dat with E0 about 22118.8 eV, while later frames were near the Cu edge. In this mixed fixture selection, most heatmap rows appeared uniformly dark and the trend had a large discontinuity. The immediate view did not explain the disparate energy ranges or distinguish absent coverage from a low signal. This is an interpretability concern demonstrated by a heterogeneous fixture, not a claim that normal homogeneous scans behave the same way.

The Series plot export menu offers CSV, PNG, and SVG, with useful text explaining that zoom does not trim exported CSV data. Results opens a separate view with Frame/Coordinate/Time and Advanced, but says **No saved trends. Choose Add trend to start.** The contrast with the overview trend already visible in Series could confuse a first-time user.

Publish exposes PNG, SVG, CSV, Analysis folder, and Markdown. PNG shows figure selection, dimensions, DPI, fonts, line width, legend/grid/guides, and Typst title/axis fields, plus a large preview. CSV says full data grids are exported; Analysis folder lists current, marked, and joint-fit inputs; Markdown changes the main action to Copy. The figure preview and image-style controls remain essentially unchanged across these output types, making it hard to preview the actual table, folder contents, or text to be produced. No export or clipboard action was executed.

Screenshots: [31-series.png](31-series.jpg), [32-publish.png](32-publish.jpg), [64-series-selection.png](64-series-selection.jpg), [65-series-data.png](65-series-data.jpg), [66-series-export-menu.png](66-series-export-menu.jpg), [67-series-results.png](67-series-results.jpg), [68-publish-csv.png](68-publish-csv.jpg), [69-publish-analysis-folder.png](69-publish-analysis-folder.jpg), [70-publish-markdown.png](70-publish-markdown.jpg).

## 8. Panel shortcuts and Assistant

Tested all supplied shortcuts. The actual mapping differs from the task's tentative panel descriptions:

| Shortcut | Observed behavior | Screenshot |
| --- | --- | --- |
| ⌘B | Toggles the left Groups panel. Captured hidden, then restored. | [33-toggle-cmd-b.png](33-toggle-cmd-b.jpg) |
| ⌘J | Toggles the right Parameters panel. Captured hidden, then restored. | [34-toggle-cmd-j.png](34-toggle-cmd-j.jpg) |
| ⌘P | Focuses the Groups filter field. It did not open a panel. | [35-toggle-cmd-p.png](35-toggle-cmd-p.jpg) |
| ⌘I | Inverts shown groups' marks. Marks changed from 2 to 4. I pressed it again to restore the selection. | [36-toggle-cmd-i.png](36-toggle-cmd-i.jpg) |
| ⌘⇧J | Toggles the bottom Journal panel. It showed 0 steps and 0 undoable. | [37-toggle-cmd-shift-j.png](37-toggle-cmd-shift-j.jpg) |

Inverting marks exposed a warning about the synthetic average's unconfirmed quantity. No inspector was opened by these shortcuts. The group menu later confirmed **Invert shown ⌘I**.

Opened Assistant with the speech-bubble toolbar icon. It is labeled Experimental and shows context for the current spectrum, Check processing, Suggest a fit model, history/new/settings icons, and an empty message field. I only inspected its UI; the Account control and prompt buttons were not activated.

The menus below the message field expose model selection, reasoning effort, and Review/Edit analysis/Workspace commands modes. The mode menu has useful descriptions. With Groups, Parameters, and Assistant all open, the plot is squeezed into a tall, thin column even at 1440 points wide.

Screenshots: [38-assistant-panel.png](38-assistant-panel.jpg), [39-assistant-model-menu.png](39-assistant-model-menu.jpg), [40-assistant-effort-menu.png](40-assistant-effort-menu.jpg), [41-assistant-mode-menu.png](41-assistant-mode-menu.jpg). [61-journal-click-attempt.png](61-journal-click-attempt.jpg) also shows Journal at the narrow size: an initial attempt to reach warning details clicked Journal instead.

## 9. Group context menu and rename

Right-clicked cu_150k.xmu. The menu includes Make current, Mark, Rename, Color, Duplicate group, Lock processing, Re-map columns, Add channel, alignment standard, input/source actions, export, removal, merge, align, and compare. Some disabled choices include explanations, which is helpful. The menu is long and mixes per-group and multi-group operations.

Clicked Rename and captured the inline editor, then escaped without typing. It has no visible commit/cancel instructions. Also opened the Groups header's ellipsis menu. It offers removal, mark/clear/invert operations, Mark scan, Mark filter results, Keep every 10th mark, Show marked, Clear hidden marks, and Lock current. The differences between shown, marked, current, scan, and filter results take effort to understand.

Screenshots: [42-group-context-menu.png](42-group-context-menu.jpg), [43-group-rename-editor.png](43-group-rename-editor.jpg), [44-group-actions-menu.png](44-group-actions-menu.jpg).

## 10. Processing tools, LCF, and PCA

Processing tools are in the Data-stage Parameters panel rather than a separate Tools stage. Align to reference, Calibrate energy, Deglitch, Truncate, Rebin, Smooth, and Difference spectrum were visible during the tour. Analysis tools are farther down the same panel, but the command palette can reach them directly.

Opened **Linear combination fit** through the palette, then **Principal components** from the right panel. Both restored existing saved results without running a calculation.

LCF shows standards, normalized/flattened/derivative/χ(k) choices, range presets, sum-to-one and energy-shift options, and a Fit button. The plot labels the saved interval and explains that contributions and residuals are vertically offset. The listed weights are 0.600 and 0.400 with synthetic persistence-sample names.

PCA shows its training set, component count, range controls, and Train + target transform. The center provides Scree, Cumulative, Error vs count, IND, score plots, Loadings, Similarity, and Reconstruction. Saved results say three training spectra, while the editable form says **Training set: 2 ready**. The display distinguishes saved analysis in small text, but a new tester could confuse existing results with the currently configured inputs. The result lines and range labels are crowded or clipped; the scree plot uses fractional tick labels between integer component numbers.

Screenshots: [45-tools-palette-lcf.png](45-tools-palette-lcf.jpg), [46-lcf-tool-form.png](46-lcf-tool-form.jpg), [47-pca-tool-form.png](47-pca-tool-form.jpg).

## 11. Menu bar, Updates, and Preferences

Opened the macOS rexafs menu. It contains **About rexafs** and **Quit rexafs**, with no general Preferences/Settings entry. Selecting About rexafs opened the application's Help popup containing Open Cu example, Licenses, and Updates, rather than a conventional version/about dialog. ⌘, produced no general Preferences window.

Opened Updates using the toolbar's download icon. The dialog identifies the exact Nightly build and offers **v0.2.11 available**, Release notes, and Download v0.2.11 · 53.4 MB. It explains that the other channel must be installed separately. The same visible version number appears as both current and available, which requires reading the channel/build details to understand.

Expanded **Preferences** inside Updates. It exposes Stable/Nightly and Check on startup. No preference was changed, and no update was downloaded. A standalone general Preferences window was not found.

Screenshots: [48-app-menu-bar.png](48-app-menu-bar.jpg), [49-about-opens-help-menu.png](49-about-opens-help-menu.jpg), [50-preferences-shortcut-check.png](50-preferences-shortcut-check.jpg), [51-updates-view.png](51-updates-view.jpg), [52-update-preferences.png](52-update-preferences.jpg).

## 12. Light theme

Used the sun/moon toolbar control. Captured Data, Wavelet, ordinary Fourier Transform, RMC Results, and the path-fitting Model view in light mode, then switched back to dark.

Text and panel borders are generally clear. The green spectrum has weaker contrast against the white plots. Ordinary plots change theme, but Wavelet's side/lower plots and heatmap canvas remain dark. The native title bar also remains dark; system appearance was not changed. The fit form's narrow Fixed label remains broken across lines in light mode.

Screenshots: [53-light-data.png](53-light-data.jpg), [54-light-transform-wavelet.png](54-light-transform-wavelet.jpg), [55-light-transform-fourier.png](55-light-transform-fourier.jpg), [56-light-fit-rmc.png](56-light-fit-rmc.jpg), [57-light-fit-mode-menu.png](57-light-fit-mode-menu.jpg), [58-light-fit-model.png](58-light-fit-model.jpg).

## 13. Narrow laptop layout

Resized the requested app's window through macOS accessibility to exactly 1100 × 700 points. Captured Fit Model and Data. The Data view retained the PCA form because that analysis panel had been opened earlier.

The stages remain accessible and toolbars wrap instead of entirely disappearing. However, the permanent Groups column, nested fit-spectrum list, form, and plots leave very little space. Fit ranges and weight controls wrap across lines; much of the parameter card falls below the viewport; both fit plots are very short. In Data, the wrapped toolbar and range explanation consume several rows, and the Parameters results extend below the visible area. Opening Journal or Problems at this size further reduces plot height.

The window was restored to its original size for supplemental captures.

Screenshots: [59-narrow-fit.png](59-narrow-fit.jpg), [60-narrow-data.png](60-narrow-data.jpg), [61-journal-click-attempt.png](61-journal-click-attempt.jpg), [62-problems-panel.png](62-problems-panel.jpg).

## 14. Errors, warnings, notices, and end of tour

Captured the encountered conditions without trying to fix the fixture:

- Stable-update notice at startup: **01**. Update details and preferences: **51–52**.
- Restored import receipt, one pending source, warning and lock badges: **14** and subsequent stage captures.
- Fit needs a second spectrum, undefined parameter names, disabled Run fit: **19, 23–24, 58–59**.
- RMC needs a structure/supercell: **27–28, 73**. Empty RMC results/run details: **29–30**.
- Synthetic-average quantity unconfirmed warning after mark inversion: **36–37**. The text says plotting/export are available and quantity must be confirmed before normalization/AUTOBK.
- Problems panel: [62-problems-panel.png](62-problems-panel.jpg). It reports **9 malformed rows skipped**, with example line numbers 620–624, and a long internal project-data path. The panel's **Clear recent / close** control combines two intentions in one label.
- Pending-source recovery: [63-pending-import-resolution.png](63-pending-import-resolution.jpg). Resolve opens **Source unavailable**, **No such file or directory (os error 2)**, Locate source…, Retry, and Review later. The full path is truncated; the source filename remains visible in the title. Escaped without locating or changing a source.

The missing source and saved warnings are fixture conditions. No application crash was observed during the successful tour. ⌘Q ended the requested process without a save prompt. There is no capture 74 because no window remained after quitting.

## UX observations

Ranked from most consequential to least consequential. These are tester observations and review leads, not code-level diagnoses or numerical-validation results.

1. **Saved analysis and current form state can be confused.** PCA displays a saved result for three training spectra alongside a form saying two are ready; LCF displays synthetic saved standards under the current Cu target. The Saved analysis line helps, but the result/form distinction deserves stronger visual treatment. Evidence: **46–47**.
2. **A mixed-energy Series selection can look like valid low-valued data.** Selecting data · 5 frames combines Ru- and Cu-edge inputs; most heatmap rows appear uniformly dark, with no immediate coverage explanation. A user could misinterpret blank coverage. Evidence: **64–65**.
3. **Fit becomes difficult to inspect at 1100 × 700.** Nested columns persist, controls wrap, model parameters disappear below the viewport, and plot heights become too small for comfortable inspection. Scrolling/panel management is necessary but not obvious in the static view. Evidence: **59**.
4. **Fit validation lacks a clear hierarchy.** The restored form simultaneously requires two spectra, shows undefined de0/dr/ss, reports zero free variables, and presents a checked fit control. A new user must infer which issue to resolve first. Evidence: **19, 23–24**.
5. **Command search can promote an unrelated state-changing action.** Searching linear selects Apply all processing settings to marked groups ahead of Linear combination fit. Enter without careful reading could invoke the wrong action. Evidence: **45**.
6. **Numeric presentation exposes excessive precision and truncation.** RMC R min reads 1.349999999…, while normalization's automatic fit maximum is clipped. The UI should present readable values without concealing their meaning. Evidence: **15, 28, 73**.
7. **Assistant substantially compresses the scientific workspace.** Opening it alongside Groups and Parameters leaves a tall, narrow plot even at the default width. Evidence: **38–41**.
8. **Routine parameter headers and labels clip or break.** The group name is squeezed by Apply to 1; LCF/PCA range labels are truncated; Fixed breaks into Fixe / d; PCA summary rows nearly run into the panel edge. Evidence: **15, 23, 46–47, 58**.
9. **The restored import receipt is temporally ambiguous.** Added 2 files reads like a new action after opening the project, and the banner persists across unrelated stages. A history/source indicator would help distinguish saved receipt state from this session's activity. Evidence: **14–73**.
10. **Important tools are buried by a long Source panel.** The Data panel mixes source provenance, channel creation, processing, analysis, and metadata. Opening a project can put LCF/PCA well below the visible area; the palette was easier to use. Evidence: **14, 45–47**.
11. **Fit mode names overlap at different levels.** Path fitting/RMC and Single spectrum/Fit multiple spectra both use Fit mode wording. Their scope is easy to confuse. Evidence: **23, 25–26**.
12. **A structure preview does not clearly mean a selected usable structure.** Structure displays atoms while saying Choose a structure and disabling Use structure. The distinction between the existing path preview and a newly selected structure is not immediate. Evidence: **20, 26**.
13. **RMC empty states leave large unused areas.** Settings occupy a narrow left column while the rest repeats a prerequisite message. Results exposes many tabs and actions despite no run. A more direct next-step explanation would reduce exploration. Evidence: **27–30, 73**.
14. **Non-image Publish modes retain image-oriented controls and preview.** CSV, Analysis folder, and Markdown do not visibly preview their actual output structure/text; dimensions and figure styling remain prominent. Evidence: **68–70**.
15. **Series overview trends and saved Results use different concepts without much explanation.** An edge-energy trend is already visible, but Results says no saved trends. Evidence: **65, 67**.
16. **Current, marked, shown, compared, scan, and filter-result states are hard to distinguish.** A count such as Compare 3 does not by itself explain why only one trace is easily recognizable; group menus introduce several more selection scopes. Evidence: **14, 36, 42, 44**.
17. **Long group names lose identifying information.** Ellipsized fixture names and synthetic-result names conceal their distinguishing portions and compete with warning/lock icons. Evidence: **14, 42–47**.
18. **Warning details expose internal paths before useful context.** The Problems panel leads with a long project-data path. A concise group/source label and a visible way to reveal the full path would be easier to scan. Evidence: **62**.
19. **Clear recent / close combines dismissal and history clearing in one label.** It is unclear whether simply closing the panel preserves the warning list. I closed through the status control instead. Evidence: **62**.
20. **About and Preferences discovery differ from macOS expectations.** About rexafs opened a Help popup; the application menu has no general Preferences entry; ⌘, did nothing; update preferences are nested inside Updates. Evidence: **48–52**.
21. **The updater presents the same version as current and available.** The Nightly/local-test build and stable-channel difference is visible but requires interpretation; the main call to action just says Download v0.2.11. Evidence: **51–52**. This may be specific to the supplied test build.
22. **Theme behavior is inconsistent across plot families.** Ordinary plots become light while Wavelet plots stay dark. The green data trace also has weaker contrast on white. Evidence: **53–55**.
23. **Wavelet axes are visually indirect and crowded.** The heatmap's axes are explained mainly by neighboring plots; the side plot's horizontal tick labels are tightly packed. Evidence: **18, 54, 72**.
24. **Forward FT and Wavelet controls coexist without immediate relationship guidance.** Two k-weight values and differing k limits are visible; users need to understand which plot each affects. Evidence: **72**.
25. **Compact scientific terms assume considerable prior knowledge.** rbkg, dk, MBACK, Reff, de0, dr, ss, IND, and Typst are not explained in the immediate forms. Hover help may exist, but the screenshots do not establish it. Evidence: **15–17, 22–23, 47, 68**.
26. **The import mapping summary is awkwardly phrased.** μ column: μ(E) = 2: mu requires interpretation of the index and label, while the editable mapping controls begin collapsed. Evidence: **12**.
27. **An ordinary import appears under Results.** This weakens the distinction between source measurements and derived outputs for a new user. Evidence: **13**.
28. **Some controls lack visible usage cues.** The inline rename editor has no commit/cancel hint; several toolbar and stage-status icons depend on discovery by hover or prior knowledge. Evidence: **01, 43**.
29. **Some plots allocate space poorly for the shown fixture.** The Background spline spike expands its vertical range, the PCA scree x-axis uses fractional labels for component numbers, and dense legends occupy substantial plot area. These are presentation observations, not numerical-error findings. Evidence: **16, 46–47**.
30. **Small grammatical inconsistencies reduce polish.** Examples include 1 warnings, 1 spectra, 1 sources, and Try 1 components. The unfiltered command list also lacks a displayed ⌘7 hint beside Publish while earlier stages show theirs. Evidence: **02, 14, 22–23, 47**.
