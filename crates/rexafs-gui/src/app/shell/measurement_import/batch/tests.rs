use super::*;
use crate::import_mapping::AxisConversion;
use crate::params::DetectionMode;

// Fabricated acquisition rows; no private experimental files are distributed.
fn qd(spacing: f64, rows: &str) -> Vec<u8> {
    format!("9809 TEST\nMono : Si(111) D= {spacing} A\nTransmission Repetition= 0 Points= 2\nAngle(c) Angle(o) time I0 It\nMode 0 0 1 2\nOffset 0 0 0 0\n{rows}").into_bytes()
}

fn source(path: &std::path::Path) -> MeasurementImport {
    let (document, bytes) = read_source(path).unwrap();
    MeasurementImport::new(path.into(), document, bytes)
}

#[test]
fn one_qd_confirmation_imports_matching_files_with_independent_data_and_evidence() {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("first.qd");
    let b = dir.path().join("second.QD");
    let first = qd(3.13551, "12 11 1 100 50\n11 10 1 200 50\n");
    let mut second = qd(
        3.13551,
        "11 10 1 300 100\n12 11 1 400 100\n13 12 1 500 100\n",
    );
    second = String::from_utf8(second)
        .unwrap()
        .replace("9809 TEST", "9809 TEST\nAcquired: later scan")
        .replace(" A\n", " A Initial angle= 12 deg\n")
        .into_bytes();
    std::fs::write(&a, &first).unwrap();
    std::fs::write(&b, &second).unwrap();
    let mut reviewed = source(&a);
    reviewed.review_batch(7, vec![a.clone(), b.clone()]);
    assert_eq!(reviewed.batch.as_ref().unwrap().id, 7);
    assert!(reviewed.apply_to_batch);
    assert_eq!(reviewed.total_import_count(), 2);
    let groups = reviewed.materialize_batch(&reviewed.config()).unwrap();
    assert_eq!(groups.len(), 2);
    assert_eq!(groups[0].energy.len(), 2);
    assert_eq!(groups[1].energy.len(), 3);
    assert!(
        (groups[0].energy[0] - 12398.419843320026 / (2. * 3.13551 * 11_f64.to_radians().sin()))
            .abs()
            < 1e-9
    );
    assert!((groups[0].mu[0] - 2_f64.ln()).abs() < 1e-12);
    assert!((groups[1].mu[0] - 5_f64.ln()).abs() < 1e-12);
    for (group, path, bytes) in [(&groups[0], a, first), (&groups[1], b, second)] {
        let evidence = &group.operation.as_ref().unwrap().parameters;
        assert_eq!(evidence["source_path"], path.to_str().unwrap());
        use base64::Engine;
        assert_eq!(
            base64::engine::general_purpose::STANDARD
                .decode(evidence["original_bytes_base64"].as_str().unwrap())
                .unwrap(),
            bytes
        );
    }
    reviewed.apply_to_batch = false;
    assert_eq!(reviewed.total_import_count(), 1);
    assert_eq!(
        reviewed
            .materialize_batch(&reviewed.config())
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn explicit_detector_and_angle_choices_are_applied_once_to_the_drop() {
    let dir = tempfile::tempdir().unwrap();
    let paths: Vec<_> = (0..3)
        .map(|i| {
            let p = dir.path().join(format!("{i}.qd"));
            std::fs::write(
                &p,
                qd(
                    3.13551,
                    &format!("12 11 1 100 {}\n11 10 1 200 40\n", 20 + i),
                ),
            )
            .unwrap();
            p
        })
        .collect();
    let mut reviewed = source(&paths[0]);
    reviewed.review_batch(0, paths);
    reviewed.signal = None;
    reviewed.confirmed = true;
    let config = ImportConfig {
        energy_col: Some(0),
        axis: AxisConversion::AngleDegrees { d_spacing: 3.13551 },
        mode: DetectionMode::Fluorescence,
        i0_col: Some(3),
        fluor_cols: Some(vec![4]),
        ..Default::default()
    };
    let groups = reviewed.materialize_batch(&config).unwrap();
    assert_eq!(groups.len(), 3);
    for (i, group) in groups.iter().enumerate() {
        assert!((group.mu[0] - (20 + i) as f64 / 100.).abs() < 1e-12);
        assert!(
            (group.energy[0] - 12398.419843320026 / (2. * 3.13551 * 12_f64.to_radians().sin()))
                .abs()
                < 1e-9
        );
    }
}

#[test]
fn other_layouts_crystals_containers_and_bad_rows_remain_for_separate_review() {
    let dir = tempfile::tempdir().unwrap();
    let original = qd(3.13551, "12 11 1 100 50\n11 10 1 200 50\n");
    let variants = [
        original.clone(),
        qd(1.637, "12 11 1 100 50\n11 10 1 200 50\n"),
        String::from_utf8(original.clone())
            .unwrap()
            .replace("Mode 0 0 1 2", "Mode 0 0 1 3")
            .into_bytes(),
        b"# energy mu\n7000 1\n7100 2\n".to_vec(),
        qd(3.13551, "12 11 1 100 bad\n11 10 1 200 50\n"),
    ];
    let paths: Vec<_> = variants
        .iter()
        .enumerate()
        .map(|(i, data)| {
            let path = dir.path().join(format!("{i}.qd"));
            std::fs::write(&path, data).unwrap();
            path
        })
        .collect();
    let mut reviewed = source(&paths[0]);
    reviewed.review_batch(0, paths);
    assert_eq!(reviewed.batch_file_count(), 1);
    assert_eq!(reviewed.batch.as_ref().unwrap().separate.len(), 4);
    let mut two = (*reviewed.document).clone();
    two.scans.push(two.scans[0].clone());
    assert!(!compatible(&reviewed.document, &two));
    // An unknown axis without signals must still distinguish crystal metadata.
    let mut a = (*reviewed.document).clone();
    a.scans[0].signals.clear();
    let mut b = a.clone();
    b.scans[0].header = b.scans[0].header.replace("3.13551", "1.637");
    assert!(!compatible(&a, &b));
}

#[test]
fn invalid_sibling_arithmetic_and_changed_sources_abort_without_partial_groups() {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("a.qd");
    let b = dir.path().join("b.qd");
    std::fs::write(&a, qd(3.13551, "12 11 1 100 50\n11 10 1 200 50\n")).unwrap();
    std::fs::write(&b, qd(3.13551, "12 11 1 100 0\n11 10 1 200 50\n")).unwrap();
    let mut reviewed = source(&a);
    reviewed.review_batch(0, vec![a, b.clone()]);
    assert_eq!(reviewed.batch_file_count(), 2);
    assert!(
        reviewed
            .materialize_batch(&reviewed.config())
            .err()
            .unwrap()
            .contains("b.qd")
    );
    std::fs::write(&b, qd(3.13551, "12 11 1 100 5\n11 10 1 200 50\n")).unwrap();
    assert!(
        reviewed
            .materialize_batch(&reviewed.config())
            .err()
            .unwrap()
            .contains("Source changed")
    );
}

#[test]
fn batch_reuses_checked_outputs_without_approving_another_drop() {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("first.spec");
    let b = dir.path().join("second.spec");
    let later = dir.path().join("later.spec");
    let text =
        "#F example\n#S 1 scan\n#N 4\n#L energy  I0  It  IFF\n7000 100 50 5\n7100 200 50 8\n";
    for path in [&a, &later] {
        std::fs::write(path, text).unwrap();
    }
    // Bad transmission must not block a deliberately selected fluorescence output.
    std::fs::write(&b, text.replace("100 50 5", "100 0 7")).unwrap();
    let mut reviewed = source(&a);
    reviewed.review_batch(0, vec![a, b]);
    let fluorescence = reviewed.document.scans[0]
        .signals
        .iter()
        .position(|s| matches!(s.mapping.signal, rexafs::io::SignalConversion::Ratio { .. }))
        .unwrap();
    reviewed.signal = Some(fluorescence);
    reviewed.included.fill(false);
    reviewed.included[fluorescence] = true;
    let groups = reviewed.materialize_batch(&reviewed.config()).unwrap();
    assert_eq!(groups.len(), 2);
    assert_eq!(groups[0].mu, [0.05, 0.04]);
    assert_eq!(groups[1].mu, [0.07, 0.04]);
    assert!(groups.iter().all(|g| !g.label.contains("later")));
    let fresh = source(&later);
    assert!(fresh.batch.is_none());
    assert!(!fresh.apply_to_batch);
}
