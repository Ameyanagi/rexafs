# Universal measurement reader implementation plan

Status: unreleased work, based on fixture commit `86133dc` on
`test/beamline-fixtures`. Written before implementation on 2026-09-14.

## Objective and acceptance criteria

Provide one Rust import contract, shared by Python, TypeScript, the desktop and
the browser workspace. Cover every one of the 132 retained measurement/project
files and ordinary numeric text and HDF5 inputs. Support means recovering the
available measurements and metadata faithfully; an image-only detector file or
an uncalibrated pixel axis must not be represented as an absorption spectrum.
Files lacking units or detector definitions must expose the missing mapping.

The baseline has 18 fixture reader regressions, not 132 supported readers.
The manifest describes acquisition formats, exports and historical projects;
facility names alone do not define file layouts. Preserve every fixture and
attribution file byte for byte, and retain the crate archive exclusions.

## Design

1. Add a content-based dispatcher with path and byte entry points in
   `rexafs::io`. Inspect signatures before generic text fallback. Extensions and
   filenames may label results but must not determine detector arithmetic.
2. Return an owned import document containing all scans, original headers,
   named columns with units, dataset paths, diagnostics and suggested signal
   mappings. Separate reading from conversion to an unprocessed spectrum.
   Keep original ordering and repeated energies in the document.
3. Provide explicit energy and signal mappings. Transmission uses the natural
   logarithm of incident/transmitted intensity; fluorescence and electron yield
   use a selected detector sum divided by incident intensity; precomputed
   absorption is copied. Validate selected channels and all resulting values.
   Never infer fluorescence sums merely because detector columns exist.
4. Add small format adapters for shared families: XDI and its acquisition
   variants, SPEC/Sardana, FIO, SRS, Japanese 9809 and yield/EX3 exports,
   EPICS/LabVIEW and other named tables, historical binary tables, Athena Perl
   and JSON projects, and HDF5/NeXus containers. Preserve scan boundaries.
5. Use a common HDF5 implementation on native and WebAssembly if fixture tests
   establish compatibility. Enumerate numeric datasets and their shapes without
   pretending that every dataset is an energy scan. Handle explicit dataset
   selection for generic containers and report unsupported filters/links.
6. Keep existing specialized APIs compatible. New bindings and GUI automatic
   import call the Rust dispatcher, with the same mappings and diagnostics.

These are rexafs design choices. Format-specific channel and axis rules will
be traced to fixture headers and verified upstream documentation in the reader
guide. Content recognition is evidence of a layout, not confidence in the
scientific accuracy of a measurement.

## Implementation sequence

### 1. Core and format tests

- Audit the manifest and original headers; create a per-file coverage matrix.
- Implement the public document/mapping contract and strict conversion tests.
- Implement text families, project containers and binary/HDF5 adapters.
- Test every retained file from disk. Check independently recorded scan/point
  counts, units, representative values and channel choices. Exercise malformed
  rows, ambiguous roles, invalid ratios, multiple scans, compression and
  extension-independent detection. Test ordinary CSV/text and generic HDF5.
- Run core tests, strict Clippy, formatting and both fixture integrity/package
  checks before extending the interfaces.

### 2. Python and TypeScript

- Expose path/bytes import in Python and bytes/text import in TypeScript.
- Expose the same document, mappings, conversion and errors, with typed editor
  help describing units, defaults, copying and unprocessed output.
- Build/install both packages; run runtime parity and installed-package editor
  checks. Verify the WebAssembly build, including HDF5 support.

### 3. Desktop and browser interfaces

- Route automatic measurement intake through the core reader.
- Show detected format, scans, available channels and missing mapping details.
- Preserve manual mapping, reference-channel import, project restoration and
  existing application resource limits. Support binary file input in browsers.
- Test automatic and explicit imports, multi-scan selection and useful failure
  messages. Run desktop and browser integration checks.

### 4. Documentation and final qualification

- Publish an unreleased reader guide with a fixture coverage matrix, examples,
  conversion equations, assumptions, source links and explicit limitations.
