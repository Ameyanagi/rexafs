# UI simplification handoff

Status: planned, except the startup/Help changes tracked in the remaining-work
plan. Do not treat the completed 0.2.0 release as completion of this design work.

## Design contract

Keep the spectrum and the current task prominent. Show primary actions directly;
reveal secondary controls when requested or relevant. Prefer familiar controls,
short labels, and tooltips to explanatory paragraphs. Icons must have accessible
names; do not replace scientific units or ambiguous actions with unexplained symbols.

Keep errors, unsaved changes, operands, affected counts, and scientific
incompatibility visible when they affect a decision. Detailed provenance,
diagnostics, licenses, and setup instructions belong behind explicit actions.

## Work packages

| ID | Scope | Acceptance |
|---|---|---|
| U1 | Simplify the top bar: move secondary project/settings commands into menus; use one primary action per context. Place linked/portable storage choices in Save/Save As with short user-facing labels. | At narrow and normal window sizes, project identity and primary actions remain visible without clipping. Keyboard access and all existing commands remain available. |
| U2 | Reduce inspector density: show common parameters first; put advanced options and derivation details in collapsible sections. Shorten repeated scope explanations. | Normalize, Background, and Transform can be completed without opening advanced sections. Units, non-default values, effective scope, locks, and invalid inputs remain discoverable and correct. |
| U3 | Compact Groups and import feedback: one current/mark/visibility vocabulary, brief receipts, detailed diagnostics on demand. | Six-file navigation, hidden marks, mixed-layout import, failed-source repair, and undo remain understandable without paragraph-length guidance. Never conceal pending sources or silently change marks. |
| U4 | Simplify tool cards: one primary action, explicit named operands, visual before/after preview, concise affected counts. | A user can distinguish the standard, targets, result, and skipped inputs visually. Preserve the different meanings of bulk Align, Calibrate, Difference, and Merge. Coordinate with feature task F4 in the remaining-work plan. |
| U5 | Simplify Fit: reveal structure, calculation, model, and result details at the relevant step; collapse setup after completion. | Current data and model identity stay visible. Browsing structures cannot replace calculated paths silently; stale fits and non-convergence remain visible. |
| U6 | Reduce Assistant chrome and onboarding text; put setup and history management behind explicit controls. | Empty, connected, busy, approval, failed, and restored-conversation states each have a clear next action. Keep permission and edit scope explicit. |
| U7 | Consolidate Help, preferences, shortcuts, and platform menus. | Secondary commands are discoverable through menus and the palette. Check keyboard-only operation, focus restoration, accessible names, dark/light themes, and narrow windows. |

U1–U7 are separate reviewable changes, not one repository-wide text deletion.
First capture the current release in the affected workflow; propose the smallest
layout change, then verify it in the native app. No scientific algorithms,
defaults, project compatibility, or authorization policy should change as a side
effect of presentation work.

## Delegation

Start from current `main`, preserving the maintainer's dirty local checkout.
Pick one work package, record before/after native screenshots, and test the
affected interaction and project restoration. Do not mark a task complete from
source inspection or static screenshots alone.

The separate feature backlog is in [remaining-work-plan.md](remaining-work-plan.md).
