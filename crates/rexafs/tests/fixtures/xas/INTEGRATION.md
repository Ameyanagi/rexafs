# Using the beamline corpus in rexafs

This unreleased test addition retains 132 measurement/project files collected
on 2026-09-14, totaling about 27 MB. All files are available from the Git checkout,
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

These tests exercise import and serialization, not fitting or scientific
validation. The remaining files supply material for future reader development;
their inclusion does not claim that rexafs can already parse every beamline
format. All 132 files, their sidecars and 90 license/metadata assets are covered
by the integrity check. The captured `validation.json` also records the original
HDF5 container inspection; HDF5 parsing is not added to the Rust library here.

Run from the repository root:

```sh
cargo test --locked -p rexafs --test beamline_formats
python scripts/check-beamline-fixtures.py
python scripts/check-beamline-fixtures.py --package
```

Normal `cargo test --locked -p rexafs` also discovers these integration tests.
The tests and checks use the local fixture files. The optional collection
restoration script is never invoked by Cargo, a build script or CI tests.

## Crates.io packaging

The core [Cargo manifest](../../../Cargo.toml) excludes
`/tests/fixtures/xas/` and `/tests/beamline_*.rs`. The fixture-dependent
integration test is omitted along with its data so the published source archive
does not contain that test with missing inputs. Ordinary crate users receive
neither the new corpus nor its attribution/metadata payloads through crates.io.
The existing library and its older tests are unchanged.

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
