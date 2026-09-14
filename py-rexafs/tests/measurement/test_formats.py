"""Installed Python reader formats checks."""
import numpy as np
import pytest
from rexafs.io import read_measurement, SpectrumMapping
from .fixtures import FIXTURES

def test_original_bmm_reference_and_hdf5():
    data = read_measurement(FIXTURES / 'samples/nsls-ii/6-bm-bmm/bmm-standards/Fe-K-IronFoil.xdi')
    (e, mu) = data.arrays()
    assert len(e) == 393 and e[0] == 6912.001 and (mu[0] == 1.981854)
    image = read_measurement(FIXTURES / 'samples/max-iv/balder-unconfirmed/parseq-xas/scan-24212_eiger_streaming.h5')
    assert image.document['format'] == 'hdf5'
    assert not image.document['scans']
    assert image.document['datasets']

def test_explicit_hdf5_dataset_paths_and_shapes():
    data = read_measurement(FIXTURES / 'samples/max-iv/balder-unconfirmed/parseq-xas/20230318.h5')
    with pytest.raises(ValueError, match='matching'):
        data.select_datasets(['/entry24212/measurement/mono1_energy_acs', '/entry24213/measurement/albaem-01_ch1'])
    index = data.select_datasets(['/entry24212/measurement/mono1_energy_acs', '/entry24212/measurement/albaem-01_ch1'])
    mapping: SpectrumMapping = {'energy_column': 0, 'energy': {'kind': 'ev'}, 'signal': {'kind': 'direct', 'column': 1}}
    (e, signal) = data.arrays(index, mapping)
    assert len(e) == 601 and e[0] == 16065 and (signal[0] == 2.0128017625796508e-06)

def test_larch_relative_energy_and_demeter_independent_samples():
    data = read_measurement(FIXTURES / 'samples/unspecified/fdmnes/xraylarch/FDMNES_2022_Mo2C_out.dat')
    assert data.document['scans'][0]['columns'][0]['values'][0] == -25
    assert data.document['scans'][0]['signals'][0]['mapping']['energy'] == {'kind': 'offset_ev', 'offset_ev': 20000.0}
    (e, signal) = data.arrays()
    assert len(e) == 559 and e[0] == 19975 and (signal[0] == 0.00011943264)
    assert data.spectrum().norm() is None
    data = read_measurement(FIXTURES / 'samples/nsls/x23a2/demeter/re4chan.000')
    with pytest.raises(ValueError, match='mapping'):
        data.arrays()
    assert len(data.document['scans'][0]['signals']) == 4
    mapping = data.document['scans'][0]['signals'][1]['mapping']
    (e, signal) = data.arrays(mapping=mapping)
    assert len(e) == 387 and abs(signal[0] - np.log(14600.25 / 5182.5)) < 1e-13

def test_larch_legacy_athena_metadata_and_optional_arrays():
    data = read_measurement(FIXTURES / 'samples/unspecified/athena-legacy/xraylarch/Br.prj')
    assert len(data.document['scans']) == 5
    assert len(data.arrays()[0]) == 545
    assert "'name' => {'name' => {}}" in data.document['scans'][0]['header']
    data = read_measurement(FIXTURES / 'samples/unspecified/athena-legacy/xraylarch/AsXAFS_a.prj')
    assert len(data.document['scans']) == 46
    assert any(('(undef)' in s['metadata']['athena.extra'] for s in data.document['scans']))
