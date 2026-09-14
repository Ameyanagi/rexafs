---
title: Reading measurements
description: Automatic beamline, HDF5, Athena, Larix and XTUNES import since 0.2.6.
audience: user
pagefind: false
---

**Next API · source checkout.** The shared reader is available since 0.2.6.
Use the [Stable reader guide](/docs/reference/stable/measurement-reading/)
for the published release.

## Unreleased reader corrections

- Detector roles must match a unique column. Repeated headings or competing
  aliases require explicit selection. Each stored absorption column remains a
  separate signal choice; repeated labels show their one-based column number.
- Malformed leading numeric rows produce a line-specific error even when
  comments separate them from later valid rows.
- Athena export retains historical `(undef)` markers before current optional
  arrays, so edited `i0`, `signal` and `stddev` values survive saving and reopening.

These corrections are not included in the 0.2.6 packages. The Rust core supplies
the same reader behavior to Python, TypeScript, desktop and browser imports.

## Inspect and select

The universal reader detects formats from file content and returns an owned
document with scans, headers, original columns/units, signal choices, numeric
datasets and warnings. It supports beamline text, ordinary numeric CSV, Athena,
XTUNES, Larix sessions, FDMNES calculation outputs and HDF5. File extensions do not determine
detector arithmetic.

```python
from rexafs.io import read_measurement
measurement = read_measurement("measurement.dat")
print(measurement.document["scans"][0]["signals"])
energy_ev, signal = measurement.arrays()
```

Omitting a mapping uses the sole detected signal. Zero or multiple choices
require an explicit selection. For example, after confirming the column order:

```python
mapping = {
    "energy_column": 0,
    "energy": {"kind": "kev"},
    "signal": {"kind": "transmission", "incident": 1, "transmitted": 2},
}
energy_ev, mu = measurement.arrays(scan=0, mapping=mapping)
```

Indices are zero-based. Names can be used wherever a mapping takes a column
index. They must match `scan.columns` exactly, including case; duplicate names
require indices. Names and indices may be mixed, and names are resolved for
each scan independently.

For QAS columns named `energy`, `i0`, `it`, `ir` and `iff`, Python also supports
short column keywords:

```python
transmission = measurement.arrays(energy="energy", i0="i0", it="it")
fluorescence = measurement.arrays(energy="energy", i0="i0", iff="iff")
reference = measurement.arrays(energy="energy", i0="it", it="ir")
spectrum = measurement.spectrum(energy=0, i0="i0", it=2)
```

Supply `energy` and exactly one of `mu`, `it` or `iff`; `it` and `iff` require
`i0`. `mu` copies stored absorption, and an `iff` list sums explicitly selected
detectors before dividing by `i0`. Omit `energy_unit` to retain the detected
axis calibration, including Bragg and relative energy conversions. Set it to
`"eV"` or `"keV"` for an explicit override. Unknown units require a choice.
Column keywords and a mapping dictionary cannot be combined.

TypeScript uses the same mapping object or an options object:

```ts
import init, { read_measurement } from "rexafs";
await init();
const measurement = read_measurement("energy (keV),mu\n7.1,1\n7.2,2\n");
try {
  const { energy, mu } = measurement.arrays();
  console.log(energy, mu, measurement.document.warnings);
} finally {
  measurement.free();
}
```

For detector data, use `measurement.arrays({energy: "energy", i0: "i0", it: "it"})`.
Use `iff: "iff"` instead of `it` for fluorescence, or `i0: "it", it: "ir"` for
the reference. `scan` defaults to 0; all column options accept names or indices.
The existing `arrays(scan, mapping)` overload remains available. No column
options means automatic detection, which requires a sole detected signal.

Python accepts paths or `parse_measurement(bytes_or_text)`. TypeScript accepts
text or `Uint8Array`, including Node `Buffer`. Rust provides
`rexafs::io::read_measurement(path)` and `parse_measurement(bytes)`, followed by
`scan.arrays(mapping)` or `scan.to_spectrum(mapping)`.

