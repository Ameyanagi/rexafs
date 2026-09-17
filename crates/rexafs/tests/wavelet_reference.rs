//! Matched-order/grid comparison with pinned Larch, not default-for-default identity.
use rexafs::Wavelet;
#[test]
fn cauchy_matches_pinned_larch_with_explicit_order_grid_and_fft_scaling() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "fixtures/analysis/wavelet/larch-reference.json"
    ))
    .unwrap();
    let array = |v: &serde_json::Value| {
        v.as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_f64().unwrap())
            .collect::<Vec<_>>()
    };
    for case in fixture["cases"].as_array().unwrap() {
        let k = array(&case["k"]);
        let chi = array(&case["chi"]);
        let map = Wavelet::new(k[0]..=k[k.len() - 1])
            .kstep(case["kstep"].as_f64().unwrap())
            .kweight(case["kweight"].as_u64().unwrap() as u8)
            .order(case["order"].as_u64().unwrap() as usize)
            .nfft(case["actual_fft_length"].as_u64().unwrap() as usize)
            .radii(array(&case["r"]))
            .calculate(&k, &chi)
            .unwrap();
        for (name, actual) in [("real", map.real()), ("imaginary", map.imaginary())] {
            let expected = array(&case[name]);
            assert_eq!(actual.len(), expected.len());
            let worst = actual
                .iter()
                .zip(expected)
                .map(|(a, b)| (a - b).abs())
                .fold(0., f64::max);
            assert!(
                worst < fixture["absolute_tolerance"].as_f64().unwrap(),
                "order {} {name}: {worst}",
                map.settings().order
            );
        }
    }
}
