use nalgebra::DVector;
use rexafs::fitting::{transform::apply_kweight_transform, FeffFitTransform};
use rexafs::rmc::*;
use rexafs::structure::Edge;

#[derive(Default)]
struct Toy {
    calls: usize,
    batches: Vec<usize>,
    fail: bool,
    fail_at: Option<usize>,
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
        if self.fail || self.fail_at.is_some_and(|n| self.calls >= n) {
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
    fn calculate_batch(
        &mut self,
        requests: &[CalculationRequest<'_>],
    ) -> Result<Vec<CalculatedSpectrum>, RmcError> {
        self.batches.push(requests.len());
        requests
            .iter()
            .map(|&r| self.calculate_request(r))
            .collect()
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
fn cooling_collective_moves_and_stopping_resume_without_history() {
    let mut p = problem();
    p.structures[0].movable_atoms = Some(vec![0, 1]);
    let mut s = settings();
    s.moves.temperature = 0.5;
    s.history_capacity = 0;
    s.cooling = CoolingSchedule::Geometric {
        factor: 0.95,
        floor: 0.01,
    };
    s.proposals = ProposalSettings {
        elements: vec![ElementProposal {
            element: 29,
            step_size: 0.02,
            weight: 0.,
        }],
        collective: vec![CollectiveProposal {
            structure: 0,
            atoms: vec![0, 1],
            step_size: 0.02,
            weight: 1.,
            independent: false,
        }],
    };
    s.stopping = StoppingSettings {
        patience: Some(9),
        minimum_improvement: 1e-8,
        acceptance_window: 5,
        minimum_acceptance: Some(0.1),
        ..Default::default()
    };
    let mut calc = Toy::default();
    let mut uninterrupted = RmcSession::new(&p, &s, &mut calc).unwrap();
    let mut split = RmcSession::new(&p, &s, &mut calc).unwrap();
    for step in 0..4 {
        let a = uninterrupted.step(&mut calc).unwrap().unwrap();
        let b = split.step(&mut calc).unwrap().unwrap();
        assert_eq!(a, b);
        assert!(matches!(a.proposal, EnsembleMove::Collective { .. }));
        assert!((a.temperature - 0.5 * 0.95_f64.powi(step)).abs() < 1e-14);
    }
    let before = serde_json::to_value(split.checkpoint()).unwrap();
    calc.fail = true;
    assert!(split.step(&mut calc).is_err());
    assert_eq!(before, serde_json::to_value(split.checkpoint()).unwrap());
    calc.fail = false;
    let checkpoint =
        serde_json::from_slice(&serde_json::to_vec(&split.checkpoint()).unwrap()).unwrap();
    let mut resumed = RmcSession::resume(checkpoint, &mut calc).unwrap();
    uninterrupted.run(&mut calc).unwrap();
    resumed.run(&mut calc).unwrap();
    assert_eq!(uninterrupted.current(), resumed.current());
    assert_eq!(uninterrupted.diagnostics(), resumed.diagnostics());
    assert_eq!(resumed.completed(), 9);
    assert_eq!(resumed.stop_reason(), Some(StopReason::Stagnation));
    assert!(resumed.history().is_empty());
    assert_eq!(resumed.diagnostics().attempts, 9);
    assert_eq!(resumed.diagnostics().recent_acceptance.len(), 5);
    assert_eq!(
        CoolingSchedule::Linear {
            final_temperature: 0.,
            attempts: 5
        }
        .temperature(1., 10)
        .unwrap(),
        0.
    );
}

#[test]
fn amplitude_energy_calibration_uses_one_union_grid_calculation() {
    let mut p = problem();
    let data = &mut p.datasets[0].exafs;
    let expected_e0 = 3.;
    let expected_s02 = 0.82;
    let q: Vec<_> = data
        .k
        .iter()
        .map(|v| (v * v - rexafs::xafs::xafsutils::constants::ETOK * expected_e0).sqrt())
        .collect();
    data.chi = Toy::default()
        .calculate(&p.structures[0].configuration, 0, Edge::K, &q)
        .unwrap()
        .iter()
        .map(|v| expected_s02 * v)
        .collect();
    let before = p.clone();
    let mut calc = Toy::default();
    let result = calibrate_dataset(
        &p,
        0,
        &CalibrationSettings {
            delta_e0: vec![1., 2., 3., 4., 5.],
            s02_bounds: [0.5, 1.2],
        },
        &mut calc,
    )
    .unwrap();
    assert_eq!(calc.calls, 1);
    assert_eq!(p, before);
    assert_eq!(result.best.delta_e0, expected_e0);
    assert!((result.best.s02 - expected_s02).abs() < 1e-12);
    assert!(result.report.normalized_k_residual.unwrap() < 1e-25);
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
    assert_eq!(calc.batches, vec![1, 5]);
    calc.batches.clear();
    let initial = whole.best().evaluation.score;
    whole.run(&mut calc).unwrap();
    assert!(calc.batches.iter().all(|&n| n > 1 && n <= 5));
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

#[test]
fn seeded_preparation_and_streaming_preserve_fixed_topology_and_bounds() {
    let c = dimer(2.6);
    let a = seeded_disorder(&c, &[1], 0.05, 42).unwrap();
    assert_eq!(a, seeded_disorder(&c, &[1], 0.05, 42).unwrap());
    assert_eq!(a.atoms[0], c.atoms[0]);
    assert!(a.atoms[1]
        .position
        .iter()
        .zip(c.atoms[1].position)
        .all(|(a, b)| (a - b).abs() <= 0.05));
    let doped = seeded_substitution(&c, 29, 30, 1, 42).unwrap();
    assert_eq!(doped, seeded_substitution(&c, 29, 30, 1, 42).unwrap());
    assert_eq!(doped.atoms.len(), 1);
    assert_eq!(doped.configuration.atoms[doped.atoms[0]].atomic_number, 30);
    assert!(seeded_substitution(&c, 8, 29, 1, 42).is_err());
    let structure = rexafs::structure::BuiltinLibrary::get()
        .unwrap()
        .structure("cu2o_cuprite")
        .unwrap();
    assert_eq!(suggested_supercell_repeats(&structure, 9.).unwrap(), [3; 3]);
    let resources = estimate_catalogue_resources(
        &c,
        &[0, 1],
        &PathCatalogueSettings {
            radius: 3.,
            max_legs: 2,
            displacement: 0.1,
            ..Default::default()
        },
        100,
    )
    .unwrap();
    assert_eq!(resources.catalogue_paths, 2);
    assert_eq!(resources.active_paths, 2);
    assert_eq!(resources.spectrum_payload_bytes, 1600);

    let p = problem();
    let s = settings();
    let mut calc = Toy::default();
    let mut stream = p.structures[0].configuration.to_xyz().unwrap();
    stream.push_str(&a.to_xyz().unwrap());
    let mut emitted = Vec::new();
    assert_eq!(
        stream_xyz_spectra(
            std::io::Cursor::new(&stream),
            &p,
            0,
            &s,
            &mut calc,
            |i, state| {
                emitted.push((i, state));
                Ok(std::ops::ControlFlow::Continue(()))
            }
        )
        .unwrap(),
        2
    );
    assert_eq!(emitted[1].0, 1);
    let mut changed = p.clone();
    changed.structures[0].configuration =
        Configuration::from_xyz(&rexafs::structure::parse_xyz(&a.to_xyz().unwrap()).unwrap())
            .unwrap();
    let expected = evaluate_ensemble(&changed, &s, &mut calc).unwrap();
    assert_eq!(emitted[1].1, expected);
    // Early termination does not parse the deliberately malformed next frame.
    let text = format!("{}bad\n", c.to_xyz().unwrap());
    assert_eq!(
        stream_xyz_spectra(
            std::io::Cursor::new(&text),
            &p,
            0,
            &s,
            &mut calc,
            |_, _| Ok(std::ops::ControlFlow::Break(()))
        )
        .unwrap(),
        1
    );
    let bad = format!("{}2\ntruncated\nCu 0 0 0\n", c.to_xyz().unwrap());
    let mut count = 0;
    assert!(
        stream_xyz_spectra(std::io::Cursor::new(&bad), &p, 0, &s, &mut calc, |_, _| {
            count += 1;
            Ok(std::ops::ControlFlow::Continue(()))
        })
        .is_err()
    );
    assert_eq!(count, 1);
    assert!(stream_xyz_spectra(
        std::io::Cursor::new(doped.configuration.to_xyz().unwrap()),
        &p,
        0,
        &s,
        &mut calc,
        |_, _| Ok(std::ops::ControlFlow::Continue(()))
    )
    .is_err());
}

#[test]
fn residual_trend_requires_recent_best_and_mean_stability_and_enough_history() {
    let settings = ResidualTrendSettings {
        window: 10,
        stable_windows: 3,
        minimum_attempts: 40,
        absolute_tolerance: 1e-8,
        relative_best_tolerance: 0.005,
        relative_mean_tolerance: 0.01,
    };
    let rows = |f: &dyn Fn(usize) -> (f64, f64)| {
        (1..=40)
            .map(|step| {
                let (score, best_score) = f(step);
                SessionStep {
                    energy: None,
                    step,
                    proposal: EnsembleMove::Atom {
                        structure: 0,
                        atom: 1,
                    },
                    accepted: true,
                    constraint_rejected: false,
                    trial_score: Some(score),
                    score,
                    best_score,
                    temperature: 0.,
                    step_scale: 1.,
                }
            })
            .collect::<Vec<_>>()
    };
    let flat = rows(&|_| (1.1, 1.));
    assert_eq!(
        residual_trend(&flat, &settings).unwrap().status,
        ResidualTrendStatus::ResidualPlateau
    );
    assert_eq!(
        residual_trend(&flat[..39], &settings).unwrap().status,
        ResidualTrendStatus::InsufficientHistory
    );
    let improving = rows(&|s| {
        let value = 2. - s as f64 * 0.02;
        (value, value)
    });
    assert_eq!(
        residual_trend(&improving, &settings).unwrap().status,
        ResidualTrendStatus::StillChanging
    );
    let drifting = rows(&|s| (1.1 + s as f64 * 0.02, 1.));
    assert_eq!(
        residual_trend(&drifting, &settings).unwrap().status,
        ResidualTrendStatus::StillChanging
    );
    let zero = rows(&|_| (0., 0.));
    assert_eq!(
        residual_trend(&zero, &settings).unwrap().status,
        ResidualTrendStatus::ResidualPlateau
    );
    let mut gap = flat;
    gap.remove(20);
    assert!(residual_trend(&gap, &settings).is_err());
}

#[test]
fn normalized_r_objective_matches_standard_fitter_real_and_imaginary_residuals() {
    use rexafs::fitting::transform::{apply_dataset_transform, residual_for_dataset};
    use rexafs::xafs::xafsutils::FTWindow;
    let mut d = problem().datasets.remove(0).exafs;
    d.k = (0..241).map(|i| i as f64 * 0.05).collect();
    d.chi =
        d.k.iter()
            .map(|k| (4. * k).sin() * (-0.01 * k * k).exp())
            .collect();
    d.sigma = vec![1.; d.k.len()];
    d.weight = 1.;
    d.kweight = 2;
    let model: Vec<_> =
        d.k.iter()
            .map(|k| (4. * k + 0.35).sin() * (-0.01 * k * k).exp())
            .collect();
    let t = FeffFitTransform {
        kmin: 3.,
        kmax: 11.5,
        dk: 1.,
        window: FTWindow::Hanning,
        rmin: 0.8,
        rmax: 4.,
        ..Default::default()
    };
    let objective = Objective::R(t.clone());
    d.weight = 1. / objective.score(&d, &vec![0.; d.k.len()]).unwrap();
    let grid = DVector::from_vec(d.k.clone());
    let data = apply_dataset_transform(&grid, &DVector::from_vec(d.chi.clone()), &t).unwrap();
    let fitted = apply_dataset_transform(&grid, &DVector::from_vec(model.clone()), &t).unwrap();
    let residual = residual_for_dataset(&data, Some(&fitted), &t, &[1.]).unwrap();
    let baseline = residual_for_dataset(&data, None, &t, &[1.]).unwrap();
    let real: f64 = residual.iter().step_by(2).map(|x| x * x).sum();
    let imag: f64 = residual.iter().skip(1).step_by(2).map(|x| x * x).sum();
    assert!(real > 0. && imag > 0.);
    let expected = (real + imag) / baseline.norm_squared();
    assert!((objective.score(&d, &model).unwrap() - expected).abs() < 1e-12);
    assert_eq!(objective.score(&d, &d.chi).unwrap(), 0.);
}

#[test]
fn adaptation_is_bounded_checkpointed_and_uses_constraint_rejections() {
    let p = problem();
    let mut s = settings();
    s.moves.steps = 20;
    s.moves.max_displacement = Some(1e-12);
    s.adaptation = Some(StepAdaptation {
        window: 4,
        factor: 2.,
        minimum_scale: 0.2,
        freeze_after: Some(10),
        ..Default::default()
    });
    let mut calc = Toy::default();
    let mut full = RmcSession::new(&p, &s, &mut calc).unwrap();
    full.run(&mut calc).unwrap();
    assert_eq!(calc.calls, 1, "hard rejects must avoid scattering");
    assert_eq!(full.history()[3].step_scale, 1.);
    assert_eq!(full.history()[4].step_scale, 0.5);
    assert_eq!(full.history()[8].step_scale, 0.25);
    assert_eq!(full.adaptation().scale, 0.25, "frozen before third window");
    assert_eq!(full.adaptation().attempts, 20);
    assert_eq!(full.adaptation().last_acceptance, Some(0.));
    let mut split = RmcSession::new(&p, &s, &mut calc).unwrap();
    for _ in 0..7 {
        split.step(&mut calc).unwrap();
    }
    let cp = serde_json::from_value(serde_json::to_value(split.checkpoint()).unwrap()).unwrap();
    let mut resumed = RmcSession::resume(cp, &mut calc).unwrap();
    resumed.run(&mut calc).unwrap();
    assert_eq!(
        serde_json::to_value(full.checkpoint()).unwrap(),
        serde_json::to_value(resumed.checkpoint()).unwrap()
    );
    let mut bad = serde_json::to_value(resumed.checkpoint()).unwrap();
    bad["adaptation"]["scale"] = serde_json::json!(0.);
    assert!(RmcSession::resume(serde_json::from_value(bad).unwrap(), &mut calc).is_err());
    s.adaptation.as_mut().unwrap().freeze_after = None;
    let mut run = RmcSession::new(&p, &s, &mut calc).unwrap();
    run.run(&mut calc).unwrap();
    assert_eq!(run.adaptation().scale, 0.2);
}

#[test]
fn hybrid_refinement_preserves_original_bounds_and_exact_resume() {
    let p = problem();
    let mut s = settings();
    s.moves.max_displacement = Some(0.14);
    s.cooling = CoolingSchedule::Geometric {
        factor: 0.99,
        floor: 0.,
    };
    s.adaptation = Some(StepAdaptation {
        window: 7,
        ..Default::default()
    });
    let e = EvolutionSettings {
        population: 5,
        elite: 1,
        generations: 12,
        local_steps: 9,
        mutation_probability: 0.,
        minimum_diversity: 0.,
        stagnation_generations: 0,
        ..Default::default()
    };
    let mut calc = Toy::default();
    let mut full = EvolutionSession::new(&p, &s, &e, &mut calc).unwrap();
    let original = full.best().evaluation.score;
    full.run(&mut calc).unwrap();
    assert_eq!(full.local_completed(), 12 * 4 * 9);
    assert!(full.best().evaluation.score < original);
    assert!(full.history().iter().all(|r| r.local_attempts == 36));
    assert!(full.history().iter().any(|r| r.local_accepted > 0));
    assert!(full
        .history()
        .iter()
        .any(|r| r.local_constraint_rejected > 0));
    assert!(full
        .history()
        .windows(2)
        .all(|r| r[1].best_score <= r[0].best_score));
    for state in full.population() {
        let atom = &state.structures[0].configuration.atoms[1];
        let initial = &p.structures[0].configuration.atoms[1];
        let displacement = atom
            .position
            .iter()
            .zip(initial.position)
            .map(|(a, b)| (a - b).powi(2))
            .sum::<f64>()
            .sqrt();
        assert!(displacement <= 0.14);
        assert_eq!(
            state.structures[0].configuration.atoms[0],
            p.structures[0].configuration.atoms[0]
        );
    }
    let mut split = EvolutionSession::new(&p, &s, &e, &mut calc).unwrap();
    for _ in 0..3 {
        split.step(&mut calc).unwrap();
    }
    let cp = serde_json::to_value(split.checkpoint()).unwrap();
    calc.fail_at = Some(calc.calls + 7);
    assert!(
        split.step(&mut calc).is_err(),
        "failure occurs after successful local moves"
    );
    assert_eq!(cp, serde_json::to_value(split.checkpoint()).unwrap());
    calc.fail_at = None;
    let mut resumed =
        EvolutionSession::resume(serde_json::from_value(cp).unwrap(), &mut calc).unwrap();
    resumed.run(&mut calc).unwrap();
    assert_eq!(
        serde_json::to_value(full.checkpoint()).unwrap(),
        serde_json::to_value(resumed.checkpoint()).unwrap()
    );
}

#[test]
fn first_shell_bridge_recovers_four_parameters_and_transfers_only_amplitude_energy() {
    use rexafs::fitting::{feffpath, ff2chi, FeffFitDataset, FeffFlavor, FitVariables};
    let file = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/testfiles/xraylarch_d867/feffit/Feff_Cu/feff0001.dat");
    let path = feffpath(file.to_str().unwrap(), FeffFlavor::Feff85L).unwrap();
    let k = DVector::from_iterator(301, (0..301).map(|i| i as f64 * 0.05));
    let truth = path
        .clone()
        .set_s02(0.83)
        .set_e0(2.5)
        .set_deltar(path.feff.reff * 0.012)
        .set_sigma2(0.004);
    let chi = ff2chi(&[truth], &FitVariables::new(), &k).unwrap().chi;
    let input = FeffFitDataset::new()
        .data(&k, &chi)
        .add_path(path)
        .krange(3., 13.)
        .rrange(1.3, 3.2);
    let before = input.clone();
    let result = calibrate_first_shell(
        &input,
        &FirstShellSettings::default(),
        "synthetic Cu fixture",
    )
    .unwrap();
    assert_eq!(input, before);
    assert!(result.fit.r_factor < 1e-10);
    for (name, expected) in [
        ("rmc_s02", 0.83),
        ("rmc_delta_e0", 2.5),
        ("rmc_distance_scale", 1.012),
        ("rmc_sigma2", 0.004),
    ] {
        assert!(
            (result.fit.variables.get(name).unwrap().value - expected).abs() < 1e-5,
            "{name}"
        );
    }
    let p = problem();
    let calibrated = result.apply_amplitude_energy(&p, 0).unwrap();
    assert_eq!(p.structures, calibrated.structures);
    assert_eq!(p.datasets[0].objective, calibrated.datasets[0].objective);
    assert!((calibrated.datasets[0].exafs.s02 - 0.83).abs() < 1e-5);
    assert!((calibrated.datasets[0].exafs.delta_e0 - 2.5).abs() < 1e-5);
    let mut bad = result.clone();
    bad.fit.solver_report.as_mut().unwrap().converged = false;
    assert!(bad.apply_amplitude_energy(&p, 0).is_err());
}

#[test]
fn explicit_model_revision_rescores_all_states_atomically_and_clears_old_trends() {
    struct Revised {
        toy: Toy,
        factor: f64,
    }
    impl ExafsCalculator for Revised {
        fn name(&self) -> &str {
            "revised dimer"
        }
        fn identity(&self) -> String {
            format!("revised-dimer/{}", self.factor)
        }
        fn calculate(
            &mut self,
            c: &Configuration,
            a: usize,
            e: Edge,
            k: &[f64],
        ) -> Result<Vec<f64>, RmcError> {
            Ok(self
                .toy
                .calculate(c, a, e, k)?
                .into_iter()
                .map(|x| x * self.factor)
                .collect())
        }
    }
    let p = problem();
    let s = settings();
    let mut old = Toy::default();
    let mut revised = Revised {
        toy: Toy::default(),
        factor: 1.1,
    };
    let mut run = RmcSession::new(&p, &s, &mut old).unwrap();
    for _ in 0..8 {
        run.step(&mut old).unwrap();
    }
    let cp = serde_json::to_value(run.checkpoint()).unwrap();
    assert!(run.step(&mut revised).is_err());
    revised.toy.fail_at = Some(2);
    assert!(run.rebase(&mut revised).is_err());
    assert_eq!(cp, serde_json::to_value(run.checkpoint()).unwrap());
    revised.toy.fail_at = None;
    run.rebase(&mut revised).unwrap();
    let after = serde_json::to_value(run.checkpoint()).unwrap();
    assert_eq!(cp["rng"], after["rng"]);
    assert_eq!(cp["problem"], after["problem"]);
    assert_eq!(cp["current"]["structures"], after["current"]["structures"]);
    assert!(run.history().is_empty() && run.trajectory().is_empty());
    assert_eq!(run.revisions().len(), 1);
    assert_eq!(run.revisions()[0].completed, 8);
    assert!(run.best().evaluation.score <= run.current().evaluation.score);
    let mut resumed = RmcSession::resume(run.checkpoint(), &mut revised).unwrap();
    run.step(&mut revised).unwrap();
    resumed.step(&mut revised).unwrap();
    assert_eq!(
        serde_json::to_value(run.checkpoint()).unwrap(),
        serde_json::to_value(resumed.checkpoint()).unwrap()
    );
    let e = EvolutionSettings {
        population: 4,
        elite: 1,
        local_steps: 2,
        ..Default::default()
    };
    let mut population = EvolutionSession::new(&p, &s, &e, &mut old).unwrap();
    population.step(&mut old).unwrap();
    let before = serde_json::to_value(population.checkpoint()).unwrap();
    population.rebase(&mut revised).unwrap();
    let after = serde_json::to_value(population.checkpoint()).unwrap();
    assert_eq!(before["rng"], after["rng"]);
    assert_eq!(before["local_completed"], after["local_completed"]);
    assert_eq!(population.revisions().len(), 1);
    assert!(population.history().is_empty());
    EvolutionSession::resume(population.checkpoint(), &mut revised).unwrap();
}

#[test]
fn acceptance_feedback_scales_species_and_collective_moves_without_changing_draws() {
    struct Flat;
    impl ExafsCalculator for Flat {
        fn name(&self) -> &str {
            "flat adaptation fixture"
        }
        fn calculate(
            &mut self,
            _: &Configuration,
            _: usize,
            _: Edge,
            k: &[f64],
        ) -> Result<Vec<f64>, RmcError> {
            Ok(vec![0.; k.len()])
        }
    }
    for collective in [false, true] {
        let mut fixed = settings();
        if collective {
            fixed.proposals.elements = vec![ElementProposal {
                element: 29,
                step_size: 0.01,
                weight: 0.,
            }];
            fixed.proposals.collective = vec![CollectiveProposal {
                structure: 0,
                atoms: vec![1],
                step_size: 0.02,
                weight: 1.,
                independent: true,
            }];
        } else {
            fixed.proposals.elements = vec![ElementProposal {
                element: 29,
                step_size: 0.01,
                weight: 1.,
            }];
        }
        let mut adaptive = fixed.clone();
        adaptive.adaptation = Some(StepAdaptation {
            window: 1,
            factor: 2.,
            maximum_scale: 2.,
            ..Default::default()
        });
        let mut a = RmcSession::new(&problem(), &fixed, &mut Flat).unwrap();
        let mut b = RmcSession::new(&problem(), &adaptive, &mut Flat).unwrap();
        a.step(&mut Flat).unwrap();
        b.step(&mut Flat).unwrap();
        assert_eq!(a.current(), b.current());
        let before = a.current().structures[0].configuration.atoms[1].position;
        a.step(&mut Flat).unwrap();
        let record = b.step(&mut Flat).unwrap().unwrap();
        assert_eq!(record.step_scale, 2.);
        for (axis, origin) in before.iter().enumerate() {
            let da = a.current().structures[0].configuration.atoms[1].position[axis] - origin;
            let db = b.current().structures[0].configuration.atoms[1].position[axis] - origin;
            assert!((db - 2. * da).abs() < 1e-14);
        }
    }
}

#[test]
fn population_trend_needs_recorded_means_and_rejects_mean_drift() {
    let settings = ResidualTrendSettings {
        window: 2,
        stable_windows: 2,
        minimum_attempts: 6,
        ..Default::default()
    };
    let mut history: Vec<_> = (1..=6)
        .map(|generation| EvolutionGeneration {
            generation,
            best_score: 1.,
            mean_score: Some(1.1),
            diversity: 0.,
            hypermutation: false,
            constraint_fallbacks: 0,
            local_attempts: 0,
            local_accepted: 0,
            local_constraint_rejected: 0,
            step_scale: 1.,
        })
        .collect();
    assert_eq!(
        evolution_residual_trend(&history, &settings)
            .unwrap()
            .status,
        ResidualTrendStatus::ResidualPlateau
    );
    history[0].mean_score = None;
    assert_eq!(
        evolution_residual_trend(&history, &settings)
            .unwrap()
            .status,
        ResidualTrendStatus::InsufficientHistory
    );
    history[0].mean_score = Some(1.1);
    history[5].mean_score = Some(1.5);
    assert_eq!(
        evolution_residual_trend(&history, &settings)
            .unwrap()
            .status,
        ResidualTrendStatus::StillChanging
    );
}

#[test]
fn identical_population_mean_does_not_round_below_its_best() {
    let e = EvolutionSettings {
        generations: 6,
        mutation_probability: 0.,
        ..Default::default()
    };
    let mut calc = Toy::default();
    let mut session = EvolutionSession::new(&problem(), &settings(), &e, &mut calc).unwrap();
    session.run(&mut calc).unwrap();
    assert!(session
        .history()
        .iter()
        .all(|r| r.mean_score.unwrap() >= r.best_score));
    let trend = ResidualTrendSettings {
        window: 2,
        stable_windows: 2,
        minimum_attempts: 6,
        ..Default::default()
    };
    assert_eq!(
        evolution_residual_trend(session.history(), &trend)
            .unwrap()
            .status,
        ResidualTrendStatus::ResidualPlateau
    );
}

#[test]
fn auto_move_policy_grows_shrinks_freezes_and_resumes_exactly() {
    let mut policy = settings();
    policy.moves.steps = 600;
    policy.moves.max_displacement = Some(1e-12);
    policy.moves.temperature = 0.001;
    let policy = policy.with_auto_moves();
    let mut calc = Toy::default();
    let mut full = RmcSession::new(&problem(), &policy, &mut calc).unwrap();
    full.run(&mut calc).unwrap();
    assert!(full.adaptation().scale < 1.);
    assert_eq!(full.adaptation().updates, 4);
    assert_eq!(policy.cooling.temperature(0.001, 600).unwrap(), 0.);
    let mut split = RmcSession::new(&problem(), &policy, &mut calc).unwrap();
    for _ in 0..230 {
        split.step(&mut calc).unwrap();
    }
    let cp = serde_json::from_value(serde_json::to_value(split.checkpoint()).unwrap()).unwrap();
    let mut resumed = RmcSession::resume(cp, &mut calc).unwrap();
    resumed.run(&mut calc).unwrap();
    assert_eq!(
        serde_json::to_value(full.checkpoint()).unwrap(),
        serde_json::to_value(resumed.checkpoint()).unwrap()
    );
    let scale = resumed.adaptation().scale;
    resumed.set_step_limit(610).unwrap();
    resumed.run(&mut calc).unwrap();
    assert_eq!(scale, resumed.adaptation().scale);
    assert_eq!(resumed.history().last().unwrap().temperature, 0.);

    struct Flat;
    impl ExafsCalculator for Flat {
        fn name(&self) -> &str {
            "constant spectrum"
        }
        fn calculate(
            &mut self,
            _: &Configuration,
            _: usize,
            _: Edge,
            k: &[f64],
        ) -> Result<Vec<f64>, RmcError> {
            Ok(vec![0.; k.len()])
        }
    }
    let mut policy = settings().with_auto_moves();
    policy.moves.steps = 500;
    policy.moves.max_displacement = Some(100.);
    policy.moves.min_distance = 0.01;
    policy.adaptation.as_mut().unwrap().freeze_after = Some(500);
    let mut run = RmcSession::new(&problem(), &policy, &mut Flat).unwrap();
    run.run(&mut Flat).unwrap();
    assert_eq!(run.adaptation().scale, 2.);
}

#[test]
fn local_gradient_refinement_reduces_full_spectrum_and_preserves_checkpoint() {
    let mut calc = Toy::default();
    let run = RmcSession::new(&problem(), &settings(), &mut calc).unwrap();
    let cp = serde_json::to_value(run.checkpoint()).unwrap();
    let options = LocalRefinementSettings {
        sweeps: 12,
        ..Default::default()
    };
    let result = run.refine_best(&options, &mut calc).unwrap();
    assert!(result.best.evaluation.score < result.initial.evaluation.score * 1e-6);
    assert!(!result.history.is_empty());
    assert!(result
        .history
        .iter()
        .all(|h| h.after < h.before && h.displacement <= options.maximum_step));
    assert_eq!(
        result.best.structures[0].configuration.atoms[0],
        result.initial.structures[0].configuration.atoms[0]
    );
    assert_eq!(serde_json::to_value(run.checkpoint()).unwrap(), cp);
    let mut p = result.problem.clone();
    p.structures = result.best.structures.clone();
    let independent = evaluate_ensemble(&p, &settings(), &mut calc).unwrap();
    assert_eq!(result.best.evaluation.score, independent.evaluation.score);
    let serialized = serde_json::to_value(&result).unwrap();
    let reloaded: LocalRefinementResult = serde_json::from_value(serialized).unwrap();
    assert_eq!(reloaded.best, result.best);
}

#[test]
fn evolutionary_local_refinement_preserves_population_and_original_bounds() {
    let mut s = settings();
    s.moves.max_displacement = Some(0.04);
    let mut calc = Toy::default();
    let mut run = EvolutionSession::new(
        &problem(),
        &s,
        &EvolutionSettings {
            population: 3,
            elite: 1,
            generations: 1,
            ..Default::default()
        },
        &mut calc,
    )
    .unwrap();
    run.run(&mut calc).unwrap();
    let checkpoint = serde_json::to_value(run.checkpoint()).unwrap();
    let result = run
        .refine_best_with_progress(
            &LocalRefinementSettings {
                sweeps: 20,
                ..Default::default()
            },
            &mut calc,
            |_| std::ops::ControlFlow::Continue(()),
        )
        .unwrap();
    assert_eq!(result.source_attempt, 1);
    assert!(result.best.evaluation.score <= result.initial.evaluation.score);
    assert!(result.best.structures[0].configuration.atoms[1].position[0] >= 2.56 - 1e-12);
    assert_eq!(serde_json::to_value(run.checkpoint()).unwrap(), checkpoint);
    assert!(run.set_generation_limit(0).is_err());
    run.set_generation_limit(2).unwrap();
    run.run(&mut calc).unwrap();
    assert_eq!(run.completed(), 2);
}

#[test]
fn local_refinement_respects_original_bounds_budget_cancellation_and_failure() {
    let mut s = settings();
    s.moves.max_displacement = Some(0.04);
    let mut calc = Toy::default();
    let run = RmcSession::new(&problem(), &s, &mut calc).unwrap();
    let cp = serde_json::to_value(run.checkpoint()).unwrap();
    let result = run
        .refine_best(
            &LocalRefinementSettings {
                sweeps: 20,
                ..Default::default()
            },
            &mut calc,
        )
        .unwrap();
    assert!(result.best.structures[0].configuration.atoms[1].position[0] >= 2.56 - 1e-12);
    assert!(result.best.evaluation.score < result.initial.evaluation.score);
    let result = run
        .refine_best(
            &LocalRefinementSettings {
                evaluations: 3,
                ..Default::default()
            },
            &mut calc,
        )
        .unwrap();
    assert_eq!(result.evaluations, 3);
    assert_eq!(result.stop, LocalRefinementStop::EvaluationLimit);
    let cancelled = run
        .refine_best_with_progress(&LocalRefinementSettings::default(), &mut calc, |_| {
            std::ops::ControlFlow::Break(())
        })
        .unwrap();
    assert_eq!(cancelled.stop, LocalRefinementStop::Cancelled);
    assert_eq!(cancelled.best, cancelled.initial);
    calc.fail_at = Some(calc.calls + 3);
    assert!(run
        .refine_best(&LocalRefinementSettings::default(), &mut calc)
        .is_err());
    assert_eq!(serde_json::to_value(run.checkpoint()).unwrap(), cp);
    for bad in [
        LocalRefinementSettings {
            difference_step: f64::NAN,
            ..Default::default()
        },
        LocalRefinementSettings {
            evaluations: 0,
            ..Default::default()
        },
    ] {
        assert!(run.refine_best(&bad, &mut calc).is_err());
    }
}

#[path = "rmc_session/energy.rs"]
mod energy;
