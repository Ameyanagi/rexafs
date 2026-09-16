//! Unreleased experimental Cu₂O RMC example using a local Athena measurement.
//!
//! `prepare PROJECT NEW_DIR` extracts `cu2o_abs`, preserves its processing
//! parameters, with an explicit Hanning background window, and calculates a crystalline
//! ReFEFF reference. `run JOB_JSON NEW_DIR` runs a reviewable JSON job with
//! checkpoints. `check JOB_JSON BEST_JSON NEW_DIR` recalculates a saved state
//! with fresh and pinned potentials, timing reference setup separately from a
//! warm geometry update. `prepare`, `run` and `check` require new output
//! directories. The project is read-only. Settings are starting choices, not
//! a validated Cu₂O protocol.
//! `curves JOB_JSON RUN_DIR` exports the experimental, initial, best and last
//! spectra and their common k²-weighted Fourier transforms to `curves.json`.
//! The Fourier window tapers over 1 Å⁻¹ inside each end of measured support.
//! `run-prepared JOB_JSON ACCELERATION_JSON NEW_DIR` uses prepared paths with
//! the same optimizer and output files. The job must explicitly select `[0,0]`
//! path criteria and match the acceleration radius/order. No input is rewritten.
//! `check-prepared JOB_JSON STATE_JSON ACCELERATION_JSON NEW_DIR` adds direct
//! typed-path and cold-cache consistency checks to the full/pinned comparison.
//! `resume-prepared JOB_JSON ACCELERATION_JSON CHECKPOINT_JSON TOTAL_STEPS NEW_DIR`
//! continues the exact stored RNG/coordinates with an increased attempt budget.
//! Setup and sampling times cover this invocation; counters include prior attempts.
//! Curve export replaces that derived file in the existing run directory.
use nalgebra::DVector;
use rexafs::io::AthenaProject;
use rexafs::rmc::*;
use rexafs::structure::{BuiltinLibrary, Edge};
use rexafs::xafs::xafsutils::FTWindow;
use rexafs::XrayFFTF;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::time::Instant;

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

