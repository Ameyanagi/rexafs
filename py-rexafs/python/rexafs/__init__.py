"""Rust spectrum processing with NumPy results. Energy is in eV.

Start with Spectrum(energy, mu).fft(); configure stages with keyword-only
PrePostEdge, AUTOBK, XrayFFTF and XrayFFTR settings. See member docstrings
for units, defaults and invalidation behavior.
"""

from typing import Literal, TypedDict

from . import io
from ._core import (
    AUTOBK,
    BackgroundMethod,
    NormalizationMethod,
    PrePostEdge,
    Spectrum,
    MeasurementResult,
    MBack,
    FluorescenceCorrection,
    FluorescenceCorrectionResult,
    Wavelet,
    WaveletMap,
    WaveletRegionValue,
    MbackErfc,
    MbackResult,
    PeakFit,
    PeakFitResult,
    PeakContribution,
    PeakFitOutcome,
    XrayFFTF,
    XrayFFTR,
    __version__,
)

FFTGrid = Literal["Input", "Larch"]
FTWindow = Literal[
    "Hanning", "Parzen", "Welch", "Gaussian", "Sine", "KaiserBessel", "FHanning"
]
AUTOBKSolver = Literal["TrustRegionDogLeg", "LegacyLm", "LinearDirect"]
AUTOBKClampScalePolicy = Literal["FixedPenalty", "Fixed", "TwoPass"]
AbsorptionMode = Literal["unknown", "transmission", "fluorescence"]

__all__ = [
    "AUTOBK",
    "AUTOBKClampScalePolicy",
    "AUTOBKSolver",
    "BackgroundMethod",
    "FFTGrid",
    "FTWindow",
    "NormalizationMethod",
    "PrePostEdge",
    "Spectrum",
    "MeasurementResult",
    "MBack",
    "FluorescenceCorrection",
    "FluorescenceCorrectionResult",
    "FluorescenceInternalNormalization",
    "AbsorptionMode",
    "Wavelet",
    "WaveletMap",
    "WaveletSize",
    "WaveletRegionValue",
    "MbackErfc",
    "MbackResult",
    "AtomicDataIdentity",
    "AtomicReference",
    "PeakFit",
    "PeakFitResult",
    "PeakContribution",
    "PeakFitOutcome",
    "XrayFFTF",
    "XrayFFTR",
    "__version__",
    "io",
]


class AtomicDataIdentity(TypedDict):
    """Exact offline provider, database version and decoded-data checksum."""
    provider: str
    data_version: str
    data_sha256: str

class AtomicReference(TypedDict):
    """Exact atomic dataset and numerical table identity."""
    data: AtomicDataIdentity
    table: Literal["ChantlerF2LogLogV1", "ElamTotalV1", "ElamTransitionsV1"]


class WaveletSize(TypedDict):
    """Checked wavelet dimensions and buffer estimate, excluding copies/scratch."""
    k_points: int
    """Number of prepared k columns, including padding."""
    r_points: int
    """Number of positive R rows."""
    nfft: int
    """Internal FFT length, in samples."""
    cells: int
    """Number of complex map cells."""
    bytes: int
    """Estimated scientific buffer bytes; input copies and scratch add overhead."""


class FluorescenceInternalNormalization(TypedDict):
    """Internal fit of original mu, distinct from final normalization (unreleased).
    All arrays are independent Python lists on the original energy grid."""
    e0: float
    """Measured edge energy in eV."""
    pre_edge: list[float]
    """Resolved pre-edge eV offsets from E0, [start, end]."""
    post_edge: list[float]
    """Resolved post-edge eV offsets from E0, [start, end]."""
    degree: int
    """Internal post-edge polynomial degree; pre-edge is linear."""
    edge_step: float
    """Positive fitted jump in original absorption units, before numerical flooring."""
    pre_curve: list[float]
    """Pre-edge line in original absorption units."""
    post_curve: list[float]
    """Pre-edge line plus post-edge polynomial, in original absorption units."""
    norm: list[float]
    """Dimensionless internal n0 used in alpha+1-n0."""
