//! Checks against the attributed data copied from the private gathering checkout.
//! Historical paths resolve to the canonical xas collection; no downloads are required.
use super::support::format_corpus_path;
use rexafs::io::*;
use sha2::{Digest, Sha256};

#[test]
fn all_222_copied_measurements_match_the_recorded_reader_outcomes() {
    // This snapshot distinguishes conversion candidates, manual mapping,
    // partial recovery and known rejections. Numerical assertions below are
    // independent checks; a passing snapshot is not complete format support.
    let coverage: serde_json::Value =
        serde_json::from_str(include_str!("format_corpus_coverage.json")).unwrap();
    let manifest_bytes = std::fs::read(format_corpus_path("manifest.json")).unwrap();
    assert_eq!(digest(&manifest_bytes), coverage["manifest_sha256"]);
    let manifest: serde_json::Value = serde_json::from_slice(&manifest_bytes).unwrap();
    let samples = manifest["samples"].as_array().unwrap();
    let records = coverage["records"].as_array().unwrap();
    assert_eq!(records.len(), 222);
    assert_eq!(samples.len(), records.len());
    let mut parsed = 0;
    let mut rejected = 0;
    for (sample, expected) in samples.iter().zip(records) {
        let path = expected["path"].as_str().unwrap();
        assert_eq!(sample["path"], path);
        assert_eq!(sample["sha256"], expected["sha256"], "{path}");
        let bytes = std::fs::read(format_corpus_path(path)).unwrap();
        assert_eq!(bytes.len() as u64, sample["bytes"].as_u64().unwrap());
        assert_eq!(digest(&bytes), expected["sha256"], "{path}");
        let result = parse_measurement(&bytes);
        if expected["status"] == "rejected" {
            let error =
                result.expect_err(&format!("{path}: now readable; qualify its new coverage"));
            assert_eq!(error.to_string(), expected["error"], "{path}");
            rejected += 1;
            continue;
        }
        let document = result.unwrap_or_else(|error| panic!("{path}: {error}"));
        parsed += 1;
        assert_eq!(document.format, expected["detected_format"], "{path}");
        assert_eq!(
            document.datasets.len() as u64,
            expected["datasets"].as_u64().unwrap(),
            "{path}"
        );
        assert_eq!(
            document.warnings.len() as u64,
            expected["warnings"].as_u64().unwrap(),
            "{path}"
        );
        let scans = expected["scans"].as_array().unwrap();
        assert_eq!(document.scans.len(), scans.len(), "{path}");
        let mut converted = 0;
        for (scan, expected_scan) in document.scans.iter().zip(scans) {
            assert_eq!(scan.id, expected_scan["id"], "{path}");
            assert_eq!(
                scan.columns.len() as u64,
                expected_scan["columns"].as_u64().unwrap(),
                "{path}"
            );
            let points = expected_scan["points"].as_u64().unwrap() as usize;
            assert!(
                scan.columns.iter().all(|c| c.values.len() == points),
                "{path}"
            );
            assert_eq!(
                scan.warnings.len() as u64,
                expected_scan["warnings"].as_u64().unwrap(),
                "{path}"
            );
            let signals = expected_scan["signals"].as_array().unwrap();
            assert_eq!(scan.signals.len(), signals.len(), "{path}");
            for (signal, expected_signal) in scan.signals.iter().zip(signals) {
                assert_eq!(signal.name, expected_signal["name"], "{path}");
                let result = scan.arrays(Some(&signal.mapping));
                if expected_signal["status"] == "converted" {
                    let (energy, mu) = result.unwrap_or_else(|error| panic!("{path}: {error}"));
                    assert_eq!(energy.len(), mu.len(), "{path}");
                    assert_eq!(
                        energy.len() as u64,
                        expected_signal["points"].as_u64().unwrap(),
                        "{path}"
                    );
                    converted += 1;
                } else {
                    assert_eq!(
                        result.unwrap_err().to_string(),
                        expected_signal["error"],
                        "{path}"
                    );
                }
            }
        }
        assert_eq!(
            converted,
            expected["converted_signals"].as_u64().unwrap(),
            "{path}"
        );
        let partial = document
            .warnings
            .iter()
            .any(|w| w.contains("cannot enumerate"));
        let status = if partial {
            "partial"
        } else if converted > 0 {
            "signal_choices"
        } else {
            "mapping_or_reduction_required"
        };
        assert_eq!(status, expected["status"], "{path}");
    }
    assert_eq!((parsed, rejected), (182, 40));
}

fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|v| format!("{v:02x}"))
        .collect()
}

fn original(path: &str, sha256: &str) -> String {
    let bytes = std::fs::read(format_corpus_path(path)).unwrap();
    let actual: String = Sha256::digest(&bytes)
        .iter()
        .map(|v| format!("{v:02x}"))
        .collect();
    assert_eq!(actual, sha256, "{path}");
    String::from_utf8(bytes).unwrap()
}

