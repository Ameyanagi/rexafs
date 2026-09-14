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

def test_named_columns_keywords_and_numeric_compatibility():
    data = parse_measurement('mu,IFF2,It,energy (keV),IFF1,I0\n0.5,4,2,7.1,3,10\n0.6,10,4,7.2,8,20\n')
    energy, mu = data.arrays(energy='energy', i0='I0', it='It')
    np.testing.assert_array_equal(energy, [7100, 7200])
    np.testing.assert_allclose(mu, np.log(5))
    np.testing.assert_array_equal(data.arrays(energy=3, mu='mu')[1], [0.5, 0.6])
    np.testing.assert_array_equal(data.arrays(energy='energy', i0=5, iff=['IFF1', 'IFF2'])[1], [0.7, 0.9])
    np.testing.assert_array_equal(data.arrays(energy=3, i0='I0', iff='IFF1')[1], [0.3, 0.4])
    mapping: SpectrumMapping = {'energy_column': 'energy', 'energy': {'kind': 'kev'},
                                'signal': {'kind': 'transmission', 'incident': 'I0', 'transmitted': 2}}
    np.testing.assert_allclose(data.arrays(mapping=mapping)[1], mu)
    assert data.spectrum(energy='energy', mu='mu').norm() is None
    np.testing.assert_array_equal(data.arrays(energy=3, mu=0, energy_unit='eV')[0], [7.1, 7.2])
    for options in [dict(energy='energy', mu='missing'), dict(energy='energy', mu='MU'),
                    dict(energy='energy', mu='mu', it='It'), dict(energy='energy', it='It'),
                    dict(energy='energy', mu='mu', i0='I0'), dict(mu='mu'),
                    dict(energy='energy', i0='I0', iff=[]), dict(energy='energy', mu=True),
                    dict(energy='energy', mu='mu', energy_unit='watts')]:
        with pytest.raises(ValueError):
            data.arrays(**options)
    with pytest.raises(ValueError, match='not both'):
        data.arrays(mapping=mapping, energy=3, mu=0)
    with pytest.raises(ValueError, match='ambiguous'):
        parse_measurement('energy (eV),mu,mu\n7100,1,2\n').arrays(energy='energy', mu='mu')
    unnamed = parse_measurement('7100 1\n7101 2\n')
    with pytest.raises(ValueError, match='energy_unit'):
        unnamed.arrays(energy=0, mu=1)
    np.testing.assert_array_equal(unnamed.arrays(energy=0, mu=1, energy_unit='eV')[1], [1, 2])
