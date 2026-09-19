# UI/UX review captures — 19 September 2026

Evidence for [the 0.2.11 desktop UI/UX review](../../ux-review-0.2.11.md).
These are observations of the interface, not scientific results.

## Build and capture method

- Build: `target/release/rexafs`, rexafs 0.2.11 `nightly-local-updater-test`,
  compiled from commit `c09ac25` (the source of the signed 0.2.11 release).
  The window title therefore reads "rexafs Nightly".
- Platform: macOS 26.5, ARM64, 2880 × 1864 physical pixels (2000 × 1294 logical)
  for most captures; 2200 × 1400 physical (1100 × 700 logical) for the two
  narrow-window captures (59, 60).
- Settings were isolated with `REXAFS_SETTINGS` pointing at an empty JSON file,
  so no personal recent projects, keys or assistant state appear.
- The app was driven by a Codex CLI (`gpt-6-astra`) computer-use session
  following a scripted tour. Captures were taken with the native window capture
  API and then re-encoded to JPEG at 1440 px width (quality 70) for the
  repository. No cropping or annotation.
- Inputs: the retained compatibility fixture
  `crates/rexafs-gui/tests/fixtures/projects/rexafs-0.2.11-embedded.rxs` (copied
  to a temporary directory before opening) and the public
  `crates/rexafs/tests/testfiles/xraylarch_d867/xafsdata/cu_150k.xmu`. The
  fixture's groups, undefined fit variables and pending source are synthetic
  persistence examples, not a user analysis.

## Captures

| Files | Content |
|---|---|
| 01–03 | Empty workspace; command palette empty and filtered by "fit" |
| 04–13 | Import of `cu_150k.xmu` through the native dialog; the file mapped automatically, so no column editor opened |
| 14 | Fixture project opened on the Data stage with the import receipt bar |
| 15–18 | Normalize, Background, Transform (Fourier) and Transform (Wavelet) |
| 19–24 | Fit stage as opened, then Structure, Calculate, Paths, Model and Results steps |
| 25–30 | Fit-mode menu and the RMC pages: settings, supercell, fit settings, empty results, run details |
| 31 | Series empty state |
| 32 | Publish |
| 33–37 | Panel toggles ⌘B, ⌘J, ⌘P, ⌘I and ⌘⇧J |
| 38–41 | Assistant panel and its model, reasoning and access menus |
| 42–44 | Group context menu, rename editor, group actions menu |
| 45–47 | Tools reached from the palette; LCF and PCA forms with results |
| 48–52 | Menu bar, About, Preferences shortcut check, Updates dialog and its preferences |
| 53–58 | After the theme toggle: Data, Transform (Wavelet and Fourier), Fit RMC, fit-mode menu, Fit Model. Plots and fields turned light while the chrome stayed dark. Revision note: this state did not reproduce on the same binary in a single-process session (see review section 3.1); only the wavelet panels kept the old theme |
| 59–60 | Fit and Data at 1100 × 700 |
| 61–62 | Warning diagnostics and the Problems drawer |
| 63 | Pending-import review dialog for the fixture's unavailable source |
| 64–67 | Series selection, series data, export menu and results |
| 68–70 | Publish with CSV, Analysis folder and Markdown formats |
| 71–73 | Wavelet settings and the lower part of the RMC settings page |

## Limits

macOS only. No FEFF, fit or RMC run was started. The light-theme state in
53–58 appeared in five captures of this tour but did not reproduce later with a
single rexafs process; several instances were running during this tour. The tester's own notes are retained as
[codex-tour-report.md](codex-tour-report.md) with private paths removed; they
are impressions, not verified findings. Every finding in the review cites a
capture or a source line. The app wrote extracted project data under
`~/.rexafs/project-data/` despite the settings override, so the override
isolates settings only, not application storage.
