use rexafs::io::*;

#[test]
fn duplicate_detector_roles_require_explicit_columns() {
    for (header, incident, transmitted) in [
        ("energy,I0,I0,It", 1usize, 3usize),
        ("energy,I0,monitor,It", 2, 3),
        ("energy,I0,It,I1", 1, 2),
    ] {
        let text = format!("{header}\n7100,100,20,5\n7101,200,40,10\n");
        let scan = parse_measurement(text.as_bytes()).unwrap().scans.remove(0);
        assert!(scan.signals.is_empty(), "{header}: {:?}", scan.signals);
        assert!(scan.arrays(None).is_err(), "{header}");
        let (_, mu) = scan
            .arrays_with(
                &SpectrumSelection::transmission(0usize, incident, transmitted)
                    .with_energy(EnergyConversion::Ev),
            )
            .unwrap();
        let expected =
            (scan.columns[incident].values[0] / scan.columns[transmitted].values[0]).ln();
        assert!(mu.iter().all(|value| (value - expected).abs() < 1e-14));
        assert_eq!(scan.columns.len(), 4);
        assert_eq!(scan.columns[2].values, [20., 40.]);
    }
}

#[test]
fn every_stored_signal_remains_an_explicit_choice() {
    for header in ["energy,mutrans,mufluor", "energy,mu,mu"] {
        let text = format!("{header}\n7100,0.2,0.3\n7101,0.4,0.5\n");
        let scan = parse_measurement(text.as_bytes()).unwrap().scans.remove(0);
        assert_eq!(scan.signals.len(), 2, "{header}");
        assert_ne!(scan.signals[0].name, scan.signals[1].name);
        assert!(scan.arrays(None).is_err(), "{header}");
        for (candidate, expected) in scan.signals.iter().zip([[0.2, 0.4], [0.3, 0.5]]) {
            assert_eq!(scan.arrays(Some(&candidate.mapping)).unwrap().1, expected);
        }
    }
    let scan = parse_measurement(
        b"energy,mufluor,mutrans,I0,It\n7100,0.3,0.2,100,20\n7101,0.5,0.4,200,40\n",
    )
    .unwrap()
    .scans
    .remove(0);
    assert_eq!(
        scan.signals
            .iter()
            .map(|signal| signal.name.as_str())
            .collect::<Vec<_>>(),
        ["mufluor", "mutrans"]
    );
    assert_eq!(
        scan.arrays_with(&SpectrumSelection::direct("energy", "mutrans"))
            .unwrap()
            .1,
        [0.2, 0.4]
    );
}

#[test]
fn hdf5_stored_signals_use_the_same_explicit_selection_contract() {
    let mut file = hdf5_pure::FileBuilder::new();
    file.create_dataset("energy").with_f64_data(&[7100., 7101.]);
    file.create_dataset("mutrans").with_f64_data(&[0.2, 0.4]);
    file.create_dataset("mufluor").with_f64_data(&[0.3, 0.5]);
    let scan = parse_measurement(&file.finish().unwrap())
        .unwrap()
        .scans
        .remove(0);
    assert_eq!(scan.signals.len(), 2);
    assert!(scan.arrays(None).is_err());
    assert_eq!(
        scan.arrays_with(&SpectrumSelection::direct("energy", "mutrans"))
            .unwrap()
            .1,
        [0.2, 0.4]
    );
    assert_eq!(
        scan.arrays_with(&SpectrumSelection::direct("energy", "mufluor"))
            .unwrap()
            .1,
        [0.3, 0.5]
    );
}

#[test]
fn names_indices_and_reordered_columns_select_the_same_signal() {
    for text in [
        "energy (keV),I0,It,IFF1,IFF2,mu\n7.1,10,2,3,4,0.5\n7.2,20,4,8,10,0.6\n",
        "mu,IFF2,It,energy (keV),IFF1,I0\n0.5,4,2,7.1,3,10\n0.6,10,4,7.2,8,20\n",
    ] {
        let doc = parse_measurement(text.as_bytes()).unwrap();
        let scan = &doc.scans[0];
        let select = SpectrumSelection::transmission("energy", "I0", "It");
        let resolved = select.resolve(scan).unwrap();
        assert_eq!(resolved.energy, EnergyConversion::Kev);
        let (energy, mu) = scan.arrays_with(&select).unwrap();
        assert_eq!(energy, [7100., 7200.]);
        assert!(mu.iter().all(|v| (v - 5f64.ln()).abs() < 1e-14));
        assert_eq!(scan.arrays(Some(&resolved)).unwrap(), (energy, mu));
        let fluorescence = SpectrumSelection::fluorescence("energy", "I0", ["IFF1", "IFF2"]);
        assert_eq!(scan.arrays_with(&fluorescence).unwrap().1, [0.7, 0.9]);
        let direct = SpectrumSelection::direct(resolved.energy_column, "mu");
        assert_eq!(scan.arrays_with(&direct).unwrap().1, [0.5, 0.6]);
        assert!(scan
            .to_spectrum_with(&direct)
            .unwrap()
            .normalization
            .is_none());
    }
}

