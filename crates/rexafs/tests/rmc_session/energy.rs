use super::*;

fn shifted_problem() -> EnsembleProblem {
    let mut p = problem();
    p.datasets[0].exafs.s02 = 0.8;
    let mut second = p.datasets[0].clone();
    second.exafs.name = "second edge".into();
    second.exafs.s02 = 0.65;
    p.datasets.push(second);
    for (d, target) in p.datasets.iter_mut().zip([1.3, -2.4]) {
        let mut true_data = d.exafs.clone();
        true_data.delta_e0 = target;
        d.exafs.chi = Toy::default()
            .calculate(
                &p.structures[0].configuration,
                0,
                Edge::K,
                &true_data.theoretical_k().unwrap(),
            )
            .unwrap()
            .iter()
            .map(|v| v * d.exafs.s02)
            .collect();
    }
    p
}
fn energy_settings() -> SessionSettings {
    let mut s = settings().with_energy_refinement(-3.0..=3.0);
    s.energy_refinement.as_mut().unwrap().interval = 3;
    s.moves.steps = 7;
    s.moves.step_size = 0.002;
    s.moves.temperature = 0.001;
    s.trajectory_stride = 1;
    s
}

#[test]
fn independent_shifts_keep_amplitude_geometry_and_measurement_unchanged() {
    let p = shifted_problem();
    let before = serde_json::to_value(&p).unwrap();
    let s = energy_settings();
    let mut calc = Toy::default();
    let run = RmcSession::new(&p, &s, &mut calc).unwrap();
    let shifts = run.best().energy_shifts(run.problem()).unwrap();
    assert!((shifts[0] - 1.3).abs() < 1e-12);
    assert!((shifts[1] + 2.4).abs() < 1e-12);
    assert!(run.best().evaluation.score < 1e-20);
    assert_eq!(run.best().structures, run.initial().structures);
    assert_eq!(
        run.initial().energy_shifts(run.problem()).unwrap(),
        [0., 0.]
    );
    assert_eq!(before, serde_json::to_value(&p).unwrap());
    assert_eq!(before, serde_json::to_value(run.problem()).unwrap());
    assert_eq!(run.initial_energy().unwrap().after, shifts);
    // A separate fixed-energy evaluation must reproduce both fitted arrays.
    let mut verified = p.clone();
    for (d, shift) in verified.datasets.iter_mut().zip(&shifts) {
        d.exafs.delta_e0 = *shift;
    }
    let direct = evaluate_ensemble(&verified, &settings(), &mut calc).unwrap();
    assert_eq!(direct.evaluation, run.best().evaluation);
    let local = run
        .refine_best(
            &LocalRefinementSettings {
                evaluations: 10,
                ..Default::default()
            },
            &mut calc,
        )
        .unwrap();
    assert_eq!(local.initial.delta_e0, shifts);
    assert_eq!(local.best.delta_e0, shifts);
    assert_eq!(local.initial.evaluation, run.best().evaluation);
    local.validate().unwrap();
}

#[test]
fn periodic_energy_updates_and_resume_preserve_exact_current_best_and_rng() {
    let p = shifted_problem();
    let s = energy_settings();
    let mut calc = Toy::default();
    let mut whole = RmcSession::new(&p, &s, &mut calc).unwrap();
    let mut split = RmcSession::new(&p, &s, &mut calc).unwrap();
    whole.run(&mut calc).unwrap();
    for _ in 0..4 {
        split.step(&mut calc).unwrap();
    }
    let cp = serde_json::from_value(serde_json::to_value(split.checkpoint()).unwrap()).unwrap();
    let mut resumed = RmcSession::resume(cp, &mut Toy::default()).unwrap();
    resumed.run(&mut calc).unwrap();
    assert_eq!(
        serde_json::to_value(whole.checkpoint()).unwrap(),
        serde_json::to_value(resumed.checkpoint()).unwrap()
    );
    assert_eq!(
        resumed
            .history()
            .iter()
            .filter(|h| h.energy.is_some())
            .map(|h| h.step)
            .collect::<Vec<_>>(),
        [3, 6]
    );
    for h in resumed.history().iter().filter_map(|h| h.energy.as_ref()) {
        assert!(h.score_after <= h.score_before);
        assert_eq!(h.after.len(), 2);
    }
    assert!(resumed.trajectory().iter().all(|f| f.delta_e0.len() == 2));
    assert_eq!(resumed.problem().datasets[0].exafs.s02, 0.8);
    assert_eq!(resumed.problem().datasets[1].exafs.s02, 0.65);
    // Tampering with parameter provenance must be detected on resume.
    for key in ["current", "best", "initial"] {
        let mut json = serde_json::to_value(resumed.checkpoint()).unwrap();
        json[key]["delta_e0"] = serde_json::json!([0.]);
        assert!(RmcSession::resume(serde_json::from_value(json).unwrap(), &mut calc).is_err());
    }
}

