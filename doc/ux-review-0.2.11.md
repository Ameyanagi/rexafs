# Desktop UI/UX review — 0.2.11

Review date: 19 September 2026. Reviewed build: rexafs 0.2.11 (`target/release/rexafs`
built from commit `c09ac25`, the same source as the signed 0.2.11 release) on
macOS 26.5, ARM64, dark theme, 2000 × 1294 logical pixels. This is a proposal
document. Nothing in it changes released behavior; each item is a recommendation
for the maintainer to accept, reject or reschedule.

## Scope and method

The numerical and scientific behavior of 0.2.11 is out of scope; the request was
to review how the desktop interface presents that behavior.

Evidence came from four sources:

1. **Fresh captures of 0.2.11.** A Codex computer-use session drove the built app
   with isolated settings (`REXAFS_SETTINGS` pointing at an empty file) through
   the empty workspace, command palette, import, every stage, the Fit steps, RMC,
   Series, Publish, panel toggles and the light theme. Captures are retained in
   [validation/2026-09-19-ux-review/](validation/2026-09-19-ux-review/README.md).
   They used only public fixtures: `rexafs-0.2.11-embedded.rxs` and `cu_150k.xmu`.
2. **Published captures** for 0.2.10, 0.2.11 and the unreleased "next" set under
   `website/public/screenshots/`. Older website captures (`welcome.jpg`,
   `fit-result.jpg`, `paths.jpg`, `publication.jpg`) predate 0.2.10 and were used
   only where the fresh captures showed the same layout.
3. **Source review** of `crates/rexafs-gui/src` (about 115 000 lines). Findings
   cite `file:line` at commit `c09ac25`. The review was split across the shell,
   the Fit/RMC/structure workspace, and the import/tools/series/publish/assistant
   areas.
4. **Prior design records**: [gui-ux-design-v2.md](gui-ux-design-v2.md),
   [ui-simplification-plan.md](ui-simplification-plan.md),
   [fitting-workspace-redesign.md](fitting-workspace-redesign.md),
   [rmc-desktop-workflow-plan.md](rmc-desktop-workflow-plan.md) and the
   validation READMEs since September 7. These define what the interface was
   meant to be, so this review measures the build against its own stated goals.

Limits: macOS only; no Windows or Linux session; no long RMC or FEFF runs were
started; no screen-reader session. Code findings that were not exercised in the
app are marked "code only".

## Summary

The 0.2.11 interface is coherent and fast. The pipeline strip, draggable range
handles, the "next action" button at the upper right of the Fit workspace, the
command palette and the empty-state card all match the v2 design principles. The
problems are not structural. They fall into six recurring patterns that repeat
across every stage, and fixing the pattern once fixes it everywhere:

| # | Pattern | Effect on the user | Where it shows |
|---|---|---|---|
| 1 | **One untyped status string** carries every error, success and progress message | Errors look like successes, get overwritten, and never draw attention | Status bar, invalid input, locked groups, tool errors, export results |
| 2 | **Raw program output leaks into labels** | Users see `1.349999999`, `FitWarning { … }`, `Stopped: Converged`, a 16-hex path id | RMC settings, Fit results, MCR, peaks, joint fit |
| 3 | **Text is too small and too grey** | 11 px is the dominant chrome size; 10–10.5 px labels in muted grey fail AA contrast in the light theme | Every panel |
| 4 | **Labels, casing and controls are inconsistent** | Four "selected" idioms, lowercase field labels next to Title Case menus, two things called "Fit mode" | Inspector, tools, Fit, Publish |
| 5 | **Fixed pixel widths clip content** | "cu_150(" inspector header, "Fixe d", "auto (spectr", middle-truncated group names | Inspector, joint fit, group list |
| 6 | **Explanations live in tooltips or paragraphs, not next to the control** | New users cannot tell what k-weight, Rbkg, Reff, "marked" or "Auto moves" mean without hovering | All stages, RMC, tools |

The ten highest-value changes, in recommended order:

1. Replace the bare status string with typed notices (severity, source, expiry) and
   use them for every error, progress and success message.
2. Format every number for display: significant figures from the uncertainty or
   the unit, never the raw `f64` and never Rust `Debug` output.
3. Raise the typographic floor to 12 px for interactive labels and 11 px for
   everything else, darken light-theme muted text to pass AA, and make the
   wavelet panels follow a theme toggle (the only reproducible light-theme
   defect; see 3.1).
4. Make the inspector resizable and never clip its header; put the "Apply to N"
   control on its own row.
5. Rename and merge the duplicated "Fit mode" controls; move the blocking reason
   next to the disabled primary button on every step.
6. Add inline validation under fields (invalid value, bound, unknown variable)
   instead of a status-bar line, and stop auto-creating variables from typos.
7. Add a persistent plot toolbar: reset view, cursor readout, legend toggle, and a
   one-line hint that the blue tabs are draggable.
8. Add a Preferences dialog (⌘,) that owns theme, text size, update channel,
   assistant defaults and structure-source paths, and persist the theme.
9. Give Series real frame navigation (first/previous/next/last, a frame field,
   play) and fix the fractional frame axis.
10. Adopt one label table and one "selected" idiom across the shell.

Sections 1–13 give the evidence and the proposed change for each finding.
Section 14 groups them into a delivery plan.

## 1. Shell, status and feedback

