| Platform | Installer | Portable archive |
|---|---|---|
| macOS · Apple Silicon | [DMG](https://github.com/Ameyanagi/rexafs/releases/download/v0.2.11/rexafs-0.2.11-aarch64-apple-darwin.dmg) | [ZIP](https://github.com/Ameyanagi/rexafs/releases/download/v0.2.11/rexafs-0.2.11-aarch64-apple-darwin.zip) |
| macOS · Intel | [DMG](https://github.com/Ameyanagi/rexafs/releases/download/v0.2.11/rexafs-0.2.11-x86_64-apple-darwin.dmg) | [ZIP](https://github.com/Ameyanagi/rexafs/releases/download/v0.2.11/rexafs-0.2.11-x86_64-apple-darwin.zip) |
| Windows · x64 preview | [Setup](https://github.com/Ameyanagi/rexafs/releases/download/v0.2.11/rexafs-0.2.11-x86_64-pc-windows-msvc-setup.exe) | [ZIP](https://github.com/Ameyanagi/rexafs/releases/download/v0.2.11/rexafs-0.2.11-x86_64-pc-windows-msvc.zip) |
| Windows · ARM64 preview | [Setup](https://github.com/Ameyanagi/rexafs/releases/download/v0.2.11/rexafs-0.2.11-aarch64-pc-windows-msvc-setup.exe) | [ZIP](https://github.com/Ameyanagi/rexafs/releases/download/v0.2.11/rexafs-0.2.11-aarch64-pc-windows-msvc.zip) |
| Linux · x64 preview | — | [Archive](https://github.com/Ameyanagi/rexafs/releases/download/v0.2.11/rexafs-0.2.11-x86_64-unknown-linux-gnu.tar.gz) |
| Linux · ARM64 preview | — | [Archive](https://github.com/Ameyanagi/rexafs/releases/download/v0.2.11/rexafs-0.2.11-aarch64-unknown-linux-gnu.tar.gz) |

Released on September 19, 2026. macOS downloads are signed and notarized.
The [qualification record](validation/2026-09-19-release-0.2.11/review.md)
records the source, build, signing and public-artifact checks.

<details>
<summary>RMC fitting improvements</summary>

RMC now copies the selected spectrum’s forward-transform settings and explicit
back-transform fit ranges when a job is initialized or **Use spectrum ranges**
is selected. ReFEFF wave-number coverage expands for the Fourier taper and
configured theoretical ΔE₀ bounds, up to the adapter’s 30 Å⁻¹ limit. Missing
theory is rejected rather than extrapolated.

Larger cells receive a prepared context for every requested absorbing site.
Identical complete electronic inputs can share immutable preparation, while each
site retains its own atom identities and path catalogue. Numerical cache and
path-search resource limits remain in force.

**CPU workers** uses available logical CPUs, capped at 64, for new desktop jobs.
Users can enter an explicit budget. Absorbers run concurrently first; **Parallel
paths** uses spare workers within an absorber. Both levels share one bounded
thread pool. Floating-point sums retain a fixed order. Older jobs retain their
captured settings on resume. More workers may increase temporary memory and are
not guaranteed to improve every calculation.

**Estimate calibration** previews theoretical energy and amplitude adjustments;
**Use calibration** applies them explicitly. New desktop jobs offer automatic
coordinate moves, starting at 0.05 Å with bounded feedback and cooling. Optional
local numerical refinement can improve the best structure after exploration.

Selecting **ΔE₀ → Refine** optimizes the theoretical energy shift before the first
move and every 250 attempts by default, with S₀² fixed at the supplied value.
Bounds and update cadence are configurable. The experimental energy axis and
normalization remain unchanged. Each dataset retains its own matching shift,
coordinates and calculated spectrum in checkpoints and exports.

These are optimization controls, not a guarantee of physical convergence. Use an
appropriate amplitude calibration, physical constraints and independent runs;
energy shifts can correlate with bond distances. The default update interval is
a starting heuristic. The [RMC guide](rmc.md) explains units and assumptions.

</details>

<details>
<summary>Packages and compatibility</summary>

The coordinated version covers Rust, Python, npm/WebAssembly and desktop builds.
FEFF10 0.2.4 initializes native array-format labels, correcting intermittent
`gg.bin` parsing failures during scattering calculations and package checks.
Windows bundles the matching helper; package self-checks validate the generated
headers on every desktop target.
The new RMC APIs are available in Rust and the native desktop; Python and
TypeScript do not yet expose RMC. Core worker defaults remain one worker with
path fallback disabled. Automatic CPU allocation is a new desktop-job default.

Project format 1 is retained. Historical projects and fixed-energy checkpoints
keep their previous behavior; changing form settings starts a new job rather
than changing a saved run. New linked and embedded compatibility fixtures are
written through this release’s writer, with earlier fixture bytes preserved.

Install packages using `cargo add rexafs@0.2.11`, `uv add rexafs==0.2.11`, or
`npm install rexafs@0.2.11`. See [installation instructions](installing.md).
Windows and Linux remain desktop previews; Windows ARM64 runs its FEFF10 helper
through Windows 11 x64 emulation. Adaptive RMC remains experimental and opt-in.

</details>

<details>
<summary>Qualification</summary>

The dev → main promotion passed all 67 checks. All 37 jobs in the
[immutable-tag build](https://github.com/Ameyanagi/rexafs/actions/runs/35415279391)
succeeded. Both Mac architectures passed signing, notarization and installation
checks. All seven public registry packages match the qualified build; all 27
desktop asset digests match the final signed manifest.

A computer-use check of the signed Apple Silicon app verified inherited fit
ranges, two CPU workers, periodic ΔE₀ updates with fixed S₀², and saved-result
reload using public Cu data. This ten-attempt software check is not a converged
scientific fit.
No unpublished experimental data, derived research results or private timing
artifacts are included in the release source.

</details>
