//! Public reader contract tests use small synthetic inputs, without the corpus.
use rexafs::io::*;
fn direct(energy: EnergyConversion) -> SpectrumMapping {
    SpectrumMapping {
        energy_column: 0,
        energy,
        signal: SignalConversion::Direct { column: 1 },
    }
}
#[test]
fn csv_units_and_owned_order() {
    let doc = parse_measurement(b"energy (keV),mu\n7.2,2\n7.1,1\n7.1,3\n").unwrap();
    let scan = &doc.scans[0];
    let (e, m) = scan.arrays(None).unwrap();
    assert_eq!(e, vec![7200., 7100., 7100.]);
    assert_eq!(m, vec![2., 1., 3.]);
    let spectrum = scan.to_spectrum(None).unwrap();
    assert_eq!(spectrum.energy.unwrap().as_slice(), &[7100., 7100., 7200.]);
    assert!(spectrum.normalization.is_none());
    assert_eq!(scan.columns[0].values[0], 7.2);
}

#[test]
fn qxafs_scan_plan_units_do_not_leak_into_detector_columns() {
    // Synthetic 9809 QXAFS layout; no third-party measurement rows are copied.
    let text = b"9809 test\n Mono : Si(111) D= 3.1 A\n BLtest Transmission(2) Repetition= 0 Points= 2\n Param file : QXAFS(SYNC) angle axis (*) Block = 1\n Block Init-Ang Final-Ang Step/deg Time/s Num\n 1 17 16 *** 1 2\n Angle(c) Angle(o) time/s 2 3\n Mode 0 0 1 2\n Offset 0 0 0 0\n 17 17 1 10 2\n 16 16 1 20 4\n";
    let doc = parse_measurement(text).unwrap();
    let scan = &doc.scans[0];
    let units: Vec<_> = scan.columns.iter().map(|c| c.units.as_deref()).collect();
    assert_eq!(units, vec![Some("deg"), Some("deg"), Some("s"), None, None]);
    assert!(scan.header.contains("QXAFS(SYNC)"));
    let (energy, mu) = scan.arrays(None).unwrap();
    assert!((energy[0] - 12398.419843320026 / (6.2 * 17_f64.to_radians().sin())).abs() < 1e-8);
    assert!((mu[0] - 5_f64.ln()).abs() < 1e-14);
}

#[test]
fn qxafs_conflicting_fluorescence_and_transmission_modes_require_mapping() {
    let doc = parse_measurement(b"9809 test\n Mono : D= 3.1 A\n BLtest Fluorescence(3) Repetition= 0 Points= 2\n Mode 0 0 1 2\n Offset 0 0 0 0\n 17 17 1 10 2\n 16 16 1 20 4\n").unwrap();
    let scan = &doc.scans[0];
    assert!(scan.signals.is_empty());
    assert!(scan.arrays(None).is_err());
    assert!(scan
        .warnings
        .iter()
        .any(|w| w.contains("fluorescence acquisition conflicts")));
    assert_eq!(scan.metadata["9809_modes"], "[0.0,0.0,1.0,2.0]");
    let mapping = SpectrumMapping {
        energy_column: 1,
        energy: EnergyConversion::Bragg {
            d_spacing: 3.1,
            degrees_per_unit: 1.,
        },
        signal: SignalConversion::Ratio {
            incident: 3,
            detectors: vec![4],
        },
    };
    let (_, signal) = scan.arrays(Some(&mapping)).unwrap();
    assert_eq!(signal, vec![0.2, 0.2]);
}
#[test]
fn ambiguous_signals_require_a_choice() {
    let doc = parse_measurement(b"# energy i0 it if\n7100 10 2 3\n7101 20 4 8\n").unwrap();
    let s = &doc.scans[0];
    assert_eq!(s.signals.len(), 2);
    assert!(s.arrays(None).is_err());
    let (_, y) = s.arrays(Some(&s.signals[0].mapping)).unwrap();
    assert!((y[0] - 5f64.ln()).abs() < 1e-14);
    let (_, y) = s.arrays(Some(&s.signals[1].mapping)).unwrap();
    assert_eq!(y, vec![0.3, 0.4]);
}

