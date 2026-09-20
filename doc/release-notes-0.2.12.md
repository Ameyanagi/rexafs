# rexafs 0.2.12 — release preparation

This patch is being qualified and is not yet published. The public downloads
remain on 0.2.11 until the exact-tag builds, signing and package checks complete.

## Platform support

Starting with 0.2.12, macOS desktop releases and Python wheels require **Apple
Silicon (ARM64)**. Intel Macs are no longer supported by the desktop, Python
release matrix or Nightly channel. The
[0.2.11 release](https://github.com/Ameyanagi/rexafs/releases/tag/v0.2.11)
remains the last stable release with Intel Mac downloads; its existing desktop
assets and Python wheels are preserved. Source builds on Intel Macs are not
qualified by the new release matrix.

The current matrix has five desktop targets: Apple Silicon macOS, Windows
x64/ARM64 and Linux x64/ARM64. Python ships three ABI3 wheels for Apple Silicon
macOS, Linux x64 and Windows x64, each tested on CPython 3.10–3.14. Windows and
Linux desktop packages remain previews. The Rust source and npm/WebAssembly
package channels are unchanged.

## In-app updates

On macOS, the updater now stages its helper as a complete signed app bundle.
The previous bare-executable copy lost its signed Info.plist context and was
killed before replacement. The corrected helper keeps its signature and resources
together, while retaining incoming-app verification, recovery and rollback.

On macOS, Windows and Linux, **Update and restart** remains available after using
the separate **Download** action. Unsupported installation layouts show the
reason. Updating another release channel does not replace the current app.

Existing affected Mac installations, including 0.2.10 and 0.2.11, need the first
corrected app installed once because their old helper cannot start. The repaired
app can perform subsequent in-app updates. The new signing qualification checks
helper startup in both signed ZIP and installed DMG bundles. Windows and Linux
keep their platform-specific helpers and remain desktop previews.

## RMC resources, search and structural history

New desktop runs choose scattering-cache memory from available physical memory
and reassess it during calculation. **Cache memory (MiB)** accepts a fixed limit
or **Auto**. Preparation reports its current stage; **Run details** separates
cold misses from repeated misses and warns when retained snapshots do not fit.
These limits cover cached snapshots, not total application memory.

**Catalogue path limit** is a separate Auto/manual control. Auto captures a
memory-based capacity when a new run starts, replacing the desktop's fixed
one-million-path guard. The wrapped error offers **Path limit…** when capacity
is insufficient. Increasing capacity retains the same calculation; reducing path
radius, scattering order or absorbing sites changes the model. Old checkpoints
retain their original guard and scientific inputs.

**Results → Structural evolution** shows element-pair initial/current/best
distributions, distance-versus-step heatmaps, coordination and distance moments.
Periodic cells use shell-volume and species-density normalization for g(r);
finite clusters show neighbor counts. Sampling runs in a bounded background
worker, with gaps and retained-history limits reported explicitly. Checkpoints
and JSON/CSV exports retain the sampled history. Optimization steps are not
physical time, and these histories are not uncertainty estimates.

**Search method** now exposes Genetic / EA and Hybrid EA–RMC alongside ordinary
RMC. Both population methods require fixed ΔE₀ and retain their full population
and random state for continuation. Larger populations require additional
scattering work. See the [resource and structural guide](rmc-structural-evolution.md)
for defaults, normalization, resource limits and implementation references.

## Desktop workflow

- **Publish report…** writes the analysis folder and offers actions to open it.
  The header tracks changes since publication. Three compact preset icons show
  their names and dimensions on hover; **Apply style to all figures** preserves
  each figure's limits, labels, captions and curve visibility.
- Fit controls distinguish **Method** from **Spectra**, explain blocked actions
  and identify stale results. RMC result actions require a saved calculation.
- Automatic numeric values and validation messages remain readable. Invalid
  edits preserve the last committed value and offer recovery actions.
- Series uses one-based frame labels, direct frame entry and previous/next
  controls; plots stack in a scrollable area in smaller workspaces.
- Theme selection persists, wavelet views refresh on theme changes, command
  search ranks title matches first, and MBACK suggests absorber/edge from E₀
  when the source does not declare them.

## Documentation and compatibility

The manual explains the new RMC controls and includes unedited computer-use
captures identified by their actual build. Earlier signed 0.2.11 screenshots,
public data attribution and historical records remain available. Synthetic
structural examples demonstrate the controls, not a refined experimental model.

The exact-scattering model and existing saved scientific settings are preserved;
new resource policies, structural analysis and desktop search choices are
documented explicitly. Project format remains 1. Rust, Python,
npm/WebAssembly and desktop package versions are
coordinated. Qualification uses new linked and embedded projects written through
the 0.2.12 writer; older fixtures retain their original bytes.

The [qualification record](validation/2026-09-19-release-0.2.12/review.md)
distinguishes completed checks from pending release gates.
