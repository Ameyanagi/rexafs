# Assistant context and analysis verification

This is an **unreleased source-build** record from 15 September 2026 on macOS
ARM64. The branch is `fix/assistant-context-size`, based on dev commit
`7bacc022d6c2587c5911509b7cf78fb8d753f856`. Cargo still reports 0.2.8; these
changes are absent from the immutable 0.2.8 release. This record is workflow
verification, not a claim of chemical accuracy or cross-platform GUI coverage.

## Scope and implementation

The reported failure sent a lossless project archive as Assistant prompt text.
A two-group Cu/QD and Larix QA project contained 3,474,202 compact JSON characters,
including original file bytes and a 310,308-character Larix header. The service
rejected input longer than 1,048,576 characters.

[`assistant_context.rs`](../../../crates/rexafs-gui/src/publication/assistant_context.rs)
now supplies a short overview and explicit retrieval of source headers, mappings,
processing, models and history. Source headers use 4096-character pages; group
lists use 50-group pages. Tool text is limited to 256 KiB of UTF-8 JSON and initial
turn text to 512 KiB. Oversized exact sections fail locally instead of silently
truncating model values. Plot images use their separate image transport. Project
saves and publication exports retain their lossless data.

[`assistant_analysis.rs`](../../../crates/rexafs-gui/src/app/shell/assistant_analysis.rs)
uses the existing Data LCF/PCA operations. It requires explicit group identities,
current processing and inspection of every requested operand. It reports scalar
results and resolved reference order. Result provenance detects changed inputs.
[`assistant_actions.rs`](../../../crates/rexafs-gui/src/app/shell/assistant_actions.rs)
configures the existing EXAFS model for independent or joint fits while retaining
paths, expressions and per-dataset settings. These changes do not alter the core
LCF, PCA or EXAFS algorithms; their assumptions remain documented in the
[analysis guide](../../../website/src/content/docs/docs/science/analysis.md).

## Computer-use checks

The app used a separate QA bundle, project copies and settings under
`/tmp/rexafs-assistant-context-qa`. The user authorized in-app AI calculations.
GPT-6-Astra with Medium reasoning ran in Edit analysis mode; workspace commands
remained off. The QA executable was an optimized release build, not the installed
stable application. Original user projects and account credentials were untouched.

| Check | Observed result |
|---|---|
| Large Cu/QD and Larix project | AI listed both groups and retrieved only the selected Cu header/mapping without the input-length error. |
| Independent Cu fits | Two completed fits and separate History entries; R factors 0.0013554187317813223 and 0.0013275945188147827. |
| Joint Cu fit | Two spectra; shared `s02`, `de0`; local `sigma2`, `deltar`; six variables; R factor 0.0013415850414304601. |
| Joint project Save | Embedded `.rxs` save succeeded with three fit records and unchanged raw arrays. Linked and embedded save/reopen regressions passed. |
| Synthetic LCF | Weights 0.20000910779088554, 0.2999970484999355, 0.4999938437091789; R factor 3.881469490473707e-11. |
| Synthetic PCA, two components | Target R factor 1.17505452e-4, read from the app tool report. |
| Synthetic PCA, three components | Target R factor 3.826245696784246e-11; first three components account for the squared signal to numerical precision. |
| Processing edit | AI changed only target `edge_step` from 1 to 1.01; recalculation completed; LCF and PCA reported stale. |
| Receipt Undo | Computer use clicked Undo; `edge_step` returned to 1; both results reported current again without refitting. |
| LCF/PCA export | Analysis folder completed with an empty notices list; original energy, absorption and explicitly supplied processing values were unchanged. |

The independent batch test used successive single-group fits through the
Assistant. It did not exercise the separate Series catalog batch panel. The
Cu tests used retained first-shell FEFF paths; no remote structure retrieval or
new FEFF calculation was requested.

The save check exposed an additional defect: an empty joint-dataset file locator
for an in-memory group resolved to the working directory. The storage fix keeps
the empty locator and resolves the group identity. The first attempted export was
incomplete; its already-written `state.json` retained the three fit records.
Recovery used that app-authored project snapshot, then the corrected GUI saved
`Cu fitting verified.rxs`. No fitted values were manually reconstructed.

