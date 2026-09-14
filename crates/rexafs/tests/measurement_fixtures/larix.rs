//! Supplied Larch 2026.3.1 sessions and independent pre-serialization expectations.
//! Excluded from the crate archive with the attributed original fixtures.
use super::support::session_root;
use base64::Engine;
use rexafs::io::*;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::path::PathBuf;

fn hex_digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn root() -> PathBuf {
    session_root("larix")
}
fn read(name: &str) -> Measurement {
    read_measurement(root().join("fixtures/valid").join(name)).unwrap()
}
fn escape(s: &str) -> String {
    s.replace('~', "~0").replace('/', "~1")
}

fn arrays(expected: &Value, path: &str, document: &Measurement, count: &mut usize) {
    if expected["kind"] == "ndarray" {
        let dataset = document
            .datasets
            .iter()
            .find(|d| d.path == path)
            .unwrap_or_else(|| panic!("Missing {path}"));
        assert_eq!(
            serde_json::to_value(&dataset.shape).unwrap(),
            expected["shape"],
            "{path}"
        );
        assert_eq!(
            dataset.attributes["larix.dtype"],
            expected["dtype"].as_str().unwrap(),
            "{path}"
        );
        let dtype = dataset.attributes["larix.dtype"].as_bytes();
        let width: usize = std::str::from_utf8(&dtype[2..]).unwrap().parse().unwrap();
        let component = if dtype[1] == b'c' { width / 2 } else { width };
        let mut bytes = base64::engine::general_purpose::STANDARD
            .decode(&dataset.attributes["larix.bytes_base64"])
            .unwrap();
        if dtype[0] == b'>' {
            for word in bytes.chunks_exact_mut(component) {
                word.reverse();
            }
        }
        assert_eq!(
            hex_digest(&bytes),
            expected["sha256_le_c"].as_str().unwrap(),
            "{path}"
        );
        // Independently reconstruct every floating/Boolean view, not just the
        // archived bytes. Integer samples below also check the documented f64 view.
        if matches!(dtype[1], b'f' | b'c' | b'b') {
            let mut numeric = Vec::new();
            for (i, &real) in dataset.values.iter().enumerate() {
                let values = std::iter::once(real).chain(dataset.imaginary.as_ref().map(|v| v[i]));
                for value in values {
                    match component {
                        1 => numeric.push(u8::from(value != 0.)),
                        4 => numeric.extend((value as f32).to_le_bytes()),
                        8 => numeric.extend(value.to_le_bytes()),
                        _ => unreachable!(),
                    }
                }
            }
            assert_eq!(
                hex_digest(&numeric),
                expected["sha256_le_c"].as_str().unwrap(),
                "numeric {path}"
            );
        }
        for sample in expected["samples"].as_array().unwrap() {
            let i = sample["index"].as_u64().unwrap() as usize;
            let value = &sample["value"];
            if dtype[1] == b'c' {
                assert_eq!(dataset.values[i], value[0].as_f64().unwrap(), "{path}");
                assert_eq!(
                    dataset.imaginary.as_ref().unwrap()[i],
                    value[1].as_f64().unwrap(),
                    "{path}"
                );
            } else {
                let value = value
                    .as_f64()
                    .or_else(|| value.as_bool().map(|b| if b { 1. } else { 0. }))
                    .unwrap();
                assert_eq!(dataset.values[i], value, "{path}");
                assert!(dataset.imaginary.is_none());
            }
        }
        *count += 1;
    } else if let Some(attributes) = expected.get("attributes") {
        for (key, child) in attributes.as_object().unwrap() {
            arrays(child, &format!("{path}/{}", escape(key)), document, count);
        }
    } else if let Some(map) = expected.as_object() {
        for (key, child) in map {
            arrays(child, &format!("{path}/{}", escape(key)), document, count);
        }
    } else if let Some(list) = expected.as_array() {
        for (i, child) in list.iter().enumerate() {
            arrays(child, &format!("{path}/{i}"), document, count);
        }
    }
}

