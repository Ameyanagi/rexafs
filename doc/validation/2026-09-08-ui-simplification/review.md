# UI and import implementation review — 2026-09-08

Unreleased candidate on `feat/ui-simplification`, based on PR #45.
The maintainer's original checkout and active app sessions were preserved.

## Behavior

- Compact shell and scientific stage navigation; named SVG icons and platform menus.
  Import retains a visible label. Overview plots are optional. Everyday plot
  controls remain visible after maintainer feedback.
  A sun/moon button keeps theme switching directly available in the top bar.
  Ordinary startup opens Data with an empty dataset and no central import prompt.
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
- The structure toolbar exposes Center focus: outer atoms and bonds fade smoothly
  while the absorber and selected scattering path remain clear. Path focus hides
  the surrounding bond network and leaves faint atom context. The shared sliders
  expose native numeric values and keyboard adjustment. These are display effects;
  global opacity and clipping still apply and calculation geometry is untouched.
- Assistant keeps model, reasoning, mode and sharing/access switches visible.
  Conversation utilities and longer descriptions use Settings. Permission and
  approval behavior retain the existing backend and busy-state guards.
  Its retained view now follows application theme changes, including composer and
  history inputs in both docked and separate-window hosts. Access text uses the
  theme warning color.
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
- After adding structure focus, eight depth tests passed, including smooth center
  fading, highlighted-path picking and preservation of explicit clipping.
  All four molecular-view tests also passed.
- After correcting Assistant theme synchronization, all 47 Assistant tests passed.
- Optimized build passed with the distributed `refeff-runner,feff10-runner` features.
- The final optimized package at source commit `bb26002` passed the extracted Cu
  example/pipeline and both FEFF backend self-checks. Its build metadata records a
  clean source tree and the same commit as the embedded build identity.
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
The final channel-choice/footer, Remove marked and structure-focus layouts therefore
still need native verification. Reconnecting for structure-focus verification failed
with the same native-pipe error. Do not treat integration tests as visual QA.

The final release candidate was packaged in a separate app bundle. The requested
launch also failed at the native connection with the same error, before the app
could open. New empty-Data startup and live Assistant theme synchronization are
implemented and compiled, but their final appearance is not visually qualified.

Also pending: narrow/light-theme workflows, populated Series navigation, completed
Fit/Assistant busy and approval states, export-preview comparison, and complete
keyboard/screen-reader coverage. Windows/Linux native adapters are not runtime
qualified in this macOS session.
