#![cfg(feature = "refeff-runner")]
//! Real ReFEFF tests validate the adapter, not physical accuracy against experiment.
use rexafs::rmc::*;
use rexafs::structure::{Edge, Lattice, Site, Structure};

fn calculator() -> RefeffCalculator {
    RefeffCalculator::new(RefeffOptions {
        cluster_radius: 4.0,
        path_radius: 3.5,
        max_legs: 2,
        kmax: 12.0,
        ..Default::default()
    })
    .unwrap()
}

fn dimer() -> Configuration {
    Configuration {
        cell: None,
        atoms: vec![
            Atom {
                atomic_number: 29,
                position: [0.0; 3],
            },
            Atom {
                atomic_number: 29,
                position: [2.5, 0.0, 0.0],
            },
        ],
    }
}

#[test]
fn real_refeff_changes_with_geometry_and_returns_to_original() {
    let mut calc = calculator();
    let mut config = dimer();
    let k: Vec<_> = (0..21).map(|i| 3.0 + i as f64 * 0.3).collect();
    let first = calc.calculate(&config, 0, Edge::K, &k).unwrap();
    assert!(first.iter().all(|v| v.is_finite()));
    assert!(first.iter().any(|v| v.abs() > 1e-5));
    config.atoms[1].position[0] = 2.7;
    let changed = calc.calculate(&config, 0, Edge::K, &k).unwrap();
    assert!(first
        .iter()
        .zip(&changed)
        .any(|(a, b)| (a - b).abs() > 1e-4));
    config.atoms[1].position[0] = 2.5;
    let restored = calc.calculate(&config, 0, Edge::K, &k).unwrap();
    assert!(first
        .iter()
        .zip(restored)
        .all(|(a, b)| (a - b).abs() < 1e-12));
}

#[test]
fn full_and_prepared_refeff_calculate_above_fifteen_inverse_angstroms() {
    let options = RefeffOptions {
        cluster_radius: 4.,
        path_radius: 3.,
        max_legs: 2,
        path_criteria: [0., 0.],
        kmax: 22.,
        ..Default::default()
    };
    let configuration = dimer();
    let k = [15., 16., 18., 20.];
    let full = RefeffCalculator::new(options.clone())
        .unwrap()
        .calculate(&configuration, 0, Edge::K, &k)
        .unwrap();
    let mut prepared = PreparedRefeffCalculator::new(
        options,
        vec![configuration.clone()],
        AccelerationSettings {
            catalogue: PathCatalogueSettings {
                radius: 3.,
                max_legs: 2,
                ..Default::default()
            },
            max_contexts: 1,
            ..Default::default()
        },
    )
    .unwrap();
    let result = prepared
        .calculate_request(CalculationRequest {
            structure: 0,
            configuration: &configuration,
            absorber: 0,
            edge: Edge::K,
            k: &k,
            options: None,
            paths: false,
        })
        .unwrap();
    assert_eq!(result.chi.len(), k.len());
    assert!(result.chi.iter().all(|v| v.is_finite()));
    assert!(result.chi.iter().any(|v| v.abs() > 1e-8));
    for (actual, expected) in result.chi.iter().zip(full) {
        assert!(
            (actual - expected).abs() < 1e-5,
            "typed {actual}, full {expected}"
        );
    }
    let error = prepared
        .calculate_request(CalculationRequest {
            structure: 0,
            configuration: &configuration,
            absorber: 1,
            edge: Edge::K,
            k: &k,
            options: None,
            paths: false,
        })
        .unwrap_err()
        .to_string();
    assert!(error.contains("1 absorber contexts"));
    assert!(error.contains("AccelerationSettings.max_contexts"));
}

