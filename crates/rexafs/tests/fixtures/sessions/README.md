# Application session fixtures

The complete bundles below were relocated from `tests/testfiles/` on 2026-09-14.
Their data, manifests, reference arrays, scripts, licenses and historical READMEs
remain byte-for-byte unchanged. Historical Larix notes describe its original
handoff, including the former path and proposed test target. Use this index and
the current [reader guide](../../../../../doc/measurement-reader.md) for implemented
support and commands.

| Bundle | Contents | Source and terms |
| --- | --- | --- |
| [XTUNES](xtunes/README.md) | Four XTS saves and two XTSP projects, saved and reopened in XTUNES 1.3 Build20200228 | [Generation repository, pinned revision](https://github.com/Ameyanagi/xtunes-analysis/tree/e320ed77469d848cd0b99125deb810f3f34ccb91/fixtures/xtunes-generated) (private), [checksums](xtunes/manifest.json), [original MIT notice](xtunes/LICENSE.txt) |
| [Larix](larix/README.md) | 15 valid sessions, six intentional invalid cases and independent expectations for 384 numeric arrays | [Larch 2026.3.1 session writer](https://github.com/xraypy/xraylarch/blob/2026.3.1/larch/io/save_restore.py), [generation and checksum manifest](larix/manifest.json), [original MIT notice](larix/LICENSE.txt) |

Both bundles derive their PFBL12C and PF9A spectra from the attributed
[xraylarch beamline examples](https://github.com/xraypy/xraylarch/tree/e3c93284fed358c2c8979cba4c139430527433c6/examples/xafsdata/beamlines).
The bundles' licenses cover the retained data and any generation code as stated
in their original records; they do not license the XTUNES application. Larix
fixtures were produced through the Larch Python API and independently reopened
in Larch, rather than saved through manual GUI interaction.

Current tests are [`../../measurement_fixtures/xtunes.rs`](../../measurement_fixtures/xtunes.rs)
and [`../../measurement_fixtures/larix.rs`](../../measurement_fixtures/larix.rs).
Synthetic parser contracts live under `../../measurement_reader/`; they do not
need these bundles. The top-level integrity command verifies both bundle
manifests and their license assets without modifying them.
