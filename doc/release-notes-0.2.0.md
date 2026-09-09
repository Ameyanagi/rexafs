# rexafs 0.2.0

| Desktop | Installer | Portable |
|---|---|---|
| macOS · Apple Silicon | [Download](https://github.com/Ameyanagi/rexafs/releases/download/v0.2.0/rexafs-0.2.0-aarch64-apple-darwin.dmg) | [Archive](https://github.com/Ameyanagi/rexafs/releases/download/v0.2.0/rexafs-0.2.0-aarch64-apple-darwin.zip) |
| macOS · Intel | [Download](https://github.com/Ameyanagi/rexafs/releases/download/v0.2.0/rexafs-0.2.0-x86_64-apple-darwin.dmg) | [Archive](https://github.com/Ameyanagi/rexafs/releases/download/v0.2.0/rexafs-0.2.0-x86_64-apple-darwin.zip) |
| Windows · preview | [Download](https://github.com/Ameyanagi/rexafs/releases/download/v0.2.0/rexafs-0.2.0-x86_64-pc-windows-msvc-setup.exe) | [Archive](https://github.com/Ameyanagi/rexafs/releases/download/v0.2.0/rexafs-0.2.0-x86_64-pc-windows-msvc.zip) |
| Linux · preview | — | [Archive](https://github.com/Ameyanagi/rexafs/releases/download/v0.2.0/rexafs-0.2.0-x86_64-unknown-linux-gnu.tar.gz) |

Mac downloads are signed and notarized. Intel was checked under Rosetta;
Windows/Linux remain previews.

[Installation and offline setup](https://github.com/Ameyanagi/rexafs/blob/main/doc/installing.md)

<details>
<summary>Python · Rust · npm</summary>

| Package | Install |
|---|---|
| [Python](https://pypi.org/project/rexafs/0.2.0/) | `python -m pip install rexafs` |
| [Rust](https://crates.io/crates/rexafs/0.2.0) | `cargo add rexafs` |
| [npm](https://www.npmjs.com/package/rexafs/v/0.2.0) | `npm install rexafs` |

</details>

<details>
<summary>What changed</summary>

The Groups panel keeps current, keyboard focus, marks, and range selection
separate. Browsing a spectrum preserves marks. Source files and their channels
share collapsible stacks; materialized outputs appear in Results. Group names,
colors, processing locks, identities, and selection state survive project save
and reopen. Remove and Duplicate are undoable and preserve independent settings.

Import appends files and folders while retaining the current spectrum. Receipts
report added groups, duplicate sources, warnings, failures, and pending layouts.
Unfamiliar or unreadable sources remain pending until reviewed. A review can
accept part of a mixed folder, choose a representative file, define multiple
output channels, or defer the remaining sources. Reload and Locate support
repairing a missing or changed source.

Re-map columns opens a focused editor with original table values, one-based
column labels, the channel formula, and a full raw μ(E) preview. Explicit eV,
keV, and monochromator-angle conversions run once; angle conversion includes
its required spacing. Fluorescence uses a unique set of ROI columns. Invalid or
stale previews cannot be applied. Mapping edits preserve processing settings,
identity, current, and marks, and can be undone.

Saved recipes record an immutable version, exact layout and units, confirmed
unit assumptions, output channels, and reuse scope. Project recipes take
precedence over remembered computer recipes. Named compatible layouts import
directly; conflicting recipes or changed units return to review. Unnamed
layouts require a representative confirmation in each new batch. Changing a
recipe never rewrites groups imported with an earlier version.

The Data inspector summarizes source, channel, formula, units, recipe version,
and full-signal checks. Review this application targets its saved members,
independently of marks and filters. Batch repair skips incompatible, locked,
or manually remapped groups and applies the accepted set as one undoable edit.
Materialized descendants report Inputs changed. Missing channel creation shows
exact counts, avoids duplicates, and gives each new group independent processing
settings.

Merge displays known incompatibility before running. Declared XDI element and
edge identities take precedence when both are available; otherwise detected
edge positions provide the compatibility check. Materialized outputs retain
edge provenance. Parser totals and bounded source-line examples are saved with
the mapping they checked; historical evidence is identified separately from
current readiness.

The integrated Phase 1 work also reuses fixed-penalty column scaling and SVD
factors for compatible geometry, while checking each spectrum and its requested
condition limit independently.

Older projects remain readable. The project format gains additive recipe,
application, intake-history, edge, and parser-evidence fields. Existing mappings
are retained on reopen, even when machine recipes differ.

Confirmed series and explicit frame ordering, frame-range processing, the
Spectrum/Catalog center switch, persistent LCF/PCA result rows, paired-reference
alignment, drag reorder, Assistant proposal review, transcript search, and
conversation export remain outside this release's current implementation scope.

Mac ZIP and DMG downloads are signed and notarized from the qualified GitHub
build. Intel validation on this host uses Rosetta; native Intel hardware and
clean-machine installation remain unqualified. Windows/Linux remain desktop
previews pending native interactive checks. Their build, archive, numerical,
and installer checks are recorded separately from graphical qualification.

</details>

<details>
<summary>Build and verification</summary>

These assets come from [release build 34204231697](https://github.com/Ameyanagi/rexafs/actions/runs/34204231697),
which passed all 29 jobs at commit `7278be4ce91b80de235ce785782b46e7f5647447`.
Both macOS ZIPs and DMGs were signed and notarized by
[signing run 34208785388](https://github.com/Ameyanagi/rexafs/actions/runs/34208785388).
The original executables were signed without rebuilding. `SHA256SUMS` covers
all final release files, including the signed replacements.

Fresh app and DMG downloads passed signature, stapling, and Gatekeeper checks;
both app architectures passed build-identity and numerical checks. Apple Silicon
passed ReFEFF/FEFF10 checks, native launch and plots, seven-group embedded project
restoration, exact recipe-application/raw preview, parser diagnostics, and saved
Assistant transcript restoration. Intel passed launch, plots, and the same
project restoration under Rosetta.

See the [qualification report](https://github.com/Ameyanagi/rexafs/blob/main/doc/validation/2026-09-08-release-0.2.0/review.md)
for the recorded evidence and remaining platform and upstream dependency limits.

The Rust crate, npm tarball, all 20 Python wheels, and Python source archive
were verified against the same qualified build. They remain available through
their package registries; GitHub's download list contains the desktop assets.

</details>
