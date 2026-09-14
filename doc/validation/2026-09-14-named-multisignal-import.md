# Named columns and multiple signal import

This is an unreleased source-checkout review on macOS ARM64, dated
14 September 2026. It covers column selection and the desktop import workflow,
not experimental calibration or agreement between processing algorithms.

## Behavior

Rust adds `ColumnSelector`, `SignalSelection` and `SpectrumSelection` while
preserving the existing numeric `SpectrumMapping` and conversion methods.
Python and TypeScript accept names in their existing mapping dictionaries and
provide column keywords/options for common stored-signal, transmission and
fluorescence selections. Names are exact and case-sensitive; duplicates require
indices. The selected axis retains its calibration unless explicitly overridden.

The desktop lists detected signals with independent inclusion checkboxes and
preview buttons. Each row shows the source-column formula. Checked outputs are
converted before the project is changed, receive distinct labels and provenance,
and share one undo action. The first imported spectrum opens as raw μ(E).

## Computer-use review

The review used an isolated app bundle and settings under
`target/multisignal-review`, opened and operated through native computer use.
The fixture was the retained QAS `Mo foil 0001-r0003.dat`, with 651 points and
`energy`, `i0`, `it`, `ir`, `iff` channels. Its original data and attribution
are retained in the [beamline corpus](../../crates/rexafs/tests/fixtures/xas/README.md).

Observed in the running app:

- Transmission, fluorescence and reference were visible and checked on opening.
- All three previews used the expected channel formula; reference showed its
  absorption edge using `ln(it / ir)`.
- Excluding fluorescence changed the action to **Import 2 spectra** without
  changing the reference preview. Excluding every output disabled import.
- Editing the reference monitor survived switching to transmission and back.
  **Reset mapping** restored `It: 3: it`.
- Importing two selected outputs created exactly transmission and reference.
  One undo removed both; redo restored both.
- A fresh final build imported all three outputs with distinct group names.
  One undo removed all three and redo restored all three.
- Full-window observations at 1187 × 768 and 963 × 768 showed the choices,
  plot and import action without clipped controls.

The initial implementation exposed a misleading transition to the normalized
plot after import. This was corrected and visually rechecked: the Data view now
selects raw μ(E), preserving the imported signal's scale. Visible inclusion
choices also remove the need to discover available outputs inside a dropdown.
Screenshots and accessibility observations were reviewed in the development
session; this record does not replace the earlier archived screenshot files.

PR update: a [full current capture](../../website/public/screenshots/next/import-signals.jpg)
now records the three selected outputs with the reference preview active, in a
clean review window. Its [capture record](2026-09-14-named-multisignal-import-capture.json)
retains the executable, source and image checksums. The public Next guide includes
this image and the source attribution; earlier EX3 captures remain historical.

## Automated checks

The following passed with Rust 1.98.1, Python 3.12 and Node 24:

- `cargo test --locked -p rexafs`: 326 tests passed, including doctests;
  three existing optional tests were ignored.
- `cargo test --locked -p rexafs-gui`: 472 passed, five existing tests ignored.
  The ten measurement-import tests passed again after the raw-view change.
- Strict core Clippy, Rust formatting and Rustdoc with warnings denied.
- Built and installed Python wheel: ten measurement tests and Pyright runtime
  package/editor checks, including new column-keyword completion and hover.
- Built browser/Node Wasm package: all 25 npm tests passed, including installed
  tarball checks and TypeScript option completion, signatures and hover.
- Website type check: no errors or warnings; 22 website tests and eight
  reference-generator tests passed. Next references were regenerated; Stable
  references were unchanged.

Numerical tests check reordered columns, mixed names/indices, duplicate/missing
names, explicit units, preserved relative/Bragg calibration, conflicting
calibrations and the three QAS formulas. GUI tests cover retained edits, reset
defaults, excluded invalid signals, empty selections and independent provenance.

Adding reference detection changed only the signal-choice expectations for six
files in the retained corpus and five in the expanded corpus. Raw measurements,
manifests, checksums and historical qualification records were not changed.

## Windows fixture checkout correction

The preceding PR revision failed its Windows core job because Git changed the
expanded corpus manifest's LF line endings to CRLF. The resulting SHA-256
`7153900de3cf8afa108206323eac4a9a1759dac7ef8be9e18332f562f2c6870f`
matches the [failed CI run](https://github.com/Ameyanagi/rexafs/actions/runs/34800077316);
the original hash remains
`85336ac90dc37f10cba8d6b9761abd96be12010ebd4c711a43b1018eec691adc`.
The `.gitattributes` rules now disable text conversion for `rexafs-corpus/` and
`sessions/`, matching the existing protection for `xas/`. A temporary Git
repository with `core.autocrlf=true` and the updated attributes preserved all
621 corpus/session files byte for byte through Git's checkout filters. No
measurement or historical checksum was rewritten to accommodate the failure.
