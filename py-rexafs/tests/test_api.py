"""Run against the built wheel, not the source package."""

import json
import unittest
from pathlib import Path

import numpy as np
import rexafs

FIXTURE = (
    Path(__file__).resolve().parents[2] / "crates/rexafs/tests/testfiles/Ru_QAS.dat"
)


class SpectrumTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        data = np.loadtxt(FIXTURE)
        cls.energy, cls.mu = data[:, 0], np.log(data[:, 1] / data[:, 2])

    def spectrum(self):
        return rexafs.Spectrum.from_arrays(self.energy, self.mu)

    def test_terminal_stage_matches_explicit_chain(self):
        spectrum = self.spectrum()
        self.assertIsNone(spectrum.chi())
        self.assertIs(spectrum.fft(), spectrum)
        explicit = self.spectrum().find_e0().normalize().calc_background().fft()
        for other in (explicit,):
            self.assertEqual(spectrum.e0(), other.e0())
            for name in ("norm", "chi", "r", "chir_mag"):
                np.testing.assert_allclose(
                    getattr(spectrum, name)(), getattr(other, name)()
                )
        np.testing.assert_allclose(
            spectrum.chir_mag(),
            np.hypot(spectrum.chir_real(), spectrum.chir_imag()),
            atol=1e-10,
        )

    def test_owned_arrays_and_input_conversion(self):
        energy, mu = self.energy.copy(), self.mu.copy()
        spectrum = rexafs.Spectrum(energy.tolist(), mu).fft()
        old_chi = spectrum.chi()
        saved_chi = old_chi.copy()
        old_chi[:] = 0
        np.testing.assert_allclose(spectrum.chi(), saved_chi)
        np.testing.assert_array_equal(energy, self.energy)
        np.testing.assert_array_equal(mu, self.mu)
        spectrum.set_e0(spectrum.e0() + 0.25).fft()
        np.testing.assert_array_equal(old_chi, np.zeros_like(old_chi))

    def test_parameters_and_invalidation(self):
        norm = rexafs.PrePostEdge()
        norm.pre_edge_start = -200
        norm.pre_edge_end = -65
        bkg = rexafs.AUTOBK()
        bkg.rbkg = 1.2
        ft = rexafs.XrayFFTF()
        ft.kweight = 1
        spectrum = (
            self.spectrum()
            .set_normalization_method(rexafs.NormalizationMethod.PrePostEdge(norm))
            .set_background_method(rexafs.BackgroundMethod.AUTOBK(bkg))
            .set_fft(ft)
            .fft()
        )
        chi, old_r = spectrum.chi(), spectrum.chir_mag()
        ft.kweight = 3
        spectrum.set_fft(ft)
        self.assertIsNone(spectrum.r())
        spectrum.fft()
        np.testing.assert_array_equal(spectrum.chi(), chi)
        self.assertFalse(np.allclose(spectrum.chir_mag(), old_r))
        edge = spectrum.e0() + 0.25
        spectrum.set_e0(edge)
        self.assertIsNone(spectrum.norm())
        self.assertIsNone(spectrum.chi())
        self.assertIsNone(spectrum.r())
        self.assertEqual(spectrum.fft().e0(), edge)
        spectrum.set_spectrum(self.energy, self.mu)
        self.assertIsNone(spectrum.e0())
        self.assertIsNone(spectrum.chi())
        self.assertIsNotNone(spectrum.fft().r())

    def test_fft_grid_is_explicit_and_preserves_chi(self):
        ft = rexafs.XrayFFTF()
        self.assertEqual(ft.grid, "Input")
        spectrum = self.spectrum().calc_background()
        ft.kmax = float(spectrum.k()[-1])
        spectrum.set_fft(ft).fft()
        chi, original = spectrum.chi(), spectrum.chir_mag()
        ft.grid = "Larch"
        self.assertEqual(ft.grid, "Larch")
        spectrum.set_fft(ft).fft()
        np.testing.assert_array_equal(spectrum.chi(), chi)
        self.assertFalse(np.allclose(spectrum.chir_mag(), original))
        ft.kstep = 0.025
        spectrum.set_fft(ft).fft()
        self.assertEqual(spectrum.kwin_k()[1], 0.025)
        self.assertEqual(len(spectrum.kwin_k()), len(spectrum.kwin()))
        with self.assertRaises(ValueError):
            ft.grid = "unknown"
        self.assertEqual(ft.grid, "Larch")
        ft.grid = "Input"
        ft.kstep = None
        np.testing.assert_array_equal(spectrum.set_fft(ft).fft().chir_mag(), original)

    def test_fixed_lambda_is_configurable_and_zero_disables_both_clamps(self):
        bkg = rexafs.AUTOBK()
        self.assertEqual(bkg.clamp_scale_policy, "FixedPenalty")
        self.assertEqual(bkg.clamp_lambda, 0.001)
        bkg.kmax = 12.0
        bkg.clamp_lo = 2
        bkg.clamp_hi = 5

        def chi():
            return (
                self.spectrum()
                .set_background_method(rexafs.BackgroundMethod.AUTOBK(bkg))
                .calc_background()
                .chi()
            )

        initial = chi()
        bkg.clamp_lambda = 1.0
        self.assertGreater(np.linalg.norm(chi() - initial), 1e-6)
        bkg.clamp_lambda = 0.0
        zero = chi()
        bkg.nclamp = 0
        np.testing.assert_array_equal(chi(), zero)
        for invalid in (-1.0, float("nan"), float("inf")):
            bkg.clamp_lambda = invalid
            with self.assertRaisesRegex(RuntimeError, "clamp_lambda"):
                chi()

    def test_fixed_lambda_matches_independent_reference(self):
        cases = json.loads(FIXTURE.with_name("autobk_fixed_reference.json").read_text())
        for case in cases:
            norm = rexafs.PrePostEdge()
            norm.e0 = case["settings"]["ek0"]
            norm.edge_step = case["edge_step"]
            bkg = rexafs.AUTOBK()
            for name, value in case["settings"].items():
                setattr(bkg, name, value)
            for penalty, expected in zip((0, 0.001, 1), case["chi"]):
                with self.subTest(case=case["name"], penalty=penalty):
                    bkg.clamp_lambda = penalty
                    spectrum = (
                        rexafs.Spectrum(case["energy"], case["mu"])
                        .set_e0(norm.e0)
                        .set_normalization_method(
                            rexafs.NormalizationMethod.PrePostEdge(norm)
                        )
                        .set_background_method(rexafs.BackgroundMethod.AUTOBK(bkg))
                        .calc_background()
                    )
                    np.testing.assert_allclose(
                        spectrum.k(), case["k"], rtol=0, atol=1e-14
                    )
                    # Independent SciPy fixture of the same fixed objective.
                    np.testing.assert_allclose(
                        spectrum.chi(), expected, rtol=1e-11, atol=1e-12
                    )

    def test_unsupported_methods_are_not_replaced(self):
        spectrum = self.spectrum().set_normalization_method(
            rexafs.NormalizationMethod.new_mback()
        )
        with self.assertRaisesRegex(ValueError, "MBack"):
            spectrum.fft()
        spectrum.set_normalization_method().set_background_method(
            rexafs.BackgroundMethod.new_ilpbkg()
        )
        with self.assertRaisesRegex(RuntimeError, "ILPBkg"):
            spectrum.fft()
        self.assertIsNone(spectrum.r())
        self.assertIsNotNone(spectrum.set_background_method().fft().r())

    def test_invalid_input_and_parameters_raise(self):
        for energy, mu in [
            ([], []),
            ([2, 1, 3], [1]),
            ([1, 2, 2], [1, 2, 3]),
            ([1, np.nan, 3], [1, 2, 3]),
            ([1, 2, 3], [1, np.inf, 3]),
            ([[1, 2]], [[1, 2]]),
        ]:
            with self.subTest(energy=energy), self.assertRaises(ValueError):
                rexafs.Spectrum.from_arrays(energy, mu)
        with self.assertRaises(ValueError):
            self.spectrum().set_e0(float("nan")).fft()
        spectrum = self.spectrum().fft()
        ft = rexafs.XrayFFTF()
        ft.nfft = 0
        with self.assertRaises(RuntimeError):
            spectrum.set_fft(ft).fft()
        self.assertIsNone(spectrum.r())
        ft.nfft = 2048
        self.assertIsNotNone(spectrum.set_fft(ft).fft().r())

    def test_keyword_settings_and_runtime_help(self):
        import inspect

        background = rexafs.AUTOBK(rbkg=1.2)
        transform = rexafs.XrayFFTF(kmax=12.0, window="Hanning")
        normalization = rexafs.PrePostEdge(pre_edge_end=-30.0)
        self.assertEqual(background.solver, "LinearDirect")
        self.assertEqual(background.clamp_lambda, 0.001)
        self.assertEqual(background.kstep, 0.05)
        self.assertEqual(transform.kweight, 2.0)
        self.assertIsNone(normalization.norm_end)
        direct = (
            self.spectrum()
            .set_normalization_method(normalization)
            .set_background_method(background)
            .set_fft(transform)
            .fft()
        )
        explicit = (
            self.spectrum()
            .set_normalization_method(
                rexafs.NormalizationMethod.PrePostEdge(normalization)
            )
            .set_background_method(rexafs.BackgroundMethod.AUTOBK(background))
            .set_fft(transform)
            .fft()
        )
        np.testing.assert_array_equal(direct.chi(), explicit.chi())
        np.testing.assert_array_equal(direct.chir_mag(), explicit.chir_mag())
        background.rbkg = 2.0
        np.testing.assert_array_equal(direct.calc_background().chi(), explicit.chi())
        with self.assertRaises(TypeError):
            rexafs.AUTOBK(rbkg_typo=1)
        with self.assertRaises(TypeError):
            rexafs.AUTOBK(1.2)
        with self.assertRaises(ValueError):
            rexafs.XrayFFTF(window="Typo")
        self.assertIn("angstroms", rexafs.AUTOBK.rbkg.__doc__)
        self.assertIn("2048", rexafs.Spectrum.fft.__doc__)
        signature = inspect.signature(rexafs.AUTOBK)
        self.assertEqual(signature.parameters["clamp_lambda"].default, 0.001)
        self.assertEqual(
            signature.parameters["rbkg"].kind, inspect.Parameter.KEYWORD_ONLY
        )

    def test_inverse_settings_preserve_forward_and_invalidate_inverse(self):
        inverse = rexafs.XrayFFTR(rmin=1.0, rmax=3.0, dr=0.5)
        spectrum = self.spectrum().ifft()
        old, forward = spectrum.chiq(), spectrum.chir_mag()
        self.assertEqual(inverse.nfft, 2048)
        self.assertEqual(inverse.qmax_out, 10.0)
        self.assertIs(spectrum.set_ifft(inverse), spectrum)
        self.assertIsNone(spectrum.q())
        self.assertIsNone(spectrum.chiq())
        np.testing.assert_array_equal(spectrum.chir_mag(), forward)
        inverse.rmax = 4.0
        spectrum.ifft()
        filtered = spectrum.chiq()
        self.assertFalse(np.allclose(filtered, old))
        self.assertEqual(len(filtered), len(spectrum.q()))
        self.assertTrue(np.isfinite(filtered).all())
        spectrum.set_ifft(inverse).ifft()
        self.assertFalse(np.allclose(spectrum.chiq(), filtered))
        inverse.nfft = 0
        spectrum.set_ifft(inverse)
        with self.assertRaisesRegex(RuntimeError, "nfft"):
            spectrum.ifft()
        self.assertIsNone(spectrum.chiq())

    def test_reader_and_removed_pipeline_facade(self):
        spectrum = rexafs.io.read_qas_transmission(FIXTURE).fft()
        self.assertIsInstance(spectrum, rexafs.Spectrum)
        np.testing.assert_allclose(spectrum.chi(), self.spectrum().fft().chi())
        for name in (
            "process",
            "ProcessedSpectrum",
            "process_qas_batch",
            "run_pipeline_arrays",
            "run_batch_qas_trans",
        ):
            self.assertFalse(hasattr(rexafs, name), name)


if __name__ == "__main__":
    unittest.main()