## Inputs and numerical interpretation

The Cu input is the retained XrayLarch `xafsdata/cu_150k.xmu` spectrum: an NSLS
X11A metal foil measured at 150 K, attributed in its source to Newville, Ravel
and Zhang, September 1992. The second spectrum is a synthetic noisy replicate
with uniform noise of amplitude 0.0005 and seed 271828. It is not an independent
experimental measurement. Existing fixture attribution remains in
[`xraylarch_d867`](../../../crates/rexafs/tests/testfiles/xraylarch_d867).
Both fits used k = 3–12 Å⁻¹, R = 1.5–3 Å, k-weight 2, and the retained
`feffit/Feff_Cu/feff0001.dat` first-shell path. Convergence does not validate the
physical model.

LCF/PCA used three generated signals on 8770–10000 eV with 1 eV spacing. Each has
a baseline `0.2 + 0.00003 × (E − 8980)`, a logistic step
`1 / (1 + exp(−(E − 8980) / 2))`, and a Gaussian feature
`a × exp(−((E − c) / w)²)`. Here E, center c and width w are in eV, and a is a
dimensionless amplitude. The `(c, a, w)` values were `(8987, 0.45, 4)`,
`(9005, 0.65, 6)` and `(9027, 0.4, 8)`. These are controlled mathematical
signals, not measured Cu, Cu₂O or CuO standards.

The target combined them with weights 0.2, 0.3 and 0.5, adding uniform noise of
amplitude 1e-5 with seed 314159. Five noise-free mixtures supplied PCA training;
their exact weights are retained in [results.json](results.json). Processing used
E₀ = 8980 eV, fixed edge step 1, pre-edge offsets −190 to −40 eV, and a degree-one
normalization fit over +150 to +900 eV. Shared fixed processing preserves the
intended linear mixture. LCF/PCA used normalized absorption over −20 to +80 eV,
giving 101 fit points. LCF constrained nonnegative weights to sum to one with
no energy shifts. PCA was uncentered: its fractions describe squared signal,
not chemical concentrations. The two/three-component comparison demonstrates
reconstruction of this known signal, not a universal component-count rule.

## Evidence and limits

[results.json](results.json) records input, executable and screenshot SHA-256
values and scalar LCF/PCA outputs. The two full, unedited GUI captures are
[`assistant-lcf.jpg`](../../../website/public/screenshots/next/assistant-lcf.jpg)
and [`assistant-pca.jpg`](../../../website/public/screenshots/next/assistant-pca.jpg).
They are 1187 × 768 captures obtained through computer use. Large QA projects,
raw captures, logs, fit verification and the complete publication export remain
local under the QA directory; they are not included in source or package uploads.

The LCF/PCA and save-check GUI executable SHA-256 was
`adb9d4205fca37b35c0772c60167d92cf0774dff634909b84c0f27aafaabfa94`.
The final source additionally aligns reported reference order with the existing
cached-reference collector and explicitly rejects standalone references that
the collector cannot include. The final source build and tests cover those
guards; the screenshots use only materialized reference groups, whose order is
unchanged. These macOS checks do not qualify Windows or Linux rendering.

LCF/PCA outputs currently live in session state. Analysis-folder export retains
their arrays and report; reopening an `.rxs` project does not restore their result
panels. Saved conversations are historical evidence, not restored calculations.

## Automated checks

- Final desktop release tests: 502 passed, 0 failed, 5 explicitly ignored.
- Full core release tests including integration and documentation tests:
  331 passed, 0 failed, 3 explicitly ignored.
- Website type/content check: 37 files, no errors, warnings or hints.
- Regression coverage includes large archives, Unicode byte bounds, source
  pagination, exact settings, strict tool inputs, known-mixture reconstruction,
  invalid analysis operands and materialized joint save/reopen.

The numerical core and language-binding APIs are unchanged by this branch.
