# Whole-project import validation

Validated on 15 September 2026 on macOS ARM64 with the pinned Rust 1.98.1
toolchain. This is unreleased source-build behavior, not a qualification of the
published 0.2.8 packages. The implementation starts from `dev` commit `0c00a45`.

## Behavior

The import editor previously appended only the currently previewed scan's
checked signals. All scans now start selected. The Scan menu provides All scans,
None, per-scan checkboxes and independent preview buttons. Every scan retains
its own column mapping, units and checked signals. The footer counts all selected
outputs. Conversion completes before groups are appended; a selected unmapped
scan blocks import until it is mapped or explicitly excluded.

The shared reader and public Rust/Python/TypeScript interfaces are unchanged.
This fixes desktop orchestration. Folder discovery additionally recognizes the
core reader's existing `.xts` save format.

## Automated checks

`cargo test --locked -p rexafs-gui`: **507 passed, 5 ignored, 0 failed**.
`cargo build --locked --release -p rexafs-gui`: passed. Existing compiler warnings
remain. Website `npm run check`: no errors, warnings or hints.

The new regression cases exercise whole-container Athena JSON, legacy Athena,
Larix and XTUNES imports; independent custom units and detector choices;
excluded invalid mappings; an empty session; and explicit NeXus mappings.
They compare output arrays and retained evidence, including source identities
and original bytes. No new source measurement files are added.

The existing fixture audit also compares **322 valid GUI signal previews from
203 original sources** with core conversions. This count describes tested
signals, not universal coverage of every possible beamline layout.

## Computer-use checks

Separate QA app bundles and settings kept the user's installed app and projects
unchanged. The final optimized build was checked using native computer use:

- Athena `json_unzipped.prj`: all four scans initially selected; a checkbox
  changed the footer to three; Space restored four. Previewing another record
  retained the selection. Import added four groups; one undo removed all four
  and redo restored them.
- A preceding build with the same conversion code imported four Athena groups,
  two groups from `two-analyzed.larix` and two from `two-analyzed.xtsp`. A portable
  project saved through the GUI contained all eight. Every energy and absorption
  array, record ID and embedded source file matched the original reader output.
  [The saved-array record](saved-array-verification.json) identifies each group.
- The NeXus fixture listed all six tables, including instrument tables. Its scan
  menu scrolled to the final record and dataset selector. Distinct source IDs
  disambiguated repeated sample labels; import stayed disabled until mapping.
  The automated NeXus case imports the three explicitly mapped energy/intensity
  records, preserving 348, 348 and 412 points.

The full, unedited documentation screenshot uses the retained MIT-distributed
Larch fixture. [Capture provenance](capture.json) records source-file hashes,
the signed QA executable, screenshot and input. Private local inputs, executable
bundles, saved projects and logs are not committed.

## Other container formats

The [34-source audit](format-audit.csv) uses the already published 0.2.8 Python
binding to the unchanged reader. It inspects all retained Athena and valid Larix
session fixtures, all six XTUNES saves, and representative SPEC/HDF5 containers.

| Format | Files | Records | Records with a valid detected signal |
| --- | ---: | ---: | ---: |
| Athena, including Larch Athena-compatible projects | 10 | 128 | 118 |
| Larix | 15 | 16 | 14 |
| XTUNES saves and projects | 6 | 8 | 8 |
| HDF5/NeXus | 2 | 10 | 0 |
| SPEC | 1 | 27 | 0 |

Detection is deliberately limited. Ten historical Athena dispersive records
have pixel axes requiring calibration. Larix sessions may contain only χ(k),
non-XAS arrays or no scans; these are not automatically absorption spectra.
The selected HDF5 and SPEC examples need explicit units and detector roles;
instrument tables and detector images must not become extra absorption spectra
by default. MDA binary input remains unsupported.

Input attribution and original checksums remain in the canonical fixture
manifests and adjacent license notices. This change neither broadens data
redistribution permissions nor changes package exclusions.
