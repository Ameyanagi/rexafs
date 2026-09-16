use rexafs::rmc::*;

fn pair() -> Configuration {
    Configuration {
        atoms: vec![
            Atom {
                atomic_number: 29,
                position: [0.; 3],
            },
            Atom {
                atomic_number: 8,
                position: [2.2, 0., 0.],
            },
        ],
        cell: None,
    }
}

#[test]
fn catalogue_includes_paths_entering_radius_and_checks_envelope() {
    let mut c = pair();
    let catalogue = PathCatalogue::new(
        c.clone(),
        0,
        PathCatalogueSettings {
            radius: 2.,
            max_legs: 2,
            displacement: 0.25,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(catalogue.paths().len(), 1);
    c.atoms[1].position[0] = 1.99;
    catalogue.validate(&c).unwrap();
    assert!(catalogue.paths()[0].half_length(&c).unwrap() < 2.);
    assert_eq!(catalogue.affected_paths(&[1]).unwrap(), vec![0]);
    c.atoms[1].position[0] = 1.9;
    assert!(catalogue.validate(&c).is_err());
    assert!(catalogue.affected_paths(&[2]).is_err());
}

#[test]
fn catalogue_preserves_periodic_self_images_and_directed_multiplicity() {
    let c = Configuration {
        atoms: vec![Atom {
            atomic_number: 29,
            position: [7.1, 0., 0.],
        }],
        cell: Some([[2.5, 0., 0.], [0., 2.5, 0.], [0., 0., 2.5]]),
    };
    let catalogue = PathCatalogue::new(
        c.clone(),
        0,
        PathCatalogueSettings {
            radius: 2.6,
            max_legs: 2,
            displacement: 0.,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(catalogue.paths().len(), 6);
    for path in catalogue.paths() {
        assert!((path.half_length(&c).unwrap() - 2.5).abs() < 1e-12);
    }
    assert_eq!(catalogue.affected_paths(&[0]).unwrap().len(), 6);
    let triangle = Configuration {
        atoms: vec![
            Atom {
                atomic_number: 29,
                position: [0.; 3],
            },
            Atom {
                atomic_number: 29,
                position: [2., 0., 0.],
            },
            Atom {
                atomic_number: 29,
                position: [1., 1.7320508075688772, 0.],
            },
        ],
        cell: None,
    };
    let catalogue = PathCatalogue::new(
        triangle,
        0,
        PathCatalogueSettings {
            radius: 3.1,
            max_legs: 3,
            displacement: 0.,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(
        catalogue
            .paths()
            .iter()
            .filter(|p| p.scatterers.len() == 2)
            .count(),
        2
    );
}

#[test]
fn four_leg_paths_include_intermediate_central_scattering() {
    let c = pair();
    let catalogue = PathCatalogue::new(
        c,
        0,
        PathCatalogueSettings {
            radius: 4.5,
            max_legs: 4,
            displacement: 0.,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(catalogue.paths().iter().any(|p| p.scatterers
        == vec![
            AtomImage {
                atom: 1,
                image: [0; 3]
            },
            AtomImage {
                atom: 0,
                image: [0; 3]
            },
            AtomImage {
                atom: 1,
                image: [0; 3]
            }
        ]));
}

#[test]
fn structural_reports_and_priors_use_arithmetic_msd_and_explicit_images() {
    let reference = pair();
    let mut current = reference.clone();
    current.atoms[0].position[0] += 0.1;
    current.atoms[1].position[0] += 0.3;
    let report = displacement_report(&reference, &current, &[0, 1], false).unwrap();
    assert!((report.mean_square - 0.05).abs() < 1e-12);
    let report = displacement_report(&reference, &current, &[0, 1], true).unwrap();
    assert!((report.mean_square - 0.01).abs() < 1e-12);
    let path = ScatteringPath {
        absorber: 0,
        scatterers: vec![AtomImage {
            atom: 1,
            image: [0; 3],
        }],
    };
    let report = path_geometry_report(&current, &path).unwrap();
    assert!((report.half_length - 2.4).abs() < 1e-12);
    assert_eq!(report.angles, vec![0., 0.]);
    let histogram =
        path_length_distribution(&current, std::slice::from_ref(&path), &[0., 2.3]).unwrap();
    assert_eq!(histogram.overflow, 1);
    let restraint = Constraints {
        msd: vec![MsdRestraint {
            structure: 0,
            reference,
            atoms: vec![0, 1],
            remove_translation: true,
            target: 0.,
            strength: 100.,
        }],
        path_distributions: vec![PathDistributionRestraint {
            structure: 0,
            paths: vec![path],
            edges: vec![0., 2.3],
            fractions: vec![1.],
            strength: 2.,
        }],
        ..Default::default()
    };
    let penalty = restraint
        .penalty(&[WeightedStructure {
            configuration: current,
            weight: 0.,
            movable_atoms: None,
        }])
        .unwrap();
    assert!((penalty - 4.01).abs() < 1e-12);
    let cell = Configuration {
        atoms: vec![Atom {
            atomic_number: 29,
            position: [0.; 3],
        }],
        cell: Some([[2., 0., 0.], [0., 2., 0.], [0., 0., 2.]]),
    };
    let angles = angle_distribution(&cell, &[0], None, 2.1, &[0., 100., 181.]).unwrap();
    assert_eq!(angles.counts, vec![12, 3]);
}

#[cfg(feature = "refeff-runner")]
#[test]
fn typed_refeff_single_path_matches_pipeline_for_disorder_and_polarization() {
    use rexafs::structure::Edge;
    let mut c = pair();
    let options = RefeffOptions {
        cluster_radius: 4.,
        path_radius: 3.,
        max_legs: 2,
        path_criteria: [0., 0.],
        kmax: 12.,
        polarization: Some([1., 0.2, 0.1]),
        ..Default::default()
    };
    let context = PreparedRefeffContext::prepare(c.clone(), 0, Edge::K, options.clone()).unwrap();
    let mut pipeline = RefeffCalculator::new(options)
        .unwrap()
        .with_frozen_potentials(vec![c.clone()])
        .unwrap();
    let path = ScatteringPath {
        absorber: 0,
        scatterers: vec![AtomImage {
            atom: 1,
            image: [0; 3],
        }],
    };
    let k: Vec<_> = (0..30).map(|i| 3.13 + i as f64 * 0.17).collect();
    for dx in [0., 0.037] {
        c.atoms[1].position[0] += dx;
        let actual = context.scattering(&path, &c).unwrap().sample(&k).unwrap();
        let reference = pipeline.calculate(&c, 0, Edge::K, &k).unwrap();
        let error = actual
            .iter()
            .zip(&reference)
            .map(|(a, b)| (a - b).powi(2))
            .sum::<f64>()
            .sqrt();
        let norm = reference.iter().map(|v| v * v).sum::<f64>().sqrt();
        assert!(
            error / norm < 5e-5,
            "typed path relative error {}",
            error / norm
        );
    }
}

#[cfg(feature = "refeff-runner")]
#[test]
fn prepared_multiple_scattering_matches_pipeline_and_rejected_trial_cache_is_safe() {
    use rexafs::structure::Edge;
    let c = Configuration {
        atoms: vec![
            Atom {
                atomic_number: 29,
                position: [0.; 3],
            },
            Atom {
                atomic_number: 29,
                position: [2.5, 0., 0.],
            },
            Atom {
                atomic_number: 29,
                position: [1.25, 2.165063509461, 0.],
            },
            Atom {
                atomic_number: 29,
                position: [20., 0., 0.],
            },
        ],
        cell: None,
    };
    let options = RefeffOptions {
        cluster_radius: 4.,
        path_radius: 4.,
        max_legs: 3,
        path_criteria: [0., 0.],
        kmax: 12.,
        ..Default::default()
    };
    let settings = AccelerationSettings {
        catalogue: PathCatalogueSettings {
            radius: 4.,
            max_legs: 3,
            displacement: 0.15,
            ..Default::default()
        },
        workers: 2,
        snapshots_per_context: 3,
        ..Default::default()
    };
    let mut prepared =
        PreparedRefeffCalculator::new(options.clone(), vec![c.clone()], settings.clone()).unwrap();
    let mut pipeline = RefeffCalculator::new(options.clone())
        .unwrap()
        .with_frozen_potentials(vec![c.clone()])
        .unwrap();
    let k: Vec<_> = (0..40).map(|i| 3.1 + i as f64 * 0.13).collect();
    let initial = prepared.calculate(&c, 0, Edge::K, &k).unwrap();
    let reference = pipeline.calculate(&c, 0, Edge::K, &k).unwrap();
    let err = initial
        .iter()
        .zip(&reference)
        .map(|(a, b)| (a - b).powi(2))
        .sum::<f64>();
    let norm = reference.iter().map(|v| v * v).sum::<f64>();
    assert!(
        (err / norm).sqrt() < 5e-5,
        "multiple-scattering error {}",
        (err / norm).sqrt()
    );
    let mut trial = c.clone();
    trial.atoms[1].position[1] += 0.027;
    prepared.calculate(&trial, 0, Edge::K, &k).unwrap();
    let before_parent = prepared.stats().exact_paths;
    // A rejected trial can be the last cached geometry; returning to accepted
    // geometry must reproduce a cold calculation exactly.
    assert_eq!(prepared.calculate(&c, 0, Edge::K, &k).unwrap(), initial);
    assert_eq!(
        prepared.stats().exact_paths,
        before_parent,
        "retained parent avoids rescattering"
    );
    let stats = prepared.stats();
    assert_eq!(
        stats.visited_paths,
        stats.exact_paths + stats.basis_paths + stats.reused_paths + stats.outside_radius_paths
    );
    assert!(stats.reused_active_paths > 0);
    assert_eq!(stats.identical_geometry_hits, 1);
    assert_eq!(stats.cached_snapshots, 2);
    assert!(stats.cached_bytes <= settings.cache_bytes);
    let calls = prepared.stats().exact_paths;
    trial = c.clone();
    trial.atoms[3].position[0] += 0.05;
    assert_eq!(prepared.calculate(&trial, 0, Edge::K, &k).unwrap(), initial);
    assert_eq!(prepared.stats().exact_paths, calls);
    let mut fast = PreparedRefeffCalculator::new(
        options,
        vec![c.clone()],
        AccelerationSettings {
            basis: ScatteringBasis::Frozen {
                max_leg_change: 0.01,
                max_angle_change: 0.01,
            },
            ..settings
        },
    )
    .unwrap();
    let request = CalculationRequest {
        structure: 0,
        configuration: &c,
        absorber: 0,
        edge: Edge::K,
        k: &k,
        options: None,
        paths: true,
    };
    let report = fast.compare_reference(request).unwrap();
    assert!(report.relative_l2.unwrap() < 1e-10);
    assert!(fast.stats().representatives < fast.stats().catalogue_paths);
    trial = c.clone();
    trial.atoms[1].position[1] += 0.08;
    let request = CalculationRequest {
        configuration: &trial,
        ..request
    };
    let cached = fast.calculate_request(request).unwrap();
    let count = fast.stats().exact_paths;
    assert!(count > 0, "geometric guard must fall back to exact paths");
    fast.clear_cache();
    assert_eq!(fast.calculate_request(request).unwrap(), cached);
    let results = fast.calculate_batch(&[request, request]).unwrap();
    assert_eq!(results, vec![cached.clone(), cached]);
    let reports = fast.path_reports(request, 2, true).unwrap();
    assert_eq!(reports.len(), results[0].paths.len());
    assert!(reports.iter().any(|r| !r.used_basis));
    assert!(reports
        .iter()
        .all(|r| r.accuracy.is_some() && r.reference_family.is_some()));
    for report in reports {
        assert_eq!(
            report.geometry.path,
            fast.catalogue(request).unwrap().paths()[report.contribution.index - 1]
        );
        assert!(report.relative_importance.unwrap() >= 0.);
    }
    fast.cancellation_token().cancel();
    assert!(
        fast.calculate_request(request).is_err(),
        "cancellation applies to cache hits"
    );
}

#[cfg(feature = "refeff-runner")]
#[test]
fn moment_mode_and_path_reporting_preserve_a_history_independent_model() {
    use rexafs::structure::Edge;
    let c = Configuration {
        atoms: vec![Atom {
            atomic_number: 29,
            position: [0.; 3],
        }],
        cell: Some([[2.5, 0., 0.], [0., 2.5, 0.], [0., 0., 2.5]]),
    };
    let options = RefeffOptions {
        cluster_radius: 4.,
        path_radius: 3.6,
        max_legs: 2,
        path_criteria: [0., 0.],
        kmax: 12.,
        ..Default::default()
    };
    let settings = AccelerationSettings {
        catalogue: PathCatalogueSettings {
            radius: 3.6,
            max_legs: 2,
            displacement: 0.1,
            ..Default::default()
        },
        basis: ScatteringBasis::Frozen {
            max_leg_change: 0.1,
            max_angle_change: 0.1,
        },
        moments: Some(MomentSettings::default()),
        ..Default::default()
    };
    let mut calc = PreparedRefeffCalculator::new(options, vec![c.clone()], settings).unwrap();
    let k = [3., 4., 5., 6., 7., 8., 9.];
    let request = CalculationRequest {
        structure: 0,
        configuration: &c,
        absorber: 0,
        edge: Edge::K,
        k: &k,
        options: None,
        paths: false,
    };
    let moments = calc.calculate_request(request).unwrap();
    let direct = calc
        .calculate_request(CalculationRequest {
            paths: true,
            ..request
        })
        .unwrap();
    assert!(!direct.paths.is_empty());
    for (a, b) in moments.chi.iter().zip(&direct.chi) {
        assert!((a - b).abs() < 1e-10);
    }
    // Retaining direct path χ in a cache must not switch the summation model.
    assert_eq!(calc.calculate_request(request).unwrap(), moments);
    calc.clear_cache();
    assert_eq!(calc.calculate_request(request).unwrap(), moments);
}

#[cfg(feature = "refeff-runner")]
#[test]
fn adaptive_basis_splits_on_measured_error_and_falls_back_outside_training_support() {
    use rexafs::structure::Edge;
    let reference = Configuration {
        atoms: vec![
            Atom {
                atomic_number: 29,
                position: [0.; 3],
            },
            Atom {
                atomic_number: 29,
                position: [2.5, 0., 0.],
            },
        ],
        cell: None,
    };
    let mut moved = reference.clone();
    moved.atoms[1].position[0] += 0.04;
    let k: Vec<_> = (0..35).map(|i| 3. + i as f64 * 0.15).collect();
    let options = RefeffOptions {
        cluster_radius: 3.5,
        path_radius: 3.,
        max_legs: 2,
        path_criteria: [0., 0.],
        kmax: 12.,
        ..Default::default()
    };
    let settings = AccelerationSettings {
        catalogue: PathCatalogueSettings {
            radius: 3.,
            max_legs: 2,
            displacement: 0.3,
            ..Default::default()
        },
        basis: ScatteringBasis::Frozen {
            max_leg_change: 0.1,
            max_angle_change: 0.1,
        },
        adaptive: Some(AdaptiveBasisSettings {
            training: vec![vec![moved.clone()]],
            k: k.clone(),
            geometry_radius: 0.08,
            relative_error: 1e-8,
            ..Default::default()
        }),
        snapshots_per_context: 3,
        ..Default::default()
    };
    let mut calc =
        PreparedRefeffCalculator::new(options.clone(), vec![reference.clone()], settings.clone())
            .unwrap();
    let request = CalculationRequest {
        configuration: &moved,
        structure: 0,
        absorber: 0,
        edge: Edge::K,
        k: &k,
        options: None,
        paths: false,
    };
    let report = calc.compare_reference(request).unwrap();
    assert!(report.relative_l2.unwrap() < 1e-8);
    let evidence = calc.adaptive_reports();
    assert_eq!(evidence.len(), 1);
    assert!(
        evidence[0].splits > 0,
        "amplitude changes need a new representative"
    );
    assert_eq!(evidence[0].training_paths, 2);
    assert!(evidence[0]
        .relative_errors
        .iter()
        .all(|e| e.unwrap() < 1e-8));
    let saved = calc.calculate_request(request).unwrap();
    calc.clear_cache();
    assert_eq!(saved, calc.calculate_request(request).unwrap());
    let mut far = reference.clone();
    far.atoms[1].position[0] += 0.2;
    let before = calc.stats().exact_paths;
    let outside = calc
        .compare_reference(CalculationRequest {
            configuration: &far,
            ..request
        })
        .unwrap();
    assert_eq!(outside.model, outside.reference);
    assert!(calc.stats().exact_paths > before);
    let other_k: Vec<_> = k.iter().map(|v| v + 0.001).collect();
    let different_grid = calc
        .compare_reference(CalculationRequest {
            k: &other_k,
            ..request
        })
        .unwrap();
    assert_eq!(different_grid.model, different_grid.reference);
    let mut cold = PreparedRefeffCalculator::new(
        options,
        vec![reference],
        serde_json::from_value(serde_json::to_value(&settings).unwrap()).unwrap(),
    )
    .unwrap();
    assert_eq!(cold.identity(), calc.identity());
    assert_eq!(cold.calculate_request(request).unwrap(), saved);
    let mut stage = settings.adaptive.unwrap();
    assert!(calc.refreshed_basis(stage.clone()).is_err());
    stage.epoch += 1;
    let refreshed = calc.refreshed_basis(stage).unwrap();
    assert_ne!(refreshed.identity(), calc.identity());
}