#[test]
fn failing_energy_search_rolls_back_the_coordinate_move_and_random_stream() {
    let p = shifted_problem();
    let s = energy_settings();
    let mut calc = Toy::default();
    let mut run = RmcSession::new(&p, &s, &mut calc).unwrap();
    for _ in 0..2 {
        run.step(&mut calc).unwrap();
    }
    let before = serde_json::to_value(run.checkpoint()).unwrap();
    let mut reference = RmcSession::resume(
        serde_json::from_value(before.clone()).unwrap(),
        &mut Toy::default(),
    )
    .unwrap();
    // Two dataset calculations complete the coordinate trial; then calibration fails.
    calc.fail_at = Some(calc.calls + 3);
    assert!(run.step(&mut calc).is_err());
    assert_eq!(before, serde_json::to_value(run.checkpoint()).unwrap());
    calc.fail_at = None;
    assert_eq!(
        run.step(&mut calc).unwrap(),
        reference.step(&mut calc).unwrap()
    );
    assert_eq!(
        serde_json::to_value(run.checkpoint()).unwrap(),
        serde_json::to_value(reference.checkpoint()).unwrap()
    );
}

#[test]
fn invalid_energy_bounds_and_grids_fail_before_scattering() {
    let p = shifted_problem();
    for policy in [
        EnergyRefinement {
            interval: 0,
            ..Default::default()
        },
        EnergyRefinement {
            step: 0.,
            ..Default::default()
        },
        EnergyRefinement {
            bounds: [1., 2.],
            ..Default::default()
        },
        EnergyRefinement {
            bounds: [-3., 50.],
            ..Default::default()
        },
        EnergyRefinement {
            step: 1e-20,
            ..Default::default()
        },
    ] {
        let s = SessionSettings {
            energy_refinement: Some(policy),
            ..settings()
        };
        let mut calc = Toy::default();
        assert!(RmcSession::new(&p, &s, &mut calc).is_err());
        assert_eq!(calc.calls, 0);
    }
}

#[test]
fn fixed_energy_checkpoints_remain_compatible_and_omit_new_fields() {
    let p = problem();
    let s = settings();
    let mut calc = Toy::default();
    let mut run = RmcSession::new(&p, &s, &mut calc).unwrap();
    run.step(&mut calc).unwrap();
    let json = serde_json::to_value(run.checkpoint()).unwrap();
    assert!(json.get("initial_energy").is_none());
    assert!(json["settings"].get("energy_refinement").is_none());
    assert!(json["best"].get("delta_e0").is_none());
    assert!(json["history"][0].get("energy").is_none());
    let resumed =
        RmcSession::resume(serde_json::from_value(json.clone()).unwrap(), &mut calc).unwrap();
    assert_eq!(json, serde_json::to_value(resumed.checkpoint()).unwrap());
}

#[test]
fn evolution_rejects_energy_policy_instead_of_resetting_shifts_during_crossover() {
    assert!(EvolutionSession::new(
        &problem(),
        &energy_settings(),
        &EvolutionSettings::default(),
        &mut Toy::default()
    )
    .is_err());
}

#[test]
fn weight_moves_reuse_components_at_the_refined_shift() {
    let mut p = shifted_problem();
    p.structures[0].movable_atoms = Some(vec![]);
    let mut second = p.structures[0].clone();
    second.configuration = dimer(2.9);
    p.structures.push(second);
    let mut s = energy_settings();
    s.weight_move_probability = 1.;
    let mut calc = Toy::default();
    let mut run = RmcSession::new(&p, &s, &mut calc).unwrap();
    for attempt in 1..=7 {
        let calls = calc.calls;
        run.step(&mut calc).unwrap();
        if attempt % 3 != 0 {
            assert_eq!(calc.calls, calls);
        } else {
            assert!(calc.calls > calls);
        }
        let mut fixed = p.clone();
        fixed.structures = run.current().structures.clone();
        for (dataset, shift) in fixed.datasets.iter_mut().zip(&run.current().delta_e0) {
            dataset.exafs.delta_e0 = *shift;
        }
        let mut fixed_settings = s.clone();
        fixed_settings.energy_refinement = None;
        let direct = evaluate_ensemble(&fixed, &fixed_settings, &mut calc).unwrap();
        assert_eq!(direct.evaluation, run.current().evaluation);
    }
}

#[test]
fn calculator_rebase_preserves_energy_provenance_and_resumes() {
    struct Revised(Toy);
    impl ExafsCalculator for Revised {
        fn name(&self) -> &str {
            "revised energy test"
        }
        fn calculate(
            &mut self,
            c: &Configuration,
            a: usize,
            e: Edge,
            k: &[f64],
        ) -> Result<Vec<f64>, RmcError> {
            Ok(self
                .0
                .calculate(c, a, e, k)?
                .into_iter()
                .map(|v| v * 1.1)
                .collect())
        }
    }
    let p = shifted_problem();
    let mut calc = Toy::default();
    let mut run = RmcSession::new(&p, &energy_settings(), &mut calc).unwrap();
    run.step(&mut calc).unwrap();
    let before = run.current().delta_e0.clone();
    let audit = run.initial_energy().unwrap().clone();
    let mut revised = Revised(Toy::default());
    run.rebase(&mut revised).unwrap();
    assert_eq!(run.current().delta_e0, before);
    assert_eq!(run.initial_energy(), Some(&audit));
    let mut resumed = RmcSession::resume(run.checkpoint(), &mut revised).unwrap();
    run.run(&mut revised).unwrap();
    resumed.run(&mut revised).unwrap();
    assert_eq!(
        serde_json::to_value(run.checkpoint()).unwrap(),
        serde_json::to_value(resumed.checkpoint()).unwrap()
    );
    let mut bad = serde_json::to_value(run.checkpoint()).unwrap();
    bad["initial_energy"]["calculator"] = serde_json::json!("unknown source");
    assert!(RmcSession::resume(serde_json::from_value(bad).unwrap(), &mut revised).is_err());
}
