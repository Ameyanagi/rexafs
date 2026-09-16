//! Real-standard, synthetic-mixture recovery tests. These fixtures and this
//! target are excluded from published crates; source and permissions are recorded
//! in fixtures/analysis/cu-mixtures/README.md. No external checkout is required.

use nalgebra::DVector;
use rexafs::prelude::*;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/analysis/cu-mixtures")
}
fn json_file(name: &str) -> Value {
    serde_json::from_slice(&fs::read(root().join(name)).unwrap()).unwrap()
}
fn prepared(path: &str) -> XASSpectrum {
    let m = rexafs::io::read_measurement(root().join(path)).unwrap();
    assert_eq!(m.scans.len(), 1);
    let (x, y) = m.scans[0].arrays(None).unwrap();
    // Prepared normalized fixtures must retain their values in both numerical backends.
    let mut spectrum = XASSpectrum::from_prepared(&x, &y, AnalysisSpace::Norm, 8979.).unwrap();
    spectrum.set_name(path);
    spectrum
}
fn standards() -> Vec<XASSpectrum> {
    ["cufoil_abs", "cu2o_abs", "cuo_abs"]
        .iter()
        .map(|label| prepared(&format!("standards/{label}.xdi")))
        .collect()
}
fn mixtures() -> Vec<XASSpectrum> {
    (1..=100)
        .map(|i| prepared(&format!("mixtures/mix_{i:03}.xdi")))
        .collect()
}
fn config() -> McrConfig {
    McrConfig {
        range: Some((-29.0, 171.0)),
        ..McrConfig::default()
    }
}
fn report(name: &str, result: &impl serde::Serialize) {
    if let Some(dir) = std::env::var_os("REXAFS_CU_REPORT") {
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            PathBuf::from(dir).join(name),
            serde_json::to_vec(result).unwrap(),
        )
        .unwrap();
    }
}

