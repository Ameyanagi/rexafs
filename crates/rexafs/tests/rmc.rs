//! Algorithm tests use an analytic toy spectrum, not a physical EXAFS reference.
//! ReFEFF integration is exercised separately in rmc_refeff.rs.
use rexafs::rmc::*;
use rexafs::structure::{Edge, Lattice, Site, Structure};
use std::ops::ControlFlow;

#[derive(Default)]
struct Dimer {
    calls: usize,
}

impl ExafsCalculator for Dimer {
    fn name(&self) -> &str {
        "analytic test dimer"
    }
    fn calculate(
        &mut self,
        c: &Configuration,
        a: usize,
        _: Edge,
        k: &[f64],
    ) -> Result<Vec<f64>, RmcError> {
        self.calls += 1;
        let b = 1 - a;
        let r = (0..3)
            .map(|i| (c.atoms[a].position[i] - c.atoms[b].position[i]).powi(2))
            .sum::<f64>()
            .sqrt();
        Ok(k.iter().map(|k| (2.0 * k * r).sin() / r.powi(2)).collect())
    }
}

fn problem(r: f64) -> RmcProblem {
    let k: Vec<_> = (0..81).map(|i| 3.0 + i as f64 * 0.1).collect();
    let target: Vec<_> = k
        .iter()
        .map(|k| (2.0 * k * 2.5_f64).sin() / 2.5_f64.powi(2))
        .collect();
    RmcProblem {
        configuration: Configuration {
            cell: None,
            atoms: vec![
                Atom {
                    atomic_number: 29,
                    position: [0.0; 3],
                },
                Atom {
                    atomic_number: 29,
                    position: [r, 0.0, 0.0],
                },
            ],
        },
        datasets: vec![ExafsDataset {
            name: "toy".into(),
            absorbers: vec![0],
            edge: Edge::K,
            k,
            chi: target,
            sigma: vec![0.01; 81],
            weight: 1.0,
            kweight: 0,
            s02: 1.0,
            delta_e0: 0.0,
        }],
    }
}

#[test]
fn recovers_distance_and_repeats_exactly_without_mutating_input() {
    let p = problem(2.7);
    let original = p.clone();
    let settings = RmcSettings {
        steps: 600,
        temperature: 0.0,
        seed: 42,
        movable_atoms: vec![1],
        ..Default::default()
    };
    let result = refine(&p, &settings, &mut Dimer::default()).unwrap();
    assert_eq!(p, original);
    assert_eq!(
        result,
        refine(&p, &settings, &mut Dimer::default()).unwrap()
    );
    assert!(result.best.evaluation.score < result.initial.evaluation.score * 1e-5);
    let pos = result.best.configuration.atoms[1].position;
    let distance = pos.iter().map(|v| v * v).sum::<f64>().sqrt();
    assert!((distance - 2.5).abs() < 0.001, "distance={distance}");
    assert_eq!(result.best.configuration.atoms[0], p.configuration.atoms[0]);
    assert!(result.history.iter().all(|s| s.best_score <= s.score));
    assert_eq!(
        result.best.evaluation,
        evaluate(
            &RmcProblem {
                configuration: result.best.configuration.clone(),
                datasets: p.datasets.clone(),
            },
            &mut Dimer::default()
        )
        .unwrap()
    );
    let serialized = serde_json::to_string(&result).unwrap();
    assert_eq!(
        result,
        serde_json::from_str::<RmcResult>(&serialized).unwrap()
    );
}

#[test]
fn rejected_moves_restore_coordinates_and_spectra() {
    let p = problem(2.5);
    let settings = RmcSettings {
        steps: 40,
        temperature: 0.0,
        movable_atoms: vec![1],
        ..Default::default()
    };
    let result = refine(&p, &settings, &mut Dimer::default()).unwrap();
    assert!(result
        .history
        .iter()
        .all(|s| !s.accepted && !s.constraint_rejected));
    assert_eq!(result.initial, result.final_state);
    assert_eq!(result.initial, result.best);
}

