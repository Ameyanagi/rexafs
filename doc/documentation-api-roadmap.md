# Documentation, APIs and platform priorities

Recommendations from the 13 September 2026 review. Completed documentation,
browser workspace and release-pipeline changes are marked below; other items
are proposed work.
Published packages remain at 0.2.4. See the [WASM build assessment](webassembly.md)
for tested capabilities and failures.

## Documentation

**Completed in this pass:** reduced repeated homepage/download copy, grouped
downloads by operating system, made missing Windows/Linux ARM64 packages
visible, and shortened manual introductions and repeated screenshot captions.
Scientific derivations, practical steps, API contracts, attribution and original
screenshots remain. Corrected the FAQ's startup-network claim, stored automatic
settings explanation, Windows FEFF10 release distinction and JavaScript cleanup
examples. Existing generated API help remains source-owned.

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
processing and exports. Next-release packaging now includes native Windows/Linux
ARM64 CI jobs and architecture checks; actual native qualification remains a CI
release gate. These changes do not create downloads for published 0.2.4.

## Python and TypeScript

Implement in this order; keep the shared `Spectrum` workflow.

| Priority | Change | Acceptance criteria |
|---|---|---|
| 1 | Release existing Next ergonomics | Ship keyword/options constructors, direct configuration setters, inverse settings and source-owned editor help together. Verify installed wheels/tarballs; move only released signatures into Stable. |
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

0.2.4 publishes macOS ARM64, macOS x64, Windows x64 and Linux x64 desktops.
The [release workflow](../.github/workflows/release-build.yml) now includes
Windows/Linux ARM64 desktops for the next release. Their native Python wheels
remain separate work. The npm WASM package
does not require a separate CPU-specific binary.

Qualify Linux ARM64 first using native hardware/runners, then Windows ARM64.
Both need packaged-app launch, rendering, import, processing, fitting, save/reopen
and export checks. Audit engine artifacts as part of each target: a ReFEFF-only
source build does not establish that the full FEFF10 package works there.
The Windows pipeline now checks installer and runtime architecture and bundles
the x64 FEFF10 helper under Windows 11 emulation. Qualify that helper separately
from native ARM64 processing. Verify updater selection on both architectures;
add download links only after matching binaries and checksums exist.

Coordinate ReFEFF portability with its upstream embedding work. Preserve the
working native Python and desktop packages while extending WASM; the
[porting sequence](webassembly.md#recommended-porting-sequence) separates
compilation, useful bindings and numerical/runtime qualification.
