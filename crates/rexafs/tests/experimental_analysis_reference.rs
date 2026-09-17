//! Independent Larch references from three attributed experimental measurements.
//! MBACK uses original μ(E); wavelet uses identical Larch-prepared χ(k) in both
//! implementations. This isolates the transform, not AUTOBK algorithm agreement.
use std::{io::Read, path::PathBuf};

use rexafs::{io::*, MBack, Wavelet};
use serde_json::Value;
use sha2::{Digest, Sha256};

fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn array(value: &Value) -> Vec<f64> {
    value
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_f64().unwrap())
        .collect()
}

fn compare(label: &str, actual: &[f64], expected: &[f64], tolerance: f64) {
    assert_eq!(actual.len(), expected.len(), "{label}: length");
    assert!(
        actual.iter().chain(expected).all(|v| v.is_finite()),
        "{label}: nonfinite value"
    );
    let worst = actual
        .iter()
        .zip(expected)
        .map(|(a, b)| (a - b).abs())
        .fold(0., f64::max);
    eprintln!("{label}: maximum absolute difference {worst:e}");
    assert!(worst < tolerance, "{label}: {worst:e} >= {tolerance:e}");
}

fn reference(id: &str) -> (Value, Value, Vec<f64>, Vec<f64>) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let refs = root.join("analysis/experimental-larch");
    let manifest: Value =
        serde_json::from_slice(&std::fs::read(refs.join("manifest.json")).unwrap()).unwrap();
    assert_eq!(manifest["kind"], "experimental");
    let case = manifest["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == id)
        .unwrap();
    let packed = std::fs::read(refs.join(case["fixture"].as_str().unwrap())).unwrap();
    assert_eq!(digest(&packed), case["sha256"]);
    let mut json = Vec::new();
    flate2::read::GzDecoder::new(packed.as_slice())
        .read_to_end(&mut json)
        .unwrap();
    let expected: Value = serde_json::from_slice(&json).unwrap();
    let raw = std::fs::read(
        root.join("xas")
            .join(case["sample"]["path"].as_str().unwrap()),
    )
    .unwrap();
    assert_eq!(digest(&raw), case["sample"]["sha256"]);
    let document = parse_measurement(&raw).unwrap();
    assert_eq!(document.scans.len(), 1);
    let scan = &document.scans[0];
    let signal = match case["conversion"].as_str().unwrap() {
        "xdi-rt" => SignalConversion::Direct { column: 3 },
        "xdi-10k" => SignalConversion::Direct { column: 1 },
        "9809" => SignalConversion::Transmission {
            incident: 3,
            transmitted: 4,
        },
        kind => panic!("Unqualified conversion {kind}"),
    };
    let mapping = &scan
        .signals
        .iter()
        .find(|s| s.mapping.signal == signal)
        .expect("recorded signal must remain available")
        .mapping;
    let (energy, mu) = scan.arrays(Some(mapping)).unwrap();
    assert_eq!(energy.len(), case["points"].as_u64().unwrap() as usize);
    compare(
        &format!("{id}/raw energy"),
        &energy,
        &array(&expected["energy"]),
        1e-9,
    );
    compare(&format!("{id}/raw mu"), &mu, &array(&expected["mu"]), 1e-13);
    (expected, manifest["tolerances"].clone(), energy, mu)
}

fn check_mback(id: &str) {
    let (expected, tolerances, energy, mu) = reference(id);
    let case = &expected["mback"];
    let pre = array(&case["pre_edge"]);
    let post = array(&case["post_edge"]);
    let result = MBack::for_edge(case["element"].as_str().unwrap(), "K")
        .e0(case["e0"].as_f64().unwrap())
        .pre_edge(pre[0]..=pre[1])
        .post_edge(post[0]..=post[1])
        .degree(2)
        .fit(&energy, &mu)
        .unwrap();
    let curve_tol = tolerances["mback_curve_absolute"].as_f64().unwrap();
    for (name, actual) in [
        ("f2", &result.f2),
        ("background", &result.background),
        ("fpp", &result.fpp),
        ("norm", &result.norm),
        ("pre_curve", &result.pre_curve),
    ] {
        compare(
            &format!("{id}/MBACK/{name}"),
            actual,
            &array(&case[name]),
            curve_tol,
        );
    }
    // The auxiliary post-edge polynomial is evaluated on absolute energy.
    // Its Ru K-edge reference has greater cross-platform roundoff than the
    // atomic match itself; keep its documented bound separate from the fit.
    compare(
        &format!("{id}/MBACK/post_curve"),
        &result.post_curve,
        &array(&case["post_curve"]),
        tolerances["mback_post_curve_absolute"].as_f64().unwrap(),
    );
    for (name, value, tolerance) in [
        (
            "scale",
            result.scale,
            tolerances["mback_scale_absolute"].as_f64().unwrap(),
        ),
        (
            "objective",
            result.objective,
            tolerances["mback_objective_absolute"].as_f64().unwrap(),
        ),
        ("edge_step", result.edge_step, curve_tol),
    ] {
        compare(
            &format!("{id}/MBACK/{name}"),
            &[value],
            &[case[name].as_f64().unwrap()],
            tolerance,
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
    let coefficients: Vec<_> = result
        .coefficients
        .iter()
        .enumerate()
        .map(|(j, c)| c / result.energy_scale.powi(j as i32))
        .collect();
    compare(
        &format!("{id}/MBACK/coefficients"),
        &coefficients,
        &array(&case["coefficients"]),
        curve_tol,
    );
}

fn check_wavelet(id: &str) {
    let (expected, tolerances, _, _) = reference(id);
    let case = &expected["wavelet"];
    let k = array(&case["k"]);
    let chi = array(&case["chi"]);
    assert_eq!(k.len(), 281);
    let r = array(&case["r"]);
    let map = Wavelet::new(k[0]..=k[k.len() - 1])
        .kstep(case["kstep"].as_f64().unwrap())
        .kweight(2)
        .order(case["order"].as_u64().unwrap() as usize)
        .nfft(case["actual_fft_length"].as_u64().unwrap() as usize)
        .radii(r.clone())
        .calculate(&k, &chi)
        .unwrap();
    assert_eq!(map.r(), r);
    assert_eq!(map.real().len(), r.len() * k.len());
    for (name, actual) in [("real", map.real()), ("imaginary", map.imaginary())] {
        compare(
            &format!("{id}/wavelet/{name}"),
            actual,
            &array(&case[name]),
            tolerances["wavelet_absolute"].as_f64().unwrap(),
        );
    }
}

#[test]
fn experimental_cu_rt_mback() {
    check_mback("cu-rt");
}
#[test]
fn experimental_cu_10k_mback() {
    check_mback("cu-10k");
}
#[test]
fn experimental_ruo2_mback() {
    check_mback("ruo2");
}
#[test]
fn experimental_cu_rt_wavelet() {
    check_wavelet("cu-rt");
}
#[test]
fn experimental_cu_10k_wavelet() {
    check_wavelet("cu-10k");
}
#[test]
fn experimental_ruo2_wavelet() {
    check_wavelet("ruo2");
}
