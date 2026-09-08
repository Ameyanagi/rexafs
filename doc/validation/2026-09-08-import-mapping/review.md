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

The final GUI suite passed 418 tests (zero failures, four ignored), including
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

Recipe dispatch, the compact inspector and the remaining phase acceptance
workflows are still pending. Release signing/publication and native Intel,
Windows and Linux interactive qualification are separate release work.

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
