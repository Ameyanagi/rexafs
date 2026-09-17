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

## Follow-up from the first PR checks

The local results above did not predict two failures on PR #88. The core CI
matrix rejected the Ru auxiliary post-edge polynomial: its maximum absolute
reference difference was 1.783×10⁻⁶ on Linux and 3.517×10⁻⁶ on Windows, while
the atomic-matching background remained within 6.78×10⁻¹⁰. Only that auxiliary
curve now uses a separate 5×10⁻⁶ absolute tolerance in f₂ units. The
[reference record](../../../crates/rexafs/tests/fixtures/analysis/experimental-larch/README.md#cross-platform-tolerance-revision--18-september-2026)
retains the initial measurements and the rationale. No numerical implementation
or oracle array was changed. All six experimental reference tests pass locally.

The website check also exposed that manually restoring Stable pages was not
sufficient: the generators intentionally use maintained source help for shared
members. Shared Python/TypeScript help now describes the historical empty MBACK
selector consistently with both versions and explicitly records that MBACK was
unimplemented through 0.2.9. Configured MBACK remains documented in Next.
Regenerated reference pages and citations reproduce byte-for-byte, all eight
generator tests pass, and all 37 Astro files have no diagnostics. CI must rerun
these corrections on the supported platforms before its status is considered
green.

The subsequent method-specific fit overlay passed all 12 plotting tests and the
complete desktop suite (609 passed, 6 ignored). Native macOS controls verified
the full MBACK curve, on/off behavior and replacement by polynomial baselines;
the release build and [new screenshot](../2026-09-18-mback-normalize/mback-fit-toggle.jpg)
show this final UI. This overlay uses the existing core fit arrays and does not
change the numerical algorithm.
