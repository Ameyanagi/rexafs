//! Prepared absorption must retain its scale throughout downstream processing.
use rexafs::prelude::*;

fn input() -> (Vec<f64>, Vec<f64>) {
    let energy: Vec<_> = (0..=650).map(|i| 8800. + 2. * i as f64).collect();
    let mu = energy
        .iter()
        .map(|e| {
            let above = (e - 8980.).max(0.);
            let k = (above / 3.81).sqrt();
            1. / (1. + (-(e - 8980.) / 2.).exp())
                + 0.06 * (4. * k).sin() * (-above / 800.).exp() * (above / 20.).min(1.)
        })
        .collect();
    (energy, mu)
}

#[test]
fn prepared_input_survives_background_fourier_edits_and_serialization() {
    let (energy, mu) = input();
    for space in [AnalysisSpace::Norm, AnalysisSpace::Flat] {
        let mut spectrum = XASSpectrum::from_prepared(&energy, &mu, space, 8980.).unwrap();
        spectrum.calc_background().unwrap().fft().unwrap();
        assert!(spectrum.chi().unwrap().iter().all(|v| v.is_finite()));
        assert!(spectrum.k().unwrap().last().unwrap() > &12.);
        assert!(spectrum.xftf.is_some());
        assert_eq!(spectrum.norm().unwrap().as_slice(), mu);
        assert_eq!(spectrum.flat().is_some(), space == AnalysisSpace::Flat);
        assert_eq!(
            spectrum.normalization.as_ref().unwrap().get_edge_step(),
            Some(1.)
        );
        assert!(spectrum.preserves_prepared_values());
        spectrum.set_e0(8981.);
        spectrum.calc_background().unwrap().fft().unwrap();
        assert_eq!(spectrum.norm().unwrap().as_slice(), mu);
        let json = serde_json::to_string(&spectrum).unwrap();
        let mut restored: XASSpectrum = serde_json::from_str(&json).unwrap();
        restored.invalidate_derived();
        restored.calc_background().unwrap().fft().unwrap();
        assert_eq!(restored.prepared_space(), Some(space));
        assert_eq!(restored.norm().unwrap().as_slice(), mu);
        assert_eq!(restored.e0(), Some(8981.));
        restored.set_spectrum(energy.clone(), mu.clone());
        assert_eq!(restored.prepared_space(), None);
        restored
            .set_normalization_method(PrePostEdge::new())
            .unwrap();
    }
}

#[test]
fn explicit_prepost_refit_keeps_original_component_and_survives_reload() {
    let (energy, values) = input();
    let values: Vec<_> = values
        .iter()
        .zip(&energy)
        .map(|(y, e)| 0.17 + 0.0001 * (e - 8980.) + 0.8 * y)
        .collect();
    let settings = PrePostEdge {
        e0: Some(8980.),
        pre_edge_start: Some(-150.),
        pre_edge_end: Some(-50.),
        norm_start: Some(200.),
        norm_end: Some(900.),
        norm_polyorder: Some(2),
        n_victoreen: Some(0),
        ..PrePostEdge::new()
    };
    let mut expected = XASSpectrum::from_arrays(&energy, &values).unwrap();
    expected
        .set_normalization_method(settings.clone())
        .unwrap()
        .normalize()
        .unwrap();
    let mut component =
        XASSpectrum::from_prepared(&energy, &values, AnalysisSpace::Flat, 8980.).unwrap();
    component
        .set_normalization_method(settings)
        .unwrap()
        .normalize()
        .unwrap();
    assert!(!component.preserves_prepared_values());
    assert_eq!(component.prepared_space(), Some(AnalysisSpace::Flat));
    assert_eq!(component.norm(), expected.norm());
    assert_eq!(component.flat(), expected.flat());
    assert_ne!(component.norm().unwrap().as_slice(), values);
    assert_eq!(component.raw_mu.as_ref().unwrap().as_slice(), values);
    assert_eq!(component.mu.as_ref().unwrap().as_slice(), values);
    component.calc_background().unwrap().fft().unwrap();
    let mut restored: XASSpectrum =
        serde_json::from_str(&serde_json::to_string(&component).unwrap()).unwrap();
    restored.set_e0(8980.).normalize().unwrap();
    assert!(!restored.preserves_prepared_values());
    assert_eq!(restored.norm(), expected.norm());
}

#[test]
fn prepared_input_rejects_ambiguous_units_and_invalid_arrays() {
    let (energy, mu) = input();
    for space in [AnalysisSpace::Deriv, AnalysisSpace::Chi { kweight: 2. }] {
        assert!(XASSpectrum::from_prepared(&energy, &mu, space, 8980.).is_err());
    }
    for e0 in [f64::NAN, 8700., 10100.] {
        assert!(XASSpectrum::from_prepared(&energy, &mu, AnalysisSpace::Flat, e0).is_err());
    }
    assert!(XASSpectrum::from_prepared(&energy, &mu[1..], AnalysisSpace::Flat, 8980.).is_err());
}

#[test]
fn mcr_components_are_owned_processable_spectra_with_historical_fallback() {
    let (energy, mu) = input();
    let data: Vec<_> = (0..8)
        .map(|i| {
            let weight = i as f64 / 7.;
            let values: Vec<_> = mu
                .iter()
                .zip(&energy)
                .map(|(m, e)| m + weight * 0.15 * (-(e - 9000.).powi(2) / 300.).exp())
                .collect();
            XASSpectrum::from_prepared(&energy, &values, AnalysisSpace::Flat, 8980.).unwrap()
        })
        .collect();
    let result = mcr_als(
        &data,
        &McrConfig {
            space: AnalysisSpace::Flat,
            range: Some((-150., 1000.)),
            components: 2,
            ..Default::default()
        },
    )
    .unwrap();
    let mut component = result.component_spectrum(0).unwrap();
    let expected: Vec<_> = result.spectra.row(0).iter().copied().collect();
    assert_eq!(component.flat().unwrap().as_slice(), expected);
    component.calc_background().unwrap().fft().unwrap();
    assert!(component.chi().unwrap().iter().all(|v| v.is_finite()));
    assert!(component.xftf.is_some());
    component.set_e0(8981.).normalize().unwrap();
    assert_eq!(result.e0, Some(8980.));
    assert_eq!(
        result.spectra.row(0).iter().copied().collect::<Vec<_>>(),
        expected
    );
    assert!(result.component_spectrum(2).is_err());
    let mut historical = serde_json::to_value(&result).unwrap();
    historical.as_object_mut().unwrap().remove("e0");
    let historical: McrResult = serde_json::from_value(historical).unwrap();
    assert!(historical.component_spectrum(0).is_err());
    let restored = XASSpectrum::from_prepared(
        historical.x.as_slice(),
        &expected,
        historical.config.space,
        8980.,
    )
    .unwrap();
    assert_eq!(restored.flat().unwrap().as_slice(), expected);
}
