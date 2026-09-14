# Additional measurement corpus audit

This historical record describes the initial copy into `tests/fixtures/rexafs-corpus/`.
The subsequent consolidation retains every original file in the canonical
[`xas/` collection](../../../crates/rexafs/tests/fixtures/xas/collections/rexafs-corpus/INDEX.md),
with a path map for the unchanged manifests and snapshot. All 222 measurement
entries and their original attribution remain available; shared files are stored
once. Tests require no private gathering checkout, and Cargo excludes the data
and their fixture tests from the published crate. The measurements and original
qualification numbers below are unchanged.

Recorded on 2026-09-14 for the **unreleased** reader on `test/beamline-fixtures`.
The inspected `../rexafs-format` checkout was at `d142ad2` with manifest SHA-256
`85336ac90dc37f10cba8d6b9761abd96be12010ebd4c711a43b1018eec691adc`.
The reader included uncommitted changes; the installed wheel still reports
version 0.2.5, so these results do **not** describe released 0.2.5 behavior.
The [machine-readable report](reader-audit.json) identifies the tested extension
and audit script by checksum and retains every source URL, license basis,
recovered scan count, detected-signal conversion outcome and diagnostic.

## What was found

The external checkout contains **222 original files** (210,838,051 bytes) from
20 sources. Its verifier passed all 222 payloads and 109 provenance/license
assets. Comparing SHA-256 values with rexafs's 149 retained beamline fixtures
identified **89 additional unique payloads**. The original ESRF BM16 BLISS file
is already retained by both collections under different paths and was counted
once in this comparison. Application-generated Larix/XTUNES bundles are separate.

The additional files comprise 56 acquisition files, 13 exports retaining raw
detector channels, ten beamline exports, four converted exports, three detector
projections, two analysis projects and one normalized export, using the external
manifest's `data_level` classifications. File counts are not experiment counts:
31 ALBA images belong to one energy scan.

| Reader outcome | All external files | Additional unique files |
| --- | ---: | ---: |
| At least one detected signal converts successfully | 96 | 24 |
| Parsed, but requires explicit mapping or detector reduction | 81 | 22 |
| Partial HDF5 recovery with group-enumeration warnings | 5 | 3 |
| Rejected | 40 | 40 |
| Total | 222 | 89 |

Thus **182 files parse**, including five partial HDF5 recoveries. This is not a
count of fully supported files. A convertible candidate can still require a
choice among detectors, appropriate units, calibration or other scientific
review. The audit calls `Measurement.arrays()` for detected candidates; it does
not claim that every candidate can become a processable `XASSpectrum`, recover
every source array, or reproduce the upstream analysis. Partial status takes
precedence over conversion success in this table.

| Additional source | Files | Observed result |
| --- | ---: | --- |
| Aichi SR, BL11S suffix unresolved | 3 | 9809 fluorescence candidates; observed-angle conversion uses the recorded crystal spacing. |
| APS 13-BM-D, 13-ID-E and 20-BM | 17 | 15 files have convertible candidates; two NeXus files have partial recovery. |
| APS 20-ID-C | 3 | Detector projections require an explicit reduction/mapping. |
| Australian Synchrotron MEX1, MEX2 and SXR | 13 | Three text files have convertible candidates; four require mapping; six native MDA binaries are rejected. |
| BESSY II EMIL OAESE | 4 | Parsed tables require detector/axis interpretation. |
| DESY PETRA III P64 | 5 | Parsed tables require explicit mapping or detector reduction. |
| SOLARIS PIRX | 4 | SPEC/text data are retained for explicit mapping; detector roles are not guessed. |
| SOLEIL SAMBA | 3 | Stored `XMU` is detected and preserved. |
| ESRF BM16 and Photon Factory BL-9A | 2 | Converted NeXus examples; BM16 requires mapping and PF has partial recovery. These are additional to the original BM16 acquisition. |
| ALBA CIRCE | 32 | One energy table parses; 31 Elmitec PEEM images are rejected. |
| Diamond I06, PLS-II 2A and SOLARIS ASTRA | 3 | Origin OPJU, XLSX and a CSV with separate energy grids and missing cells are rejected. |

No `.qc` or `.qd` payload was found. These additions do not establish support for
Photon Factory QXAFS condition/data pairs. See the
[source investigation](../../beamline-source-repositories.md) for the actual PF
download lead and the distinction between `.qc` conditions and `.qd` data.

## Attribution and storage

The external manifest records 210 files with documented terms and 12 historical
RefXAS candidates retaining their original usage notice. For the 89 additional
payloads, it records 52 CC BY 4.0 files, five CC0 files, and 32 files relying on
the MIT license of the distributing repository. A repository distribution
license is recorded separately from an explicit creator-issued data license.
These are the corpus's recorded license bases, not a new blanket license.

