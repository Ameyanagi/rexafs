# Live acquisition: first development increment (historical)

This record describes the initial snapshot-only prototype. The current development
workflow is documented in [Live acquisition](live-acquisition.md), with subsequent
work recorded in [milestones B–F](analysis-b-f-progress.md).

Branch: `feature/live-acquisition`, based on the Phase A PR at `153aa88`.
This is the start of B1 from the [analysis design](complete-analysis-design.md),
not a released Live workflow. There is no Live button or automatic directory
processing yet. The Phase A computer-use record belongs to its separate PR.

The first implementation is `crates/rexafs-gui/src/live_intake.rs`. It has no
external service, new dependency, producer-file write, or background thread.
A future coordinator supplies absolute file paths and monotonic observation
times, then receives readiness, review, unavailable, or accepted outcomes.

Two completion contracts are explicit:

- **Quiet file (inferred):** the default requires three observations with matching
  length/modification evidence, at least one second apart. More frequent polls
  do not count as additional stable observations. Changed or missing inputs
  restart the observation count. A writer pausing long enough can look complete;
  this policy does not prove acquisition completion.
- **Producer digest marker:** the producer closes its source, writes a JSON
  marker to a temporary path, then atomically renames that marker to
  `<source>.ready` (or the selected suffix). The marker contains `bytes` and a
  lowercase `sha256`. The consumer requires both to match the captured source.
  A rename of an arbitrary data file is not treated as proof of completion.
  The digest checks consistency, not the identity or trustworthiness of a producer.

Input reading is limited to 256 MiB, matching the universal reader; markers are
limited to 4 KiB. File metadata is checked before and after capture. The same
captured bytes enter `parse_measurement`, and the owned snapshot carries those
bytes, their SHA-256, parsed scans and completion evidence together. Subsequent
processing must consume that snapshot rather than reopening its source path.
Unknown or malformed formats remain explicit review outcomes. A readable table
can still need a user-selected detector mapping; readiness does not select it.

Distinct paths retain distinct source identities even if their bytes match.
Changed content at a reused path is a new content revision, not an invented
acquisition time. A result must be committed to the future durable ledger before
calling `acknowledge`; acknowledgement is idempotent. Until then the snapshot
can be retried. Restoring a committed revision does not resume acquisition or
skip validation of current bytes. Symlink inputs need explicit review.

This foundation uses ordinary local metadata and immutable captured bytes; it
cannot prove coherence during an adversarial or undetectable same-metadata
rewrite under quiet-file mode. Producers requiring authoritative completion
must use the digest-marker contract. Growing HDF5/SWMR and instrument control
remain outside this implementation.

## Next integration steps

1. Add bounded folder reconciliation, an explicit include-existing preview,
   source filters, and a queue of source locators rather than retained payloads.
2. Freeze the selected Phase A recipe and check every parsed scan/channel.
   Persist source/record/revision identity and result atomically before publishing.
3. Add Live controls within Series: Start, Pause, Resume, Stop, Retry, Review
   input and optional Follow latest. Reopened sessions remain paused.
4. Use computer use with synthetic incremental writers, restarts and bursts;
   qualify Windows/Linux and actual network shares separately.

Until these steps pass, no automatic intake or crash-safe publication claim is
made. The current deterministic tests exercise spaced observations, incomplete
writes, malformed rows, completion-marker mismatches, changed-during-read
sources, identical-content distinct files, reused names, missing/reappearing
sources, multi-scan retention, bounded source/marker sizes, marker changes during
capture and restoration of committed revisions.

Validation on the development macOS host: all ten tests passed with
`cargo test --locked -p rexafs-gui live_intake`. These include a real source
replacement during capture, not only simulated state transitions. Native
Windows/Linux execution of this new branch has not yet been performed.
