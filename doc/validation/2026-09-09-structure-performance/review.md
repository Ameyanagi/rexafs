# Structure renderer performance — 2026-09-09

The restored bond appearance at `0d3090b` remains the reference. This revision
reduces scene-order bookkeeping by grouping the non-overlapping lighting annuli
of each atom and the flat-ended segments of each straight stroke into paint
layers. Individual bond highlight passes keep their original order, colors,
widths, endpoint trimming, depth subdivision and opacity behavior.

Straight segments now submit the same rectangular coverage directly, avoiding a
fresh Lyon path/tessellator for each segment. A regression compares the actual
vertices and triangle areas with Lyon across both directions, diagonals, thin
lines, thick highlights and near-degenerate segments. Lyon's collapsed short
segments remain empty. Curved paths and closed outlines keep the original path
builder. Calculation and picking geometry are unchanged.

## Measurements

Optimized macOS Apple M4 builds with `refeff-runner,feff10-runner`; same window
size, dark theme, curated CuO, Crystal + cluster at 8 Å, 217 cluster atoms,
356 displayed bonds, Ball + stick, shading and Depth cue on, 100% camera zoom.
Computer Use issued ten pairs of opposite drags for rotation and six pairs with
Center focus at 34%. Startup, source loading and cold initial frames were excluded.

`ZED_MEASUREMENTS=1` uses the pinned GPUI frame timer around window draw, present
submission and frame-arena cleanup. It measures CPU frame work, including driver
submission, but not completed GPU rendering or physical screen presentation.
Discrete automated gestures coalesce differently between runs, so frame counts
vary; neither tool-call duration nor idle intervals are treated as frame time.

| Scene | Baseline frames | Revised frames | Median CPU frame, before → after | p95, before → after |
| --- | ---: | ---: | ---: | ---: |
| Rotation, center focus 0% | 37 | 31 | 42.1 → 26.0 ms | 48.6 → 33.8 ms |
| Rotation, center focus 34% | 17 | 18 | 54.1 → 28.1 ms | 87.2 → 34.5 ms |

These runs show about 38% and 48% lower median CPU frame cost. They are small
samples on a shared development machine with variable background load. They do
**not** establish steady 60 Hz rendering: the median total CPU frame still exceeds
16.7 ms. Native rotation, fading and the restored bond shading were inspected.

The revised build additionally enables `REXAFS_DEBUG_STATS=1`. Its `[structure]`
records measure CPU canvas paint duration and the oldest coalesced camera event
handler to the end of canvas painting. Rotation median canvas paint was 10.9 ms;
handler-to-paint median was 10.2 ms (21 observed frames), with p95 29.4 ms. The
faded scene measured 9.9 ms and 15.8 ms respectively (12 event-bearing frames).
These are application-handler measurements; OS input delivery and final display
latency are excluded. Diagnostics are off by default.

Raw timing lines and `timings.json` accompany this report. The JSON records the
measured development binary and renderer source hashes. Percentiles use inclusive
linear interpolation. The earlier native sampling profile put most sampled
application draw work in viewport painting, particularly primitive-bound ordering,
with additional Metal submission cost; surrounding-control layout was smaller.

## Validation and integration

The full distributed-feature GUI suite passes: **449 tests, zero failures, five
ignored**. Geometry/opacity, picking, clipping, project persistence, publication
and both FEFF backend coverage pass alongside the new stroke equivalence test.
Formatting and whitespace checks pass.

The separate ruviz font/Typst/international typography work in upstream PR #187
is not included in this measurement. This branch retains ruviz/ruviz-gpui 0.13.1;
that update can be integrated and qualified after the upstream release. These
changes operate in rexafs's direct GPUI structure renderer.
