# Pending import controls, 2026-09-17

These checks cover unreleased changes after `5b6cb96` on
`feature/pending-import-controls`. Computer use operated a separate native macOS ARM64
release app with generated signals. The user's experimental import session was
not used for these tests. Windows and Linux visual behavior was not checked.

## Reproduce the inputs

Run `python3 generate.py /tmp/rexafs-pending-example` using a new output directory.
Import `accepted.xdi` and accept its single stored signal. Import the generated
`pending` folder, then close the mapping preview to view its five pending files.

The 401-point signal is a logistic edge plus a Gaussian peak on an axis in eV;
it is a software-test curve, not a physical absorption measurement. The two
project files and the `.xts` file contain identical synthetic Athena text.
That deliberately mixed naming tests extension-based skipping independently of
content-based reader detection. It does not claim native XTUNES format coverage.
The two `.spec` files contain the same signal in SPEC syntax.

## Observed workflow

- The list showed five filenames with individual × buttons and a Skip menu.
  Repeated explanatory paragraphs were replaced by hover details.
- Skipping one SPEC file changed Pending from 5 to 4; the accepted spectrum and
  its plot were unchanged. Undo skip restored the file and Pending returned to 5.
- Skip .prj (2) removed both the lowercase `.prj` and uppercase `.PRJ` entries.
  The `.xts` and two `.spec` entries remained.
- Skip all pending (3) removed the remaining entries. The summary reported five
  skipped sources; the Undo notice covered the most recent three. Undo restored
  those three, without restoring the earlier project-file selection.
- Skipping one SPEC sibling before reviewing the other changed its import button
  to Import 1 spectrum. Accepting created only that spectrum. The two accepted
  groups remained present when the final `.xts` entry was skipped.
- The menu overlays the workspace. Opening and closing it does not resize the
  spectrum plot. The public screenshot shows the first five-source menu.

The skip action captures exact batch/path identities and rechecks pending status.
Already approved sources cannot be skipped. A previously opened measurement
preview is checked again before adding groups, so it cannot import a skipped
source. Later imports are independent. Undo avoids duplicating a source that has
since entered another import. Project serialization retains the original pending
reason and any earlier skip note, in addition to the new skipped status.

## Checks

The import-focused GUI suite passed: 82 tests, zero failures and one ignored test.
This includes nine intake-state tests; three new regressions cover extension
matching and captured scope, source/group preservation, Undo, persisted evidence,
later reimports and stale review rejection. The optimized release build passed.
`cargo fmt --all -- --check` and `git diff --check` passed. The website's
`npm run check` reported zero errors, warnings or hints across 37 files.
All six reproduced input files matched the GUI validation inputs byte for byte.
