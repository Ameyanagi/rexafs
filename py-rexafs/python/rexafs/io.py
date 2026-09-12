"""Native readers that return spectra ready for the processing pipeline."""

from os import PathLike, fspath

from . import _core
from ._core import Spectrum


def read_qas_transmission(path: str | PathLike[str]) -> Spectrum:
    """Read a QAS transmission scan as energy (eV) and mu = ln(I0 / It).

    I0 and It are positive incident/transmitted intensities in matching units.
    ln is the natural logarithm, so mu is dimensionless optical depth.
    Accepts a filename or pathlib.Path. Returns an unprocessed Spectrum;
    call .fft() to run the default pipeline. File and parse failures raise
    RuntimeError. The reader uses the native QAS column conventions.
    """
    return _core.read_qas_transmission(fspath(path))
