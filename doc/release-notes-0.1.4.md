# rexafs 0.1.4

Analysis operations now keep their intended spectrum and settings attached to
the work. Tools reject stale or unreadable inputs, stage-specific parameter copy
preserves channel mapping, and mapping edits respect processing locks and undo.
Merge checks quantity and edge-position compatibility. Tool outputs retain their
quantity and operation inputs; parser diagnostics identify skipped rows and
excluded signal points.

Fit paths have an explicit coordination number **N** field. Leave it empty to
use the FEFF value, or enter a number or expression. Fit results provide **Copy
as Markdown**, including fitted values, uncertainties and fit statistics.

The experimental **Assistant** connects to an installed Codex CLI and offers
model and reasoning choices, a transcript with folded thinking and tool
activity, and receipts for analysis changes. Review mode is read-only; Edit
analysis permits supported changes. Web search and structure retrieval are
available. Extended access starts off and requires explicit consent in the
current session.

Assistant opens docked beside the analysis. Drag its left edge to resize it,
choose **Pop out** for a separate window, or **Dock** to bring it back. Closing
the panel retains the live conversation. Side panels collapse when necessary
to keep room for the plots.

**Conversations** contains saved conversations, **New**, and **Resume**. Project
Save keeps the newest five completed conversations by default; change
**Conversations kept per project** in the picker, or use zero to disable saving.
Resume continues the saved server thread when available. Otherwise, the next
message starts a new thread with a bounded summary of the last ten entries;
the transcript identifies which route was used. Restored receipts are historical
records and do not restore live approvals or undo permissions.

The `.rxs` format remains version 1. Earlier projects load with an empty
conversation history. Both linked and embedded 0.1.4 fixtures retain the new
fields, and all previously released samples remain in the compatibility suite.

Mac packages use signed, notarized ZIPs and DMG installers for Apple Silicon and
Intel. Windows/Linux portable downloads remain desktop previews pending native
interactive qualification; Windows also has a per-user setup installer with
automated installation checks. Release build, signing and validation provenance
are retained with the published artifacts. Python, npm and Rust retain their
analysis APIs.


## Build and qualification

These assets come from [release build 34184418160](https://github.com/Ameyanagi/rexafs/actions/runs/34184418160),
which passed all 29 jobs at commit `6041c78824b18c54e6ffc932000411bfc6a19382`.
Both macOS ZIPs and DMGs were signed and notarized by
[signing run 34189680533](https://github.com/Ameyanagi/rexafs/actions/runs/34189680533).
The original executables were signed without rebuilding. `SHA256SUMS` covers the
final release files, including the signed replacements.

Fresh extractions of both macOS ZIPs passed signature, stapling, Gatekeeper, and
numerical self-checks. Apple Silicon passed launch, plot rendering, embedded
project restoration, saved Assistant transcript/receipt restoration, and ReFEFF
and FEFF10 checks. Intel passed launch, plots, project restoration, and numerical
checks under Rosetta on Apple Silicon. Native Intel hardware and a clean-machine
installation were not tested locally. Windows/Linux remain desktop previews;
the build and installer checks passed, while native interactive qualification
remains pending.

[Rust publication](https://github.com/Ameyanagi/rexafs/actions/runs/34190269699),
[npm publication](https://github.com/Ameyanagi/rexafs/actions/runs/34190271206), and
[PyPI publication](https://github.com/Ameyanagi/rexafs/actions/runs/34190273012)
used that same build. The published Rust crate, npm tarball, all 20 Python wheels,
and Python source archive match its hashes.
