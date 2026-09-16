use nalgebra::DVector;
use rexafs::fitting::{transform::apply_kweight_transform, FeffFitTransform};
use rexafs::rmc::*;
use rexafs::structure::Edge;

#[derive(Default)]
struct Toy {
    calls: usize,
    fail: bool,
    options: Vec<Option<RefeffOptions>>,
}
impl ExafsCalculator for Toy {
    fn name(&self) -> &str {
        "deterministic dimer test v1"
    }
    fn calculate(
        &mut self,
        c: &Configuration,
        _: usize,
        _: Edge,
        k: &[f64],
    ) -> Result<Vec<f64>, RmcError> {
        self.calls += 1;
        if self.fail {
            return Err(RmcError::Calculator("injected failure".into()));
        }
        let r = c.atoms[1]
            .position
            .iter()
            .zip(c.atoms[0].position)
            .map(|(x, y)| (x - y).powi(2))
            .sum::<f64>()
            .sqrt();
        Ok(k.iter().map(|k| (2. * k * r).sin() / r.powi(2)).collect())
    }
    fn calculate_request(
        &mut self,
        r: CalculationRequest<'_>,
    ) -> Result<CalculatedSpectrum, RmcError> {
        self.options.push(r.options.cloned());
        Ok(CalculatedSpectrum {
            chi: self.calculate(r.configuration, r.absorber, r.edge, r.k)?,
            paths: Vec::new(),
        })
    }
}
fn dimer(r: f64) -> Configuration {
    Configuration {
        atoms: vec![
            Atom {
                atomic_number: 29,
                position: [0.; 3],
            },
            Atom {
                atomic_number: 29,
                position: [r, 0., 0.],
            },
        ],
        cell: None,
    }
}
fn problem() -> EnsembleProblem {
    let k: Vec<_> = (0..101).map(|i| 2. + i as f64 * 0.1).collect();
    let chi = Toy::default()
        .calculate(&dimer(2.5), 0, Edge::K, &k)
        .unwrap();
    RmcProblem {
        configuration: dimer(2.6),
        datasets: vec![ExafsDataset {
            name: "Cu".into(),
            absorbers: vec![0],
            edge: Edge::K,
            chi,
            sigma: vec![0.01; k.len()],
            k,
            weight: 1.,
            kweight: 0,
            s02: 1.,
            delta_e0: 0.,
        }],
    }
    .into()
}
fn settings() -> SessionSettings {
    SessionSettings {
        moves: RmcSettings {
            steps: 200,
            seed: 17,
            step_size: 0.06,
            temperature: 0.,
            movable_atoms: vec![1],
            ..Default::default()
        },
        trajectory_stride: 3,
        trajectory_capacity: 7,
        history_capacity: 200,
        ..Default::default()
    }
}

#[test]
fn exact_resume_and_failure_recovery_preserve_rng_best_and_trajectory() {
    let (p, s) = (problem(), settings());
    let mut calc = Toy::default();
    let mut full = RmcSession::new(&p, &s, &mut calc).unwrap();
    full.run(&mut calc).unwrap();
    let mut split = RmcSession::new(&p, &s, &mut calc).unwrap();
    for _ in 0..37 {
        split.step(&mut calc).unwrap();
    }
    let before = serde_json::to_value(split.checkpoint()).unwrap();
    calc.fail = true;
    assert!(split.step(&mut calc).is_err());
    assert_eq!(before, serde_json::to_value(split.checkpoint()).unwrap());
    calc.fail = false;
    let json = serde_json::to_string(&split.checkpoint()).unwrap();
    let mut resumed = RmcSession::resume(serde_json::from_str(&json).unwrap(), &mut calc).unwrap();
    resumed.run(&mut calc).unwrap();
    assert_eq!(
        serde_json::to_value(full.checkpoint()).unwrap(),
        serde_json::to_value(resumed.checkpoint()).unwrap()
    );
    assert!(full.best().evaluation.score < full.initial().evaluation.score * 0.01);
    assert_eq!(full.trajectory().len(), 7);
    assert_eq!(full.trajectory().last().unwrap().step, 198);
    let mut invalid = before;
    invalid["current"]["evaluation"]["score"] = serde_json::json!(123.);
    assert!(RmcSession::resume(serde_json::from_value(invalid).unwrap(), &mut calc).is_err());
}

