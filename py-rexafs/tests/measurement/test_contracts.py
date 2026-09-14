"""Installed Python reader contracts checks."""
import numpy as np
import pytest
from rexafs.io import parse_measurement, SpectrumMapping

def test_units_owned_arrays_and_spectrum():
    data = parse_measurement('energy (keV),mu\n7.2,2\n7.1,1\n')
    (energy, mu) = data.arrays()
    np.testing.assert_array_equal(energy, [7200, 7100])
    energy[0] = 0
    assert data.arrays()[0][0] == 7200
    assert data.spectrum().norm() is None
    for scan in [-1, 99, 2 ** 100]:
        with pytest.raises(IndexError):
            data.arrays(scan)

def test_signal_choice_and_explicit_mapping():
    data = parse_measurement('# energy i0 it if\n7100 10 2 3\n7101 20 4 8\n')
    with pytest.raises(ValueError, match='mapping'):
        data.arrays()
    mapping: SpectrumMapping = data.document['scans'][0]['signals'][1]['mapping']
    np.testing.assert_allclose(data.arrays(mapping=mapping)[1], [0.3, 0.4])
    with pytest.raises(ValueError):
        parse_measurement('7100 1\n7101 bad\n')