**Layout as built.** Top bar (project name, undo/redo, Import…, save, theme,
search, panel toggles, assistant, updates, help); seven equal-width stage tabs
with a status dot; Groups panel on the left (resizable 240–400 px); the stage's
center; a fixed 312 px inspector on the right; optional docked Assistant; optional
Problems and Journal drawers; a 28 px status bar
(`app/shell/mod.rs:442-460`, `app.rs:9752-9849`).

### 1.1 The status bar is the only notice channel (high)

*What the user sees.* Every message, from "Exported spectrum CSV" to "Processing
is locked" to "Stable update available — open Updates to review it.", is written to
one `SharedString` (`app.rs:1328`) and rendered at 12 px muted grey at the bottom
left (`app.rs:9768-9777`). It never clears, is silently overwritten by the next
message, has no severity colour, and is truncated without an ellipsis. In
capture 01 the update notice is plain text with nothing to click. In capture 24
the reason the fit cannot run ("Add at least two spectra in Spectra & paths.")
sits in the bottom right corner of the Results panel while the disabled button is
on the Model step.

*Why it matters.* Errors are indistinguishable from successes; a user who looks
away misses them; and the app's own design contract says errors and affected
counts must stay visible when they affect a decision
([ui-simplification-plan.md](ui-simplification-plan.md)).

*Proposed change.* A `Notice { severity, text, action, timestamp }` list with a
severity colour, an optional button ("Open Updates", "Show problems", "Undo"), a
short auto-expiry for successes, and a "recent notices" popover. Route the ~60
`self.status = …` sites through it. Keep the status bar for the current-group
summary only.

### 1.2 Developer metrics in the status bar (medium)

`jobs:0 cache:3/1024` (`app.rs:9778-9783`) is visible in every capture. Neither
number helps a user. Replace with a named progress item ("Fitting cu_150k…",
"Calculating paths 3/9") and keep the cache counter behind `REXAFS_DEBUG_STATS`.

### 1.3 Errors do not attract attention (medium, code only)

`record_job_error` only increments the "0 errors · 1 warnings" counter
(`app.rs:3256-3270`); the Problems drawer must be opened by hand. Post a notice
and pulse the counter when a new problem arrives.

### 1.4 Problems drawer shows internal paths (low)

Capture 62: the only warning reads "Warning:
/Users/…/.rexafs/project-data/12afd916…/raw/import-diagnostics.dat · 9 malformed
rows skipped". That is the embedded copy inside the project store, not the
user's file. Show the group name and original source path, and keep the store
path behind "Reveal".

### 1.5 Receipt bar wording and icons (low)

Capture 14: "Added 2 files → 2 groups · 1 needs mapping · 1 warnings" followed by
an eye icon, an info icon and an ×, none labelled. Use "1 warning", label the
icons ("Show details", "Dismiss"), and make "1 needs mapping" the button.

## 2. Navigation and discoverability

### 2.1 Stage status dots have no legend (medium)

Four colours (`app/shell/mod.rs:233-241`: green Ok, blue Auto, yellow Attention,
grey Idle) are never explained. The Fit tab turns yellow in capture 19 with no
visible reason. Show the reason in the tab tooltip's first line, and add a legend
row to Help.

### 2.2 Features reachable only by shortcut or right-click (medium)

- ⌘P focuses the group filter (`app.rs:577-581`); the filter box gives no hint.
- Space, F2, Enter, ←, → act on group rows (`app.rs:514-548`) with no visible hint.
- Field-level "reset / copy to marked" actions open only on right-click
  (`app/shell/parameter_actions.rs:407-425`); the row has no affordance.
- The Help menu has three items and no shortcut list (`app/shell/help.rs:161-166`).
- The palette omits ⌘7 for Publish although it is bound (`palette.rs:163`,
  `app.rs:566`).

Add a "Keyboard shortcuts" sheet to Help, show the ⌘P hint in the filter
placeholder, and reveal a small "⋯" on field hover.

### 2.3 Plot interaction has no affordance (high)

Range handles are 7 × 11 px tabs above the axis (`app/shell/handles.rs:603-617`),
explained only in a tooltip on section headings (`inspector.rs:437-450`). Zoom is
right-drag and "Reset view" is in a right-click menu supplied by the plotting
toolkit; rexafs adds no button. There is no cursor readout on processing plots and
the legend strip is not clickable (`app.rs:9153-9180`).

Add a plot-card toolbar: reset view, cursor x/y readout, legend on/off, and a
first-run caption "Drag the blue tabs to change the fit range". Colour the range
fields' left border with the handle colour so field and handle read as one thing.

### 2.4 Panel toggles change meaning per stage (low)

⌘J is removed on Fit and Publish and does nothing visible on Series until an
overview exists (`app/shell/mod.rs:500-504`, `:637`). Keep the button, disable it,
and say why in its tooltip.

## 3. Typography, contrast and density

Measured in `crates/rexafs-gui/src`: 118 uses of 11 px, 34 of 10.5 px, 23 of
10 px and one of 9 px text; base body text is 12.5 px (`app/shell/mod.rs:438`).
Section labels are 10.5 px uppercase (`mod.rs:250`); hints 10 px
(`inspector.rs:253, 286`); stepper glyphs 7 px inside 14 × 11 px targets
(`widgets/numeric_field.rs:319-335`); wavelet plot font 8.5 px
(`app/shell/wavelet/plots.rs:16`).

