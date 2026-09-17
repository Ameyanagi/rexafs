//! Experimental RMC command-line workflow using ReFEFF as its primary calculator.
//! Run `--demo OUTPUT [STEPS]` for a synthetic Cu dimer smoke test, or
//! `JOB.json OUTPUT` for explicit finite/periodic geometry and measured χ(k).
//! See doc/rmc.md. The output directory must not already exist.

use rexafs::rmc::{
    evaluate, refine_with_progress, Atom, Configuration, ExafsDataset, RefeffCalculator,
    RefeffOptions, RmcProblem, RmcSettings,
};
use rexafs::structure::Edge;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs;
use std::ops::ControlFlow;
use std::path::Path;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Job {
    problem: RmcProblem,
    settings: RmcSettings,
    refeff: RefeffOptions,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() < 2 || args.len() > 3 || (args[0] != "--demo" && args.len() != 2) {
        return Err("usage: rmc_refeff --demo NEW_OUTPUT_DIRECTORY [STEPS]\n       rmc_refeff JOB.json NEW_OUTPUT_DIRECTORY".into());
    }
    let output = Path::new(&args[1]);
    // Refuse overwrites before starting an expensive calculation.
    fs::create_dir(output)?;
    let job = if args[0] == "--demo" {
        let steps = args.get(2).map(|s| s.parse()).transpose()?.unwrap_or(12);
        synthetic_job(output, steps)?
    } else {
        serde_json::from_str(&fs::read_to_string(&args[0])?)?
    };
    fs::write(output.join("job.json"), serde_json::to_string_pretty(&job)?)?;
    let mut calculator = RefeffCalculator::new(job.refeff.clone())?;
    save_inputs(
        output,
        "initial",
        &job.problem.configuration,
        &job.problem.datasets,
        &calculator,
    )?;
    let started = std::time::Instant::now();
    let result = refine_with_progress(&job.problem, &job.settings, &mut calculator, |step| {
        eprintln!(
            "step {}: score={:.8e} best={:.8e} accepted={} constraint={}",
            step.step, step.score, step.best_score, step.accepted, step.constraint_rejected
        );
        ControlFlow::Continue(())
    })?;
    let elapsed = started.elapsed().as_secs_f64();
    let record = serde_json::json!({
        "format": "rexafs-rmc-result-v1", "rexafs_version": env!("CARGO_PKG_VERSION"),
        "refeff_options": job.refeff, "elapsed_refinement_seconds": elapsed,
        "diagnostics": calculator.diagnostics(), "result": result,
    });
    fs::write(
        output.join("result.json"),
        serde_json::to_string_pretty(&record)?,
    )?;
    fs::write(output.join("best.xyz"), result.best.configuration.to_xyz()?)?;
    fs::write(
        output.join("final.xyz"),
        result.final_state.configuration.to_xyz()?,
    )?;
    save_inputs(
        output,
        "best",
        &result.best.configuration,
        &job.problem.datasets,
        &calculator,
    )?;
    for (index, dataset) in job.problem.datasets.iter().enumerate() {
        let mut csv =
            String::from("k_inv_angstrom,observed_chi,sigma,initial_chi,best_chi,final_chi\n");
        for i in 0..dataset.k.len() {
            csv.push_str(&format!(
                "{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e}\n",
                dataset.k[i],
                dataset.chi[i],
                dataset.sigma[i],
                result.initial.evaluation.datasets[index].chi[i],
                result.best.evaluation.datasets[index].chi[i],
                result.final_state.evaluation.datasets[index].chi[i]
            ));
        }
        fs::write(output.join(format!("dataset-{index}.csv")), csv)?;
    }
    println!(
        "ReFEFF RMC: {:.8e} -> {:.8e}; {} accepted / {} attempted; {:.3} s\nResults: {}",
        result.initial.evaluation.score,
        result.best.evaluation.score,
        result.history.iter().filter(|s| s.accepted).count(),
        result.history.len(),
        elapsed,
        output.display()
    );
    for diagnostic in calculator.diagnostics() {
        eprintln!("ReFEFF: {diagnostic}");
    }
    Ok(())
}

fn save_inputs(
    output: &Path,
    prefix: &str,
    configuration: &Configuration,
    datasets: &[ExafsDataset],
    calculator: &RefeffCalculator,
) -> Result<(), Box<dyn Error>> {
    for (dataset, data) in datasets.iter().enumerate() {
        for &absorber in &data.absorbers {
            fs::write(
                output.join(format!("{prefix}-dataset-{dataset}-atom-{absorber}.inp")),
                calculator.input_for(configuration, absorber, data.edge)?,
            )?;
        }
    }
    Ok(())
}

fn synthetic_job(output: &Path, steps: usize) -> Result<Job, Box<dyn Error>> {
    // This artificial two-atom geometry checks software self-consistency only.
    // It is not a realistic Cu sample or independent experimental validation.
    let truth = Configuration {
        cell: None,
        atoms: vec![
            Atom {
                atomic_number: 29,
                position: [0.0, 0.0, 0.0],
            },
            Atom {
                atomic_number: 29,
                position: [2.5, 0.0, 0.0],
            },
        ],
    };
    let mut problem = RmcProblem {
        configuration: truth.clone(),
        datasets: vec![ExafsDataset {
            name: "synthetic Cu K edge; noise-free ReFEFF dimer".into(),
            absorbers: vec![0],
            edge: Edge::K,
            k: (0..61).map(|i| 3.0 + i as f64 * 0.1).collect(),
            chi: vec![0.0; 61],
            sigma: vec![0.01; 61],
            weight: 1.0,
            kweight: 0,
            s02: 1.0,
            delta_e0: 0.0,
        }],
    };
    let refeff = RefeffOptions {
        cluster_radius: 4.0,
        path_radius: 3.5,
        max_legs: 2,
        kmax: 12.0,
        ..Default::default()
    };
    let mut calculator = RefeffCalculator::new(refeff.clone())?;
    eprintln!("Generating noise-free synthetic target with ReFEFF...");
    let calculated = evaluate(&problem, &mut calculator)?;
    problem.datasets[0].chi = calculated.datasets[0].chi.clone();
    fs::write(
        output.join("synthetic-truth.json"),
        serde_json::to_string_pretty(&truth)?,
    )?;
    fs::write(output.join("synthetic-truth.xyz"), truth.to_xyz()?)?;
    fs::write(
        output.join("synthetic-diagnostics.json"),
        serde_json::to_string_pretty(calculator.diagnostics())?,
    )?;
    problem.configuration.atoms[1].position[0] = 2.7;
    let settings = RmcSettings {
        steps,
        seed: 42,
        step_size: 0.08,
        temperature: 0.0,
        min_distance: 1.8,
        max_displacement: Some(0.5),
        movable_atoms: vec![1],
    };
    Ok(Job {
        problem,
        settings,
        refeff,
    })
}
