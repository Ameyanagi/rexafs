use super::*;
#[test]
fn synthetic_scale_background_and_input_scaling_are_recovered() {
    let data = AtomicData::new().unwrap();
    let energy = (0..=700).map(|i| 8679. + 2. * i as f64).collect::<Vec<_>>();
    let f2 = data.f2("Cu", &energy).unwrap().values;
    let mu = energy
        .iter()
        .zip(&f2)
        .map(|(e, f)| (f + 0.7 + 0.002 * (e - 8979.) + 1e-6 * (e - 8979.).powi(2)) / 3.2)
        .collect::<Vec<_>>();
    let model = MBack::for_edge("Cu", "K")
        .e0(8979.)
        .pre_edge(-300.0..=-50.0)
        .post_edge(100.0..=1100.0);
    let result = model.fit(&energy, &mu).unwrap();
    assert!((result.scale - 3.2).abs() < 1e-10);
    assert!(result.objective < 1e-20);
    for (a, b) in result.fpp.iter().zip(&f2) {
        assert!((a - b).abs() < 1e-10);
    }
    let scaled = model
        .fit(&energy, &mu.iter().map(|v| v * 1e6).collect::<Vec<_>>())
        .unwrap();
    assert!((scaled.scale * 1e6 - result.scale).abs() < 1e-9);
    assert!((scaled.edge_step / 1e6 - result.edge_step).abs() < 1e-8);
    for (a, b) in scaled.norm.iter().zip(&result.norm) {
        assert!((a - b).abs() < 1e-7);
    }
    let mut spectrum = crate::Spectrum::from_arrays(&energy, &mu).unwrap();
    spectrum
        .set_normalization_method(model)
        .unwrap()
        .normalize()
        .unwrap();
    assert_eq!(spectrum.norm().unwrap().as_slice(), result.norm);
    let Some(crate::NormalizationMethod::MBack(saved)) = &spectrum.normalization else {
        panic!("MBACK missing")
    };
    assert_eq!(saved.options.reference.as_ref(), Some(&result.reference));
    spectrum.invalidate_derived();
    let Some(crate::NormalizationMethod::MBack(saved)) = spectrum.normalization else {
        panic!("MBACK missing")
    };
    assert!(saved.result.is_none());
}

