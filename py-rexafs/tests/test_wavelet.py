"""Installed wavelet API: native reference parity, ownership and automatic preparation."""
import json
from pathlib import Path
import unittest
import numpy as np
from rexafs import Spectrum, Wavelet, WaveletMap

REFERENCE = json.loads((Path(__file__).resolve().parents[2] /
    'crates/rexafs/tests/fixtures/analysis/wavelet/larch-reference.json').read_text())


class WaveletTests(unittest.TestCase):
    def test_native_reference_matrix_layout_and_owned_replay(self):
        for case in REFERENCE['cases']:
            with self.subTest(weight=case['kweight']):
                k = np.array(case['k']); chi = np.array(case['chi'])
                model = Wavelet((k[0], k[-1]), kweight=case['kweight'], order=case['order'],
                    kstep=case['kstep'], nfft=case['actual_fft_length'], radii=case['r'])
                initial = model.to_json()
                result = model.calculate(k, chi)
                self.assertEqual(result.shape, (len(case['r']), len(k)))
                for name in ['real', 'imaginary']:
                    np.testing.assert_allclose(getattr(result, name).ravel(), case[name],
                        rtol=0, atol=REFERENCE['absolute_tolerance'])
                self.assertEqual(model.to_json(), initial)
                self.assertIsNone(result.preparation)
                self.assertEqual(model.estimate(k)['cells'], result.real.size)
                region = result.integral((1., 3.), (.2, 1.))
                self.assertGreater(region.value, 0)
                self.assertEqual(region.k_range, (1., 3.))
                self.assertEqual(region.method, 'bilinear_magnitude_v1')
                mean = result.mean((1., 3.), (.2, 1.))
                maximum = result.maximum((1., 3.), (.2, 1.))
                self.assertAlmostEqual(mean.value, region.value / 1.6)
                self.assertGreaterEqual(maximum.value, mean.value)
                self.assertEqual(mean.method, 'bilinear_magnitude_mean_v1')
                self.assertEqual(maximum.method, 'bilinear_magnitude_maximum_v1')
                for operation in [result.mean, result.maximum]:
                    with self.assertRaises(ValueError):
                        operation((0., 100.), (.2, 1.))
                saved = result.to_json()
                modified = result.real; modified[:] = 999
                chi[:] = 999
                np.testing.assert_array_equal(result.input_chi, case['chi'])
                self.assertEqual(result.to_json(), saved)
                restored = WaveletMap.from_json(saved)
                np.testing.assert_array_equal(restored.real, result.real)
                np.testing.assert_array_equal(result.definition.calculate(result.input_k, result.input_chi).real, result.real)
                self.assertEqual(restored.integral((1., 3.), (.2, 1.)).value, region.value)
                np.testing.assert_array_equal(result.slice_at_r(result.r[2]), result.magnitude[2])
                np.testing.assert_array_equal(result.slice_at_k(result.k[2]), result.magnitude[:, 2])
                result.phase()
                self.assertEqual(result.to_json(), saved)

    def test_automatic_processing_preserves_the_source(self):
        energy = np.arange(8760., 10181.)
        k = np.sqrt(np.maximum(0, energy - 8980) / 3.80998212)
        mu = .1 + .00003*(energy-8980) + (1+.06*np.sin(4.6*k)*np.exp(-((k-8)/4)**2))/(1+np.exp(-(energy-8980)/1.5))
        source = Spectrum(energy, mu)
        result = source.wavelet(Wavelet((2., 12.)))
        self.assertIsNone(source.norm())
        self.assertIsNone(source.chi())
        self.assertEqual(result.preparation['backend'], 'nalgebra')
        self.assertGreater(result.magnitude.max(), 0)
        source.calc_background()
        replay = Wavelet((2., 12.)).calculate(source.k(), source.chi())
        np.testing.assert_array_equal(result.real, replay.real)

    def test_masked_phase_and_invalid_shapes_coverage_and_budgets(self):
        k = np.arange(0., 10.1, .1)
        result = Wavelet((1., 9.)).calculate(k, np.zeros(len(k)))
        self.assertTrue(np.isnan(result.phase()).all())
        self.assertEqual(result.integral((2., 8.), (1., 3.)).value, 0)
        for run in [lambda: Wavelet((0., 20.)).calculate(k, k),
                    lambda: Wavelet((1., 9.)).calculate([k], k),
                    lambda: Wavelet((1., 9.), kstep=1e-100).estimate(k),
                    lambda: result.integral((0., 3.), (1., 3.)),
                    lambda: result.phase(2)]:
            with self.assertRaises(ValueError): run()
        bad = json.loads(result.to_json()); bad['real'].pop()
        with self.assertRaisesRegex(ValueError, 'dimensions'):
            WaveletMap.from_json(json.dumps(bad))
