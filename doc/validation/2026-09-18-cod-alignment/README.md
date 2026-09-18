# COD search and editable alignment — 18 September 2026

These checks use a local, unreleased macOS build from `feature/analysis-b-f`.
The isolated test applications contain the repository's public experimental
Cu measurements at 10 K and room temperature. No unpublished ReGe data was used.
See the [fixture provenance](../../../crates/rexafs/tests/fixtures/xas/README.md).

## COD search

The official COD endpoint returned 36,668 Cu-containing structure IDs in a
293,344-byte list. rexafs retrieves metadata in bounded batches and applies its
200-result limit and local filters. The JSON body limit remains unchanged.
The implementation uses the documented list and ID-query formats, rather than
an undocumented pagination parameter; see the
[COD REST API](https://wiki.crystallography.net/RESTful_API/).

Eight offline tests passed, with two live tests ignored by default. An explicit
live Cu search test then returned 200 results successfully. Computer use also
confirmed that the Cu search displays results without the previous oversized
JSON error: [COD Cu results](cod-cu.jpg). That screenshot does not show the count
footer; the count was verified by the live test.

## Alignment

The XANES preview shows derivatives on the spectra's separate energy grids,
scaled independently for visual comparison. An automatic correction of about
+2.99 eV plus a +0.5 eV manual adjustment produced a +3.49 eV displayed offset.
**Apply offset** changed the selected group without adding a group. **Zero**
returned the active offset to zero and E₀ from 8981.1 to 8977.6 eV. **Undo**
restored the exact offset, 3.485252958794127 eV. The last alignment record remained
visible after resetting, and the group count stayed at two.

These interactions were checked in the release application using macOS
accessibility and window screenshots after the computer-use connector stopped
working. Earlier COD and manual-preview checks used the computer-use connector.
Native Save dialog actions were unreliable in the fallback session, so project
round-trip claims below come from automated tests, not a completed GUI save.

- [Current alignment preview](alignment-offset-preview.jpg).
- [Editable offset in the final build](energy-offset.jpg), with Source details
  collapsed. The two-decimal field stays available. Selecting the reference
  confirmed its offset remained 0.00 eV.
- [Earlier manual adjustment check](alignment-manual.jpg), before the change
  from a materialized output group to an editable offset.

The full desktop suite passed 604 tests, with six ignored. Four subsequent
focused tests passed for reversible offsets, matching file/snapshot readers,
scoped copying of offsets and explicit E₀ settings, and linked/embedded project
round-trips retaining original arrays and alignment records after zeroing.
Five numeric-field tests passed after limiting the offset display to two decimal
places; submitting its unchanged rounded display retains the original precision.
The final desktop release build passed and its two-decimal field was verified
in the GUI. Website checks reported no errors, warnings or hints. Platform
qualification here is limited to macOS.

The [desktop processing guide](../../../website/src/content/docs/docs/desktop/processing.md)
describes active offsets separately from corrections already stored in historical
groups' arrays and from reference-channel calibration.

## Core integration during PR review

The desktop axis adjustment now delegates to `Spectrum::set_energy_offset(ev)`.
The core reuses its existing correction field and offers `energy_offset()`;
repeated assignments do not accumulate, and replacing arrays establishes a new
zero-offset baseline. Existing repeated energy points remain supported. Three
core regressions pass, including experimental Ru round trips, atomic error
handling and repeated-sample preservation. The final desktop suite passes all
608 tests (6 ignored). See the [review record](../2026-09-18-pr-review/README.md)
for the complete validation scope and remaining binding work.
