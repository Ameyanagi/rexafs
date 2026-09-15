# rexafs 0.2.7

Published on 15 September 2026 on crates.io, PyPI, npm and GitHub Releases. The
[qualification record](validation/2026-09-15-release-0.2.7/review.md) tracks the
reviewed source, exact-tag builds, package publication and signed downloads.
See the [illustrated Assistant guide](https://rexafs.com/docs/desktop/assistant/)
for the updated menus and panel workflow.

## Assistant

- The message field has a compact footer with Model, Reasoning and Access menus.
  Model and reasoning controls show the effective selections while retaining
  Automatic and Model default. Plot images and Web search are in Assistant settings.
- Parameters and Groups remain accessible beside the Assistant. Opening a panel
  closes the older panel first if space is tight, then temporarily narrows the
  Assistant. Its preferred width returns when space permits.
- Keyboard navigation preserves Tab and workspace shortcuts. Enter and Space
  activate menus once; arrow keys browse choices and Escape closes a menu.
- On macOS, apps opened from Finder discover common runtime installations needed
  by npm/Bun Codex launchers. Startup failures show a bounded diagnostic instead
  of only “Codex disconnected.” Explicit executable overrides retain priority.

Authentication, analysis permissions and command approval behavior are unchanged.
The Assistant remains experimental and uses the user's installed Codex account.

## Measurement readers

- Repeated or conflicting detector-role labels require explicit column selection.
  Distinct stored signals, such as transmission and fluorescence absorption,
  remain separate choices. Repeated stored labels identify their column.
- Malformed leading numeric rows are rejected with a line-specific error even
  when comments separate them from later valid rows.
- Edited Athena `i0`, `signal` and `stddev` arrays survive save/reopen when a
  historical project contains retained `(undef)` statements. Those original
  statements remain available as historical evidence.

The reader corrections apply through the shared Rust core to Python,
TypeScript/WebAssembly and GUI import. Public signatures, energy-axis conventions,
numerical defaults and the format-1 project schema are unchanged. New regressions
use synthetic inputs; original beamline files and their attribution are preserved.

## Release channels and compatibility

Nightly desktop builds follow `dev`. Reviewed stable releases are promoted from
`dev` to `main`; nightly builds do not publish registry packages. This release
also incorporates the reviewed `encoding_rs` 0.8.41 dependency update.

The coordinated packages retain four ABI3 wheels for GIL-enabled CPython
3.10–3.14, the Rust crate, and the npm/WebAssembly package. Desktop qualification
targets macOS ARM64/Intel, Linux x64/ARM64 and Windows x64/ARM64. Windows and Linux
remain previews. Windows ARM64 runs the x64 FEFF10 helper through Windows 11
emulation; rexafs and ReFEFF run natively.
