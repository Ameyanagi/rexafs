//! Small, self-contained MCR contract tests retained in the published crate.
use nalgebra::DMatrix;
use rexafs::prelude::*;

fn collection() -> Vec<XASSpectrum> {
    (0..9)
        .map(|i| {
            let energy: Vec<_> = (0..41).map(|j| 8970.0 + j as f64).collect();
            let w = i as f64 / 8.0;
            let mu: Vec<_> = (0..41)
                .map(|j| {
                    let x = j as f64 / 40.0;
                    w * (0.1 + x * x) + (1.0 - w) * (0.2 + (x * 5.0).sin().powi(2))
                })
                .collect();
            XASSpectrum::from_prepared(&energy, &mu, AnalysisSpace::Norm, 8980.).unwrap()
        })
        .collect()
}
fn cfg() -> McrConfig {
    McrConfig {
        components: 2,
        range: Some((-10.0, 30.0)),
        ..McrConfig::default()
    }
}

#[test]
fn constraints_and_explicit_anchors_are_preserved_in_owned_results() {
    let mut data = collection();
    let config = McrConfig {
        nonnegative_spectra: true,
        anchors: vec![
            McrAnchor {
                sample: 0,
                weights: vec![0., 1.],
            },
            McrAnchor {
                sample: 8,
                weights: vec![1., 0.],
            },
        ],
        ..cfg()
    };
    let result = mcr_als(&data, &config).unwrap();
    assert!(result.relative_error < 1e-20);
    assert!(result.spectra.iter().all(|v| *v >= 0.0));
    assert_eq!(
        result
            .concentrations
            .row(0)
            .iter()
            .copied()
            .collect::<Vec<_>>(),
        vec![0., 1.]
    );
    assert_eq!(
        result
            .concentrations
            .row(8)
            .iter()
            .copied()
            .collect::<Vec<_>>(),
        vec![1., 0.]
    );
    for i in 0..9 {
        assert!((result.concentrations[(i, 0)] - i as f64 / 8.).abs() < 1e-8);
    }
    let old = result.data[(0, 0)];
    data[0].set_e0(8981.0);
    assert_eq!(result.data[(0, 0)], old);
    let restored: McrResult =
        serde_json::from_str(&serde_json::to_string(&result).unwrap()).unwrap();
    assert_eq!(restored.spectra, result.spectra);
    assert_eq!(restored.config.anchors.len(), 2);
}

#[test]
fn iteration_limit_and_cancellation_are_distinct_from_convergence() {
    let data = collection();
    let config = McrConfig {
        initial_spectra: Some(DMatrix::from_fn(2, 41, |i, j| {
            data[3 + i].norm().unwrap()[j]
        })),
        max_iterations: 1,
        ..cfg()
    };
    let result = mcr_als(&data, &config).unwrap();
    assert_eq!(result.termination, McrTermination::IterationLimit);
    assert_eq!(result.iterations, 1);
    let mut calls = 0;
    let cancelled = mcr_als_with_progress(&data, &cfg(), |_, error| {
        calls += 1;
        assert!(error.is_finite());
        false
    })
    .unwrap();
    assert_eq!(calls, 1);
    assert_eq!(cancelled.termination, McrTermination::Cancelled);
    assert_eq!(cancelled.best_iteration, 1);
}

#[test]
fn invalid_arrays_settings_anchors_and_extrapolation_return_errors() {
    let data = collection();
    for config in [
        McrConfig {
            components: 0,
            ..cfg()
        },
        McrConfig {
            components: 3,
            ..cfg()
        },
        McrConfig {
            max_iterations: 0,
            ..cfg()
        },
        McrConfig {
            tolerance: f64::NAN,
            ..cfg()
        },
        McrConfig {
            range: Some((30., -10.)),
            ..cfg()
        },
        McrConfig {
            range: Some((-11., 30.)),
            ..cfg()
        },
        McrConfig {
            anchors: vec![McrAnchor {
                sample: 10,
                weights: vec![1., 0.],
            }],
            ..cfg()
        },
        McrConfig {
            anchors: vec![McrAnchor {
                sample: 0,
                weights: vec![0.2, 0.2],
            }],
            ..cfg()
        },
        McrConfig {
            anchors: vec![
                McrAnchor {
                    sample: 0,
                    weights: vec![1., 0.]
                };
                2
            ],
            ..cfg()
        },
        McrConfig {
            initial_spectra: Some(DMatrix::zeros(2, 41)),
            ..cfg()
        },
        McrConfig {
            initial_spectra: Some(DMatrix::zeros(2, 2)),
            ..cfg()
        },
    ] {
        assert!(mcr_als(&data, &config).is_err(), "{config:?}");
    }
    assert!(mcr_als::<XASSpectrum>(&[], &cfg()).is_err());
    let mut bad = data.clone();
    bad[3].energy.as_mut().unwrap()[5] = bad[3].energy.as_ref().unwrap()[4];
    assert!(mcr_als(&bad, &cfg())
        .unwrap_err()
        .to_string()
        .contains("sample 3"));
    let mut bad = data.clone();
    bad[1].energy.as_mut().unwrap()[0] = 8970.5;
    assert!(mcr_als(&bad, &cfg())
        .unwrap_err()
        .to_string()
        .contains("extrapolation"));
    let mut bad = data;
    bad[2].normalization = None;
    bad[2].mu = None;
    assert!(mcr_als(&bad, &cfg()).is_err());
}

#[test]
fn closure_can_be_disabled_and_initialization_is_repeatable() {
    let data = collection();
    let config = McrConfig {
        sum_to_one: false,
        seed: 3,
        ..cfg()
    };
    let first = mcr_als(&data, &config).unwrap();
    let second = mcr_als(&data, &config).unwrap();
    assert_eq!(first.initial_samples, second.initial_samples);
    assert_eq!(first.spectra, second.spectra);
    assert!(first.relative_error < 1e-20);
    assert!(first.concentrations.iter().all(|v| *v >= 0.0));
}
