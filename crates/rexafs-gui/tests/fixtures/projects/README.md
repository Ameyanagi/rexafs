# Retained project fixtures

The first release project format is `.rxs` version 1. Unreleased codename formats
are not supported or retained as compatibility fixtures.

| File | Coverage |
|---|---|
| `minimal-v1.rxs` | Required header and defaults for omitted optional state |
| `format-v1-defaults.json` | Frozen meaning of omitted format-1 fields; additive fields may be introduced without changing existing defaults |
| `rexafs-0.1.0-links.rxs` | Relative sources, metadata, overrides, bounds, derived data, joint assignments, history and publication settings/captions |
| `rexafs-0.1.0-embedded.rxs` | The same state with compressed originals and duplicate-payload deduplication |
| `rexafs-0.1.1-links.rxs`, `rexafs-0.1.1-embedded.rxs` | Saved and reopened through the 0.1.1 writer from the 0.1.0 linked fixture; same format and complete state in both storage modes |
| `rexafs-0.1.2-links.rxs`, `rexafs-0.1.2-embedded.rxs` | Saved and reopened through the 0.1.2 writer; stable reference-group identity, independent processing, embedded synthetic χ standard and explicit inverse grid |
| `rexafs-0.1.3-links.rxs`, `rexafs-0.1.3-embedded.rxs` | Saved and reopened through the 0.1.3 writer from the 0.1.2 linked fixture; same format and preserved reference/standard/inverse-grid state |
| `rexafs-0.2.0-links.rxs`, `rexafs-0.2.0-embedded.rxs` | Groups and locks, two immutable recipe versions, an application retaining version 1, pending source, declared Cu K identity and full parser diagnostics |
| `rexafs-0.2.1-links.rxs`, `rexafs-0.2.1-embedded.rxs` | Retained 0.2.0 state plus an AUTOBK/FFT weight link with its independent value, reversed Viridis group assignments, and publication style with a mixed Japanese/Latin title |
| `rexafs-0.2.2-links.rxs`, `rexafs-0.2.2-embedded.rxs` | Saved and reopened through the 0.2.2 writer; unchanged format-1 state including recipes, locks, weight links, palettes, publication style and Assistant history |
| `rexafs-0.2.3-links.rxs`, `rexafs-0.2.3-embedded.rxs` | Saved and reopened through the corrected 0.2.3 writer; unchanged format-1 state |
| `future-version.rxs` | Future format: reject without modification |
| `truncated.rxs` | Corrupt/incomplete input: reject without modification |
| `data/*.xmu`, `feff/*.dat` | Real inputs for relocation, byte recovery and processing checks |

The 0.2.2 pair is retained from an unpublished qualification attempt. Its tag
remains immutable; the corrected release is 0.2.3.

Settings, the derived example and recorded fit statistics are synthetic persistence
examples, not scientific fit-reference results. Raw data is copied unchanged from
`crates/rexafs/tests/testfiles/xraylarch_d867/xafsdata/cu_150k.xmu`;
`second.xmu` deliberately duplicates it. The path is copied unchanged from
`feffit/Feff_Cu/feff0001.dat` in the same collection. See the original
[provenance](../../../../rexafs/tests/testfiles/xraylarch_d867/README.md) and
retained comments for attribution. The manifest checksums every project and input.

For **each release**, save small linked and embedded projects with its fields,
review their contents, add new files and append a release entry and hashes.
Keep every previously released sample. Rust discovers all project samples from
the manifest and checks full state after load/save/reopen. Intentionally invalid
samples must appear in `invalid_projects`. Add targeted assertions for new
migrations/defaults; do not regenerate previously released fixtures.

Run `python scripts/check-compatibility-fixtures.py` with Python 3.12+ and the
desktop tests in the [compatibility policy](../../../../../doc/project-compatibility.md).
GitHub runs these checks across release platforms. A version bump without both
fixture modes fails the release gate.

The 0.1.2 reference source `data/Ru_QAS.dat` is copied unchanged from the repository public test fixture `crates/rexafs/tests/testfiles/Ru_QAS.dat`. Its channel is linked by group ID 27. The embedded χ standard is a synthetic persistence example, not a recommended background standard.

The 0.1.4 pair adds per-path coordination number N and two synthetic saved
Assistant conversations, including thinking, tool activity, receipts and status
entries. It was saved through the 0.1.4 writer. The current maintainer writer loads
the retained 0.2.0 linked project and adds the 0.2.1 settings. To generate a new pair explicitly,
set `REXAFS_FIXTURE_OUTPUT` and run `cargo test -p rexafs-gui
write_release_compatibility_fixtures -- --ignored`; the maintainer test refuses
to overwrite existing fixtures. Review and checksum the new files afterward.


The 0.2.0 pair retains the 0.1.4 state and adds two import samples.
`data/import-Cu.xdi` and `data/import-diagnostics.dat` contain the exact decoded
energy/μ values from `data/cu_150k.xmu`, serialized with round-trip precision.
The XDI wrapper declares Cu K; the generic wrapper adds nine deliberately
malformed tail rows to exercise saved counts and bounded line examples. These
wrappers are persistence test fixtures. `data/unavailable-pending.dat` is an
intentionally absent source in the pending-import ledger, not an embedded asset.
The saved application refers to stopped recipe version 1 while version 2 is
eligible for future reuse. Both new sources are marked; the diagnostics group
also has a saved processing lock. All older fixture bytes and hashes are retained.
