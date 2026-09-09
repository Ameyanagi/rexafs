| Desktop | Installer | Portable |
|---|---|---|
| macOS · Apple Silicon | [Download](https://github.com/Ameyanagi/rexafs/releases/download/v0.2.4/rexafs-0.2.4-aarch64-apple-darwin.dmg) | [Archive](https://github.com/Ameyanagi/rexafs/releases/download/v0.2.4/rexafs-0.2.4-aarch64-apple-darwin.zip) |
| macOS · Intel | [Download](https://github.com/Ameyanagi/rexafs/releases/download/v0.2.4/rexafs-0.2.4-x86_64-apple-darwin.dmg) | [Archive](https://github.com/Ameyanagi/rexafs/releases/download/v0.2.4/rexafs-0.2.4-x86_64-apple-darwin.zip) |
| Windows · preview | [Download](https://github.com/Ameyanagi/rexafs/releases/download/v0.2.4/rexafs-0.2.4-x86_64-pc-windows-msvc-setup.exe) | [Archive](https://github.com/Ameyanagi/rexafs/releases/download/v0.2.4/rexafs-0.2.4-x86_64-pc-windows-msvc.zip) |
| Linux · preview | — | [Archive](https://github.com/Ameyanagi/rexafs/releases/download/v0.2.4/rexafs-0.2.4-x86_64-unknown-linux-gnu.tar.gz) |

[Installation and offline setup](https://github.com/Ameyanagi/rexafs/blob/v0.2.4/doc/installing.md)

<details>
<summary>Python · Rust · npm</summary>

| Package | Install |
|---|---|
| [Python](https://pypi.org/project/rexafs/0.2.4/) | `python -m pip install --upgrade rexafs` |
| [Rust](https://crates.io/crates/rexafs/0.2.4) | `cargo add rexafs@0.2.4` |
| [npm](https://www.npmjs.com/package/rexafs/v/0.2.4) | `npm install rexafs@0.2.4` |

</details>

<details>
<summary>What changed</summary>

The desktop opens within the available display area, including small Linux
screens. The empty workspace offers Import spectra, Open project and Open Cu
example actions. Numeric fields use each platform's monospace font, clip long
values inside the field, and scroll to keep the editing caret visible. Clicking
and IME positioning follow the scrolled text. Stage shortcuts return focus to the
workspace before switching, so they keep working after a focused plot control
disappears.

Folder scanning now bounds queued batches and stops cancelled scans while walking
unrelated files, limiting memory growth when the UI is busy. Windows tests and
benchmarks no longer pull in the Unix-only profiler.

Linux release builds exercise the packaged application under X11 with Mesa
software rendering: open an embedded project from a Unicode path, render plots,
switch stages, resize and quit. Windows runs the core tests and all-target compile
check in addition to the desktop and installer checks. Repository pre-commit and
pre-push hooks cover formatting, compatibility fixtures, release tooling and core
quality checks.

Existing format-1 projects remain readable. The 0.2.4 linked and embedded fixtures
are saved and reopened through this version's writer. Numerical algorithms,
scientific defaults and dependency versions are unchanged from 0.2.3.

</details>

<details>
<summary>Platform scope</summary>

Mac downloads use signed and notarized ZIP archives and DMG installers. Windows
and Linux remain desktop previews. Linux interaction checks use X11 and Mesa
software rendering; physical GPUs, native Wayland and clean-machine graphical
installation remain unqualified. Windows CI checks the native core, desktop,
packaging and installer; the retained native Windows interaction report predates
these UI changes. macOS and Linux include ReFEFF and FEFF10; Windows includes
ReFEFF.

The [Linux interaction report](https://github.com/Ameyanagi/rexafs/blob/v0.2.4/doc/validation/2026-09-09-linux-desktop/README.md)
records the available-host checks and screenshots. The
[release qualification record](https://github.com/Ameyanagi/rexafs/blob/main/doc/validation/2026-09-09-release-0.2.4/review.md)
tracks the exact build, signing and publication evidence.

[PR #39](https://github.com/Ameyanagi/rexafs/pull/39) remains excluded because its
nalgebra update is incompatible with the current solver. The legacy clamp
Jacobian and output-FFT window-domain differences tracked by
[issue #20](https://github.com/Ameyanagi/rexafs/issues/20) remain open.

</details>
