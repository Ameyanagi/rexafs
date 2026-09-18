//! Independent, pinned lmfit references. All inputs are synthetic; regenerate
//! with scripts/generate-peakfit-reference.py and retain package versions.
use rexafs::prelude::*;

#[test]
fn four_peak_profiles_match_pinned_lmfit_fit_and_covariance() {
    let reference: serde_json::Value = serde_json::from_str(include_str!(
        "fixtures/analysis/xanes-peaks/lmfit-reference.json"
    ))
    .unwrap();
    let array = |v: &serde_json::Value| -> Vec<f64> {
        v.as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_f64().unwrap())
            .collect()
    };
    for case in reference["cases"].as_array().unwrap() {
        let energy = array(&case["energy"]);
        let signal = array(&case["signal"]);
        let sigma = array(&case["sigma"]);
        let shape = case["shape"].as_str().unwrap();
        let initial = &case["initial"];
        let val = |key: &str| initial[key].as_f64().unwrap();
        let definition = PeakFit::new(-15.0..=35.0);
        let definition = match shape {
            "Gaussian" => definition.gaussian("p", val("p_center"), val("p_area"), val("p_width")),
            "Lorentzian" => {
                definition.lorentzian("p", val("p_center"), val("p_area"), val("p_width"))
            }
            "PseudoVoigt" => definition.pseudo_voigt(
                "p",
                val("p_center"),
                val("p_area"),
                val("p_width"),
                val("p_fraction"),
            ),
            "Voigt" => definition.voigt(
                "p",
                val("p_center"),
                val("p_area"),
                val("p_width"),
                val("p_lorentz_width"),
            ),
            _ => unreachable!(),
        }
        .linear_baseline(val("baseline_offset"), val("baseline_slope"))
        .exclude(9.0..=10.0);
        let result = definition
            .fit_prepared(&energy, &signal, Some(9000.), Some(&sigma))
            .unwrap();
        assert_eq!(
            result.termination,
            PeakTermination::Converged,
            "{shape}: {}",
            result.termination_detail
        );
        let tolerance = &reference["tolerances"];
        for (name, expected) in case["parameters"].as_object().unwrap() {
            let actual = &result.parameters.vars[name];
            assert!(
                (actual.value - expected["value"].as_f64().unwrap()).abs()
                    < tolerance["parameter_absolute"].as_f64().unwrap(),
                "{shape} {name}: {} versus {}",
                actual.value,
                expected["value"]
            );
            let expected_error = expected["stderr"].as_f64().unwrap();
            assert!(
                (actual.stderr.unwrap() / expected_error - 1.).abs()
                    < tolerance["standard_error_relative"].as_f64().unwrap(),
                "{shape} {name} stderr: {:?} versus {expected_error}",
                actual.stderr
            );
        }
        assert_eq!(
            result.source_indices,
            case["source_indices"]
                .as_array()
                .unwrap()
                .iter()
                .map(|i| i.as_u64().unwrap() as usize)
                .collect::<Vec<_>>()
        );
        let curve_error = result
            .model
            .iter()
            .zip(array(&case["model"]))
            .map(|(a, b)| (a - b).abs())
            .fold(0., f64::max);
        assert!(
            curve_error < tolerance["curve_absolute"].as_f64().unwrap(),
            "{shape} maximum curve error {curve_error}"
        );
        assert!(
            (result.objective - case["objective"].as_f64().unwrap()).abs()
                < tolerance["objective_absolute"].as_f64().unwrap(),
            "{shape} objective {} versus {}",
            result.objective,
            case["objective"]
        );
    }
}

#[test]
fn spectrum_entry_point_prepares_selected_representation_without_mutation() {
    let energy: Vec<_> = (0..1001).map(|i| 8900. + i as f64).collect();
    let mu: Vec<_> = energy
        .iter()
        .map(|e| 0.2 + 0.0001 * (e - 9000.) + 1. / (1. + (-(e - 9000.) / 2.).exp()))
        .collect();
    let mut spectrum = Spectrum::from_arrays(&energy, &mu).unwrap();
    let mut norm = PrePostEdge::new();
    norm.e0 = Some(9000.);
    spectrum.set_normalization_method(norm).unwrap();
    let before = serde_json::to_value(&spectrum).unwrap();
    for definition in [
        PeakFit::new(-50.0..=100.0),
        PeakFit::new(-50.0..=100.0).flat(),
    ] {
        let result = definition
            .erf_step("edge", 0., 1., 4.)
            .constant_baseline(0.)
            .fit(&spectrum)
            .unwrap();
        assert!(result.model.iter().all(|v| v.is_finite()));
        assert_eq!(result.origin_ev, 9000.);
    }
    assert_eq!(serde_json::to_value(&spectrum).unwrap(), before);
}
