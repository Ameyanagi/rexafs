//! Pinned Larch match_f2 objective and preedge normalization convention. Inputs
//! are synthetic; fixtures record every loaded source hash and package version.
use rexafs::prelude::*;
#[test]
fn full_mback_matches_pinned_larch_objective_and_normalization() {
    let reference: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/analysis/mback/larch-reference.json")).unwrap();
    let array = |v: &serde_json::Value| {
        v.as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_f64().unwrap())
            .collect::<Vec<_>>()
    };
    let curve_tolerance = reference["tolerances"]["curve_absolute"].as_f64().unwrap();
    for case in reference["cases"].as_array().unwrap() {
        let pre = array(&case["pre_edge"]);
        let post = array(&case["post_edge"]);
        let erfc = case["erfc"].as_bool().unwrap();
        let mut model = MBack::for_edge("Cu", "K")
            .e0(case["e0"].as_f64().unwrap())
            .pre_edge(pre[0]..=pre[1])
            .post_edge(post[0]..=post[1])
            .degree(case["degree"].as_u64().unwrap() as usize);
        if erfc {
            model = model.erfc(MbackErfc::new("Ka1", 500.0..=1500.0, 0.0..=10.0));
        }
        let result = model
            .fit(&array(&case["energy"]), &array(&case["mu"]))
            .unwrap();
        for (name, actual) in [
            ("f2", &result.f2),
            ("background", &result.background),
            ("fpp", &result.fpp),
            ("norm", &result.norm),
            ("pre_curve", &result.pre_curve),
            ("post_curve", &result.post_curve),
        ] {
            let expected = array(&case[name]);
            assert_eq!(actual.len(), expected.len());
            let worst = actual
                .iter()
                .zip(expected)
                .map(|(a, b)| (a - b).abs())
                .fold(0., f64::max);
            assert!(
                worst < curve_tolerance,
                "erfc={erfc}, {name}: error {worst}"
            );
        }
        assert_eq!(
            result.fit_indices,
            case["fit_indices"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_u64().unwrap() as usize)
                .collect::<Vec<_>>()
        );
        assert!(
            (result.scale - case["scale"].as_f64().unwrap()).abs()
                < reference["tolerances"]["scale_absolute"].as_f64().unwrap()
        );
        assert!(
            (result.objective - case["objective"].as_f64().unwrap()).abs()
                < reference["tolerances"]["objective_absolute"]
                    .as_f64()
                    .unwrap()
        );
        assert!((result.edge_step - case["edge_step"].as_f64().unwrap()).abs() < curve_tolerance);
        for (j, &coefficient) in result.coefficients.iter().enumerate() {
            let expected = case["coefficients"][j].as_f64().unwrap();
            assert!(
                (coefficient / result.energy_scale.powi(j as i32) - expected).abs()
                    < curve_tolerance
            );
        }
        if erfc {
            assert!(
                (result.erfc_width.unwrap() - case["erfc_width"].as_f64().unwrap()).abs()
                    < reference["tolerances"]["erfc_width_absolute"]
                        .as_f64()
                        .unwrap()
            );
        }
    }
}