#[test]
fn all_100_files_preserve_recorded_composition_and_checksums() {
    let manifest = json_file("manifest.json");
    let files = manifest["files"].as_array().unwrap();
    assert_eq!(files.len(), 103);
    for file in files {
        let bytes = fs::read(root().join(file["path"].as_str().unwrap())).unwrap();
        let strict = XdiFile::parse(std::str::from_utf8(&bytes).unwrap()).unwrap();
        assert_eq!(strict.data.len(), 517);
        assert!(strict.header.warnings.is_empty());
        let digest: String = Sha256::digest(&bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        assert_eq!(digest, file["sha256"]);
    }
    let standards = standards();
    let mixtures = mixtures();
    let truth = json_file("truth.json");
    assert_eq!(truth.as_array().unwrap().len(), 100);
    for (i, mixture) in mixtures.iter().enumerate() {
        let weights: Vec<f64> = truth[i]["weights"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_f64().unwrap())
            .collect();
        assert_eq!(weights.len(), 3);
        assert!((weights.iter().sum::<f64>() - 1.0).abs() < 1e-14);
        assert!(weights.iter().all(|w| *w > 0.0 && *w < 1.0));
        assert_eq!(mixture.energy.as_ref().unwrap().len(), 517);
        let mut expected = DVector::zeros(517);
        for (standard, weight) in standards.iter().zip(&weights) {
            assert_eq!(standard.energy, mixture.energy);
            expected += standard.norm().unwrap() * *weight;
        }
        assert!(
            (expected - mixture.norm().unwrap()).amax() < 1e-14,
            "mixture {}",
            i + 1
        );
    }
}

#[test]
fn native_lcf_recovers_all_100_compositions() {
    let cfg = LcfConfig {
        range: config().range,
        ..LcfConfig::default()
    };
    let truth = json_file("truth.json");
    let standards = standards();
    let mut rows = Vec::new();
    let fits = lcf_batch(&mixtures(), &standards, &cfg);
    assert_eq!(fits.len(), 100);
    for (i, result) in fits.into_iter().enumerate() {
        let result = result.unwrap();
        for (j, coefficient) in result.weights.iter().enumerate() {
            assert!(
                (coefficient.weight - truth[i]["weights"][j].as_f64().unwrap()).abs() < 1e-8,
                "mixture {} component {j}: {}",
                i + 1,
                coefficient.weight
            );
        }
        assert!(result.r_factor < 1e-18);
        rows.push(json!({"mixture":i+1,"weights":result.weights,"r_factor":result.r_factor}));
    }
    report("native-lcf.json", &rows);
}

#[test]
fn recovered_component_enters_the_ordinary_exafs_fit_pipeline() {
    let data = mixtures();
    let result = mcr_als(
        &data,
        &McrConfig {
            range: Some((-150., 750.)),
            max_iterations: 1,
            ..config()
        },
    )
    .unwrap();
    // This checks downstream compatibility, not chemical identity or recovery
    // accuracy: a single ALS iteration is intentionally an unfinished estimate.
    let mut component = result.component_spectrum(0).unwrap();
    component.calc_background().unwrap().fft().unwrap();
    let k = DVector::from_column_slice(component.k().unwrap());
    let chi = DVector::from_column_slice(component.chi().unwrap());
    let path = feffpath(
        &format!(
            "{}/tests/testfiles/feffcu01.dat",
            env!("CARGO_MANIFEST_DIR")
        ),
        FeffFlavor::Feff85L,
    )
    .unwrap()
    .set_s02("amp")
    .set_sigma2(0.003);
    let fit = FeffFit::new()
        .data(&k, &chi)
        .add_path(path)
        .set_inits([("amp", 0.8)])
        .set_bounds("amp", 0.0, 2.0)
        .krange(2., 10.)
        .rrange(1., 3.)
        .fit()
        .unwrap();
    assert!(fit.chi_square.is_finite());
    assert!(fit.model_chi.iter().all(|v| v.is_finite()));
    assert_eq!(fit.datasets.len(), 1);
    assert_eq!(
        component.norm().unwrap().as_slice(),
        result.spectra.row(0).iter().copied().collect::<Vec<_>>()
    );
}

#[test]
fn native_pca_has_three_uncentered_and_two_centered_directions() {
    let mixtures = mixtures();
    let cfg = PcaConfig {
        range: config().range,
        center: false,
        ..PcaConfig::default()
    };
    let uncentered = pca_train(&mixtures, &cfg).unwrap();
    let centered = pca_train(
        &mixtures,
        &PcaConfig {
            center: true,
            ..cfg
        },
    )
    .unwrap();
    assert!(uncentered.variance_explained[2] > 1e-5);
    assert!(uncentered.variance_explained.iter().skip(3).sum::<f64>() < 1e-24);
    assert!(centered.variance_explained[1] > 1e-3);
    assert!(centered.variance_explained.iter().skip(2).sum::<f64>() < 1e-24);
    let transforms: Vec<_> = standards()
        .iter()
        .map(|s| {
            let two = uncentered.target_transform(s, 2).unwrap();
            let three = uncentered.target_transform(s, 3).unwrap();
            assert!(three.r_factor < 1e-24);
            assert!(two.r_factor > 1e-6);
            json!({"label":s.name,"two_components":two.r_factor,"three_components":three.r_factor})
        })
        .collect();
    report(
        "native-pca.json",
        &json!({"uncentered":uncentered,"centered":centered,"standard_projections":transforms}),
    );
}

#[test]
fn native_blind_mcr_reconstructs_mixtures_without_access_to_truth() {
    let mixtures = mixtures();
    for seed in [0, 71] {
        let result = mcr_als(
            &mixtures,
            &McrConfig {
                seed,
                max_iterations: 2000,
                ..config()
            },
        )
        .unwrap();
        assert!(
            result.relative_error < 1e-10,
            "seed {seed}: {} ({:?})",
            result.relative_error,
            result.termination
        );
        assert!(result.concentrations.iter().all(|v| *v >= -1e-10));
        for row in result.concentrations.row_iter() {
            assert!((row.sum() - 1.0).abs() < 1e-9);
        }
        assert!(result
            .objective_history
            .windows(2)
            .all(|w| w[1] <= w[0] + 1e-9));
        assert!(result.config.anchors.is_empty());
        assert!(result.config.initial_spectra.is_none());
        report(&format!("native-mcr-blind-{seed}.json"), &result);
    }
}

#[test]
fn explicitly_known_pure_samples_anchor_recovery_of_original_spectra() {
    let mut collection = mixtures();
    let standards = standards();
    collection.extend(standards.iter().cloned());
    let anchors = (0..3)
        .map(|i| McrAnchor {
            sample: 100 + i,
            weights: (0..3).map(|j| if i == j { 1.0 } else { 0.0 }).collect(),
        })
        .collect();
    let result = mcr_als(
        &collection,
        &McrConfig {
            anchors,
            max_iterations: 3000,
            tolerance: 1e-10,
            ..config()
        },
    )
    .unwrap();
    assert!(
        result.relative_error < 1e-12,
        "{} {:?}",
        result.relative_error,
        result.termination
    );
    let pca = pca_train(
        &collection,
        &PcaConfig {
            range: config().range,
            ..PcaConfig::default()
        },
    )
    .unwrap();
    for j in 0..3 {
        let expected = pca.data.row(100 + j);
        let relative = (result.spectra.row(j) - expected).norm() / expected.norm();
        assert!(relative < 1e-5, "component {j}: {relative}");
    }
    report("native-mcr-anchored.json", &result);
}
