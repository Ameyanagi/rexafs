| Desktop | Installer | Portable |
|---|---|---|
| macOS · Apple Silicon | [Download](https://github.com/Ameyanagi/rexafs/releases/download/v0.2.1/rexafs-0.2.1-aarch64-apple-darwin.dmg) | [Archive](https://github.com/Ameyanagi/rexafs/releases/download/v0.2.1/rexafs-0.2.1-aarch64-apple-darwin.zip) |
| macOS · Intel | [Download](https://github.com/Ameyanagi/rexafs/releases/download/v0.2.1/rexafs-0.2.1-x86_64-apple-darwin.dmg) | [Archive](https://github.com/Ameyanagi/rexafs/releases/download/v0.2.1/rexafs-0.2.1-x86_64-apple-darwin.zip) |
| Windows · preview | [Download](https://github.com/Ameyanagi/rexafs/releases/download/v0.2.1/rexafs-0.2.1-x86_64-pc-windows-msvc-setup.exe) | [Archive](https://github.com/Ameyanagi/rexafs/releases/download/v0.2.1/rexafs-0.2.1-x86_64-pc-windows-msvc.zip) |
| Linux · preview | — | [Archive](https://github.com/Ameyanagi/rexafs/releases/download/v0.2.1/rexafs-0.2.1-x86_64-unknown-linux-gnu.tar.gz) |

[Installation and offline setup](https://github.com/Ameyanagi/rexafs/blob/v0.2.1/doc/installing.md)

<details>
<summary>Python · Rust · npm</summary>

| Package | Install |
|---|---|
| [Python](https://pypi.org/project/rexafs/0.2.1/) | `python -m pip install --upgrade rexafs` |
| [Rust](https://crates.io/crates/rexafs/0.2.1) | `cargo add rexafs@0.2.1` |
| [npm](https://www.npmjs.com/package/rexafs/v/0.2.1) | `npm install rexafs@0.2.1` |

</details>

<details>
<summary>What changed</summary>

The desktop opens in a complete, empty Data workspace. Help, theme switching,
common processing controls, contextual Fit steps, and the publication preview
are easier to reach. Assistant and structure fields follow the selected theme.

Import requires an explicit main spectrum and optional channels. Selecting
Fluorescence and Reference creates those groups without implicitly adding
Transmission. Remove marked captures the intended groups and supports Undo.
Processing tools preview their results before application.

Auto normalization maximum follows the measured spectrum endpoint. An optional
AUTOBK weight link follows FFT while preserving the independent background
weight for later unlinking. Palette cycles and gradients, including reversal,
remain assigned to group identities across selection, save/reopen, and Undo.

Publish offers flattened or normalized XANES, full-energy spectra, weighted
chi(k), chi(R), and R-space fits. Defaults include 300 DPI, a legend, grid, and
Typst labels; exported CSV files retain the full numerical arrays. The update to
ruviz and ruviz-gpui 0.14.1 improves font consistency and international fallback,
keeps angstrom labels complete, and removes sharp spikes at noisy PNG joins.

The structure viewer adds center focus and camera-relative depth cues while
preserving calculation geometry and the established bond appearance. Reduced
paint overhead improves interaction. The measured CPU draw/submission costs and
their limits are recorded in the
[renderer review](https://github.com/Ameyanagi/rexafs/blob/v0.2.1/doc/validation/2026-09-09-structure-performance/review.md);
these measurements do not establish a physical-display frame rate.

Existing format-1 projects remain readable. New linked and embedded samples
retain the weight link, reversed color assignments, publication settings, and
earlier import/Assistant state. Save the project before updating and retain its
backup when moving between application versions.

</details>

<details>
<summary>Platform scope</summary>

Mac releases use signed and notarized ZIP archives and DMG installers. Intel
interactive checks on the available Apple Silicon host use Rosetta; native Intel
hardware and clean-machine installation remain unqualified. Windows/Linux
remain previews pending native interactive qualification. Windows includes
ReFEFF; macOS and Linux include ReFEFF and FEFF10.

Series/frame workflows and the deferred Assistant features remain outside this
release. The prior
[implementation review](https://github.com/Ameyanagi/rexafs/blob/v0.2.1/doc/validation/2026-09-08-ui-simplification/review.md)
records the broader UI coverage and remaining interaction checks.

</details>