fn synthetic() -> (Vec<f64>, Vec<f64>, MBack) {
    let energy = (0..=250)
        .map(|i| 8579. + 6. * i as f64 + if i % 2 == 0 { 0. } else { 0.2 })
        .collect::<Vec<_>>();
    let f2 = AtomicData::new().unwrap().f2("Cu", &energy).unwrap().values;
    let mu = energy
        .iter()
        .zip(&f2)
        .map(|(&e, &f)| (f + 1. + 0.001 * (e - 8979.)) / 2.)
        .collect::<Vec<_>>();
    (
        energy,
        mu,
        MBack::for_edge("Cu", "K")
            .e0(8979.)
            .pre_edge(-390.0..=-60.0)
            .post_edge(100.0..=1000.0),
    )
}
#[test]
fn irregular_grid_balances_regions_and_preserves_excluded_structure() {
    let (energy, mut mu, model) = synthetic();
    for (e, m) in energy.iter().zip(&mut mu) {
        if *e > 8960. && *e < 9030. {
            *m += 0.8;
        }
    }
    let result = model.fit(&energy, &mu).unwrap();
    assert!((result.scale - 2.).abs() < 1e-10);
    let mut weights = [0.; 2];
    for (&i, &w) in result.fit_indices.iter().zip(&result.weights) {
        weights[usize::from(energy[i] > 8979.)] += w * w;
    }
    for sum in weights {
        assert!((sum - 1.).abs() < 1e-13);
    }
    assert!(result.residual.iter().any(|r| r.abs() > 1.5));
    assert!(result.objective < 1e-20);
    let recovered: MbackResult =
        serde_json::from_str(&serde_json::to_string(&result).unwrap()).unwrap();
    assert_eq!(recovered, result);
}
#[test]
fn ranges_identity_and_nonidentifiable_requests_fail() {
    let (energy, mu, model) = synthetic();
    for bad in [
        model.clone().pre_edge(-900.0..=-50.0),
        model.clone().pre_edge(5.0..=50.0),
        model.clone().post_edge(10.0..=11.0),
        model.clone().degree(6),
        model.clone().e0(500.),
        MBack::new(),
    ] {
        assert!(bad.fit(&energy, &mu).is_err());
    }
    assert!(model.fit(&energy, &vec![1.; energy.len()]).is_err());
    assert!(model
        .fit(&energy, &mu.iter().map(|v| -v).collect::<Vec<_>>())
        .is_err());
    let mut nan = mu.clone();
    nan[8] = f64::NAN;
    assert!(model.fit(&energy, &nan).is_err());
    let mut duplicate = energy.clone();
    duplicate[4] = duplicate[3];
    assert!(model.fit(&duplicate, &mu).is_err());
    let mut original = model.fit(&energy, &mu).unwrap().reference;
    original.data.data_sha256 = "unavailable".into();
    assert!(model
        .reference(original)
        .fit(&energy, &mu)
        .unwrap_err()
        .to_string()
        .contains("archived"));
    let low = (0..10)
        .map(|i| 10000000. + i as f64 * 10.)
        .collect::<Vec<_>>();
    assert!(MBack::for_edge("Cu", "K")
        .e0(low[4])
        .fit(&low, &vec![1.; 10])
        .is_err());
}
#[test]
fn neighboring_edges_require_revised_regions_and_auto_ranges_record_the_margin() {
    let data = AtomicData::new().unwrap();
    let edge = data.edge("Au", "L3").unwrap().energy_ev;
    let l2 = data.edge("Au", "L2").unwrap().energy_ev;
    let energy = (0..=700)
        .map(|i| edge - 600. + i as f64 * 5.)
        .collect::<Vec<_>>();
    let mu = data.f2("Au", &energy).unwrap().values;
    let bad = MBack::for_edge("Au", "L3")
        .e0(edge)
        .pre_edge(-500.0..=-100.0)
        .post_edge(100.0..=2200.0);
    assert!(bad
        .fit(&energy, &mu)
        .unwrap_err()
        .to_string()
        .contains("neighboring Au L2"));
    let result = MBack::for_edge("Au", "L3")
        .e0(edge)
        .fit(&energy, &mu)
        .unwrap();
    assert_eq!(result.post_edge[1] + edge, l2 - 10.);
    assert_eq!(result.requested.post_edge, None);
}
#[test]
fn erfc_recovers_width_amplitude_and_refits_at_an_active_bound() {
    use errorfunctions::RealErrorFunctions;
    let (energy, _, model) = synthetic();
    let data = AtomicData::new().unwrap();
    let line = data
        .emission("Cu", &EmissionSelection::Line("Ka1".into()))
        .unwrap()
        .energy_ev;
    let f2 = data.f2("Cu", &energy).unwrap().values;
    let mu = energy
        .iter()
        .zip(&f2)
        .map(|(&e, &f)| {
            (f + 1. + 0.001 * (e - 8979.) + 3. * RealErrorFunctions::erfc((e - line) / 900.)) / 2.
        })
        .collect::<Vec<_>>();
    let model = model
        .degree(1)
        .erfc(MbackErfc::new("Ka1", 500.0..=1500.0, 0.0..=10.0));
    let result = model.fit(&energy, &mu).unwrap();
    assert!((result.scale - 2.).abs() < 1e-7);
    assert!((result.erfc_width.unwrap() - 900.).abs() < 0.01);
    assert!((result.erfc_amplitude - 3.).abs() < 1e-5);
    assert!(result.objective < 1e-15);
    let limited = model
        .erfc(MbackErfc::new("Ka1", 500.0..=1500.0, 0.0..=2.0))
        .fit(&energy, &mu)
        .unwrap();
    assert_eq!(limited.erfc_amplitude, 2.);
    assert!(limited.warnings.iter().any(|s| s.contains("amplitude")));
    assert!(limited.objective > result.objective);
    // At the active amplitude bound, scale and polynomial still solve their
    // normal stationarity conditions; clamping an unconstrained fit fails this.
    for column in 0..3 {
        let gradient: f64 = limited
            .fit_indices
            .iter()
            .zip(&limited.weights)
            .map(|(&i, &w)| {
                let basis = match column {
                    0 => mu[i],
                    1 => 1.,
                    _ => (energy[i] - limited.e0) / limited.energy_scale,
                };
                limited.residual[i] * w * w * basis
            })
            .sum();
        assert!(
            gradient.abs() < 1e-10,
            "unrefitted column {column}: {gradient}"
        );
    }
}
#[test]
fn erfc_rejects_incompatible_lines_and_unidentifiable_zero_background() {
    let (energy, mu, model) = synthetic();
    let incompatible = model
        .clone()
        .erfc(MbackErfc::new("La1", 500.0..=1500.0, -10.0..=10.0));
    assert!(incompatible
        .fit(&energy, &mu)
        .unwrap_err()
        .to_string()
        .contains("selected absorber edge"));
    let tiny = model.erfc(MbackErfc::new("Ka1", 1.0..=5.0, -10.0..=10.0));
    assert!(tiny.fit(&energy, &mu).is_err());
}
#[test]
fn historical_output_remains_readable_but_failed_recalculation_clears_results() {
    let old: MBack = serde_json::from_str(r#"{"e0":8979.0,"edge_step":0.5}"#).unwrap();
    assert_eq!(old.edge_step, Some(0.5));
    let (energy, mu, mut model) = synthetic();
    let e = DVector::from_vec(energy);
    let m = DVector::from_vec(mu);
    model.normalize(&e, &m).unwrap();
    model.options.element = "unknown".into();
    assert!(model.normalize(&e, &m).is_err());
    assert!(
        model.norm.is_none()
            && model.flat.is_none()
            && model.result.is_none()
            && model.edge_step.is_none()
    );
}
