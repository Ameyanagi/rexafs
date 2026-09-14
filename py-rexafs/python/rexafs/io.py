"""Measurement readers and explicit conversion to unprocessed spectra."""

from os import PathLike, fspath
from sys import maxsize

from . import _core
from ._core import Spectrum


def read_qas_transmission(path: str | PathLike[str]) -> Spectrum:
    """Read a whitespace-delimited QAS transmission scan into a new Spectrum.

    The first three columns are energy in eV, incident intensity I0 and
    transmitted intensity It; additional columns are ignored and # starts
    a comment. The reader computes mu = ln(I0 / It), where ln is the natural
    logarithm and the intensities have matching units. Positive intensities
    give dimensionless optical depth, without dividing by sample thickness.
    This reader is unsuitable for files that already contain absorption mu.

    path accepts a filename string or pathlib.Path. The returned spectrum
    owns its data and is unprocessed: call .fft() to run the default pipeline.
    Missing files, unreadable data or fewer than three columns raise
    RuntimeError. The reader sorts energy and its calculated mu together
    into increasing order when needed, but retains duplicate energy rows.
    It does not enforce positive intensities or finite ratios: check both
    measured intensities before taking the ratio. Later processing rejects
    non-finite data; duplicate energies may require cleanup for the selected
    numerical stage. This permissive reader differs from the strict array
    constructor, which rejects unordered or duplicate energy values.

    For the transmission equation and its physical assumptions, see
    [Newville, Fundamentals of XAFS, section 4](https://docs.xrayabsorption.org/tutorials/XAFS_Fundamentals.pdf)."""
    return _core.read_qas_transmission(fspath(path))


from typing import Literal, TypeAlias, TypedDict

ColumnSelector: TypeAlias = int | str
"""Zero-based column index or exact, case-sensitive name; duplicate names require indices."""

class EnergyConversion(TypedDict, total=False):
    """Axis conversion: ev, kev, offset_ev, or bragg (crystal spacing in Å).

    offset_ev adds a finite energy origin in eV to each source value. FDMNES
    detection selects its declared E_edge; source arrays remain unchanged.
    bragg requires positive d_spacing and degrees_per_unit.
    """
    kind: Literal["ev", "kev", "offset_ev", "bragg"]
    """Axis convention: ev, kev, offset_ev or bragg. Required at runtime."""
    offset_ev: float
    """Finite energy origin in eV; required for offset_ev. Zero leaves eV unchanged."""
    d_spacing: float
    """Positive crystal-plane spacing in angstroms; required for bragg."""
    degrees_per_unit: float
    """Positive multiplier from source axis to degrees; required for bragg."""

class SignalConversion(TypedDict, total=False):
    """Names or zero-based indices for direct, transmission or detector/monitor ratio roles."""
    kind: Literal["direct", "transmission", "ratio"]
    """Arithmetic: direct, transmission or ratio. Required at runtime."""
    column: ColumnSelector
    """Exact name or zero-based stored-signal column for direct."""
    incident: ColumnSelector
    """Exact name or zero-based incident monitor for transmission or ratio."""
    transmitted: ColumnSelector
    """Exact name or zero-based transmitted intensity for transmission."""
    detectors: list[ColumnSelector]
    """Nonempty list of distinct detector names or indices for ratio."""

class SpectrumMapping(TypedDict):
    """Explicit axis and detector mapping; energy converts to eV without inferred corrections."""
    energy_column: ColumnSelector
    """Exact source axis name or zero-based index. Names must be unique within the scan."""
    energy: EnergyConversion
    """Declared energy unit or explicit Bragg calibration."""
    signal: SignalConversion
    """Detector arithmetic; no corrections are inferred."""

class SignalCandidate(TypedDict):
    """Header-supported signal label and conversion; multiple choices require selection."""
    name: str
    """Human-readable signal label."""
    mapping: SpectrumMapping
    """Complete conversion supported by source metadata."""

class MeasurementColumn(TypedDict):
    """Original channel: source name, optional units and samples; None marks nonfinite raw cells."""
    name: str
    """Original label or generated column_N name."""
    units: str | None
    """Original unit declaration, or None when undeclared."""
    values: list[float | None]
    """Owned samples in source order; None denotes nonfinite cells in snapshots."""

