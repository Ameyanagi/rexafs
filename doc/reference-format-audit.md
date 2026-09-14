# Demeter and Larch format audit

Audited on 2026-09-14 for the **unreleased** universal reader. These are reference
implementations and example collections, not an exhaustive description of current
beamline acquisition software. File counts describe retained payloads, not the
number of independent formats or scientifically validated spectra.

## Sources and collection decision

- [Demeter](https://github.com/bruceravel/demeter/tree/06afc8da08a5a7d5a26ee14992170fcf5dc67406),
  commit `06afc8da08a5a7d5a26ee14992170fcf5dc67406`: inspected the complete
  `lib/Demeter/Plugins` directory, `t/filetypes` and multichannel/plugin recipes.
  Its [license](https://github.com/bruceravel/demeter/blob/06afc8da08a5a7d5a26ee14992170fcf5dc67406/LICENSE)
  offers the same terms as Perl; the retained fixtures use its supplied Artistic
  license option.
- [Larch](https://github.com/xraypy/xraylarch/tree/e3c93284fed358c2c8979cba4c139430527433c6),
  commit `e3c93284fed358c2c8979cba4c139430527433c6`: inspected `larch/io`,
  beamline-reader tests, data documentation and `examples/xafsdata`.
  Bundled examples use the repository's
  [MIT license](https://github.com/xraypy/xraylarch/blob/e3c93284fed358c2c8979cba4c139430527433c6/LICENSE).
  Credit Matthew Newville, Mauro Rovezzi, Bruce Ravel, Margaret Koker,
  Ryuichi Shimogawa and contributors, with original file-level credit preserved.

The collection records these as repository distribution licenses. A separate
creator-issued data license was not located for these bundled examples.
Original notices, source URLs, commit identifiers and SHA-256 checksums are in
the [manifest](../crates/rexafs/tests/fixtures/xas/manifest.json), adjacent
`.license` files and [citation catalog](../crates/rexafs/tests/fixtures/xas/CITATIONS.md).
No upstream implementation source was copied into the reader.

Thirteen originals were added: four from Demeter and nine from Larch. Several
Demeter examples already occur byte for byte in the retained collection; those
were not collected again. The three added Demeter SSRL/Vortex tables have
different byte hashes from their existing Larch counterparts but represent the
same format families. The fourth is a four-sample acquisition example. The
corpus now contains **148 attributed files**, plus **six separately attributed
XTUNES files**, for **154 retained reader inputs**. All 132 original corpus
payloads remain unchanged. All 154 are read in automated tests; some expose raw
arrays requiring mapping, calibration or detector reduction.

Subsequent work added [Larix 1.0 session import](larix-import.md) using the
separately supplied 15 valid sessions and six invalid cases. The current reader
total is 169 valid inputs; the 148-file attributed corpus above is unchanged.

## Demeter plugin findings

The plugin identifiers below link through the pinned
[plugin directory](https://github.com/bruceravel/demeter/tree/06afc8da08a5a7d5a26ee14992170fcf5dc67406/lib/Demeter/Plugins).
The rexafs rules are implemented in
[`beamlines.rs`](../crates/rexafs/src/xafs/io/reader/beamlines.rs),
[`text.rs`](../crates/rexafs/src/xafs/io/reader/text.rs) and
[`binary.rs`](../crates/rexafs/src/xafs/io/reader/binary.rs).

| Plugin or family | Reader coverage and deliberate limits |
| --- | --- |
| `SSRLA` | Original SSRL tables, including newly retained `ssrla.dat`. Prefer the achieved-energy column when both requested and achieved energy are present. Retain offsets and detector counts; do not subtract offsets a second time. |
| `SSRLB`, `X10C`, `X15B` | Original legacy binary examples were already retained. Recover numeric tables through independent binary layout adapters. Unfamiliar detector roles remain explicit. |
| `SSRLmicro` | `ssrlmicro.dat` has 296 rows and 69 columns, including 32 single-channel analyzer (SCA) values and 32 input count rates (ICR). Offer each SCA divided by I0 separately. Preserve ICR without applying dead-time correction or summing detectors. Station attribution remains unresolved. |
| `X23A2MED` | `x23a2med.dat` has 422 rows and 17 columns. Offer transmission and four individual fluorescence ratios; retain fast/slow count rates and offsets without correction. |
| `X23A2MultiChannel` | `re4chan.000` has 387 rows and 12 columns. Four incident/transmitted pairs represent independent samples, so they produce four transmission choices. The reference channel is retained without assuming its geometry. |
| `HXMA` | Recover actual quoted process-variable labels, including Event-ID, from the existing CLS acquisition example. Known BL1606-I channels offer transmission and fluorescence choices; unfamiliar labels require mapping. Raw rows and channel values are retained. |
| `CMC`, `10BMMultiChannel` | Existing MRCAT tables retain raw and precomputed channels. CMC nonfinite values are not replaced by zero. Configurable 10-BM detector pairings and corrections are not inferred from an arbitrary table. |
| `SPEC`, `SpecFileLongLine` | Existing SPEC/Sardana/SNBL examples preserve scan boundaries and all columns. Long labels do not justify dropping data columns. |
| `SRS`, `DUBBLE` | Original SRS examples include wrapped detector rows. The DUBBLE angle-to-energy calibration remains an explicit Bragg mapping; the reader does not assume a crystal spacing from the facility name. |
| `PFBL12C`, `Lytle` | Original files exercise 9809 observed-angle/header calibration and Lytle motor-step calibration. Detector modes remain separate choices. |
| `LNLS` | The retained original table preserves date/time cells as metadata and numeric channels in acquisition order. |
| `BM23` | Synthetic text tests cover the documented signature, energy labels, explicit eV and fallback keV, and a trailing separator. Existing BM23 HDF5 examples exercise a different container layout. No original example of this plugin's ASCII dialect was found in `t/filetypes`. |
| `B18` | Synthetic tests retain every row and all 43 columns, rather than reproducing the plugin's optional subsampling or detector summation. No original B18 example was found in `t/filetypes`. |
| `BL8Ar` | The existing SLRI BL8 text example is readable. The plugin's argon correction is a processing operation and is not applied during import. |
| `SLRIBL4` | Its pixel-energy polynomial depends on external configuration. Generic numeric arrays remain inspectable, but automatic pixel calibration is unqualified without a calibration record and an original fixture. |
| `Zip` | Archive extraction is not implemented. Extract the intended measurement before reading it. Gzip-compressed measurements and Athena projects have separate supported adapters. |
| `FileType`, `Beamlines/BL8`, `Beamlines/MX`, `Beamlines/X11A`, `Beamlines/XDAC` | Plugin infrastructure or metadata helpers; these are not additional numeric payload formats. |

Detector conversions are rexafs choices documented in the
[reader guide](measurement-reader.md). Reader agreement does not validate a
detector correction, establish sample geometry or reproduce Demeter processing.

## Larch reader findings

The pinned [I/O directory](https://github.com/xraypy/xraylarch/tree/e3c93284fed358c2c8979cba4c139430527433c6/larch/io)
contains more than absorption-scan readers. The useful format references are:

| Reference | Findings and rexafs coverage |
| --- | --- |
| `columnfile.py`, `xafs_beamlines.py` | Generic numeric ASCII and named APS GSE/12-BM/MRCAT/XSD, NSLS XDAC, SSRL, CLS HXMA and KEK Photon Factory headings. Existing fixtures cover these families. The APS 12-BM numbered heading now supplies all 26 labels and its declared keV axis. |
| `xdi.py` | XDI metadata, arrays and multielement detector columns. Added `fe_xanes_8ch.xdi`: 100 rows, 39 columns, region-of-interest counts, clocks and dead-time factors. Selected detector sums are explicit; factors are retained without being applied again. |
| `columnfile.py:read_fdmnes` | Added both FDMNES Mo2C calculation outputs, with 559 and 589 rows. Preserve source-relative energy and use declared `E_edge` when converting to absolute eV. The convolved output remains a stored result; no convolution or additional `Shift` is applied. |
| `athena_project.py` | Historical Perl and JSON project arrays and settings. Added four projects covering empty-hash journals, nested metadata hashes, optional `(undef)` arrays and double-quoted journals. Opaque metadata is preserved, never evaluated. |
| Generic processed references | Added `nonuniform.chi` (476 rows) and `generic_columns_no_header.dat` (198 rows, seven columns). A chi(k) axis is not absorption energy; an unlabeled table supplies no automatic detector mapping. |
| `specfile_reader.py`, `nexus_xas.py`, `hdf5group.py` | Useful references for SPEC/Sardana and NeXus/ESRF/SOLEIL dataset layouts. Existing HDF5 fixtures expose numeric paths, shapes and equal-length vector selection. The shared backend still has limitations on soft aliases, external links and nonnumeric types. |
| `stepscan_file.py` and EPICS ASCII | Existing acquisition tables use the text fallback and known labels; unfamiliar detector names require manual selection. A filename alone does not establish channel roles. |
| XRF/MCA, detector maps, netCDF and TIFF/XRD/RIXS | These represent detector spectra, images or other experimental dimensions. They are outside the current absorption conversion contract. Generic HDF5 numeric arrays may be inspectable, but detector reduction is not implied. |
| `save_restore.py`, `jsonutils.py`, `npjson.py` | The subsequent [Larix adapter](larix-import.md) reads 1.0 sessions, stored absorption and archived arrays/metadata. It does not restore an executable Python environment or every saved object type. |

The [FDMNES reference reader](https://github.com/xraypy/xraylarch/blob/e3c93284fed358c2c8979cba4c139430527433c6/larch/io/columnfile.py#L624-L646)
adds `E_edge` by default. rexafs implements this as an explicit mapping:
`E_absolute = E_source + E_edge`, with all three quantities in eV. `E_source`
is the original relative axis; `E_edge` is the declared origin. Increasing the
origin shifts the converted axis upward without changing the signal or raw
columns. A missing/nonfinite origin is not guessed. A generic `offset_ev`
mapping also permits other finite origins when all converted energies are
positive and finite. This is a serialization convention, not an independently
calibrated experimental edge energy.

The legacy Athena compatibility check additionally read **all 79 `.prj` files**
in Larch's `examples/xafsdata/AthenaProjectFiles` directory through the installed
Python binding at the pinned revision: 79 readable, zero failures. That was a
local audit, not 79 new retained regression fixtures. It checks parsing, not
scientific agreement with Larch. Four representative failures were retained
and now run offline in the normal core suite. The parser does not execute Perl,
including embedded expressions in journals; unknown journal text is archived.

## Qualification and next collection targets

[`measurement_fixtures/reference.rs`](../crates/rexafs/tests/measurement_fixtures/reference.rs) checks
original row counts, energy endpoints, detector arithmetic, relative axes and
legacy metadata. [`measurement_reader.rs`](../crates/rexafs/tests/measurement_reader.rs)
adds malformed-input, explicit calibration, synthetic B18/BM23 and nesting-limit
tests. The [per-file table](../crates/rexafs/tests/measurement_fixtures/coverage.csv) supplies counts and
source URLs for all 148 corpus inputs. Python, Wasm and both GUI conversion paths
exercise representative new originals.

Future format collection should target original B18/BM23 ASCII, SLRI BL4 with
its actual pixel calibration, configurable 10-BM detector layouts and additional
NeXus link/filter variants. These gaps are not covered by increasing the count
of ordinary two-column reference spectra. ZIP input and multidimensional detector
reduction require separate designs. The present reader can expose partial HDF5
results with warnings; it must not describe them as complete recovery.
