//! Independent offline XrayDB table queries; fixture provenance retains versions
//! and the original database checksum. No measured or private data is included.
use rexafs::atomic::{AtomicData, EmissionSelection};
#[test]
fn pinned_atomic_queries_match_python_xraydb() {
    let reference: serde_json::Value = serde_json::from_str(include_str!(
        "fixtures/analysis/atomic/xraydb-reference.json"
    ))
    .unwrap();
    let db = AtomicData::new().unwrap();
    let array = |v: &serde_json::Value| {
        v.as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_f64().unwrap())
            .collect::<Vec<_>>()
    };
    let tolerance = reference["tolerance_relative"].as_f64().unwrap();
    let compare = |name: &str, actual: &[f64], expected: Vec<f64>| {
        assert_eq!(actual.len(), expected.len());
        for (i, (a, b)) in actual.iter().zip(expected).enumerate() {
            assert!(
                (a - b).abs() <= tolerance * b.abs(),
                "{name}[{i}]: {a} versus {b}"
            );
        }
    };
    for case in reference["cases"].as_array().unwrap() {
        let element = case["element"].as_str().unwrap();
        let shell = case["shell"].as_str().unwrap();
        assert_eq!(
            db.edge(element, shell).unwrap().energy_ev,
            case["edge"].as_f64().unwrap()
        );
        let energy = array(&case["energy"]);
        compare(
            element,
            &db.f2(element, &energy).unwrap().values,
            array(&case["f2"]),
        );
        compare(
            element,
            &db.attenuation(element, &energy).unwrap().values,
            array(&case["attenuation"]),
        );
        for (name, line) in case["lines"].as_object().unwrap() {
            let actual = db
                .emission(element, &EmissionSelection::Line(name.clone()))
                .unwrap();
            assert_eq!(actual.energy_ev, line["energy"].as_f64().unwrap());
            assert_eq!(
                actual.lines[0].intensity,
                line["intensity"].as_f64().unwrap()
            );
        }
    }
    for case in reference["compounds"].as_array().unwrap() {
        let formula = case["formula"].as_str().unwrap();
        compare(
            formula,
            &db.compound_attenuation(formula, &array(&case["energy"]))
                .unwrap()
                .curve
                .values,
            array(&case["values"]),
        );
    }
}
