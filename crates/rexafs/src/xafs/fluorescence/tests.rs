use super::*;
use crate::Spectrum;
fn input() -> (Vec<f64>, Vec<f64>, FluorescenceCorrection) {
    let energy: Vec<_> = (0..701).map(|i| 8579. + 2. * i as f64).collect();
    let mu: Vec<_> = energy
        .iter()
        .map(|e| {
            0.2 + 0.00001 * (e - 8979.)
                + 1. / (1. + (-(e - 8979.) / 1.5).exp())
                + 0.15 * (-((e - 8990.) / 8.).powi(2)).exp()
        })
        .collect();
    let model = FluorescenceCorrection::new("CuO", "Cu", "K")
        .line("Ka1")
        .angles(45., 45.)
        .e0(8979.)
        .pre_edge(-350. ..=-50.)
        .post_edge(100. ..=900.);
    (energy, mu, model)
}
#[test]
fn fluorescence_retains_inputs_replays_and_normalizes_without_mutation() {
    let (e, m, model) = input();
    let mut original = Spectrum::from_arrays(&e, &m).unwrap();
    original.normalize().unwrap();
    let saved = original.clone();
    let mut corrected = original.correct_fluorescence(&model).unwrap();
    assert_eq!(original, saved);
    let record = corrected.fluorescence_correction().unwrap().clone();
    assert_eq!(record.original_mu, m);
    assert_eq!(record.input_mode, AbsorptionMode::Unknown);
    assert_eq!(record.energy, e);
    let replay = record.definition().apply(&e, &m).unwrap();
    assert_eq!(record.corrected_mu, replay.corrected_mu);
    corrected.normalize().unwrap();
    assert!(corrected.norm().is_some());
    assert!(corrected.calc_background().is_err());
    assert!(corrected.fft().is_err());
    assert!(corrected.correct_fluorescence(&model).is_err());
    let json = serde_json::to_string(&corrected).unwrap();
    let mut restored: Spectrum = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.fluorescence_correction(), Some(&record));
    assert!(restored.fft().is_err());
    restored
        .set_normalization_method(
            crate::MBack::for_edge("Cu", "K")
                .pre_edge(-350. ..=-50.)
                .post_edge(100. ..=900.),
        )
        .unwrap()
        .normalize()
        .unwrap();
    assert!(restored.norm().is_some());
    original.set_absorption_mode(AbsorptionMode::Transmission);
    assert!(original
        .correct_fluorescence(&model)
        .unwrap_err()
        .to_string()
        .contains("transmission"));
}
#[test]
fn fluorescence_rejects_invalid_science_without_clipping() {
    let (e, m, model) = input();
    for angle in [0., -45., 91., f64::NAN, f64::INFINITY] {
        assert!(model.clone().angles(angle, 45.).apply(&e, &m).is_err());
        assert!(model.clone().angles(45., angle).apply(&e, &m).is_err());
    }
    assert!(FluorescenceCorrection::new("CuO", "Cu", "K")
        .apply(&e, &m)
        .is_err());
    assert!(model.clone().line("La1").apply(&e, &m).is_err());
    assert!(model
        .clone()
        .e0(e[e.len() - 1] - 0.1)
        .apply(&e, &m)
        .is_err());
    assert!(model
        .clone()
        .pre_edge(-1000. ..=-50.)
        .apply(&e, &m)
        .is_err());
    assert!(model.clone().post_edge(100. ..=101.).apply(&e, &m).is_err());
    let mut missing = model.clone();
    missing.formula = "Fe2O3".into();
    assert!(missing.apply(&e, &m).is_err());
    assert!(model
        .apply(&e, &m.iter().map(|v| -v).collect::<Vec<_>>())
        .is_err());
    let mut large = m.clone();
    large[210] = 1e6;
    assert!(model
        .apply(&e, &large)
        .unwrap_err()
        .to_string()
        .contains("denominator"));
    let mut identity = model.apply(&e, &m).unwrap().definition();
    identity.reference.as_mut().unwrap().data_version = "unavailable".into();
    assert!(identity.apply(&e, &m).is_err());
}
#[test]
fn fluorescence_dilute_limit_and_input_unit_invariance() {
    let (e, m, model) = input();
    let ordinary = model.apply(&e, &m).unwrap();
    let scaled = model
        .apply(&e, &m.iter().map(|v| v * 1e-8).collect::<Vec<_>>())
        .unwrap();
    for (a, b) in ordinary.factor.iter().zip(scaled.factor) {
        assert!((a - b).abs() < 1e-7);
    }
    let mut dilute = model.clone();
    dilute.formula = "Cu0.001SiO2".into();
    let weak = dilute.apply(&e, &m).unwrap();
    assert!(weak.factor.iter().all(|f| (f - 1.).abs() < 0.02));
    for (m, c, f) in itertools::izip!(&m, &ordinary.corrected_mu, &ordinary.factor) {
        assert_eq!(m * f, *c);
    }
    let grazing = model.angles(45., 1.).apply(&e, &m).unwrap();
    assert!(grazing.warnings.iter().any(|w| w.contains("grazing")));
    let mut too_dilute = dilute;
    too_dilute.formula = "Cu0.000001SiO2".into();
    assert!(too_dilute
        .apply(&e, &m)
        .unwrap_err()
        .to_string()
        .contains("positive jump"));
}

