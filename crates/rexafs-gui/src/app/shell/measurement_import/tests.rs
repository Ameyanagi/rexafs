use super::*;

#[test]
fn project_import_includes_every_selected_scan_with_original_arrays_and_identity() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../rexafs/tests/fixtures");
    for (file, count) in [
        (
            "xas/samples/unspecified/unspecified/xraylarch/json_unzipped.prj",
            4,
        ),
        (
            "xas/samples/unspecified/athena-legacy/xraylarch/ESRF_Athena0920.prj",
            2,
        ),
        ("sessions/larix/fixtures/valid/two-analyzed.larix", 2),
        ("sessions/xtunes/two-analyzed.xtsp", 2),
    ] {
        let path = root.join(file);
        let (document, bytes) = read_source(&path).unwrap();
        let original = base64::engine::general_purpose::STANDARD.encode(&bytes);
        let mut source = MeasurementImport::new(path, document, bytes);
        assert_eq!(source.document.scans.len(), count, "{file}");
        assert!(source.import_ready(), "{file}");
        assert_eq!(source.import_count(), count, "{file}");
        let groups = source.materialize_import(&source.config()).unwrap();
        assert_eq!(groups.len(), count, "{file}");
        let ids: std::collections::BTreeSet<_> = groups.iter().map(|g| &g.group_id).collect();
        assert_eq!(ids.len(), count);
        for (scan, group) in source.document.scans.iter().zip(&groups) {
            let (energy, mu) = scan.arrays(None).unwrap();
            assert_eq!(group.energy, energy);
            assert_eq!(group.mu, mu);
            assert!(group.label.ends_with(&scan.label));
            let evidence = &group.operation.as_ref().unwrap().parameters;
            assert_eq!(evidence["record_id"], scan.id);
            assert_eq!(evidence["original_bytes_base64"], original);
        }
        // Previewing another scan must not change which groups are imported.
        source.select_scan(count - 1);
        source.selected_scans[0] = false;
        let subset = source.materialize_import(&source.config()).unwrap();
        assert_eq!(subset.len(), count - 1);
        assert_eq!(subset[0].mu, groups[1].mu);
        println!("{file}: {count} scans imported and source arrays verified");
    }
}

#[test]
fn scan_selection_keeps_independent_detector_choices_and_edits() {
    let bytes = b"#F two.spec\n#S 1 first\n#N 5\n#L energy  I0  It  Ir  IFF\n7100 100 50 25 3\n7200 200 50 10 8\n#S 2 second\n#N 5\n#L energy  I0  It  Ir  IFF\n7100 100 20 10 4\n7200 200 40 10 6\n";
    let doc = rexafs::io::parse_measurement(bytes).unwrap();
    let mut source = MeasurementImport::new("two.spec".into(), doc, bytes.to_vec());
    assert_eq!(source.import_count(), 6);
    let original = source.materialize_import(&source.config()).unwrap();
    assert_eq!(original.len(), 6);
    source.included = vec![false, true, false];
    source.select_scan(1);
    source.included = vec![true, false, true];
    source.signal = Some(2);
    let mut config = source.config();
    config.it_col = Some(1);
    source.remember_config(&config);
    source.select_scan(0);
    assert_eq!(source.included, [false, true, false]);
    assert_eq!(source.import_count(), 3);
    let groups = source.materialize_import(&source.config()).unwrap();
    assert_eq!(groups[0].mu, original[1].mu);
    assert_eq!(groups[1].mu, original[3].mu);
    for (actual, expected) in groups[2].mu.iter().zip([10_f64.ln(), 20_f64.ln()]) {
        assert!((actual - expected).abs() < 1e-13);
    }
    source.select_scan(1);
    assert_eq!(source.signal, Some(2));
    assert_eq!(source.config(), config);
}

