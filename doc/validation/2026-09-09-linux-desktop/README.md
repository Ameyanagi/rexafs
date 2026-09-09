# Linux desktop audit — 2026-09-09

Host: Ubuntu 24.04.4, ARM64, Rust 1.98.1. Tested an X11 session at 1366×768
under Xvfb with Mesa llvmpipe. Native development packages were extracted into
the ignored `target/linux-deps/` directory because this shell has no sudo access.
The release GUI includes ReFEFF and FEFF10. This is software rendering and does
not measure physical GPU performance or native Wayland behavior.

The audit began at `dc993fe` and was rebased onto `6960765`, which independently
fixed the Unix accessibility return-type error and prepared version 0.2.3.
The branch retains that upstream fix and adds the changes below.

## Observed and corrected

- Startup previously left the center empty. The new [welcome view](welcome.png)
  provides Import spectra, Open project, and Open Cu example actions.
- The initial 1440×900 size exceeded laptop screens. The app now opens at
  1298×691 on this display; [960×640 resizing](small-window.png) also renders.
- Numeric fields assumed Menlo on every platform. They now use Menlo on macOS,
  Consolas on Windows, and DejaVu Sans Mono on Linux.
- Long placeholders painted over adjacent controls. Single-line painting is now
  clipped to the field. A long rename keeps the caret visible at
  [the end](long-input-end.png); Home reveals [the start](long-input-home.png).
  Character hit testing uses coordinates relative to the painted text origin.
- Folder scans had an unbounded event queue. The producer now waits when the UI
  falls behind and checks cancellation even for non-spectrum files. A regression
  test drains more than four batches and verifies every filename and completion.
- The Unix pprof development dependency prevented Windows core test/benchmark
  builds. It and the profiler implementation are now Unix-only; Windows core and
  all-target checks are included in CI.

## Interactive checks

Mouse and keyboard input used X11 automation (`xdotool`); screenshots were
captured with ImageMagick and inspected. No dedicated Computer Use connector was
available in this session.

- Opened the Cu example through Help, selected μ as the main spectrum, confirmed
  eV, and imported all 618 points. Data and normalization plots rendered.
- Opened a temporary embedded project through the GTK portal file chooser using
  the new Open project action. Only temporary copies were used for edits.
- Changed E0 from 8979 to 8990 eV, observed recalculation, and used Ctrl+Z to
  [restore E0 and the previous spectrum](e0-undo.png).
- Typed a long group name and used Home; both caret positions stayed inside the
  editor, and Escape discarded the draft.
- Saved a new embedded project through the native GTK Save dialog (82,637 bytes),
  then reopened it with the packaged executable and repeated the GUI smoke test.
- The repeatable smoke test opened a project with spaces/Japanese characters in
  its path, exercised Ctrl+1/2/3/4/5/7, resized, and quit with Ctrl+Q.
  [Recorded checks](gui-checks.json) cover ten assertions; screenshots remain
  review evidence rather than a pixel-perfect visual oracle.

## Automated checks

[Rust results](test-results.json): 236 core tests, 264 with ndarray compatibility,
and 461 release GUI tests passed. Optional/manual tests remain ignored as marked
by the repository. Strict core Clippy and both installed hook stages passed.

Python API: eight tests. JavaScript: six Node tests and a real Chromium Wasm
pipeline. All eight release-tool suites passed. The packaged Linux ARM64 archive
was extracted to a fresh directory and passed version, numerical-example, and
both embedded-backend checks; dynamic libraries resolved.

The optional core plotting tests also passed (see the recorded Rust results).
Windows checks and the new Linux CI smoke job require a successful remote PR run
before merge/release. The historical Windows interaction report covers the
earlier baseline, not these new input-widget changes. The local archive is
validation output, not a published release artifact.
