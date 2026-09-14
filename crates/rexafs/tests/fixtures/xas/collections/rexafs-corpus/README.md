# rexafs measurement-format corpus

**222 original measurement/project files, 210.8 MB (201.1 MiB), from 20 repositories and dataset sources.** This private repository contains the corpus, provenance and verification tools for rexafs format work. Measurement bytes and original filenames are retained.

The worldwide research pass added **90 files** to the original 132. The corpus now represents **68 confirmed beamlines/stations at 25 facilities**, including historical acquisitions. See the [research report](research/worldwide-beamlines.md), [worldwide inventory](research/beamlines.csv) and [remaining download leads](research/download-leads.csv).

- **210 files in `samples/`** have documented licenses: 132 use explicit data licenses and 78 rely on the upstream repository distribution license. These cover 61 confirmed beamlines/stations at 22 facilities.
- **12 examples obtained from RefXAS** remain in `candidates/refxas/`, with citations and the original usage notice. Seven stations are additional to the documented-license subset; SAMBA is now represented in both.
- Every measurement has an adjacent `.license` file and a manifest record containing its citation, license evidence, source/download URLs, SHA-256, size, modes and data level.

The inventory has **245 rows across 47 facilities and 22 countries/territories**: 236 beamline/station discovery rows and nine unresolved corpus-attribution rows. It distinguishes directory-only leads from individually investigated sources and downloaded examples. It is not an exhaustive or uniformly current operating-beamline census.

Browse [manifest.csv](manifest.csv), use [manifest.json](manifest.json) for scripts, and retain [CITATIONS.md](CITATIONS.md) and [LICENSE.md](LICENSE.md). [validation.json](validation.json) records verification results.

## Contents

The files include transmission, fluorescence, multielement fluorescence, TEY, PEY, PFY, TFY, CEY, HERFD detector projections, PEEM energy images and an energy-dispersive Athena example. Mode tags describe source metadata or available detector channels; they do not guarantee that every channel is active or scientifically useful.

Formats include 9809, MRCAT, EPICS/LabVIEW ASCII, SPEC/SRS, FIO, XDAC, SSRL ASCII and binary, NSLS legacy binary, XDI, interpolated QAS/ISS data, Athena Perl/JSON projects, EX3/yield exports, EPICS MDA, BLISS and Sardana HDF5, NeXus NXxas, Elmitec PEEM images, CSV, XLSX and Origin OPJU.

There are **118 acquisition-format files**, **19 exports retaining raw detector channels**, **five detector projections**, and 80 other exports/projects. File counts are not experiment counts: 31 ALBA image files form one Fe-L3 energy scan, and one native BM16 HDF5 file holds multiple scans and complete MCA arrays. Upstream NeXus conversions and analysis projects are explicitly labeled.