#[test]
fn empty_session_requires_data_and_rejects_import_without_panicking() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../rexafs/tests/fixtures/sessions/larix/fixtures/valid/empty.larix");
    let (document, bytes) = read_source(&path).unwrap();
    let mut source = MeasurementImport::new(path, document, bytes);
    assert!(source.document.scans.is_empty());
    source.select_scan(0);
    let config = source.config();
    source.remember_config(&config);
    assert_eq!(source.import_count(), 0);
    assert!(!source.import_ready());
    assert!(source.materialize_import(&config).is_err());
    assert!(source.preview(&config).is_err());
}

#[test]
fn per_scan_custom_units_and_invalid_mapping_survive_preview_switches() {
    let bytes = b"#F two.spec\n#S 1 first\n#N 2\n#L axis  stored\n7.1 2\n7.2 3\n#S 2 second\n#N 2\n#L axis  stored\n7100 4\n7200 5\n";
    let doc = rexafs::io::parse_measurement(bytes).unwrap();
    let mut source = MeasurementImport::new("two.spec".into(), doc, bytes.to_vec());
    assert_eq!(source.document.scans.len(), 2);
    assert!(!source.import_ready());
    let first = ImportConfig {
        axis: AxisConversion::EnergyKev,
        mode: DetectionMode::MuColumn,
        energy_col: Some(0),
        mu_col: Some(1),
        ..Default::default()
    };
    source.confirmed = true;
    source.remember_config(&first);
    source.select_scan(1);
    let second = ImportConfig {
        axis: AxisConversion::EnergyEv,
        ..first.clone()
    };
    source.confirmed = true;
    source.remember_config(&second);
    assert!(source.import_ready());
    source.select_scan(0);
    assert_eq!(source.config(), first);
    let groups = source.materialize_import(&source.config()).unwrap();
    assert_eq!(groups[0].energy, [7100., 7200.]);
    assert_eq!(groups[1].energy, [7100., 7200.]);
    assert_eq!(groups[0].mu, [2., 3.]);
    assert_eq!(groups[1].mu, [4., 5.]);
    source.select_scan(1);
    let bad = ImportConfig {
        mu_col: Some(99),
        ..second.clone()
    };
    source.remember_config(&bad);
    source.select_scan(0);
    assert!(!source.import_ready());
    assert!(source.materialize_import(&source.config()).is_err());
    // Exclusion is explicit, and an excluded invalid preview cannot block others.
    source.selected_scans[1] = false;
    source.select_scan(1);
    assert_eq!(source.config(), bad);
    assert!(!source.preview_included());
    assert_eq!(
        source.materialize_import(&source.config()).unwrap().len(),
        1
    );
    source.selected_scans.fill(false);
    assert!(!source.import_ready());
    assert!(source.materialize_import(&source.config()).is_err());
}

#[test]
fn nexus_import_keeps_explicit_record_selection_and_individual_mappings() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../rexafs/tests/fixtures/xas/samples/aps/13-bm-d/pynxxas/Fe_XDIFiles/converted.nxs");
    let (document, bytes) = read_source(&path).unwrap();
    let mut source = MeasurementImport::new(path, document, bytes);
    assert_eq!(source.document.scans.len(), 6);
    assert!(!source.import_ready());
    source.selected_scans.fill(false);
    // This container also exposes instrument-only tables. Explicitly select
    // only its three stored energy/intensity records; do not invent an axis for
    // the independent i0/itrans tables or treat them as additional spectra.
    for index in 0..6 {
        let names: Vec<_> = source.document.scans[index]
            .columns
            .iter()
            .map(|c| c.name.as_str())
            .collect();
        if names != ["energy", "intensity"] {
            continue;
        }
        source.select_scan(index);
        source.selected_scans[index] = true;
        source.confirmed = true;
        source.remember_config(&ImportConfig {
            mode: DetectionMode::MuColumn,
            axis: AxisConversion::EnergyEv,
            energy_col: Some(0),
            mu_col: Some(1),
            ..Default::default()
        });
    }
    assert!(source.import_ready());
    let groups = source.materialize_import(&source.config()).unwrap();
    assert_eq!(groups.len(), 3);
    assert_eq!(
        groups.iter().map(|g| g.energy.len()).collect::<Vec<_>>(),
        [348, 348, 412]
    );
    for group in &groups {
        let record = &group.operation.as_ref().unwrap().parameters["source_record"];
        let x: Vec<f64> = serde_json::from_value(record["columns"][0]["values"].clone()).unwrap();
        let y: Vec<f64> = serde_json::from_value(record["columns"][1]["values"].clone()).unwrap();
        assert_eq!(group.energy, x);
        assert_eq!(group.mu, y);
    }
}

