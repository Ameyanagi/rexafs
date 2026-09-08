# Release 0.1.4 qualification

Published [v0.1.4](https://github.com/Ameyanagi/rexafs/releases/tag/v0.1.4) has
42 assets. Tag commit: `6041c78824b18c54e6ffc932000411bfc6a19382`, merged by
[PR #38](https://github.com/Ameyanagi/rexafs/pull/38). Its tree matches the
reviewed release preparation `96b909b`.

The manually dispatched [build 34184418160](https://github.com/Ameyanagi/rexafs/actions/runs/34184418160)
passed all 29 jobs. [Signing 34189680533](https://github.com/Ameyanagi/rexafs/actions/runs/34189680533)
passed for both macOS targets, using that build's original binaries. The signed
ZIPs, DMGs, and checksums replaced the draft's unsigned Mac assets. All 42 server
asset SHA-256 digests match the final manifest; original non-Mac artifact hashes
were retained.

Both fresh Mac extractions passed codesign, stapling, Gatekeeper, and numerical
self-checks. Apple Silicon passed launch, plot rendering, embedded project
restoration, and restoration of both saved Assistant conversations and historical
receipts. ReFEFF and FEFF10 self-checks passed. The Intel archive passed launch,
plots, project restoration, and numerical checks under Rosetta on Apple Silicon.
Native Intel hardware and clean-machine installation were not tested locally.
Windows/Linux remain previews pending native interactive qualification; their
build/installer checks passed in the release matrix.

All registry workflows succeeded using build 34184418160:

- [Rust: 34190269699](https://github.com/Ameyanagi/rexafs/actions/runs/34190269699)
- [npm: 34190271206](https://github.com/Ameyanagi/rexafs/actions/runs/34190271206)
- [PyPI: 34190273012](https://github.com/Ameyanagi/rexafs/actions/runs/34190273012)

Registry metadata/download checks confirmed the exact Rust crate checksum, npm
package bytes, all 20 Python wheel hashes, and the source archive hash against
the GitHub build. Draft creation was run 34189914663.

Local preparation included core/default/ndarray/trust-region tests, strict core
clippy, rustdoc, verified Cargo packaging, licenses, release/installer script
tests, actionlint, all CPython 3.10–3.14 wheel consumers, source rebuild/API checks,
Node/Chromium/TypeScript package consumers, and the 291-test release GUI suite.
Logs are retained under `/tmp/rexafs-014-gates/`; final registry verification is
`/tmp/rexafs-014-registries.log`, and final artifact digest evidence is
`/tmp/rexafs-014-draft-asset-digests.json`.
