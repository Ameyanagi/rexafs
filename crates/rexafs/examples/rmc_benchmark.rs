//! Reproducible ReFEFF benchmark: identical prescribed trial geometries in every
//! mode, with cold setup separated from steady trial evaluation. Emits JSON to stdout.
//! Run: cargo run --release -p rexafs --features refeff-runner --example rmc_benchmark
use rexafs::rmc::*;
use rexafs::structure::Edge;
use serde_json::json;
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let steps = std::env::args()
        .nth(1)
        .map(|s| s.parse::<usize>())
        .transpose()?
        .unwrap_or(12);
    if steps == 0 || steps > 1000 {
        return Err("steps must be 1..=1000".into());
    }
    let height = 2.5 * 3f64.sqrt() / 2.;
    let configuration = Configuration {
        cell: None,
        atoms: vec![
            [0., 0., 0.],
            [2.5, 0., 0.],
            [1.25, height, 0.],
            [20., 0., 0.],
            [22.5, 0., 0.],
            [21.25, height, 0.],
        ]
        .into_iter()
        .map(|position| Atom {
            atomic_number: 29,
            position,
        })
        .collect(),
    };
    let k: Vec<_> = (0..121).map(|i| 3. + i as f64 * 0.05).collect();
    let problem: EnsembleProblem = RmcProblem {
        configuration: configuration.clone(),
        datasets: vec![ExafsDataset {
            name: "two Cu triangles".into(),
            absorbers: vec![0, 1, 3, 4],
            edge: Edge::K,
            chi: vec![0.; k.len()],
            sigma: vec![1.; k.len()],
            k,
            weight: 1.,
            kweight: 0,
            s02: 1.,
            delta_e0: 0.,
        }],
    }
    .into();
    let options = RefeffOptions {
        cluster_radius: 4.,
        path_radius: 4.,
        max_legs: 3,
        path_criteria: [0., 0.],
        kmax: 12.,
        ..Default::default()
    };
    let settings = SessionSettings {
        moves: RmcSettings {
            steps: 0,
            min_distance: 1.5,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut rows = Vec::new();
    let mut baseline: Option<Vec<Vec<f64>>> = None;
    for mode in ["full_uncached", "exact_local_cache", "frozen_potentials"] {
        let mut calculator = RefeffCalculator::new(options.clone())?;
        if mode == "full_uncached" {
            calculator.set_cache_capacity(0);
        }
        if mode == "frozen_potentials" {
            calculator = calculator.with_frozen_potentials(vec![configuration.clone()])?;
        }
        let start = Instant::now();
        let initial = evaluate_ensemble(&problem, &settings, &mut calculator)?;
        let setup = start.elapsed().as_secs_f64();
        let start = Instant::now();
        let mut spectra = Vec::new();
        let mut times = Vec::new();
        for i in 0..steps {
            let mut trial = problem.clone();
            // ±0.003, ±0.006, … Å, returning to the exact reference every sixth move.
            let offset = if (i + 1) % 6 == 0 {
                0.
            } else {
                0.003 * (i + 1) as f64 * if i % 2 == 0 { 1. } else { -1. }
            };
            trial.structures[0].configuration.atoms[1].position[0] += offset;
            let tick = Instant::now();
            let evaluated = evaluate_ensemble(&trial, &settings, &mut calculator)?;
            times.push(tick.elapsed().as_secs_f64());
            spectra.push(evaluated.evaluation.datasets[0].chi.clone());
        }
        let seconds = start.elapsed().as_secs_f64();
        let reference = baseline.as_ref().unwrap_or(&spectra);
        let mut squared = 0.;
        let mut reference_squared = 0.;
        let mut max_error = 0f64;
        let mut count = 0;
        for (a, b) in spectra.iter().zip(reference) {
            for (&x, &y) in a.iter().zip(b) {
                squared += (x - y).powi(2);
                reference_squared += y * y;
                max_error = max_error.max((x - y).abs());
                count += 1;
            }
        }
        let record = json!({"mode":mode,"setup_seconds":setup,"trial_seconds":seconds,"cold_total_seconds":setup+seconds,"trials_per_second":steps as f64/seconds,"per_trial_seconds":times,"chi_rmse_vs_full":(squared/count as f64).sqrt(),"chi_relative_l2_vs_full":(squared/reference_squared).sqrt(),"chi_max_abs_vs_full":max_error,"stats":calculator.stats(),"initial_score":initial.evaluation.score,"diagnostics":calculator.diagnostics()});
        eprintln!(
            "{mode}: {:.3} s setup, {:.3} s / {steps} trial evaluations, relative L2 error {:.6}",
            setup,
            seconds,
            (squared / reference_squared).sqrt()
        );
        rows.push(record);
        if baseline.is_none() {
            baseline = Some(spectra);
        }
    }
    println!(
        "{}",
        serde_json::to_string_pretty(
            &json!({"benchmark_version":1,"rexafs_version":env!("CARGO_PKG_VERSION"),"refeff_facade_version":"0.3.0","refeff_engine_version":"0.2.0","profile":"run with --release","trial_count":steps,"options":options,"problem":problem,"results":rows})
        )?
    );
    Ok(())
}