class MeasurementScan(TypedDict):
    """One original scan with headers, metadata, columns, signal choices and diagnostics."""
    id: str
    """Source record identifier; duplicate display names retain distinct identifiers."""
    label: str
    """Display label, without inferred scientific meaning."""
    columns: list[MeasurementColumn]
    """Original channel order; columns have equal lengths."""
    header: str
    """Original text header; XTUNES retains the complete record text."""
    metadata: dict[str, str]
    """Extracted source metadata; XTUNES ordered_parameters contains ordered JSON triples of section, key and value."""
    signals: list[SignalCandidate]
    """Suggested mappings; zero or multiple choices require explicit selection."""
    warnings: list[str]
    """Unit assumptions, channel ambiguity and format limitations."""

class MeasurementDataset(TypedDict):
    """Numeric dataset or saved Larix/XTUNES array with original dimensions and row-major values; None marks nonfinite cells."""
    path: str
    """HDF5 path, XTUNES table path, or Larix /symbol/attribute path; Larix escapes ~ and / as ~0 and ~1."""
    shape: list[int]
    """Original dimensions in row-major order; an empty list denotes a scalar."""
    values: list[float | None]
    """Owned row-major values; None represents nonfinite source cells."""
    imaginary: list[float | None] | None
    """Imaginary components matching values and shape for complex Larix arrays; None for real arrays. Values contains the real components."""
    attributes: dict[str, str]
    """Source attributes and quantity descriptions. Larix retains exact numeric bytes in larix.bytes_base64 with NumPy dtype in larix.dtype, even when f64 views round large integers."""

class MeasurementDocument(TypedDict):
    """Independent snapshot of format, scans, saved arrays, session metadata and container diagnostics."""
    format: str
    """Content-detected format family."""
    scans: list[MeasurementScan]
    """All recovered acquisition scans and project records."""
    datasets: list[MeasurementDataset]
    """HDF5 arrays and archived Larix/XTUNES results, including independent grids and complex components."""
    metadata: dict[str, str]
    """Container provenance. Larix session_text, command_history and symbol_order keys are prefixed larix.; commands and saved Python objects remain inert text."""
    warnings: list[str]
    """Container and encoding diagnostics, including partial HDF5 recovery."""


def _selection_json(mapping, energy, mu, i0, it, iff, energy_unit):
    """Translate convenience keywords; Rust resolves selectors and validates arithmetic."""
    import json
    if all(v is None for v in (energy, mu, i0, it, iff, energy_unit)):
        return None if mapping is None else json.dumps(mapping, allow_nan=False)
    if mapping is not None:
        raise ValueError("Use mapping or column keywords, not both")
    if energy is None or sum(v is not None for v in (mu, it, iff)) != 1:
        raise ValueError("Specify energy and exactly one of mu, it, or iff")
    if mu is not None:
        if i0 is not None:
            raise ValueError("A direct mu column cannot be combined with i0")
        signal = {"kind": "direct", "column": mu}
    else:
        if i0 is None:
            raise ValueError("Transmission and fluorescence require i0")
        signal = ({"kind": "transmission", "incident": i0, "transmitted": it}
                  if it is not None else
                  {"kind": "ratio", "incident": i0,
                   "detectors": list(iff) if isinstance(iff, (list, tuple)) else [iff]})
    if energy_unit not in (None, "eV", "keV"):
        raise ValueError("energy_unit must be eV or keV")
    return json.dumps({"energy_column": energy,
                       "energy": None if energy_unit is None else {"kind": energy_unit.lower()},
                       "signal": signal}, allow_nan=False)