#[test]
fn every_valid_session_matches_all_recorded_arrays() {
    let expected: Value =
        serde_json::from_slice(&std::fs::read(root().join("expected.json")).unwrap()).unwrap();
    let manifest: Value =
        serde_json::from_slice(&std::fs::read(root().join("manifest.json")).unwrap()).unwrap();
    let (mut files, mut total) = (0, 0);
    for entry in manifest["files"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| e["category"] == "valid")
    {
        let path = entry["path"].as_str().unwrap();
        let document = read_measurement(root().join(path)).unwrap();
        assert_eq!(document.format, "larix");
        let record = &expected["sessions"][path];
        let mut count = 0;
        for (symbol, value) in record["symbols"].as_object().unwrap() {
            arrays(
                value,
                &format!("/{}", escape(symbol)),
                &document,
                &mut count,
            );
        }
        assert_eq!(
            count,
            document.datasets.len(),
            "Unexpected untested arrays in {path}"
        );
        let history: Value =
            serde_json::from_str(&document.metadata["larix.command_history"]).unwrap();
        assert_eq!(history, record["command_history"], "{path}");
        let automatic: Vec<_> = document
            .scans
            .iter()
            .filter(|s| s.signals.len() == 1)
            .map(|s| s.metadata["larix.symbol"].clone())
            .collect();
        assert_eq!(
            serde_json::to_value(automatic).unwrap(),
            entry["expected_absorption_groups"],
            "{path}"
        );
        for scan in document.scans.iter().filter(|s| s.signals.len() == 1) {
            let (energy, mu) = scan.arrays(None).unwrap();
            assert_eq!(energy, scan.columns[0].values);
            assert_eq!(mu, scan.columns[1].values);
        }
        total += count;
        files += 1;
    }
    assert_eq!((files, total), (15, 384));
}

#[test]
fn damaged_and_inconsistent_sessions_fail_at_their_documented_layer() {
    for (name, context) in [
        ("truncated-gzip", "gzip"),
        ("bad-magic", "numeric"),
        ("invalid-json", "JSON"),
        ("bad-array-shape", "array"),
        ("mismatched-energy-mu", "mapping"),
        ("missing-group-reference", "reference"),
    ] {
        let error = read_measurement(root().join(format!("fixtures/invalid/{name}.larix")))
            .unwrap_err()
            .to_string();
        assert!(error.contains(context), "{name}: {error}");
    }
}

#[test]
fn raw_rows_complex_results_and_unmapped_groups_remain_distinct() {
    let raw = read("pf9a-raw.larix");
    let (energy, _) = raw.scans[0].arrays(None).unwrap();
    assert_eq!(energy.len(), 1426);
    assert_eq!(energy.windows(2).filter(|v| v[0] == v[1]).count(), 6);
    let document = read("two-analyzed.larix");
    assert_eq!(document.scans.len(), 2);
    assert_eq!(document.scans[0].arrays(None).unwrap().0.len(), 818);
    assert_eq!(document.scans[1].arrays(None).unwrap().0.len(), 1420);
    let chir = document
        .datasets
        .iter()
        .find(|d| d.path.ends_with("/chir"))
        .unwrap();
    assert_eq!(chir.shape, [326]);
    assert_eq!(chir.imaginary.as_ref().unwrap().len(), 326);
    assert!(document
        .dataset_scan(&[chir.path.replace("/chir", "/r"), chir.path.clone()])
        .is_err());
    for name in ["chi-only.larix", "non-xas.larix"] {
        let document = read(name);
        assert!(document.scans.iter().all(|s| s.signals.is_empty()));
        assert!(document.scans[0].arrays(None).is_err());
    }
    let empty = read("empty.larix");
    assert!(empty.scans.is_empty() && empty.datasets.is_empty());
    assert!(empty.metadata["larix.session_text"].contains("##</Symbols>"));
}

#[test]
fn metadata_integer_precision_and_plain_text_survive_without_evaluation() {
    let document = read("typed-metadata.larix");
    assert_eq!(document.scans[0].label, "Fe foil — 日本語 μ(E).dat");
    let header = &document.scans[0].header;
    assert!(
        header.contains("first entry") && header.contains("second entry; preserve repeated keys")
    );
    assert!(
        header.contains("2 * amplitude") && header.contains("-Infinity") && header.contains("NaN")
    );
    let counts = document
        .datasets
        .iter()
        .find(|d| d.path.ends_with("/metadata/counts"))
        .unwrap();
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(&counts.attributes["larix.bytes_base64"])
        .unwrap();
    assert_eq!(
        i64::from_le_bytes(bytes[8..16].try_into().unwrap()),
        9007199254740993
    );
    assert!(document.warnings.iter().any(|w| w.contains("precision")));
    let plain = read("plain-session.larix");
    assert_eq!(plain.scans, document.scans);
    assert_eq!(plain.datasets, document.datasets);
    assert_eq!(plain.metadata, document.metadata);
}

#[test]
fn every_incomplete_gzip_prefix_is_rejected_and_complete_source_recovers() {
    let source = std::fs::read(root().join("fixtures/valid/typed-metadata.larix")).unwrap();
    // Exercise every truncation point of this small real compressed session,
    // including partial headers, deflate blocks and CRC/length trailer fields.
    for end in 0..source.len() {
        assert!(
            parse_measurement(&source[..end]).is_err(),
            "accepted prefix {end}"
        );
    }
    assert_eq!(parse_measurement(&source).unwrap().format, "larix");
}
