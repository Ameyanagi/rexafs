---
title: "Import XDI data"
description: "Interpret XAS Data Interchange metadata, columns and axis units."
audience: user
---

Implemented against the XraySpectroscopy working group's [XDI 1.0 draft
specification](https://github.com/XraySpectroscopy/XAS-Data-Interchange/blob/master/specification/spec.md)
and [metadata dictionary](https://github.com/XraySpectroscopy/XAS-Data-Interchange/blob/master/specification/dictionary.md),
reviewed on 2026-09-05. This is XAS Data Interchange, not the unrelated OASIS XDI format.

Import `.xdi` and `.XDI` files directly or through folder scans. The
`# XDI/1.x` signature also identifies XDI in `.dat` and other supported text files.
A `.xdi` file without that signature is an import error. Project reopen and batch
calculations use the same reader.

The Import panel shows the version, sample, absorber, edge and axis units. Expand
it for measurement details, comments, original values and column assignments.
The preview keeps original units; processing receives energy in eV.
`Scan.edge_energy` is metadata and does not override the processing $E_0$.

## Interpretation

- `Column.N` declarations determine column names and units. The optional label
  line is not required; conflicting labels and inconsistent row widths are errors.
- Field names are case insensitive, duplicate fields use their last value, and
  unknown extension fields are retained. User comments are kept separately from
  metadata, including blank lines and interior whitespace. Application/version
  tokens keep their order. LF, CRLF and CR line endings and a UTF-8 BOM are accepted.
- Energy in eV or keV is converted to eV. Angle in degrees or radians uses
  first-order Bragg diffraction with `Mono.d_spacing` in Å. Unsupported axes or
  missing conversion information cause an error rather than an assumed scale.
- Auto import prefers precomputed sample μ (`mutrans`, `mufluor`, `normtrans`,
  `normfluor`), then sample intensities, then reference signals. Transmission uses
  ln(I0/itrans); fluorescence uses ifluor/I0. Reference mode accepts `murefer` /
  `normrefer`, or computes ln(itrans/irefer). Explicit GUI column/mode assignments
  remain available. Precomputed μ is never logarithmically transformed again.
- Invalid or non-finite numeric tokens and damaged rows report errors; they are
  not silently skipped. Missing absorber/edge descriptions and malformed metadata
  fields produce warnings. The reader is not a full metadata-dictionary validator.

Numeric-row checks precede detector arithmetic. A finite input can still produce
an undefined logarithm or division, for example with zero transmitted intensity.
The desktop excludes non-finite calculated signals; review the affected point
count and source lines in import diagnostics before accepting the mapping.
Conversion sorts energy/μ pairs by increasing energy without merging duplicates.
See the [shared desktop signal
conversion](https://github.com/Ameyanagi/rexafs/blob/v0.2.4/crates/rexafs-gui/src/params.rs#L913).

Pixel and motor-step energy calibrations, χ(k)/χ(R) import into the μ(E)
processing pipeline, and XDI export are not implemented. The low-level reader can
retain those tables, but conversion to an energy spectrum rejects unsupported axes.
