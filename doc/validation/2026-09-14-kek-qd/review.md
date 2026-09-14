# KEK Photon Factory QD investigation

Investigated on 2026-09-14 using the unreleased shared reader. Five actual `.qd`
files were downloaded from the official Photon Factory database. All begin with
`9809`, followed by `KEK-PF` and the beamline identifier. The four transmission
examples import through the existing 9809 adapter; `.qd` is not a separate binary
format in these examples. This corrects the earlier unqualified extension list.

| Official record and original file | Beamline | Sample | Rows | Result |
| --- | --- | --- | ---: | --- |
| [132: `fe002_0.qd`](https://pfxafs.kek.jp/xafsdata/view.php?id=132) | BL12C | LiFePO4; Yasuhiro Inada | 3,917 | Transmission; automatic Bragg conversion and ln(I0/It) verified. |
| [22: `cu_foil_0.qd`](https://pfxafs.kek.jp/xafsdata/view.php?id=22) | BL9C | Cu foil; Hiroaki Nitani | 5,135 | Transmission; automatic conversion verified. |
| [187: `Fe003_0.qd`](https://pfxafs.kek.jp/xafsdata/view.php?id=187) | BL9C | Fe foil; Hitoshi Abe | 4,000 | Transmission; automatic conversion verified. |
| [153: `sr01_0.qd`](https://pfxafs.kek.jp/xafsdata/view.php?id=153) | NW10A | SrCO3; Yasuhiro Niwa | 3,999 | Transmission; conversion uses the recorded 1.63747 Å spacing for Si(311). |
| [133: `ScotchTape02_0.qd`](https://pfxafs.kek.jp/xafsdata/view.php?id=133) | BL9A | Scotch tape; Yasuo Takeichi | 452 | Both the database and acquisition summary say fluorescence, while detector Mode codes say transmission. Columns are retained; automatic arithmetic is suppressed. |

## Access and provenance

Open the [database entry page](https://pfxafs.kek.jp/xafsdata/), enter the database,
open the record and use its Download button. Programmatic access needs the same
cookie session and the current hidden form fields. A stateless request to
`download.php` can return HTML instead of data. Validate the attachment name,
content and checksum after downloading; there is no stable direct `.qd` URL on
the record page.

The database permits academic, nonmilitary research use and asks users to contact
the experimenter for publication citations. The initial investigation retained
original files, landing pages, terms and checksums locally under
`target/format-investigation/kek-qd/`. The [verification record](verification.json)
records that initial inspection and its numerical results.

Retention update, 2026-09-14: at the user’s request, all five originals are now
[repository test fixtures](../../../crates/rexafs/tests/fixtures/xas/candidates/kek-pf/README.md)
for academic, nonmilitary research and reader regression testing only. Original
Japanese terms, experimenter attribution, source pages and unchanged data bytes
are retained together. The custom usage notice is recorded without assigning a
standard open-data license or asserting broader permissions. These files are
excluded from crates.io, Python and npm packages. The historical 222-file
gathering snapshot remains unchanged.

## Numerical checks and reader changes

Independent NumPy table loading was compared with every recovered source column.
Every converted energy was checked using first-order Bragg diffraction,
E = hc / (2 d sin θ), with hc = 12398.419843320026 eV Å, the header's crystal-plane
spacing d in Å and the observed angle θ converted from degrees to radians.
Transmission was checked as ln(I0/It) for each row. These checks verify parsing
and arithmetic, not detector calibration or experimental accuracy; see the
[reader's assumptions and references](../../measurement-reader.md).

Two synthetic regression cases in
[`contracts.rs`](../../../crates/rexafs/tests/measurement_reader/contracts.rs)
exercise the observed layouts without copying third-party measurement rows:

- The scan-plan `Step/deg` header must not assign degree units to detector counts.
  Only the two angle columns and dwell-time column have prescribed deg/deg/s
  units; undeclared detector units remain absent.
- A fluorescence acquisition with only transmission candidates must retain the
  conflicting declarations, report a warning and require explicit arithmetic.
  An explicit detector/incident ratio is still available. Mixed detector files
  with a fluorescence candidate retain their normal choices.

The implementation is in
[`text.rs`](../../../crates/rexafs/src/xafs/io/reader/text.rs). The shared reader
propagates these corrections to Rust, Python, TypeScript/WebAssembly and desktop
imports; no new public API or extension filter is required.

## Validation results

Initial investigation: all 24 synthetic reader contracts and 24 fixture integration tests passed after
the corrections. Installed Python tests (23), JavaScript/package tests (24),
eight desktop import tests and strict core Clippy also passed. Each of the five
local QD files was checked through both the Python and Node/WebAssembly bindings;
the BL9A file required the recorded explicit ratio mapping.

The rebuilt release desktop was inspected through computer use: `fe002_0.qd`
opens directly into a 3,917-point plotted preview with transmission and the
recorded 3.13551 Å Bragg conversion selected. The original file remains available
locally for further investigation.

After repository retention, 26 fixture integration tests and 24 self-contained
reader contracts passed. The two new [QD fixture tests](../../../crates/rexafs/tests/measurement_fixtures/kek.rs)
check all original rows, detector units, Bragg conversion and signal arithmetic,
including the BL9A conflict. Strict fixture Clippy and integrity checks passed:
154 measurements and 99 attribution/evidence assets in `xas/`, plus the unchanged
expanded corpus and application session bundles. The [retention record](retention.json)
pins the five originals and the inspected package archives. Newly built Rust,
Python source and npm archives, and the existing release Python wheel, contain
none of these raw fixture bundles.