#[test]
fn gzip_integrity_includes_the_complete_container() {
    use std::io::Write;
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(b"energy mu\n7100 1\n7101 2\n").unwrap();
    let source = encoder.finish().unwrap();
    assert!(parse_measurement(&source).is_ok());
    for suffix in [b"extra measurement".as_slice(), source.as_slice()] {
        let mut combined = source.clone();
        combined.extend_from_slice(suffix);
        assert!(parse_measurement(&combined)
            .unwrap_err()
            .to_string()
            .contains("trailing data"));
    }
    let mut corrupt = source.clone();
    let crc_start = corrupt.len() - 8;
    corrupt[crc_start] ^= 1;
    assert!(parse_measurement(&corrupt)
        .unwrap_err()
        .to_string()
        .contains("gzip"));
    assert!(parse_measurement(&source).is_ok());
}
#[test]
fn generic_unknown_columns_remain_available() {
    let doc = parse_measurement(b"1 2 3\n4 5 6\n").unwrap();
    assert!(doc.scans[0].signals.is_empty());
    assert_eq!(
        doc.scans[0]
            .arrays(Some(&direct(EnergyConversion::Kev)))
            .unwrap(),
        (vec![1000., 4000.], vec![2., 5.])
    );
}
#[test]
fn malformed_rows_are_never_silently_dropped() {
    for bytes in [
        b"# energy mu\n7100 1\n7101 bad\n7102 3\n".as_slice(),
        b"7100 1\n7101 2 3\n",
        b"# energy mu\n7100 bad\n7101 2\n",
        b"energy,mu\n7100,\n7101,2\n",
        b"7100,1\n7101,\n",
        b"7100 1\n7101\n",
    ] {
        assert!(parse_measurement(bytes).is_err());
    }
}
#[test]
fn conversion_validates_mutated_documents_and_indices() {
    let mut doc = parse_measurement(b"7100 1\n7101 2\n").unwrap();
    let s = &mut doc.scans[0];
    let mut m = direct(EnergyConversion::Ev);
    m.energy_column = 100;
    assert!(s.arrays(Some(&m)).is_err());
    m.energy_column = 0;
    s.columns[1].values.pop();
    assert!(s.arrays(Some(&m)).is_err());
    s.columns[1].values.push(f64::NAN);
    assert!(s.arrays(Some(&m)).is_err());
}
#[test]
fn detector_arithmetic_handles_polarity_and_extreme_values() {
    let doc =
        parse_measurement(b"# energy i0 it\n7100 -1e-300 -2e-300\n7101 1e300 1e-300\n").unwrap();
    let (_, y) = doc.scans[0].arrays(None).unwrap();
    assert!((y[0] + 2f64.ln()).abs() < 1e-12);
    assert!(y[1].is_finite());
    for row in ["7100 1 0", "7100 -1 1"] {
        let d = parse_measurement(format!("# energy i0 it\n{row}\n").as_bytes()).unwrap();
        let m = SpectrumMapping {
            energy_column: 0,
            energy: EnergyConversion::Ev,
            signal: SignalConversion::Transmission {
                incident: 1,
                transmitted: 2,
            },
        };
        assert!(d.scans[0].arrays(Some(&m)).is_err());
    }
}
#[test]
fn bragg_conversion_uses_declared_calibration() {
    let d = parse_measurement(b"30 2\n45 3\n").unwrap();
    let m = direct(EnergyConversion::Bragg {
        d_spacing: 3.1355,
        degrees_per_unit: 1.,
    });
    let (e, _) = d.scans[0].arrays(Some(&m)).unwrap();
    assert!((e[0] - 12398.419843320026 / 3.1355).abs() < 1e-10);
    let m = direct(EnergyConversion::Bragg {
        d_spacing: 0.,
        degrees_per_unit: 1.,
    });
    assert!(d.scans[0].arrays(Some(&m)).is_err());
}
#[test]
fn spec_preserves_scan_boundaries() {
    let d=parse_measurement(b"#F file\n#S 1 scan\n#L energy  mu\n7100 1\n7101 2\n#S 2 scan\n#L energy  mu\n7100 3\n7101 4\n").unwrap();
    assert_eq!(d.format, "spec");
    assert_eq!(d.scans.len(), 2);
    assert_eq!(d.scans[1].columns[1].values, vec![3., 4.]);
}
#[test]
fn hdf5_keeps_detector_shapes_and_discovers_scan_units() {
    use hdf5_pure::{AttrValue, FileBuilder};
    let mut b = FileBuilder::new();
    b.create_dataset("energy")
        .with_f64_data(&[7.1, 7.2])
        .set_attr("units", AttrValue::String("keV".into()));
    b.create_dataset("mu").with_f64_data(&[1., 2.]);
    b.create_dataset("image")
        .with_f64_data(&[1., 2., 3., 4.])
        .with_shape(&[2, 2]);
    let d = parse_measurement(&b.finish().unwrap()).unwrap();
    assert_eq!(d.format, "hdf5");
    assert_eq!(d.scans.len(), 1);
    assert_eq!(d.datasets.len(), 3);
    assert_eq!(d.scans[0].arrays(None).unwrap().0, vec![7100., 7200.]);
    let image = d.datasets.iter().find(|d| d.path == "/image").unwrap();
    assert_eq!(image.shape, vec![2, 2]);
    assert_eq!(image.values, vec![1., 2., 3., 4.]);
}

