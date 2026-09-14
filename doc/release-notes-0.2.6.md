# rexafs 0.2.6

Published on 14 September 2026: [desktop downloads](https://github.com/Ameyanagi/rexafs/releases/tag/v0.2.6),
[crates.io](https://crates.io/crates/rexafs/0.2.6),
[PyPI](https://pypi.org/project/rexafs/0.2.6/) and
[npm](https://www.npmjs.com/package/rexafs/v/0.2.6).
The [qualification record](validation/2026-09-14-release-0.2.6/review.md) tracks
the immutable source tag, GitHub builds, signing and registry verification.

## Measurement import

- A shared content-detected reader serves Rust, Python, TypeScript/WebAssembly,
  the desktop and the browser workspace. It reads beamline text, numeric tables,
  Athena projects, Larix sessions, XTUNES saves and numeric HDF5 datasets.
- Column selection accepts exact names, zero-based indices, or both. Rust adds
  `SpectrumSelection` helpers; Python offers column keywords; TypeScript offers
  an `arrays(options)` overload. Existing numeric mappings remain available.
- The desktop shows a plotted preview before import, with independent signal
  checkboxes, detector formulas and expandable source information. Transmission,
  fluorescence and reference signals can be imported together in one undoable
  action. The Data view opens on the original signal scale.
- Energy conversions preserve declared eV/keV units, relative-energy calibration
  and Bragg-angle metadata. Ambiguous units or detector roles require an explicit
  choice. Reading does not normalize spectra or repeat beamline corrections.

The retained collection covers 243 unique beamline/project payloads: 203 parse,
including partial recoveries and tables requiring manual interpretation, while
40 have explicit rejection expectations. Six XTUNES files and 15 valid Larix
sessions are checked separately. These counts describe fixture-backed coverage,
not support for every format produced at a facility. HDF5 linked-group recovery
and detector images still have documented limits. See the
[reader guide](measurement-reader.md) for formats, examples and qualifications.

## Packaging and documentation

- Four `cp310-abi3` Python wheels share one platform build across GIL-enabled
  CPython 3.10–3.14. Release CI installs those exact wheel bytes on all supported
  interpreter/platform combinations with minimum and latest compatible NumPy.
  Free-threaded Python remains unqualified. The 0.2.5 assets are unchanged.
- Duplicate fixtures share one canonical collection while retaining source URLs,
  original measurements, licenses and historical checksums. Fixture bundles are
  excluded from crates.io, Python and npm packages. Windows checkouts preserve
  their original line endings.
- Guides, editor help and import screenshots describe the new workflow. Earlier
  screenshots retain their capture versions. Website deployment and its ReFEFF
  browser preview are separate from the versioned packages.

The numerical defaults and format-1 project format are unchanged. The desktop
continues to target macOS ARM64/Intel, Linux x64/ARM64 and Windows x64/ARM64.
Windows and Linux remain previews; Windows ARM64 uses the x64 FEFF10 helper
through Windows 11 emulation, while rexafs and ReFEFF run natively.
