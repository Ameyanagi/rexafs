# Expanded corpus: canonical storage

The expanded corpus and the earlier beamline collection now share one physical
collection under [`xas/`](../../README.md). This directory retains the expanded
collection's original catalogs and historical records. Tests require no external
checkout, download, symbolic link or generated copy.

- [`manifest.json`](manifest.json) retains all 222 measurement entries and their
  original paths, source URLs, licenses and checksums.
- [`SNAPSHOT.json`](SNAPSHOT.json) retains checksums for all 571 original files,
  including the measurements, license texts, sidecars and research records.
- [`paths.json`](paths.json) maps each former collection-relative filename to
  its canonical path, relative to `xas/`. Resolve through this map before opening
  a file named in the historical manifest or snapshot.
- [`README.md`](README.md), [`CITATIONS.md`](CITATIONS.md), [`LICENSE.md`](LICENSE.md),
  the research records and collection script preserve their original bytes.
  Relative paths and commands in those archived documents describe the original
  gathering layout. Use this index and the repository commands below for the
  current checkout.

The consolidation removed 356 identical copies and moved 216 unique files,
retaining every original byte sequence. Identical measurements still retain all
their manifest entries and original attribution. No license is inferred from
deduplication, and no source or license notice has been discarded.

The shared ESRF BM16 acquisition has two distinct historical attribution
sidecars. Both are retained: the original `xas/` sidecar stays beside the canonical
measurement, and the expanded collection's additional notice remains at the
path recorded in `paths.json`. Their text is checked separately.

From the repository root:

```sh
python scripts/check-beamline-fixtures.py
python scripts/check-beamline-fixtures.py --package
cargo test --locked -p rexafs --test measurement_fixtures
```

The checker resolves every snapshot entry, verifies its size and SHA-256, checks
the original attribution text, and runs the retained container checks. Rust tests
use the same path map to check all 222 reader outcomes and the independent
numerical regressions. Python, TypeScript and GUI tests already use the canonical
`xas/` paths. Session bundles remain in `fixtures/sessions/`.