Theme values (`theme.rs:43-72`): dark muted `#8a909c` on surface `#1d2026` is
about 5.1:1 and on raised cards about 4.5:1; light muted `#6e7681` on surface
`#fafafa` is about 4.4:1 and on background `#f2f3f5` about 4.2:1. WCAG AA requires
4.5:1 for normal text, so light-theme muted labels fail and dark-theme card labels
sit at the floor. Almost all sub-11 px text uses the muted colour.

Proposed change: 12 px minimum for interactive labels, 11 px for captions, no
muted colour below 11 px, light muted about `#5c636d`, steppers at least 20 px
tall, and a text-size preference (see 12.2).

### 3.1 Light theme: wavelet panels keep the old theme (medium; corrected finding)

Revision note, 19 September 2026: the first version of this section reported that
the whole window chrome stayed dark after a theme toggle (captures 53, 55, 56 and
58). That state did not reproduce on the untouched 0.2.11 binary in a controlled
session with a single rexafs process: after one toggle the top bar, stage strip,
Groups panel, inspector, status bar and ordinary plots all switched. During the
original tour several rexafs processes were running, so the menu action and the
capture most likely targeted different processes. The captures are retained but
should not be read as evidence of a chrome defect.

The reproducible part is narrower: the Transform stage's wavelet map, its two
marginal plots and its four numeric fields keep the theme they were built with
(dark canvases on a light window and the reverse), because the toggle rebuilt
every other plot and field but not those (`app.rs:6097-6130`,
`app/shell/wavelet/plots.rs`). Rebuild the wavelet plots and fields in the toggle,
and persist the chosen theme so it survives a restart.

## 4. Consistency

### 4.1 Label casing and wording (medium)

Field labels are lowercase ("pre-edge start", "rbkg", "fft k min", "victoreen n",
`app.rs:4056-4137`) while the copy/reset menu Title-cases the same settings
("Pre-edge start (eV)", "Rbkg (Å)", `app/shell/parameter_actions.rs:28-87`).
Chips mix "norm"/"flat" with "XANES"/"Full spectrum" (`center.rs:669-673`). Tool
hints are lowercase fragments ("shift onto a named alignment standard",
`tools.rs:101-113`). Path-picker headers are "path · Reff · N · legs · amp"
(`path_picker.rs:181-186`). Adopt sentence case everywhere and keep the label text
in one table shared by fields, menus, tooltips and the stage strip.

### 4.2 Four idioms for "selected" (medium)

`chip`, `segment`, the bespoke k-weight tabs (`center.rs:914-963`) and
`button(primary = true)` are all used to show the selected option; primary also
means the main action ("Apply", "Run fit"). In capture 15, "Polynomial" is a
primary button and "Full spectrum" is an outlined chip for the same kind of choice.
Use segments for exclusive choices, chips for toggles, primary for the one main
action per view.

### 4.3 Two "Fit mode" controls (medium)

Capture 23 shows "Fit mode: Path fitting ▾" at the top right and a "Fit mode
Single spectrum | Fit multiple spectra" bar below it (`rmc.rs:94`,
`joint_fit.rs:333`). Rename to "Method: Path fitting / RMC" and "Spectra: Single /
Multiple".

### 4.4 Number formatting (high)

- `format_value` prints the raw float when no decimal count is configured
  (`widgets/numeric_field.rs:97-104`); capture 28 shows R min as `1.349999999`
  after "Use spectrum ranges".
- Fit results use a fixed five decimals with no units (`fit.rs:1059`); E₀ in eV
  and σ² in Å² look alike.
- E₀ is shown with one decimal in the stage strip, two in its tooltip and one in
  the result card; edge step with three, five and four (`stage_strip.rs:331-334`,
  `:132-136`, `inspector.rs:783`).

One formatter per physical quantity, with significant figures derived from the
uncertainty where one exists, and units in a fixed column.

### 4.5 Debug output shown to users (high)

- Fit warnings are rendered with `format!("⚠ {warning:?}")` (`fit.rs:1139`), but
  `FitWarning` is a struct with `symbol`, `inferred_from`, `default_value` and a
  human `message` (`crates/rexafs/src/xafs/fitting/types.rs:1172-1181`). Users will
  see `FitWarning { symbol: "…", … }`. Print `message`.
- MCR termination `"{:?} · {} iterations"` (`tools.rs:2291`); peaks
  `"{:?} μ(E)"` axis labels, `"{:?} · objective …"` status and `"{} · {:?}"`
  component shapes (`peaks.rs:857, 901, 1414, 1629`); stage tooltip
  `format!("{:?}", p.import.mode)` and fit space (`stage_strip.rs:128, 267`).
- The joint editor shows a path identity hash `115e273b7599b6ed` beside the path
  name (capture 23).

Implement `Display` for those enums and hide the hash behind a tooltip.

### 4.6 Glyphs instead of icons (low)

"☑/☐" in button labels (`fit.rs:810-831`, `import_editor.rs:1607`), "✓" at
9–10 px, "▸/▾", "⋯", "⚠", and an emoji lock 🔒 in group rows
(`groups_panel.rs:974-978`). The emoji renders differently per platform. Use the
existing SVG icon set.

## 5. Layout and clipping

### 5.1 Inspector header clips (high)

