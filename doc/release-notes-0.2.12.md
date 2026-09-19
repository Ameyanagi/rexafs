# rexafs 0.2.12 — release preparation

This patch is being qualified and is not yet published. The public downloads
remain on 0.2.11 until the exact-tag builds, signing and package checks complete.

## In-app updates

On macOS, the updater now stages its helper as a complete signed app bundle.
The previous bare-executable copy lost its signed Info.plist context and was
killed before replacement. The corrected helper keeps its signature and resources
together, while retaining incoming-app verification, recovery and rollback.

On macOS, Windows and Linux, **Update and restart** remains available after using
the separate **Download** action. Unsupported installation layouts show the
reason. Updating another release channel does not replace the current app.

Existing affected Mac installations, including 0.2.10 and 0.2.11, need the first
corrected app installed once because their old helper cannot start. The repaired
app can perform subsequent in-app updates. The new signing qualification checks
helper startup in both signed ZIP and installed DMG bundles. Windows and Linux
keep their platform-specific helpers and remain desktop previews.

## Documentation and compatibility

The website refresh includes 29 unedited computer-use captures from the signed
0.2.11 Mac release, covering import, the Assistant, processing, fitting and Series.
Saved PCA/MCR calculations are identified separately from workflows rerun during
capture. Public data attribution and historical screenshots remain available.

This patch preserves scientific algorithms, numerical defaults and project
format 1. Rust, Python, npm/WebAssembly and desktop package versions are
coordinated. Qualification uses new linked and embedded projects written through
the 0.2.12 writer; older fixtures retain their original bytes.

The [qualification record](validation/2026-09-19-release-0.2.12/review.md)
distinguishes completed checks from pending release gates.
