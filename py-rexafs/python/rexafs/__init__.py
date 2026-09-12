"""Rust spectrum processing with NumPy results. Energy is in eV.

Start with Spectrum(energy, mu).fft(); configure stages with keyword-only
PrePostEdge, AUTOBK, XrayFFTF and XrayFFTR settings. See member docstrings
for units, defaults and invalidation behavior.
"""

from typing import Literal

from . import io
from ._core import (
    AUTOBK,
    BackgroundMethod,
    NormalizationMethod,
    PrePostEdge,
    Spectrum,
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
    "XrayFFTF",
    "XrayFFTR",
    "__version__",
    "io",
]
