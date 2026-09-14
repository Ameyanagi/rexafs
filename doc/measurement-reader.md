# Reading beamline measurements

**Unreleased source-checkout feature.** The universal reader in
[`rexafs::io::reader`](../crates/rexafs/src/xafs/io/reader/mod.rs) supplies the
Rust, Python, TypeScript/WebAssembly, desktop and browser interfaces. Existing
specialized XDI, QAS and Athena APIs remain available.

## Read first, select a spectrum second

Reading returns an owned `Measurement`: scans, original numeric channels,
headers, unit declarations, signal choices, numeric datasets and diagnostics.
Content signatures select an adapter before generic numeric-text fallback;
the filename extension never chooses detector arithmetic. Inspect `warnings`
on both the document and the selected scan.

`arrays()` converts a selected scan to energy in electronvolts (eV) and signal.
Omitting the mapping uses the sole detected signal. Zero or multiple candidates
require an explicit mapping. A recognized format does not establish that its
channels form an absorption spectrum. Image arrays need detector reduction;
uncalibrated encoder/pixel axes need calibration. Neither operation is invented
by the reader.

The reader retains acquisition order and repeated energies. Rust
`to_spectrum()` and Python `spectrum()` sort energy and signal together, retaining
duplicates. TypeScript returns arrays; its strict `Spectrum` constructor and
the browser processing workspace require increasing, unique energy. Choose any
sorting, averaging or removal policy before processing. No normalization,
background subtraction, Fourier transform, detector correction or file write
runs during reading or conversion.

## Scientific conversions