Rust also provides `SpectrumSelection::direct(energy, mu)`,
`SpectrumSelection::transmission(energy, i0, it)` and
`SpectrumSelection::fluorescence(energy, i0, detectors)`. Pass the result to
`scan.arrays_with(&selection)` or `scan.to_spectrum_with(&selection)`.
Constructors accept names and indices; `.with_energy(EnergyConversion::Ev)`
overrides calibration. `.resolve(scan)` returns an indexed `SpectrumMapping`,
whose existing numeric struct fields remain unchanged.

Snapshots and converted arrays are independent copies. Nonfinite raw cells
appear as `None`/`null` in snapshots; selected nonfinite values fail conversion.
No normalization, background subtraction, Fourier processing, file changes or
detector corrections run during reading.

## Units and signals

Transmission calculates dimensionless optical thickness `ln(I0 / It)`, where
`I0` and `It` are incident and transmitted intensities in matching units.
This is not an absolute absorption coefficient without sample thickness.
rexafs accepts nonzero intensities with the same sign, allowing matching
negative electronics polarity. Fluorescence/yield uses the explicitly selected
detector sum divided by a nonzero incident monitor. Its scale and physical
interpretation depend on detector units, efficiency, geometry and the sample.
See [Newville, section 4](https://docs.xrayabsorption.org/tutorials/XAFS_Fundamentals.pdf)
for measurement assumptions. Dark current, dead time, gains and self-absorption
are not corrected automatically.

keV is converted to eV by multiplying by 1000. Relative energy uses
`E_absolute = E_source + offset_ev`, with all terms in eV. The finite origin
shifts the converted axis without changing the raw columns or signal; the
result must remain positive and finite. Use `{"kind": "offset_ev", "offset_ev": 20000}`
for a source axis relative to 20,000 eV. FDMNES automatically uses its declared
`E_edge`, following the
[Larch reader convention](https://github.com/xraypy/xraylarch/blob/e3c93284fed358c2c8979cba4c139430527433c6/larch/io/columnfile.py#L624-L646):
-25 eV becomes 19,975 eV in the retained Mo2C example. Stored calculation
amplitudes and convolved results are copied; no extra `Shift` or convolution
runs during import. This convention does not establish experimental calibration.

Bragg conversion uses
`E = hc / (2 d sin(theta))`, with `hc = 12398.419843320026 eV Å`, positive
crystal-plane spacing `d` in Å and Bragg angle `theta` from 0 to 90 degrees,
excluding zero. The mapping's positive `degrees_per_unit` multiplier converts
the selected axis to degrees. Use 1 for degrees, 180/π for radians or the
reciprocal of motor steps per degree. No angle offset is inferred. The relation
follows the [IUCr Bragg-law definition](https://dictionary.iucr.org/Bragg%27s_law)
for first-order reflection. 9809 uses its observed angle and header spacing;
its detector Mode fields produce separate signal choices.

An undeclared named energy axis may be interpreted as eV with a warning.
Unlabeled axes require explicit units. Read warnings and override assumptions
when the experiment differs. The arrays retain acquisition order and duplicates.
Python `spectrum()` and Rust `to_spectrum()` sort paired arrays, retaining
duplicates; the TypeScript `Spectrum` constructor and browser processing require
increasing, unique energy. Review any cleanup policy before processing.

SSRL ASCII prefers achieved energy when available. HXMA process-variable labels,
SSRL MicroEXAFS count-rate channels and X23A2 fast/slow counts remain visible.
Individual fluorescence channels and independent sample transmission pairs are
separate choices. A multielement XDI file does not imply summing every detector
or applying saved dead-time factors again. A chi(k) reference remains a processed
table rather than an automatically mapped absorption spectrum.

## Containers and archived results

Historical Athena projects retain nested metadata and journals as archival
values, never executable Perl. Optional `(undef)` arrays are absent. Saved
analysis settings do not activate rexafs processing stages. ZIP extraction and
uncalibrated pixel-to-energy conversion remain unsupported.

HDF5 returns numeric dataset paths, shapes, attributes and values. To combine
vectors across groups, call `select_datasets(["/energy", "/detector"])` in a
binding and use its returned scan index. Select at least two distinct nonempty
vectors of equal length. Rust returns a new scan from `dataset_scan(&paths)`.
Images need an explicit detector reduction. External links, some soft-link
aliases and nonnumeric dataset types are unsupported. BLISS alias-group
warnings indicate partial recovery; canonical instrument arrays remain
available. Unsupported numeric filters return errors. Integers outside the
exact f64 range (beyond 2^53) can lose precision.

XTUNES imports each XTS/XTSP record's stored E/Mu absorption and archives counted
saved tables independently, including χ(k), weighted data and Fourier results.
These remain imported results rather than active rexafs processing caches.
`ordered_parameters` metadata preserves repeated settings as ordered JSON
triples. Original paths are provenance strings and need not exist. The current
qualification covers four application-generated XTS saves, two projects and
synthetic XTSD. It does not establish XTUNES fitting or Fourier reproduction.

Input and gzip expansion are each limited to 256 MiB; decoded HDF5 arrays have
a separate 256 MiB budget. Copies can require additional memory. Gzip requires
one complete member, a valid checksum and no trailing data. The browser
workspace uses an 8 MiB file limit and a 100,000-point processing limit.

## Interfaces

In the desktop, use **Import…** or drop a single measurement file. The same
plotted preview handles beamline text, Athena, XTUNES and Larix files. Choose
**Scan** when several records are available. Detected signals appear as visible
checkboxes, initially checked when conversion succeeds. QAS inputs with the
corresponding channels offer **Transmission**, **Fluorescence**, and **Reference**.
Reference uses the transmitted monitor and downstream reference detector,
`ln(It / Ir)`, with the same transmission requirements above. Each row shows its
source-column formula. Uncheck signals you do not need; **Preview** changes the
displayed curve without changing the selected outputs. Each signal keeps its
edited mapping when switching previews. **Reset mapping** restores the previewed
signal's detected roles. **Custom mapping…** provides one manual output.

**Energy axis** shows the detected conversion and offers explicit
eV, keV, degree and radian overrides. Angles require the crystal-plane spacing
in Å. **Columns** exposes detector roles, and **Source details** shows the
original header and source rows. **Import N spectra** creates every checked
output with distinct signal names and portable original bytes in the saved
project. All selected conversions must succeed before any groups are added;
invalid signals can be excluded. The Data view opens on raw μ(E) for the
first imported spectrum, and one undo removes the entire import.
The desktop accepts files up to 256 MiB. **Scan → Select datasets…** supports
explicit selection of equal-length real HDF5 vectors; incomplete HDF5 group
recovery is flagged visibly. A readable container does not guarantee that every
detector group was recovered.

[![Pre-release import preview showing transmission, fluorescence and reference inclusion checkboxes, with the reference spectrum plotted.](/screenshots/next/import-signals.jpg)](/screenshots/next/import-signals.jpg)

Pre-release macOS ARM64 build, captured on 14 September 2026. The QAS
example has three selected outputs; **Previewing Reference** shows `ln(it / ir)`
without changing which spectra will be imported. The 651-point Mo-foil file is
retained from [xasref](https://github.com/Ameyanagi/xasref/blob/74d1e795855055c7731da406b276bd50b27aafff/foil_QAS_sample_position/Mo%20foil%200001-r0003.dat)
with its [MIT repository notice](https://github.com/Ameyanagi/xasref/blob/74d1e795855055c7731da406b276bd50b27aafff/LICENSE)
and credit to Ryuichi Shimogawa and contributors. See
[screenshot provenance](/licenses/#documentation-screenshots).

[![Earlier unreleased desktop import preview with the 624-point PbTe EX3 spectrum and detected energy and stored signal.](/screenshots/next/import-preview.jpg)](/screenshots/next/import-preview.jpg)

Earlier unreleased macOS ARM64 source build, captured on 14 September 2026 before
the visible signal checkboxes were added. The preview
shows the stored absorption before import. Data: Masashi Ishii and the Industrial
Application and Partnership Division, [XAFS spectrum of Lead telluride](https://doi.org/10.48505/nims.3178),
under [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/). See
[screenshot provenance](/licenses/#documentation-screenshots).

[![The same import preview with Source details expanded to show the original EX3 header.](/screenshots/next/import-source-details.jpg)](/screenshots/next/import-source-details.jpg)

**Source details** expands below the plot; the header remains available without
crowding the initial selection screen. Both images are full, unedited window captures.

Folders and multiple-file imports retain their batch recipe workflow; import
containers individually. The browser's **Custom columns and units** controls
also support explicit relative-energy origins. Recognized relative-energy and
motor-step calibrations are retained automatically in the desktop preview.
KEK-PF `.qd` measurements using the 9809 layout are read by content: tested
transmission examples from BL12C, BL9C and NW10A retain their recorded angles,
crystal spacing and detector counts. When a fluorescence acquisition has
transmission-only detector Mode codes, select the signal arithmetic explicitly;
the reader reports that conflict. `.qc` measurement-condition files are not
imported as spectra. Other `.qd` dialects are not implied by the extension.

The browser workspace offers scan/signal choices, original-header previews,
warnings and HDF5 vector selection. Processing runs locally in its worker.
Exported provenance identifies the source bytes, scan, mapping and dataset
paths. No source data is uploaded.

## Larix sessions

Larix 1.0 `.larix` files are accepted as gzip-compressed or plain text. Saved
Groups keep their source order and display labels. Compatible one-dimensional
`energy` and `mu` arrays become a stored absorption choice; eV and keV are
supported, with an eV assumption warning when units are absent. The reader
copies stored μ and does not calculate another detector ratio or rerun saved
normalization, background subtraction or Fourier transforms.

Saved arrays keep independent shapes and grids. A complex dataset has its real
components in `values` and matching imaginary components in `imaginary`;
`imaginary` is null/None for real arrays. Dataset selection rejects complex
values, and both interfaces label them. Chi-only and non-XAS groups have no
automatic absorption mapping. Raw PF9A sessions preserve all 1,426 points,
including six repeated energies; browser processing requires an explicit cleanup
policy before such data can be processed.

The serialization follows Larch's
[session writer](https://github.com/xraypy/xraylarch/blob/2026.3.1/larch/io/save_restore.py)
and [numeric encoder](https://github.com/xraypy/xraylarch/blob/2026.3.1/larch/utils/npjson.py).
Modern base64 and legacy numeric-list arrays are supported. Numeric views use
f64; exact bytes are retained in dataset attribute `larix.bytes_base64` with
NumPy dtype `larix.dtype`, including integer values rounded by the f64 view.
Unsupported dtypes and inconsistent array sizes return contextual errors.

Container `metadata` includes `larix.session_text`, `larix.command_history` and
`larix.symbol_order`. Commands, repeated Journals, parameter expressions and
unknown objects remain inert provenance. No saved Python code runs. Desktop
projects retain the original bytes and saved arrays alongside the imported
spectrum. Calling `spectrum()` creates an unprocessed object with fresh settings.

Qualification covers 15 API-generated sessions reopened in Larch, 384 array
hashes and six invalid cases. It does not establish manual Larix GUI save
coverage, full fitting-session restoration or agreement of processing algorithms.
Larix decoded numeric bytes/views have a 256 MiB budget; object nesting and
symbol counts are bounded. Copies and textual metadata use additional memory.

Larix session text must be UTF-8. Duplicate JSON object keys are errors; repeated
Journal entries remain ordered list entries. Import tests check truncated gzip
streams and recovery after failed loads. An ordinary text file ending at a
complete row may have no declared acquisition length to verify.
