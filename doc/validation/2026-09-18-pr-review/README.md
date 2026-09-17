# Analysis B–F review before the dev pull request

Reviewed on 18 September 2026 in `feature/analysis-b-f`, including its 19 commits
ahead of `dev` and the subsequent local refinements. The PR contains all requested
B–F development work, not only the final alignment and normalization UI changes.

## Issues corrected during review

- Added checked absolute energy-offset operations to the core and routed desktop
  axis adjustment through them. Replacing arrays resets the recorded offset;
  zeroing an offset does not undo independent data edits.
- A full GUI regression caught rejection of repeated energy points during merge.
  Existing repeated samples now survive both zero and nonzero offsets; shifts
  that collapse distinct samples still fail before mutation.
- Restored released Stable reference pages after unreleased MBACK descriptions
  had leaked into them. New API documentation remains in Next.
- Updated the Windows archive test's isolated repository fixture for the newly
  bundled atomic-data notices. Both architecture cases now verify that those
  notice bytes survive packaging.
- Moved the MBACK method editor into the Normalize sidebar. Group changes and
  Undo/Redo replace the editor state and invalidate stale calculations; atomic
  matching and method comparison remain optional views in the same tab.

Review also checked immutable source/result retention, calculation generation
checks, Live snapshot limits and transaction ordering, experimental-fixture
provenance, bounded COD requests, wavelet region validation, and immutable plot
exports. No additional blocking finding remains in these inspected paths.

## Checks completed

| Check | Result |
| --- | --- |
| Core with desktop-enabled features: 32 unit/integration suites | 481 passed, 6 ignored. |
| Final energy-offset regressions, including experimental Ru and repeated samples | 3 passed. |
| Final complete desktop suite | 608 passed, 6 ignored. |
| Rust documentation examples | 9 passed. |
| Core strict Clippy, all targets | Passed. |
| Release desktop build | Passed; 12 existing dead-code warnings. |
| Installed Python release wheel | 38 tests and 41 subtests passed. |
| Installed Python editor completion/signature/hover checks | Passed. |
| Built browser/Node Wasm and npm runtime/editor/package checks | 38 passed. |
| Astro documentation checks | 37 files, no errors, warnings or hints. |
| Documentation generator regressions | 8 passed. |
| Release/package tooling | All 12 suites passed. |
| Release-version and project-compatibility fixtures | Passed; version 0.2.9 and 40 retained project samples. |
| Beamline fixture integrity and Cargo package contents | Passed; attributed fixtures and their integration targets excluded. |
| Rust formatting and patch whitespace | Passed. |

The combined core/desktop command finished all 32 core suites. Its pending
redundant desktop invocation, compiled before the final fixes, was stopped;
the separate final desktop command supplied the 608-test result above. Core
offset regressions, doctests and Clippy were rerun after the repeated-point fix.
The binding APIs were unchanged by that final core offset adjustment; these new
Rust offset methods are not yet exposed in Python or TypeScript.

## Evidence and remaining qualification

The [MBACK desktop record](../2026-09-18-mback-normalize/README.md) includes fresh
native screenshots. The [alignment/COD record](../2026-09-18-cod-alignment/README.md)
and other dated records preserve the earlier checks. Only public attributed
measurements or explicitly labeled synthetic workflow demonstrations appear in
published screenshots. Private unpublished measurements remain outside this PR.

Experimental Cu/Ru MBACK references and matched-input complex wavelet references
are retained with licenses and checksums, and excluded from published packages.
Wavelet agreement is qualified with identical prepared χ arrays; it does not
establish end-to-end AUTOBK equivalence or physical accuracy.

This local review does not qualify native Windows/Linux interaction, sustained
Live acquisition on network storage, or release installers. CI and platform
qualification remain separate. LCF/PCA/MCR bindings and the new energy-offset
bindings remain tracked follow-up work. The PR does not change the release
version or publish packages.
