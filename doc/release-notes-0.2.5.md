# rexafs 0.2.5

Release preparation: publication and signing remain pending. The qualification
record will identify the successful tagged build and published artifact hashes.

| Desktop | Installer | Portable |
|---|---|---|
| macOS · Apple Silicon | [Download](https://github.com/Ameyanagi/rexafs/releases/download/v0.2.5/rexafs-0.2.5-aarch64-apple-darwin.dmg) | [Archive](https://github.com/Ameyanagi/rexafs/releases/download/v0.2.5/rexafs-0.2.5-aarch64-apple-darwin.zip) |
| macOS · Intel | [Download](https://github.com/Ameyanagi/rexafs/releases/download/v0.2.5/rexafs-0.2.5-x86_64-apple-darwin.dmg) | [Archive](https://github.com/Ameyanagi/rexafs/releases/download/v0.2.5/rexafs-0.2.5-x86_64-apple-darwin.zip) |
| Windows · x64 preview | [Download](https://github.com/Ameyanagi/rexafs/releases/download/v0.2.5/rexafs-0.2.5-x86_64-pc-windows-msvc-setup.exe) | [Archive](https://github.com/Ameyanagi/rexafs/releases/download/v0.2.5/rexafs-0.2.5-x86_64-pc-windows-msvc.zip) |
| Windows · ARM64 preview | [Download](https://github.com/Ameyanagi/rexafs/releases/download/v0.2.5/rexafs-0.2.5-aarch64-pc-windows-msvc-setup.exe) | [Archive](https://github.com/Ameyanagi/rexafs/releases/download/v0.2.5/rexafs-0.2.5-aarch64-pc-windows-msvc.zip) |
| Linux · x64 preview | — | [Archive](https://github.com/Ameyanagi/rexafs/releases/download/v0.2.5/rexafs-0.2.5-x86_64-unknown-linux-gnu.tar.gz) |
| Linux · ARM64 preview | — | [Archive](https://github.com/Ameyanagi/rexafs/releases/download/v0.2.5/rexafs-0.2.5-aarch64-unknown-linux-gnu.tar.gz) |

[Installation and offline setup](https://github.com/Ameyanagi/rexafs/blob/v0.2.5/doc/installing.md)

<details>
<summary>Python · Rust · npm</summary>

| Package | Install |
|---|---|
| [Python](https://pypi.org/project/rexafs/0.2.5/) | `uv add rexafs==0.2.5` |
| [Rust](https://crates.io/crates/rexafs/0.2.5) | `cargo add rexafs@0.2.5` |
| [npm](https://www.npmjs.com/package/rexafs/v/0.2.5) | `bun add rexafs@0.2.5` |

Python supports CPython 3.10–3.14; Node requires 22 or newer. The desktop ARM64
additions do not add new Python wheel targets. The npm package includes browser
WebAssembly and Node entry points.

</details>

<details>
<summary>What changed</summary>

- **More desktop targets.** Linux and Windows gain ARM64 packages. Windows ZIPs
  include the matching Microsoft runtime, and installers verify their payloads.
- **FEFF10 on Windows.** Both Windows architectures include the verified x64
  FEFF10 helper; Windows ARM64 runs it through Windows 11 emulation. rexafs and
  ReFEFF run natively on ARM64.
- **Simpler processing APIs.** Python gains keyword constructors and TypeScript
  gains options constructors. Both accept background and normalization settings
  directly and expose inverse-transform settings through `XrayFFTR` and
  `Spectrum.set_ifft()`. Existing construction/setter forms remain available.
- **Browser processing preview.** Import text/CSV, select columns and energy units,
  process a spectrum locally, inspect plots and export CSV plus a provenance
  record. Work runs in a cancellable Worker with input and numerical limits.
  The [website preview](https://rexafs.com/app/) follows the website's source
  deployment; it is separate from the versioned npm package. Scattering engines
  and the full desktop workflow are outside its current scope.
- **Clearer documentation.** Shorter product copy, explicit supported formats,
  source-owned API help, corrected scientific explanations and separate Stable
  and source-checkout references. Original screenshots retain their capture
  versions. Rust Fourier plot-axis units now include the integration measure;
  numerical amplitudes are unchanged.

The numerical defaults and format-1 project format are unchanged. New linked and
embedded compatibility fixtures are saved through the 0.2.5 writer; historical
fixtures remain intact. ReFEFF WASM support is being handled upstream.

</details>

<details>
<summary>Platform scope and qualification</summary>

Mac downloads require the signed and notarized ZIP/DMG outputs from the qualified
build. Windows and Linux remain desktop previews. Windows ARM64 requires Windows
11 for the x64 FEFF10 helper. Linux uses the Ubuntu 24.04 runtime baseline and
requires a graphical session with a Vulkan-capable driver.

Release CI checks six desktop architectures, both Windows installers, Linux X11
rendering with Mesa software drivers, 20 Python wheels, the Python source archive,
Rust and npm packages, numerical regressions, licenses and artifact checksums.
These checks do not qualify every physical GPU, native Wayland session or clean
installation environment. The [qualification record](https://github.com/Ameyanagi/rexafs/blob/main/doc/validation/2026-09-13-release-0.2.5/review.md)
records actual results and remaining platform limits.

</details>
