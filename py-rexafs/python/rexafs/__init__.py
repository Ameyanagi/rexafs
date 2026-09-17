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
