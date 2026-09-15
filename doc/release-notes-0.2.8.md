# rexafs 0.2.8

Release candidate; publication is pending. The
[qualification record](validation/2026-09-15-release-0.2.8/review.md) tracks the
reviewed source, exact-tag build, signing and package publication.

## Desktop updates

On macOS, **Update and restart** downloads and verifies a newer release of the
current channel, saves an embedded recovery project, installs the app and reopens
that project. The downloaded bundle must have the official Developer ID signature,
pass Gatekeeper and match the requested channel, version and architecture before
its executable runs. The previous app and recovery project are retained.

Progress and cancellation are available before restart. Active calculations,
imports and Assistant turns must finish first. A failed preparation keeps the
current app open; a failed replacement or launch restores the previous bundle.
The recovered analysis is a separate project copy. Save it to your preferred
location; the session undo stack and running calculations are not restored.

Run the app from a writable installation folder. Disk images, source executables,
channel changes and Windows/Linux retain the manual download workflow. Released
0.2.7 and earlier still need manual installation to acquire this new updater.

## Measurement discovery and preview

Folder imports discover the extensions handled by the shared measurement reader,
including KEK `.qd`, EX3, SPEC/FIO, HDF5/NeXus, Athena, Larix, XTUNES, numbered
scans and compressed measurements. Extensionless and JSON sources are also
discovered when a bounded content check recognizes measurement data. An unusual
filename can always be selected individually; the file picker has no extension
restriction. KEK `.qc` condition files are not spectra.

Beamline files and containers open in the plotted scan/signal preview. Accepted
signals remove the source from pending import and restore workspace keyboard
focus. Plain text and XDI retain batch recipes. Projects containing these
materialized imports reopen their explicit source list and saved groups, avoiding
duplicate groups from a folder rescan.

The reader still requires explicit choices for missing units, ambiguous signals
and HDF5 dataset mappings. Discovery does not guarantee conversion of every
layout or every dataset in a container. See the
[import guide](https://rexafs.com/docs/desktop/import/) for extensions and limits.

## Compatibility and validation

Public Rust, Python and TypeScript signatures, numerical defaults and the format-1
project schema are unchanged. The coordinated release includes the Rust crate,
four ABI3 Python wheels and source distribution, npm/WebAssembly package and six
desktop targets. Windows and Linux remain previews with their existing limits.

Local tests exercise 203 readable original fixtures and 322 valid GUI signal
previews against the core conversions. Original measurement files, attribution
and package exclusions are unchanged. Release publication requires the complete
GitHub build of the immutable version tag; local tests alone do not qualify it.
