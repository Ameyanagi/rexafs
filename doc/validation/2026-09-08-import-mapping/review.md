# Import mapping and review qualification

Implementation is on `feature/import-groups-phase2`, based on the reviewed
0.1.4 release preparation. The desktop candidate is a development build with
package version 0.1.4; it is not the tagged 0.1.4 release artifact.

## Completed slices

- Mapping model: explicit eV/keV/degree/radian conversion, fixed channel roles,
  unique ROI selections, original source rows and full revision-bound raw preview.
- Single-target editor: Cancel, Apply, Undo, invalid drafts, angle spacing entry,
  and opening another group's mapping while retaining current and marks.
- Batch repair: strict layout compatibility, frozen membership, source/target
  preflight, separate locked/changed/incompatible counts, one undo transaction,
  and provenance notices on materialized dependents.
- Channel creation: independent processing defaults, missing-channel counts,
  duplicate prevention and one creation inverse. The 40-source test creates
  exactly 37 missing Reference channels when three already exist.
- Pending review: unfamiliar layouts and unreadable sources remain outside the
  catalog until explicitly accepted. Each selected output is validated against
  every accepted source. Representative selection, partial acceptance, Review
  later, Reload and Locate are available.

## Native acceptance evidence

The packaged Apple Silicon candidate was operated through native computer use.
Temporary source fixtures are in `/tmp/rexafs-phase2-ui/` and
`/tmp/rexafs-review-ui/`; these checks do not modify scientific source files.

Batch channel review captured every source, including previously unseen lazy
primary rows. Six compatible fluorescence channels were created (16 to 22
groups); repeating the action offered zero duplicates. One Undo restored 16
groups while retaining the marked current Cu spectrum.

Mixed review imported one known file and left five sources pending in three
clusters. Selecting a second representative retained the channel draft. An
incomplete optional mu column cleared the preview and disabled Add. After
assigning that column, accepting two files with two channels created four
groups. The next layout opened automatically. Deferring it retained the pending
sources. Locate replaced the broken source with a repaired fixture; the full
645-point preview appeared and explicit acceptance created its group. Current
and marks remained intact throughout.

## Recipe and persistence qualification

The Phase 2.6 GUI suite passed 418 tests (zero failures, four ignored), including
linked and embedded project round trips on a simulated machine with a
conflicting reusable recipe. Saved effective mappings, recipe versions, group
identities, application members and deferred sources remained authoritative.

Native qualification exposed compressed scrollable rows after adding recipe
controls. The raw preview, column picker, ROI list and target list now retain
their content height inside the scrolling modal. The final candidate displayed
an unobstructed table/plot and usable column buttons.

A native save/open cycle retained all five pending sources, the current Cu
spectrum and its mark. An explicit two-output review then saved
`Review qualification · v1`, scoped to the fixture folder with a confirmed eV
assumption for the undeclared axis units. Inspection of the native writer's
output confirmed four unique group members from exactly two source identities,
with channel and mapping revisions, plus three deferred sources. Reopening
that project restored six groups and the same three pending sources and mark.

Disposable native writer outputs:
`/tmp/rexafs-recipe-qualification.rxs` and `/tmp/rexafs-recipe-accepted.rxs`.
The intake receipt also has a regression test ensuring that restore work does
not replace the saved import receipt.

Machine-wide remembering was exercised through isolated settings serialization
tests; native qualification used project scope.

Recipe dispatch and the compact inspector are covered below. Release 0.1.4
qualification is recorded separately in
[its release report](../2026-09-08-release-0.1.4/review.md).

## Phase 2.7 — recipe dispatch and compact inspector

The full GUI suite passed **420 tests, zero failed, four ignored**. Focused
coverage includes project-over-machine priority, same-priority conflicts,
changed units/conversion metadata, stopped reuse, unnamed-column suggestions,
and exact application repair excluding manual mappings and locked groups.
The worker test verifies a shared application for two matching sources and
review fallback for a changed-unit source. Its initial failure was a test
fixture using a noncanonical temporary path; intake supplies canonical paths,
and the corrected fixture exercises that same contract.

Native qualification used the packaged release build, not the test harness.
The saved project restored six groups, pending sources, recipe membership, and
current/marks. The Data inspector displayed source, channel, formula, units,
recipe version, full-signal point count, energy correction, and scoped actions.
Files/Scans tabs and the old inline mapping form are removed; Groups stays unified.