#[test]
fn qas_imports_checked_outputs_with_independent_mappings_and_provenance() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(
        "../rexafs/tests/fixtures/xas/samples/nsls-ii/7-bm-qas/xasref/Mo foil 0001-r0003.dat",
    );
    let (document, bytes) = read_source(&path).unwrap();
    let mut source = MeasurementImport::new(path, document, bytes);
    assert_eq!(source.included, [true, true, true]);
    let groups = source.materialize_selected(&source.config()).unwrap();
    assert_eq!(groups.len(), 3);
    let scan = &source.document.scans[0];
    for (group, name) in groups
        .iter()
        .zip(["transmission", "fluorescence", "reference"])
    {
        assert!(group.label.ends_with(name));
        assert_eq!(group.energy.len(), 651);
        assert_eq!(
            group.operation.as_ref().unwrap().parameters["signal_name"],
            name
        );
    }
    let row = 100;
    let i0 = scan.columns[1].values[row];
    let it = scan.columns[2].values[row];
    let ir = scan.columns[3].values[row];
    let iff = scan.columns[4].values[row];
    assert!((groups[0].mu[row] - (i0 / it).ln()).abs() < 1e-13);
    assert!((groups[1].mu[row] - iff / i0).abs() < 1e-13);
    assert!((groups[2].mu[row] - (it / ir).ln()).abs() < 1e-13);
    assert_ne!(groups[0].group_id, groups[1].group_id);
    source.included[1] = false;
    source.signal = Some(2);
    assert_eq!(source.config().mode, DetectionMode::Reference);
    let original = source.original_config();
    let mut edited = original.clone();
    edited.it_col = Some(1);
    source.remember_config(&edited);
    source.signal = Some(0);
    assert_eq!(
        source.materialize_selected(&source.config()).unwrap().len(),
        2
    );
    source.signal = Some(2);
    assert_eq!(source.config().it_col, Some(1));
    assert_eq!(source.original_config().it_col, Some(2));
    source.remember_config(&original);
    assert_eq!(
        source.materialize_selected(&source.config()).unwrap()[1].mu,
        groups[2].mu
    );
    source.included.fill(false);
    assert!(!source.included_valid());
    assert!(source.materialize_selected(&source.config()).is_err());
}

#[test]
fn invalid_signals_can_be_excluded_without_blocking_valid_outputs() {
    let bytes = b"energy,I0,It,Ir,IFF\n7100,100,0,5,3\n7101,200,40,10,8\n";
    let document = rexafs::io::parse_measurement(bytes).unwrap();
    let mut source = MeasurementImport::new("invalid.dat".into(), document, bytes.to_vec());
    assert_eq!(source.included, [false, true, false]);
    assert_eq!(source.signal, Some(1));
    source.signal = Some(0); // Previewing an unchecked invalid signal is allowed.
    assert!(!source.preview_included());
    let groups = source.materialize_selected(&source.config()).unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].mu, [0.03, 0.04]);
    source.included[0] = true;
    assert!(source.materialize_selected(&source.config()).is_err());
}