- Update Rust API comments, Python stubs/docstrings, TypeScript declarations,
  GUI guides and the fixture integration notes; regenerate Next API references.
- Run the relevant repository checks and confirm the original corpus remains
  unchanged and excluded from the published crate. Do not publish or push.

## Initial completion record

This records the first qualification before the Demeter/Larch follow-up below.

Implemented on `test/beamline-fixtures`, without publishing or committing:

- Shared Rust path/byte reader, owned scans/datasets, explicit conversions and
  contextual failures; all 135 attributed corpus files have count regressions.
- Three additional original NIST BMM standards with license, pinned source URLs,
  citations, SHA-256 and reference-channel tests. The original 132 payloads still
  match commit `86133dc` byte for byte.
- Six user-added XTUNES saves/projects, researched against the neighboring
  `xtunes-analysis` checkout. Absorption, all counted result tables, repeated
  settings and project-record boundaries are preserved. Synthetic XTSD, CP932,
  malformed counts and 9809 Mode/fixed-width cases are tested.
- Python and TypeScript/Wasm document, conversion and HDF5 dataset-selection
  APIs with editor help. Browser intake uses the shared Rust reader. The desktop
  provides a dedicated review entry point, materialized groups, undo and portable
  `.rxs` import provenance containing original bytes and saved XTUNES tables.
- Reader guide, per-file coverage CSV, source repository catalog and generated
  Next Python/TypeScript/Rust references. Stable signatures remain unchanged.

Qualification in the macOS ARM64 checkout:

| Check | Observed result |
| --- | --- |
| Full core suite | 292 passed, 3 ignored, including Rust doctests |
| Desktop suite and added import checks | 462 existing tests passed, 5 ignored; two added import tests passed, including real `.rxs` save/reopen with unavailable source paths |
| Installed Python wheel | 19 tests and 30 subtests passed |
| JavaScript/Wasm package | 20 tests passed, including installed tarball type/completion/signature/hover checks |
| Installed Python editor | Pyright type, completion, signature and hover checks passed |
| Browser workspace | Six Playwright tests passed, including XTUNES processing, HDF5 shapes/vector selection, cancellation, Cu parity, accessibility and mobile overflow |
| Documentation | Astro check/build passed; 22 website tests and eight generator tests passed; Stable/Next Rustdoc built |
| Core quality | Strict Clippy and formatting passed |
| Original-data integrity | 135 corpus payloads, 91 retained assets and eight XTUNES payload/attribution files verified |
| Packaging | Cargo's actual file list excludes both fixture directories and all `beamline_*.rs` tests |
| Repository tooling | All 12 tooling suites passed using Python 3.12 |

Support boundaries are explicit in the [reader guide](measurement-reader.md).
Some HDF5 soft-link alias groups are only partially readable; canonical BLISS
instrument arrays remain available with diagnostics. Nonnumeric datasets and
external-link traversal are not implemented. Image reduction, pixel calibration,
unknown channel semantics and ambiguous detector sums require user decisions.
XTUNES processing/fitting reproduction and native standalone XTSD/XTSA fixtures
remain unqualified. Generic CSV is a numeric-table fallback, not a general
spreadsheet/localized-number parser. The desktop's existing folder/recipe import
workflow remains separate; use **Beamline / HDF5…** for the universal reader.

September 14 workflow update: **Import…** and single-file drops now open the
universal reader for every measurement extension. The extra beamline-specific
toolbar button was removed. Folders and multiple-file selections still use the
batch recipe importer; see the current [reader guide](measurement-reader.md).

The tests establish parsing, preservation and software consistency, not the
physical validity of each experimental signal or every dialect at a beamline.

## Demeter follow-up plan