Review this application captured exactly two Transmission groups. After one was
independently remapped, the same application reported one compatible and one
changed group, and offered Apply to one file. A named four-column layout was
accepted for two pending sources. Importing a later folder added its matching
named source immediately, while an unnamed source stayed pending with its prior
mapping offered as a suggestion. Existing current and its Cu mark were retained.

The native save `/tmp/rexafs-dispatch-qualification.rxs` contains three distinct
applications: original two-source/two-channel review (four members), named review
(two members), and later named fast-path import (one member). The later source
has not joined the earlier application. The unreadable and unnamed sources
remain pending. No machine recipe was saved during native qualification.

Logs: `/tmp/rexafs-phase27-final-tests.log`,
`/tmp/rexafs-phase27-repair-test.log`, `/tmp/rexafs-phase27-dispatch-test2.log`,
`/tmp/rexafs-phase27-build1.log`, and `/tmp/rexafs-phase27-package1.log`.


## Follow-up gaps D1–D4

Merge now disables its toolbar and row-menu actions when available evidence
shows an incompatible quantity, channel, energy range, or edge, and presents
the reason. The worker still loads and validates every source before producing
a result. Declared XDI element/edge headers are normalized and retained; matching
declared identities take precedence over the 50 eV fallback used when either
identity is absent. Materialized tool and merge outputs retain edge provenance.

Full parser records are persisted by durable group identity and mapping revision.
They retain every occurrence count and at most five source-line examples per
category. Source Details and row warnings identify restored records as a previous
parser check until the current source is checked. Linked and embedded round-trip
tests verify identical diagnostics, translated paths, and rejection of evidence
for a different mapping.

The final packaged Apple Silicon candidate was operated through native computer
use. Two synthetic XDI sources contain identical 618-point numeric spectra but
declare Ni K and Cu K. Marking both disabled Merge and displayed both declared
identities, despite their matching numerical edge positions. A generic text
source with nine malformed tail rows displayed all nine occurrences and source
examples 620–624. Native Save and Open restored all four groups, the two marks,
parser diagnostics, and the declared-edge refusal. The disposable native writer
output is `/tmp/rexafs-edge-qualification-valid/evidence-qualification.rxs`.
A deliberately malformed XDI fixture remained pending; it was not accepted as a
valid source to obtain this result.

The two production `cfg!(test)` storage redirects are removed. Project extraction
and FEFF workspace creation accept injected roots; normal entry points retain the
standard user directories. Tests prove that linked projects do not initialize a
cache, the same embedded archive restores into distinct injected roots, and
concurrent FEFF snapshots remain isolated.

Final validation: **426 GUI tests passed, zero failed, four ignored**. Formatting,
diff checks, the release build, packaged numerical/ReFEFF/FEFF10 self-checks, and
all 18 retained compatibility fixtures passed. GUI clippy completed successfully
with warnings. Core/default, ndarray, trust-region, and strict core clippy were
also rerun successfully on the integrated branch, including the fixed-penalty
SVD-cache change carried by the existing Phase 1.2 commit.
Native Windows/Linux and native Intel hardware qualification remain outstanding.

Logs: `/tmp/rexafs-gaps-final-tests.log`, `/tmp/rexafs-gaps-build.log`,
`/tmp/rexafs-gaps-package.log`, `/tmp/rexafs-gaps-clippy.log`,
`/tmp/rexafs-roots-project-tests.log`, and `/tmp/rexafs-roots-feff-tests.log`.


## Integrated six-scan acceptance

The final candidate imported six 645-point fluorescence scans and retained the
current Cu spectrum. Marking the six scans and arrowing to the third kept all
six marks. Align to reference named the third scan as target and the first scan
as standard; Apply created a RawMu result with a measured −0.9999997 eV shift
(the synthetic input offset was exactly 1 eV). Replacing the third source's mark
with its aligned result and merging created a second RawMu result from exactly
six inputs. Five source scans and the aligned result remained marked.

The native writer saved
`/tmp/rexafs-six-scan-qualification/six-scan-qualification.rxs`. Inspection
confirmed the alignment's two identity-bound operands, the six exact merge
inputs, retained fluorescence mappings, and separate materialized result IDs.
This exercises single-target alignment; operation-specific bulk tool execution
remains explicitly deferred.

Integrated core logs: `/tmp/rexafs-integration-core.log`,
`/tmp/rexafs-integration-ndarray.log`, `/tmp/rexafs-integration-trust.log`, and
`/tmp/rexafs-integration-strict.log`; every command exited zero.
Integration review and live CI: [PR #41](https://github.com/Ameyanagi/rexafs/pull/41).
