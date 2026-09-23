# Pull-request integration check

On 23 September 2026, the reviewed desktop changes were applied to a new branch
from `origin/dev` at `b0b7872e`. That base identifies the source as 0.2.13 and
includes the newer release documentation, copper example and RMC work. The
three-way application completed without conflicts. This is an unreleased
feature branch, not a new published version.

The earlier [desktop review](README.md) retains its actual 0.2.12 working-tree
identity, screenshots and timed replay. Those observations are not relabeled
as computer-use tests of the newer base. The following checks ran after
integration, on macOS 26.5.1 with Rust 1.98.1:

- `cargo test --locked -p rexafs-gui -- --test-threads=4 --skip feff --skip rmc`:
  **646 passed, 8 ignored, 37 filtered out**, with no failures, in 107.72 seconds.
  This intentionally excludes FEFF/RMC-named tests and ignored integrations;
  it is not the complete optional-backend suite.
  [GUI test output](pr-gui-tests.log).
- `uv run --no-project --python 3.12 python scripts/check-tooling.py`:
  all 12 tooling suites passed. [Tooling output](pr-tooling-tests.log).
- `npm --prefix website run check`: 38 files checked with no errors, warnings
  or hints. [Website output](pr-website-check.log).
- Pre-commit 4.5.1 passed on the staged changes for both commit and pre-push
  stages. Formatting, conflict markers, added-file sizes and release tooling
  passed. Core-only hooks were correctly skipped because the change does not
  modify core sources or dependency manifests. [Commit checks](pr-precommit.log)
  and [push checks](pr-prepush.log).
- Relative links in the changed Markdown files resolve locally. The new
  workflow guides and qualification records are classified in the documentation
  audience inventory. Whitespace checks passed.

The GUI tests emit an existing unused-field warning and a linker unwind-table
warning. This is not a warning-free qualification. Website type/content checks
do not replace a full website build and browser suite. Desktop computer-use
evidence covers macOS; physical beamline acquisition and end-to-end connected
Google Drive downloads were not qualified by these integration checks.
