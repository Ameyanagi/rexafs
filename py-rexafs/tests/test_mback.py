"""Installed native MBACK agrees with pinned synthetic Larch reference cases."""
import json
from pathlib import Path
import unittest
import numpy as np
from rexafs import MBack, MbackErfc, Spectrum

REFERENCE = json.loads((Path(__file__).resolve().parents[2]/
    'crates/rexafs/tests/fixtures/analysis/mback/larch-reference.json').read_text())


def model(case):
    erfc = MbackErfc('Ka1', width=(500,1500), amplitude=(0,10)) if case['erfc'] else None
    return MBack('Cu','K',e0=case['e0'],pre_edge=tuple(case['pre_edge']),
                 post_edge=tuple(case['post_edge']),degree=case['degree'],erfc=erfc)


class MbackTests(unittest.TestCase):
    def test_reference_and_spectrum_pipeline_share_the_native_result(self):
        for case in REFERENCE['cases']:
            with self.subTest(erfc=case['erfc']):
                settings=model(case)
                initial=settings.to_json()
                energy=np.array(case['energy']); mu=np.array(case['mu'])
                result=settings.fit(energy,mu)
                for key in ['norm','fpp','background','pre_curve','post_curve']:
                    np.testing.assert_allclose(getattr(result,key),case[key],rtol=0,atol=1e-6)
                self.assertAlmostEqual(result.objective,case['objective'],delta=1e-10)
                self.assertEqual(settings.to_json(),initial)
                self.assertEqual(result.reference['data']['data_version'],'9.2')
                spectrum=Spectrum(energy,mu).set_normalization_method(settings).normalize()
                saved=spectrum.mback_result()
                np.testing.assert_array_equal(spectrum.norm(),result.norm)
                np.testing.assert_array_equal(saved.flat,result.flat)
                snapshot=saved.to_json();array=saved.norm;array[0]=999
                self.assertNotEqual(saved.norm[0],999)
                self.assertEqual(saved.to_json(),snapshot)
                replay=saved.definition.fit(energy,mu)
                np.testing.assert_array_equal(replay.norm,saved.norm)
                spectrum.set_e0(case['e0']+1)
                self.assertIsNone(spectrum.mback_result())
                self.assertEqual(saved.to_json(),snapshot)
                np.testing.assert_array_equal(mu,case['mu'])

    def test_ranges_missing_reference_and_shape_errors_are_explicit(self):
        case=REFERENCE['cases'][0]
        with self.assertRaisesRegex(ValueError,'outside the scan'):
            MBack('Cu','K',e0=case['e0'],pre_edge=(-5000,-50)).fit(case['energy'],case['mu'])
        with self.assertRaisesRegex(ValueError,'one-dimensional'):
            model(case).fit([[1,2]],case['mu'])
        result=model(case).fit(case['energy'],case['mu'])
        definition=json.loads(result.definition.to_json())
        definition['options']['reference']['data']['data_sha256']='not-available'
        with self.assertRaisesRegex(ValueError,'archived atomic reference'):
            MBack.from_json(json.dumps(definition)).fit(case['energy'],case['mu'])

    def test_atomic_notices_are_present_in_the_installed_package(self):
        import rexafs
        notices=Path(rexafs.__file__).parent/'licenses/atomic'
        self.assertIn('Some of', (notices/'XrayDB-LICENSE.txt').read_text())
        self.assertEqual(json.loads((notices/'provenance.json').read_text())['embedded_database']['version'],'0.4.1')
