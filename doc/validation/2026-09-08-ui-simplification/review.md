# UI and import implementation review — 2026-09-08

Unreleased candidate on `feat/ui-simplification`, based on PR #45.
The maintainer's original checkout and active app sessions were preserved.

## Behavior

- Compact shell and scientific stage navigation; named SVG icons and platform menus.
  Import retains a visible label. Overview plots are optional. Everyday plot
  controls remain visible after maintainer feedback.
- Common inspector parameters stay open; advanced controls have disclosure panels
  and override indicators. Source mapping is visible at the top of Data unless
  an explicitly opened tool takes that position.
- New imports require an explicit main spectrum and optional channels, grouped by
  layout. Detected and saved mappings are suggestions. Only selected channels are
  created. Mapping preview and fixed footer preserve units, file/group counts,
  source validation, missing-source repair and recipe controls.
- Remove marked captures every marked identity, includes hidden marks in its count,
  refuses changed selection/project state, and uses the existing single Undo entry.
- Processing tools and Merge use previews of private copies; Apply retains the
  existing scientific operation. Operand names and affected counts are explicit.
- Fit has compact steps, contextual result controls and responsive setup panels.
  Structure appearance uses a menu; model/data identity and blocking errors remain.
- Assistant keeps model, reasoning, mode and sharing/access switches visible.
  Conversation utilities and longer descriptions use Settings. Permission and
  approval behavior retain the existing backend and busy-state guards.
- Series opens the actual scan selector, suppresses invalid frame/run UI and
  preserves marks during frame navigation. Publish has one export area, explicit
  format/scope and a large preview; Style remains open by default.
- AccessKit connects rendered controls, focus handles and actual callbacks to
  native accessibility. Main icons, navigation, inputs, dialogs and disclosure
  controls expose names and states. Adapters are included for macOS, Windows and
  Linux; native validation so far is macOS only.

## Validation

- `cargo test -p rexafs-gui`: **433 passed, 0 failed, 5 ignored** (159 s).
  Ignored cases include the opt-in external-folder test, run separately below.
- Targeted tests passed for explicit selected-channel intake, replacement of the
  main channel, immutable processing previews, and complete marked identity capture.
- Optimized build passed with the distributed `refeff-runner,feff10-runner` features.
- The earlier optimized package passed extracted Cu example/pipeline and both FEFF
  backend self-checks. The final candidate is repackaged from the final release binary.
- `cargo fmt --all` and `git diff --check` passed.

A local opt-in integration test used the maintainer-designated 102-file folder.
It discovered one layout, validated Fluorescence and Reference for every source,
accepted exactly 102 groups of each channel and zero Transmission groups, and
processed all 204 spectra successfully. It checked 127,296 raw points and verified
that source size/mtime revisions did not change. No source data or identifying
header metadata is included in this repository.

Reproduce locally with `REXAFS_IMPORT_VALIDATION_DIR=/path/to/folder cargo test
-p rexafs-gui validate_selected_channels_in_external_folder -- --ignored --nocapture`.
This opt-in test expects a folder suitable for Fluorescence and Reference.

## Native observations and limits

Earlier debug candidates were inspected in isolated macOS app bundles. Native
accessibility exposed application controls, selected/disabled states, import
mapping fields and modal isolation; setting an E0 field through accessibility
updated the app. Native inspection found invisible SVG strokes and missing Tab
stops, which were corrected. The maintainer used those candidates and identified
excessive hiding and automatic Transmission import; this implementation includes
those corrections.

An optimized release candidate was launched and its empty-window controls and
platform menus were inspected. Before the revised channel-choice and Remove
marked flows could be exercised visually, computer-use failed repeatedly with
`Sky Computer Use native pipe startup failed`, including after reconnecting.
The final channel-choice/footer and Remove marked layouts therefore still need
native verification. Do not treat the data-path integration test as visual QA.

Also pending: narrow/light-theme workflows, populated Series navigation, completed
Fit/Assistant busy and approval states, export-preview comparison, and complete
keyboard/screen-reader coverage. Windows/Linux native adapters are not runtime
qualified in this macOS session.
