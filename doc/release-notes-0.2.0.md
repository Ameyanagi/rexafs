# rexafs 0.2.0 — development notes

These notes describe the integrated Groups and import work being prepared for
review. Version 0.2.0 has not been tagged or published.

The Groups panel keeps current, keyboard focus, marks, and range selection
separate. Browsing a spectrum preserves marks. Source files and their channels
share collapsible stacks; materialized outputs appear in Results. Group names,
colors, processing locks, identities, and selection state survive project save
and reopen. Remove and Duplicate are undoable and preserve independent settings.

Import appends files and folders while retaining the current spectrum. Receipts
report added groups, duplicate sources, warnings, failures, and pending layouts.
Unfamiliar or unreadable sources remain pending until reviewed. A review can
accept part of a mixed folder, choose a representative file, define multiple
output channels, or defer the remaining sources. Reload and Locate support
repairing a missing or changed source.

Re-map columns opens a focused editor with original table values, one-based
column labels, the channel formula, and a full raw μ(E) preview. Explicit eV,
keV, and monochromator-angle conversions run once; angle conversion includes
its required spacing. Fluorescence uses a unique set of ROI columns. Invalid or
stale previews cannot be applied. Mapping edits preserve processing settings,
identity, current, and marks, and can be undone.

Saved recipes record an immutable version, exact layout and units, confirmed
unit assumptions, output channels, and reuse scope. Project recipes take
precedence over remembered computer recipes. Named compatible layouts import
directly; conflicting recipes or changed units return to review. Unnamed
layouts require a representative confirmation in each new batch. Changing a
recipe never rewrites groups imported with an earlier version.

The Data inspector summarizes source, channel, formula, units, recipe version,
and full-signal checks. Review this application targets its saved members,
independently of marks and filters. Batch repair skips incompatible, locked,
or manually remapped groups and applies the accepted set as one undoable edit.
Materialized descendants report Inputs changed. Missing channel creation shows
exact counts, avoids duplicates, and gives each new group independent processing
settings.

Merge displays known incompatibility before running. Declared XDI element and
edge identities take precedence when both are available; otherwise detected
edge positions provide the compatibility check. Materialized outputs retain
edge provenance. Parser totals and bounded source-line examples are saved with
the mapping they checked; historical evidence is identified separately from
current readiness.

Older projects remain readable. The project format gains additive recipe,
application, intake-history, edge, and parser-evidence fields. Existing mappings
are retained on reopen, even when machine recipes differ.

Confirmed series and explicit frame ordering, frame-range processing, the
Spectrum/Catalog center switch, persistent LCF/PCA result rows, paired-reference
alignment, drag reorder, Assistant proposal review, transcript search, and
conversation export remain outside this release's current implementation scope.
