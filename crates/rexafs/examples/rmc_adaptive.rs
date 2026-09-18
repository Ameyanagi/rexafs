//! Experimental, opt-in adaptive-basis audit and checkpoint example (unreleased).
//! `cargo run --release -p rexafs --features refeff-runner --example rmc_adaptive -- NEW_DIR`
//! Synthetic Cu dimer only: this checks the API, not a material fit or convergence.
//! Audits use the ordinary complex-R objective and exact paths at fixed potentials.
//! Exact caching is the recommended default; this example deliberately enables
//! the approximation to demonstrate auditing, not a qualified speedup.
use rexafs::{fitting::FeffFitTransform, rmc::*, structure::Edge};
use serde_json::json;
use std::path::PathBuf;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .ok_or("usage: rmc_adaptive NEW_DIR")?,
    );
    eprintln!("Experimental adaptive mode; exact caching is recommended for routine refinement.");
    std::fs::create_dir(&out)?;
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
    let k: Vec<_> = (0..121).map(|i| 2. + i as f64 * 0.05).collect();
    let mut calc = PreparedRefeffCalculator::new(
        RefeffOptions {
            cluster_radius: 3.5,
            path_radius: 3.,
            max_legs: 2,
            path_criteria: [0., 0.],
            kmax: 12.,
            ..Default::default()
        },
        vec![reference.clone()],
        AccelerationSettings {
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
                k: k.clone(),
                geometry_radius: 0.1,
                relative_error: 1e-8,
                ..Default::default()
            }),
            ..Default::default()
        },
    )?;
    // The electronic reference is also a training geometry, so this synthetic
    // target agrees with its exact typed paths. No energy shift is used here;
    // with nonzero delta_e0 the training grid must be the shifted theory grid.
    let chi = calc.calculate(&reference, 0, Edge::K, &k)?;
    let mut moved = reference;
    moved.atoms[1].position[0] += 0.04;
    let mut problem: EnsembleProblem = RmcProblem {
        configuration: moved,
        datasets: vec![ExafsDataset {
            name: "synthetic Cu dimer".into(),
            absorbers: vec![0],
            edge: Edge::K,
            sigma: vec![1.; k.len()],
            k,
            chi,
            weight: 1.,
            kweight: 2,
            s02: 1.,
            delta_e0: 0.,
        }],
    }
    .into();
    problem.datasets[0].objective = Objective::R(FeffFitTransform {
        kmin: 2.5,
        kmax: 7.5,
        rmin: 1.,
        rmax: 3.,
        kstep: Some(0.05),
        nfft: 1024,
        ..Default::default()
    });
    let settings = SessionSettings {
        moves: RmcSettings {
            steps: 30,
            seed: 43,
            movable_atoms: vec![1],
            min_distance: 1.,
            max_displacement: Some(0.1),
            step_size: 0.02,
            temperature: 0.,
        },
        ..Default::default()
    };
    let mut session = RmcSession::new(&problem, &settings, &mut calc)?;
    let initial = session.best().evaluation.score;
    let mut control = AdaptiveBasisController::new(
        AdaptiveAuditSettings {
            interval: 5,
            spectral_tolerance: 1e-5,
            score_tolerance: 1e-5,
            ..Default::default()
        },
        &calc,
    )?;
    for _ in 0..10 {
        control.step_rmc(&mut session, &mut calc)?;
    }
    let checkpoint = control.checkpoint_rmc(&session, &calc)?;
    checkpoint.save(out.join("midpoint.json"))?;
    let checkpoint: AdaptiveRmcCheckpoint =
        serde_json::from_slice(&std::fs::read(out.join("midpoint.json"))?)?;
    let (mut session, mut calc, mut control) = checkpoint.resume()?;
    loop {
        match control.step_rmc(&mut session, &mut calc) {
            Ok(Some(_)) => (),
            Ok(None) => break,
            Err(error) => {
                control
                    .checkpoint_rmc(&session, &calc)?
                    .save(out.join("recovery.json"))?;
                return Err(error.into());
            }
        }
    }
    control.audit_rmc(&mut session, &mut calc)?;
    control
        .checkpoint_rmc(&session, &calc)?
        .save(out.join("final.json"))?;
    let report = json!({"experimental":true,"note":"Experimental adaptive API demonstration; exact caching is recommended. No physical convergence or speedup claim.",
        "initial_score_before_audit":initial,"best_score":session.best().evaluation.score,
        "attempts":session.completed(),"exact_fallback":control.uses_exact_paths(),
        "audits":control.reports(),"revisions":session.revisions(),"stage_stats":calc.stats()});
    std::fs::write(out.join("report.json"), serde_json::to_vec_pretty(&report)?)?;
    println!("{report:#}");
    Ok(())
}
