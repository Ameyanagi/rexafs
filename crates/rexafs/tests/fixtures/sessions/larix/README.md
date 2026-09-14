# Larix session fixtures

This bundle supplies test files for adding `.larix` import support to rexafs.
The 15 valid sessions were generated with `larch.io.save_session()` from
xraylarch 2026.3.1 and reopened with `larch.io.read_session()`. The plain-text
variant is a decompressed copy. Six deliberately damaged or inconsistent files
exercise rejection and diagnostic behavior. No rexafs reader is implemented here.

These are Python API saves, not manual Larix GUI saves. They use the
[session writer used by Larix](https://github.com/xraypy/xraylarch/blob/2026.3.1/larch/io/save_restore.py).
The [format notes](FORMAT.md) describe the serialization and import boundaries.

## Files and coverage

All paths below are relative to `fixtures/valid/`.

| File | Intended coverage |
| --- | --- |
| `pfbl12c-raw.larix` | Transmission, 818 original absorption and detector rows. |
| `pf9a-raw.larix` | Fluorescence, all 1,426 original rows; six repeated energy positions. |
| `pfbl12c-normalized.larix` | Absorption plus pre-edge, normalization and flattened arrays. |
| `pfbl12c-autobk.larix` | Background subtraction, independent k grid and chi(k). |
| `pfbl12c-ft.larix` | Fourier transform, complex chi(R), real/imaginary/magnitude arrays. |
| `pf9a-ft.larix` | Processed fluorescence, 1,420 retained absorption rows. |
| `two-analyzed.larix` | Two processed spectra with independent energy grids. |
| `mixed-raw-analyzed.larix` | Raw transmission together with processed fluorescence. |
| `legacy-arrays.larix` | Numeric-list `Array` encoding, including complex arrays. |
| `no-xasgroups.larix` | Session without the optional display-name index. |
| `typed-metadata.larix` | Synthetic spectrum, Unicode, repeated Journal keys, parameters, shaped arrays, byte order and integers above 2^53. |
| `chi-only.larix` | chi(k) without energy/mu; no automatic absorption conversion. |
| `non-xas.larix` | Synthetic position scan in millimeters; no energy interpretation. |
| `empty.larix` | Empty session with no spectra. |
| `plain-session.larix` | Exact decompressed `typed-metadata.larix`, accepted by Larch. |

`fixtures/invalid/` contains `truncated-gzip`, `bad-magic`, `invalid-json`,
`bad-array-shape`, `mismatched-energy-mu` and `missing-group-reference`, each
with a `.larix` extension. The last two have valid serialization but inconsistent
content: report a mapping error or dangling reference. The manifest distinguishes
these cases from damaged containers. These are proposed expectations for rexafs;
Larch itself can print decoding errors or remove unresolved references.

## Verification and regeneration

Use the repository's virtual environment, or create one using uv:

```sh
uv venv --python 3.12 .venv
uv pip install --python .venv/bin/python -r requirements-lock.txt
python3 verify.py
.venv/bin/python verify.py --larch
```

The first verification command needs only the Python standard library. It
checks all file hashes, the fixture inventory, original source checksums,
session framing, and array payload sizes. Its independent layout inspector
also checks that each invalid fixture fails at its documented layer.
`--larch` additionally reopens
every valid session, fails on decoding diagnostics, and compares all recorded
arrays and metadata to `expected.json`. No verification command downloads data.

```sh
.venv/bin/python generate.py
.venv/bin/python verify.py --larch
```

Regeneration overwrites generated files and reference metadata. Expectations
come from the in-memory groups **before serialization**, using a separate
snapshot function. Every array has its dtype, shape, full-data SHA-256 and
first/middle/last samples. Complex samples are `[real, imaginary]` pairs;
sample indices refer to the flattened C-order array. The hash normalizes the
original dtype to little endian and retains its width. Integers above 2^53
must be read with a JSON implementation that retains integer precision.

Machine name, MAC ID, save and Journal timestamps, session configuration, and
gzip timestamps are controlled fixture values. The generator starts Python
with `PYTHONHASHSEED=0` to stabilize lmfit's saved symbol order.
Actual software and platform
versions are recorded. Numerical values may change with dependency versions;
use the lock file and review changes rather than updating references merely
to satisfy a reader. This verifies serialization, not scientific accuracy or
equivalence of rexafs processing algorithms.

## Measurement provenance and transformations

The unchanged `PFBL12C_2005.dat` and `PF9A_2022.dat` inputs are in `sources/`.
Their [source manifest](sources/manifest.json) preserves exact upstream commit
URLs, SHA-256 values, beamline attribution and citations. They were copied from
the attributed rexafs beamline corpus. The upstream filename `PFBL12C_2005.dat`
does not establish its acquisition year; its original header says `07.05.12`.

Energy is calculated from the observed `Angle(o)` column using
`E = hc / (2 d sin(theta))`: E is photon energy in electronvolts, d is the
header's crystal spacing of 3.13551 angstroms, and theta is the observed Bragg
angle converted from degrees to radians. The value of hc in electronvolt
angstroms comes from Larch's `PLANCK_HC` constant and is recorded per group.
This is the conversion used in
[Larix's column importer](https://github.com/xraypy/xraylarch/blob/2026.3.1/larch/wxlib/columnframe.py).

Transmission uses `mu = ln(i0 / signal)`; fluorescence uses `mu = signal / i0`,
with detector roles from each file's Mode header. These stored signals are
dimensionless and do not infer an absolute absorption coefficient. No second
offset correction is applied. Raw fixtures retain every row. Processed
checkpoints retain the first row at each observed energy and record the
zero-based `retained_source_rows` array. This removes six repeated positions
from PF9A and none from PFBL12C. It is a fixture-generation choice, not an
instruction for the rexafs importer to discard data.

The exact `pre_edge`, `autobk` and `xftf` arguments are in `manifest.json`
and `generate.py`. Pre-edge and normalization ranges are offsets in eV from
the edge; `nnorm=2` selects a quadratic normalization polynomial. The background
cutoff `rbkg=1` is in angstroms; k ranges and `kstep=0.05` are in inverse
angstroms. Fourier settings use k=2 to 10 inverse angstroms, k weight 2,
a Hanning window with `dk=1`, and 2,048 transform points. They exercise saved
output arrays and settings and are not optimized fits to these samples.

## Attribution and rexafs handoff

Retain [LICENSE.txt](LICENSE.txt) with copied or derived data. It is the
unchanged MIT notice from the upstream xraylarch repository, including its
original spelling. Source sidecars carry per-file attribution. The generation
scripts and synthetic fixtures use the same MIT terms. Larch is described by
Matthew Newville and contributors in
[Larch: An Analysis Package for XAFS and Related Spectroscopies](https://doi.org/10.1088/1742-6596/430/1/012007).

The complete bundle is intended for
`crates/rexafs/tests/testfiles/larix/` in `~/dev/rexafs` and is excluded from
the published crate. Future integration tests can load the binary fixtures
and `expected.json` from disk without installing Larch. Larch is required only
to regenerate or independently reopen the fixtures. Keep format tests under
`tests/beamline_larix.rs` to follow rexafs's existing packaging convention.
