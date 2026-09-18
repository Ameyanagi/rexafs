"""Installed-wheel peak contracts against attributed, pinned synthetic references."""
import json
from pathlib import Path
import unittest

import numpy as np
from rexafs import PeakFit, PeakFitResult, Spectrum, PrePostEdge

REFERENCE = json.loads((Path(__file__).resolve().parents[2] /
    "crates/rexafs/tests/fixtures/analysis/xanes-peaks/lmfit-reference.json").read_text())


def definition(case):
    p = case["initial"]
    model = PeakFit((-15, 35)).raw_mu().reference(9000)
    name = {"Gaussian": "gaussian", "Lorentzian": "lorentzian",
            "PseudoVoigt": "pseudo_voigt", "Voigt": "voigt"}[case["shape"]]
    args = dict(center=p["p_center"], area=p["p_area"])
    if name == "voigt":
        args.update(gaussian_fwhm=p["p_width"], lorentzian_fwhm=p["p_lorentz_width"])
    else:
        args["fwhm"] = p["p_width"]
    if name == "pseudo_voigt":
        args["fraction"] = p["p_fraction"]
    return getattr(model, name)("p", **args).linear_baseline(
        offset=p["baseline_offset"], slope=p["baseline_slope"]).exclude((9, 10))


class PeakTests(unittest.TestCase):
    def test_profiles_noise_masks_and_owned_results_match_lmfit(self):
        for case in REFERENCE["cases"]:
            with self.subTest(shape=case["shape"]):
                model = definition(case)
                initial = model.to_json()
                source = Spectrum(case["energy"], case["signal"])
                result = source.fit_peaks(model, errors=case["sigma"])
                self.assertIsInstance(result, PeakFitResult)
                self.assertEqual(result.termination, "Converged")
                self.assertIsNone(result.uncertainty_unavailable)
                self.assertEqual(result.source_indices, case["source_indices"])
                np.testing.assert_allclose(result.model, case["model"], atol=2e-7, rtol=0)
                self.assertAlmostEqual(result.objective, case["objective"], delta=1e-6)
                for name, ref in case["parameters"].items():
                    self.assertAlmostEqual(result.parameters[name], ref["value"], delta=1e-4)
                    self.assertAlmostEqual(result.parameter_errors[name] / ref["stderr"], 1, delta=3e-4)
                self.assertAlmostEqual(result.components[0].center_ev, 9000 + result.parameters["p_center"])
                self.assertIsNone(result.components[1].area)
                self.assertEqual(model.to_json(), initial)
                self.assertEqual(result.definition.to_json(), initial)
                fit_model = result.fitted_model()
                np.testing.assert_allclose(fit_model.evaluate(result.energy), result.model, atol=1e-12)
                self.assertEqual(PeakFit.from_json(initial).to_json(), initial)
                stored = result.to_json()
                result.model[:] = 100
                result.components[0].curve[:] = 100
                self.assertEqual(result.to_json(), stored)
                self.assertIsNone(source.norm())
                self.assertIsNone(source.e0())

    def test_automatic_norm_and_flat_are_copies(self):
        energy = np.arange(8900., 9901.)
        signal = .2 + .0001 * (energy - 9000) + 1 / (1 + np.exp(-(energy - 9000)/2))
        source = Spectrum(energy, signal).set_normalization_method(PrePostEdge(e0=9000))
        explicit = Spectrum(energy, signal).set_normalization_method(PrePostEdge(e0=9000)).normalize()
        original = PeakFit((-50, 100)).erf_step("edge", center=0, height=1, scale=4).constant_baseline()
        for model in (original, original.flat()):
            fit = source.fit_peaks(model)
            reference = explicit.fit_peaks(model)
            np.testing.assert_array_equal(fit.data, reference.data)
            np.testing.assert_allclose(fit.model, reference.model)
            self.assertEqual(fit.origin_ev, 9000)
        self.assertIsNone(source.norm())
        self.assertEqual(source.e0(), 9000)
        self.assertEqual(json.loads(original.to_json())["space"], "Norm")

    def test_batch_keeps_failure_rows_and_starts(self):
        case = REFERENCE["cases"][0]
        model = definition(case)
        source = Spectrum(case["energy"], case["signal"])
        short = Spectrum([8999, 9000, 9001], [1, 1, 1])
        rows = model.fit_batch([source, short, source])
        self.assertEqual([r.index for r in rows], [0, 1, 2])
        self.assertIsNone(rows[0].error)
        self.assertIsNone(rows[1].result)
        self.assertTrue(rows[1].error)
        self.assertEqual(rows[0].result.parameters, rows[2].result.parameters)
        self.assertEqual(model.fit_batch([]), [])

    def test_constraints_baseline_initialization_and_contextual_errors(self):
        energy = np.linspace(-20, 30, 401)
        truth = PeakFit((-20, 30)).raw_mu().absolute().gaussian("p", center=3, area=4, fwhm=3).linear_baseline(offset=.2, slope=.001)
        source = Spectrum(energy, truth.evaluate(energy))
        start = truth.parameter("baseline_offset", 0).parameter("baseline_slope", 0)
        initialized = start.initialize_baseline(source, [( -5, 12)])
        self.assertAlmostEqual(json.loads(initialized.to_json())["parameters"]["vars"]["baseline_offset"]["value"], .2, delta=1e-8)
        tied = start.parameter("p_width", 3, vary=False).parameter("baseline_slope", .001, expression="p_width/3000")
        result = source.fit_peaks(tied)
        self.assertEqual(result.parameters["p_width"], 3)
        self.assertAlmostEqual(result.parameters["baseline_slope"], .001)
        self.assertEqual(result.free_parameters, 3)
        self.assertEqual(start.as_baseline("p").to_json().count('"role":"Baseline"'), 2)
        with self.assertRaisesRegex(ValueError, "Unknown peak parameter"):
            start.parameter("typo", 1)
        with self.assertRaisesRegex(ValueError, "energy must be one-dimensional"):
            start.evaluate([[1, 2]])
        with self.assertRaises(ValueError):
            source.fit_peaks(start, errors=np.zeros(401))
        with self.assertRaises(ValueError):
            PeakFit((30, -20)).raw_mu().absolute().constant_baseline().evaluate(energy)
        with self.assertRaises(ValueError):
            source.fit_peaks(start.solver(max_iterations=0))


if __name__ == "__main__":
    unittest.main()