fn prepare(project: &Path, out: &Path) -> Result<()> {
    let input = AthenaProject::read(project)?;
    let mut group = input
        .group_by_label("cu2o_abs")
        .ok_or("no cu2o_abs group")?
        .clone();
    // Athena 0.8 uses *_win; the typed reader uses modern *_kwindow names.
    group.params.bkg_kwindow = group
        .params
        .bkg_kwindow
        .clone()
        .or_else(|| group.arg_str("bkg_win").map(str::to_owned));
    group.params.fft_kwindow = group
        .params
        .fft_kwindow
        .clone()
        .or_else(|| group.arg_str("fft_win").map(str::to_owned));
    // The archived Kaiser–Bessel shape parameter is zero. This is singular in
    // rexafs's normalized Kaiser implementation; use and record a finite taper.
    group.params.bkg_kwindow = Some("hanning".into());
    group.params.bkg_dk = Some(1.);
    if group.params.eshift.unwrap_or(0.) != 0. {
        return Err("this example requires an unshifted source energy axis".into());
    }
    let mut spectrum = group.to_spectrum()?;
    spectrum.calc_background()?;
    let k = spectrum.k().ok_or("background did not produce k")?.to_vec();
    let chi = spectrum
        .chi()
        .ok_or("background did not produce chi")?
        .to_vec();
    let library = BuiltinLibrary::get()?;
    let configuration =
        Configuration::from_structure(&library.structure("cu2o_cuprite")?, [2, 2, 2])?;
    let absorbers: Vec<_> = configuration
        .atoms
        .iter()
        .enumerate()
        .filter_map(|(i, a)| (a.atomic_number == 29).then_some(i))
        .collect();
    let options = RefeffOptions {
        cluster_radius: 6.,
        path_radius: 4.5,
        max_legs: 4,
        kmax: 16.,
        ..Default::default()
    };
    let provenance = json!({
        "source":project, "sha256":Sha256::digest(std::fs::read(project)?).iter().map(|b|format!("{b:02x}")).collect::<String>(),
        "group":group.label, "tag":group.tag, "athena_args":group.args.iter()
            .map(|(k,v)|(k.clone(),v.to_string())).collect::<Vec<_>>(),
        "normalization":spectrum.normalization, "background":spectrum.background,
        "structure":library.entry("cu2o_cuprite"),
        "note":"Reprocessed in rexafs; no assertion of numerical identity with Athena. Experimental uncertainty is not provided."
    });
    std::fs::create_dir(out)?;
    write(
        out.join("experimental.json"),
        &json!({"energy":group.x,"mu":group.y,"k":k,"chi":chi,"provenance":provenance}),
    )?;
    let selected: Vec<_> = k
        .iter()
        .enumerate()
        .filter_map(|(i, &v)| (2.5..=12.).contains(&v).then_some(i))
        .collect();
    let data = ExafsDataset {
        name: "Cu2O experimental Cu K edge".into(),
        absorbers,
        edge: Edge::K,
        k: selected.iter().map(|&i| k[i]).collect(),
        chi: selected.iter().map(|&i| chi[i]).collect(),
        sigma: vec![1.; selected.len()],
        weight: 1.,
        kweight: 2,
        s02: 0.9,
        delta_e0: 0.,
    };
    let problem: EnsembleProblem = RmcProblem {
        configuration: configuration.clone(),
        datasets: vec![data],
    }
    .into();
    let settings = SessionSettings {
        moves: RmcSettings {
            steps: 1000,
            seed: 20260916,
            step_size: 0.035,
            temperature: 0.0002,
            min_distance: 1.4,
            max_displacement: Some(0.4),
            movable_atoms: (1..configuration.atoms.len()).collect(),
        },
        constraints: Constraints {
            pairs: vec![
                PairDistance {
                    elements: [29, 8],
                    minimum: 1.5,
                },
                PairDistance {
                    elements: [29, 29],
                    minimum: 2.35,
                },
                PairDistance {
                    elements: [8, 8],
                    minimum: 2.7,
                },
            ],
            ..Default::default()
        },
        trajectory_stride: 20,
        ..Default::default()
    };
    let job = Job {
        problem,
        settings,
        refeff: options.clone(),
        frozen_references: vec![configuration.clone()],
        provenance,
    };
    write(out.join("job-template.json"), &job)?;
    let mut calc = RefeffCalculator::new(options)?;
    let grid: Vec<_> = (0..311).map(|i| 0.5 + i as f64 * 0.05).collect();
    let start = Instant::now();
    let calculated = calc.calculate_request(CalculationRequest {
        structure: 0,
        configuration: &configuration,
        absorber: job.problem.datasets[0].exafs.absorbers[0],
        edge: Edge::K,
        k: &grid,
        options: None,
        paths: true,
    })?;
    write(
        out.join("crystal-reference.json"),
        &json!({"k":grid,"calculated":calculated,"seconds":start.elapsed().as_secs_f64(),"stats":calc.stats(),"diagnostics":calc.diagnostics()}),
    )?;
    println!(
        "prepared {} atoms; {} Cu absorbers; reference {:.3} s",
        configuration.atoms.len(),
        job.problem.datasets[0].exafs.absorbers.len(),
        start.elapsed().as_secs_f64()
    );
    Ok(())
}