The copied snapshot retains all **571 original files**, totaling 214,706,079
bytes, including adjacent `.license` files, `LICENSES/`, `CITATIONS.md`, manifests,
provenance, collection scripts and historical research documents. Every copied
file was compared byte for byte with the gathering checkout. An added
[`SNAPSHOT.json`](../../../crates/rexafs/tests/fixtures/xas/collections/rexafs-corpus/SNAPSHOT.json)
records their checksums and source revision. Original collection documents remain
unchanged and describe the gathering repository at that revision.

Existing `xas/` fixtures remain unchanged. The two collections share 133 payloads
by SHA-256 and contain **238 unique beamline/project payloads** in total. They are
separate snapshots so their manifests and historical attribution stay intact.
Per-record links in [reader-audit.json](reader-audit.json) lead to the original
public repositories or deposits; full notices accompany the copied data.

## Reproduction and numerical regressions

First build and install the source-checkout Python wheel as described in the
[binding checks](../../../js-rexafs/test/README.md). Then run:

```sh
python scripts/audit-measurement-corpus.py \
  --corpus crates/rexafs/tests/fixtures/xas --collection rexafs-corpus \
  --output /tmp/rexafs-corpus-audit.json

cargo test --locked -p rexafs --test measurement_fixtures
python scripts/check-beamline-fixtures.py
python scripts/check-beamline-fixtures.py --package
```

All **24 fixture tests pass**. Four tests in
[`format_corpus.rs`](../../../crates/rexafs/tests/measurement_fixtures/format_corpus.rs)
cover the copied collection. One checks all 222 payload hashes and their recorded
reader outcomes, including explicit rejection expectations for the 40 known
gaps. Three numerical tests pin seven original payloads by SHA-256 and independently read numeric cells
and check every Aichi fluorescence ratio and Bragg-converted energy, all SAMBA
columns and stored absorption values, and all numeric cells and scan boundaries
in the 27-scan PIRX SPEC file. The tests establish preservation and conversion
behavior, not experimental accuracy or detector calibration. They run in the
normal source-checkout suite, without environment variables or downloads, and
are excluded with the fixture test target from the crate archive.

## Package verification

The copied data are repository test assets, not library runtime assets.
[`Cargo.toml`](../../../crates/rexafs/Cargo.toml) excludes all three attributed
fixture directories and the dedicated fixture test target. The integrity helper
also checks Cargo's package file list to catch accidental future inclusion.

Actual package contents were checked after copying the collection:

| Artifact | Entries | Measurement corpora or dedicated fixture tests |
| --- | ---: | --- |
| Rust `.crate`, built with `cargo package --locked --allow-dirty --no-verify -p rexafs` | 363 | None |
| Python source distribution, built with `maturin sdist` | 392 | None |
| Installed-build Python wheel | 13 | No fixture files |
| npm package file list, checked with `npm pack --dry-run --json` | 27 | No fixture files or tests |

The `.crate` was inspected as an archive; `--no-verify` skips Cargo's separate
archive rebuild, not packaging exclusions. Normal core/fixture tests and strict
Clippy passed in the source checkout. Self-contained synthetic Rust contracts
remain in the source archive and require none of the copied measurements.

## Implementation priorities

1. **MDA acquisition reader:** use the six Australian files and their upstream
   pyNexafs source as independent references. Preserve dimensions, scan position,
   detector names/units and interrupted-scan lengths; add malformed-offset and
   truncation tests before making automatic mappings available.
2. **HDF5/NeXus completeness:** resolve linked-group enumeration in the five
   partial examples. Compare full dataset inventories and selected arrays with
   an independent HDF5 reader; retain visible partial-read diagnostics until
   completeness is established.
3. **Explicit detector profiles:** verify units and signal definitions for the
   BESSY, P64, PIRX and Australian text families. Generic column preservation is
   useful now, but automatic scientific mappings require source evidence.
4. **Multiple-grid CSV and spreadsheets:** represent independent energy/signal
   groups and missing cells without truncating unrelated columns or silently
   interpolating. ASTRA's CSV needs this model before it can be treated as a
   collection of spectra; XLSX needs bounded worksheet extraction as well.
5. **Image and Origin projects:** retain ALBA's image/energy relationship and
   define explicit regions of interest before reducing images to spectra.
   Evaluate OPJU separately as an analysis-project format. Neither family should
   be accepted as an ordinary two-column spectrum based on its extension.

Each new adapter should enter the shared Rust reader first, then receive Python,
TypeScript/WebAssembly and GUI workflow checks. Format coverage should increase
when numerical and workflow checks pass, not merely when files are added.
