#![cfg(feature = "refeff-runner")]
use rexafs::rmc::*;
use rexafs::structure::Edge;
fn geometry() -> Configuration {
    Configuration {
        cell: None,
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
    }
}
fn options() -> RefeffOptions {
    RefeffOptions {
        cluster_radius: 4.,
        path_radius: 4.,
        max_legs: 3,
        kmax: 12.,
        path_criteria: [0., 0.],
        ..Default::default()
    }
}
fn calculate(
    calc: &mut RefeffCalculator,
    c: &Configuration,
    o: Option<&RefeffOptions>,
    paths: bool,
) -> CalculatedSpectrum {
    calc.calculate_request(CalculationRequest {
        structure: 0,
        configuration: c,
        absorber: 0,
        edge: Edge::K,
        k: &[3., 4., 5., 6., 7., 8., 9.],
        options: o,
        paths,
    })
    .unwrap()
}
#[test]
fn exact_cache_skips_distant_moves_and_tracks_polarization_and_eviction() {
    let mut c = geometry();
    let mut calc = RefeffCalculator::new(options()).unwrap();
    let original = calculate(&mut calc, &c, None, false);
    c.atoms[3].position[0] += 0.2;
    let distant = calculate(&mut calc, &c, None, false);
    assert_eq!(original, distant);
    assert_eq!(calc.stats().full_calculations, 1);
    assert_eq!(calc.stats().spectrum_hits, 1);
    let resampled = calc.calculate(&c, 0, Edge::K, &[3.5, 5.5]).unwrap();
    assert_eq!(resampled.len(), 2);
    assert_eq!(calc.stats().full_calculations, 1);
    let polarized = RefeffOptions {
        polarization: Some([1., 0., 0.]),
        ..options()
    };
    let directional = calculate(&mut calc, &c, Some(&polarized), false);
    assert!(original
        .chi
        .iter()
        .zip(&directional.chi)
        .any(|(a, b)| (a - b).abs() > 1e-6));
    assert_eq!(calc.stats().full_calculations, 2);
    let repeat = calculate(&mut calc, &c, None, false);
    assert_eq!(repeat, original);
    calc.set_cache_capacity(0);
    let uncached = calculate(&mut calc, &c, None, false);
    assert_eq!(uncached, original);
    assert_eq!(calc.stats().spectrum_bytes, 0);
    calc.cancellation_token().cancel();
    assert!(calc.calculate(&c, 0, Edge::K, &[3., 4.]).is_err());
}
#[test]
fn pinned_potentials_recompute_paths_and_path_sum_matches_total() {
    let c = geometry();
    let mut calc = RefeffCalculator::new(options())
        .unwrap()
        .with_frozen_potentials(vec![c.clone()])
        .unwrap();
    let full = calculate(&mut calc, &c, None, false);
    let reported = calculate(&mut calc, &c, None, true);
    assert_eq!(full.chi, reported.chi);
    assert!(reported.paths.iter().any(|p| p.legs == 3));
    for i in 0..full.chi.len() {
        let sum = reported.paths.iter().map(|p| p.chi[i]).sum::<f64>();
        assert!(
            (sum - full.chi[i]).abs() < 1e-7,
            "path sum {sum} vs {}",
            full.chi[i]
        );
    }
    let mut changed = c.clone();
    changed.atoms[1].position[0] += 0.04;
    let moved = calculate(&mut calc, &changed, None, true);
    assert!(moved
        .chi
        .iter()
        .zip(&full.chi)
        .any(|(a, b)| (a - b).abs() > 1e-5));
    assert_eq!(calc.stats().full_calculations, 1);
    assert_eq!(calc.stats().path_calculations, 2);
    let mut restored = RefeffCalculator::new(options())
        .unwrap()
        .with_frozen_potentials(vec![c])
        .unwrap();
    let rebuilt = calculate(&mut restored, &changed, None, true);
    assert_eq!(moved, rebuilt);
    assert_eq!(calc.identity(), restored.identity());
}
#[test]
fn potential_indices_stay_stable_and_topology_change_is_rejected() {
    let mut c = geometry();
    c.atoms[1].atomic_number = 8;
    c.atoms[2].atomic_number = 26;
    let mut calc = RefeffCalculator::new(options())
        .unwrap()
        .with_frozen_potentials(vec![c.clone()])
        .unwrap();
    let a = calc.input_for(&c, 0, Edge::K).unwrap();
    c.atoms[1].position[0] = 2.6;
    let b = calc.input_for(&c, 0, Edge::K).unwrap();
    let potentials = |s: &str| {
        s.split("POTENTIALS\n")
            .nth(1)
            .unwrap()
            .split("ATOMS\n")
            .next()
            .unwrap()
            .to_owned()
    };
    assert_eq!(potentials(&a), potentials(&b));
    assert!(potentials(&a).contains("1 8 O\n2 26 Fe"));
    c.atoms[3].atomic_number = 1;
    assert!(calc.calculate(&c, 0, Edge::K, &[3., 4.]).is_err());
}
