//! Rust API example: fixed-potential ReFEFF RMC with constraints, trajectory,
//! path reports and an exact-resume checkpoint. Generates synthetic Cu data.
//! Run: cargo run --release -p rexafs --features refeff-runner --example rmc_session -- NEW_DIR
use rexafs::rmc::*;
use rexafs::structure::Edge;
use std::path::PathBuf;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let directory = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .ok_or("usage: rmc_session NEW_DIR")?,
    );
    std::fs::create_dir(&directory)?;
    let mut configuration = Configuration {
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
        kmax: 12.,
        ..Default::default()
    };
    let reference = configuration.clone();
    let mut calculator =
        RefeffCalculator::new(options.clone())?.with_frozen_potentials(vec![reference.clone()])?;
    let k: Vec<_> = (0..121).map(|i| 3. + i as f64 * 0.05).collect();
    let chi = calculator.calculate(&configuration, 0, Edge::K, &k)?;
    configuration.atoms[1].position[0] = 2.65;
    let mut problem: EnsembleProblem = RmcProblem {
        configuration,
        datasets: vec![ExafsDataset {
            name: "synthetic Cu".into(),
            absorbers: vec![0],
            edge: Edge::K,
            sigma: vec![0.01; k.len()],
            k,
            chi,
            weight: 1.,
            kweight: 0,
            s02: 1.,
            delta_e0: 0.,
        }],
    }
    .into();
    // Each dataset can override polarization, radii and path filters independently.
    problem.datasets[0].refeff = Some(options.clone());
    let settings = SessionSettings {
        moves: RmcSettings {
            steps: 64,
            seed: 42,
            step_size: 0.06,
            temperature: 0.,
            min_distance: 1.8,
            movable_atoms: vec![1],
            ..Default::default()
        },
        constraints: Constraints {
            pairs: vec![PairDistance {
                elements: [29, 29],
                minimum: 2.,
            }],
            ..Default::default()
        },
        trajectory_stride: 4,
        retain_paths: true,
        ..Default::default()
    };
    std::fs::write(
        directory.join("job.json"),
        serde_json::to_vec_pretty(
            &serde_json::json!({"problem":problem,"settings":settings,"refeff":options,"frozen_references":[reference]}),
        )?,
    )?;
    let mut session = RmcSession::new(&problem, &settings, &mut calculator)?;
    for _ in 0..16 {
        session.step(&mut calculator)?;
    }
    session.save_checkpoint(directory.join("checkpoint.json"))?;
    let mut session =
        RmcSession::load_checkpoint(directory.join("checkpoint.json"), &mut calculator)?;
    if let Err(error) = session.run(&mut calculator) {
        session.save_checkpoint(directory.join("checkpoint.json"))?;
        return Err(error.into());
    }
    session.save_checkpoint(directory.join("checkpoint.json"))?;
    std::fs::write(
        directory.join("best.json"),
        serde_json::to_vec_pretty(session.best())?,
    )?;
    std::fs::write(
        directory.join("best.xyz"),
        session.best().structures[0].configuration.to_xyz()?,
    )?;
    let mut xyz = String::new();
    for frame in session.trajectory() {
        xyz.push_str(&frame.structures[0].configuration.to_xyz()?);
    }
    std::fs::write(directory.join("trajectory.xyz"), xyz)?;
    let edges: Vec<_> = (0..101).map(|i| i as f64 * 0.05).collect();
    let rdf = distance_distribution(
        &session.best().structures[0].configuration,
        &[0],
        Some(29),
        &edges,
    )?;
    std::fs::write(
        directory.join("distances.json"),
        serde_json::to_vec_pretty(&rdf)?,
    )?;
    std::fs::write(
        directory.join("calculator.json"),
        serde_json::to_vec_pretty(
            &serde_json::json!({"identity":calculator.identity(),"stats":calculator.stats(),"diagnostics":calculator.diagnostics()}),
        )?,
    )?;
    println!(
        "score {:.6} -> {:.6}; {} steps; {:?}",
        session.initial().evaluation.score,
        session.best().evaluation.score,
        session.completed(),
        calculator.stats()
    );
    Ok(())
}
