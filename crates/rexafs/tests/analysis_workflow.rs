//! Shared analysis preparation, coverage, batch and count-diagnostic contracts.
use rexafs::prelude::*;

fn raw(bump: f64) -> Spectrum {
    let energy: Vec<_> = (0..651).map(|i| 8700. + 2. * i as f64).collect();
    let mu: Vec<_> = energy
        .iter()
        .map(|e| {
            let x = e - 8979.;
            0.1 + 0.0001 * x
                + 1. / (1. + (-x / 2.).exp())
                + bump * (-((x - 10.) / 6.).powi(2)).exp()
        })
        .collect();
    let mut spectrum = Spectrum::from_arrays(&energy, &mu).unwrap();
    spectrum.set_name(format!("sample {bump}"));
    spectrum
        .set_normalization_method(PrePostEdge {
            e0: Some(8979.),
            pre_edge_start: Some(-200.),
            pre_edge_end: Some(-50.),
            norm_start: Some(100.),
            norm_end: Some(700.),
            norm_polyorder: Some(1),
            ..PrePostEdge::new()
        })
        .unwrap();
    spectrum
}

fn prepared(start: i32, end: i32, fraction: f64) -> Spectrum {
    let energy: Vec<_> = (start..=end).map(|i| 8979. + i as f64).collect();
    let mu: Vec<_> = energy
        .iter()
        .map(|e| {
            let x = (e - 8950.) / 100.;
            fraction * (0.2 + x * x) + (1. - fraction) * (0.1 + (x * 5.).sin().powi(2))
        })
        .collect();
    Spectrum::from_prepared(&energy, &mu, AnalysisSpace::Flat, 8979.).unwrap()
}

#[test]
fn raw_inputs_use_existing_settings_on_copies_in_all_analyses() {
    let raw: Vec<_> = (0..5).map(|i| raw(0.1 * i as f64)).collect();
    let before = serde_json::to_value(&raw).unwrap();
    let manual: Vec<_> = raw
        .iter()
        .cloned()
        .map(|mut s| {
            s.normalize().unwrap();
            s
        })
        .collect();
    for space in [
        AnalysisSpace::Norm,
        AnalysisSpace::Flat,
        AnalysisSpace::Deriv,
    ] {
        let cfg = LcfConfig {
            space,
            ..Default::default()
        };
        let automatic = lcf(&raw[2], &[&raw[0], &raw[4]], &cfg).unwrap();
        let explicit = lcf(&manual[2], &[&manual[0], &manual[4]], &cfg).unwrap();
        assert_eq!(automatic, explicit);
        let cfg = PcaConfig {
            space,
            center: true,
            ..Default::default()
        };
        assert_eq!(
            pca_train(&raw, &cfg).unwrap(),
            pca_train(&manual, &cfg).unwrap()
        );
        if space != AnalysisSpace::Deriv {
            let cfg = McrConfig {
                space,
                components: 2,
                ..Default::default()
            };
            assert_eq!(
                serde_json::to_value(mcr_als(&raw, &cfg).unwrap()).unwrap(),
                serde_json::to_value(mcr_als(&manual, &cfg).unwrap()).unwrap()
            );
        }
    }
    assert_eq!(serde_json::to_value(&raw).unwrap(), before);
    assert!(raw[0].norm().is_none());
    assert_eq!(pca_train(&raw, &PcaConfig::default()).unwrap().x[0], 8960.);
}

#[test]
fn unset_edge_is_found_on_the_copy_before_resolving_offsets() {
    let configured = raw(0.2);
    let spectrum = Spectrum::from_arrays(
        configured.energy.as_ref().unwrap().as_slice(),
        configured.mu.as_ref().unwrap().as_slice(),
    )
    .unwrap();
    let mut manual = spectrum.clone();
    manual.normalize().unwrap();
    let model = pca_train(&[&spectrum, &spectrum], &PcaConfig::default()).unwrap();
    assert_eq!(
        model,
        pca_train(&[&manual, &manual], &PcaConfig::default()).unwrap()
    );
    assert!(spectrum.e0.is_none());
    assert!(spectrum.norm().is_none());
}

#[test]
fn chi_preparation_matches_explicit_background_and_does_not_touch_input() {
    let original = raw(0.2);
    let before = serde_json::to_value(&original).unwrap();
    let mut explicit = original.clone();
    explicit.calc_background().unwrap();
    let space = AnalysisSpace::Chi { kweight: 2. };
    assert_eq!(
        space.arrays(&original).unwrap(),
        space.arrays(&explicit).unwrap()
    );
    assert_eq!(serde_json::to_value(&original).unwrap(), before);
}

