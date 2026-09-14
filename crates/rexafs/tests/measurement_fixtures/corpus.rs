//! Universal reader regressions against the attributed, unchanged source corpus.
//! Excluded together with the corpus from the crate archive.
use super::support::xas_root as root;
use rexafs::io::*;
#[test]
fn every_retained_measurement_is_readable() {
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root().join("manifest.json")).unwrap()).unwrap();
    // Qualification snapshot of observed counts, separate from the original
    // data manifest. Independent numerical assertions follow below.
    let coverage = include_str!("coverage.csv");
    let expected: std::collections::BTreeMap<_, _> = coverage
        .lines()
        .skip(1)
        .map(|line| {
            let cells: Vec<_> = line.splitn(7, ',').collect();
            (cells[0], cells)
        })
        .collect();
    let mut errors = Vec::new();
    for sample in manifest["samples"].as_array().unwrap() {
        let path = sample["path"].as_str().unwrap();
        match read_measurement(root().join(path)) {
            Ok(doc) => {
                let row = &expected[path];
                assert_eq!(doc.format, row[1], "{path}");
                let counts = |field: &str| -> Vec<usize> {
                    if field.is_empty() {
                        vec![]
                    } else {
                        field.split(';').map(|n| n.parse().unwrap()).collect()
                    }
                };
                assert_eq!(
                    doc.scans
                        .iter()
                        .map(|s| s.columns.first().map_or(0, |c| c.values.len()))
                        .collect::<Vec<_>>(),
                    counts(row[2]),
                    "{path}"
                );
                assert_eq!(
                    doc.scans
                        .iter()
                        .map(|s| s.signals.len())
                        .collect::<Vec<_>>(),
                    counts(row[3]),
                    "{path}"
                );
                assert_eq!(
                    doc.datasets.len(),
                    row[4].parse::<usize>().unwrap(),
                    "{path}"
                );
                assert_eq!(
                    doc.warnings.len(),
                    row[5].parse::<usize>().unwrap(),
                    "{path}"
                );
                assert_eq!(row[6], sample["source_url"].as_str().unwrap(), "{path}");
                assert!(!doc.scans.is_empty() || !doc.datasets.is_empty(), "{path}");
                for scan in &doc.scans {
                    assert!(!scan.columns.is_empty(), "{path}");
                    assert!(
                        scan.columns
                            .iter()
                            .all(|c| c.values.len() == scan.columns[0].values.len()),
                        "{path}"
                    );
                }
                println!(
                    "{} {} scans={} points={:?} choices={:?} datasets={}",
                    path,
                    doc.format,
                    doc.scans.len(),
                    doc.scans
                        .iter()
                        .map(|s| s.columns[0].values.len())
                        .collect::<Vec<_>>(),
                    doc.scans
                        .iter()
                        .map(|s| s.signals.len())
                        .collect::<Vec<_>>(),
                    doc.datasets.len()
                );
            }
            Err(error) => errors.push(format!("{path}: {error}")),
        }
    }
    assert!(
        errors.is_empty(),
        "{} files failed:\n{}",
        errors.len(),
        errors.join("\n")
    );
}

#[test]
fn bmm_reference_foil_exports_keep_the_stored_signal() {
    // Independently read from the three original NIST files. The header says
    // xmu = ln(It/Ir); using I0/It would produce a different spectrum.
    for (file, first, last, mu) in [
        ("Fe-K-IronFoil.xdi", 6912.001, 7964.837, 1.981854),
        ("Au-L3-GoldFoil.xdi", 11718.696, 12771.623, 0.505607),
        ("As-K-ArsenicTrioxide.xdi", 11666.678, 12719.586, 0.423532),
    ] {
        let doc = read_measurement(
            root()
                .join("samples/nsls-ii/6-bm-bmm/bmm-standards")
                .join(file),
        )
        .unwrap();
        let scan = &doc.scans[0];
        let (e, y) = scan.arrays(None).unwrap();
        assert_eq!(e.len(), 393);
        assert_eq!(e[0], first);
        assert_eq!(e[392], last);
        assert_eq!(y[0], mu);
        let it = scan.columns[5].values[0];
        let ir = scan.columns[6].values[0];
        assert!((y[0] - (it / ir).ln()).abs() < 1e-6);
        assert!(scan.metadata["scan.plot_hint"].starts_with("ln(It/Ir)"));
    }
}

#[test]
fn historical_and_wrapped_tables_retain_original_values() {
    // Point counts and first raw cells read independently from the source files.
    for (path, points, column, value) in [
        (
            "samples/aps/10-bm/xraylarch/APS10BM_2019.dat",
            3464,
            0,
            6389.9975,
        ),
        (
            "samples/aps/12-bm/xraylarch/APS12BM_2019.dat",
            492,
            0,
            6.339,
        ),
        (
            "samples/aps/13-bm-d/xraylarch/APS13ID_2008.dat",
            469,
            2,
            101896.7,
        ),
        (
            "samples/photon-factory/bl-12c/xraylarch/PFBL12C_2005.dat",
            818,
            1,
            9.44420,
        ),
        (
            "samples/ritsumeikan-sr/bl-11/mdr-97686182/21033010.dat",
            223,
            12,
            509.8407,
        ),
        (
            "samples/lnls-uvx/unspecified-xas/demeter/lnls.dat",
            999,
            0,
            6919.999938111,
        ),
        (
            "samples/unspecified/lytle-archive/demeter/lytle.dat",
            480,
            0,
            85290.,
        ),
    ] {
        let doc = read_measurement(root().join(path)).unwrap();
        let s = &doc.scans[0];
        assert_eq!(s.columns[0].values.len(), points, "{path}");
        assert!(
            (s.columns[column].values[0] - value).abs() < 1e-9,
            "{path}: {}",
            s.columns[column].values[0]
        );
    }
}

#[test]
fn contradictory_headers_do_not_offer_automatic_arithmetic() {
    for path in [
        "samples/cls/bioxas-s/cls-xasdb/Fe-Foil_2024_04_07_scan1_idcj28n9.dat",
        "candidates/refxas/soleil/samba/xafsdb-webserver/Pt_foil Pt L3 SAMBA Soleil.txt",
    ] {
        let d = read_measurement(root().join(path)).unwrap();
        assert!(d.scans[0].signals.is_empty());
        assert!(d.scans[0].arrays(None).is_err());
        assert!(d.scans[0]
            .warnings
            .iter()
            .any(|w| w.to_lowercase().contains("conflict")));
    }
}

#[test]
fn detector_projections_are_not_reported_as_absorption_spectra() {
    for file in [
        "scan-24212_eiger_streaming.h5",
        "scan-24213_eiger_streaming.h5",
    ] {
        let d = read_measurement(
            root()
                .join("samples/max-iv/balder-unconfirmed/parseq-xas")
                .join(file),
        )
        .unwrap();
        assert!(d.scans.is_empty());
        assert!(d.datasets.iter().any(|d| d.shape.len() == 2));
        assert!(d
            .datasets
            .iter()
            .all(|d| d.values.len() as u64 == d.shape.iter().product::<u64>()));
    }
}