#[test]
fn electronic_reuse_preserves_distinct_sites_paths_and_changed_geometries() {
    // Two translated Cu-O-O clusters have identical complete local inputs;
    // a third has a genuinely different bond length and must prepare separately.
    let mut c = Configuration {
        atoms: Vec::new(),
        cell: None,
    };
    for i in 0..3 {
        let x = i as f64 * 20.;
        c.atoms.extend([
            Atom {
                atomic_number: 29,
                position: [x, 0., 0.],
            },
            Atom {
                atomic_number: 8,
                position: [x + 1.8 + if i == 2 { 0.1 } else { 0. }, 0., 0.],
            },
            Atom {
                atomic_number: 8,
                position: [x, 1.8, 0.],
            },
        ]);
    }
    let options = RefeffOptions {
        cluster_radius: 3.,
        path_radius: 3.,
        max_legs: 3,
        path_criteria: [0., 0.],
        kmax: 14.,
        ..Default::default()
    };
    let settings = |reuse| AccelerationSettings {
        catalogue: PathCatalogueSettings {
            radius: 3.,
            max_legs: 3,
            ..Default::default()
        },
        reuse_electronic_inputs: reuse,
        ..Default::default()
    };
    let k = [3., 5., 8., 12.];
    let request = |absorber| CalculationRequest {
        structure: 0,
        configuration: &c,
        absorber,
        edge: Edge::K,
        k: &k,
        options: None,
        paths: true,
    };
    let requests: Vec<_> = [0, 3, 6].into_iter().map(request).collect();
    let mut legacy =
        PreparedRefeffCalculator::new(options.clone(), vec![c.clone()], settings(false)).unwrap();
    let mut shared =
        PreparedRefeffCalculator::new(options.clone(), vec![c.clone()], settings(true)).unwrap();
    assert_ne!(legacy.identity(), shared.identity());
    let before = legacy.calculate_batch(&requests).unwrap();
    let after = shared.calculate_batch(&requests).unwrap();
    for (a, b) in after.iter().zip(&before) {
        assert_eq!(a.paths.len(), b.paths.len());
        for (x, y) in a.chi.iter().zip(&b.chi) {
            assert!((x - y).abs() < 1e-10, "{x} != {y}");
        }
    }
    assert_eq!(shared.stats().contexts, 3);
    assert_eq!(shared.stats().electronic_preparations, 2);
    assert_eq!(shared.stats().shared_electronic_contexts, 1);
    assert_eq!(
        shared.stats().catalogue_paths,
        legacy.stats().catalogue_paths
    );
    // A cold calculator visiting the equivalent sites in reverse order gives
    // identical results; cache history does not select a different potential.
    let mut cold =
        PreparedRefeffCalculator::new(options.clone(), vec![c.clone()], settings(true)).unwrap();
    for i in (0..requests.len()).rev() {
        assert_eq!(cold.calculate_request(requests[i]).unwrap(), after[i]);
    }
    let mut changed = c.clone();
    changed.atoms[4].position[0] += 0.025;
    let moved = CalculationRequest {
        configuration: &changed,
        ..request(3)
    };
    let a = shared.calculate_request(moved).unwrap();
    let b = legacy.calculate_request(moved).unwrap();
    assert!(a
        .chi
        .iter()
        .zip(&after[1].chi)
        .any(|(x, y)| (x - y).abs() > 1e-6));
    for (x, y) in a.chi.iter().zip(&b.chi) {
        assert!((x - y).abs() < 1e-10);
    }
    // Full options are in the cache key, including polarization.
    let polarized = RefeffOptions {
        polarization: Some([1., 0., 0.]),
        ..options
    };
    shared
        .calculate_request(CalculationRequest {
            options: Some(&polarized),
            ..request(0)
        })
        .unwrap();
    assert_eq!(shared.stats().electronic_preparations, 3);
    shared.cancellation_token().cancel();
    assert!(shared.calculate_request(request(3)).is_err());
}

#[test]
fn multiple_scattering_changes_the_triangle_spectrum() {
    let mut config = dimer();
    config.atoms.push(Atom {
        atomic_number: 29,
        position: [1.25, 2.165063509461, 0.0],
    });
    let options = RefeffOptions {
        cluster_radius: 4.0,
        path_radius: 4.0, // Includes the triangle's 3.75 Å half-path length.
        max_legs: 2,
        // The default plane-wave screening removes this triangle's weak path.
        // Retain it explicitly to test multiple-scattering calculation itself.
        path_criteria: [0.0, 0.0],
        kmax: 12.0,
        ..Default::default()
    };
    let k = [3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0];
    let single = RefeffCalculator::new(options.clone())
        .unwrap()
        .calculate(&config, 0, Edge::K, &k)
        .unwrap();
    let multiple = RefeffCalculator::new(RefeffOptions {
        max_legs: 4,
        ..options
    })
    .unwrap()
    .calculate(&config, 0, Edge::K, &k)
    .unwrap();
    assert!(single
        .iter()
        .zip(multiple)
        .any(|(a, b)| (a - b).abs() > 1e-6));
}

#[test]
fn input_preserves_displacements_and_builds_periodic_images() {
    let calc = calculator();
    let mut config = dimer();
    config.atoms[1].position[1] = 0.00000012;
    let input = calc.input_for(&config, 0, Edge::K).unwrap();
    assert!(input.contains("0.000000120000"));
    assert!(input.contains("CONTROL 1 1 1 1 1 1"));
    assert!(!input.contains("DEBYE"));
    assert!(!input.contains("SIG2"));
    let structure = Structure::new(
        "simple cubic test",
        Lattice::cubic(3.0).unwrap(),
        vec![Site::new("Cu", "Cu", [0.0; 3])],
    );
    let config = Configuration::from_structure(&structure, [1, 1, 1]).unwrap();
    let periodic = calc.input_for(&config, 0, Edge::K).unwrap();
    assert_eq!(periodic.split("ATOMS\n").nth(1).unwrap().lines().count(), 8); // absorber + 6 neighbors + END
    assert!(periodic.contains("-3.000000000000"));
}

#[test]
fn cancellation_and_invalid_options_are_errors() {
    assert!(RefeffCalculator::new(RefeffOptions {
        path_criteria: [-1.0, 0.0],
        ..Default::default()
    })
    .is_err());
    assert!(RefeffCalculator::new(RefeffOptions {
        threads: 0,
        ..Default::default()
    })
    .is_err());
    assert!(RefeffCalculator::new(RefeffOptions {
        polarization: Some([0.0; 3]),
        ..Default::default()
    })
    .is_err());
    let mut calc = calculator();
    calc.cancellation_token().cancel();
    assert!(matches!(
        calc.calculate(&dimer(), 0, Edge::K, &[3.0, 4.0]),
        Err(RmcError::Calculator(_))
    ));
}
