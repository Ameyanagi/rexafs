//! Synthetic qualification against unchanged numerical functions from pinned Larch.
use rexafs::FluorescenceCorrection;
#[test]
fn fluo_matches_pinned_larch_with_explicit_geometry_and_internal_normalization() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "fixtures/analysis/fluorescence/larch-reference.json"
    ))
    .unwrap();
    let array = |v: &serde_json::Value| {
        v.as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_f64().unwrap())
            .collect::<Vec<_>>()
    };
    for c in fixture["cases"].as_array().unwrap() {
        let pre = array(&c["pre_edge"]);
        let post = array(&c["post_edge"]);
        let angles = array(&c["angles"]);
        let mut model = FluorescenceCorrection::new(
            c["formula"].as_str().unwrap(),
            c["element"].as_str().unwrap(),
            "K",
        )
        .angles(angles[0], angles[1])
        .e0(c["e0"].as_f64().unwrap())
        .pre_edge(pre[0]..=pre[1])
        .post_edge(post[0]..=post[1])
        .degree(c["degree"].as_u64().unwrap() as usize);
        model = if c["family"].as_bool().unwrap() {
            model.line_family(c["line"].as_str().unwrap())
        } else {
            model.line(c["line"].as_str().unwrap())
        };
        let r = model.apply(&array(&c["energy"]), &array(&c["mu"])).unwrap();
        for (name, actual) in [
            ("internal_norm", &r.internal.norm),
            ("pre_curve", &r.internal.pre_curve),
            ("post_curve", &r.internal.post_curve),
            ("corrected_mu", &r.corrected_mu),
        ] {
            let expected = array(&c[name]);
            assert_eq!(actual.len(), expected.len());
            let error = actual
                .iter()
                .zip(expected)
                .map(|(a, b)| (a - b).abs())
                .fold(0., f64::max);
            assert!(
                error < fixture["tolerances"]["curve_absolute"].as_f64().unwrap(),
                "{} {name}: {error}",
                c["formula"]
            );
        }
        for (name, actual) in [
            ("alpha", r.alpha),
            ("emission_ev", r.emission.energy_ev),
            ("geometry_ratio", r.geometry_ratio),
        ] {
            let expected = c[name].as_f64().unwrap();
            assert!(
                (actual - expected).abs() / expected.abs()
                    < fixture["tolerances"]["atomic_relative"].as_f64().unwrap(),
                "{name}: {actual} != {expected}"
            );
        }
        for (actual, expected) in r
            .attenuation
            .curve
            .values
            .iter()
            .zip(array(&c["attenuation"]))
        {
            assert!((actual - expected).abs() / expected < 2e-10);
        }
    }
}
