//! Unreleased prepared-path Rust API demonstration, using synthetic Cu data.
//! Run with `cargo run --release --locked -p rexafs --features refeff-runner
//! --example rmc_prepared -- NEW_DIR`. No experimental files are required.
//! Demonstrates calibration at a known structure, seeded disorder, cooling,
//! exact checkpoint resume with cold caches, path/structural reports, transforms,
//! and streaming trajectory spectra. The dimer is an API fixture, not a material.
use rexafs::rmc::*;
use rexafs::structure::Edge;
use serde::Serialize;
use serde_json::json;
use std::{io::Cursor, ops::ControlFlow, path::PathBuf};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .ok_or("usage: rmc_prepared NEW_DIR")?,
    );
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
    let options = RefeffOptions {
        cluster_radius: 4.,
        path_radius: 3.5,
        max_legs: 2,
        path_criteria: [0., 0.],
        kmax: 12.,
        ..Default::default()
    };
    let acceleration = AccelerationSettings {
        catalogue: PathCatalogueSettings {
            radius: 3.5,
            max_legs: 2,
            displacement: 0.3,
            ..Default::default()
        },
        workers: 2,
        ..Default::default()
    };
    let calculator = || {
        PreparedRefeffCalculator::new(
            options.clone(),
            vec![reference.clone()],
            acceleration.clone(),
        )
    };
    let mut calc = calculator()?;
    let k: Vec<_> = (0..141).map(|i| 3. + 0.05 * i as f64).collect();
    let q: Vec<_> = k
        .iter()
        .map(|k| (k * k - rexafs::xafs::xafsutils::constants::ETOK * 2.).sqrt())
        .collect();
    let chi = calc
        .calculate(&reference, 0, Edge::K, &q)?
        .into_iter()
        .map(|x| 0.85 * x)
        .collect();
    let mut problem: EnsembleProblem = RmcProblem {
        configuration: reference.clone(),
        datasets: vec![ExafsDataset {
            name: "synthetic Cu dimer".into(),
            absorbers: vec![0],
            edge: Edge::K,
            k: k.clone(),
            chi,
            sigma: vec![1.; k.len()],
            weight: 1.,
            kweight: 2,
            s02: 1.,
            delta_e0: 0.,
        }],
    }
    .into();
    // Calibration uses the independently known generating geometry in this fixture.
    let calibration = calibrate_dataset(
        &problem,
        0,
        &CalibrationSettings {
            delta_e0: vec![0., 1., 2., 3., 4.],
            s02_bounds: [0.5, 1.2],
        },
        &mut calc,
    )?;
    problem.datasets[0].exafs.s02 = calibration.best.s02;
    problem.datasets[0].exafs.delta_e0 = calibration.best.delta_e0;
    problem.structures[0].configuration = seeded_disorder(&reference, &[1], 0.08, 42)?;
    let settings = SessionSettings {
        moves: RmcSettings {
            steps: 80,
            seed: 42,
            step_size: 0.025,
            temperature: 0.001,
            min_distance: 1.8,
            max_displacement: Some(0.15),
            movable_atoms: vec![1],
        },
        cooling: CoolingSchedule::Geometric {
            factor: 0.98,
            floor: 1e-5,
        },
        proposals: ProposalSettings {
            elements: vec![ElementProposal {
                element: 29,
                step_size: 0.025,
                weight: 1.,
            }],
            ..Default::default()
        },
        trajectory_stride: 10,
        ..Default::default()
    };
    let mut session = RmcSession::new(&problem, &settings, &mut calc)?;
    for _ in 0..20 {
        session.step(&mut calc)?;
    }
    session.save_checkpoint(out.join("checkpoint.json"))?;
    let mut calc = calculator()?;
    let mut session = RmcSession::load_checkpoint(out.join("checkpoint.json"), &mut calc)?;
    session.run(&mut calc)?;
    session.save_checkpoint(out.join("checkpoint.json"))?;
    let c = &session.best().structures[0].configuration;
    let paths = calc.path_reports(
        CalculationRequest {
            structure: 0,
            configuration: c,
            absorber: 0,
            edge: Edge::K,
            k: &q,
            options: None,
            paths: true,
        },
        2,
        true,
    )?;
    let map = LocalSpectrumTransform::new(
        &k,
        &LocalSpectrumSettings::morlet(WaveletSettings {
            k_centers: k.clone(),
            r: vec![1., 2., 3., 4.],
            omega0: 6.,
        }),
    )?;
    let weighted: Vec<_> = session.best().evaluation.datasets[0]
        .chi
        .iter()
        .zip(&k)
        .map(|(v, k)| v * k * k)
        .collect();
    let mut trajectory = String::new();
    for frame in session.trajectory() {
        trajectory.push_str(&frame.structures[0].configuration.to_xyz()?);
    }
    let mut frames = Vec::new();
    stream_xyz_spectra(
        Cursor::new(&trajectory),
        &problem,
        0,
        &settings,
        &mut calc,
        |i, state| {
            frames.push(json!({"frame":i,"score":state.evaluation.score}));
            Ok(ControlFlow::Continue(()))
        },
    )?;
    save(
        out.join("result.json"),
        &json!({"problem":problem,"settings":settings,"refeff":options,"acceleration":acceleration,"calibration":calibration,"initial":session.initial(),"best":session.best(),"diagnostics":session.diagnostics(),"fit":fit_report(&problem.datasets[0].exafs,&session.best().evaluation.datasets[0].chi,&problem.datasets[0].objective)?,"displacements":displacement_report(&reference,c,&[0,1],false)?,"paths":paths,"local_map":map.transform(&weighted)?,"streamed":frames,"stats":calc.stats()}),
    )?;
    std::fs::write(out.join("trajectory.xyz"), trajectory)?;
    std::fs::write(out.join("best.xyz"), c.to_xyz()?)?;
    println!(
        "S02 {:.6}, delta E0 {:.3} eV; score {:.6} -> {:.6}; {} trials",
        calibration.best.s02,
        calibration.best.delta_e0,
        session.initial().evaluation.score,
        session.best().evaluation.score,
        session.completed()
    );
    Ok(())
}
fn save(path: PathBuf, value: &impl Serialize) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::write(path, serde_json::to_vec_pretty(value)?)?;
    Ok(())
}