The supplied [MDR ZrN record](https://mdr.nims.go.jp/datasets/3ff63598-8621-4ef6-8de9-5426bf12adac) remains included as Photon Factory BL-12C `zr02.dat` with its CC-BY-NC-SA-4.0 declaration.

## Verify or restore

```sh
# Verify every file, notice, license and provenance asset.
python3 scripts/corpus.py verify --include-candidates

# Also read all local HDF5 datasets and write the report.
uv run --with h5py python scripts/corpus.py verify --include-candidates --hdf5 --report validation.json

# List the documented-license subset.
python3 scripts/corpus.py list

# Restore missing payloads and notices from their recorded sources.
python3 scripts/corpus.py fetch
```

`fetch` leaves existing changed files untouched and checks restored sizes/checksums before writing. ZIP-member records include the archive URL, member name, size and SHA-256. Each archive is downloaded once per run and read without extracting arbitrary paths. Restoring the P64 examples requires the upstream 625.7 MB archive; that archive is not stored in Git. License texts and provenance snapshots are supplied by the checkout, rather than recreated by `fetch`.

Use `--include-candidates` to include the examples obtained from RefXAS. GitHub payload URLs are pinned to commits; other sources are pinned by checksums. Local tests can read this separate checkout through an environment variable or configured path, so the data need not be distributed through crates.io. This repository has no crate packaging or rexafs parser implementation.

Verification covers bytes, hashes, attribution fields, companion paths, provenance assets, text/JSON/gzip readability, XLSX ZIP/XML structure and all visited HDF5 datasets. Native BM16 external-link inspection found no external files needed. Legacy binary, MDA, Elmitec and OPJU semantic readers were not run. These checks do not establish scientific validity or rexafs parser support.

## Confirmed beamlines in the documented-license subset

BM08 GILDA/LISA is counted once; APS 20-ID-C is grouped under 20-ID to avoid increasing coverage for a station suffix. Project files can represent more than one beamline. Historical station names remain distinct from modern replacements.

| Facility | Beamline/station | Files | Modes / channels |
| --- | --- | ---: | --- |
| ALBA | [CIRCE](samples/alba/circe/zenodo-10044133) | 32 | PEEM, electron-yield |
| ALS | [10.3.2](samples/als/10-3-2/xraylarch) | 1 | unresolved |
| APS | [10-BM](samples/aps/10-bm/xraylarch) | 1 | fluorescence, transmission |
| APS | [12-BM](samples/aps/12-bm/xraylarch) | 1 | fluorescence, transmission |
| APS | [13-BM-D](samples/aps/13-bm-d) | 5 | fluorescence, transmission |
| APS | [13-ID-C](samples/aps/13-id-c/xasdatalibrary) | 2 | transmission |
| APS | [13-ID-E](samples/aps/13-id-e) | 15 | fluorescence, transmission |
| APS | [20-BM](samples/aps) | 4 | fluorescence, transmission |
| APS | [20-ID](samples/aps) | 5 | HERFD, fluorescence, transmission |
| APS | [9-BM](samples/aps/9-bm/xraylarch) | 2 | fluorescence, transmission |
| Aichi SR | [BL11S2](samples/aichi-sr/bl11s2) | 2 | transmission |
| Aichi SR | [BL5S1](samples/aichi-sr/bl5s1) | 2 | transmission |
| Australian Synchrotron | [MEX1](samples/australian-synchrotron/mex1/pynexafs) | 3 | fluorescence, transmission |
| Australian Synchrotron | [MEX2](samples/australian-synchrotron/mex2/pynexafs) | 6 | TEY, fluorescence |
| Australian Synchrotron | [SXR](samples/australian-synchrotron/sxr/pynexafs/2024-03) | 4 | TEY, fluorescence |
| BESSY II | [EMIL OAESE](samples/bessy-ii/emil-oaese) | 4 | PFY, TFY |
| CLS | [BIOXAS-S](samples/cls/bioxas-s/cls-xasdb) | 3 | transmission |
| CLS | [HXMA](samples/cls/hxma) | 4 | transmission |
| CLS | [IDEAS](samples/cls/ideas/cls-xasdb) | 3 | transmission |
| CLS | [SXRMB](samples/cls/sxrmb/cls-xasdb) | 3 | TEY |
| CLS | [VLS-PGM](samples/cls/vls-pgm/cls-xasdb) | 3 | fluorescence |
| DESY PETRA III | [P64](samples/desy-petra-iii/p64/darus-5088) | 5 | HERFD, fluorescence |
| Diamond | [I06](samples/diamond/i06/figshare-24530623) | 1 | PEEM, electron-yield |
| ESRF | [BM08 LISA](samples/esrf/bm08-lisa) | 7 | transmission |
| ESRF | [BM16 FAME-UHD](samples/esrf/bm16-fame-uhd/pynxxas/BlissMultiModal) | 2 | fluorescence, transmission |
| ESRF | [BM26A DUBBLE](samples/esrf/bm26a-dubble/demeter) | 1 | fluorescence |
| ESRF | [BM29](samples/esrf/id24/xraylarch) | 1 | energy-dispersive |
| ESRF | [ID21](samples/esrf/id21/cls-xasdb) | 3 | fluorescence, transmission |
| ESRF | [ID24](samples/esrf/id24/xraylarch) | 1 | energy-dispersive |
| ESRF | [SNBL BM01B](samples/esrf/snbl-bm01b/xraylarch) | 1 | fluorescence, transmission |
| NSLS | [X10C](samples/nsls/x10c/demeter) | 1 | transmission |
| NSLS | [X11A](samples/nsls/x11a/xasdatalibrary) | 1 | transmission |
| NSLS | [X15B](samples/nsls/x15b/demeter) | 1 | unresolved |
| NSLS | [X23A2](samples/nsls/x23a2/xraylarch) | 1 | fluorescence, transmission |
| NSLS-II | [6-BM BMM](samples/nsls-ii/6-bm-bmm/xraylarch) | 1 | fluorescence, transmission |
| NSLS-II | [7-BM QAS](samples/nsls-ii/7-bm-qas/xasref) | 3 | fluorescence, transmission |
| NSLS-II | [8-ID ISS](samples/nsls-ii/8-id-iss/xraylarch) | 1 | fluorescence, transmission |
| PLS-II | [2A](samples/pls-ii/2a/figshare-28944977) | 1 | TEY |
| Photon Factory | [AR-NW10A](samples/photon-factory/ar-nw10a) | 2 | transmission |
| Photon Factory | [BL-12C](samples/photon-factory/bl-12c) | 2 | transmission |
| Photon Factory | [BL-9A](samples/photon-factory/bl-9a) | 4 | fluorescence, transmission |
| Photon Factory | [BL-9C](samples/photon-factory/bl-9c) | 2 | transmission |
| Ritsumeikan SR | [BL-10](samples/ritsumeikan-sr/bl-10) | 4 | PFY, TEY |
| Ritsumeikan SR | [BL-11](samples/ritsumeikan-sr/bl-11) | 4 | PEY, TEY |
| Ritsumeikan SR | [BL-13](samples/ritsumeikan-sr/bl-13) | 4 | TEY |
| Ritsumeikan SR | [BL-2](samples/ritsumeikan-sr/bl-2) | 4 | TEY |
| SAGA-LS | [BL10](samples/saga-ls/bl10) | 2 | TEY |
| SAGA-LS | [BL11](samples/saga-ls/bl11) | 4 | CEY, PFY |
| SAGA-LS | [BL12](samples/saga-ls/bl12) | 2 | TEY |
| SLS | [PHOENIX](samples/sls/phoenix/xraylarch) | 1 | TEY, fluorescence |
| SOLARIS | [ASTRA](samples/solaris/astra/rodbuk-VRJ85T) | 1 | fluorescence, transmission |
| SOLARIS | [PIRX](samples/solaris/pirx/rodbuk-58IJY5) | 4 | PFY, TEY, TFY |
| SOLEIL | [SAMBA](samples/soleil/samba/zenodo-7801896) | 3 | fluorescence, transmission |
| SPring-8 | [BL14B2](samples/spring-8/bl14b2) | 5 | transmission |
| SRS Daresbury | [SOX1](samples/srs-daresbury/sox1/demeter) | 1 | unresolved |
| SRS Daresbury | [ex1a](samples/srs-daresbury/ex1a/demeter) | 1 | fluorescence |
| SRS Daresbury | [exf9](samples/srs-daresbury/exf9/demeter) | 1 | fluorescence |
| SSRL | [2-3](samples/ssrl/2-3) | 3 | transmission |
| SSRL | [4-1](samples/ssrl/4-1/xasdatalibrary) | 1 | transmission |
| SSRL | [4-3](samples/ssrl/4-3/xasdatalibrary) | 2 | transmission |
| SSRL | [7-3](samples/ssrl/7-3/demeter) | 1 | transmission |

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

The DELTA example has no identified station. The [original RefXAS notice](LICENSES/LicenseRef-RefXAS-Usage-Notice.txt) and [RefXAS paper citation](https://doi.org/10.1107/S1600577524006751) accompany all 12 examples.

## Provenance and unresolved details

- MAX IV/Balder attribution for the ParSeq examples is tentative and excluded from the confirmed count. The HERFD master and two Eiger projection files are linked by `companion_files`; the latter hold 601×1030 and 526×1030 projections, not full 2D camera-frame stacks.
- DORIS FIO, CAMD projects, LNLS, SSRL MicroEXAFS and Lytle examples retain unresolved station labels. No modern beamline is inferred from a facility name alone.
- `APS13ID_2008.dat` actually identifies 13-BM-D in its header. `SSRL1_2006.dat` identifies SSRL 2-3. The original filenames remain unchanged.
- SAGA BL11 raw files say `Transmission(2)` in the acquisition header, while the dataset column metadata identifies ROI fluorescence and conversion electrons. Those records use PFY/CEY mode tags and preserve the conflict. Aichi `-f` filenames contain transmission data; the suffix was not used to infer fluorescence.
- CLS files are the database’s downloadable exports. Their original acquisition headers are not assumed to survive. Per-spectrum API metadata supplies the license for `Fe_idcj28n9`, whose payload header omits it.
- MDR `record.json` evidence is extracted JSON-LD from each landing page. Downloaded instrument metadata is preserved separately; licenses were checked per record, not inferred from the whole collection.

[RESEARCH_NOTES.md](RESEARCH_NOTES.md) records the first collection pass. The [worldwide research report](research/worldwide-beamlines.md) and [download leads](research/download-leads.csv) record the expanded search and unresolved access/license questions. Coverage figures describe this collection, not an exhaustive census of current beamlines.

Catalog correction: `as2o3_10K_scan1.xdi` identifies SSRL 2-3 in its original header. Its catalog entry and directory were corrected from 4-1 during rexafs integration; the measurement bytes are unchanged.
