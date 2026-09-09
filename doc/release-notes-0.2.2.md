| Desktop | Installer | Portable |
|---|---|---|
| macOS · Apple Silicon | [Download](https://github.com/Ameyanagi/rexafs/releases/download/v0.2.2/rexafs-0.2.2-aarch64-apple-darwin.dmg) | [Archive](https://github.com/Ameyanagi/rexafs/releases/download/v0.2.2/rexafs-0.2.2-aarch64-apple-darwin.zip) |
| macOS · Intel | [Download](https://github.com/Ameyanagi/rexafs/releases/download/v0.2.2/rexafs-0.2.2-x86_64-apple-darwin.dmg) | [Archive](https://github.com/Ameyanagi/rexafs/releases/download/v0.2.2/rexafs-0.2.2-x86_64-apple-darwin.zip) |
| Windows · preview | [Download](https://github.com/Ameyanagi/rexafs/releases/download/v0.2.2/rexafs-0.2.2-x86_64-pc-windows-msvc-setup.exe) | [Archive](https://github.com/Ameyanagi/rexafs/releases/download/v0.2.2/rexafs-0.2.2-x86_64-pc-windows-msvc.zip) |
| Linux · preview | — | [Archive](https://github.com/Ameyanagi/rexafs/releases/download/v0.2.2/rexafs-0.2.2-x86_64-unknown-linux-gnu.tar.gz) |

[Installation and offline setup](https://github.com/Ameyanagi/rexafs/blob/v0.2.2/doc/installing.md)

<details>
<summary>Python · Rust · npm</summary>

| Package | Install |
|---|---|
| [Python](https://pypi.org/project/rexafs/0.2.2/) | `python -m pip install --upgrade rexafs` |
| [Rust](https://crates.io/crates/rexafs/0.2.2) | `cargo add rexafs@0.2.2` |
| [npm](https://www.npmjs.com/package/rexafs/v/0.2.2) | `npm install rexafs@0.2.2` |

</details>

<details>
<summary>What changed</summary>

Windows opens the application without an extra console window. Ctrl shortcuts
now work for Undo, Open, Save, stage navigation, clipboard actions, and group or
atom multi-selection. Text fields use Ctrl+Arrow for word navigation on
Windows/Linux and retain the standard macOS gestures. macOS continues to use Cmd
for command shortcuts.

Accessibility updates compare nodes by identity, share the complete activation
snapshot, and skip unchanged native updates. This reduces work during plot and
pointer redraws while preserving changes to controls, focus, and tree structure.

Windows packaging checks the executable's GUI subsystem. Installer checks wait
for diagnostic processes and capture their output. Opt-in debug statistics also
report the selected graphics adapter. The
[Windows interaction review](https://github.com/Ameyanagi/rexafs/blob/v0.2.2/doc/validation/2026-09-09-windows-gui/README.md)
records development-build checks and the limits of the measured timings.

Existing format-1 projects remain readable. New linked and embedded fixtures
preserve group identities, imports and recipes, locks, weight links, palettes,
publication style, and saved Assistant conversations. Processing defaults and
numerical dependencies remain the same as 0.2.1.

</details>

<details>
<summary>Platform scope and reviewed follow-ups</summary>

Mac releases use signed and notarized ZIP archives and DMG installers. Intel
interactive checks on the available Apple Silicon host use Rosetta; native Intel
hardware and clean-machine installation remain unqualified. Windows/Linux
remain desktop previews. Native Windows development-build checks are available;
interactive qualification of the final Windows/Linux downloads remains pending.
Windows includes ReFEFF; macOS and Linux include ReFEFF and FEFF10.

[PR #39](https://github.com/Ameyanagi/rexafs/pull/39) is excluded: updating nalgebra
to 0.35 alone fails compilation against the current solver's 0.34 matrix types.
The release retains the compatible dependency versions.

[Issue #20](https://github.com/Ameyanagi/rexafs/issues/20) remains open for the
legacy dynamic-clamp Jacobian and output-FFT window-domain differences with
XrayLarch. The default fixed-penalty solver improvements from earlier releases
remain in place; this patch does not change those numerical algorithms.

</details>