Captures 15 and 17: once "Apply to 1 · excludes current · 1 locked" appears, the
group name collapses to "cu_150(". The inspector is a fixed 312 px
(`app/shell/assistant_shell.rs:50`, `inspector.rs:143`) and is not resizable. Put
"Apply to marked" on its own row under the name, and let the panel resize like the
Groups panel does.

### 5.2 Auto-value placeholders clip (medium)

"auto (8981.0", "auto (spectr" in captures 15 and in the 0.2.10 assistant capture;
the field is 98 px (`numeric_field.rs:336-412`). Show "auto" in the field and the
resolved value as a right-aligned muted suffix or tooltip, or widen the field.

### 5.3 Wrapped and truncated labels (medium)

- "Fixed" wraps to "Fixe / d" in a 26 px cell (`joint_browser.rs:532`, capture 23).
- Group names are middle-truncated ("Fixture i…stics.dat", "Synthetic …nconfirmed")
  even at 380 px (`groups_panel.rs:845-855`); the tag column is fixed.
- Variable names truncate at 56 px (`fit.rs:550-558`); template chips are 164 px
  (`fit.rs:666`); Publish's style column is 312 px (`publish/editor.rs:472`).

Replace fixed widths with min-width plus flex, and show the full name on hover.

### 5.4 Narrow window (1100 × 700) (medium)

