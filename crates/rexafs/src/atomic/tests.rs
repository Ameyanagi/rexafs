use super::*;
#[test]
fn references_are_identified_and_out_of_range_is_not_clamped() {
    let db = AtomicData::new().unwrap();
    let curve = db.f2("Cu", &[8978., 8979., 8980.]).unwrap();
    assert_eq!(curve.reference.table, AtomicTable::ChantlerF2LogLogV1);
    assert_eq!(curve.reference.data.data_sha256.len(), 64);
    assert_eq!(db.edge("Cu", "k").unwrap().energy_ev, 8979.);
    for e in [0., -1., f64::NAN, f64::INFINITY, 1e10] {
        assert!(db.f2("Cu", &[e]).is_err());
    }
    for e in [99., 800001., f64::NAN] {
        assert!(db.attenuation("Cu", &[e]).is_err());
    }
    assert!(db.f2("unobtainium", &[1000.]).is_err());
    assert!(db.f2("Cu", &[]).is_err());
    let mut reference = curve.reference;
    db.require_reference(&reference).unwrap();
    reference.data.data_sha256 = "different-data".into();
    assert!(matches!(
        db.require_reference(&reference),
        Err(AtomicDataError::VersionMismatch)
    ));
    assert_eq!(db.identity().data_version, "9.2");
    assert_eq!(
        db.identity().data_sha256,
        "fb29697588bd24ffafac8e2d5bfcf808df9a66b05987ef35b1224d3b8fb7c67e"
    );
}
#[test]
fn formula_retains_mass_fraction_arithmetic_and_rejects_unsupported_notation() {
    let db = AtomicData::new().unwrap();
    let result = db
        .compound_attenuation("Cu2O", &[8000., 9000., 10000.])
        .unwrap();
    assert_eq!(
        result
            .composition
            .iter()
            .map(|c| (&*c.element, c.atoms))
            .collect::<Vec<_>>(),
        vec![("Cu", 2.), ("O", 1.)]
    );
    let sum = result
        .composition
        .iter()
        .map(|c| c.mass_fraction)
        .sum::<f64>();
    assert!((sum - 1.).abs() < 1e-15);
    let cu = db.attenuation("Cu", &result.curve.energy_ev).unwrap();
    let o = db.attenuation("O", &result.curve.energy_ev).unwrap();
    for (i, &v) in result.curve.values.iter().enumerate() {
        assert_eq!(
            v,
            result.composition[0].mass_fraction * cu.values[i]
                + result.composition[1].mass_fraction * o.values[i]
        );
    }
    let nested = db.compound_attenuation("CuSO4(H2O)5", &[10000.]).unwrap();
    assert_eq!(
        nested
            .composition
            .iter()
            .find(|c| c.element == "H")
            .unwrap()
            .atoms,
        10.
    );
    assert_eq!(
        nested
            .composition
            .iter()
            .find(|c| c.element == "O")
            .unwrap()
            .atoms,
        9.
    );
    let fractional = db.compound_attenuation("Fe0.7Mg0.3O", &[10000.]).unwrap();
    assert_eq!(fractional.composition[0].atoms, 0.7);
    for formula in [
        "Cu0O",
        "Cu(OH",
        "Cu[OH]2",
        "CuSO4·5H2O",
        "D2O",
        "Cu1e-3O",
        "water",
        "Cu2+",
        "Cu O",
        "()",
        "XxO",
    ] {
        let message = db
            .compound_attenuation(formula, &[10000.])
            .unwrap_err()
            .to_string();
        assert!(message.contains("byte"), "{formula}: {message}");
    }
}
#[test]
fn explicit_line_and_family_have_distinct_retained_members() {
    let db = AtomicData::new().unwrap();
    let individual = db
        .emission("Cu", &EmissionSelection::Line("Ka1".into()))
        .unwrap();
    let family = db
        .emission("Cu", &EmissionSelection::Family("Ka".into()))
        .unwrap();
    assert_eq!(individual.lines.len(), 1);
    assert!(family.lines.len() > 1);
    assert_ne!(individual.energy_ev, family.energy_ev);
    let intensity = family.lines.iter().map(|l| l.intensity).sum::<f64>();
    assert_eq!(
        family.energy_ev,
        family
            .lines
            .iter()
            .map(|l| l.energy_ev * (l.intensity / intensity))
            .sum::<f64>()
    );
    assert!(db
        .emission("Cu", &EmissionSelection::Line("Ka".into()))
        .is_err());
    assert!(db
        .emission("Cu", &EmissionSelection::Family("".into()))
        .is_err());
}
