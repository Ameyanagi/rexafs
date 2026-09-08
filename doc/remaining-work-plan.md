# Remaining work plan — 2026-09-08

Releases 0.1.4 and [0.2.0](https://github.com/Ameyanagi/rexafs/releases/tag/v0.2.0)
are published on GitHub and all three package registries. Phase 1, Phase 2.1–2.7,
and concrete follow-up gaps D1–D4 are implemented and merged in
[PR #41](https://github.com/Ameyanagi/rexafs/pull/41).
Release preparation [PR #42](https://github.com/Ameyanagi/rexafs/pull/42) passed all
34 checks and merged. Tag `v0.2.0` points to
`7278be4ce91b80de235ce785782b46e7f5647447`, whose tree exactly matches that tested
candidate. The final [manual build 34204231697](https://github.com/Ameyanagi/rexafs/actions/runs/34204231697)
passed all 29 jobs. Signed downloads are qualified, the initial 42-asset
publication verified, and every published package matches that build. Hardware qualification,
upstream advisories, and the explicitly deferred features remain as listed below.

This replaces the earlier overlapping progress entries. The original Groups
UX design and implementation specification remain local design references in
the main checkout; the committed reports below record executed validation.

## A. In-flight work — complete

- [x] A1: Phase 1.7 intake records and receipts, including asynchronous intake,
  batched notices, indexed-folder restoration, identity-bound diagnostics,
  unknown freshness, lazy primary registration, focus/reveal, deduplicated
  errors, and bounded Details pages.
- [x] A2: Docked and popped-out Assistant hosts share retained state, settings,
  keyboard isolation, and narrow controls; qualified in the packaged app.
- [x] A3: Saved Assistant conversations and history settings, native restoration,
  and linked/embedded compatibility fixtures written after the format change.

## B. Release 0.1.4 — published

- [x] B1: Release preparation merged as
  [PR #38](https://github.com/Ameyanagi/rexafs/pull/38).
- [x] B2: Coordinated version, changelog, release notes, desktop-update and
  experimental-Assistant documentation.
- [x] B3: Local release gates, including core/default/ndarray/trust-region,
  strict core clippy, rustdoc, packaging, licenses, Python 3.10–3.14 wheel and
  source consumers, and JavaScript/Chromium/TypeScript consumers.
- [x] B4: Retained writer-generated linked and embedded 0.1.4 fixtures with
  saved conversations and manifest hashes; all earlier fixtures retained.
- [x] B5: Tag, final build, signing, all registry publications, and public
  [GitHub release](https://github.com/Ameyanagi/rexafs/releases/tag/v0.1.4).
- [x] B6 available-host qualification: final signed Apple Silicon and Intel
  downloads passed signature, stapling, Gatekeeper, and numerical checks.
  Both launched and restored projects; Intel was exercised under Rosetta.
  Apple Silicon ReFEFF and FEFF10 checks passed.
- [ ] B6 remaining platform qualification: native Intel hardware, clean-machine
  installation, and native interactive Windows/Linux checks. Windows/Linux
  retain their preview labels; their build and installer checks passed.

The release tag is `6041c78824b18c54e6ffc932000411bfc6a19382`.
[Manual build 34184418160](https://github.com/Ameyanagi/rexafs/actions/runs/34184418160)
passed all 29 jobs; signing run 34189680533 used its original binaries.
All 42 GitHub asset digests match the final manifest. Published Rust and npm
packages, 20 Python wheels, and the source archive match that same build.
See [release qualification](validation/2026-09-08-release-0.1.4/review.md) and
[release notes](release-notes-0.1.4.md).

## C. Groups and Phase 2 — merged; 0.2.0 published

The integration branch contains Phase 1's durable identities, source stacks,
Results, current/focus/mark separation, filters, row actions, locks, undoable
removal/duplication, drag-and-drop intake, and receipts. The inherited Phase 1.2
commit also carries fixed-penalty SVD-factor caching and its numerical/concurrency
regression tests. Phase 2 follows in these
separate implementation commits:

| Slice | Completed scope |
|---|---|
| 2.1 | Mapping drafts; explicit eV/keV/angle conversion; role/ROI validation; original rows and revision-bound full raw preview |
| 2.2 | Single-target mapping modal; native text input and focus isolation; full μ(E) preview; Cancel/Apply/Undo |
| 2.3 | Frozen batch repair; exact compatibility/preflight; locked/changed counts; one undo transaction; dependent Inputs changed notices |
| 2.4 | Missing-channel creation across captured sources; exact counts; independent processing defaults; duplicate prevention and one Undo |
| 2.5 | Pending layout clusters outside the catalog; representative selection; multiple outputs; partial acceptance; Review later/Reload/Locate |
| 2.6 | Immutable recipe versions, strict layout/units, explicit missing-unit confirmation, project and opt-in machine scope, exact application membership, persistence |
| 2.7 | Project-over-machine dispatch, conflicts/changed units to review, unnamed-layout confirmation per batch, compact Data inspector, exact application repair, Stop reuse |

Native workflows covered six-scan fluorescence navigation, named-target alignment,
and a six-input merge with saved provenance, plus mixed layouts, raw previews,
bulk channel creation and
Undo, partial review, repaired sources, recipe persistence, named fast-path
imports, unnamed review fallback, and repairs excluding manual remaps. Current
and marks remained intact. Unreadable sources remain pending and do not expose
another group's spectrum to tools. Declared Cu/Ni merge refusal was exercised
with identical numeric spectra.

Integration GUI suite: **426 passed, zero failed, four ignored**. Formatting, diff
checks, release build, packaged numerical/ReFEFF/FEFF10 self-checks, and all 18
retained compatibility samples passed. GUI clippy exits successfully with
warnings. Core/default, ndarray, trust-region, and strict core clippy were also
rerun successfully on the integrated branch. See [mapping and follow-up qualification](validation/2026-09-08-import-mapping/review.md)
and [0.2.0 release notes](release-notes-0.2.0.md).

- [x] Integrate Phase 1 and Phase 2 implementation in an isolated worktree.
- [x] Rebase the integration branch onto published main (`6041c78`); the rebase
  preserved the tested tree exactly.
- [x] Complete integration review and merge [PR #41](https://github.com/Ameyanagi/rexafs/pull/41).
  The Windows path correction passed the actual Windows matrix. All 34 checks
  passed on the final 0.2.0 tree in PR #42, including Intel DMG installation.
  The integration run 34196413430 also passed all 29 release jobs on retry; its
  earlier Intel failure was a busy-volume eject after numerical and FEFF checks.
  All 34 integration checks are green.

- [x] Prepare coordinated 0.2.0 versions, release notes, writer-generated linked
  and embedded fixtures, full local release gates, Python 3.10–3.14 consumers,
  source rebuild, JavaScript/Chromium/TypeScript consumers, and native Apple
  Silicon qualification. The release GUI suite passed 427 tests and verified all
  22 retained compatibility samples. See [0.2.0 qualification](validation/2026-09-08-release-0.2.0/review.md).
- [x] Complete release-preparation CI/review and tag the reviewed merge. PR #42
  passed Release builds 34198489189, Rust 34198489183, and Larch 34198489209.
- [x] Finish the final manual GitHub build and signing; qualify the actual signed
  ZIPs/DMGs, native Apple Silicon app and Intel app under Rosetta. All 42 original
  asset digests verified; the current desktop-only list has 19 verified assets.
- [x] Repair publisher downloads in [PR #43](https://github.com/Ameyanagi/rexafs/pull/43)
  after repeated unrelated desktop artifact transfer failures. Registry channels
  now verify only their complete required package set against the original build;
  `v0.2.0-publish-tools.1` resumes the unchanged source tag/build.
- [x] Publish all registries, verify their exact hashes, and make the
  [GitHub release](https://github.com/Ameyanagi/rexafs/releases/tag/v0.2.0) public
  as latest. Rust, npm, all 20 Python wheels, and the source archive match the
  qualified build. All three registries select 0.2.0 as the default stable version.

## D. Follow-up gaps and deferred work

- [x] D1: Disable Merge with an available compatibility reason in both the
  toolbar and row menu; retain full worker validation for every input.
- [x] D2: Retain declared XDI element/edge identities and prefer them when both
  are present; retain the numeric edge fallback and materialized provenance.
- [x] D3: Persist typed full-parser totals and bounded line examples by durable
  group and mapping revision. Restored evidence is explicitly historical until
  the current source is checked. Linked and embedded reopen tests passed.
- [x] D4: Replace both production `cfg!(test)` storage branches with injected
  FEFF workspace and project-cache roots; isolated-root tests passed.
- [x] D6 concrete cleanup: remove direct `derivative`, pin GitHub Actions by
  commit SHA, and configure Dependabot. These shipped in 0.1.4.
- [ ] D6 upstream follow-up: `cargo deny --locked check advisories` still fails.
  The current scan reports unmaintained transitive crates and quick-xml
  vulnerabilities through profiling and Typst dependencies. The indicated
  quick-xml fix requires 0.41+, beyond those dependents' current constraints.
  No advisory suppressions were added; the release license gate passes.

The following remain deliberate product decisions or explicitly deferred scope:

- D5: Legacy derived groups require quantity confirmation before normalization.
- D7: Confirmed series and explicit ordering/coordinate, frame-range processing,
  Spectrum/Catalog center view, persistent LCF/PCA result rows, paired-reference
  alignment, full tool previews and operation-specific bulk execution, drag reorder,
  and the separately deferred provenance graph.
- D8: Assistant proposal-review mode, transcript search, and conversation export.

## E. Feature handoff — not yet implemented

CI/CD and publication for 0.2.0 are complete. The broader product design is not.
The following expands D7/D8 into work that can be delegated independently.

| ID | Work package | Done when |
|---|---|---|
| F1 | Confirmed series with durable ordered membership and coordinates. Folder intake must not imply a scientific series. | Treat as series previews numeric filename order, first/last members, ties/gaps, channel, coordinate, and edge/quantity compatibility. Unknown metadata requires an explicit decision; later imports never silently renumber frames. Save/reopen preserves the exact order. |
| F2 | Frame-range browsing and processing for large series. Depends on F1. | Captured ranges are processed with bounded memory, progress/cancellation, exact coverage and failed-frame reporting; displayed overview samples are never confused with the full calculation. Qualify a 100,000-frame case. |
| F3 | Spectrum/Catalog center switch and persistent display ordering. | Both views share durable current/marks/locks and filters. Sorting and drag/move reorder survive reopen without changing source identity, recipe membership, or acquisition order. Keep display order distinct from F1's scientific frame order. |
| F4 | Full tool previews and operation-specific bulk execution. | Show one named before/after preview and exact target counts. Align each target to one named standard; Calibrate applies one measured shift; Difference uses one named baseline and common coverage; Merge remains N→1. Deglitch/Truncate/Rebin/Smooth retain their own units, bounds, and no-op rules. Preserve provenance, independent outputs, current/marks, and undo. |
| F5 | Paired-reference alignment. Depends on durable group/source identities; coordinate with F4. | Persist explicit sample/reference pairs, align references to the selected standard, and transfer each measured correction only to its paired sample. Missing/ambiguous pairs and duplicate corrections are visible and blocked until resolved. |
| F6 | Persistent LCF/PCA result rows. | Results have stable identity, recorded inputs/settings, export, save/reopen, and Inputs changed state. Distinguish analysis results from absorption spectra so they cannot enter incompatible tools. |
| F7 | Assistant proposal-review mode. | A proposed edit shows its exact targets and change preview before application; reject/cancel changes nothing. Apply validates current revisions/permissions and records one undoable accepted operation. Existing saved transcripts remain readable. |
| F8 | Assistant transcript search and conversation export. | Search can locate and open a saved conversation/message. Export preserves roles, timestamps, and historical receipts without treating them as executable actions. User chooses the conversation and destination; credentials are excluded. |
| F9 | Provenance graph — optional later design. | Define the scope first. Any graph reflects recorded identities and operations; it must not invent ancestry or replace the simple source-stack/Results list. |
| F10 | Native picker improvements — upstream-dependent. | Use a real `.rxs` filter and sensible initial folder when supported by the pinned toolkit. Retain post-selection validation, recent-folder access, and cross-platform behavior. |

F1–F10 are future work, not hidden requirements to republish 0.2.0. Legacy
quantity confirmation (D5) is intentional behavior, not a missing automatic
conversion. Existing Series, LCF/PCA, and tool features do not imply that all of
the newer contracts above are implemented.

## F. Plans missing from the previous summary

These cover broader audit/performance work outside the completed import phases.
Where a finding came from an older audit, reproduce it on current `main` before
changing code; do not reopen bugs already fixed by Phase 0–2.

| ID | Plan | Acceptance / first step |
|---|---|---|
| R1 | Project Save/Save As, dirty state, close protection, and recovery snapshots. | Current Save still prompts for a path and the close handler does not implement a complete unsaved-work lifecycle. Define Save vs Save As; cover edits, pending saves, cancel, failed writes, recovery after interruption, linked relocation, and embedded projects. Preserve atomic writes and old fixtures. |
| R2 | Input, allocation, and work limits. | Add checked budgets for FFT sizes, tiny-cell/large-cluster enumeration, decompressed input, and recursive parsers. Reproduce each reachable case with bounded tests; return useful GUI/Python/JS errors. Do not use blanket panic catching as the solution. |
| R3 | Imported-project and Assistant trust review. | Specify how external linked files/comments enter context and which actions require authority. Preserve legitimate multi-directory projects and existing opt-in controls. Treat prompt-injection claims as hypotheses until reproduced. |
| R4 | Remaining scientific/UI audit findings. | Recheck data-edge vs structure-absorber mismatch, persistent invalid-field feedback, reset subsection scope, and batch-local error counts. Confirm exact current behavior first; keep deliberate expert choices possible. |
| R5 | Processing performance after cached AUTOBK SVD. | Separate PRs for FFT window/k-weight preparation, exact cubic-resampling preparation, normalization fit preparation, then allocation/cache-lock reduction. Preserve numerical conventions and per-spectrum validity checks. Compare cold/warm and changed geometry, one/ten workers, stage and full-pipeline timings. |
| R6 | Render and fitting performance. | Profile render-time parameter serialization and fitting preparation, repeated FEFF interpolation/transform setup, and smoothing cost before selecting changes. Require measured benefits and numerical equivalence; do not infer speedups from call counts. |
| R7 | Upstream advisory maintenance and native platform qualification. | Close D6 and B6 with actual dependency/platform evidence. Keep all-features inventories distinct from shipped binary reachability, and Rosetta distinct from native Intel validation. |
| R8 | Website/documentation deployment. | The release runbook treats domain hosting as separate. Verify current hosting, DNS/HTTPS, installation instructions, and download links before claiming deployment is still missing or complete. |

The requested broader UI review is planned in
[UI simplification handoff](ui-simplification-plan.md), U1–U7. It prioritizes
progressive disclosure and visual hierarchy rather than adding instructions.

## G. Current requested polish

- [x] Quiet startup implementation: no automatic Cu example; Import/Open project first; examples
  available explicitly from Help.
- [x] Offline license reader implementation under Help, including packaged dependency notices.
- [x] Desktop-focused release downloads and concise registry/offline installation
  instructions.

These are the only implementation changes requested after the 0.2.0 release.
The feature and broader UI work above remains a handoff for later delegation.
The startup and Help changes passed 428 GUI tests and native packaged-app
checks; they are not part of the already-published 0.2.0 binaries.

## Workspace handoff

Implementation worktree: `/private/tmp/rexafs-import-phase2`, branch
`feature/import-groups-phase2`. Release preparation: `/private/tmp/rexafs-release-020`,
branch `release/0.2.0`. Post-release evidence is recorded in
`/private/tmp/rexafs-release-020-evidence`, branch `docs/release-020-evidence`. The main checkout at
`/Users/ryuichi/dev/rexafs` remains on `feature/import-groups-phase1`; its unrelated
dirty README, documentation, benchmark, and experiment files are preserved.
Only this plan is synchronized back to that checkout. Build/test logs and
disposable native fixtures are linked from the validation reports.