// Dispatch keeps the same optimizer and output schema for both calculators.
enum Calculator {
    Pipeline(RefeffCalculator),
    Prepared(PreparedRefeffCalculator),
}
impl Calculator {
    fn stats(&self) -> serde_json::Value {
        match self {
            Self::Pipeline(c) => json!(c.stats()),
            Self::Prepared(c) => json!(c.stats()),
        }
    }
    fn diagnostics(&self) -> serde_json::Value {
        match self {
            Self::Pipeline(c) => json!(c.diagnostics()),
            Self::Prepared(_) => {
                json!({"note":"Fixed electronic reference and catalogue; inspect direct/fresh reference checks."})
            }
        }
    }
}
impl ExafsCalculator for Calculator {
    fn name(&self) -> &str {
        match self {
            Self::Pipeline(c) => c.name(),
            Self::Prepared(c) => c.name(),
        }
    }
    fn identity(&self) -> String {
        match self {
            Self::Pipeline(c) => c.identity(),
            Self::Prepared(c) => c.identity(),
        }
    }
    fn calculate(
        &mut self,
        c: &Configuration,
        a: usize,
        e: Edge,
        k: &[f64],
    ) -> std::result::Result<Vec<f64>, RmcError> {
        match self {
            Self::Pipeline(v) => v.calculate(c, a, e, k),
            Self::Prepared(v) => v.calculate(c, a, e, k),
        }
    }
    fn calculate_batch(
        &mut self,
        r: &[CalculationRequest<'_>],
    ) -> std::result::Result<Vec<CalculatedSpectrum>, RmcError> {
        match self {
            Self::Pipeline(v) => v.calculate_batch(r),
            Self::Prepared(v) => v.calculate_batch(r),
        }
    }
}

fn run(
    job_path: &Path,
    out: &Path,
    acceleration: Option<&Path>,
    resume: Option<(&Path, usize)>,
) -> Result<()> {
    let mut job: Job = serde_json::from_slice(&std::fs::read(job_path)?)?;
    std::fs::create_dir(out)?;
    let mut calc = if let Some(path) = acceleration {
        let settings: AccelerationSettings = serde_json::from_slice(&std::fs::read(path)?)?;
        write(out.join("acceleration.json"), &settings)?;
        Calculator::Prepared(PreparedRefeffCalculator::new(
            job.refeff.clone(),
            job.frozen_references.clone(),
            settings,
        )?)
    } else {
        let mut c = RefeffCalculator::new(job.refeff.clone())?
            .with_frozen_potentials(job.frozen_references.clone())?;
        c.set_cache_capacity(256 * 1024 * 1024);
        Calculator::Pipeline(c)
    };
    let start = Instant::now();
    let mut session = if let Some((checkpoint_path, total)) = resume {
        // Ensure the caller's job describes the checkpoint being continued.
        let bytes = std::fs::read(checkpoint_path)?;
        let value: serde_json::Value = serde_json::from_slice(&bytes)?;
        let checkpoint_problem: EnsembleProblem = serde_json::from_value(value["problem"].clone())?;
        let checkpoint_settings: SessionSettings =
            serde_json::from_value(value["settings"].clone())?;
        if checkpoint_problem != job.problem || checkpoint_settings != job.settings {
            return Err("resume job differs from checkpoint problem/settings".into());
        }
        let mut session = RmcSession::resume(serde_json::from_slice(&bytes)?, &mut calc)?;
        session.set_step_limit(total)?;
        job.settings.moves.steps = total;
        write(
            out.join("resume.json"),
            &json!({"source":checkpoint_path,"completed_before":session.completed(),"new_step_limit":total}),
        )?;
        session
    } else {
        RmcSession::new(&job.problem, &job.settings, &mut calc)?
    };
    let completed_before = session.completed();
    write(out.join("job.json"), &job)?;
    write(out.join("initial.json"), session.initial())?;
    session.save_checkpoint(out.join("checkpoint.json"))?;
    println!(
        "initial score {:.8}; setup {:.3} s",
        session.initial().evaluation.score,
        start.elapsed().as_secs_f64()
    );
    let setup_seconds = start.elapsed().as_secs_f64();
    let sampling_start = Instant::now();
    let mut uphill = session.diagnostics().uphill;
    let mut accepted = session.diagnostics().accepted;
    loop {
        let previous = session.current().evaluation.score;
        let result = match session.step(&mut calc) {
            Ok(step) => step,
            Err(e) => {
                session.save_checkpoint(out.join("checkpoint.json"))?;
                return Err(e.into());
            }
        };
        let Some(step) = result else { break };
        if step.accepted {
            accepted += 1;
            if step.score > previous {
                uphill += 1;
            }
        }
        if step.step % 20 == 0 || step.step == job.settings.moves.steps {
            let trend = residual_trend(session.history(), &ResidualTrendSettings::default())?;
            write(out.join("convergence.json"), &trend)?;
            if step.step % 500 == 0 {
                println!(
                    "residual trend {:?}: {:?}",
                    trend.status,
                    trend
                        .windows
                        .last()
                        .map(|w| (w.relative_best_improvement, w.relative_mean_change))
                );
            }
            session.save_checkpoint(out.join("checkpoint.json"))?;
            write(out.join("best.json"), session.best())?;
            write(out.join("final.json"), session.current())?;
            write(
                out.join("progress.json"),
                &json!({"step":step.step,"score":step.score,"best_score":step.best_score,"accepted":accepted,"uphill_accepted":uphill,"seconds":start.elapsed().as_secs_f64(),"stats":calc.stats()}),
            )?;
            println!(
                "step {} score {:.8} best {:.8} accepted {} uphill {} elapsed {:.1} s",
                step.step,
                step.score,
                step.best_score,
                accepted,
                uphill,
                start.elapsed().as_secs_f64()
            );
        }
    }
    write(
        out.join("convergence.json"),
        &residual_trend(session.history(), &ResidualTrendSettings::default())?,
    )?;
    session.save_checkpoint(out.join("checkpoint.json"))?;
    write(out.join("best.json"), session.best())?;
    write(out.join("final.json"), session.current())?;
    write(out.join("history.json"), &session.history())?;
    write(out.join("trajectory.json"), &session.trajectory())?;
    for (name, state) in [
        ("initial", session.initial()),
        ("best", session.best()),
        ("final", session.current()),
    ] {
        std::fs::write(
            out.join(format!("{name}.xyz")),
            state.structures[0].configuration.to_xyz()?,
        )?;
    }
    let edges: Vec<_> = (0..=250).map(|i| i as f64 * 0.02).collect();
    let absorbers = &job.problem.datasets[0].exafs.absorbers;
    let mut distributions = Vec::new();
    for (name, state) in [
        ("initial", session.initial()),
        ("best", session.best()),
        ("final", session.current()),
    ] {
        for element in [8, 29] {
            distributions.push(json!({"state":name,"neighbor_z":element,"distribution":distance_distribution(&state.structures[0].configuration,absorbers,Some(element),&edges)?}));
        }
    }
    write(out.join("distances.json"), &distributions)?;
    write(
        out.join("summary.json"),
        &json!({"backend":calc.identity(),"refeff_version":"0.4.0","setup_seconds":setup_seconds,"sampling_seconds":sampling_start.elapsed().as_secs_f64(),"completed_before":completed_before,"attempts_this_invocation":session.completed()-completed_before,"stop_reason":session.stop_reason(),"steps":session.completed(),"initial_score":session.initial().evaluation.score,"best_score":session.best().evaluation.score,"final_score":session.current().evaluation.score,"accepted":accepted,"uphill_accepted":uphill,"seconds":start.elapsed().as_secs_f64(),"stats":calc.stats(),"diagnostics":calc.diagnostics()}),
    )?;
    Ok(())
}

fn check(job_path: &Path, state_path: &Path, out: &Path) -> Result<()> {
    let job: Job = serde_json::from_slice(&std::fs::read(job_path)?)?;
    let state: EnsembleState = serde_json::from_slice(&std::fs::read(state_path)?)?;
    if state.structures.len() != 1 || job.problem.datasets.len() != 1 {
        return Err("this Cu2O check requires one structure and one dataset".into());
    }
    let mut problem = job.problem.clone();
    problem.structures = state.structures.clone();
    std::fs::create_dir(out)?;
    let start = Instant::now();
    let options = job.problem.datasets[0].refeff.clone().unwrap_or(job.refeff);
    let mut calc = RefeffCalculator::new(options.clone())?;
    let evaluation = evaluate_ensemble(&problem, &job.settings, &mut calc)?.evaluation;
    write(
        out.join("full-potentials.json"),
        &json!({"evaluation":evaluation,"seconds":start.elapsed().as_secs_f64(),"stats":calc.stats(),"diagnostics":calc.diagnostics()}),
    )?;
    println!(
        "fresh potentials score {:.8}; {:.3} s",
        evaluation.score,
        start.elapsed().as_secs_f64()
    );
    let mut pinned =
        RefeffCalculator::new(options)?.with_frozen_potentials(job.frozen_references.clone())?;
    let setup = Instant::now();
    let mut reference_problem = job.problem.clone();
    reference_problem.structures[0].configuration = job.frozen_references[0].clone();
    evaluate_ensemble(&reference_problem, &job.settings, &mut pinned)?;
    let setup_seconds = setup.elapsed().as_secs_f64();
    let update = Instant::now();
    let pinned_evaluation = evaluate_ensemble(&problem, &job.settings, &mut pinned)?.evaluation;
    let update_seconds = update.elapsed().as_secs_f64();
    write(
        out.join("pinned-recheck.json"),
        &json!({"evaluation":pinned_evaluation,
        "setup_seconds":setup_seconds,"update_seconds":update_seconds,"stats":pinned.stats(),
        "matches_saved_spectrum":pinned_evaluation.datasets[0].chi==state.evaluation.datasets[0].chi,
        "diagnostics":pinned.diagnostics()}),
    )?;
    println!("pinned reference setup {setup_seconds:.3} s; geometry update {update_seconds:.3} s");
    Ok(())
}

fn check_prepared(
    job_path: &Path,
    state_path: &Path,
    acceleration: &Path,
    out: &Path,
) -> Result<()> {
    // The retained pipeline supplies both pinned and fully refreshed references.
    check(job_path, state_path, out)?;
    let job: Job = serde_json::from_slice(&std::fs::read(job_path)?)?;
    let state: EnsembleState = serde_json::from_slice(&std::fs::read(state_path)?)?;
    let settings: AccelerationSettings = serde_json::from_slice(&std::fs::read(acceleration)?)?;
    let mut problem = job.problem.clone();
    problem.structures = state.structures.clone();
    for (name, basis) in [
        ("prepared", settings.basis.clone()),
        ("typed-exact", ScatteringBasis::Exact),
    ] {
        let mut c = PreparedRefeffCalculator::new(
            job.refeff.clone(),
            job.frozen_references.clone(),
            AccelerationSettings {
                basis,
                ..settings.clone()
            },
        )?;
        let start = Instant::now();
        evaluate_ensemble(&job.problem, &job.settings, &mut c)?;
        let setup_seconds = start.elapsed().as_secs_f64();
        let start = Instant::now();
        let evaluated = evaluate_ensemble(&problem, &job.settings, &mut c)?;
        let seconds = start.elapsed().as_secs_f64();
        let d = &job.problem.datasets[0].exafs;
        write(
            out.join(format!("{name}.json")),
            &json!({"state":evaluated,"setup_seconds":setup_seconds,"update_seconds":seconds,"stats":c.stats(),"report":fit_report(d,&evaluated.evaluation.datasets[0].chi,&job.problem.datasets[0].objective)?,"matches_saved_spectrum":evaluated.evaluation.datasets[0].chi==state.evaluation.datasets[0].chi}),
        )?;
        println!(
            "{name} setup {setup_seconds:.3} s; best update {seconds:.3} s; score {:.8}",
            evaluated.evaluation.score
        );
    }
    Ok(())
}

fn curves(job_path: &Path, out: &Path) -> Result<()> {
    let job: Job = serde_json::from_slice(&std::fs::read(job_path)?)?;
    let data = &job.problem.datasets[0].exafs;
    let dk = 0.05;
    let count = (data.k.last().ok_or("empty k")? / dk).round() as usize + 1;
    let grid = DVector::from_iterator(count, (0..count).map(|i| i as f64 * dk));
    let mut spectra = vec![("experimental", data.chi.clone())];
    for name in ["initial", "best", "final"] {
        let state: EnsembleState =
            serde_json::from_slice(&std::fs::read(out.join(format!("{name}.json")))?)?;
        spectra.push((name, state.evaluation.datasets[0].chi.clone()));
    }
    let mut transformed = serde_json::Map::new();
    for (name, chi) in spectra {
        let mut padded = DVector::zeros(grid.len());
        for (&k, &value) in data.k.iter().zip(&chi) {
            let i = (k / dk).round() as usize;
            if (grid[i] - k).abs() > 1e-8 {
                return Err("curve export requires the prepared 0.05 Å^-1 grid".into());
            }
            padded[i] = value;
        }
        let mut ft = XrayFFTF {
            kmin: Some(data.k[0] + 0.5),
            kmax: Some(data.k[data.k.len() - 1] - 0.5),
            dk: Some(1.),
            dk2: Some(1.),
            window: Some(FTWindow::Hanning),
            kstep: Some(dk),
            kweight: Some(f64::from(data.kweight)),
            nfft: Some(2048),
            rmax_out: Some(6.),
            ..Default::default()
        };
        ft.xftf(&grid, &padded)?;
        transformed.insert(name.into(),json!({"chi":chi,
            "r":ft.get_r().unwrap().as_slice(),"real":ft.get_chir_real().unwrap().as_slice(),
            "imag":ft.get_chir_imag().unwrap().as_slice(),"magnitude":ft.get_chir_mag().unwrap().as_slice()}));
    }
    write(
        out.join("curves.json"),
        &json!({"k":data.k,"kweight":data.kweight,
        "transform":{"kernel":"exp(-2 i k R)","scale":"dk/sqrt(pi)","dk":dk,"nfft":2048,"window":"Hanning; 1 A^-1 taper wholly inside both ends of k support","phase_corrected":false},"spectra":transformed}),
    )
}

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    match args.first().and_then(|s|s.to_str()) {
        Some("prepare") if args.len()==3 => prepare(&PathBuf::from(&args[1]),&PathBuf::from(&args[2])),
        Some("run") if args.len()==3 => run(&PathBuf::from(&args[1]),&PathBuf::from(&args[2]),None,None),
        Some("run-prepared") if args.len()==4 => run(&PathBuf::from(&args[1]),&PathBuf::from(&args[3]),Some(&PathBuf::from(&args[2])),None),
        Some("resume-prepared") if args.len()==6 => run(&PathBuf::from(&args[1]),&PathBuf::from(&args[5]),Some(&PathBuf::from(&args[2])),Some((&PathBuf::from(&args[3]),args[4].to_str().ok_or("invalid step count")?.parse()?))),
        Some("check-prepared") if args.len()==5 => check_prepared(&PathBuf::from(&args[1]),&PathBuf::from(&args[2]),&PathBuf::from(&args[3]),&PathBuf::from(&args[4])),
        Some("check") if args.len()==4 => check(&PathBuf::from(&args[1]),&PathBuf::from(&args[2]),&PathBuf::from(&args[3])),
        Some("curves") if args.len()==3 => curves(&PathBuf::from(&args[1]),&PathBuf::from(&args[2])),
        _=>Err("usage: rmc_cu2o prepare PROJECT NEW_DIR | run JOB_JSON NEW_DIR | check JOB_JSON STATE_JSON NEW_DIR | curves JOB_JSON RUN_DIR | run-prepared JOB_JSON ACCELERATION_JSON NEW_DIR | check-prepared JOB_JSON STATE_JSON ACCELERATION_JSON NEW_DIR | resume-prepared JOB_JSON ACCELERATION_JSON CHECKPOINT_JSON TOTAL_STEPS NEW_DIR".into())
    }
}
