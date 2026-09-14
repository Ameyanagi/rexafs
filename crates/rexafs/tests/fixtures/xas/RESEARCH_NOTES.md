# Collection notes

Collected on 2026-09-14 for noncommercial rexafs format tests. Selection favored
distinct beamlines, acquisition formats and detector modes over hundreds of
nearly identical scans. This is a broad initial corpus, not an exhaustive
collection of everything publicly downloadable.

| Source | Measurement/project files | Selection |
| --- | ---: | --- |
| [Larch](https://github.com/xraypy/xraylarch) | 26 | Beamline fixtures plus Athena project variants; repository MIT license |
| [Demeter](https://github.com/bruceravel/demeter) | 9 | Additional legacy binary, SRS, LNLS and Lytle fixtures; supplied Artistic license |
| [XASDataLibrary](https://github.com/XraySpectroscopy/XASDataLibrary) | 11 | XDI spectra adding independently identified beamlines; explicit data CC0 dedication |
| [MDR XAFS DB](https://mdr.nims.go.jp/collections/a0f3fbf1-f94a-4be7-8a66-6b91a24f6b54) | 40 | 27 records across Japanese beamlines; raw files and selected processed companions; per-record licenses |
| [CLS XASDB](https://xasdb.lightsource.ca/) | 24 | Three examples from each of eight source beamline labels, including the historical GILDA/LISA aliases; per-spectrum CC BY declarations |
| [ParSeq-XAS](https://github.com/kklmn/ParSeq-XAS) | 7 | FIO, foil scans and complete supplied HERFD master/projection example; repository MIT license |
| [xasref](https://github.com/Ameyanagi/xasref) | 3 | NSLS-II QAS interpolated scans retaining detector channels; repository MIT license |
| [RefXAS](https://github.com/San-WierPa/xafsdb_webserver) | 12 | Obtained from RefXAS; native or channel-preserving format examples plus normalized exports |

MDR JSON-LD identifies each dataset license and DOI. Where available, the
original JSON/instrument metadata is included in `provenance/mdr/`. These
metadata files are especially useful for the Japanese formats, because a
detector column can differ from the acquisition program's generic mode label.
The originally requested [ZrN dataset](https://mdr.nims.go.jp/datasets/3ff63598-8621-4ef6-8de9-5426bf12adac)
is included.

CLS spectrum metadata was retrieved through the site's public
`xasgetcompounds` API. The evidence records preserve its per-spectrum header
values and exact download path. API requests are described in the manifest
assets. The original database downloads are kept byte-for-byte.

These 12 examples were obtained from RefXAS, from its public repository's
`quality_control/example data/SYNCHROTRON/` directory, pinned to commit
`91b3f9be8e628e4c140de663a73b913ad138ad1a`. Some files already contain explanatory
comments or processing applied upstream. “Unmodified” here means unchanged
from that published copy. The live RefXAS ZIP inspected for its notice supplied
raw-mu/normalized-mu exports and metadata rather than the original native
acquisition file; the GitHub fixtures therefore provide useful additional
format coverage. The original usage notice and citation are retained.

## Investigated but not selected

- [DanPorter/xmcd_analysis_example](https://github.com/DanPorter/xmcd_analysis_example)
  contains useful Diamond I06-1 SRS and I10-1 NeXus spectra, but no explicit
  repository/data license was located. Their payloads are not in this corpus.
- [DanPorter/hdfmap](https://github.com/DanPorter/hdfmap) has an Apache-licensed
  I06 NeXus fixture. Its scan uses two fixed energies for XMCD/PEEM imaging,
  rather than an XANES/EXAFS energy scan; it was not counted as XAS coverage.
- [Diamond's older XAS website repository](https://github.com/DiamondLightSource/diamond-xas-website)
  includes mirrored examples whose headers identify other facilities. They
  were not relabeled as Diamond measurements.
- [Cruzeiro-do-Sul-Database](https://github.com/jamesmalmeida/Cruzeiro-do-Sul-Database)
  contains copied SSHADE/FAME spectra. The software license alone was not used
  to assign a license to those third-party records. Primary SSHADE record
  permissions and attribution remain to be checked before adding them.
- Larch FDMNES examples are simulated spectra, so they were excluded from this
  experimental measurement collection.
- Exact duplicates shared by Larch and Demeter were avoided. The final corpus
  has no duplicate measurement SHA-256 hashes. Distinct scans and raw/export
  companions remain when they exercise useful format differences.

## Remaining gaps

Further useful targets include explicitly licensed Diamond energy scans,
Australian Synchrotron, BESSY, additional MAX IV and ESRF stations, and newer
native NeXus formats. Fluorescence and transmission are represented across the
corpus but not necessarily by separate confirmed acquisitions at every station.
Full detector image frames are not supplied by the ParSeq HERFD example; its
provided detector projections are included.

The license evidence is a record of published terms, not a guarantee that every
upstream contributor held every right. For automatic selection, filter on
`test_eligibility` and, if desired, require `license_basis: explicit-data-license`.
All notices and citations must accompany redistributed fixtures.
