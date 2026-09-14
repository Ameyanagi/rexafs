//! Original KEK QD measurements, retained for academic, nonmilitary testing.
//! See fixtures/xas/candidates/kek-pf/README.md for usage terms and attribution.
use super::support::xas_root;
use rexafs::io::*;

/// Independently load the five-column source body, without the shared reader.
fn original(path: &str, points: usize) -> (Measurement, Vec<[f64; 5]>) {
    let path = xas_root().join("candidates/kek-pf").join(path);
    let text = std::fs::read_to_string(&path).unwrap();
    let rows: Vec<[f64; 5]> = text
        .lines()
        .skip_while(|line| !line.trim_start().starts_with("Offset"))
        .skip(1)
        .map(|line| line.trim_matches(|c: char| c.is_whitespace() || c == '\u{1a}'))
        .filter(|line| !line.is_empty())
        .map(|line| {
            line.split_whitespace()
                .map(|cell| cell.parse::<f64>().unwrap())
                .collect::<Vec<_>>()
                .try_into()
                .unwrap()
        })
        .collect();
    assert_eq!(rows.len(), points);
    let doc = read_measurement(path).unwrap();
    assert_eq!(doc.format, "9809");
    assert_eq!(doc.scans.len(), 1);
    let scan = &doc.scans[0];
    assert_eq!(scan.columns.len(), 5);
    for (column, values) in scan.columns.iter().enumerate() {
        assert_eq!(values.values.len(), points);
        for (row, raw) in rows.iter().enumerate() {
            assert_eq!(
                values.values[row], raw[column],
                "row {row}, column {column}"
            );
        }
    }
    for (column, unit) in [Some("deg"), Some("deg"), Some("s"), None, None]
        .into_iter()
        .enumerate()
    {
        assert_eq!(scan.columns[column].units.as_deref(), unit);
    }
    (doc, rows)
}

#[test]
fn kek_qd_transmission_preserves_raw_columns_and_converts_recorded_angles() {
    for (path, points, spacing) in [
        ("bl-12c/fe002_0.qd", 3917, 3.13551),
        ("bl-9c/cu_foil_0.qd", 5135, 3.13551),
        ("bl-9c/Fe003_0.qd", 4000, 3.13551),
        ("ar-nw10a/sr01_0.qd", 3999, 1.63747),
    ] {
        let (doc, rows) = original(path, points);
        let scan = &doc.scans[0];
        assert_eq!(scan.signals.len(), 1, "{path}");
        assert_eq!(
            scan.signals[0].mapping,
            SpectrumMapping {
                energy_column: 1,
                energy: EnergyConversion::Bragg {
                    d_spacing: spacing,
                    degrees_per_unit: 1.,
                },
                signal: SignalConversion::Transmission {
                    incident: 3,
                    transmitted: 4,
                },
            }
        );
        let (energy, mu) = scan.arrays(None).unwrap();
        for (index, row) in rows.iter().enumerate() {
            // First-order Bragg energy in eV; spacing is in angstroms and the
            // original observed angle (column 1) is in degrees. This verifies
            // the arithmetic, not monochromator calibration or data quality.
            let expected = 12398.419843320026 / (2. * spacing * row[1].to_radians().sin());
            assert!(
                (energy[index] - expected).abs() < 1e-8,
                "{path} row {index}"
            );
            assert!((mu[index] - (row[3] / row[4]).ln()).abs() < 1e-13);
        }
    }
}

#[test]
fn kek_bl9a_conflicting_modes_require_explicit_fluorescence_arithmetic() {
    let (doc, rows) = original("bl-9a/ScotchTape02_0.qd", 452);
    let scan = &doc.scans[0];
    assert!(scan.header.contains("Fluorescence( 3)"));
    assert!(scan.header.lines().any(|line| {
        line.split_whitespace().collect::<Vec<_>>() == ["Mode", "0", "0", "1", "2"]
    }));
    assert!(scan.signals.is_empty());
    assert!(scan.arrays(None).is_err());
    assert!(scan.warnings.iter().any(|warning| warning.contains(
        "fluorescence acquisition conflicts with transmission-only detector Mode fields"
    )));
    let (energy, signal) = scan
        .arrays(Some(&SpectrumMapping {
            energy_column: 1,
            energy: EnergyConversion::Bragg {
                d_spacing: 3.13551,
                degrees_per_unit: 1.,
            },
            signal: SignalConversion::Ratio {
                incident: 3,
                detectors: vec![4],
            },
        }))
        .unwrap();
    for (index, row) in rows.iter().enumerate() {
        let expected = 12398.419843320026 / (2. * 3.13551 * row[1].to_radians().sin());
        assert!((energy[index] - expected).abs() < 1e-8);
        assert!((signal[index] - row[4] / row[3]).abs() < 1e-13);
    }
}
