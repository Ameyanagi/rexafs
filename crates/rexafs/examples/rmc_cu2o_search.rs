//! Unreleased local Cu₂O comparison of Metropolis and evolutionary search.
//!
//! `rmc_cu2o_search rmc|ea|hybrid SOURCE_JOB ACCELERATION SECONDS NEW_DIR SEED` retains the
//! source data/model/constraints and starts from its original configuration.
//! Both searches minimize the experimental-normalized complex R residual over
//! 0.8–4 Å, with the existing REXAFS fitter's real/imaginary transform convention.
//! The k² Hanning window tapers over 1 Å⁻¹ within the measured support. No new
//! uncertainty estimate is introduced: this example requires sigma=1 throughout.
//! A constant dataset weight normalizes the objective by experimental R power;
//! the Metropolis tolerance is rescaled from the source k objective to retain
//! its experimental-normalized value. EA does not use that tolerance.
//! Electronic preparation is timed separately. The search budget includes EA
//! population initialization and commits whole generations, so EA may overrun
//! by one generation. Checkpoints retain the exact random stream and population.
//! Final scores are cross-checked against the standard fitter's residual vector.
//! `inspect R_JOB STATE OUTPUT_JSON` exports the existing fitter's transformed
//! plot arrays, windowed residuals and normalized score without scattering.
//! One seed/time-limited comparison cannot establish general optimizer superiority.
use nalgebra::DVector;
use rexafs::fitting::{
    transform::{apply_dataset_transform, residual_for_dataset},
    FeffFitTransform,
};
use rexafs::rmc::*;
use rexafs::xafs::xafsutils::FTWindow;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{path::Path, time::Instant};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Job {
    problem: EnsembleProblem,
    settings: SessionSettings,
    refeff: RefeffOptions,
    frozen_references: Vec<Configuration>,
    provenance: serde_json::Value,
}
fn write(path: impl AsRef<Path>, value: &impl Serialize) -> Result<()> {
    std::fs::write(path, serde_json::to_vec_pretty(value)?)?;
    Ok(())
}
fn fitter_view(job: &Job, state: &EnsembleState) -> Result<serde_json::Value> {
    let d = &job.problem.datasets[0];
    let Objective::R(t) = &d.objective else {
        return Err("expected R objective".into());
    };
    let count = (d.exafs.k.last().ok_or("empty k")? / 0.05).round() as usize + 1;
    let grid = DVector::from_iterator(count, (0..count).map(|i| i as f64 * 0.05));
    let padded = |chi: &[f64]| -> Result<DVector<f64>> {
        let mut result = DVector::zeros(count);
        for (&k, &chi) in d.exafs.k.iter().zip(chi) {
            let i = (k / 0.05).round() as usize;
            if (grid[i] - k).abs() > 1e-9 {
                return Err("requires 0.05 Å^-1 grid".into());
            }
            result[i] = chi;
        }
        Ok(result)
    };
    let data = apply_dataset_transform(&grid, &padded(&d.exafs.chi)?, t)?;
    let model = apply_dataset_transform(&grid, &padded(&state.evaluation.datasets[0].chi)?, t)?;
    let residual = residual_for_dataset(&data, Some(&model), t, &[1.])?;
    let baseline = residual_for_dataset(&data, None, t, &[1.])?;
    let score = residual.norm_squared() / baseline.norm_squared();
    let a = &data.primary().r_space;
    let b = &model.primary().r_space;
    Ok(
        json!({"score":score,"objective_score":d.objective.score(&d.exafs,&state.evaluation.datasets[0].chi)?,
        "k":d.exafs.k,"chi_experiment":d.exafs.chi,"chi_model":state.evaluation.datasets[0].chi,
        "r":a.r.as_slice(),"experimental_real":a.chir_re.as_slice(),"experimental_imag":a.chir_im.as_slice(),"experimental_magnitude":a.chir_mag.as_slice(),
        "model_real":b.chir_re.as_slice(),"model_imag":b.chir_im.as_slice(),"model_magnitude":b.chir_mag.as_slice(),
        "fit_mask":a.mask_indices,"fit_residual_real_imag":residual.as_slice(),"data_only_real_imag":baseline.as_slice()}),
    )
}
fn fitter_ratio(job: &Job, state: &EnsembleState) -> Result<f64> {
    Ok(fitter_view(job, state)?["score"]
        .as_f64()
        .ok_or("invalid fitter score")?)
}
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() == 4 && args[0] == "inspect" {
        let job: Job = serde_json::from_slice(&std::fs::read(&args[1])?)?;
        let state: EnsembleState = serde_json::from_slice(&std::fs::read(&args[2])?)?;
        return write(&args[3], &fitter_view(&job, &state)?);
    }
    if args.len() != 6 || !matches!(args[0].as_str(), "rmc" | "ea" | "hybrid") {
        return Err(
            "usage: rmc_cu2o_search rmc|ea|hybrid SOURCE_JOB ACCELERATION SECONDS NEW_DIR SEED"
                .into(),
        );
    }
    let mode = &args[0];
    let budget: f64 = args[3].parse()?;
    // The final argument is an explicit seed, allowing repetitions without editing data.
    let seed: u64 = args[5].parse()?;
    if !budget.is_finite() || !(1. ..=86400.).contains(&budget) {
        return Err("invalid budget".into());
    }
    let out = Path::new(&args[4]);
    let mut job: Job = serde_json::from_slice(&std::fs::read(&args[1])?)?;
    let mut acceleration: AccelerationSettings = serde_json::from_slice(&std::fs::read(&args[2])?)?;
    if job.problem.datasets.len() != 1 || job.problem.structures.len() != 1 {
        return Err("this comparison requires one dataset and one structure".into());
    }
    let d = &mut job.problem.datasets[0];
    if d.exafs.sigma.iter().any(|s| *s != 1.)
        || d.exafs.kweight != 2
        || !matches!(d.objective, Objective::K)
    {
        return Err("this comparison requires a source K objective, kweight=2 and sigma=1".into());
    }
    let zero = vec![0.; d.exafs.k.len()];
    let k_power = Objective::K.score(&d.exafs, &zero)?;
    job.settings.moves.temperature /= k_power;
    job.settings.moves.seed = seed;
    job.settings.moves.steps = 1_000_000;
    let t = FeffFitTransform {
        kmin: d.exafs.k[0] + 0.5,
        kmax: d.exafs.k[d.exafs.k.len() - 1] - 0.5,
        kweight: 2.,
        dk: 1.,
        dk2: Some(1.),
        window: FTWindow::Hanning,
        rmin: 0.8,
        rmax: 4.,
        dr: 0.,
        dr2: Some(0.),
        kstep: Some(0.05),
        nfft: 2048,
        ..Default::default()
    };
    d.objective = Objective::R(t);
    d.exafs.weight = 1.;
    let r_power = d.objective.score(&d.exafs, &zero)?;
    if !r_power.is_finite() || r_power <= 0. {
        return Err("zero experimental R power".into());
    }
    d.exafs.weight = 1. / r_power;
    let evolution = EvolutionSettings {
        generations: 100_000,
        local_steps: if mode == "hybrid" { 8 } else { 0 },
        ..Default::default()
    };
    if mode == "hybrid" {
        job.settings.adaptation = Some(StepAdaptation::default());
        acceleration.snapshots_per_context = 2 * evolution.population + 1;
    }
    job.provenance = json!({"source_job":args[1],"source_provenance":job.provenance,
        "objective":"normalized sum of squared real and imaginary R residuals",
        "r_range_A":[0.8,4.0],"k_power":k_power,"r_power":r_power,
        "temperature_policy":"source temperature / source experimental k power",
        "search":mode,"budget_seconds":budget,"seed":seed});
    std::fs::create_dir(out)?;
    write(out.join("job.json"), &job)?;
    write(out.join("acceleration.json"), &acceleration)?;
    write(out.join("evolution-settings.json"), &evolution)?;
    let mut calc = PreparedRefeffCalculator::new(
        job.refeff.clone(),
        job.frozen_references.clone(),
        acceleration,
    )?;
    let setup = Instant::now();
    let initial = evaluate_ensemble(&job.problem, &job.settings, &mut calc)?;
    let setup_seconds = setup.elapsed().as_secs_f64();
    let initial_check = fitter_ratio(&job, &initial)?;
    if (initial_check - initial.evaluation.score).abs() > 1e-10 * initial_check.abs().max(1.) {
        return Err("initial R objective differs from standard fitter".into());
    }
    write(out.join("initial.json"), &initial)?;
    let baseline_stats = calc.stats();
    let absorbers_per_candidate = baseline_stats.requests;
    if absorbers_per_candidate == 0 {
        return Err("no absorber evaluations".into());
    }
    println!(
        "{mode}: setup {setup_seconds:.3}s; initial normalized R residual {initial_check:.8}; T {}",
        job.settings.moves.temperature
    );
    let start = Instant::now();
    let mut trace = Vec::new();
    let evolution_trend = ResidualTrendSettings {
        window: 10,
        minimum_attempts: 60,
        ..Default::default()
    };
    let best;
    let completed;
    if mode == "rmc" {
        let mut session = RmcSession::new(&job.problem, &job.settings, &mut calc)?;
        loop {
            if start.elapsed().as_secs_f64() >= budget {
                break;
            }
            let step = match session.step(&mut calc) {
                Ok(Some(step)) => step,
                Ok(None) => break,
                Err(e) => {
                    session.save_checkpoint(out.join("checkpoint.json"))?;
                    return Err(e.into());
                }
            };
            if step.step % 20 == 0 {
                trace.push(json!({"step":step.step,"seconds":start.elapsed().as_secs_f64(),"best":step.best_score,"current":step.score,"candidate_evaluations":(calc.stats().requests-baseline_stats.requests)/absorbers_per_candidate}));
                session.save_checkpoint(out.join("checkpoint.json"))?;
                write(out.join("trace.json"), &trace)?;
                write(out.join("best.json"), session.best())?;
                write(
                    out.join("convergence.json"),
                    &residual_trend(session.history(), &ResidualTrendSettings::default())?,
                )?;
                if step.step % 100 == 0 {
                    println!(
                        "rmc step {} best {:.8} {:.1}s",
                        step.step,
                        step.best_score,
                        start.elapsed().as_secs_f64()
                    );
                }
            }
        }
        session.save_checkpoint(out.join("checkpoint.json"))?;
        write(out.join("history.json"), &session.history())?;
        write(out.join("final.json"), session.current())?;
        write(
            out.join("convergence.json"),
            &residual_trend(session.history(), &ResidualTrendSettings::default())?,
        )?;
        completed = session.completed();
        best = session.best().clone();
    } else {
        let mut session =
            EvolutionSession::new(&job.problem, &job.settings, &evolution, &mut calc)?;
        loop {
            if start.elapsed().as_secs_f64() >= budget {
                break;
            }
            let record = match session.step(&mut calc) {
                Ok(Some(record)) => record,
                Ok(None) => break,
                Err(e) => {
                    session.save_checkpoint(out.join("checkpoint.json"))?;
                    return Err(e.into());
                }
            };
            let mean = session
                .population()
                .iter()
                .map(|s| s.evaluation.score)
                .sum::<f64>()
                / session.population().len() as f64;
            trace.push(json!({"generation":record.generation,"seconds":start.elapsed().as_secs_f64(),"best":record.best_score,"mean":mean,"diversity_A":record.diversity,"hypermutation":record.hypermutation,"constraint_fallbacks":record.constraint_fallbacks,"local_attempts":record.local_attempts,"local_accepted":record.local_accepted,"local_constraint_rejected":record.local_constraint_rejected,"step_scale":record.step_scale,"candidate_evaluations":(calc.stats().requests-baseline_stats.requests)/absorbers_per_candidate}));
            session.save_checkpoint(out.join("checkpoint.json"))?;
            write(out.join("trace.json"), &trace)?;
            write(
                out.join("convergence.json"),
                &evolution_residual_trend(session.history(), &evolution_trend)?,
            )?;
            write(out.join("best.json"), session.best())?;
            println!(
                "{mode} generation {} best {:.8} mean {mean:.8} diversity {:.5} fallback {} {:.1}s",
                record.generation,
                record.best_score,
                record.diversity,
                record.constraint_fallbacks,
                start.elapsed().as_secs_f64()
            );
        }
        completed = session.completed();
        best = session.best().clone();
        session.save_checkpoint(out.join("checkpoint.json"))?;
        write(out.join("history.json"), &session.history())?;
        write(out.join("population.json"), &session.population())?;
        write(
            out.join("convergence.json"),
            &evolution_residual_trend(session.history(), &evolution_trend)?,
        )?;
        write(out.join("final.json"), &best)?;
    }
    let search_seconds = start.elapsed().as_secs_f64();
    let convergence: serde_json::Value =
        serde_json::from_slice(&std::fs::read(out.join("convergence.json"))?)?;
    let check = fitter_ratio(&job, &best)?;
    if (check - best.evaluation.score).abs() > 1e-10 * check.abs().max(1.) {
        return Err("best R objective differs from standard fitter".into());
    }
    write(out.join("best.json"), &best)?;
    std::fs::write(
        out.join("best.xyz"),
        best.structures[0].configuration.to_xyz()?,
    )?;
    write(
        out.join("summary.json"),
        &json!({"mode":mode,"seed":seed,"completed":completed,"budget_seconds":budget,"search_seconds":search_seconds,"setup_seconds":setup_seconds,"stop_reason":"WallTimeBudget","best_score":best.evaluation.score,"initial_score":initial.evaluation.score,"standard_fitter_ratio":check,"stats":calc.stats(),"setup_stats":baseline_stats,"candidate_evaluations_including_population_initialization":(calc.stats().requests-baseline_stats.requests)/absorbers_per_candidate,"convergence":convergence}),
    )?;
    println!("{mode} finished {completed}; best {check:.8}; {search_seconds:.3}s");
    Ok(())
}