#[test]
fn every_beamline_signal_has_the_same_preview_and_import_conversion() {
    let root =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../rexafs/tests/fixtures/xas");
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("manifest.json")).unwrap()).unwrap();
    let mut sources = 0;
    let mut conversions = 0;
    let mut paths = manifest["samples"]
        .as_array()
        .unwrap()
        .iter()
        .map(|sample| sample["path"].as_str().unwrap().to_owned())
        .collect::<std::collections::BTreeSet<_>>();
    let mapping: std::collections::BTreeMap<String, String> = serde_json::from_slice(
        &std::fs::read(root.join("collections/rexafs-corpus/paths.json")).unwrap(),
    )
    .unwrap();
    let coverage: serde_json::Value = serde_json::from_slice(
        &std::fs::read(root.join("../../measurement_fixtures/format_corpus_coverage.json"))
            .unwrap(),
    )
    .unwrap();
    paths.extend(
        coverage["records"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|record| record["status"] != "rejected")
            .map(|record| mapping[record["path"].as_str().unwrap()].clone()),
    );
    for path in paths {
        let path = root.join(path);
        let (document, bytes) = read_source(&path).unwrap();
        let mut source = MeasurementImport::new(path.clone(), document, bytes);
        sources += 1;
        for scan in 0..source.document.scans.len() {
            source.select_scan(scan);
            assert_eq!(
                source.confirmed,
                !source.document.scans[scan].signals.is_empty()
            );
            for choice in 0..source.document.scans[scan].signals.len() {
                source.signal = Some(choice);
                source.confirmed = true;
                let candidate = &source.document.scans[scan].signals[choice].mapping;
                let expected = source.document.scans[scan].to_spectrum(Some(candidate));
                let config = source.config();
                let preview = source.preview(&config).unwrap();
                let draft = crate::import_mapping::MappingDraft::new(&preview.table, &config);
                assert_eq!(
                    &source.mapping(draft.config()).unwrap(),
                    candidate,
                    "{}",
                    path.display()
                );
                match expected {
                    Ok(expected) => {
                        let raw = preview
                            .raw
                            .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
                        assert_eq!(raw.energy, expected.energy.unwrap().as_slice());
                        assert_eq!(raw.mu, expected.mu.unwrap().as_slice());
                        conversions += 1;
                    }
                    Err(_) => assert!(preview.raw.is_err(), "{}", path.display()),
                }
            }
        }
    }
    assert!(sources >= 200 && conversions >= 298);
    println!("{sources} source files; {conversions} valid GUI signal previews matched the core.");
}

#[test]
fn manual_degree_and_radian_axes_match_and_require_valid_crystal_spacing() {
    for radians in [false, true] {
        let angle = if radians {
            std::f64::consts::PI / 6.
        } else {
            30.
        };
        let bytes = format!("axis signal\n{angle} 2\n{angle} 3\n").into_bytes();
        let doc = rexafs::io::parse_measurement(&bytes).unwrap();
        let mut source = MeasurementImport::new("angles.dat".into(), doc, bytes);
        source.confirmed = true;
        let mut config = source.config();
        config.axis = if radians {
            AxisConversion::AngleRadians { d_spacing: 3. }
        } else {
            AxisConversion::AngleDegrees { d_spacing: 3. }
        };
        let result = source.preview(&config).unwrap().raw.unwrap();
        assert!((result.energy[0] - 12398.419843320026 / 3.).abs() < 1e-9);
        assert_eq!(result.mu, vec![2., 3.]);
        assert_eq!(source.materialize(&config).unwrap().energy, result.energy);
        config.axis = AxisConversion::AngleDegrees { d_spacing: 0. };
        assert!(source.preview(&config).unwrap().raw.is_err());
        assert!(source.materialize(&config).is_err());
    }
}

