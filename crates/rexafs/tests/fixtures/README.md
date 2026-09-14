# Measurement fixtures

The shared reader uses one source-checkout collection across Rust, Python,
TypeScript/WebAssembly and GUI tests. Tests read local files; they never download
or regenerate measurements. The fixture bundles are excluded from crates.io.

| Collection | Retained inputs | Provenance and license records |
| --- | --- | --- |
| [Beamline measurements](xas/README.md) | 154 valid files, grouped by facility and beamline | [Per-file source manifest](xas/manifest.json), [citations](xas/CITATIONS.md), [license index](xas/LICENSE.md) |
| [Expanded measurement corpus](rexafs-corpus/README.md) | 222 files: 182 parsed, including five partial HDF5 recoveries; 40 known rejections | [Original manifest](rexafs-corpus/manifest.json), [licenses](rexafs-corpus/LICENSE.md), [snapshot checksums](rexafs-corpus/SNAPSHOT.json) |
| [Saved sessions](sessions/README.md) | 6 XTUNES files, 15 valid Larix sessions and 6 intentionally invalid Larix sessions | Separate original bundle manifests, source records and MIT notices |

The `xas/` and session suites cover **175 readable inputs**. Readability includes preserved tables,
images and saved results that require explicit interpretation before conversion
to an absorption spectrum. The six invalid sessions are rejection cases, not
supported measurements. See the [reader guide](../../../../doc/measurement-reader.md)
for the distinction and current format limitations.

The expanded corpus was copied from the private `rexafs-format` gathering
checkout after integrity and provenance review. Its 571 original data and
attribution files are retained unchanged. The copied README/research documents
describe that historical collection; no external checkout is needed for tests.
The [audit](../../../../doc/validation/2026-09-14-measurement-corpus/review.md)
records format gaps and numerical regressions. Across `xas/` and `rexafs-corpus/`
there are **243 unique measurement/project payloads** (133 occur in both
snapshots). Of those, 203 parse, including partial recoveries, and 40 have
explicit rejection expectations. The 21 valid application sessions are separate.

## Layout and tests

- `xas/samples/` retains original beamline files in facility/beamline/source order.
- `xas/candidates/kek-pf/` retains five QD measurements for academic, nonmilitary testing under the [original custom usage notice](xas/candidates/kek-pf/README.md).
- `rexafs-corpus/` retains the complete expanded collection with its original
  manifests and attribution. `SNAPSHOT.json` pins every original copied file.
- `sessions/` retains complete application-generated bundles, including original
  licenses, manifests, expected values and generation evidence.
- [`../measurement_reader.rs`](../measurement_reader.rs) runs self-contained
  parser and conversion contracts from `../measurement_reader/`. Synthetic inputs
  are declared in those tests and remain in the published crate archive.
- [`../measurement_fixtures.rs`](../measurement_fixtures.rs) runs the new corpus,
  reference-format and session checks from `../measurement_fixtures/`. Its
  [coverage table](../measurement_fixtures/coverage.csv) stores expected reader
  output separately from the original data manifests.
- [`../beamline_formats.rs`](../beamline_formats.rs) retains the original
  specialized-reader regression target.

From the repository root:

```sh
cargo test --locked -p rexafs --test measurement_reader --test measurement_fixtures --test beamline_formats
uv run --no-project --python 3.12 scripts/check-beamline-fixtures.py
uv run --no-project --python 3.12 scripts/check-beamline-fixtures.py --package
```

## Adding data

For a raw beamline example, follow the [corpus contribution workflow](xas/INTEGRATION.md).
Retain its source URL, revision, upstream license text, original filename, byte
length and checksum. For an application session, retain those records beside
the saved file and identify the source measurements, application version and
save/reopen or generation procedure. Record which expectations come from an
independent reference and which describe rexafs behavior. Keep synthetic damage
cases clearly labeled and linked to their valid parent.

Data licenses apply per source. The library's MIT/Apache-2.0 license does not
replace upstream fixture terms or license third-party applications. Do not infer
redistribution permission merely because a repository is public. Keep notices
with any copied or derived data and preserve historical records unchanged.
