---
title: "Python · io.Measurement"
description: "io.Measurement signatures, defaults and API explanations."
audience: user
pagefind: true
---

**Stable 0.2.12.** These signatures match the released Python package. Explanations are maintained in the source docstrings and reviewed against this release.

[Installation and version guide](/docs/reference/) · [Python tutorial](/docs/libraries/python/)

[Declaration source](https://github.com/Ameyanagi/rexafs/blob/v0.2.12/py-rexafs/python/rexafs/io.pyi) · [Docstring source](https://github.com/Ameyanagi/rexafs/blob/main/py-rexafs/python/rexafs/io.pyi)

Owned, content-detected measurement document (since 0.2.6).

Use read_measurement(path) or parse_measurement(data). Reading retains scans,
headers, channel units and dataset shapes, including Larix 1.0 sessions. It does not normalize,
correct detectors or modify files. Conversion is a separate operation.

## Measurement

```python
Measurement(data: bytes)
```

Parse bytes through the Rust reader; invalid files raise ValueError.

## document

```python
document(self) -> MeasurementDocument
```

Independent metadata/array snapshot, including scans and HDF5 datasets.

Nonfinite original cells are represented by None in this JSON-compatible
snapshot. The native document retains them; arrays() rejects selected
nonfinite values. Editing this copy does not change the native document.

## select_datasets

```python
select_datasets(self, paths: list[str]) -> int
```

Append a scan from explicit real dataset paths; return its zero-based index.

Select at least two distinct one-dimensional, nonempty datasets of equal
length. Complex arrays are rejected. Paths determine column order; arrays are copied. This supports
axis/detector vectors in different groups. Images require reduction
outside the reader. Invalid paths/shapes raise ValueError. No spectrum
processing occurs, and original scans remain unchanged.

## arrays

```python
arrays(self, scan: int=0, mapping: SpectrumMapping | None=None, *, energy: ColumnSelector | None=None, mu: ColumnSelector | None=None, i0: ColumnSelector | None=None, it: ColumnSelector | None=None, iff: ColumnSelector | list[ColumnSelector] | tuple[ColumnSelector, ...] | None=None, energy_unit: Literal['eV', 'keV'] | None=None) -> tuple[NDArray[np.float64], NDArray[np.float64]]
```

Return independent NumPy float64 energy (eV) and signal arrays.

Column arguments accept exact, case-sensitive names or zero-based indices.
For example, arrays(energy="energy", i0="I0", it="It") selects transmission;
mu selects stored absorption and iff selects one detector or an explicit list.
Specify energy and exactly one of mu, it, or iff; it/iff also require i0.
Keywords cannot be combined with mapping. Missing or duplicate names fail.
Omit energy_unit to retain detected axis calibration/declared units; set
"eV" or "keV" to override. Unknown units require an explicit choice.
The mapping dictionary also accepts names and indices in any combination.

scan is zero-based. Recommended mapping=None uses the sole detected
signal; zero or multiple choices require a mapping from document or an
explicit energy_column/energy/signal dictionary. Transmission computes
ln(incident/transmitted), direct copies a stored signal, and ratio sums
selected detectors then divides by the incident monitor. Transmission
requires nonzero same-sign intensities; ratio requires a nonzero monitor.
No gain, dark-current, dead-time or self-absorption correction is inferred.
Source order and duplicate energies remain. Invalid roles, units, Bragg
calibration or nonfinite selected values raise ValueError; an invalid
scan raises IndexError. No processing runs or input changes occur.

## spectrum

```python
spectrum(self, scan: int=0, mapping: SpectrumMapping | None=None, *, energy: ColumnSelector | None=None, mu: ColumnSelector | None=None, i0: ColumnSelector | None=None, it: ColumnSelector | None=None, iff: ColumnSelector | list[ColumnSelector] | tuple[ColumnSelector, ...] | None=None, energy_unit: Literal['eV', 'keV'] | None=None) -> Spectrum
```

Create an owned, unprocessed Spectrum from the selected scan.

Accepts the same column-name/index keywords and energy_unit override as
arrays(), or the existing mapping dictionary. These options are mutually
exclusive. Uses arrays() conversion rules, then sorts energy and signal together.
Duplicate energies remain and may require cleanup before processing.
No normalization/background/FFT prerequisites run; existing objects and
source files are unchanged. Selection and conversion errors are the
same as arrays(). This reader differs from the strict
Spectrum array constructor, which requires increasing, unique energy.
