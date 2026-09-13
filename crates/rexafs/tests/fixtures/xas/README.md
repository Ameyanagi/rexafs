# rexafs measurement-format corpus

132 original measurement/project files collected on 2026-09-14, totaling 26.8 MB (25.5 MiB), from eight repositories/databases. The collection targets format coverage with a few examples per beamline; projects may contain multiple spectra. The source-checkout integration tests exercise a subset of the collected formats; see [INTEGRATION.md](INTEGRATION.md).

- **120 files in `samples/`**, with documented licenses, covering **49 identified beamlines/stations at 14 facilities**. Additional files retain unresolved station labels. Of these files, 75 have explicit data licenses and 45 rely on upstream repository distribution licenses.
- **12 examples obtained from RefXAS**, stored in `candidates/refxas/` and representing eight additional identified beamlines. Source links, citations and the original usage notice are retained.
- Every measurement has an adjacent `.license` attribution file and a manifest entry with source/download URLs, license evidence, citation, SHA-256, byte size, mode and provenance. Original measurement bytes and filenames are retained.

Start with [manifest.csv](manifest.csv) for browsing, [manifest.json](manifest.json) for scripts, [CITATIONS.md](CITATIONS.md) for attribution, and [LICENSE.md](LICENSE.md) for license scope. [validation.json](validation.json) records the integrity/container checks.

## Contents

The files include transmission, fluorescence, multielement fluorescence, TEY, PEY, PFY, CEY, HERFD detector projections and an energy-dispersive Athena example. Mode tags describe source metadata or available detector channels, not a guarantee that every channel is active or scientifically useful.

Formats include PF/SPring-8 9809, MRCAT and EPICS/LabVIEW ASCII, SPEC/SRS (including wrapped rows and multiple scans), FIO, XDAC, SSRL ASCII and binary, NSLS legacy binary, XDI, interpolated QAS/ISS data, Athena Perl/JSON projects, EX3 and yield exports, Sardana HDF5 and RefXAS BLISS HDF5. There are 61 acquisition-format files across both subsets, six additional exports retaining raw detector channels, and two HERFD detector projection files. “Raw” does not imply that the beamline applied no corrections.