#[test]
fn name_resolution_is_exact_and_duplicate_names_require_indices() {
    let mut scan = parse_measurement(b"energy,mu\n7100,1\n7101,2\n")
        .unwrap()
        .scans
        .remove(0);
    for name in ["MU", " mu", "1", "absent"] {
        assert!(scan
            .arrays_with(&SpectrumSelection::direct("energy", name))
            .unwrap_err()
            .to_string()
            .contains("no column named"));
    }
    scan.columns.push(scan.columns[1].clone());
    assert!(scan
        .arrays_with(&SpectrumSelection::direct("energy", "mu"))
        .unwrap_err()
        .to_string()
        .contains("ambiguous"));
    assert_eq!(
        scan.arrays_with(&SpectrumSelection::direct("energy", 2usize))
            .unwrap()
            .1,
        [1., 2.]
    );
    assert!(scan
        .arrays_with(&SpectrumSelection::direct("energy", 99usize))
        .is_err());
    assert!(scan
        .arrays_with(&SpectrumSelection::direct("energy", "energy"))
        .is_err());
}

#[test]
fn named_mapping_json_preserves_numeric_compatibility_and_checks_units() {
    let scan = parse_measurement(b"1 2\n3 4\n").unwrap().scans.remove(0);
    let selection = SpectrumSelection::direct(0usize, 1usize);
    assert!(selection
        .resolve(&scan)
        .unwrap_err()
        .to_string()
        .contains("energy_unit"));
    let selection = selection.with_energy(EnergyConversion::Kev);
    assert_eq!(scan.arrays_with(&selection).unwrap().0, [1000., 3000.]);
    let mapping = selection.resolve(&scan).unwrap();
    let decoded: SpectrumSelection =
        serde_json::from_value(serde_json::to_value(&mapping).unwrap()).unwrap();
    assert_eq!(decoded.resolve(&scan).unwrap(), mapping);
    for selector in [
        serde_json::json!(-1),
        serde_json::json!(1.5),
        serde_json::json!(true),
    ] {
        assert!(serde_json::from_value::<ColumnSelector>(selector).is_err());
    }
}

#[test]
fn reference_uses_transmitted_monitor_and_remains_an_explicit_choice() {
    let scan = parse_measurement(b"energy,I0,It,Ir,IFF\n7100,100,20,5,3\n7101,200,40,10,8\n")
        .unwrap()
        .scans
        .remove(0);
    assert_eq!(
        scan.signals
            .iter()
            .map(|s| s.name.as_str())
            .collect::<Vec<_>>(),
        ["transmission", "fluorescence", "reference"]
    );
    assert!(scan.arrays(None).is_err());
    let (_, reference) = scan.arrays(Some(&scan.signals[2].mapping)).unwrap();
    assert!(reference.iter().all(|v| (v - 4f64.ln()).abs() < 1e-14));
}

#[test]
fn selected_axis_preserves_calibration_and_rejects_conflicts() {
    let mut scan = parse_measurement(b"energy,mu\n7100,1\n7101,2\n")
        .unwrap()
        .scans
        .remove(0);
    let select = SpectrumSelection::direct("energy", "mu");
    for calibration in [
        EnergyConversion::OffsetEv { offset_ev: 20000. },
        EnergyConversion::Bragg {
            d_spacing: 3.1355,
            degrees_per_unit: 0.001,
        },
    ] {
        scan.signals[0].mapping.energy = calibration.clone();
        assert_eq!(select.resolve(&scan).unwrap().energy, calibration);
    }
    let mut conflict = scan.signals[0].clone();
    conflict.mapping.energy = EnergyConversion::Ev;
    scan.signals.push(conflict);
    assert!(select
        .resolve(&scan)
        .unwrap_err()
        .to_string()
        .contains("Conflicting"));
    assert_eq!(
        select
            .with_energy(EnergyConversion::Kev)
            .resolve(&scan)
            .unwrap()
            .energy,
        EnergyConversion::Kev
    );
}