#[test]
fn mixture_weight_refinement_uses_no_scattering_after_initialization() {
    let mut p = problem();
    p.structures = vec![
        WeightedStructure {
            configuration: dimer(2.4),
            weight: 0.5,
            movable_atoms: Some(vec![]),
        },
        WeightedStructure {
            configuration: dimer(2.8),
            weight: 0.5,
            movable_atoms: Some(vec![]),
        },
    ];
    let mut calc = Toy::default();
    let a = calc
        .calculate(
            &p.structures[0].configuration,
            0,
            Edge::K,
            &p.datasets[0].exafs.k,
        )
        .unwrap();
    let b = calc
        .calculate(
            &p.structures[1].configuration,
            0,
            Edge::K,
            &p.datasets[0].exafs.k,
        )
        .unwrap();
    p.datasets[0].exafs.chi = a.iter().zip(&b).map(|(x, y)| 0.7 * x + 0.3 * y).collect();
    let s = SessionSettings {
        moves: RmcSettings {
            steps: 300,
            temperature: 0.,
            ..Default::default()
        },
        weight_move_probability: 1.,
        weight_step: 0.1,
        ..Default::default()
    };
    let mut session = RmcSession::new(&p, &s, &mut calc).unwrap();
    let start = calc.calls;
    session.run(&mut calc).unwrap();
    assert_eq!(calc.calls, start);
    assert!((session.best().structures[0].weight - 0.7).abs() < 0.002);
    assert!(session.history().iter().any(|s| s.accepted));
}

#[test]
fn coordinate_moves_recalculate_only_changed_structure_and_forward_settings() {
    let mut p = problem();
    p.structures.push(WeightedStructure {
        configuration: dimer(2.9),
        weight: 1.,
        movable_atoms: Some(vec![]),
    });
    p.datasets[0].refeff = Some(RefeffOptions {
        polarization: Some([1., 0., 0.]),
        ..Default::default()
    });
    let mut calc = Toy::default();
    let mut session = RmcSession::new(&p, &settings(), &mut calc).unwrap();
    assert_eq!(calc.calls, 2);
    session.step(&mut calc).unwrap();
    assert_eq!(calc.calls, 3);
    assert!(calc
        .options
        .iter()
        .all(|o| o.as_ref().unwrap().polarization == Some([1., 0., 0.])));
    assert_eq!(
        session.current().structures[1].configuration,
        p.structures[1].configuration
    );
}

#[test]
fn pair_overrides_bond_energies_and_coordination_obey_periodic_images() {
    let mut p = problem();
    p.structures[0].configuration = dimer(0.8);
    p.structures[0].configuration.atoms[1].atomic_number = 8;
    let mut s = SessionSettings {
        moves: RmcSettings {
            steps: 0,
            min_distance: 1.,
            ..Default::default()
        },
        ..Default::default()
    };
    assert!(RmcSession::new(&p, &s, &mut Toy::default()).is_err());
    s.constraints.pairs.push(PairDistance {
        elements: [8, 29],
        minimum: 0.7,
    });
    s.constraints.bonds.push(BondRestraint {
        structure: 0,
        atoms: [0, 1],
        bounds: Some([0.7, 1.]),
        target: 0.9,
        strength: 100.,
    });
    let session = RmcSession::new(&p, &s, &mut Toy::default()).unwrap();
    assert!((session.current().penalty - 1.).abs() < 1e-12);
    let periodic = Configuration {
        atoms: vec![Atom {
            atomic_number: 29,
            position: [0.; 3],
        }],
        cell: Some([[2., 0., 0.], [0., 2., 0.], [0., 0., 2.]]),
    };
    let dist = distance_distribution(&periodic, &[0], Some(29), &[0., 1.9, 2.1, 2.7]).unwrap();
    assert_eq!(dist.counts_per_absorber, vec![0., 6., 0.]);
    assert_eq!(dist.mean, Some(2.));
    assert_eq!(dist.variance, Some(0.));
    let constraint = Constraints {
        coordination: vec![CoordinationRestraint {
            structure: 0,
            atom: 0,
            element: 29,
            cutoff: 2.1,
            target: 4.,
            strength: 2.,
        }],
        ..Default::default()
    };
    assert_eq!(
        constraint
            .penalty(&[WeightedStructure {
                configuration: periodic,
                weight: 1.,
                movable_atoms: None
            }])
            .unwrap(),
        8.
    );
}