#[test]
fn aichi_fluorescence_uses_observed_angles_and_original_counts() {
    for (file, hash) in [
        (
            "3YSZ_aged-1.dat",
            "1f03faf8d7d2aa1f48b232f68ab61ac055a0a932a1fe636e8e0a09d0d47ec893",
        ),
        (
            "3YSZ_unaged-1.dat",
            "f02be8321f06c3b32654e02c6d3b00e67a8b575669859a6f13e5a8f468865400",
        ),
        (
            "8YSZ_unaged-1.dat",
            "3237556d3ce8117a9b49af3ff22061540788be015c4b60ed266380e5f100b454",
        ),
    ] {
        let text = original(
            &format!("samples/aichi-sr/unresolved-bl11s/zenodo-18522047/{file}"),
            hash,
        );
        let doc = parse_measurement(text.as_bytes()).unwrap();
        let scan = &doc.scans[0];
        assert_eq!(doc.format, "9809");
        assert!(scan.header.contains("D=  3.13553 A"));
        assert_eq!(
            scan.signals[0].mapping.signal,
            SignalConversion::Ratio {
                incident: 3,
                detectors: vec![4]
            }
        );
        let (energy, mu) = scan.arrays(None).unwrap();
        // Read numeric cells independently, after the original Offset header.
        let rows: Vec<Vec<f64>> = text
            .lines()
            .skip_while(|line| !line.trim_start().starts_with("Offset"))
            .skip(1)
            .filter(|line| !line.trim().is_empty())
            .map(|line| {
                line.split_whitespace()
                    .map(|v| v.parse().unwrap())
                    .collect()
            })
            .collect();
        assert_eq!(energy.len(), rows.len());
        assert_eq!(rows.len(), 4245);
        for (i, row) in rows.iter().enumerate() {
            // First-order Bragg law; hc in eV Å, d in Å, angle in degrees.
            let expected_energy = 12398.419843320026 / (2. * 3.13553 * row[1].to_radians().sin());
            assert!((energy[i] - expected_energy).abs() < 1e-8, "{file} row {i}");
            assert!((mu[i] - row[4] / row[3]).abs() < 1e-12, "{file} row {i}");
        }
    }
}

#[test]
fn samba_exports_preserve_stored_xmu_and_acquisition_order() {
    for (file, hash) in [
        (
            "Aeschynite_0001.txt",
            "f5f0a138b261fa8d608044104b43218514d999ed8df7449db95dbd0af56ad033",
        ),
        (
            "Ba_pcl_0001.txt",
            "9d0194f02fab50cea5c85798053b534b99c0b25d4ae0fc40bee72e0eeac58e5a",
        ),
        (
            "Nb_anatase_10pc_0001.txt",
            "91c4b8e25080bed54baa685a33ca61408857bf010926189a54d5945e80f52e0f",
        ),
    ] {
        let text = original(&format!("samples/soleil/samba/zenodo-7801896/{file}"), hash);
        let doc = parse_measurement(text.as_bytes()).unwrap();
        let scan = &doc.scans[0];
        assert_eq!(scan.columns[2].name, "XMU");
        assert_eq!(
            scan.signals[0].mapping.signal,
            SignalConversion::Direct { column: 2 }
        );
        let (energy, mu) = scan.arrays(None).unwrap();
        let rows: Vec<Vec<f64>> = text
            .lines()
            .filter(|line| !line.starts_with('#') && !line.trim().is_empty())
            .map(|line| {
                line.split_whitespace()
                    .map(|v| v.parse().unwrap())
                    .collect()
            })
            .collect();
        assert_eq!(energy.len(), rows.len());
        for (i, row) in rows.iter().enumerate() {
            assert_eq!(scan.columns.len(), row.len());
            for (column, value) in scan.columns.iter().zip(row) {
                assert!(
                    (column.values[i] - value).abs() <= 1e-12 * value.abs().max(1.),
                    "{file} row {i}"
                );
            }
            assert!((energy[i] - row[0]).abs() < 1e-8);
            assert!((mu[i] - row[2]).abs() < 1e-12);
        }
    }
}

#[test]
fn pirx_spec_retains_scan_boundaries_and_requires_detector_mapping() {
    let file = "samples/solaris/pirx/rodbuk-58IJY5/AFJ_CuALD_650c_Si.dat";
    let text = original(
        file,
        "6c98e3f2f8994230c11bad1557431691d66ce3841c6e2c26ddb36f2027cb8dd2",
    );
    let doc = parse_measurement(text.as_bytes()).unwrap();
    let blocks: Vec<_> = text.split("#S ").skip(1).collect();
    assert_eq!(doc.format, "spec");
    assert_eq!(doc.scans.len(), 27);
    assert_eq!(doc.scans.len(), blocks.len());
    for (scan, block) in doc.scans.iter().zip(blocks) {
        assert!(scan.signals.is_empty());
        assert!(scan.arrays(None).is_err());
        // Preserve every numeric cell in each SPEC scan independently of any
        // guessed detector role. This does not establish TEY/PFY calibration.
        let rows: Vec<Vec<f64>> = block
            .lines()
            .skip_while(|line| !line.starts_with("#L "))
            .skip(1)
            .filter(|line| !line.starts_with('#') && !line.trim().is_empty())
            .map(|line| {
                line.split_whitespace()
                    .map(|v| v.parse().unwrap())
                    .collect()
            })
            .collect();
        assert_eq!(scan.columns[0].values.len(), rows.len());
        for (i, row) in rows.iter().enumerate() {
            assert_eq!(scan.columns.len(), row.len());
            for (column, value) in scan.columns.iter().zip(row) {
                assert!(
                    (column.values[i] - value).abs() <= 1e-12 * value.abs().max(1.),
                    "{} row {i}",
                    scan.id
                );
            }
        }
    }
}