#[test]
fn prepared_arrays_are_reused_and_norm_is_not_relabeled_flat() {
    let mut s = prepared(-25, 35, 0.4);
    let original = s.mu.clone().unwrap();
    // Too little coverage for an ordinary normalization; existing arrays suffice.
    assert_eq!(AnalysisSpace::Flat.arrays(&s).unwrap().1, original);
    assert!(lcf(
        &s,
        &[&s],
        &LcfConfig {
            space: AnalysisSpace::Flat,
            ..Default::default()
        }
    )
    .is_ok());
    s = Spectrum::from_prepared(
        s.energy.as_ref().unwrap().as_slice(),
        original.as_slice(),
        AnalysisSpace::Norm,
        8979.,
    )
    .unwrap();
    assert!(matches!(
        AnalysisSpace::Flat.arrays(&s),
        Err(AnalysisError::InvalidInput { .. })
    ));
    assert!(s.preserves_prepared_values());
    let invalid = Spectrum::new();
    assert!(matches!(
        AnalysisSpace::Norm.arrays(&invalid),
        Err(AnalysisError::Preparation { .. })
    ));
}

#[test]
fn all_analyses_reject_invalid_bounds_and_incomplete_requested_coverage() {
    let data: Vec<_> = (0..5).map(|i| prepared(-25, 35, i as f64 / 4.)).collect();
    for range in [
        (30., -20.),
        (0., 0.),
        (f64::NAN, 30.),
        (-20., f64::INFINITY),
    ] {
        let l = lcf(
            &data[2],
            &[&data[0], &data[4]],
            &LcfConfig {
                range: Some(range),
                ..Default::default()
            },
        );
        let p = pca_train(
            &data,
            &PcaConfig {
                range: Some(range),
                ..Default::default()
            },
        );
        let m = mcr_als(
            &data,
            &McrConfig {
                range: Some(range),
                components: 2,
                ..Default::default()
            },
        );
        assert!(
            matches!(l, Err(AnalysisError::InvalidRange { .. })),
            "{l:?}"
        );
        assert!(
            matches!(p, Err(AnalysisError::InvalidRange { .. })),
            "{p:?}"
        );
        assert!(
            matches!(m, Err(AnalysisError::InvalidRange { .. })),
            "{m:?}"
        );
    }
    for range in [(-26., 30.), (-20., 36.)] {
        assert!(matches!(
            lcf(
                &data[0],
                &data[1..],
                &LcfConfig {
                    range: Some(range),
                    ..Default::default()
                }
            ),
            Err(AnalysisError::IncompleteCoverage { .. })
        ));
        assert!(matches!(
            pca_train(
                &data,
                &PcaConfig {
                    range: Some(range),
                    ..Default::default()
                }
            ),
            Err(AnalysisError::IncompleteCoverage { .. })
        ));
        assert!(matches!(
            mcr_als(
                &data,
                &McrConfig {
                    range: Some(range),
                    components: 2,
                    ..Default::default()
                }
            ),
            Err(AnalysisError::IncompleteCoverage { .. })
        ));
    }
    let narrow = prepared(-19, 29, 0.4);
    assert!(matches!(
        lcf(&data[0], &[&narrow], &LcfConfig::default()),
        Err(AnalysisError::IncompleteCoverage { .. })
    ));
    assert!(matches!(
        pca_train(&[&data[0], &narrow], &PcaConfig::default()),
        Err(AnalysisError::IncompleteCoverage { .. })
    ));
    assert!(matches!(
        mcr_als(
            &[&data[0], &narrow],
            &McrConfig {
                components: 2,
                ..Default::default()
            }
        ),
        Err(AnalysisError::IncompleteCoverage { .. })
    ));
    let model = pca_train(&data, &PcaConfig::default()).unwrap();
    assert!(matches!(
        model.target_transform(&narrow, 2),
        Err(AnalysisError::IncompleteCoverage { .. })
    ));
    // Shifted LCF needs coverage for every permitted shift, not only zero shift.
    assert!(matches!(
        lcf(
            &data[0],
            &[&data[1]],
            &LcfConfig {
                fit_e0_shift: true,
                max_e0_shift: 6.,
                ..Default::default()
            }
        ),
        Err(AnalysisError::IncompleteCoverage { .. })
    ));
}

