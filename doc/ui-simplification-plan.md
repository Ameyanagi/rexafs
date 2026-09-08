# UI simplification handoff

Status: implemented on `feat/ui-simplification`, based on
[PR #45](https://github.com/Ameyanagi/rexafs/pull/45). This is an unreleased candidate.
Validation and remaining native checks are recorded in
[the implementation review](validation/2026-09-08-ui-simplification/review.md).

## Design contract

Make the plot the main workspace. Keep the current task and its next action
visible. Keep everyday controls visible: the maintainer found the initial implementation
hid too much. Fold advanced settings and supporting explanations. Prefer direct
manipulation, short labels, and familiar icons to instructional paragraphs.
Icons need accessible names; scientific units and ambiguous actions still need labels.
Import keeps its word label. Source mapping, common plot toggles, Assistant model,
reasoning, permissions and mode, and publication style remain visible.

Keep errors, unsaved changes, named operands, affected counts, active overrides,
and permission scope visible when they affect a decision. Put detailed provenance,
diagnostics, licenses, and setup instructions behind explicit actions.

## Findings from the running app

Reviewed the packaged PR #45 build on macOS at approximately 1187 × 768, with
seven groups, two marks, a pending source, an unfinished fit, and Assistant open.
These are observations from native interaction, not just source inspection.

| Area | Observed | Proposed presentation |
|---|---|---|
| Shell / plots | Stage cards, import receipt/history, plot toggles, and four overview plots remain visible together. Plot controls truncate even at this window size. | Compact stage navigation; project commands in a menu; secondary plot controls in a View menu; collapsible overview. Preserve the active view and current spectrum identity. |
| Normalize / Background / Transform | Help paragraphs interrupt parameter groups. Back-transform and detailed background controls are open alongside common settings. | Common parameters first; advanced sections collapsed. Show an active-override indicator and reveal the relevant section when its plot or parameter is selected. Keep units and draggable range handles. |
| Import / Groups | A missing-source dialog still exposes three channel-control rows, recipe setup, validation controls, and disabled navigation. Pending import also occupies the sidebar and a persistent multi-row receipt. | Lead missing-source repair with **Locate source**. Reveal mapping after repair. Collapse receipt/history to one actionable pending count; retain unresolved sources. Show hidden-mark controls only when relevant. The existing column-table/plot mapping preview is worth keeping. |
| Tools / LCF / PCA | Align opens below the visible inspector area. LCF/PCA name the target but leave input identities implicit in marks. | Bring the opened tool into view. Put named operands and the preview above its action; keep exact input lists one click away. Preserve each tool's different use of current and marked groups. |
| Fit | Two navigation rows, repeated headings/instructions, and many structure-display buttons compete with the model and plots. Results exposes contribution controls before any fit exists. | Compact step navigation; completed setup as a summary; structure appearance in a View menu. Keep parameter errors beside the action that resolves them. Reveal result controls when results exist. |
| Assistant | Model, reasoning, image sharing, web search, access, mode, copy, and explanatory text consume much of the panel. Opening it squeezes Fit into narrow columns. | Keep conversation and composer prominent. Keep model/reasoning/mode and sharing controls visible; put supporting explanations and conversation utilities in settings. Offer a focus layout without silently changing access or scientific settings. |
| Help / Updates | Help is already compact. Updates says **Up to date** while highlighting a download of the same version. | Show current version/status first. Put channel/preferences and same-version download behind secondary controls. Preserve release notes and offline licenses. |
| Series | Empty state says **Pick a scan in the Scans tab**, but no such tab is visible. Inspector shows **Frame 1 / 0**, an unrelated current spectrum, and run buttons. | One **Select scan** action opening the actual scan selector. Show frame/trend/run controls only with a valid scan; give an actionable empty state when none exist. |
| Publish | PNG/SVG/CSV actions are at the top; folder export/Markdown at the bottom. Style controls and export explanations are always open. | One export area with format and explicit scope; large preview; Style and Caption disclosure panels. Show format-specific details when that format is chosen. |
| Accessibility | Native accessibility inspection exposes window controls but no application controls. The command palette opens with ⌘K and dismisses with Escape. | Expose control names, roles, states, and focus. Verify menus and disclosure panels with keyboard and a screen reader; icons alone cannot solve this. |

## Order

First fix blocked or misleading navigation: missing-source repair (U3), Series
selection (U8), and same-version update emphasis (U7). Then recover plot space
through the shell, inspectors, and Assistant (U1/U2/U6). Follow with Fit, tools,
and Publish (U5/U4/U9). Check accessibility throughout.

## Work packages

| ID | Scope | Acceptance |
|---|---|---|
| U1 | Compact shell, plot toolbar, and optional overview. Put project/settings commands in menus; storage choices in Save/Save As. | Current spectrum, unsaved state, and primary actions remain visible at normal and narrow widths. Controls overflow into a menu without clipping; all commands remain accessible. |
| U2 | Common inspector parameters first; advanced controls, help, and derivation details on demand. | Normalize, Background, and Transform work without opening advanced sections. Active overrides, units, effective scope, locks, and invalid inputs stay clear. |
| U3 | Repair-first import, compact receipts, and contextual group actions. | Missing-source repair has a direct next action. Six-file navigation, hidden marks, mixed-layout mapping, pending sources, recipe reuse, and undo remain clear. Never change marks implicitly. |
| U4 | Simplify tool cards: one primary action, explicit named operands, visual before/after preview, concise affected counts. | A user can distinguish the standard, targets, result, and skipped inputs visually. Preserve the different meanings of bulk Align, Calibrate, Difference, and Merge. Coordinate with feature task F4 in the remaining-work plan. |
| U5 | Compact Fit steps; contextual setup/results; structure-display menu. | Current data/model identity, missing parameters, stale fits, and non-convergence stay visible. Browsing structures cannot silently replace calculated paths. The fit action and its blocking reason remain reachable with Assistant open. |
| U6 | Conversation-first Assistant; settings/history menus and optional focus layout. | Empty, connected, busy, approval, failed, and restored states each have a clear next action. Keep permissions, sharing, and edit scope explicit; preserve drafts and conversation state. |
| U7 | Help/preferences/platform menus, contextual update actions, and accessible controls. | Up-to-date state does not promote redundant download. Verify keyboard operation, focus restoration, accessible names/roles/states, dark/light themes, and narrow windows. |
| U8 | Actionable Series selection and contextual inspector. Coordinate with F1/F2. | Empty state opens an actual scan selector. No impossible frame count or runnable calculation without valid inputs. Populated view retains exact range and sampled-preview distinctions; selection must not silently create a scientific series. |
| U9 | Unified export area; preview-first Publish; optional style/caption controls. | Before export, format, quantity, current/marked/fit-input scope, and destination are clear. PNG/SVG preview matches output; CSV full-grid behavior remains explicit. |

U1–U9 describe the areas of this change. Capture the affected workflow, make the
smallest layout change, then verify it in the native app. Scientific algorithms,
defaults, project compatibility, and authorization policy must stay intact.

## Coverage and follow-up checks

Inspected Data, Normalize, Background, Transform, all five Fit steps, empty
Series, Publish, Align/LCF/PCA cards, missing-source review, existing-column
mapping, Help/Updates, the command palette, and Assistant with its panel open
and closed. Returned to the original current spectrum, Fit step, and Assistant
view. No analysis operation, mapping, import, export, or permission change was applied.

Still require native checks for populated Series, completed/failed fits,
Assistant busy/approval/error states, fresh multi-layout imports, narrow windows,
light theme, and complete keyboard/screen-reader use. This review does not qualify
those states. Startup and offline-license checks are recorded in the
[startup/Help review](validation/2026-09-08-startup-help/review.md).

Implementation starts in `crates/rexafs-gui/src/app/shell/`: `mod.rs`,
`center.rs`, `inspector.rs`, `groups_panel.rs`, `import_receipt.rs`,
`import_editor.rs`, `tools.rs`, `fit_workspace.rs`, `structure_view.rs`,
`assistant.rs`, `series.rs`, `publish/editor.rs`, and `updates_view.rs`.

## Delegation

Start from current `main`, incorporating the reviewed PR #45 first and preserving
the maintainer's dirty checkout. Pick one work package, record before/after native
screenshots, and test the affected interaction and project restoration. Do not
mark a task complete from source inspection or static screenshots alone.

The separate feature backlog is in [remaining-work-plan.md](remaining-work-plan.md).

## Import correction from hands-on feedback

Do not silently choose Transmission from a file containing multiple detector
channels. A new import stages sources, groups matching layouts, and opens the
mapping preview before adding groups. The user explicitly chooses the main
spectrum and optional channels once per layout. Only those channels are added;
project restoration and already accepted reviews retain their stored mappings.
Saved recipes provide suggestions in this flow.

Switching the main spectrum replaces the previous main-channel choice. It does
not retain an unwanted Transmission output. Additional selected channels remain
explicit. Channel choices, formulas, column assignments, units, preview, file
counts and Import/Cancel remain reachable. The footer stays visible while the
mapping table scrolls.

The Groups panel also has **Remove marked…**, with a captured identity list,
hidden-mark count, stale-selection check, and one undoable removal. It never
deletes source files.

## Structure clarity

Keep **Center focus** visible above the structure canvas. Increasing it smoothly
fades atoms and bonds beyond the clear coordination radius while preserving the
absorber and selected scattering path. **Path focus** retains faint atom context
and shows only scattering legs. The inspection center defaults to the absorber;
clicking an atom changes the inspection center. Clear radius, slicing and global
opacity remain in Structure display. These controls never change the FEFF cluster,
calculation radius or path geometry.