Captures 59 and 60: the layout survives but the plot toolbar wraps to two rows,
the Fit "Fit in" chips wrap under the k-weight row, the joint parameter table is
cut off below "This spectrum", and inspector labels truncate to "range start
(re…". The Groups panel keeps a quarter of the width. Collapse the Groups panel
to icons below about 1200 px, wrap labels above fields at narrow inspector
widths, and let the parameter table scroll inside its card.

### 5.5 Fixed-height bottom drawers (low, code only)

Problems (180 px) and Journal (150 px) stack with the status bar to 358 px, none
resizable (`app.rs:9646`, `journal.rs:797`). At the 960 × 640 minimum window the
plot is about 150 px tall. Merge them into one tabbed, resizable drawer.

### 5.6 Empty-state inspector (low)

Capture 01: with no data, the inspector still lists every tool, and two of them
("Fluorescence correction…", "XANES peak fit…") are full-width buttons while the
rest are chevron rows. Hide the tool list until a group exists, or show one row
style.

## 6. Data stage and import

**As built.** Import starts from the top bar, the Project menu, the Groups "+"
or a drop. Files are read in the background; a receipt bar summarises; sources
that need a decision open a full-window modal editor
(`app/shell/import_editor.rs`). Provenance (source, channel, mapping, energy
correction) is shown at the top of the inspector.

### 6.1 Provenance before controls (medium)

Capture 13/14: the inspector opens with six lines of provenance prose ("Mapping:
detected or manually assigned · no reusable recipe", "Spectrum checked · 618
points") before the energy-offset field and the tools. The design contract puts
provenance behind an explicit action. Collapse it into a "Source" disclosure that
opens closed, and keep the offset and tools first.

### 6.2 Silent column guesses (high, code only)

"Custom" mapping fills `mu = 1, i0 = 1, it = 2, ir = 3` with no "guessed" marker
(`import_editor/measurement_view.rs:81-87`, `measurement_import.rs:340-344`). A
wrong but plausible plot appears. Show roles as "Choose…" with a "guessed" badge
and require confirmation only for guessed roles.

### 6.3 Hidden Apply gate (medium, code only)

Apply is blocked with "Choose the main spectrum above." until the user clicks a
"Main spectrum" tab, even when one is suggested (`import_editor.rs:1167-1169`,
`:1933`). Accept the suggestion by default.

### 6.4 Repair flow is long and stalls (medium, code only)

Edit → "This channel in its import batch…" → "Validating…" → "Revalidate" → "Apply
to N files" (`import_editor.rs:1970-2053`), and any edit invalidates the previous
validation. Validation loads every file without progress or cancel
(`app/import_repair.rs:131-178`). Debounce revalidation and show N/M progress.

### 6.5 Cancel during import does nothing (medium, code only)

Close and Cancel stay enabled while "Importing…" but `close()` never cancels the
batch (`import_editor.rs:667-700, 843-847`). Wire the existing `intake_cancel` or
disable the buttons.

### 6.6 Group-panel diagnostics repeat themselves (low)

Capture 36: the note under the group list reads "Synthetic average · quantity
unconfirmed: Quantity unconfirmed: confirm the quantity before
normalization/AUTOBK; plotting/export available." The label suffix
(`params.rs:1773`) and the message both carry the phrase. Show the message once
and make "confirm the quantity" a link to the confirmation control. Capture 63's
review dialog is a good pattern (clear title, primary "Locate source…"), except
that it prints the raw "No such file or directory (os error 2)" text.

### 6.7 Jargon in the import editor (low)

"I0/It/Ir", "μ column: μ(E) = 2: mu", "ROIs", "First-order Bragg conversion",
"Freshness not recorded; reload to check", "The bounded header read did not reach
numeric data" (`import_editor.rs:1664-1729`, `import_state.rs:67`,
`importing.rs:415`). Give each detector role a tooltip and rewrite the pending
reasons as plain sentences ("The file has no numeric rows in its first 64 lines").

## 7. Processing stages: Normalize, Background, Transform

### 7.1 Invalid input has no message (high)

A rejected value flashes the field border for 1.4 s and reverts
(`numeric_field.rs:18, 170-190`); a semantic rejection goes to the status bar
(`app.rs:4217-4222`). Locked groups reject edits the same way
(`app.rs:4176-4184`) and the inspector shows no lock state. Show a one-line reason
under the field and a "Processing locked · Unlock" banner in the inspector.

### 7.2 Commit patterns differ (medium)

Numeric fields commit on Enter/blur and recompute; k-weight and enum rows apply on
click; MBACK needs "Apply" and shows "Settings changed · Apply to update" in grey
(`normalization.rs:194, 616, 648`). Highlight the Apply button when dirty, and
mark pending controls visually.

### 7.3 Three k-weights that look the same (medium)

The v2 design named this as an Athena pain point to avoid. Capture 72 shows
"k-weight 3" under Forward FT and "k-weight 2" under Wavelet settings in the
same panel; Background has "bkg k-weight"; Fit has its own. Prefix each with its
purpose in the label ("Plot k-weight", "AUTOBK k-weight", "Fit k-weight") and show
the link state ("linked to Transform") where one exists.

### 7.4 Transform view bar (low)

Capture 17: "k R k+R q Wavelet | Compare 3 | k-weight 0 1 2 3 | Re | Window" runs
as one row with no separators between groups. Group the view segments and the
overlay toggles visibly, and move "Wavelet" into the view segment set with the
same style.

### 7.5 Section overrides (positive)

The "override ✕" chip on section headers and the "Modified" suffix on Back FT
(capture 17) are clear. Keep this pattern and use it for locks too.

## 8. Fit workspace (path fitting)

**As built.** Step chips Structure → Calculate → Paths → Model → Results with ✓
marks; a primary "next action" button at the upper right; a status strip
("Data · cu_150k.xmu · 1 / 62 paths · 4 free variables"); a side panel with the
library, calculation setup, path picker or parameter tabs
(`app/shell/fit_workspace.rs:175-692`).

### 8.1 Opening a saved project lands in an error state (high)

Capture 19: the first thing the Fit stage shows for the fixture project is the
joint-fit editor with "Undefined: de0 / dr / ss" in orange and "Add at least two
spectra in Spectra & paths." This is the retained fixture, not a user project, but
any project whose expressions reference removed variables will look like this.
Open on the last completed step, show undefined variables as a single banner with
"Create missing variables", and do not switch the user into multi-spectrum mode
when only one spectrum is assigned.

### 8.2 Blocking reasons are far from the disabled button (medium)

The reason sits in the header line (capture 20: "Choose a structure") or at the
bottom of the Results panel (capture 24), while the disabled button is elsewhere.
Render the reason as the disabled button's tooltip and directly beneath it.

### 8.3 No cancel or progress for fit and FEFF (medium, code only)

`run_fit_now` only sets `fit_running` (`app.rs:7210-7326`); the FEFF calculation
has no cancel (`app.rs:8449-8573`) and its status line is found by string prefix
(`fit_workspace.rs:436-453`). Add an explicit running flag, an indeterminate bar
and a Cancel that bumps the generation counter.

### 8.4 A typo silently creates a variable (high, code only)

A value that does not parse is treated as an expression and every identifier in
it becomes a variable at 0 (`app.rs:6951-6975, 6992-6996`). Typing `sigma_1` for
`sigma2_1` adds a free parameter with no message. Highlight unknown identifiers
and ask "Create variable sigma_1?".

### 8.5 Bound errors go to the status bar (medium, code only)

"invalid min bound for dr_1" is written to the status string
(`app.rs:6928-6933`) while the field keeps the bad text. Use the inline error
state the joint editor already has (`joint_browser.rs:74`).

### 8.6 Starting values are overwritten after a fit (medium, code only)

Fitted values replace the guesses (`app.rs:7297-7307`) and the view jumps to
Results; the originals survive only in History. Show guess and fitted value side
by side, or a one-time notice with "Restore starting values".

### 8.7 Results readability (medium)

The published `fit-result.jpg` and the code agree: statistics are listed as
"R-factor, χ², reduced χ², N idp / N var, ε k (noise), objective" with no
thresholds or definitions (`fit.rs:982-1018`); internal names `dr_1`, `e0`,
`s02`, `ss_1`; a single expandable History row and no side-by-side comparison
(`fit.rs:1280-1348`); correlations only above 0.9 (`fit.rs:1101-1125`); export is
"Copy as Markdown" only. Add per-row tooltips ("R-factor: fractional misfit;
below 0.02 is typical for a good first-shell fit"), display names with units, a
compare table across History entries, and "Save report…".

### 8.8 Sliders toggle can hide the only working surface (low, code only)

On Structure and Calculate the side panel is the whole form; the toggle hides it
and the header still says "Choose a structure" (`fit_workspace.rs:463-465`).
Exempt those steps.

### 8.9 Results step never offers "Run fit" (low, code only)

Results always shows "Edit model →" (`fit_workspace.rs:262-267`); a stale result
says "press Fit to refresh" but no button is named "Fit" (`fit.rs:56`). Show "Run
fit again" when stale.

## 9. RMC (reverse Monte Carlo) mode

**As built.** Numbered steps 1 · Structure, 2 · Supercell, 3 · Fit settings,
4 · Results; "Recover latest run" and the mode picker at the top right; "Preview
initial fit" and "Run RMC →" beneath; Results has tabs Fit plots · Structure ·
Convergence · Run details · Refinement (`app/shell/rmc.rs:726-1089`).

### 9.1 Numbers and empty space (medium)

Capture 28: R min `1.349999999` (see 4.4); the right half of the page holds one
sentence ("Choose a structure and build its supercell.") while the form scrolls on
the left. Show the supercell preview or the spectrum's χ(k) with the fit window in
that space.

### 9.2 Buttons enabled without results (low)

Capture 29: "Refine best…" and "Open checkpoint…" are enabled on an empty Results
page; "Export result…" is disabled. Disable all three until a run exists, and
merge "Recover latest run", "Open RMC checkpoint…" and "Open checkpoint…"
(`rmc.rs:755, 815, 1519`) into one "Open run…" menu.

### 9.3 Explanations as stacked paragraphs (medium)

The published run-details capture is a wall of 11 px text under eight section
labels; the settings page interleaves a sentence under most controls ("Auto
adjusts the starting move size and cools toward improvements only."). Move the
explanations into tooltips or a collapsible "About these settings", and present run
details as a two-column table.

### 9.4 Mislabelled and mispositioned controls (low, code only)

"Move size" sits under "Auto moves" though Auto adjusts it (label it "Starting
move size"); "CPU workers" sits under "Run budget"; `FIELD_LABELS[26]` is "Update
interval" while the field reads "Every N attempts" (`rmc.rs:1274` vs `:2347`), so
validation messages name a control that does not exist. Fields are addressed by
index (`fields[8]`, `fields[27]`), which is how labels drift.

### 9.5 Progress before the first move (low, code only)

"Preparing potentials and exact scattering paths. This may take several minutes;
you can stop safely." is text only (`rmc.rs:165`); the bar appears after progress.
Show an indeterminate bar from the start.

## 10. Structure library and 3D viewer

### 10.1 Viewer controls hidden in a popup (medium)

Captures 20 and 22: the canvas shows "Center focus", "Depth cue" and a sliders
icon. Camera presets, atom style, bonds, labels, zoom and the interaction hint
"Drag rotates · wheel zooms · click inspects" are inside the popup
(`structure_view.rs:1465-1915`). The RMC scene has a different bar and hint
("Reset view"). Give both a shared mini toolbar (style, view presets, zoom, reset)
and show the drag/zoom hint on the canvas.

### 10.2 Text overlay in the canvas (low)

"Depth: back ●●●● front · follows rotation", "Auto · 1 displayed bonds", "FEFF
cluster · 2 atoms · 1 shells · outermost atom 2.5 Å" (captures 20, 22). Pluralise
correctly, and move the depth legend into the appearance popup.

### 10.3 Online sources fail late (medium, code only)

Materials Project without a key shows "not set — free key at …" but Search stays
enabled and the provider error is echoed verbatim (`structure_view.rs:2382,
2507-2513`); "No structures found" appears before any search (`:2515-2521`).
Disable Search with the reason inline; map network failures to "You appear to be
offline".

## 11. Tools, analysis, Series, Peaks, Publish

### 11.1 Tools (medium)

- Tool hints are tooltip-only lowercase fragments (`tools.rs:101-113`). Show one
  sentence as a caption inside the open form.
- Align edits the current group while every other tool creates a derived group
  (`tools.rs:1493-1521`, `:1698`); nothing says which. Badge each tool "Edits
  current group" or "Creates new group".
- Derived names are inconsistent abbreviations: `align: A → B (+0.35 eV)`,
  `diff: A − B`, `smooth: A (σ 1.00 eV)`, `truncate: A` (`tools.rs:366-423`). Use
  "Tool · source · key parameter" for all.
- LCF/PCA/MCR take "marked groups" as input with no way to mark from the form
  (`tools.rs:1976-1988`); the empty-state error is "mark at least two standards".
  Link to the Groups panel and explain marks once.
- "Train + target transform" (`tools.rs:145`) and "Apply → new group" as button
  labels; "Add to Groups" for LCF results lives in the plot bar, not next to the
  result table (`center.rs:294`).

### 11.2 Series (high)

- The published `series-difference.jpg` shows a two-frame series with a frame axis
  labelled 0, 0.2, 0.4 … 1 and a trend plot with x from 0 to 1. Frames are
  integers; the axis must be integer-ticked.
- No previous/next/first/last, play, or frame field; only a 96-segment click
  strip (`series.rs:544-576`) and keyboard bindings documented in a help tooltip.
  Add a transport row with the shortcuts printed on it.
- "white line" has no unit or definition (`series.rs:656-667`); the trend list says
  "Edge energy" while the plot says "E₀ (eV)". Series colorbars have no title
  (`plotting.rs:974`) and "Auto" silently swaps palettes in difference mode.
- Captures 31, 64 and 67: the Series empty state offers "Live acquisition…" and
  "Select series or scan" with no sentence saying what a series is or that
  groups must be marked first; the selection page lists "data · 5 frames" with
  "Use marked groups / Use all groups"; the Results page is an empty area with
  "No saved trends. Choose Add trend to start." and the "Add trend…" button at
  the bottom-left corner. Choosing the series also changed the current group to
  Ru_QAS.dat in the status bar with no notice, which is the "silently switching
  when the current group changes" behavior the v2 design ruled out. Add a
  one-line definition, put "Add trend…" in the empty area, and keep the current
  group unless the user picks a frame.

### 11.3 Peaks (medium, code only)

Components cannot be placed by clicking the plot (`peaks.rs:962-971`); results are
prose with `{:?}` leaks; every run is named "XANES peaks" (`peaks.rs:220, 622`).
Add click-to-place, a result table, and run naming.

### 11.4 Publish (medium)

The published `publication.jpg` and `publish/editor.rs` agree with the 7 September
audit item that is still open: a single long column mixing figure list, style,
title, Typst-syntax axis labels (`$abs(chi(R))$ ($"Å"^(-3)$)`) and axis limits.
Format chips silently change export scope (PNG/SVG/CSV export one figure;
"Analysis folder"/"Markdown" export the current plus marked groups,
`editor.rs:354-374`) under the same "Export…" button. Preview always renders the
light theme.

Proposed: two labelled groups ("Figure export" / "Bundle export") with verb-object
buttons ("Export figure as PNG"), a preset row (journal single column, double
column, slide), "Copy style to all figures", plain-text labels with a "Typst"
toggle for advanced users, and a 1:1 preview toggle.

## 12. Assistant, preferences, updates, theme

### 12.1 Assistant (medium)

Positive: the docked panel, suggested prompts ("Check processing", "Explain this
spectrum") and the Access menu with lock icons are clear (0.2.10 captures). Open
items: with no Codex CLI, the panel shows a red line, an unrelated "Retry", a
plain "Retry connection" and a separate "Install Codex CLI…" button while the
composer stays enabled (`assistant.rs:3239-3263`); "Edit analysis" pre-authorises
the whole turn with only after-the-fact receipts (`assistant.rs:2075-2078`);
the settings popover mixes Copy conversation, Plot images, Web search and Shared
context with two long privacy paragraphs. Present one setup card when Codex is
missing, state "applies immediately; Undo in the receipt" under Edit analysis,
and split the popover into Conversation and Privacy sections.

### 12.2 There is no Preferences dialog (high)

Settings are scattered: update channel and startup check in the Updates modal
(`updates_view.rs:574-645`); assistant model, reasoning, web search and access in
the composer popover; Materials Project key, CIF library and AMCSD path in the
Fit-stage structure panel (`structure_view.rs:236-244`); recipes in the import
editor; `assistant_history_limit` has no UI at all (`settings.rs:57`). The theme
is not persisted: `UserSettings` has no theme field and startup reads only
`REXAFS_THEME` (`settings.rs:32-58`, `app.rs:2975`), so every launch reverts to
dark. There is no text-size or UI-scale setting.

Add a Preferences window (⌘,) with General (theme, text size, startup), Assistant,
Structure sources and Updates, and persist every value it shows.

### 12.3 Updates (low)

The update icon in the top bar is drawn with the same blue box as the active
panel toggles (capture 01), so "update available" reads as "panel on". Use a badge
dot instead. The status-bar notice needs a button (see 1.1). Capture 51: a
0.2.11 nightly build shows "v0.2.11 available" with a "Download v0.2.11 · 53.4
MB" button and, beneath it, "Install the other channel separately; this app
keeps its current channel." A same-version stable release should read "Stable
0.2.11 is the same version; switch channels to install it", and the download
button should be secondary.

## 12A. Further observations from the scripted tour

The Codex tester's notes are retained beside the captures as
[codex-tour-report.md](validation/2026-09-19-ux-review/codex-tour-report.md).
They are tester impressions, not diagnoses. The items below were checked against
the captures and are not covered elsewhere in this review.

- **Palette ranking promotes a state-changing action.** Capture 45: typing
  "linear" ranks "Apply all processing settings to marked groups" above "Linear
  combination fit…", so Enter applies settings to marked groups. Rank exact and
  prefix matches on the title first, and never rank a write action above a
  navigation or open action for a partial match.
- **Mixed-edge series produce a silent, mostly blank overview.** Captures 64–65:
  "Use all groups" builds a five-frame series from Ru and Cu spectra; the heatmap
  is dark for four of five rows and the E₀ trend drops from 22 000 to 9 000 eV
  with no warning. The 7 September audit's "directories become scans even for
  different edges" item is therefore still open. Refuse or warn when frames have
  different absorption edges.
- **The import receipt is replayed on project open.** Every capture from 14
  onward carries "Added 2 files → 2 groups · 1 needs mapping · 1 warnings"
  although nothing was imported in this session; the bar persists across all
  stages. Label restored receipts ("From the saved project") and auto-dismiss
  them on the first stage change.
- **A fresh import lands under a "Results" heading.** Capture 13: the only group
  in the list sits below "Results", the heading the fixture project uses for
  derived groups (capture 14). Use "Groups" or "Spectra" for sources and keep
  "Results" for derived output.
- **Wavelet plots keep the dark palette in the light theme** while the Fourier
  plots switch (captures 54–55). This is the reproducible defect described in 3.1.
- **Fractional ticks on integer axes** also affect the PCA scree plot ("1.25,
  1.5 … principal component", captures 46–47), the same defect as the Series
  frame axis (11.2).
- **Plural strings**: "1 warnings", "1 spectra", "1 sources", "Try 1
  components" (captures 14, 22–23, 47). Add a plural helper.
- **"Clear recent / close"** on the Problems drawer combines two actions in one
  label (capture 62). Split them.
- **Publish keeps image controls for CSV, Analysis folder and Markdown** and
  shows a figure preview for exports that are not figures (captures 68–70).
  Show a file-list preview for bundle formats.

## 13. Keyboard and accessibility

Foundation is good: an AccessKit tree with roles and names
(`accessibility.rs:340-526`), focus rings on chips, segments and buttons, modals
that restore focus. Gaps (code only):

- Bare `div` controls with no role, tab stop or focus style: row "⋯"
  (`groups_panel.rs:980-994`), thumbnails (`center.rs:1085-1110`), palette items
  (`palette.rs:436-456`), group-menu items (`group_menu.rs:800-813`), enum
  dropdowns (`app.rs:9503-9558`), steppers and "↺ auto"
  (`numeric_field.rs:319-388`), status-bar links (`app.rs:9784-9838`).
- The group context menu has Escape only, no ↑/↓ (`app.rs:522`); stage tabs are
  `Role::Tab` without arrow-key navigation (`stage_strip.rs:37-41`).
- Row diagnostics exist only in hover tooltips (`groups_panel.rs:800-822`).

Wrap each in `accessibility::Control`, add arrow keys to menus and tabs, and expose
the diagnostic text as the accessible description.

## 14. Delivery plan

Each item names the files most affected. Estimates are for one developer familiar
with the crate.

**Quick wins (hours each).**
1. Print `FitWarning.message`; implement `Display` for the peaks, MCR, import-mode
   and fit-space enums (`fit.rs:1139`, `peaks.rs`, `tools.rs:2291`,
   `stage_strip.rs:128, 267`).
2. Shortest-round-trip float formatting in `numeric_field::format_value` with a
   significant-figure cap.
3. "Fixed" cell width and the inspector "Apply to" row (`joint_browser.rs:532`,
   `inspector.rs` header).
4. "1 warnings" → "1 warning"; pluralisation in the viewer overlay.
5. Disable "Refine best…"/"Open checkpoint…" without a run; merge the three
   open-run entries.
6. Rename the two "Fit mode" controls.
7. Print ⌘P in the filter placeholder; add ⌘7 to the palette.
8. Persist the theme in `UserSettings`; rebuild the wavelet panels on toggle
   (3.1).
9. Integer ticks on Series frame axes.
10. Show the source path, not the project-store path, in Problems.

**One to three days each.**
11. Typed notices replacing `self.status` writes; severity colours; "Open Updates"
    and "Show problems" actions.
12. Inline field validation (invalid, bound, locked, unknown variable) using the
    existing `set_error` path; stop auto-creating variables without confirmation.
13. Typography pass: 12 px / 11 px floors, muted-colour rule, light-theme muted
    value, 20 px steppers.
14. Label table and one selected-state idiom; sentence case across fields, menus
    and chips.
15. Plot-card toolbar: reset view, cursor readout, legend toggle, handle hint,
    coloured range-field borders.
16. Resizable inspector; flex widths for group names and variable names.
17. Series transport row and frame field; colorbar titles; palette menu with
    swatches shared with Wavelet.
18. Fit blocking reason under the disabled button on every step; open projects on
    the last completed step; single-spectrum default.
19. Structure viewer mini toolbar shared by path fitting and RMC.

**Larger (one to two weeks each).**
20. Preferences window (theme, text size, startup, assistant, structure sources,
    updates) and removal of the scattered settings.
21. Fit results: statistics tooltips and units, guess-versus-fitted columns,
    History compare table, "Save report…".
22. Publish: figure/bundle export groups, presets, copy-style-to-all, plain-text
    labels with Typst toggle, 1:1 preview.
23. Import: guessed-role badges, default-accepted main spectrum, debounced
    revalidation with progress, working Cancel, plain-language reasons.
24. RMC settings page: right-pane preview, explanations in tooltips, run-details
    table, field labels bound to a typed struct instead of indices.
25. Accessibility: `Control` wrappers for the listed bare divs, arrow-key menus
    and tabs, diagnostic descriptions.

## 15. Open questions for the maintainer

1. Should provenance (source, mapping, checks) stay at the top of the inspector
   for the Data stage, or move into a disclosure as the simplification plan says?
2. Is a Preferences window acceptable, or should settings stay in their feature
   panels with a single "Settings…" index page?
3. For fit results, is a compare table across History entries wanted, or is
   restoring one entry at a time sufficient?
4. Is Typst markup in Publish labels a feature to keep visible, or an advanced
   option behind a toggle?
5. Windows and Linux were not checked in this review. The font, emoji-lock and
   stepper-size findings will differ there and need a native pass.

## Relation to earlier records

The 7 September UX audit's "Publish has a long scrolling control column" and "no
clear dirty indicator" items remain open in 0.2.11. The simplification plan's
work packages U1–U9 remain deferred per
[remaining-work-plan.md](remaining-work-plan.md); sections 1, 5, 6, 8, 11 and 12
above overlap with U1, U2, U3, U5, U8 and U9 and can reuse their acceptance
criteria. The v2 design's "three k-weights that look the same" pain point is
reproduced in 0.2.10 and 0.2.11 captures (section 7.3).