#[test]
fn fluorescence_inverts_the_assumed_model_and_reports_near_singular_data() {
    let (e, _, model) = input();
    let baseline: Vec<_> = e.iter().map(|v| if *v < 8979. { 0. } else { 1. }).collect();
    let alpha = model.apply(&e, &baseline).unwrap().alpha;
    let truth: Vec<_> = e
        .iter()
        .map(|v| {
            if *v < 8979. {
                0.
            } else {
                1. + if *v < 9050. {
                    0.3 * (-((v - 8999.) / 8.).powi(2)).exp()
                } else {
                    0.
                }
            }
        })
        .collect();
    let measured: Vec<_> = truth
        .iter()
        .map(|t| (alpha + 1.) * t / (alpha + t))
        .collect();
    let recovered = model.apply(&e, &measured).unwrap();
    for (actual, expected) in recovered.corrected_mu.iter().zip(&truth) {
        assert!((actual - expected).abs() < 1e-10);
    }
    let mut unstable = baseline;
    unstable[210] = alpha + 1. - 100. * recovered.singularity_threshold;
    let near = model.apply(&e, &unstable).unwrap();
    assert!(near.maximum_amplification > 1e10);
    assert!(near
        .warnings
        .iter()
        .any(|w| w.contains("Large amplification")));
    unstable[210] = (alpha + 1. - 0.5 * recovered.singularity_threshold) * near.internal.edge_step
        + near.internal.pre_curve[210];
    assert!(model.apply(&e, &unstable).is_err());
}

#[test]
fn fluorescence_import_does_not_reinterpret_transmission_or_generic_yield() {
    use crate::xafs::io::parse_measurement;
    let transmission =
        parse_measurement(b"# energy I0 It\n8900 10 9\n8910 10 8\n8920 10 7\n").unwrap();
    assert_eq!(
        transmission.scans[0]
            .to_spectrum(None)
            .unwrap()
            .absorption_mode(),
        AbsorptionMode::Transmission
    );
    let ratio = parse_measurement(b"# energy I0 If\n8900 10 1\n8910 10 2\n8920 10 3\n").unwrap();
    assert_eq!(
        ratio.scans[0].to_spectrum(None).unwrap().absorption_mode(),
        AbsorptionMode::Unknown
    );
}

#[test]
fn fluorescence_inherited_xanes_restriction_survives_edits_and_roundtrip() {
    let (e, m, model) = input();
    let mut sp = Spectrum::from_arrays(&e, &m).unwrap();
    sp.normalize().unwrap();
    sp.restrict_to_xanes();
    assert!(sp.is_xanes_only());
    assert!(sp.fluorescence_correction().is_none());
    sp.set_spectrum(e, m);
    sp.set_absorption_mode(AbsorptionMode::Fluorescence);
    let mut restored: Spectrum =
        serde_json::from_str(&serde_json::to_string(&sp).unwrap()).unwrap();
    restored.normalize().unwrap();
    assert!(restored.is_xanes_only());
    assert!(restored.calc_background().is_err());
    assert!(restored.fft().is_err());
    assert!(restored.correct_fluorescence(&model).is_err());
}