#[test]
fn hard_constraints_skip_calculator_and_callback_can_stop() {
    let mut calculator = Dimer::default();
    let settings = RmcSettings {
        steps: 50,
        max_displacement: Some(1e-12),
        ..Default::default()
    };
    let result = refine_with_progress(&problem(2.7), &settings, &mut calculator, |step| {
        if step.step == 7 {
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
    })
    .unwrap();
    assert_eq!(calculator.calls, 1);
    assert!(result.stopped);
    assert_eq!(result.history.len(), 7);
    assert!(result
        .history
        .iter()
        .all(|s| s.constraint_rejected && s.trial_score.is_none()));
    assert_eq!(result.initial, result.final_state);
}

#[test]
fn positive_tolerance_accepts_uphill_while_preserving_best() {
    let settings = RmcSettings {
        steps: 5,
        temperature: 1e100,
        movable_atoms: vec![1],
        ..Default::default()
    };
    let result = refine(&problem(2.5), &settings, &mut Dimer::default()).unwrap();
    assert!(result
        .history
        .iter()
        .any(|s| s.accepted && s.score > result.initial.evaluation.score));
    assert_eq!(result.best, result.initial);
    assert_ne!(result.final_state, result.best);
}

struct Grid;
impl ExafsCalculator for Grid {
    fn name(&self) -> &str {
        "grid and absorber test"
    }
    fn calculate(
        &mut self,
        _: &Configuration,
        a: usize,
        _: Edge,
        k: &[f64],
    ) -> Result<Vec<f64>, RmcError> {
        Ok(k.iter().map(|v| v + 2.0 * a as f64).collect())
    }
}

#[test]
fn averages_before_residual_and_applies_energy_amplitude_and_joint_weights() {
    let mut p = problem(2.5);
    let d = &mut p.datasets[0];
    d.k = vec![2.0, 3.0];
    d.chi = vec![0.0, 0.0];
    d.sigma = vec![2.0, 2.0];
    d.absorbers = vec![0, 1];
    d.s02 = 0.8;
    d.kweight = 1;
    d.weight = 3.0;
    d.delta_e0 = 2.0;
    let second = ExafsDataset {
        name: "second edge".into(),
        weight: 2.0,
        edge: Edge::L3,
        ..d.clone()
    };
    p.datasets.push(second);
    let result = evaluate(&p, &mut Grid).unwrap();
    let etok = rexafs::xafs::xafsutils::constants::ETOK;
    let expected: Vec<_> = [2.0_f64, 3.0]
        .iter()
        .map(|k| 0.8 * ((k * k - etok * 2.0).sqrt() + 1.0))
        .collect();
    assert_eq!(result.datasets[0].chi, expected);
    let unweighted_score = expected
        .iter()
        .zip([2.0_f64, 3.0])
        .map(|(c, k)| (c * k / 2.0).powi(2))
        .sum::<f64>()
        / 2.0;
    assert!((result.score - 5.0 * unweighted_score).abs() < 1e-12);
}

#[test]
fn invalid_inputs_fail_before_calculation() {
    let settings = RmcSettings::default();
    let mut variants = Vec::new();
    let p = problem(2.5);
    let mut bad = p.clone();
    bad.datasets[0].sigma[0] = 0.0;
    variants.push(bad);
    let mut bad = p.clone();
    bad.datasets[0].k[1] = bad.datasets[0].k[0];
    variants.push(bad);
    let mut bad = p.clone();
    bad.datasets[0].chi[0] = f64::NAN;
    variants.push(bad);
    let mut bad = p.clone();
    bad.datasets[0].absorbers = vec![0, 0];
    variants.push(bad);
    let mut bad = p.clone();
    bad.datasets[0].absorbers = vec![2];
    variants.push(bad);
    let mut bad = p.clone();
    bad.datasets[0].absorbers = vec![0, 1];
    bad.configuration.atoms[1].atomic_number = 8;
    variants.push(bad);
    let mut bad = p.clone();
    bad.datasets[0].delta_e0 = 10000.0;
    variants.push(bad);
    let mut bad = p.clone();
    bad.configuration.atoms[1].position = [f64::INFINITY, 0.0, 0.0];
    variants.push(bad);
    let mut bad = p.clone();
    bad.configuration.cell = Some([[0.0; 3]; 3]);
    variants.push(bad);
    let mut bad = p.clone();
    bad.configuration.atoms[1].position = [0.0; 3];
    variants.push(bad);
    for bad in variants {
        let mut calculator = Dimer::default();
        assert!(refine(&bad, &settings, &mut calculator).is_err());
        assert_eq!(calculator.calls, 0);
    }
    for bad in [
        RmcSettings {
            temperature: -1.0,
            ..settings.clone()
        },
        RmcSettings {
            movable_atoms: vec![1, 1],
            ..settings.clone()
        },
        RmcSettings {
            step_size: f64::NAN,
            ..settings
        },
    ] {
        assert!(refine(&p, &bad, &mut Dimer::default()).is_err());
    }
    let mut json = serde_json::to_value(RmcSettings::default()).unwrap();
    json["temprature"] = 0.into();
    assert!(serde_json::from_value::<RmcSettings>(json).is_err());
}

#[test]
fn periodic_constraints_detect_boundary_skew_and_self_image_contacts() {
    let settings = RmcSettings {
        steps: 0,
        min_distance: 0.8,
        ..Default::default()
    };
    let mut p = problem(2.5);
    p.configuration.cell = Some([[10.0, 0.0, 0.0], [0.0, 10.0, 0.0], [0.0, 0.0, 10.0]]);
    p.configuration.atoms[1].position = [9.8, 0.0, 0.0];
    assert!(refine(&p, &settings, &mut Grid).is_err());
    p.configuration.cell = Some([[10.0, 0.0, 0.0], [9.9, 1.0, 0.0], [0.0, 0.0, 10.0]]);
    p.configuration.atoms[1].position = [9.751, 0.49, 0.0];
    assert!(refine(&p, &settings, &mut Grid).is_err());
    // One atom still interacts with its own periodic images. The shortest
    // translation here is b-a, shorter than any individual cell vector.
    p.configuration.atoms.truncate(1);
    p.configuration.cell = Some([[10.0, 0.0, 0.0], [9.9, 0.3, 0.0], [0.0, 0.0, 10.0]]);
    assert!(refine(&p, &settings, &mut Grid).is_err());
    p.configuration.cell = Some([[10.0, 0.0, 0.0], [0.0, 10.0, 0.0], [0.0, 0.0, 10.0]]);
    let result = refine(&p, &settings, &mut Grid).unwrap();
    assert!(result.history.is_empty());
    assert!(!result.stopped);
}

#[test]
fn expands_supercell_and_rejects_implicit_disorder() {
    let mut structure = Structure::new(
        "Cu",
        Lattice::cubic(3.6).unwrap(),
        vec![Site::new("Cu1", "Cu", [0.0; 3])],
    );
    let config = Configuration::from_structure(&structure, [2, 1, 1]).unwrap();
    assert_eq!(config.atoms.len(), 2);
    assert_eq!(config.atoms[1].position, [3.6, 0.0, 0.0]);
    assert_eq!(config.cell.unwrap()[0], [7.2, 0.0, 0.0]);
    structure.sites[0].species[0].occupancy = 0.5;
    assert!(Configuration::from_structure(&structure, [1, 1, 1]).is_err());
}

struct InvalidCalculator;
impl ExafsCalculator for InvalidCalculator {
    fn name(&self) -> &str {
        "invalid"
    }
    fn calculate(
        &mut self,
        _: &Configuration,
        _: usize,
        _: Edge,
        k: &[f64],
    ) -> Result<Vec<f64>, RmcError> {
        Ok(vec![f64::NAN; k.len()])
    }
}

#[test]
fn invalid_backend_spectrum_is_an_error() {
    assert!(refine(
        &problem(2.5),
        &RmcSettings::default(),
        &mut InvalidCalculator
    )
    .is_err());
}
