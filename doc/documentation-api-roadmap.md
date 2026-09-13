# Documentation, APIs and platform priorities

Recommendations from the 13 September 2026 review. Completed documentation,
browser workspace and release-pipeline changes are marked below; other items
are proposed work.
Publication update: [0.2.5](release-notes-0.2.5.md) ships the API ergonomics and
ARM64 packages described below. See its
[qualification record](validation/2026-09-13-release-0.2.5/review.md) for release
evidence and the [WASM build assessment](webassembly.md) for tested capabilities
and failures.

## Documentation

**Completed in this pass:** reduced repeated homepage/download copy, grouped
downloads by operating system, made missing Windows/Linux ARM64 packages
visible, and shortened manual introductions and repeated screenshot captions.
Scientific derivations, practical steps, API contracts, attribution and original
screenshots remain. Corrected the FAQ's startup-network claim, stored automatic
settings explanation, Windows FEFF10 release distinction and JavaScript cleanup
examples. Existing generated API help remains source-owned.

Lead product copy with open source, format interoperability, speed, a small
footprint and advanced analysis. Name concrete formats and capabilities instead
of adding long feature lists. Treat browser analysis as a developing preview;
avoid suggesting that it already includes the full desktop workflow. Keep
numerical speed and size claims tied to a measured version and environment.

Keep each page responsible for one task:

| Page | Keep | Link elsewhere |
|---|---|---|
| Homepage | What rexafs does; desktop, browser and library entry points; short visual tour | Complete feature tables, FAQ, installation |
| Downloads | Actual packages, architectures, support status, install commands | Feature marketing and complete setup tutorials |
| How-to guides | Steps, expected outputs, relevant errors and limitations | Repeated product descriptions and long theory derivations |
| Theory | Equations, symbols, units, assumptions, parameter effects, citations | General marketing |
| API reference | Source-generated signatures, defaults, ownership, errors, behavior | Repeated installation instructions |

Retain full original application screenshots. A caption should identify the
state being illustrated and its actual version; link capture provenance once
per walkthrough. Recapture when a documented workflow changes, not merely to
match a newer release number. Add a new image only when it resolves a question
the existing steps cannot answer. Historical validation screenshots remain
dated evidence.

The subsequent browser implementation adds `/app/` with local import, Worker
processing and exports. The 0.2.5 pipeline includes native Windows/Linux ARM64 CI
jobs and architecture checks. Native qualification remains a gate for every
release; historical 0.2.4 has no Windows/Linux ARM64 downloads.

## Python and TypeScript

Implement in this order; keep the shared `Spectrum` workflow.

| Priority | Change | Acceptance criteria |
|---|---|---|
| 1 · Completed in 0.2.5 | Release configuration ergonomics | Keyword/options constructors, direct configuration setters, inverse settings and source-owned editor help ship together, with installed wheel/tarball checks and matching Stable references. |
| 2 | Export reproducible settings/results | A serializable snapshot distinguishes requested settings, actual values used, version/backend and units. Capture AUTOBK's inferred `kmax`/`nknots` during calculation: serializing unset settings cannot recover them. Taking a snapshot must not mutate the spectrum. |
| 3 | Add shared text/byte input | Users provide text or bytes plus column roles/units; return validated arrays and import metadata. Keep file access as a host adapter. Document sorting, duplicate handling and intensity requirements consistently. |
| 4 | Add bounded batch processing | Return results/errors in input order, preserve successful spectra after another fails, and resolve automatic settings per input. Python and browser scheduling may differ; document worker limits and copying. |
| 5 | Bind tools and advanced analysis | Expose alignment/rebin/merge, then LCF/PCA and fitting of supplied FEFF paths. Match core results and result metadata before adding scattering-engine execution. |
| 6 | Extract a reusable Worker adapter | The browser workspace now runs processing in a cancellable Worker. Extend this into a documented package adapter with ownership, disposal, structured errors and progress. Terminating a Worker discards its state. |

Implementation starting points:
[binding contract](api.md), [Python declarations](../py-rexafs/python/rexafs/__init__.pyi),
[TypeScript declarations](../js-rexafs/types.d.ts),
[Group](../crates/rexafs/src/xafs/xasgroup.rs),
[analysis](../crates/rexafs/src/xafs/analysis/mod.rs) and
[fitting](../crates/rexafs/src/xafs/fitting/mod.rs).

## ARM64 distribution

0.2.5 publishes macOS, Windows and Linux desktops on both ARM64 and x64. The
[release workflow](../.github/workflows/release-build.yml) qualifies those six
targets. Windows/Linux ARM64 Python wheels remain separate work. The npm WASM package
does not require a separate CPU-specific binary.

Extend Linux ARM64 rendering checks to physical GPUs and native Wayland. Add
interactive Windows ARM64 checks for import, processing, fitting, save/reopen
and export. The [current qualification](validation/2026-09-13-release-0.2.5/review.md)
covers Linux X11/software rendering and native Windows installation and engine
execution. Continue qualifying Windows 11's x64 FEFF10 helper separately from
native ARM64 processing, and verify updater selection on both architectures.

## CI cost

[PR #61](https://github.com/Ameyanagi/rexafs/pull/61) skips unrelated native builds
for website and documentation changes, reuses compatible dependencies and the
pinned browser build tool, and cancels superseded PR runs. Manual releases
retain the full matrix and separate run groups. See the
[CI guide](ci.md) for selection rules and measured bottlenecks. Next, compare
complete-run durations and evaluate scheduling long desktop jobs earlier.
Rerun a PR's current revision: manually rerunning an obsolete revision can
replace the newer run in that PR's group.

The [ABI3 pilot](validation/2026-09-13-python-abi3/review.md) passed all 14 Python
API tests on CPython 3.10–3.14 using one macOS ARM64 wheel. The unreleased source
now builds four shared wheels and retains all 20 runtime combinations, each with
minimum and latest compatible NumPy. Native PR CI must qualify all four platforms
before merging; a future versioned release must qualify the new artifact
contract before publication. See the [release checks](releasing.md#github-is-the-release-build-authority).
Keep 0.2.5's released binaries and historical qualification unchanged.

## Browser scope

ReFEFF 0.4.0 now supplies the upstream WASI browser engine. The website loads it
separately from spectrum processing, with cancellation and downloadable outputs.
Next, broaden retained native/browser cases and expose fitting of supplied paths
through the rexafs bindings. The [WASM assessment](webassembly.md) separates
the current integration from historical compilation barriers.
