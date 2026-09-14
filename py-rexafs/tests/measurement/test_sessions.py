"""Installed Python reader sessions checks."""
import numpy as np
import pytest
from rexafs.io import read_measurement
from .fixtures import FIXTURES, LARIX

def test_xtunes_project_order_and_saved_tables():
    data = read_measurement(FIXTURES.parent / 'sessions/xtunes/mixed-raw-analyzed.xtsp')
    assert data.document['format'] == 'xtunes_project'
    assert len(data.document['scans']) == 2
    assert len(data.arrays(0)[0]) == 818
    (e, mu) = data.arrays(1)
    assert len(e) == 1420 and e[0] == 6606.167326 and (mu[0] == 0.007819)
    assert any((d['path'] == '/record_2/Xi Plot' and d['shape'] == [877, 2] for d in data.document['datasets']))

def test_all_larix_sessions_and_invalid_inputs():
    import json
    manifest = json.loads((LARIX / 'manifest.json').read_text())
    for entry in manifest['files']:
        if entry['category'] == 'invalid':
            with pytest.raises(ValueError):
                read_measurement(LARIX / entry['path'])
            continue
        data = read_measurement(LARIX / entry['path'])
        doc = data.document
        assert doc['format'] == 'larix'
        automatic = [s['metadata']['larix.symbol'] for s in doc['scans'] if len(s['signals']) == 1]
        assert automatic == entry['expected_absorption_groups']
        for (i, scan) in enumerate(doc['scans']):
            if scan['signals']:
                (energy, mu) = data.arrays(i)
                np.testing.assert_array_equal(energy, scan['columns'][0]['values'])
                np.testing.assert_array_equal(mu, scan['columns'][1]['values'])

def test_larix_complex_arrays_and_exact_typed_metadata():
    import base64
    data = read_measurement(LARIX / 'fixtures/valid/two-analyzed.larix')
    assert [len(data.arrays(i)[0]) for i in range(2)] == [818, 1420]
    doc = data.document
    chir = next((d for d in doc['datasets'] if d['path'].endswith('/chir')))
    assert chir['shape'] == [326] and len(chir['imaginary']) == 326
    exact = np.frombuffer(base64.b64decode(chir['attributes']['larix.bytes_base64']), dtype=chir['attributes']['larix.dtype'])
    np.testing.assert_array_equal(exact.real, chir['values'])
    np.testing.assert_array_equal(exact.imag, chir['imaginary'])
    with pytest.raises(ValueError, match='complex'):
        data.select_datasets([chir['path'].replace('/chir', '/r'), chir['path']])
    assert data.spectrum().norm() is None
    data = read_measurement(LARIX / 'fixtures/valid/typed-metadata.larix')
    doc = data.document
    counts = next((d for d in doc['datasets'] if d['path'].endswith('/metadata/counts')))
    exact = np.frombuffer(base64.b64decode(counts['attributes']['larix.bytes_base64']), dtype=counts['attributes']['larix.dtype'])
    assert exact[1] == 9007199254740993
    assert doc['scans'][0]['label'] == 'Fe foil — 日本語 μ(E).dat'
    assert '2 * amplitude' in doc['metadata']['larix.session_text']
    assert 'preserve repeated keys' in doc['metadata']['larix.session_text']
    doc['metadata']['larix.session_text'] = 'changed copy'
    assert data.document['metadata']['larix.session_text'].startswith('##LARIX:')
