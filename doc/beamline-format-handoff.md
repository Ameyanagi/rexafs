# Prompt for implementing one measurement format

Replace `<FORMAT_OR_BEAMLINE>` and pass this prompt to one Codex thread. Use a
separate Git worktree for each thread, based on the committed fixture baseline
branch `test/beamline-fixtures` in `~/dev/rexafs`. This avoids switching another
thread's checkout. The baseline contains all 132 fixtures and packaging rules.

```text
Implement rexafs reader support for <FORMAT_OR_BEAMLINE>.

Repository: ~/dev/rexafs
Fixture baseline: test/beamline-fixtures

Create your own branch and separate Git worktree from the committed fixture
baseline. Preserve existing work and do not switch or modify another thread's
checkout. Read AGENTS.md and CONTRIBUTING.md before making changes.

The original measurement corpus is under
crates/rexafs/tests/fixtures/xas/.
Use manifest.json, manifest.csv, INTEGRATION.md and the supplied metadata to
identify the fixtures for your assigned format. The manifest records source
URLs, citations, licenses, beamline attribution, modes and SHA-256 checksums.
The RefXAS examples are obtained from RefXAS.

Scope your work to this format and its necessary import integration. Inspect
existing rexafs::io APIs and follow their conventions. Preserve the original
measurement files and attribution; do not edit fixtures to make a parser pass.
Keep parsing and signal conversion distinct where the existing APIs do so.

Handle the variants actually present: headers, delimiters, encodings, energy
units, detector columns and multiple scans where applicable. Use explicit
metadata and channel definitions for transmission, fluorescence and electron
yield. Do not assume every signal is a logarithmic intensity ratio or every
listed detector is active. Document ambiguous or conflicting metadata and
return useful errors instead of silently guessing or discarding bad rows.

Add meaningful tests in tests/beamline_<format>.rs under crates/rexafs/. Verify
point counts, energy conversion to eV, channel mapping and representative
numeric values against the original files. Exercise all relevant collected
variants and add focused malformed-input tests when they validate real parser
failure modes. Clearly document what remains unsupported.

Keep these fixture-dependent tests named beamline_*.rs. Cargo already excludes
that test-file pattern and tests/fixtures/xas/ from the published crate. Read
fixtures from disk during tests; do not add automatic downloads, runtime data
dependencies or production include_bytes! calls for the corpus.

Run the relevant new tests, then:
  cargo test --locked -p rexafs
  cargo clippy --locked -p rexafs --all-targets -- -D warnings
  cargo fmt --all -- --check
  python scripts/check-beamline-fixtures.py
  python scripts/check-beamline-fixtures.py --package

Update the reader's documentation and report supported variants, validation
results and remaining limitations. Preserve the fixture and package checks.
Do not publish packages or push to a public repository.
```
