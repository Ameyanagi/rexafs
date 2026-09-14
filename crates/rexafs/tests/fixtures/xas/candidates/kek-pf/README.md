# KEK Photon Factory QD fixtures

These five original measurements are retained for **academic, nonmilitary
research and reader regression testing only**. The [KEK database usage
notice](https://pfxafs.kek.jp/xafsdata/) allows academic research excluding
military-related research and asks users to contact the experimenter listed on
the source record before citing the data in a publication.

The [original Japanese notice and English explanation](../../LICENSES/LicenseRef-KEK-PF-Academic-Use-Notice.txt)
are retained with [the source pages](../../provenance/kek-pf-qd/).
`LicenseRef-KEK-PF-Academic-Use-Notice` is a local identifier for those custom
terms, not a standard open-data license. The rexafs software license does not
replace the data's terms. Each file has an adjacent `.license` attribution
record; preserve it with the unchanged measurement bytes.

These are repository test fixtures, excluded from crates.io, Python and npm
packages. They require no private gathering repository or network access during
testing. Original filenames, byte sizes and SHA-256 checksums are recorded in
[the manifest](../../manifest.json).

| Original file | Beamline | Experimenter and measurement date | Official record |
| --- | --- | --- | --- |
| [cu_foil_0.qd](bl-9c/cu_foil_0.qd) | BL-9C | Hiroaki Nitani (Photon Factory), 2018-05-30 | [Record 22](https://pfxafs.kek.jp/xafsdata/view.php?id=22) |
| [fe002_0.qd](bl-12c/fe002_0.qd) | BL-12C | Yasuhiro Inada (Ritsumeikan University), 2021-11-12 | [Record 132](https://pfxafs.kek.jp/xafsdata/view.php?id=132) |
| [ScotchTape02_0.qd](bl-9a/ScotchTape02_0.qd) | BL-9A | Yasuo Takeichi (Photon Factory), 2021-11-19 | [Record 133](https://pfxafs.kek.jp/xafsdata/view.php?id=133) |
| [sr01_0.qd](ar-nw10a/sr01_0.qd) | AR-NW10A | Yasuhiro Niwa (KEK-PF), 2022-05-25 | [Record 153](https://pfxafs.kek.jp/xafsdata/view.php?id=153) |
| [Fe003_0.qd](bl-9c/Fe003_0.qd) | BL-9C | Hitoshi Abe (IMSS), 2025-03-02 | [Record 187](https://pfxafs.kek.jp/xafsdata/view.php?id=187) |

All five files use the text-based 9809 layout. The four transmission examples
use the observed monochromator angle and recorded crystal-plane spacing to
calculate energy, then ln(I0/It) for absorption, where I0 and It are the incident
and transmitted detector signals. The BL9A file declares
fluorescence in its acquisition summary but transmission in its detector Mode
fields; it requires explicit signal arithmetic. Its raw columns and both
conflicting declarations remain available.

The [QD regressions](../../../../measurement_fixtures/kek.rs) compare every original
column and every converted energy and signal value. See the
[investigation record](../../../../../../../doc/validation/2026-09-14-kek-qd/review.md)
for scientific assumptions and binding/GUI checks. From the repository root:

```sh
cargo test --locked -p rexafs --test measurement_fixtures kek_
python3 scripts/check-beamline-fixtures.py
python3 scripts/check-beamline-fixtures.py --package
```

To restore a missing original, open its official record through the database
entry page and use the Download form in the same browser session. Then verify
the recorded checksum. A stateless request to `download.php` may return HTML
instead of the measurement; the generic fetch script reports this requirement.