#[test]
fn explicit_hdf5_selection_is_owned_and_checks_shapes() {
    use hdf5_pure::FileBuilder;
    let mut b = FileBuilder::new();
    b.create_dataset("energy").with_f64_data(&[7100., 7101.]);
    b.create_dataset("mu").with_f64_data(&[1., 2.]);
    b.create_dataset("short").with_f64_data(&[3.]);
    let d = parse_measurement(&b.finish().unwrap()).unwrap();
    let s = d.dataset_scan(&["/energy".into(), "/mu".into()]).unwrap();
    assert_eq!(s.arrays(None).unwrap().1, vec![1., 2.]);
    for paths in [
        vec!["/energy"],
        vec!["/energy", "/energy"],
        vec!["/energy", "/missing"],
        vec!["/energy", "/short"],
    ] {
        assert!(d
            .dataset_scan(&paths.into_iter().map(String::from).collect::<Vec<_>>())
            .is_err());
    }
}

#[test]
fn mode_driven_9809_handles_fixed_width_and_opaque_dates() {
    let row = format!(
        "{:10.5}{:10.5}{:10.2}{:10}{:10}\n",
        17., 17.5, 1., 1234567890u64, 1111111111u64
    );
    let text=format!("9809 test\n sample_20.01.02.001 invalid-date\n Mono : D= 3.13551 A\n Mode 0 0 1 3\n Offset 0 0 42 43\n{row}");
    let d = parse_measurement(text.as_bytes()).unwrap();
    assert_eq!(d.scans[0].columns[3].values[0], 1234567890.);
    let (e, y) = d.scans[0].arrays(None).unwrap();
    assert!((e[0] - 12398.419843320026 / (2. * 3.13551 * 17.5f64.to_radians().sin())).abs() < 1e-9);
    assert!((y[0] - 1234567890f64.recip() * 1111111111.).abs() < 1e-15);
    assert!(d.scans[0].header.contains("sample_20.01.02.001"));
}

#[test]
fn gzip_and_damaged_recognized_containers_are_checked() {
    use std::io::Write;
    let mut gzip = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    gzip.write_all(b"energy,mu\n7100,1\n7101,2\n").unwrap();
    let bytes = gzip.finish().unwrap();
    let doc = parse_measurement(&bytes).unwrap();
    assert_eq!(doc.scans[0].arrays(None).unwrap().1, vec![1., 2.]);
    assert!(parse_measurement(&bytes[..bytes.len() - 4]).is_err());
    assert!(parse_measurement(b"\x89HDF\r\n\x1a\n").is_err());
}

#[test]
fn malformed_athena_optional_arrays_return_errors_without_panicking() {
    for json in [
        r#"{"_____order":["x"],"x":{"i0":[1,2]}}"#,
        r#"{"_____order":["x"],"x":{"x":[1,2],"y":[1],"args":{}}}"#,
    ] {
        assert!(parse_measurement(json.as_bytes()).is_err());
    }
}

