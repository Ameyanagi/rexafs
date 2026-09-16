# Series interaction validation, 2026-09-17

This record covers unreleased local changes on `feature/complete-analysis`,
after `a06e9a6`. Checks used the native macOS ARM64 release build and computer use.
Windows and Linux visual behavior was not checked in this session.

## Synthetic source and retained evidence

The input is the 513-frame output of
[`generate-series-metric-example.py`](../../scripts/generate-series-metric-example.py).
It includes a deliberately larger peak at frame 258. No private ReGe measurements,
unpublished spectra or experimental projects were added to the repository.
The local portable project `Series controls demo.rxs` retained the completed
trend and reopened successfully.

## Observed interactions

- Dragging the lower and upper energy boundaries updated the fields to −8.5
  and 35.4 eV relative to E₀, without changing the plot viewport. The Mean preview
  was 0.846835. Moving the upper boundary to 19.1 changed it to 0.703378.
- A k-space boundary moved from 10 to 8.92 Å⁻¹. Attempting to cross the lower
  boundary left the interval unchanged. K-space plots kept zero centered, with
  symmetric limits that retained the full amplitude.
- Right advanced the trend preview from frame 1 to 2; End selected frame 513;
  Left selected 512. While editing From, Left/Right moved the text cursor and
  did not change the selected spectrum.
- On frame 512, dragging the upper boundary changed the Flat Mean interval from
  −10…30 to −10…34.2 eV. The preview value changed from 0.789494 to 0.816083.
  Calculate all 513 frames completed, and the saved trend showed 513/513.
- Difference against frame 1 showed its own cursor trace at zero. With reference
  258, frame 1 showed a negative difference near the deliberately larger peak;
  selecting frame 258 returned the cursor trace to zero.
- Plasma, reversed Plasma and reversed Gray changed the heatmap colors without
  changing the selected spectrum or its numerical scale. Auto restored the
  default colors; disabling Difference restored the original spectrum.

The initial GUI checks found and corrected two reference-entry issues: the
overview captured the numeric field's focus, and clicking Set reference could
read a previous committed value before blur. The submit path now validates the
visible field text directly. Invalid text never falls back to the previous number.

The final popup-menu build was checked in a separate Heatmap QA application:

- Colors opened as a vertical menu with gradient swatches. Selecting Plasma
  closed the menu and changed only the colors. Reverse updated both the swatches
  and heatmap; Auto restored the ordinary or difference default.
- The heatmap card retained its displayed bounds (approximately x=241…577,
  y=126…732 in the 1187 × 768 capture) with the menu open and closed. Difference,
  Reference and Colors remained in the same positions when the reference changed
  from absent to 258 to 1. The neighboring plots retained their sizes.
- Typing 258 and clicking Set reference without pressing Enter applied frame 258.
  Entering 514 in the 513-frame series kept the menu open, displayed a range error
  and retained reference 258. Escape and outside clicks dismissed the menus.
- Selecting reference frame 258 produced a zero cursor trace; Right selected
  frame 259 and showed the negative peak difference while reference 258 stayed
  fixed. Left returned to 258. Changing the reference to 1 showed its positive
  difference peak.

The public `series-difference-colors.jpg` is an unedited native screenshot of
this synthetic project, with frame 258, reference 1 and the Colors menu open.

## Numerical scope

Difference is a display of `frame(x) − reference(x)` on the overview's common
absolute axis, with no extra alignment or fitting. R-space subtracts magnitudes.
Missing coverage, failed rows and mismatched k weights yield gaps. The reference
is processed from the exact frame, even if it is not an overview sample.
The cursor also uses the exact selected frame. The heatmap still samples at most
192 frames; full-frame trends visit every series member.

Reference subtraction and colors are temporary view settings. They do not change
original groups, processing parameters or saved measurement values.

## Automated checks

The full GUI suite passed with 550 tests, zero failures and six ignored tests.
Focused tests also passed after the reference-entry changes. Tests cover boundary
rounding and coverage, crossed boundaries, keyboard focus predicates, rendered
symmetric k-space bounds, subtraction sign and gaps, incompatible k weights and
palette reversal. The optimized release build completed successfully.
Rust formatting and patch-whitespace checks passed. The website check reported
zero errors, warnings and hints across 37 files.
