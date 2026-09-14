# Measurement reader and fixture organization

Requested on 2026-09-14 after the universal reader and Larix import work.
This refactor keeps public signatures, format detection, numerical conversions
and supported data unchanged. The baseline is 310 passing Rust core tests,
23 Python tests and 24 TypeScript/WebAssembly tests.

1. Keep raw beamline measurements in `tests/fixtures/xas`. Move complete Larix
   and XTUNES bundles from `tests/testfiles` into `tests/fixtures/sessions`.
   Keep each original bundle, license, manifest, expected array and generation
   record byte-for-byte intact. Add a current fixture index outside those
   historical bundles, with source links, license scope and validation commands.
2. Keep synthetic parser/conversion contracts under `tests/measurement_reader`,
   with a small integration-test entry point. Collect the new real-data tests
   under `tests/measurement_fixtures`, with shared path helpers and their coverage
   expectations. The original specialized-reader regression remains separate.
   Only fixture-dependent targets and bundles are excluded from crates.io.
3. Group Python and JavaScript reader tests into contracts, format examples and
   session examples. Centralize their paths; do not copy fixture data into each
   binding. Keep assertions at their relevant interface and retain every case.
4. Separate the shared reader model/conversion code from byte detection. Split
   Athena, historical binary and HDF5 adapters into named modules. Put Larix
   framing, inert JSON parsing and numeric decoding together in one directory.
   Move shared signal inference out of the text adapter. Keep API names stable.
5. Update code references, guides, editor help and generated Next documentation.
   Verify original bytes against both the pre-move snapshot and bundle manifests;
   check Cargo's actual package listing and rerun contract, corpus, installed
   binding and GUI tests. Rebuild and open the release GUI after qualification.

The source checkout's fixture licenses do not change the library's license or
license the external applications. Historical bundle notes retain their original
paths and proposed behavior; the current fixture index explains the relocation
and links to implemented support.

## Preview and raw-data follow-up

The subsequent GUI review restores a plotted modal for all single-file imports,
compact scan/signal/axis choices, optional column controls and original source
details. GUI preview conversion is checked against the shared reader across
the corpus and sessions. The angle regressions cover every retained 9809 file.
The newly retained ESRF BM16 acquisition increases the source corpus to 149
files and the total valid-input collection to 170. Its partial HDF5 group
coverage is recorded explicitly; `.qc`/`.qd` qualification remains a documented
gap pending appropriate reference payloads. See the
[source audit](beamline-source-repositories.md#additional-raw-data-audit).

The expanded collection has subsequently been copied into
`tests/fixtures/rexafs-corpus/` for self-contained repository tests. All 571
original data/attribution files are unchanged; no external checkout or download
is required. Its 222 measurement files are checked against explicit readable,
partial and rejected outcomes, with separate numerical regressions for Aichi,
SAMBA and PIRX. Both corpus snapshots remain excluded from the crate archive.
See the [copied-corpus audit](validation/2026-09-14-measurement-corpus/review.md).
