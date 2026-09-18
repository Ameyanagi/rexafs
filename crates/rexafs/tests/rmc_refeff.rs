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