#[test]
fn all_objectives_compare_whitened_residuals_and_fourier_matches_existing_engine() {
    let mut d = problem().datasets.remove(0).exafs;
    d.k = (0..241).map(|i| i as f64 * 0.05).collect();
    d.chi = vec![0.; d.k.len()];
    d.sigma = vec![2.; d.k.len()];
    d.weight = 3.;
    d.kweight = 1;
    let model: Vec<_> = d.k.iter().map(|k| (2. * 2.5 * k).sin()).collect();
    let transform = FeffFitTransform {
        kmin: 2.,
        kmax: 11.,
        rmin: 1.5,
        rmax: 3.5,
        kstep: Some(0.05),
        ..Default::default()
    };
    let residual =
        DVector::from_iterator(d.k.len(), model.iter().zip(&d.k).map(|(m, k)| m * k / 2.));
    let ft = apply_kweight_transform(&DVector::from_vec(d.k.clone()), &residual, &transform, 0.)
        .unwrap();
    let expected_r = 3.
        * ft.r_space
            .mask_indices
            .iter()
            .map(|&i| ft.r_space.chir[i].norm_sqr())
            .sum::<f64>()
        / ft.r_space.mask_indices.len() as f64;
    let expected_q =
        3. * ft.q_mask.iter().map(|&i| ft.chiq[i].powi(2)).sum::<f64>() / ft.q_mask.len() as f64;
    assert!(
        (Objective::R(transform.clone()).score(&d, &model).unwrap() - expected_r).abs() < 1e-10
    );
    assert!(
        (Objective::Q(transform.clone()).score(&d, &model).unwrap() - expected_q).abs() < 1e-10
    );
    for o in [
        Objective::K,
        Objective::R(transform.clone()),
        Objective::Q(transform),
        Objective::Wavelet(WaveletSettings {
            k_centers: vec![4., 6., 8.],
            r: vec![2., 2.5, 3.],
            omega0: 6.,
        }),
    ] {
        assert_eq!(o.score(&d, &d.chi).unwrap(), 0.);
        assert!(o.score(&d, &model).unwrap() > 0.);
    }
    d.kweight = 0;
    let peak = Objective::Wavelet(WaveletSettings {
        k_centers: vec![6.],
        r: vec![2.5],
        omega0: 8.,
    })
    .score(&d, &model)
    .unwrap();
    let off = Objective::Wavelet(WaveletSettings {
        k_centers: vec![6.],
        r: vec![4.5],
        omega0: 8.,
    })
    .score(&d, &model)
    .unwrap();
    assert!(peak > 1000. * off);
}

#[test]
fn evolutionary_elitism_resume_diversity_and_failure_are_transactional() {
    let (p, s) = (problem(), settings());
    let e = EvolutionSettings {
        population: 6,
        elite: 1,
        generations: 15,
        minimum_diversity: 1.,
        mutation_probability: 1.,
        ..Default::default()
    };
    let mut calc = Toy::default();
    let mut whole = EvolutionSession::new(&p, &s, &e, &mut calc).unwrap();
    let initial = whole.best().evaluation.score;
    whole.run(&mut calc).unwrap();
    assert!(whole.best().evaluation.score < initial);
    assert!(whole
        .history()
        .windows(2)
        .all(|v| v[1].best_score <= v[0].best_score));
    assert!(whole.history().iter().all(|v| v.hypermutation));
    let mut split = EvolutionSession::new(&p, &s, &e, &mut calc).unwrap();
    for _ in 0..4 {
        split.step(&mut calc).unwrap();
    }
    let before = serde_json::to_value(split.checkpoint()).unwrap();
    calc.fail = true;
    assert!(split.step(&mut calc).is_err());
    assert_eq!(before, serde_json::to_value(split.checkpoint()).unwrap());
    calc.fail = false;
    let checkpoint =
        serde_json::from_str(&serde_json::to_string(&split.checkpoint()).unwrap()).unwrap();
    let mut resumed = EvolutionSession::resume(checkpoint, &mut calc).unwrap();
    resumed.run(&mut calc).unwrap();
    assert_eq!(
        serde_json::to_value(whole.checkpoint()).unwrap(),
        serde_json::to_value(resumed.checkpoint()).unwrap()
    );
}

#[test]
fn energy_terms_and_bounds_are_not_disabled_by_zero_weight() {
    let structures = vec![WeightedStructure {
        configuration: dimer(2f64.powf(1. / 6.)),
        weight: 0.,
        movable_atoms: None,
    }];
    let restraints = Constraints {
        lennard_jones: vec![LennardJones {
            elements: [29, 29],
            sigma: 1.,
            epsilon: 2.,
            cutoff: 3.,
        }],
        ..Default::default()
    };
    let shift = 8. * ((1f64 / 3.).powi(12) - (1f64 / 3.).powi(6));
    assert!((restraints.penalty(&structures).unwrap() - (-2. - shift)).abs() < 1e-12);
    let mut p = problem();
    let mut s = settings();
    s.constraints.displacements.push(ElementDisplacement {
        element: 29,
        maximum: 1e-10,
    });
    let mut calc = Toy::default();
    let mut run = RmcSession::new(&p, &s, &mut calc).unwrap();
    run.run(&mut calc).unwrap();
    assert_eq!(calc.calls, 1);
    assert!(run.history().iter().all(|r| r.constraint_rejected));
    p.datasets[0].absorbers_by_structure = vec![vec![99]];
    assert!(RmcSession::new(&p, &s, &mut calc).is_err());
}