#[test]
fn larix_import_preserves_complex_results_and_session_provenance_in_projects() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../rexafs/tests/fixtures/sessions/larix/fixtures/valid/two-analyzed.larix");
    let (document, bytes) = read_source(&path).unwrap();
    let mut groups = Vec::new();
    for i in 0..2 {
        let mut group =
            materialize(&path, &bytes, &document, i, &initial_mapping(&document, i)).unwrap();
        group.id = (i + 1) as u64;
        assert!(group.params.is_none() && group.source.is_none());
        let provenance = &group.operation.as_ref().unwrap().parameters;
        assert!(
            provenance["container_metadata"]["larix.session_text"]
                .as_str()
                .unwrap()
                .starts_with("##LARIX:")
        );
        let chir = provenance["archived_tables"]
            .as_array()
            .unwrap()
            .iter()
            .find(|d| d["path"].as_str().unwrap().ends_with("/chir"))
            .unwrap();
        assert_eq!(chir["shape"], serde_json::json!([326]));
        assert_eq!(chir["imaginary"].as_array().unwrap().len(), 326);
        groups.push(group);
    }
    let project = crate::project::ProjectFile {
        version: 1,
        derived: groups,
        ..Default::default()
    };
    let directory = std::env::temp_dir().join(format!("rexafs-larix-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let saved = directory.join("larix.rxs");
    crate::project::save(&saved, &project).unwrap();
    let reopened =
        crate::project::load_with_cache_root(&saved, || Ok(directory.join("cache"))).unwrap();
    assert_eq!(reopened.derived[0].energy.len(), 818);
    assert_eq!(reopened.derived[1].energy.len(), 1420);
    assert_eq!(reopened.derived[1].operation, project.derived[1].operation);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn relative_reference_energy_materializes_without_changing_source_columns() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../rexafs/tests/fixtures/xas/samples/unspecified/fdmnes/xraylarch/FDMNES_2022_Mo2C_out.dat");
    let (document, bytes) = read_source(&path).unwrap();
    let group = materialize(&path, &bytes, &document, 0, &initial_mapping(&document, 0)).unwrap();
    assert_eq!(group.energy.len(), 559);
    assert_eq!(group.energy[0], 19975.);
    assert_eq!(group.mu[0], 0.00011943264);
    let provenance = &group.operation.unwrap().parameters;
    assert_eq!(provenance["mapping"]["energy"]["offset_ev"], 20000.);
    assert_eq!(provenance["source_record"]["columns"][0]["values"][0], -25.);
}

#[test]
fn independent_sample_choices_materialize_distinct_signals() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../rexafs/tests/fixtures/xas/samples/nsls/x23a2/demeter/re4chan.000");
    let (document, bytes) = read_source(&path).unwrap();
    let signals = &document.scans[0].signals;
    assert_eq!(signals.len(), 4);
    let first = materialize(&path, &bytes, &document, 0, &signals[0].mapping).unwrap();
    let last = materialize(&path, &bytes, &document, 0, &signals[3].mapping).unwrap();
    assert_eq!(first.energy.len(), 387);
    assert_eq!(first.energy, last.energy);
    assert!((first.mu[0] - (13720.5_f64 / 2864.).ln()).abs() < 1e-12);
    assert!((last.mu[0] - (13071_f64 / 3671.75).ln()).abs() < 1e-12);
    assert_ne!(first.group_id, last.group_id);
}