#[test]
fn batch_rows_match_single_fits_retain_failures_and_cancel_in_order() {
    let data: Vec<_> = (0..5).map(|i| prepared(-25, 35, i as f64 / 4.)).collect();
    let bad = prepared(-19, 29, 0.5);
    let inputs = [&data[1], &bad, &data[3]];
    let standards = [&data[0], &data[4]];
    let cfg = LcfConfig::default();
    let rows = lcf_batch(&inputs, &standards, &cfg);
    assert_eq!(rows.len(), 3);
    assert_eq!(
        rows[0].as_ref().unwrap(),
        &lcf(inputs[0], &standards, &cfg).unwrap()
    );
    assert!(matches!(
        rows[1],
        Err(AnalysisError::IncompleteCoverage { .. })
    ));
    assert_eq!(
        rows[2].as_ref().unwrap(),
        &lcf(inputs[2], &standards, &cfg).unwrap()
    );
    let mut reported = Vec::new();
    let rows = lcf_batch_with_progress(&inputs, &standards, &cfg, |index, row| {
        reported.push((index, row.is_ok()));
        index < 1
    });
    assert_eq!(reported, vec![(0, true), (1, false)]);
    assert_eq!(rows.len(), 2);
    assert!(lcf_batch::<Spectrum, _>(&[], &standards, &cfg).is_empty());
    let failed = lcf_batch(&inputs, &[Spectrum::new()], &cfg);
    assert_eq!(failed.len(), inputs.len());
    assert!(failed
        .iter()
        .all(|r| matches!(r, Err(AnalysisError::Preparation { .. }))));
}

#[test]
fn core_count_diagnostics_match_reconstruction_with_and_without_centering() {
    let data: Vec<_> = (0..9).map(|i| prepared(-25, 35, i as f64 / 8.)).collect();
    for (center, rank) in [(false, 2), (true, 1)] {
        let model = pca_train(
            &data,
            &PcaConfig {
                center,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(model.numerical_rank(), rank);
        assert_eq!(
            model.component_count_suggestion(),
            Some(PcaCountSuggestion {
                count: rank,
                basis: PcaCountBasis::NumericalRank
            })
        );
        let curve = model.reconstruction_errors();
        assert_eq!(curve.len(), model.n_components() + 1);
        for point in curve {
            let actual: f64 = (0..data.len())
                .map(|i| {
                    model
                        .reconstruct_training(i, point.components)
                        .unwrap()
                        .chi_square
                })
                .sum();
            assert!(
                (actual - point.sse).abs() < 1e-20 + actual * 1e-10,
                "{}: {actual} != {}",
                point.components,
                point.sse
            );
            assert!((point.relative_error - point.sse / model.data.norm_squared()).abs() < 1e-12);
        }
    }
    let energy: Vec<_> = (0..61).map(|i| 8954. + i as f64).collect();
    let zero = Spectrum::from_prepared(&energy, &vec![0.; 61], AnalysisSpace::Norm, 8979.).unwrap();
    let model = pca_train(&[&zero, &zero], &PcaConfig::default()).unwrap();
    assert_eq!(model.component_count_suggestion(), None);
    assert!(model
        .reconstruction_errors()
        .iter()
        .all(|r| r.relative_error.is_nan()));
}

#[test]
fn an_ind_suggestion_requires_an_interior_minimum() {
    let energy: Vec<_> = (0..61).map(|i| 8954. + i as f64).collect();
    let spectra: Vec<_> = (0..4)
        .map(|i| {
            let mu: Vec<_> = (0..61)
                .map(|j| ((j as f64 + 1.) * (i as f64 + 1.) * 0.1).sin())
                .collect();
            Spectrum::from_prepared(&energy, &mu, AnalysisSpace::Norm, 8979.).unwrap()
        })
        .collect();
    let mut model = pca_train(&spectra, &PcaConfig::default()).unwrap();
    assert_eq!(model.numerical_rank(), 4);
    // Controlled indicator curves isolate endpoint handling from noise estimation.
    for ind in [vec![1., 2., 3., 4.], vec![4., 3., 2., 1.]] {
        model.ind = ind;
        assert_eq!(model.component_count_suggestion(), None);
    }
    model.ind = vec![4., 3., 1., 2.];
    assert_eq!(
        model.component_count_suggestion(),
        Some(PcaCountSuggestion {
            count: 2,
            basis: PcaCountBasis::IndicatorMinimum
        })
    );
}