The supplied [MDR ZrN record](https://mdr.nims.go.jp/datasets/3ff63598-8621-4ef6-8de9-5426bf12adac) is included as Photon Factory BL-12C `zr02.dat`, with its metadata and CC-BY-NC-SA-4.0 declaration.

## Verify or restore

```sh
# Verify the documented-license subset, sidecars, licenses and metadata evidence.
python3 scripts/corpus.py verify

# Optional: verify all local files and read every HDF5 dataset.
uv run --with h5py python scripts/corpus.py verify --include-candidates --hdf5 --report validation.json

# Print paths eligible for the requested noncommercial test set.
python3 scripts/corpus.py list

# Restore missing measurement files from recorded URLs; SHA-256 must match.
python3 scripts/corpus.py fetch
```

All 132 measurement files, including the RefXAS examples, are retained in this Git checkout. Cargo excludes this entire directory from the published crate. Tests use local files and do not download data.

`fetch` restores measurement payloads and their sidecars, leaving existing changed files untouched. License texts and evidence snapshots are supplied in this repository; the fetch command does not recreate those extracted metadata records. Use `--include-candidates` only when deliberately working with the RefXAS local candidate set. GitHub URLs are pinned to commits; other sources are pinned by checksums.

Verification checks byte sizes, SHA-256, Git blob hashes where available, required attribution fields, companions and license/metadata assets. It also checks text/JSON/gzip readability; optional HDF5 verification reads dataset payloads. It does not validate the scientific quality of spectra, implement legacy binary readers, or prove rexafs parser support. Numeric text row counts in the report can include headers and wrapped rows.

## Identified beamlines in the documented-license subset

BM08 GILDA and LISA are counted once. The ESRF dispersive project contains both ID24 data and a BM29 reference, so its one file appears in both rows. SRS station identifiers are retained as written in the source.

| Facility | Beamline/station | Files | Modes / channels |
| --- | --- | ---: | --- |
| ALS | [10.3.2](samples/als/10-3-2) | 1 | unresolved |
| APS | [10-BM](samples/aps/10-bm) | 1 | fluorescence, transmission |
| APS | [12-BM](samples/aps/12-bm) | 1 | fluorescence, transmission |
| APS | [13-BM-D](samples/aps/13-bm-d) | 2 | fluorescence, transmission |
| APS | [13-ID-C](samples/aps/13-id-c) | 2 | transmission |
| APS | [13-ID-E](samples/aps/13-id-e) | 2 | fluorescence, transmission |
| APS | [20-BM](samples/aps/20-bm) | 2 | fluorescence, transmission |
| APS | [20-ID](samples/aps/20-id) | 2 | fluorescence, transmission |
| APS | [9-BM](samples/aps/9-bm) | 2 | fluorescence, transmission |
| Aichi SR | [BL11S2](samples/aichi-sr/bl11s2) | 2 | transmission |
| Aichi SR | [BL5S1](samples/aichi-sr/bl5s1) | 2 | transmission |
| CLS | [BIOXAS-S](samples/cls/bioxas-s) | 3 | transmission |
| CLS | [HXMA](samples/cls/hxma) | 4 | transmission |
| CLS | [IDEAS](samples/cls/ideas) | 3 | transmission |
| CLS | [SXRMB](samples/cls/sxrmb) | 3 | TEY |
| CLS | [VLS-PGM](samples/cls/vls-pgm) | 3 | fluorescence |
| ESRF | [BM08 LISA](samples/esrf/bm08-lisa) | 7 | transmission |
| ESRF | [BM26A DUBBLE](samples/esrf/bm26a-dubble) | 1 | fluorescence |
| ESRF | [BM29](samples/esrf/id24) | 1 | energy-dispersive |
| ESRF | [ID21](samples/esrf/id21) | 3 | fluorescence, transmission |
| ESRF | [ID24](samples/esrf/id24) | 1 | energy-dispersive |
| ESRF | [SNBL BM01B](samples/esrf/snbl-bm01b) | 1 | fluorescence, transmission |
| NSLS | [X10C](samples/nsls/x10c) | 1 | transmission |
| NSLS | [X11A](samples/nsls/x11a) | 1 | transmission |
| NSLS | [X15B](samples/nsls/x15b) | 1 | unresolved |
| NSLS | [X23A2](samples/nsls/x23a2) | 1 | fluorescence, transmission |
| NSLS-II | [6-BM BMM](samples/nsls-ii/6-bm-bmm) | 1 | fluorescence, transmission |
| NSLS-II | [7-BM QAS](samples/nsls-ii/7-bm-qas) | 3 | fluorescence, transmission |
| NSLS-II | [8-ID ISS](samples/nsls-ii/8-id-iss) | 1 | fluorescence, transmission |
| Photon Factory | [AR-NW10A](samples/photon-factory/ar-nw10a) | 2 | transmission |
| Photon Factory | [BL-12C](samples/photon-factory/bl-12c) | 2 | transmission |
| Photon Factory | [BL-9A](samples/photon-factory/bl-9a) | 3 | fluorescence, transmission |
| Photon Factory | [BL-9C](samples/photon-factory/bl-9c) | 2 | transmission |
| Ritsumeikan SR | [BL-10](samples/ritsumeikan-sr/bl-10) | 4 | PFY, TEY |
| Ritsumeikan SR | [BL-11](samples/ritsumeikan-sr/bl-11) | 4 | PEY, TEY |
| Ritsumeikan SR | [BL-13](samples/ritsumeikan-sr/bl-13) | 4 | TEY |
| Ritsumeikan SR | [BL-2](samples/ritsumeikan-sr/bl-2) | 4 | TEY |
| SAGA-LS | [BL10](samples/saga-ls/bl10) | 2 | TEY |
| SAGA-LS | [BL11](samples/saga-ls/bl11) | 4 | CEY, PFY |
| SAGA-LS | [BL12](samples/saga-ls/bl12) | 2 | TEY |
| SLS | [PHOENIX](samples/sls/phoenix) | 1 | TEY, fluorescence |
| SPring-8 | [BL14B2](samples/spring-8/bl14b2) | 5 | transmission |
| SRS Daresbury | [SOX1](samples/srs-daresbury/sox1) | 1 | unresolved |
| SRS Daresbury | [ex1a](samples/srs-daresbury/ex1a) | 1 | fluorescence |
| SRS Daresbury | [exf9](samples/srs-daresbury/exf9) | 1 | fluorescence |
| SSRL | [2-3](samples/ssrl/2-3) | 3 | transmission |
| SSRL | [4-1](samples/ssrl/4-1) | 1 | transmission |
| SSRL | [4-3](samples/ssrl/4-3) | 2 | transmission |
| SSRL | [7-3](samples/ssrl/7-3) | 1 | transmission |

## Examples obtained from RefXAS

| Facility | Beamline | Files |
| --- | --- | ---: |
| DESY PETRA III | P65 | 3 |
| ESRF | BM23 | 2 |
| Elettra | XAFS | 1 |
| KIT ANKA | CATACT | 1 |
| SLRI | BL8 | 1 |
| SLS | SuperXAS | 1 |
| SOLEIL | ROCK | 1 |
| SOLEIL | SAMBA | 1 |

The DELTA two-column example has no identified station. The original RefXAS notice is retained in [LICENSES](LICENSES/LicenseRef-RefXAS-Usage-Notice.txt); the [RefXAS paper](https://doi.org/10.1107/S1600577524006751) is cited for all 12 examples.

## Provenance and unresolved details

- MAX IV/Balder attribution for the ParSeq examples is tentative and excluded from the confirmed count. The HERFD master and two Eiger projection files are linked by `companion_files`; the latter hold 601×1030 and 526×1030 projections, not full 2D camera-frame stacks.
- DORIS FIO, CAMD projects, LNLS, SSRL MicroEXAFS and Lytle examples retain unresolved station labels. No modern beamline is inferred from a facility name alone.
- `APS13ID_2008.dat` actually identifies 13-BM-D in its header. `SSRL1_2006.dat` identifies SSRL 2-3. The original filenames remain unchanged.
- SAGA BL11 raw files say `Transmission(2)` in the acquisition header, while the dataset column metadata identifies ROI fluorescence and conversion electrons. Those records use PFY/CEY mode tags and preserve the conflict. Aichi `-f` filenames contain transmission data; the suffix was not used to infer fluorescence.
- CLS files are the database’s downloadable exports. Their original acquisition headers are not assumed to survive. Per-spectrum API metadata supplies the license for `Fe_idcj28n9`, whose payload header omits it.
- MDR `record.json` evidence is extracted JSON-LD from each landing page. Downloaded instrument metadata is preserved separately; licenses were checked per record, not inferred from the whole collection.

[RESEARCH_NOTES.md](RESEARCH_NOTES.md) records source selection and remaining coverage gaps. All coverage figures describe this collection, not an exhaustive census of beamline formats.

Integration correction: `as2o3_10K_scan1.xdi` identifies SSRL 2-3 in its original header; the collected catalog previously placed it under 4-1. The catalog and directory were corrected, with measurement bytes retained.
