# Desktop review follow-up, 20 September 2026

This record covers the six usability findings from the manual review of the
0.2.12 nightly at `e504548a`. The implementation is unreleased. Verification used
a locally built macOS Apple Silicon application, a separate application bundle
and isolated settings. The installed stable and nightly applications were not
used for these checks. Logical window sizes were 1440 × 932 and 1100 × 699.

## Requirement checks

| Finding | Implemented behavior and observed check | Evidence |
|---|---|---|
| Publication freshness | Editing a preset after publishing shows “Changes since last publish” and “Publish changes…”. A successful second export clears the indicator and updates the timestamp. “Open last published report” names the saved version explicitly. | [Edited report](publish-changes.jpg), [second export](publish-updated.jpg) |
| Narrow Series layout | At 1100 × 699, the heatmap and detail plots stack in a scrollable area. Difference, Reference, Colors and Export remain visible. Scrolling reaches the detail spectrum and trend. | [Compact Series](series-small.jpg) |
| Automatic values | The compact input says Auto, while a separate line shows the complete automatic value or range description. The edge energy and “spectrum end” remain readable in the narrow inspector. | [Edge energy](numeric-auto.jpg), [normalization range](numeric-range.jpg) |
| Invalid numeric input | Entering `oops` preserves the invalid text and shows an inline explanation. The previous committed value remains active. “Use Auto” recovers successfully and clears the error. | [Inline error](numeric-validation.jpg) |
| Publication style scope | “Apply style to all figures” sits beside the presets, above the detailed fields, with an explicit figure-type scope label. Applying Double column to all figures and selecting Energy retained its 7 × 4.8-inch dimensions. | [Style controls](publish-changes.jpg) |
| Frame numbering and navigation | Previous/Next, direct entry, the heatmap, the trend and the sidebar agree on frame numbers starting at 1. Entering 5 selects the last of five frames and disables Next. Entering 6 keeps frame 5 selected and shows a range error. Clicking heatmap row 2 selects frame 2. | [Direct entry](frame-navigation.jpg), [frame 2](series-small.jpg), [trend CSV](series-trend.csv) |

The first report retained an Energy SVG measuring 2100 × 1440, corresponding to
7 × 4.8 inches at 300 dots per inch (DPI). The second report contained an Energy
SVG measuring 2100 × 1560, corresponding to 3.5 × 2.6 inches at 600 DPI. Both
exports completed with manifests. The earlier directory remained intact.

The shared-style operation retains each figure's axis limits, labels, captions
and curve visibility. Existing style-copy tests cover this separation. Frame
indices in scientific arrays and saved projects remain unchanged; conversion to
one-based numbers occurs at the plot, navigation and overview CSV boundaries.

## Automated checks

- `cargo check --locked -p rexafs-gui` passed.
- `cargo build --locked -p rexafs-gui` passed for the inspected application.
- `cargo test --locked --release -p rexafs-gui` passed: 643 tests, 7 ignored.
- `npm --prefix website run check` passed with no errors, warnings or hints.
- Rust formatting and whitespace checks passed.

The first debug-profile test run had 640 passes and three RMC worker timeouts.
All three passed with the optimized release profile used by desktop CI. No
timeouts were increased or tests disabled. New regression checks cover exported
revision tracking, complete automatic hints and frame-entry bounds; existing
heatmap tests now check one-based display coordinates and zero-based storage
indices. This manual review does not qualify Windows or Linux rendering.

## Sources and provenance

Implementation: [publication state](../../../crates/rexafs-gui/src/app/shell/publish.rs),
[publication controls](../../../crates/rexafs-gui/src/app/shell/publish/editor.rs),
[numeric fields](../../../crates/rexafs-gui/src/widgets/numeric_field.rs),
[Series controls](../../../crates/rexafs-gui/src/app/shell/series.rs),
[plot coordinates](../../../crates/rexafs-gui/src/plotting.rs) and
[CSV export](../../../crates/rexafs-gui/src/app/shell/plot_export.rs).

The walkthrough opened a copy of the retained
[`rexafs-0.2.12-embedded.rxs`](../../../crates/rexafs-gui/tests/fixtures/projects/rexafs-0.2.12-embedded.rxs)
fixture. The original and review copy remained byte-identical, with SHA-256
`e9fc3e80152aa70529d38bb07d4a078d93bf1cb5b886fbb059cfd07fb77c252d`.
Its synthetic settings, mixed measurement sources and deliberately missing source
are persistence examples, not a physical time series or scientific fit reference.
See the [fixture provenance](../../../crates/rexafs-gui/tests/fixtures/projects/README.md).

The JPEGs are unedited computer-use captures. Wide windows were scaled by the
capture tool; their logical dimensions are recorded above. The
[evidence manifest](evidence.json) identifies the original capture filenames and
checksums. Additional captures, accessibility transcripts, build/test logs and
both exported reports remain under the ignored local
`target/ux-review-implementation-20260920/` directory; test logs are under `target/`.
