# Remaining work plan — 2026-09-08

The 0.1.4 release is published. Phase 1, Phase 2.1–2.7, and concrete follow-up
gaps D1–D4 are implemented and locally qualified on
`feature/import-groups-phase2`. Integration review/CI and the platform checks
listed below remain; the explicitly deferred features are still backlog.
Version 0.2.0 has not been tagged or published.

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

## C. Groups and Phase 2 — implemented, integration review next

The integration branch contains Phase 1's durable identities, source stacks,
Results, current/focus/mark separation, filters, row actions, locks, undoable
removal/duplication, drag-and-drop intake, and receipts, followed by these
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

Native workflows covered mixed layouts, raw previews, bulk channel creation and
Undo, partial review, repaired sources, recipe persistence, named fast-path
imports, unnamed review fallback, and repairs excluding manual remaps. Current
and marks remained intact. Unreadable sources remain pending and do not expose
another group's spectrum to tools. Declared Cu/Ni merge refusal was exercised
with identical numeric spectra.

Final GUI suite: **426 passed, zero failed, four ignored**. Formatting, diff
checks, release build, packaged numerical/ReFEFF/FEFF10 self-checks, and all 18
retained compatibility samples passed. GUI clippy exits successfully with
warnings. See [mapping and follow-up qualification](validation/2026-09-08-import-mapping/review.md)
and [0.2.0 development notes](release-notes-0.2.0.md).

- [x] Integrate Phase 1 and Phase 2 implementation in an isolated worktree.
- [ ] Rebase the integration branch onto the published main commit, push the
  review PR, and complete its CI/review before merge.
- [ ] Plan and execute a separate 0.2.0 release after integration. The development
  candidate still reports package version 0.1.4 and is not the tagged artifact.

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
  alignment, full tool previews and operation-specific bulk execution, drag reorder.
- D8: Assistant proposal-review mode, transcript search, and conversation export.

## Workspace handoff

Implementation worktree: `/private/tmp/rexafs-import-phase2`, branch
`feature/import-groups-phase2`. The main checkout at
`/Users/ryuichi/dev/rexafs` remains on `feature/import-groups-phase1`; its unrelated
dirty README, documentation, benchmark, and experiment files are preserved.
Only this plan is synchronized back to that checkout. Build/test logs and
disposable native fixtures are linked from the validation reports.
