# Beamline source repositories

Research recorded on 2026-09-14 for the unreleased universal reader. A beamline
acquisition repository documents its software; it does not necessarily contain
redistributable measurements. Pin commit URLs when adding files and check the
license covering the data themselves. Preserve original bytes, credit and
license notices. The complete retained-source catalog is in the
[fixture manifest](../crates/rexafs/tests/fixtures/xas/manifest.json).

The later [expanded-corpus audit](validation/2026-09-14-measurement-corpus/review.md)
checks `../rexafs-format` at `d142ad2`: 222 original files, including 89 additional
unique payloads beyond the retained rexafs corpus. It records reader outcomes,
per-file source URLs, license bases, numerical tests and implementation priorities.
The complete collection has been copied to `tests/fixtures/rexafs-corpus/`, with
no test dependency on the private gathering checkout. It remains excluded from
the crate archive; parsing success is not a claim of complete format support.

| Beamline or source | GitHub repository | Findings and follow-up |
|---|---|---|
| NSLS-II 6-BM BMM | [BMM standards](https://github.com/NSLS2/bmm-standards) | Three original XDI files added from commit `7f5728eb2b59d98305e41a908d0791b0baeab869`: Fe K, Au L3 and As K reference standards. The original `NSLS-II-BMM/bmm-standards` URL redirects here. |
| NSLS-II 6-BM BMM | [Acquisition profile](https://github.com/NSLS-II-BMM/profile_collection) | Commit `6edf773d3216e26947eff0704ba9b8325287cd79`; no `.dat`, `.xdi`, `.h5`, `.nxs` or `.fio` measurement files found in the inspected tree. Useful for current channel and XDI writer definitions. |
| NSLS-II 7-BM QAS | [Acquisition profile](https://github.com/NSLS-II-QAS/profile_collection) | Commit `c72294ff5d1e3726debfe4d97ede355a922de39e`; BSD-3-Clause repository license. No measurement payloads with the extensions above found. Retained QAS measurements already come from xasref. |
| NSLS-II 8-ID ISS | [Acquisition profile](https://github.com/NSLS-II-ISS/profile_collection) | Commit `b2c6b292c51ec4983d041165aff6b6ce641bd13a`; BSD-3-Clause repository license. No measurement payloads with the extensions above found. |
| NSLS-II 8-ID ISS | [xas](https://github.com/NSLS-II-ISS/xas) and [isstools](https://github.com/NSLS-II-ISS/isstools) | Useful acquisition/analysis sources; no measurement payloads with the inspected extensions found. GitHub reports nonstandard licenses; no data were copied. |
| Multiple facilities | [pynxxas](https://github.com/XraySpectroscopy/pynxxas) | MIT-licensed NeXus conversion library with BLISS, Photon Factory and multi-element fluorescence examples. Commit `7e5f738c160e41f949c8441115856130010ca1da`. Many text examples duplicate the retained corpus; candidate for additional independently checked NeXus layouts. |
| Historical NSLS, SSRL, SRS, Photon Factory and Lytle formats | [Demeter plugins](https://github.com/bruceravel/demeter/tree/master/lib/Demeter/Plugins) | Audited commit `06afc8da08a5a7d5a26ee14992170fcf5dc67406`. Four original SSRL/X23A2 files added with the supplied Artistic license option; existing duplicate examples reused. See the [plugin audit](reference-format-audit.md). Retain Bruce Ravel and original contributor credit. |
| Multiple beamlines and calculated references | [Larch](https://github.com/xraypy/xraylarch) | Audited commit `e3c93284fed358c2c8979cba4c139430527433c6`. Nine original MIT-licensed examples added: FDMNES, multielement XDI, chi(k), generic columns and legacy Athena projects. The [audit](reference-format-audit.md) records reader scope and remaining gaps. |
| MAX IV attribution unresolved | [ParSeq-XAS](https://github.com/kklmn/ParSeq-XAS) | Existing Sardana and Eiger fixtures. Keep BALDER attribution tentative, as recorded in the manifest. |

The BMM [license](https://github.com/NSLS2/bmm-standards/blob/7f5728eb2b59d98305e41a908d0791b0baeab869/LICENSE)
explicitly grants worldwide distribution rights with NIST acknowledgment. The
original notice is retained as
[NSLS2--bmm-standards--LICENSE](../crates/rexafs/tests/fixtures/xas/LICENSES/NSLS2--bmm-standards--LICENSE).
Credit: Bruce Ravel (2025), National Institute of Standards and Technology,
[A collection of X-ray Absorption Spectroscopy data…](https://doi.org/10.18434/mds2-4032),
version 1.0.0. The three files are unmodified NIST data. Their `Scan.plot_hint`
identifies reference transmission `ln(It/Ir)`; tests verify that stored `xmu`
is retained rather than recomputed from the sample's incident monitor.

This is a record of sources actually inspected, not an exhaustive list of every
beamline's repository. Future additions should follow
[the fixture integration instructions](../crates/rexafs/tests/fixtures/xas/INTEGRATION.md)
and add numeric format regressions, rather than increasing file counts alone.

## Additional raw-data audit

The following sources were inspected on 2026-09-14. A download advertised as
“raw data” may contain averaged or interpolated spectra; inspect the original
header before assigning a data level. File availability, redistribution rights,
and reader coverage are separate findings.

| Source | Data and reuse evidence | Result |
|---|---|---|
| ESRF BM16 | [Original BLISS acquisition](https://github.com/XraySpectroscopy/pynxxas/blob/7e5f738c160e41f949c8441115856130010ca1da/examples/manual/BlissMultiModal/test_Assolution_Mauro_0001.h5), distributed with the repository's [MIT notice](https://github.com/XraySpectroscopy/pynxxas/blob/7e5f738c160e41f949c8441115856130010ca1da/LICENSE). | Added the complete 77,957,635-byte original file, pinned Git blob, SHA-256 and license. Its HDF5 metadata explicitly identifies `ESRF-bm16`. The current reader recovers 1,974 numeric datasets but reports 64 linked-group enumeration warnings; complete HDF5 coverage is **not** established. Regression tests compare selected canonical energy and intensity arrays against independent h5py inspection. |
| Photon Factory BL12C, BL9C, BL9A and NW10A | [LiFePO4 Quick Scan, `fe002_0.qd`](https://pfxafs.kek.jp/xafsdata/view.php?id=132), measured by Yasuhiro Inada, and four other [audited examples](validation/2026-09-14-kek-qd/review.md). The [database terms](https://pfxafs.kek.jp/xafsdata/) permit academic, nonmilitary research use and request contact with the experimenter for publication citation. | Downloaded five actual 9809 `.qd` files through the site's session-aware download forms. Four transmission scans pass independent numerical checks. One BL9A file has conflicting fluorescence/transmission declarations and requires explicit arithmetic. At the user’s request, the five originals are now retained as [repository fixtures](../crates/rexafs/tests/fixtures/xas/candidates/kek-pf/README.md) for academic, nonmilitary research and reader regression testing only, with original usage notices and attribution. No standard data license is assigned; they are excluded from published packages. |
| Photon Factory QXAFS | [KEK-PF QXAFS v8 manual](https://pfxafs.kek.jp/wp-content/uploads/bldata/QXAFS_v8.pdf), pages 7 and 9. | `.qc` is a measurement-condition file, not a spectrum, and remains outside the reader. Actual `.qd` examples have now confirmed 9809 content; `.dat` and `.qd` are accepted by the same content detector. |
| Diamond B18 | [University of Strathclyde dataset](https://doi.org/10.15129/17f08512-ae0a-42e8-be4e-2f816f53da4a), Edward Brightman and credited collaborators; original Quick EXAFS ASCII data from proposal SP33569-1. The download is explicitly CC BY 4.0. | A promising source of native B18 data. The repository's 534 MB ZIP download returned HTTP 403 during this audit, so no files were added or claimed as tested. [ixdat's reader](https://github.com/ixdat/ixdat/blob/6adb4239693977aec80ce08d2b5c24e29581bde5/src/ixdat/readers/qexafs.py) provides an independent format reference; its inspected repository did not contain the corresponding measurement fixture. |
| SOLEIL ROCK | [Research Data Gouv dataset](https://doi.org/10.57745/EQL6HT), Axel Wilson and credited collaborators, published 2025-04-30 under Etalab Open License 2.0. | Downloaded and inspected `DataXAS.zip`: 3,361 files, including 3,214 text files. The inspected Au record states averaging 2,350 scans and interpolation onto a new energy grid, and references separate acquisition `.nxs` files. This is useful exported-channel evidence, not an untouched acquisition stream. No duplicate exports were added to the existing ROCK coverage. |
| Soft-X-ray example collection | [Thorondor examples](https://github.com/DSimonne/Thorondor/tree/f44bdec0bfc4c374b1c24eb38dace1e6043ee1cd/thorondor/Example/ExampleData), distributed with GPL-3.0. | Inspected `240120_003.txt`: energy-like values are negative and its header does not identify a facility or define the necessary calibration. No beamline attribution or automatic energy convention was inferred, and no new fixture was added. |

The ESRF example's [upstream converter](https://github.com/XraySpectroscopy/pynxxas/blob/7e5f738c160e41f949c8441115856130010ca1da/examples/manual/BlissMultiModal/convert_to_nexus.py)
selects `energy_enc` and the background-subtracted `p201` intensity channels.
The reader retains those channels without repeating the beamline's corrections.
The new numerical regression is in
[`measurement_fixtures/reference.rs`](../crates/rexafs/tests/measurement_fixtures/reference.rs).
The original file and its tests remain excluded from the crate archive.
