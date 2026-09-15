# rexafs 0.2.8 release qualification

Status: release preparation. No 0.2.8 artifacts have been published.

This release adds one-button desktop updates and extends GUI folder discovery to
the formats already handled by the core measurement reader. Feature development
started from `dev` at `e5cbccb1060c56ce9bb6b18bf2e9bb4a02ef0608`.
The [release runbook](../../releasing.md) governs source promotion, exact-tag
builds, signing, publication and public artifact verification.

## Local preparation evidence

Apple Silicon macOS checks on 15 September 2026 used the pinned Rust 1.98.1
toolchain. Before the coordinated version change:

- The default GUI suite passed 483 tests with five ignored. A later focused
  suite passed 482 tests, with five ignored and four previously passed expensive
  FEFF tests filtered out. These runs overlap and must not be added together.
- The final updater suite passed ten tests covering signature rejection before
  execution, archive link rejection, concurrent installation locks, replacement
  and launch rollback, cancellation, and reuse of a cancelled cached download.
- Folder discovery covered 203 readable original fixtures. All 322 valid GUI
  signal previews matched the shared core conversions.
- The website passed Astro checking with no errors, warnings or hints, a
  139-page build and 22 content/runtime tests. Existing generated binding and
  reference artifacts were reused only after checking that their source matched.
- The normal optimized desktop build passed. No local artifact is a release
  publication input.

## Interactive update and import review

Computer use exercised disposable app copies with isolated settings and public
KEK Cu data plus a synthetic Larix session. KEK `.qd` displayed 5,135 points,
the recorded Bragg-angle conversion and transmission arithmetic. Both Larix
scans could be selected and plotted. Accepting the chosen signals cleared their
pending entries, and workspace shortcuts worked after the preview closed.

An explicitly old, locally built nightly identity allowed the updater to offer
the official signed `nightly-20260915-34921472793` release. One click completed
download, verification, replacement and automatic launch. The installed signed
app then reopened the saved recovery with exactly two accepted spectra and their
plots; no extra folder entries appeared. The transaction retained the old bundle
and the recovery file. A separate cancellation check left the test app unchanged.

These tests used temporary copies under `/tmp/rexafs-updates-review/`, not the
user's installed app or analysis. Their receipt paths are temporary local evidence.
The 0.2.8 signed release must be checked separately after its GitHub build.

## Pending release gates

The 0.2.8 maintainer writer saved and reopened both linked and embedded fixtures.
All 38 retained samples passed checksum/header validation; historical bytes and
hashes are unchanged. The 29 project tests passed with the explicit writer
ignored. The coordinated version check and formatting passed. Core strict Clippy
and the default core suite also passed before the metadata-only version change.

A final review extended the update pause to Assistant receipt navigation/undo
and preferences, including the separate-window host. The 70 selected Assistant
tests passed, including the recovery-pause regression. CI must qualify this
final correction before either branch promotion.

1. Retain the reviewed 0.2.8 linked and embedded fixture pair with this source.
2. Pass and merge the feature PR into `dev`; review its nightly and selected CI.
3. Pass the `dev` → `main` release PR and merge with a merge commit.
4. Tag the merged main commit `v0.2.8` and pass the full manually dispatched
   release matrix for that immutable source.
5. Sign and notarize both Mac targets, inspect actual downloads, and publish
   only the qualified checksummed desktop and registry artifacts.
6. Verify public package bytes and fresh consumers, update public release
   metadata and Stable references, and merge the released state back to `dev`.

## Windows and Linux updater qualification

The release scope now includes one-button updates on Windows and Linux. The
candidate selects native x64/ARM64 archives, verifies a package-file inventory,
preserves user files, and restores a recovery project after restarting. Windows
installations use the matching per-user installer and retain an uninstall-key
backup; portable Windows and Linux copies replace the full extracted folder.
The local updater suite currently passes 17 tests, including archive traversal,
links, case collisions, inventory validation, user-file preservation and rollback.
At `d5215e83d124e910284d05f5818b0db8227c8de7`, all six native desktop jobs
passed in [release build 34934449396](https://github.com/Ameyanagi/rexafs/actions/runs/34934449396).
Both Windows architectures passed portable and installed updates, damaged-payload
rejection, installer rollback, recovery launch, user-file preservation, shortcut
checks and uninstall. Both Linux architectures passed portable updates,
damaged-payload rejection and recovery launch; their saved screenshots showed
the reopened fixture workspace and spectra. Windows checks verified a visible
native window, but did not exercise interaction on a physical Windows desktop.

That run's final manifest job failed because the new updater evidence contained
duplicate basenames such as `checks.json`. The workflow now archives each target's
evidence under a unique name, preserving the receipts while maintaining the
manifest's duplicate-name rejection. The corrected workflow must pass before
merge or publication; successful desktop jobs alone do not qualify the run.
Local verification downloaded the failed run's complete artifact set,
reproduced the duplicate-name error, then executed the corrected archive command
for all four updater targets. All 56 original evidence files retained identical
contents, and creation and verification of the 35-asset manifest passed.

A whole-GUI strict Clippy attempt also found pre-existing lint failures outside
the updater (including `depth_controls.rs` formatting and a `journal.rs` import).
Those diagnostics are not a passing strict GUI check; the release workflow's
required checks remain authoritative.