For transmission, the imported signal is optical thickness
`mu = ln(I0 / It)`. `I0` is incident intensity and `It` is transmitted intensity,
in matching units; `ln` is the natural logarithm. The result is dimensionless,
not an absolute absorption coefficient without a sample thickness. For a
reference foil behind a sample, explicitly select `It` as incident and `Ir`
as transmitted. See Newville's
[measurement discussion, section 4](https://docs.xrayabsorption.org/tutorials/XAFS_Fundamentals.pdf).

The implementation evaluates `ln(abs(I0)) - ln(abs(It))` after checking that
both are finite, nonzero and have the same sign. This rexafs convention allows
matching negative electronics polarity and avoids ratio overflow. It does not
establish that negative detector readings are physically appropriate. Opposite
signs and zero intensities fail. A direct stored signal retains its source
scale, including a negative baseline.

Fluorescence or electron yield uses `y = (sum over j of Dj) / I0`, where `Dj`
are the explicitly selected detector channels and `I0` is a nonzero incident
monitor. The signal's scale depends on channel units, geometry and efficiency;
it is proportional to absorption only under the measurement's assumptions,
including suitable thickness/concentration for fluorescence. This reader does
not correct dark current, detector dead time, self-absorption or gains. It never
sums every detector merely because the columns exist. The
[implementation](../crates/rexafs/src/xafs/io/reader/mod.rs) checks selected
indices, distinct roles, equal lengths and finite inputs/results.

An axis in keV is multiplied by 1000. A relative energy axis uses the explicit
`offset_ev` mapping: `E_absolute = E_source + offset_ev`, with every term in eV.
The source axis and signal remain unchanged; increasing the origin shifts the
converted axis upward. The origin must be finite and the result positive and
finite. FDMNES calculation outputs select the header's positive `E_edge` as
this origin, following Larch's
[reference reader](https://github.com/xraypy/xraylarch/blob/e3c93284fed358c2c8979cba4c139430527433c6/larch/io/columnfile.py#L624-L646).
For the retained Mo2C calculation, -25 eV plus a 20,000 eV origin gives 19,975 eV.
This is a file convention, not an independently calibrated experimental edge.
The [adapter](../crates/rexafs/src/xafs/io/reader/text.rs) preserves the original
relative columns and stored `<xanes>` amplitude; it does not reapply `Shift` or
perform convolution. Missing or invalid declared origins are rejected.

For a monochromator angle, first-order
Bragg conversion is `E = hc / (2 d sin(theta))`. Here `E` is energy in eV,
`hc = 12398.419843320026 eV Å`, `d` is the reflecting-plane spacing in angstroms
(Å), and `theta` is the Bragg angle, not twice that angle. The selected axis is
multiplied by `degrees_per_unit` before converting degrees to radians for the
sine. Use 1 for degrees, 180/π for radians, 0.001 for millidegrees, or the
reciprocal of motor steps per degree. Positive finite `d` and scale, with
`0 < theta <= 90°`, are required. Increasing `d` reduces the derived energy;
no angular offset is inferred. This follows the
[IUCr Bragg-law definition](https://dictionary.iucr.org/Bragg%27s_law), with
first-order reflection and rexafs' stated physical constant.

9809 uses the observed angle and header `D`, excluding numeric scan-plan rows.
Its recorded modes identify incident (1), transmission (2), fluorescence (3),
and yield (4) channels. Each supported detector becomes a separate choice;
auxiliary modes 101/103 do not become detector contributions. Counts, dwell
time and offsets are retained without applying an offset again. Optional
filename/date text is opaque and cannot block usable data. The fixture-backed
rules and fixed-width handling are in
[`text.rs`](../crates/rexafs/src/xafs/io/reader/text.rs); the original
[9809 format definition](https://titan.nusr.nagoya-u.ac.jp/dokuwiki/lib/exe/fetch.php?media=tabuchi:fileformat.pdf)
and [XTUNES research record](https://github.com/Ameyanagi/xtunes-analysis/blob/e320ed77469d848cd0b99125deb810f3f34ccb91/docs/xtunes-9809-rexafs-support.md)
provide supporting context. The latter repository is private.

## Examples

The following APIs are unreleased. Existing zero-based numeric mappings remain
valid. Explicit selectors also accept exact, case-sensitive column names from
`scan.columns`; names are resolved separately for each scan, so reordered columns
do not change their meaning. Missing or duplicate names produce an error; use an
index to distinguish repeated labels. Names are not detector aliases: `I0` and
`i0` select different labels, and the string `"1"` is a name, not index 1.

Rust, from a source checkout:

```rust,no_run
use rexafs::io::read_measurement;
let document = read_measurement("measurement.dat")?;
let scan = &document.scans[0];
println!("{:?}", scan.warnings);
// None succeeds only when there is exactly one detected signal.
let (energy_ev, signal) = scan.arrays(None)?;
# Ok::<(), rexafs::io::ReadError>(())
```

For QAS columns named `energy`, `i0`, `it`, `ir` and `iff`, select each signal
explicitly with the new `SpectrumSelection` API:

```rust,no_run
use rexafs::io::{read_measurement, SpectrumSelection};
let document = read_measurement("qas.dat")?;
let scan = &document.scans[0];
let transmission = scan.arrays_with(&SpectrumSelection::transmission("energy", "i0", "it"))?;
let fluorescence = scan.arrays_with(&SpectrumSelection::fluorescence("energy", "i0", ["iff"]))?;
let reference = scan.arrays_with(&SpectrumSelection::transmission("energy", "it", "ir"))?;
// Indices still work, and may be mixed with names.
let same_transmission = scan.arrays_with(&SpectrumSelection::transmission(0usize, "i0", 2usize))?;
# Ok::<(), rexafs::io::ReadError>(())
```

`SpectrumSelection::direct("energy", "mu")` copies stored absorption.
`selection.resolve(scan)` returns the existing numeric `SpectrumMapping`, whose
struct fields and `scan.arrays(Some(&mapping))` API are unchanged. The constructors
retain the selected axis's detected calibration, including Bragg and relative
energy conversions. Unknown or conflicting calibration requires
`.with_energy(EnergyConversion::Ev)` or another explicit conversion. No units are
guessed from the numerical values.

Python, after building/installing the source wheel:

```python
from rexafs.io import read_measurement
energy_ev, mu = read_measurement("measurement.dat").arrays()
```

This uses the first scan and its sole detected signal. For a multi-scan session,
pass `scan=1` to select the second scan. If several signals are detected, inspect
their names and pass the chosen candidate's `mapping` to `arrays()`. Only files
with ambiguous or missing metadata need custom column roles, for example:

```python
from rexafs.io import read_measurement, SpectrumMapping
measurement = read_measurement("measurement.dat")
print(measurement.document["scans"][0]["signals"])
mapping: SpectrumMapping = {
    "energy_column": 0,
    "energy": {"kind": "kev"},
    "signal": {"kind": "transmission", "incident": 1, "transmitted": 2},
}
energy_ev, mu = measurement.arrays(scan=0, mapping=mapping)
```

For common selections, column keywords avoid writing the nested mapping:

```python
measurement = read_measurement("qas.dat")
transmission = measurement.arrays(energy="energy", i0="i0", it="it")
fluorescence = measurement.arrays(energy="energy", i0="i0", iff="iff")
reference = measurement.arrays(energy="energy", i0="it", it="ir")
spectrum = measurement.spectrum(energy=0, i0="i0", it=2)
```

Use `mu="mu"` for stored absorption or `iff=["iff1", "iff2"]` for an explicit
detector sum. Supply `energy` and exactly one of `mu`, `it`, or `iff`; the last
two also require `i0`. `energy_unit="eV"` or `"keV"` overrides the detected
calibration; omit it to retain that calibration. The nested mapping also accepts
names, for example `"energy_column": "energy"` and `"incident": "i0"`.
Do not combine a mapping dictionary with column keywords.

TypeScript, after building/installing the source npm package:

```ts
import init, { read_measurement, type SpectrumMapping } from "rexafs";
await init(); // Required in browsers; harmless in Node.
const measurement = read_measurement("energy (keV),mu\n7.1,1\n7.2,2\n");
try {
  const { energy, mu } = measurement.arrays(); // Independent Float64Arrays.
  console.log(energy, mu, measurement.document.warnings);
} finally {
  measurement.free();
}
```

TypeScript supports the same selectors in an options object:

```ts
const measurement = read_measurement("energy,i0,it,ir,iff\n7100,100,20,5,3\n7101,200,40,10,8\n");
try {
  const transmission = measurement.arrays({ energy: "energy", i0: "i0", it: "it" });
  const fluorescence = measurement.arrays({ energy: "energy", i0: "i0", iff: "iff" });
  const reference = measurement.arrays({ energy: "energy", i0: "it", it: "ir" });
  const sameTransmission = measurement.arrays({ scan: 0, energy: 0, i0: "i0", it: 2 });
} finally {
  measurement.free();
}
```

Use `mu` for stored absorption, an `iff` array for a detector sum, and
`energy_unit: "eV"` or `"keV"` only when overriding the source calibration.
The existing `arrays(scan, mapping)` overload also accepts names in its mapping.
Options cannot be combined with a positional mapping. With no column options,
automatic selection still requires exactly one detected signal.

Python also accepts `parse_measurement(bytes_or_text)`; TypeScript
`read_measurement()` accepts `Uint8Array` (including Node `Buffer`) or text.
Pass bytes for HDF5, gzip and legacy encodings. Snapshot dictionaries/objects
are independent copies; editing them does not mutate native data. Nonfinite raw
cells appear as `None`/`null` in JSON snapshots and remain nonfinite internally.
Selected nonfinite values fail conversion. Python objects release memory through
normal object lifetime; TypeScript callers must call `free()`.

For generic HDF5, use `measurement.select_datasets(["/energy", "/detector"])`
in either binding, then select columns in the returned scan index. Rust uses
`document.dataset_scan(&paths)` to return a new scan without changing the
document. Select at least two distinct, nonempty one-dimensional arrays with
equal lengths. Paths determine column order, even across different groups.

## Format coverage and limits

The [per-file coverage table](../crates/rexafs/tests/measurement_fixtures/coverage.csv) records measured
reader results for all 154 files in the attributed beamline/reference corpus: format,
scan point counts, signal-choice counts, dataset counts, diagnostics and original
source URLs. It is a checkout qualification snapshot, not an independent
scientific reference or a claim to support every dialect at a facility.
The [source repository catalog](beamline-source-repositories.md) records research
and license decisions, including three added original BMM standards and thirteen
Demeter/Larch examples. Together with six XTUNES files and 15 valid Larix
sessions, **175 retained inputs** are read in automated tests. The new ESRF BM16 BLISS file has partial group coverage with explicit warnings; readability does not imply complete detector recovery. Six additional
damaged/inconsistent Larix files are expected to fail. The [reference-format audit](reference-format-audit.md)
distinguishes original fixtures, synthetic coverage and unimplemented formats.

The [expanded corpus audit](validation/2026-09-14-measurement-corpus/review.md)
adds a self-contained copy of 222 files gathered in `rexafs-format`, including
89 payloads not present in the original corpus. All 222 enter normal fixture
tests: 182 parse (five with partial HDF5 recovery), while 40 have explicit
rejection expectations for unsupported formats. Across both beamline snapshots
there are 243 unique payloads; a copied file or a passing rejection test is not
counted as successful format support. Original bytes and attribution remain in
the Git repository and are excluded from published packages.

| Family | Imported content and selection behavior |
| --- | --- |
| XDI, QAS/ISS and named beamline text | Header labels/units and detector channels. Stored absorption takes precedence; ambiguous channels require selection. Contradictory XDI column counts and SAMBA sample annotations disable automatic arithmetic. |
| SPEC/Sardana, FIO, EPICS/LabVIEW, MRCAT | Numeric columns and headers; SPEC scan boundaries remain separate. Unnamed or unfamiliar channels require manual roles. |
| 9809, Lytle | Crystal calibration and original detector columns; Mode-driven 9809 choices and Lytle step calibration. |
| SRS, SSRL ASCII, LNLS, EX3, Ritsumeikan | Wrapped SRS detector records are joined; SRS axis calibration remains explicit. LNLS date/time cells retain row order in metadata. Ritsumeikan's intentional empty spacer is retained. |
| HXMA, SSRL MicroEXAFS, X23A2 multichannel | Preserve process-variable labels, SCA/ICR and fast/slow channels. Offer individual fluorescence channels and independent sample transmission pairs. SSRL ASCII prefers achieved energy. No dead-time correction or detector sum is inferred. |
| FDMNES calculation outputs | Preserve source-relative energy and stored calculated amplitudes; use declared `E_edge` for conversion to absolute eV. These are reference calculations, not acquired measurements. |
| Historical X10C, X15B and SSRL binary | Independently implemented legacy layouts, preserving numeric detector tables; explicit choices where needed. |
| Athena Perl/JSON, including gzip | All project groups and available x/y, monitor, signal and uncertainty arrays. Pixel and non-absorption axes remain unmapped. Saved parameters are metadata, not active rexafs processing settings. |
| Larix 1.0 sessions, including gzip | Ordered Groups, stored energy/mu, original detector arrays and independent saved grids. Complex arrays retain real/imaginary components; exact dtype bytes and inert session text remain archived. |
| HDF5/NeXus | Numeric datasets with paths, attributes and shapes; equal-length vectors can become channel sets. Canonical BLISS instrument vectors are recovered. No automatic image integration or external-file resolution. |
| XTUNES XTS/XTSP, synthetic XTSD | Stored E/Mu and every counted saved table, with independent grids and ordered settings. Project records remain separate. |
| Generic text/CSV | Rectangular numeric whitespace/comma tables with optional headers, inline # comments and Fortran D exponents. Unlabeled columns need explicit axis units and roles. This is not a general spreadsheet or localized decimal-comma parser. |

An undeclared but recognized energy label defaults to eV **with a warning**;
the documented ZapEnergy and BM23 text conventions use keV when units are absent.
Explicit units take precedence. APS 12-BM uses its numbered heading's units.
Unlabeled axes never use an
energy-magnitude heuristic. Override assumptions when the experiment differs.

The shared HDF5 backend is `hdf5-pure` 0.45.0 on native and WebAssembly. HDF5 is
a container, and the [NeXus NXxas definition](https://manual.nexusformat.org/classes/applications/NXxas.html)
does not make arbitrary detector arrays self-interpreting. Nonnumeric datasets,
references and external/soft-link traversal are outside this reader's contract.
Some historical BLISS alias groups cannot be enumerated by this backend; the
reader reports their paths and returns canonical instrument arrays from other
groups. Such results are partial and must be reviewed. Unsupported numeric
filters and corrupt numeric payloads return errors. Values are represented as
f64, so integers beyond 2^53 may lose exact integer precision.

Input and gzip expansion are each limited to 256 MiB; decoded HDF5 numeric
arrays have a separate 256 MiB budget. These limits do not bound total process
memory: documents, snapshots and conversions own copies. Deep/cyclic HDF5 group
walks are bounded. Gzip requires one complete member; nested gzip, additional
members and trailing data are rejected. The reader makes no network calls
and does not execute settings, constraint expressions or embedded source paths.
Legacy Athena nested hashes and unrecognized journals are retained as opaque
metadata. Optional arrays written as `(undef)` are absent, not fabricated zeros.
ZIP archives, unknown pixel calibrations and multidimensional detector reduction
remain outside this import contract; see the format audit for specific gaps.

## XTUNES records and saved results

The six [original XTUNES fixtures](../crates/rexafs/tests/fixtures/sessions/xtunes/README.md)
were saved and reopened in XTUNES 1.3 Build20200228. They contain 818-point
BL12C and 1,420-point PF9A absorption records. XTUNES had already removed six
PF9A duplicate energy positions; rexafs does not remove additional points.
Standalone application-generated XTSD, XTSA scattering references, localized
numeric dialects, and XTUNES fitting/processing reproduction are unqualified.

The [adapter](../crates/rexafs/src/xafs/io/reader/xtunes.rs) uses `BG Plot` E/Mu
or `QD Plot` E/Mu directly. It does not subtract the saved Back or Base columns.
Every saved plot and embedded function table is exposed as a shaped dataset
under `/record_N/table_name`, with headings and quantity descriptions. Xi,
weighted ED, Fourier components/magnitude and empty optional tables remain
separate. A chi-only record has no automatic absorption mapping. `Pow` is saved
magnitude, checked against the square root of Cos² + Sin²; it is not squared
power. This test establishes serialized array consistency, not Fourier phase
or algorithm equivalence with rexafs.

`metadata["ordered_parameters"]` is a JSON array of `(section, key, value)`
triples. Repeated parameter/error keys remain distinct. The complete normalized
record text is also retained in `header`; embedded Windows paths are provenance
strings. UTF-8/BOM and valid Shift-JIS/CP932 text are accepted without replacing
undecodable bytes silently.

## Desktop and browser use

In the desktop, choose **Import…** or drop a single measurement file. Text,
beamline files, Athena projects, XTUNES and Larix sessions use the same plotted
preview. **Project → Open measurement…** uses a single-file picker for this view.
Choose a record from **Scan** when the file contains several. Detected signals
appear as visible checkboxes, initially checked when their conversion is valid.
For QAS files with `i0`, `it`, `ir` and `iff`, the choices are **Transmission**
(ln(I0/It)), **Fluorescence** (IFF/I0), and **Reference** (ln(It/Ir)). These are
the detector operations described above; no detector corrections are inferred.
Each row shows the actual source column names in its formula. Uncheck outputs
you do not need, then use **Preview** to inspect a curve or edit its columns.
Switching previews keeps each signal's mapping and does not toggle its checkbox.
**Reset mapping** restores the previewed signal's detected mapping.

The plot updates when the selected scan, detector columns or axis units change.
**Import N spectra** adds all checked, unprocessed spectra together, with distinct
signal names and source mappings. The main Data view initially shows raw μ(E)
for the first imported spectrum. One undo removes the entire import. No groups
are added if any checked conversion fails; invalid signals can be excluded.
**Custom mapping…** provides a single manual output when detection is insufficient.

**Energy axis** shows the detected conversion. Recognized 9809 inputs use the
observed Bragg angle and header crystal spacing; recognized motor-step and
relative-energy calibrations retain their source conversion. Manual overrides
include eV, keV, degrees and radians. Angular overrides require a positive
crystal-plane spacing `d` in Å. Missing units require an explicit selection.
**Columns** exposes the selected axis and detector roles; advanced mappings in
the core API also support arbitrary degrees-per-unit and relative-energy origins.

**Source details** is collapsed initially and shows the original header, path,
format, warnings and the first source rows. Unsupported HDF5 group warnings are
also flagged above the preview. **Scan → Select datasets…** allows explicit
selection of equal-length real HDF5 vectors. Complex arrays and detector images
are retained as evidence and cannot be treated directly as absorption signals.

Saved projects embed the selected record, mapping, original file bytes and
applicable archived result tables, so loading the spectrum does not depend on
the old source path. Desktop review accepts inputs up to 256 MiB. Folder and
multi-file recipe imports retain the existing workflow; those batch recipes do
not yet use the universal container reader. Import a container as a single file.
Starting another file read clears the previous review, so a failed load cannot
add a record from the old file.

The browser workspace accepts bytes, offers the same scan/signal and HDF5
vector selection, and performs conversion in its WebAssembly worker. It shows
the detected signal and collapses **Custom columns and units** for a sole
candidate. Ambiguous inputs require choosing a signal or confirming custom
columns before processing. It exposes source headers (preview limited to 32,768
characters), warnings and saved table shapes. Processing retains its 8 MiB source
limit, 100,000-point limit and
numerical workspace guards. Unordered/duplicate energy is reported without
silently dropping rows. Cancelling terminates the active worker. Exported
provenance records the original byte SHA-256, selected scan/datasets and mapping.

## Validation

`measurement_fixtures/corpus.rs` reads every attributed measurement; `measurement_fixtures/xtunes.rs`
checks all six XTUNES files, counts, endpoint values, independent saved grids,
project boundaries, repeated settings, CP932 and corrupt counted tables.
`measurement_fixtures/reference.rs` checks original Demeter/Larch numeric values, detector
pairing, relative-energy calculations and historical Athena metadata.
`measurement_fixtures/larix.rs` checks all 15 valid sessions, all 384 saved numeric arrays
against independent full-data hashes, and six damaged/inconsistent sessions.
`measurement_reader.rs` covers synthetic formats, units, explicit detector
arithmetic, damaged inputs and generic HDF5 dataset selection. Existing XDI,
QAS and Athena regressions remain active. Binding tests run real inputs through
the installed Python wheel and built WebAssembly package. Editor checks verify
the installed declarations; GUI tests exercise intake and browser processing.
Fixture integrity and Cargo package checks ensure retained measurement bytes
and attribution remain intact and outside the crate archive.

Robustness tests also reject every incomplete gzip prefix of a real Larix
session, bad checksums, concatenated members, trailing bytes, duplicate JSON
keys and invalid session encodings. Browser tests cover failed imports followed
by successful recovery, with previous exports disabled. These checks do not
prove that a syntactically valid text file contains an entire acquisition: a
file ending after a complete row may lack any declared total to validate.

See the [Larix import guide](larix-import.md) for modern and legacy array
encodings, exact integer recovery, complex components, inert metadata and
session-specific resource limits. Larix saved processing is archived rather
than installed as rexafs processing state.

## Japanese Quick-XAFS extensions

The [current desktop screenshots and capture record](validation/2026-09-14-measurement-import-ui/README.md)
show the plotted EX3 preview, expandable source header and KEK angle conversion
in the unreleased source build. Released-version screenshots remain versioned
separately in the public manual.

Angles are supported independently of file extensions. Actual KEK-PF `.qd`
files downloaded from BL12C, BL9C, BL9A and NW10A use the 9809 layout. Four
transmission examples passed original-column, Bragg-energy and absorption-ratio
checks; they import through the existing 9809 reader without an extension-specific
adapter. See the [five-file audit](validation/2026-09-14-kek-qd/review.md).
A BL9A fluorescence example has transmission-only detector Mode codes: the
reader retains its columns and warns that the signal arithmetic must be selected
explicitly. Detector columns do not inherit angle units from the scan-plan header.
A `.qd` suffix by itself does not establish a layout. The
[KEK-PF QXAFS v8 manual](https://pfxafs.kek.jp/wp-content/uploads/bldata/QXAFS_v8.pdf)
identifies `.qc` as measurement conditions, not a spectrum. Import of those
condition files remains unimplemented. The five originals are retained as
[repository fixtures](../crates/rexafs/tests/fixtures/xas/candidates/kek-pf/README.md)
for academic, nonmilitary research and reader regression testing only, with the
original KEK usage notice and experimenter attribution. Publication citation
requires contacting the experimenter. They are excluded from published packages;
no standard open-data license is assigned.

REX2000 `.ex3` records use the two numeric fields between `[EX_BEGIN]` and
`[EX_END]` as energy in eV and absorption. Crystal metadata outside that body
remain header information and are not used as column names. This interpretation
follows Appendix III, format 2, of the Rigaku REX2000 manual MJ13242B02
([archival mirror](https://123deta.com/document/yjdxwmpy-cat.html)); the
[EX3 regression](../crates/rexafs/tests/measurement_fixtures/reference.rs) checks
original values, declared units and header retention.