class Measurement:
    """Owned, content-detected measurement document (since 0.2.6).

    Use read_measurement(path) or parse_measurement(data). Reading retains scans,
    headers, channel units and dataset shapes, including Larix 1.0 sessions. It does not normalize,
    correct detectors or modify files. Conversion is a separate operation.
    """

    def __init__(self, data: bytes) -> None:
        """Parse bytes through the Rust reader; invalid files raise ValueError."""
        self._native = _core.Measurement(data)

    @property
    def document(self) -> MeasurementDocument:
        """Independent metadata/array snapshot, including scans and HDF5 datasets.

        Nonfinite original cells are represented by None in this JSON-compatible
        snapshot. The native document retains them; arrays() rejects selected
        nonfinite values. Editing this copy does not change the native document.
        """
        import json
        return json.loads(self._native.document_json())

    def select_datasets(self, paths: list[str]) -> int:
        """Append a scan from explicit real dataset paths; return its zero-based index.

        Select at least two distinct one-dimensional, nonempty datasets of equal
        length. Complex arrays are rejected. Paths determine column order; arrays are copied. This supports
        axis/detector vectors in different groups. Images require reduction
        outside the reader. Invalid paths/shapes raise ValueError. No spectrum
        processing occurs, and original scans remain unchanged.
        """
        return self._native.select_datasets(paths)

    def arrays(self, scan: int = 0, mapping: SpectrumMapping | None = None, *,
               energy: ColumnSelector | None = None, mu: ColumnSelector | None = None,
               i0: ColumnSelector | None = None, it: ColumnSelector | None = None,
               iff: ColumnSelector | list[ColumnSelector] | tuple[ColumnSelector, ...] | None = None,
               energy_unit: Literal["eV", "keV"] | None = None):
        """Return independent NumPy float64 energy (eV) and signal arrays.

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
        """
        if not isinstance(scan, int) or isinstance(scan, bool) or scan < 0 or scan > maxsize:
            raise IndexError("Scan index out of range")
        selection = _selection_json(mapping, energy, mu, i0, it, iff, energy_unit)
        return self._native.arrays(scan, selection)

    def spectrum(self, scan: int = 0, mapping: SpectrumMapping | None = None, *,
               energy: ColumnSelector | None = None, mu: ColumnSelector | None = None,
               i0: ColumnSelector | None = None, it: ColumnSelector | None = None,
               iff: ColumnSelector | list[ColumnSelector] | tuple[ColumnSelector, ...] | None = None,
               energy_unit: Literal["eV", "keV"] | None = None) -> Spectrum:
        """Create an owned, unprocessed Spectrum from the selected scan.

        Accepts the same column-name/index keywords and energy_unit override as
        arrays(), or the existing mapping dictionary. These options are mutually
        exclusive. Uses arrays() conversion rules, then sorts energy and signal together.
        Duplicate energies remain and may require cleanup before processing.
        No normalization/background/FFT prerequisites run; existing objects and
        source files are unchanged. Selection and conversion errors are the
        same as arrays(). This reader differs from the strict
        Spectrum array constructor, which requires increasing, unique energy.
        """
        if not isinstance(scan, int) or isinstance(scan, bool) or scan < 0 or scan > maxsize:
            raise IndexError("Scan index out of range")
        selection = _selection_json(mapping, energy, mu, i0, it, iff, energy_unit)
        return self._native.spectrum(scan, selection)


def parse_measurement(data: bytes | str) -> Measurement:
    """Read text or binary measurement content without filesystem/network access.

    Universal Rust reader, added in 0.2.6: XDI, beamline text, CSV, historical binary,
    Athena (Perl/JSON, optionally gzip), Larix 1.0 sessions, XTUNES and HDF5. Strings are encoded as UTF-8.
    Returns owned scans, original metadata, signal choices and saved arrays;
    inspect .document and select .arrays() or .spectrum() to convert a scan.
    Ambiguous detector roles and image-only datasets require explicit selection
    or reduction; reading them does not imply a ready-to-process spectrum.
    Input and expanded gzip sizes are limited to 256 MiB each. Gzip requires one
    complete member with no trailing data. Invalid containers
    and malformed rows raise ValueError. No input data or settings are modified.
    """
    return Measurement(data.encode('utf-8') if isinstance(data, str) else data)


def read_measurement(path: str | PathLike[str]) -> Measurement:
    """Read a local measurement file through the universal reader added in 0.2.6.

    Accepts a filename or pathlib.Path; content determines the format, not the
    extension. For an unambiguous first scan, use read_measurement(path).arrays()
    for energy in eV and signal, or .spectrum() for an unprocessed Spectrum.
    Returns an owned Measurement; inspect .document, then select a
    scan and mapping with .arrays() or .spectrum(). No processing, file writes
    or network requests occur. Input and expanded gzip are limited to 256 MiB
    each. File errors raise OSError, and parsing/conversion errors raise
    ValueError. The document records ambiguous channels and HDF5 limitations.
    """
    with open(path, 'rb') as stream:
        data = stream.read(256 * 1024 * 1024 + 1)
    return Measurement(data)
