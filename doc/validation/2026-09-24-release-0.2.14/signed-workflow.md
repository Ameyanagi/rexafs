# Signed 0.2.14 desktop workflow

The signed Apple Silicon application was installed from the qualified DMG and
reviewed on 24 September 2026 in Japan, on an Apple M4 running macOS 26.5.1.
Source `4925e34b735ec8908e5084e797110aba7c235fe7` was built by
[run 35920006031](https://github.com/Ameyanagi/rexafs/actions/runs/35920006031) and
signed/notarized by [run 35924216636](https://github.com/Ameyanagi/rexafs/actions/runs/35924216636).
Strict signature, notarization, Gatekeeper, installed core, updater-helper and
FEFF checks passed. The existing user app and project were left unchanged;
the review used a separate installation, copied project and isolated settings.

The Computer Use connector returned error −10000, “Sender process is not
authenticated.” It did not complete a GUI interaction. The review instead used
already-authorized public macOS Accessibility controls and native mouse/keyboard
events. No permission or authentication setting was changed. The eight selected
PNGs are full, unedited native window captures, not connector output. Their
[capture manifest](../../../website/public/screenshots/0.2.14/capture.json)
records original dimensions, hashes, input identity and the fallback limitation.

## Cu fits, expressions and ranges

The retained review project contains a prepared 517-point Cu foil reference,
two-frame Series fit and earlier 64-revision replay. Its original 0.2.12 preview
identity is retained. The reference derives from the user-supplied 2016 Athena
Cu-oxide project; [fixture attribution](../../../crates/rexafs/tests/fixtures/analysis/cu-mixtures/README.md)
records permission and the unknown original acquisition author/license. It is
not raw data downloaded for this review or MIT-licensed measurement data.

Through **Series → Add trend… → EXAFS fit…**, the signed app reused the saved
one-path, four-variable Cu model and fitted both frames, using k = 2–12 Å⁻¹ and
R = 1–3 Å. Both converged. A custom **Cu–Cu distance** trend used `reff + dr_1`
with display unit Å; the app showed approximately 2.54160 ± 0.00438 Å.
The error bars are local covariance estimates, not total physical uncertainties.
All 102 numerical fit-summary values, including covariance and path statistics,
agreed with the retained result within `1e-8 + 1e-6 * abs(retained value)`.
The maximum absolute difference was 1.5752865056839482 × 10⁻⁷; the results were
not bit-identical. This is software agreement, not experimental validation.

The plot context menu opened **Axis range…**. A Y minimum of zero and automatic
remaining limits survived a frame change. Selecting `ss_1` retained its separate
natural range, and returning to distance restored the zero minimum. **All Auto**
restored the distance range. Saving with source files and reopening through the
normal project picker retained the new fit, expression and earlier 64 accepted
revisions/63 Live fit updates. The fresh fit project has SHA-256
`d1826204f3e49bcafcd0919a48de1e618972cd65d35d4eeea46e1468497f335a`.

## Three-signal arrivals

The tagged synthetic generator created three 401-point XDI files separately.
Preview selected transmission, fluorescence and reference together, **Running
averages**, and named processing reference **Cu_foil_001.xdi · scan_1**. Completion
used the default three quiet observations one second apart. EXAFS and XANES peak
fits were off for this synthetic test; Cu fitting was qualified separately.

1. The first source produced three successful signal frames. Opening the separate
   monitor returned the main window to Data with the Cu spectrum still selected.
2. A second arrival advanced the monitor to two files without replacing that
   selection.
3. Pause held the accepted count at two while a third completed source waited.
   Resume accepted it once, yielding nine successful signal frames.
4. A further pause/resume left the entire recorded ledger payload unchanged.
   Each of the three final running averages retained exactly three inputs.
5. All 401 raw-absorption values in each average were compared with the arithmetic
   mean of its three accepted inputs. Maximum absolute differences were
   5.551115123125783 × 10⁻¹⁷ for fluorescence and zero for reference/transmission,
   against a 10⁻¹⁰ absolute tolerance. Energy grids matched exactly.
6. Docking the monitor in the sidebar preserved the Cu view. **Finish acquisition**
   stopped the watcher, and a separate portable project
   retained the three accepted revisions and average provenance. No feeder or
   acquisition session was left running.

Synthetic counts and repeated Cu spectra do not establish signal-to-noise
improvement, acquisition throughput, physical beamline operation or behavior on
network shares. The earlier source reviews retain batch/rewrite coverage.

## Assistant inspection

The fresh settings had Automatic model selection; the connected catalog resolved
it to **GPT-6-Sol**. **Access** exposed **Connected apps and files**, verified off.
No Assistant prompt, file transfer or cloud operation was sent. This inspection
does not establish an end-to-end Google Drive connection.
