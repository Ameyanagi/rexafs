# Importing Larix sessions

**Available since 0.2.6.** The shared Rust measurement reader
accepts Larix 1.0 `.larix` sessions in gzip or plain UTF-8 form. Python,
TypeScript/WebAssembly, the desktop reader and browser workspace use the same
parser. It imports saved data and provenance; it does not restore an executable
Larch environment.

## Select stored absorption

```python
from rexafs.io import read_measurement

measurement = read_measurement("two-analyzed.larix")
for index, scan in enumerate(measurement.document["scans"]):
    print(index, scan["label"], scan["signals"], scan["warnings"])
energy_ev, mu = measurement.arrays(scan=0)
```

Larix stores each Group as a named symbol. rexafs retains top-level Groups in
serialized order and uses the optional `_xasgroups` display-name index without
merging records. Its absence is allowed; a reference to a missing Group is an
error. This follows the container structure of Larch 2026.3.1's
[session writer](https://github.com/xraypy/xraylarch/blob/2026.3.1/larch/io/save_restore.py).
The writer can omit its automatically added index from the declared count;
rexafs accepts that specific difference and reports it.

Compatible one-dimensional `energy` and `mu` arrays produce a direct stored
absorption choice unless `datatype` identifies another quantity. Declared eV
and keV units are supported; undeclared energy uses Larch's XAS eV convention
with a warning. Other units require explicit mapping. Conversion copies the
stored signal; it does not calculate another detector ratio. Its original scale
is retained. No edge alignment or angular calibration is inferred.

The reader preserves acquisition order and duplicate energy positions. The raw
PF9A fixture retains 1,426 points, including six repeats; the processed fixture
already contains 1,420 points. Import does not remove further rows. Rust
`to_spectrum()` and Python `spectrum()` sort paired arrays and retain duplicates;
the TypeScript processing constructor and browser require strictly increasing,
unique energy. Chi-only, position-scan and empty sessions remain readable but
have no automatic absorption mapping. A malformed energy/mu shape or unequal
length returns a contextual error.

## Saved arrays and exact numeric values

Every supported numeric array is exposed in `document.datasets`, including
arrays nested inside saved objects. Paths use `/symbol/attribute`; `~` and `/`
inside a name become `~0` and `~1`, respectively. Arrays keep their original
shape and row-major order. Energy-grid results (`norm`, `flat`, `bkg`), k-grid
results (`k`, `chi`) and radial-grid results (`r`, `chir`) remain independent.
For the supplied analyzed examples, the latter grids have 241 and 326 points.

For a complex array, `values` contains its real components and `imaginary`
contains the matching imaginary components. `imaginary` is null/None for real
arrays. A saved Fourier result is not an active rexafs processing cache; its
normalization, weighting, window and units depend on the saved calculation.
Importing it does not establish agreement between Larch and rexafs transforms.
Generic dataset selection accepts real nonempty vectors of equal length and
rejects complex data, so it cannot silently discard imaginary components.

The decoder independently implements the numeric layouts in Larch's
[jsonutils](https://github.com/xraypy/xraylarch/blob/2026.3.1/larch/utils/jsonutils.py)
and [npjson](https://github.com/xraypy/xraylarch/blob/2026.3.1/larch/utils/npjson.py).
Both modern `b64ndarray` and legacy `Array` encodings are supported. Modern
arrays require explicit byte order. Supported element types are Boolean,
signed/unsigned 8/16/32/64-bit integers, float32/float64 and
complex64/complex128. Shapes, widths and decoded byte counts are validated;
unsupported or ambiguous array dtypes return errors.

Numeric views use f64, which cannot represent every integer above 2^53 exactly.
Rounding is reported. Exact bytes remain in each dataset's
`attributes["larix.bytes_base64"]`, paired with its NumPy dtype descriptor in
`attributes["larix.dtype"]`. For legacy numeric lists, rexafs constructs bytes
in the declared dtype; the original serialized lists remain in session text.
For example, this recovers exact int64 counts without passing through a float:

```python
import base64
import numpy as np

measurement = read_measurement("typed-metadata.larix")
dataset = next(d for d in measurement.document["datasets"]
               if d["path"].endswith("/metadata/counts"))
exact = np.frombuffer(
    base64.b64decode(dataset["attributes"]["larix.bytes_base64"]),
    dtype=dataset["attributes"]["larix.dtype"],
).reshape(dataset["shape"])
```

Use `typed-metadata.larix` for that example; its second count is
9,007,199,254,740,993. Byte buffers, dtype and shape are also available in the
TypeScript snapshot. Nonfinite array cells appear as null/None in snapshots;
their original bits remain archived and selecting them for absorption fails.

## Session metadata and interface behavior

`document.metadata["larix.session_text"]` retains the complete normalized
decompressed session, including configuration, unknown objects, Journals and
parameter expressions. `larix.command_history` and `larix.symbol_order` contain
ordered JSON lists as strings. Repeated Journal keys remain ordered entries.
Group headers retain their serialized record. Python-style `NaN` and infinite
metadata tokens are tolerated during inspection; their original spelling and
distinction from quoted strings remain in the retained session text.

No commands, Python constructors, parameter expressions or embedded source paths
are executed. Saved parameters do not activate rexafs processing settings.
Calling `spectrum()` creates a fresh unprocessed spectrum with no imported
normalization, background or Fourier cache.

In the desktop, choose **Import…**, or drop a single `.larix` file.
Select a group, review warnings, then choose **Add selected spectrum**. Repeat
for other groups. Desktop `.rxs` projects retain the original file bytes,
session metadata, selected mapping and saved arrays, including complex results,
so reopening does not need the original Larix path. Folder/batch recipe import
remains a separate workflow.
Custom columns and source details are collapsed when the stored absorption is
unambiguous; open them only to override the mapping or inspect archived results.

The browser accepts `.larix` files through its file input. Select a saved group
and process its stored absorption locally. Dataset review labels complex arrays
and prevents selecting them as real channels. Previews truncate text and numeric
samples; the worker reparses the original input for conversion. Exported
processing provenance identifies the source byte hash, format, scan and mapping.

Input and gzip expansion are limited to 256 MiB each. Larix additionally bounds
decoded numeric bytes and f64 views to 256 MiB, symbol count to 10,000, object
nesting to 64 levels and inspected nodes to one million. These are not total
process-memory limits: snapshots, metadata, base64 text and conversions own
additional copies. Existing desktop and browser input limits are 32 and 8 MiB.
Session text must be valid UTF-8. Duplicate JSON object keys are rejected at
every nesting level, including embedded array descriptions. Repeated Journal
entries remain valid ordered list entries. Gzip must contain one complete
member with a valid checksum and no trailing data or additional members.

## Qualification and limits

[`measurement_fixtures/larix.rs`](../crates/rexafs/tests/measurement_fixtures/larix.rs) reads all 15 valid
sessions and compares all 384 numeric arrays with independent pre-serialization
expectations: dtype, shape, full-data SHA-256 and sampled values. It also checks
floating/Boolean views against full hashes, complex components, exact integer
bytes, metadata, group order, resource limits and all six invalid inputs.
These are serialization checks, not experimental or processing-algorithm
validation. Binding and GUI tests exercise real processed and multigroup files.

The supplied [fixture bundle](../crates/rexafs/tests/fixtures/sessions/larix/README.md)
predates this reader; its unchanged notes describe the original handoff.
Its files were created with the Larch Python API and reopened in Larch, rather
than saved by manually operating Larix's GUI. Original files, expectations,
license notices and source hashes remain unchanged and excluded from the crate
archive. Full FEFF fitting sessions, PCA/linear-combination results, external
handles, object/string array dtypes, every historical version and older
`save_groups()` containers remain unqualified.