Requested on 2026-09-14. Audit the complete plugin directory and bundled test
files at Demeter commit `06afc8da08a5a7d5a26ee14992170fcf5dc67406`.
Compare SHA-256 hashes before collecting examples: HXMA, CMC, SNBL and PF12C
examples already occur in the retained Larch corpus. Add the four distinct
SSRL ASCII/MicroEXAFS and X23A2 Vortex/four-sample examples with the supplied
Artistic license option, original bytes and pinned URLs.

Improve HXMA channel-label recovery, expose individual SSRL/Vortex detector
choices, and pair the four independent X23A2 incident/transmitted channels.
Qualify Demeter-described BM23 and B18 text variants with clearly labeled
synthetic tests where original examples are unavailable. Preserve offsets,
count-rate channels and all rows; plugin-specific corrections, detector sums,
pixel calibration and archive extraction are separate operations.

Run core regressions, rebuild both bindings, and test representative new
formats through Python, Wasm and the GUI conversion paths. Publish a plugin
audit showing original-fixture coverage, synthetic coverage and remaining
limitations. Also investigate the HDF5 backend's alias limitation and prevent
late desktop reader results from entering a replaced project.

The same audit now includes Larch commit
`e3c93284fed358c2c8979cba4c139430527433c6`: beamline label readers, XDI,
SPEC/NeXus sources, Athena projects and FDMNES reference calculations. Add
representative original files for previously untested layouts and regressions
for the discovered legacy Athena metadata/undefined-array failures. Keep
calculated references and chi(k) data distinct from raw absorption measurements.

## Demeter and Larch follow-up completion

Implemented and qualified on 2026-09-14. The
[format audit](reference-format-audit.md) records every Demeter plugin family,
relevant Larch readers, pinned sources and license decisions. Thirteen original
examples were added with byte hashes and notices: four from Demeter and nine
from Larch. Exact duplicates already retained were reused. The corpus now has
148 attributed files plus six XTUNES inputs: **154 retained reader inputs**.

The core recovers HXMA and APS 12-BM headings, prefers SSRL achieved energy,
offers individual SSRL/Vortex channels and four independent X23A2 sample pairs,
and imports FDMNES relative energy through a declared eV origin. Historical
Athena hash metadata, undefined optional arrays and journal variants are retained
without evaluation. All 79 Athena project examples in the pinned Larch directory
passed a separate local parsing audit; four representative regressions are
retained in the repository. B18/BM23 text variants have synthetic tests, labeled
as such. These checks establish parsing and conversion, not processing parity.

Python, TypeScript/Wasm and both GUI conversion paths support the eV origin.
Desktop results are guarded by import-request and project-generation identifiers
so late reads cannot populate a replaced project. The HDF5 investigation
confirmed the backend's historical soft-alias enumeration limitation; canonical
arrays and partial-result warnings remain the supported behavior. ZIP extraction,
unknown pixel calibration and detector correction/reduction remain separate work.

| Check | Follow-up result |
| --- | --- |
| Core, including doctests | 302 passed, 3 ignored |
| Desktop | 466 passed, 5 ignored; includes FDMNES origin/provenance and independent sample materialization |
| Installed Python wheel | 21 tests passed; installed editor checks passed |
| JavaScript/Wasm | 22 tests passed, including installed package declarations and editor checks |
| Browser workspace | All 24 Playwright tests passed, including seven reader/processing tests and real FDMNES/four-sample import |
| Documentation | Astro check/build, 22 website tests and eight generator tests passed; Next references regenerated |
| Core quality | Strict Clippy, formatting and whitespace checks passed |
| Integrity and packaging | 148 corpus payloads, 91 assets and eight XTUNES payload/attribution files verified; original 132 payload hashes unchanged; fixture/test archive exclusions passed |
| Repository tooling | All 12 tooling suites passed using Python 3.12 |

Changes remain uncommitted and unpublished on `test/beamline-fixtures`.

## Larix follow-up

The subsequent [Larix implementation plan](larix-import-plan.md) and
[import guide](larix-import.md) cover the 15 supplied valid sessions and six
invalid cases. This extends the retained readable-input count to 169, with
complex arrays and session metadata shared by the core, bindings and GUIs.
