# Using the beamline corpus in rexafs

This unreleased collection retains 149 measurement, project and reference files as of
2026-09-14: the original 132, three original NIST BMM standards and 13 Demeter/Larch examples. All files are available from the Git checkout,
including the 12 examples obtained from RefXAS. Source links, citations and the
original RefXAS usage notice are retained. The original collection's directory
names and license-evidence classifications remain in the manifest; every
payload is included in the integrity check.

The [manifest](manifest.json) records the source, byte size, SHA-256 checksum,
beamline, format and attribution for each file. [CITATIONS.md](CITATIONS.md) and
[LICENSE.md](LICENSE.md) explain the source attributions and retained terms.
Measurement bytes are not rewritten to conform to rexafs readers. A few original
acquisition formats contain prior beamline corrections; the manifest distinguishes
them from normalized exports and analysis projects.

## Tests available now

[`beamline_formats.rs`](../../beamline_formats.rs) reads the files from disk
through the existing rexafs APIs. Its 18 regressions cover:

- Eleven XAS Data Interchange (XDI) files, including transmission, fluorescence,
  reordered detector columns, direct absorption columns and retained beamline
  metadata. Expected point counts, energy endpoints in electronvolts and the
  first absorption value come from the supplied measurement tables.
- Three NSLS-II QAS transmission scans with extra detector channels, including
  a scan containing very small nonzero detector values.
- Four Athena projects from ALS, CAMD and ESRF, covering gzip and plain text
  serialization. Tests retain all groups and check round-trip preservation of
  measured arrays, including the pixel axes in the dispersive project.

The universal reader now adds `measurement_fixtures/corpus.rs`, which reads every one
of the 149 files, and `measurement_reader.rs`, which tests conversion rules and
synthetic malformed inputs. `measurement_fixtures/reference.rs` checks original Demeter/Larch detector arithmetic, FDMNES relative energy and legacy Athena metadata. `measurement_fixtures/xtunes.rs` covers six separately attributed
XTUNES saves/projects. See the [reader guide](../../../../../doc/measurement-reader.md)
and [per-file coverage](../../measurement_fixtures/coverage.csv).
Readable arrays requiring calibration or detector reduction are distinguished
from automatically convertible absorption scans. These are parser/software
regressions, not experimental or processing-algorithm validation.

All 149 corpus files, their sidecars and 92 retained license/metadata assets are
integrity checked. The captured `validation.json` remains the historical
collection-time report; it is not the qualification record for the new reader.

Run from the repository root:

```sh
cargo test --locked -p rexafs --test beamline_formats --test measurement_fixtures
python scripts/check-beamline-fixtures.py
python scripts/check-beamline-fixtures.py --package
```

Normal `cargo test --locked -p rexafs` also discovers these integration tests.
The tests and checks use the local fixture files. The optional collection
restoration script is never invoked by Cargo, a build script or CI tests.

## Crates.io packaging

The core [Cargo manifest](../../../Cargo.toml) excludes
`/tests/fixtures/xas/`, `/tests/fixtures/sessions/`, `/tests/beamline_formats.rs`,
`/tests/measurement_fixtures.rs` and `/tests/measurement_fixtures/`.
Fixture-dependent test targets are omitted along with their data so the published source archive
does not contain that test with missing inputs. Ordinary crate users receive
neither the new corpus nor its attribution/metadata payloads through crates.io.
Self-contained `measurement_reader` contracts remain in the published archive.

The exclusion is checked against Cargo's real file list in Rust CI and release
qualification. Cargo's [include/exclude rules](https://doc.rust-lang.org/cargo/reference/manifest.html#the-exclude-and-include-fields)
govern the archive; `include`, if added later, overrides `exclude`, so keep this
packaging check when changing the manifest.

## Adding or correcting fixtures

Retain the original bytes and filename, add the source and attribution to
`manifest.json`, and supply the adjacent `.license` file and referenced license
text. Keep the CSV and citation catalog consistent. Record corrections to
catalog metadata explicitly without editing historical measurements.

Add format regressions to a dedicated `tests/beamline_<format>.rs` file as reader
support becomes available. That naming convention keeps independently developed
format tests outside the published crate. Missing fixtures must fail the test rather than silently skipping it
or triggering a download. Verify the corpus and the package list before review.
The original-file directory has a narrow exemption from the general 1 MB file
limit, and Git attributes preserve its line endings and whitespace.