#[test]
fn project_records_keep_portable_originals_and_independent_identities() {
    let bytes=b"#XTSP FilePath=missing\\same.xtsd\nDataFileName=missing\\raw.001\n#QD Plot\t2\nE Mu\n7101 2\n7100 1\n#XTSP FilePath=missing\\same.xtsd\nDataFileName=missing\\raw.001\n#QD Plot\t2\nE Mu\n7101 4\n7100 3\n";
    let document = rexafs::io::parse_measurement(bytes).unwrap();
    let first = materialize(
        std::path::Path::new("missing.xtsp"),
        bytes,
        &document,
        0,
        &initial_mapping(&document, 0),
    )
    .unwrap();
    let second = materialize(
        std::path::Path::new("missing.xtsp"),
        bytes,
        &document,
        1,
        &initial_mapping(&document, 1),
    )
    .unwrap();
    assert_ne!(first.group_id, second.group_id);
    assert_eq!(first.energy, vec![7100., 7101.]);
    assert_eq!(second.mu, vec![3., 4.]);
    let restored: DerivedSpectrum =
        serde_json::from_slice(&serde_json::to_vec(&first).unwrap()).unwrap();
    assert_eq!(restored.energy, first.energy);
    assert!(restored.source.is_none());
    assert!(restored.params.is_none());
    assert_eq!(restored.quantity, crate::params::Quantity::RawMu);
    let p = &restored.operation.unwrap().parameters;
    assert_eq!(
        base64::engine::general_purpose::STANDARD
            .decode(p["original_bytes_base64"].as_str().unwrap())
            .unwrap(),
        bytes
    );
    assert_eq!(p["source_record"]["columns"][0]["values"][0], 7101.);
    assert_eq!(p["archived_tables"][0]["shape"], serde_json::json!([2, 2]));
    let directory = std::env::temp_dir().join(format!(
        "rexafs-measurement-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&directory).unwrap();
    let path = directory.join("portable.rxs");
    let mut first = first;
    first.id = 1;
    let mut second = second;
    second.id = 2;
    let project = crate::project::ProjectFile {
        version: 1,
        derived: vec![first, second],
        ..Default::default()
    };
    crate::project::save(&path, &project).unwrap();
    let reopened =
        crate::project::load_with_cache_root(&path, || Ok(directory.join("cache"))).unwrap();
    assert_eq!(reopened.derived.len(), 2);
    assert_eq!(reopened.derived[1].mu, vec![3., 4.]);
    assert_eq!(reopened.derived[0].operation, project.derived[0].operation);
    std::fs::remove_dir_all(directory).unwrap();
}
#[test]
fn invalid_detector_roles_never_materialize_a_group() {
    let bytes = b"energy,i0,it\n7100,10,0\n7101,10,1\n";
    let document = rexafs::io::parse_measurement(bytes).unwrap();
    assert!(
        materialize(
            std::path::Path::new("input"),
            bytes,
            &document,
            0,
            &initial_mapping(&document, 0)
        )
        .is_err()
    );
}

#[test]
fn all_valid_saved_sessions_use_the_same_preview_mapping() {
    let root =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../rexafs/tests/fixtures/sessions");
    let mut sources = 0;
    for bundle in ["larix", "xtunes"] {
        let folder = root.join(bundle);
        let manifest: serde_json::Value =
            serde_json::from_slice(&std::fs::read(folder.join("manifest.json")).unwrap()).unwrap();
        for file in manifest["files"].as_array().unwrap() {
            let name = file["path"].as_str().unwrap();
            if bundle == "larix" && file["category"] != "valid" {
                continue;
            }
            if bundle == "xtunes"
                && !name.ends_with(".xts")
                && !name.ends_with(".xtsd")
                && !name.ends_with(".xtsp")
            {
                continue;
            }
            let path = folder.join(name);
            let (document, bytes) = read_source(&path).unwrap();
            let mut source = MeasurementImport::new(path, document, bytes);
            for i in 0..source.document.scans.len() {
                source.select_scan(i);
                for choice in 0..source.document.scans[i].signals.len() {
                    source.signal = Some(choice);
                    source.confirmed = true;
                    let config = source.config();
                    let preview = source.preview(&config).unwrap();
                    let draft = crate::import_mapping::MappingDraft::new(&preview.table, &config);
                    assert_eq!(
                        source.mapping(draft.config()).unwrap(),
                        source.document.scans[i].signals[choice].mapping
                    );
                    let raw = preview.raw.unwrap();
                    let group = source.materialize(draft.config()).unwrap();
                    assert_eq!(group.energy, raw.energy);
                    assert_eq!(group.mu, raw.mu);
                }
            }
            sources += 1;
        }
    }
    assert_eq!(sources, 21);
}
