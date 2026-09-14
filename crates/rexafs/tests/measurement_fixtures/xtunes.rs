//! Real XTUNES saves from the separately attributed source-checkout fixtures.
use super::support::session_root;
use rexafs::io::read_measurement;

#[test]
fn every_native_save_retains_absorption_and_independent_saved_tables() {
    let root = session_root("xtunes");
    for (file, counts, back_rows) in [
        ("pfbl12c-bg.xts", vec![818], Some(256)),
        ("pfbl12c-ft.xts", vec![818], Some(196)),
        ("pf9a-bg.xts", vec![1420], Some(261)),
        ("pf9a-ft.xts", vec![1420], Some(181)),
        ("two-analyzed.xtsp", vec![818, 1420], Some(0)),
        ("mixed-raw-analyzed.xtsp", vec![818, 1420], None),
    ] {
        let doc = read_measurement(root.join(file)).unwrap();
        assert_eq!(doc.scans.len(), counts.len(), "{file}");
        for (scan, count) in doc.scans.iter().zip(counts) {
            let (e, mu) = scan.arrays(None).unwrap();
            assert_eq!(e.len(), count);
            let expected = if count == 818 {
                (12049.083108, 13243.311414, -0.851609, -0.608707)
            } else {
                (6606.167326, 8211.094542, 0.007819, 0.126127)
            };
            assert_eq!(
                (e[0], *e.last().unwrap(), mu[0], *mu.last().unwrap()),
                expected
            );
            assert!(scan.metadata["source_path"].starts_with("C:\\"));
            assert!(scan.to_spectrum(None).unwrap().norm().is_none());
        }
        if let Some(n) = back_rows {
            let table = doc
                .datasets
                .iter()
                .find(|d| d.path == "/record_1/BackFT Plot")
                .unwrap();
            assert_eq!(table.shape, vec![n, 3]);
        }
        for table in &doc.datasets {
            assert_eq!(
                table.values.len() as u64,
                table.shape.iter().product::<u64>()
            );
            assert!(table.values.iter().all(|v| v.is_finite()));
            if table.path.ends_with("/FT Plot") {
                assert_eq!(table.shape, vec![1024, 4]);
                for row in table.values.as_chunks::<4>().0 {
                    assert!((row[1].hypot(row[2]) - row[3]).abs() < 1.5e-6);
                }
            }
        }
        if file.ends_with(".xts") {
            let p: Vec<(String, String, String)> =
                serde_json::from_str(&doc.scans[0].metadata["ordered_parameters"]).unwrap();
            assert!(p.iter().any(|(s, k, _)| s == "#BG" && k == "dk"));
            assert!(p.iter().any(|(s, k, _)| s == "#FT" && k == "dk"));
            let cf_n: Vec<_> = p
                .iter()
                .filter(|(s, k, _)| s == "#CF" && k.trim() == "Func1 N")
                .collect();
            assert_eq!(
                cf_n.len(),
                2,
                "parameter and uncertainty must not overwrite each other"
            );
        }
    }
}
