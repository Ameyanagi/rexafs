# Larix session import plan

Written before implementation on 2026-09-14 for the unreleased shared reader.
Use the supplied 15 valid and six invalid sessions without regenerating them.
The reference is Larch 2026.3.1's
[session writer](https://github.com/xraypy/xraylarch/blob/2026.3.1/larch/io/save_restore.py)
and the fixture bundle's [format notes](../crates/rexafs/tests/fixtures/sessions/larix/FORMAT.md).

1. Detect the `##LARIX: 1.0` content signature after bounded gzip decoding.
   Validate section endings, unique symbol names and display-name references.
   Accommodate the writer's documented extra `_xasgroups` count. Retain session
   configuration, command history, symbol order and serialized objects as inert
   provenance; never evaluate history, expressions or saved Python classes.
2. Decode modern base64 arrays and legacy numeric-list arrays. Validate dtype,
   byte order, checked shape products and exact payload lengths. Preserve
   complex real/imaginary components, independent grids, empty arrays and exact
   integer bytes. Numeric views use f64; warn when large integers lose precision.
   Retain nonfinite metadata without replacing it with finite values. Bound
   nesting and decoded array memory.
3. Expose each top-level data Group in source order. Automatically map only
   compatible one-dimensional `energy` and stored `mu` arrays with supported
   energy units. Preserve duplicate energy positions and saved detector channels.
   Archive normalization, background, chi(k), Fourier arrays and settings without
   activating rexafs processing caches. Chi-only and non-XAS groups stay unmapped.
4. Compare all 384 supplied array payloads against independent pre-serialization
   expectations, including shapes, dtypes, full hashes and sampled values.
   Exercise all six invalid inputs, empty sessions, metadata and resource limits.
   Run core regressions before extending interface tests.
5. Expose session metadata and complex arrays in Python and TypeScript/Wasm
   snapshots. Test installed packages and editor help. Both GUIs use the shared
   reader; preserve Larix provenance and saved arrays in desktop projects, show
   session warnings, and test real processed/multigroup imports in the browser.
6. Update the reader guide and Next API references, run integrity/package checks,
   and build the release desktop executable. Keep the currently running GUI
   available while building; do not discard an open user project to restart it.

Follow-up requested during implementation: check import robustness and simplify
the APIs and workflows. Reject duplicate JSON keys, gzip truncation and trailing
data; clear stale desktop reviews when a new read starts. Make **Import…** use
the universal reader for any single file. Keep detected signals visible and
collapse custom columns for unambiguous scans. Require an explicit signal or
custom mapping when detection is ambiguous, in both GUIs. Preserve the existing
folder/multiple-file recipe workflow and document that boundary.

This imports saved measurements and results. It does not restore an executable
Larch environment, reproduce Larch processing, or add support for every Python
object or older `save_groups()` container.