#[test]
fn csv_axis_suffix_and_inline_comments_remain_interpretable() {
    let d = parse_measurement(b"energy_keV,mu\n7.1,1 # first\n7.2,2\n").unwrap();
    assert_eq!(d.scans[0].arrays(None).unwrap().0, vec![7100., 7200.]);
    assert_eq!(d.scans[0].metadata["row_comments"], r#"[" first",""]"#);
}

#[test]
fn relative_energy_conversion_is_explicit_and_validated() {
    let doc = parse_measurement(b"relative,mu\n-25,1\n0,2\n").unwrap();
    let s = &doc.scans[0];
    assert!(s.arrays(None).is_err());
    let mapping = direct(EnergyConversion::OffsetEv { offset_ev: 20000. });
    assert_eq!(s.arrays(Some(&mapping)).unwrap().0, vec![19975., 20000.]);
    assert_eq!(s.columns[0].values[0], -25.);
    for offset_ev in [0., f64::INFINITY, f64::NAN] {
        assert!(s
            .arrays(Some(&direct(EnergyConversion::OffsetEv { offset_ev })))
            .is_err());
    }
    assert!(parse_measurement(b"# FDMNES program\nEnergy <xanes>\n-25 1\n0 2\n").is_err());
}

#[test]
fn demeter_described_bm23_and_b18_layouts_preserve_every_row() {
    // Synthetic layouts: no original B18 or historical BM23 text file was
    // found in Demeter's filetype examples at the audited commit.
    let bm23=b"#F E.S.R.F. BM23\n#S 1 scan\n#L E_kev_  Seconds  I0  I1\n7.1 1 10 2\n7.2 1 20 4\n----------------\n";
    let doc = parse_measurement(bm23).unwrap();
    assert_eq!(doc.format, "bm23");
    assert_eq!(doc.scans[0].arrays(None).unwrap().0, vec![7100., 7200.]);
    let explicit = String::from_utf8_lossy(bm23).replace("E_kev_", "Energy (eV)");
    assert_eq!(
        parse_measurement(explicit.as_bytes()).unwrap().scans[0]
            .arrays(None)
            .unwrap()
            .0,
        vec![7.1, 7.2]
    );
    assert!(parse_measurement(&[bm23.as_slice(), b"7.3 invalid 10 2\n"].concat()).is_err());
    let labels = std::iter::once("energy".to_owned())
        .chain((1..43).map(|i| format!("detector_{i}")))
        .collect::<Vec<_>>()
        .join("  ");
    let rows = (0..5)
        .map(|i| {
            std::iter::once((7100 + i).to_string())
                .chain((1..43).map(|c| (i * 100 + c).to_string()))
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect::<Vec<_>>()
        .join("\n");
    let doc =
        parse_measurement(format!("# Diamond\n# B18-CORE XAS\n# {labels}\n{rows}\n").as_bytes())
            .unwrap();
    assert_eq!(doc.format, "b18");
    assert_eq!(doc.scans[0].columns.len(), 43);
    assert_eq!(doc.scans[0].columns[0].values.len(), 5);
    assert_eq!(doc.scans[0].columns[42].values[4], 442.);
    assert!(doc.scans[0].signals.is_empty());
}

#[test]
fn athena_nested_metadata_is_inert_and_missing_arrays_are_preserved() {
    let text="# Athena project file -- version 0.9.26\n$old_group = 'a';\n@args = ('is_xmu',1,'xdi',{'name' => {'value' => 'literal } text'}});\n@x = (7100,7101);\n@y = (1,2);\n@stddev = (undef);\n[record]\n@journal = system(\"never_execute_this\");\n1;\n";
    let doc = parse_measurement(text.as_bytes()).unwrap();
    assert_eq!(doc.scans[0].arrays(None).unwrap().1, vec![1., 2.]);
    assert_eq!(doc.scans[0].columns.len(), 2);
    assert!(doc.scans[0].metadata["athena.extra"].contains("(undef)"));
    assert!(doc.scans[0].metadata["athena.project_extra"].contains("never_execute_this"));
    assert!(doc.scans[0].header.contains("literal } text"));
    assert!(
        parse_measurement(text.replace("@x = (7100,7101)", "@x = (undef)").as_bytes()).is_err()
    );
    let nested = format!("{}1{}", "[".repeat(65), "]".repeat(65));
    assert!(parse_measurement(
        text.replace("'is_xmu',1", &format!("'nested',{nested}"))
            .as_bytes()
    )
    .is_err());
}

#[test]
fn cls_event_header_width_cannot_silently_change_channel_roles() {
    let bytes=b"# CLS Data Acquisition\n#(1) \"Event-ID\" \"MONO16061I1001:Energy:sp\"\n1,7100,10\n1,7101,20\n";
    assert!(parse_measurement(bytes)
        .unwrap_err()
        .to_string()
        .contains("headings"));
}
