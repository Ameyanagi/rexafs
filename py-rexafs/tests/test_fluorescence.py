"""Installed correction API: independent references, history and processing domain."""
import json
from pathlib import Path
import unittest
import numpy as np
from rexafs import Spectrum, FluorescenceCorrection, MBack, Wavelet
from rexafs.io import parse_measurement

REFERENCE = json.loads((Path(__file__).resolve().parents[2] /
    'crates/rexafs/tests/fixtures/analysis/fluorescence/larch-reference.json').read_text())


def model(case, **overrides):
    settings = dict(line=case['line'], angles=tuple(case['angles']), family=case['family'],
        e0=case['e0'], pre_edge=tuple(case['pre_edge']), post_edge=tuple(case['post_edge']), degree=case['degree'])
    settings.update(overrides)
    return FluorescenceCorrection(case['formula'], case['element'], 'K', **settings)


class FluorescenceTests(unittest.TestCase):
    def test_all_native_reference_cases_and_independent_replay(self):
        for case in REFERENCE['cases']:
            with self.subTest(formula=case['formula']):
                energy, mu = np.array(case['energy']), np.array(case['mu'])
                definition = model(case)
                settings = definition.to_json()
                result = definition.apply(energy, mu)
                np.testing.assert_allclose(result.corrected_mu, case['corrected_mu'], rtol=0,
                    atol=REFERENCE['tolerances']['curve_absolute'])
                np.testing.assert_allclose(result.internal['norm'], case['internal_norm'], rtol=0,
                    atol=REFERENCE['tolerances']['curve_absolute'])
                self.assertAlmostEqual(result.alpha/case['alpha'], 1., delta=REFERENCE['tolerances']['atomic_relative'])
                self.assertEqual(definition.to_json(), settings)
                saved = result.to_json()
                result.corrected_mu[:] = -999
                result.internal['norm'][:] = [999] * len(energy)
                result.atomic['edge'] = None
                mu[:] = 999
                self.assertEqual(result.to_json(), saved)
                np.testing.assert_array_equal(result.original_mu, case['mu'])
                replay = result.definition.apply(result.energy, result.original_mu)
                np.testing.assert_array_equal(replay.corrected_mu, result.corrected_mu)
                self.assertEqual(replay.atomic, result.atomic)
                restored = FluorescenceCorrection.from_json(result.definition.to_json())
                np.testing.assert_array_equal(restored.apply(case['energy'], case['mu']).factor, result.factor)

    def test_spectrum_history_and_xanes_domain_survive_processing_and_edits(self):
        case = REFERENCE['cases'][0]
        definition = model(case)
        source = Spectrum(case['energy'], case['mu'])
        corrected = source.correct_fluorescence(definition)
        self.assertIsNone(source.fluorescence_correction())
        self.assertIsNone(source.norm())
        self.assertEqual(source.absorption_mode(), 'unknown')
        self.assertEqual(corrected.absorption_mode(), 'fluorescence')
        record = corrected.fluorescence_correction()
        self.assertEqual(record.input_mode, 'unknown')
        self.assertTrue(any('unknown' in warning for warning in record.warnings))
        saved = record.to_json()
        corrected.normalize()
        self.assertIsNotNone(corrected.norm())
        corrected.set_normalization_method(MBack(case['element'], 'K', e0=case['e0'],
            pre_edge=tuple(case['pre_edge']), post_edge=tuple(case['post_edge']))).normalize()
        self.assertIsNotNone(corrected.mback_result())
        self.assertEqual(corrected.fluorescence_correction().to_json(), saved)
        corrected.set_spectrum(case['energy'], np.asarray(case['mu']) * 1.01)
        self.assertIs(corrected.set_absorption_mode('unknown'), corrected)
        self.assertEqual(corrected.fluorescence_correction().to_json(), saved)
        for run in [lambda: corrected.correct_fluorescence(definition), corrected.calc_background,
                    corrected.fft, corrected.ifft, lambda: corrected.wavelet(Wavelet((2, 10)))]:
            with self.assertRaises((ValueError, RuntimeError)): run()
        source.set_absorption_mode('transmission')
        with self.assertRaisesRegex(ValueError, 'transmission'): source.correct_fluorescence(definition)
        with self.assertRaises(ValueError): source.set_absorption_mode('guess')

    def test_geometry_coverage_singularities_and_imported_transmission(self):
        case = REFERENCE['cases'][0]
        for overrides in [dict(angles=(0,45)), dict(line='La1'), dict(pre_edge=(-9999,-30)), dict(degree=8)]:
            with self.assertRaises(ValueError): model(case, **overrides).apply(case['energy'],case['mu'])
        with self.assertRaises(TypeError): FluorescenceCorrection('CuO','Cu','K',line='Ka1')
        bad = np.asarray(case['mu']).copy(); bad[len(bad)//2] = 1e12
        with self.assertRaises(ValueError): model(case).apply(case['energy'],bad)
        imported = parse_measurement('energy I0 It\n7100 10 5\n7101 10 4\n').spectrum()
        self.assertEqual(imported.absorption_mode(), 'transmission')
        with self.assertRaisesRegex(ValueError, 'transmission'): imported.correct_fluorescence(model(case))


if __name__ == '__main__': unittest.main()
