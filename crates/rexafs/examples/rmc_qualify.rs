//! Unreleased reproducible Spectrum-input and adaptive-calculator qualification.
//!
//! `import SPECTRUM R_JOB STATE NEW_DIR` checks the accepted spectrum without
//! scattering and saves a snapshot-bearing job. `check JOB ACCEL STATES NEW_DIR`
//! compares exact/adaptive spectra on the named states in a JSON string array.
//! `run exact|adaptive JOB ACCEL SEED SECONDS NEW_DIR` starts from the job's
//! original crystal. Budget includes setup, monitoring, audits, refreshes and
//! checkpoint I/O. Final audit/verification are measured separately. Approximate
//! histories are segmented at model changes; final ranking uses exact scores.
//! These are fixed-potential checks, not fresh-electronic-potential validation.
//! Exact caching is recommended. Adaptive mode is an experimental opt-in;
//! this harness measures its accuracy and total cost, not an assumed speedup.
//! This bounded research harness requires one R-space dataset and one structure;
//! the general library supports multiple datasets and weighted structures.
use rexafs::{rmc::*, Spectrum};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{json, Value};
use std::{path::Path, time::Instant};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Job {
    problem: EnsembleProblem,
    settings: SessionSettings,
    refeff: RefeffOptions,
    frozen_references: Vec<Configuration>,
    provenance: Value,
}
fn validate_job(job: &Job) -> Result<()> {
    if job.problem.datasets.len() != 1
        || job.problem.structures.len() != 1
        || !matches!(job.problem.datasets[0].objective, Objective::R(_))
    {
        return Err("qualification requires one R-space dataset and one structure".into());
    }
    Ok(())
}
fn read<T: DeserializeOwned>(path: impl AsRef<Path>) -> Result<T> {
    Ok(serde_json::from_slice(&std::fs::read(path)?)?)
}
fn write(path: impl AsRef<Path>, value: &impl Serialize) -> Result<()> {
    let path = path.as_ref();
    let temporary = path.with_extension("pending");
    std::fs::write(&temporary, serde_json::to_vec_pretty(value)?)?;
    std::fs::rename(temporary, path)?;
    Ok(())
}
fn calculator(job: &Job, settings: AccelerationSettings) -> Result<PreparedRefeffCalculator> {
    Ok(PreparedRefeffCalculator::new(
        job.refeff.clone(),
        job.frozen_references.clone(),
        settings,
    )?)
}
fn adaptive(job: &Job, mut settings: AccelerationSettings) -> Result<AccelerationSettings> {
    eprintln!("Experimental adaptive mode; exact caching is the recommended default.");
    settings.basis = ScatteringBasis::Frozen {
        max_leg_change: 0.02,
        max_angle_change: 0.02,
    };
    // The adaptive manifest uses pair-distance features, replacing legacy guards.
    settings.adaptive = Some(AdaptiveBasisSettings {
        k: job.problem.datasets[0].exafs.theoretical_k()?,
        geometry_radius: 0.02,
        ..Default::default()
    });
    Ok(settings)
}
fn evaluate(
    job: &Job,
    state: &EnsembleState,
    calc: &mut PreparedRefeffCalculator,
) -> Result<EnsembleState> {
    let mut p = job.problem.clone();
    p.structures = state.structures.clone();
    Ok(evaluate_ensemble(&p, &job.settings, calc)?)
}
fn error(job: &Job, approximate: &EnsembleState, exact: &EnsembleState) -> Result<Value> {
    let data = &job.problem.datasets[0];
    let power = data
        .objective
        .score(&data.exafs, &vec![0.; data.exafs.k.len()])?;
    let mut difference = data.exafs.clone();
    difference.chi.clone_from(&exact.evaluation.datasets[0].chi);
    let norm = (data
        .objective
        .score(&difference, &approximate.evaluation.datasets[0].chi)?
        / power)
        .sqrt();
    let drift = (approximate.evaluation.datasets[0].score - exact.evaluation.datasets[0].score)
        .abs()
        / power;
    Ok(json!({"spectral_error":norm, "score_error":drift,
        "approximate_score":approximate.evaluation.score,"exact_score":exact.evaluation.score,
        "passes":norm<=0.001 && drift<=0.0001}))
}
fn import(spectrum: &str, input: &str, state: &str, out: &Path) -> Result<()> {
    let spectrum: Spectrum = read(spectrum)?;
    let mut job: Job = read(input)?;
    let accepted: EnsembleState = read(state)?;
    validate_job(&job)?;
    let old = &job.problem.datasets[0];
    let Objective::R(transform) = &old.objective else {
        return Err("source must use R fitting".into());
    };
    let mut options = RmcSpectrumOptions::new(
        old.exafs.absorbers.clone(),
        old.exafs.edge,
        transform.clone(),
    );
    options.k_range = Some([old.exafs.k[0], *old.exafs.k.last().ok_or("empty k")?]);
    options.s02 = old.exafs.s02;
    options.delta_e0 = old.exafs.delta_e0;
    if old.exafs.sigma.iter().any(|s| *s != 1.) {
        return Err("import check expects archived unit sigma".into());
    }
    let mut new = RmcDataset::from_spectrum(&spectrum, options)?;
    new.exafs.name = old.exafs.name.clone();
    new.refeff = old.refeff.clone();
    new.absorbers_by_structure = old.absorbers_by_structure.clone();
    if new.exafs != old.exafs || new.objective != old.objective {
        return Err(
            "Spectrum adapter changed experimental arrays, objective or calibration".into(),
        );
    }
    let old_score = old
        .objective
        .score(&old.exafs, &accepted.evaluation.datasets[0].chi)?;
    let new_score = new
        .objective
        .score(&new.exafs, &accepted.evaluation.datasets[0].chi)?;
    if old_score != new_score || (new_score - accepted.evaluation.score).abs() > 1e-12 {
        return Err("accepted score changed".into());
    }
    job.problem.datasets[0] = new;
    job.provenance = json!({"source_job":input,"source_provenance":job.provenance,"api":"RmcDataset::from_spectrum","accepted_state":state});
    std::fs::create_dir(out)?;
    write(out.join("job.json"), &job)?;
    write(
        out.join("api-regression.json"),
        &json!({"arrays_and_settings_identical":true,"saved_score":accepted.evaluation.score,"old_score":old_score,"spectrum_api_score":new_score,"bitwise_score_equal":old_score==new_score,"note":"Saved spectrum rescoring; no scattering or optimization here"}),
    )?;
    println!("Accepted score reproduced: {new_score:.16}");
    Ok(())
}
fn check(
    job: Job,
    acceleration: AccelerationSettings,
    paths: Vec<String>,
    out: &Path,
) -> Result<()> {
    validate_job(&job)?;
    std::fs::create_dir(out)?;
    let settings = adaptive(&job, acceleration)?;
    write(out.join("adaptive-settings.json"), &settings)?;
    let started = Instant::now();
    let mut approximate = calculator(&job, settings)?;
    let initial = evaluate_ensemble(&job.problem, &job.settings, &mut approximate)?;
    let setup_seconds = started.elapsed().as_secs_f64();
    let mut exact = approximate.exact_reference()?;
    let mut rows = Vec::new();
    let mut named = vec![("initial reference".to_owned(), initial)];
    for path in paths {
        named.push((path.clone(), read::<EnsembleState>(path)?));
    }
    for (path, state) in named {
        let start = Instant::now();
        let a = evaluate(&job, &state, &mut approximate)?;
        let approximate_seconds = start.elapsed().as_secs_f64();
        let start = Instant::now();
        let e = evaluate(&job, &state, &mut exact)?;
        let exact_seconds = start.elapsed().as_secs_f64();
        let mut row = error(&job, &a, &e)?;
        row["state"] = json!(path);
        row["approximate_seconds"] = json!(approximate_seconds);
        row["exact_seconds"] = json!(exact_seconds);
        row["saved_score"] = json!(state.evaluation.score);
        row["exact_saved_score_difference"] = json!(e.evaluation.score - state.evaluation.score);
        println!("{row}");
        rows.push(row);
        write(
            out.join("checks.json"),
            &json!({"setup_seconds":setup_seconds,"elapsed_seconds":started.elapsed().as_secs_f64(),"states":rows,"adaptive_stats":approximate.stats(),"exact_stats":exact.stats(),"training":approximate.adaptive_reports()}),
        )?;
    }
    Ok(())
}
#[derive(Default, Serialize)]
struct Work {
    requests: u64,
    exact_paths: u64,
    basis_paths: u64,
    reused_active_paths: u64,
    proposal_seconds: f64,
    audit_seconds: f64,
    monitoring_seconds: f64,
}
fn save(
    session: &RmcSession,
    calc: &PreparedRefeffCalculator,
    control: Option<&AdaptiveBasisController>,
    out: &Path,
) -> Result<()> {
    if let Some(c) = control {
        c.checkpoint_rmc(session, calc)?
            .save(out.join("checkpoint.json"))?;
    } else {
        session.save_checkpoint(out.join("checkpoint.json"))?;
    }
    write(out.join("best.json"), session.best())?;
    Ok(())
}
fn run(
    mode: &str,
    mut job: Job,
    mut acceleration: AccelerationSettings,
    seed: u64,
    seconds: f64,
    out: &Path,
) -> Result<()> {
    validate_job(&job)?;
    if !seconds.is_finite() || !(1. ..=86400.).contains(&seconds) {
        return Err("invalid wall-time budget".into());
    }
    std::fs::create_dir(out)?;
    job.settings.moves.seed = seed;
    job.settings.moves.steps = 1_000_000;
    let acceleration = if mode == "adaptive" {
        adaptive(&job, acceleration)?
    } else {
        acceleration.basis = ScatteringBasis::Exact;
        acceleration.adaptive = None;
        acceleration
    };
    let policy = AdaptiveAuditSettings::default();
    write(out.join("job.json"), &job)?;
    write(out.join("acceleration.json"), &acceleration)?;
    write(out.join("audit-policy.json"), &policy)?;
    let started = Instant::now();
    let mut calc = calculator(&job, acceleration)?;
    let mut session = RmcSession::new(&job.problem, &job.settings, &mut calc)?;
    let setup_seconds = started.elapsed().as_secs_f64();
    let mut monitor = calc.exact_reference()?;
    let mut control = if mode == "adaptive" {
        Some(AdaptiveBasisController::new(policy.clone(), &calc)?)
    } else {
        None
    };
    write(out.join("initial.json"), session.initial())?;
    let mut work = Work::default();
    let mut trace = Vec::new();
    let mut monitors = Vec::new();
    let mut audits = Vec::new();
    let mut last_monitor = None;
    let mut terminal_audit_seconds = 0.;
    let mut search_error = None;
    let mut best_exact_seen: Option<EnsembleState> = None;
    let search_result: Result<()> = (|| {
        while started.elapsed().as_secs_f64() < seconds {
            let n = session.completed();
            if n % policy.interval == 0 && last_monitor != Some(n) {
                if let Some(c) = &mut control {
                    let start = Instant::now();
                    let before = calc.stats();
                    let reports_before = c.reports().len();
                    c.audit_rmc(&mut session, &mut calc)?;
                    let elapsed = start.elapsed().as_secs_f64();
                    work.audit_seconds += elapsed;
                    audits.push(json!({"attempt":n,"seconds":elapsed,"before_stage_stats":before,"after_stage_stats":calc.stats(),"report":c.reports().get(reports_before)}));
                }
                let start = Instant::now();
                let exact = evaluate(&job, session.best(), &mut monitor)?;
                work.monitoring_seconds += start.elapsed().as_secs_f64();
                if best_exact_seen
                    .as_ref()
                    .is_none_or(|best| exact.evaluation.score < best.evaluation.score)
                {
                    best_exact_seen = Some(exact.clone());
                }
                monitors.push(json!({"attempt":n,"elapsed_seconds":started.elapsed().as_secs_f64(),"exact_score":exact.evaluation.score,"best_exact_seen":best_exact_seen.as_ref().unwrap().evaluation.score,"errors":error(&job,session.best(),&exact)?}));
                last_monitor = Some(n);
                if started.elapsed().as_secs_f64() >= seconds {
                    break;
                }
            }
            let start = Instant::now();
            let before = calc.stats();
            let Some(record) = session.step(&mut calc)? else {
                break;
            };
            work.proposal_seconds += start.elapsed().as_secs_f64();
            let after = calc.stats();
            work.requests += after.requests - before.requests;
            work.exact_paths += after.exact_paths - before.exact_paths;
            work.basis_paths += after.basis_paths - before.basis_paths;
            work.reused_active_paths += after.reused_active_paths - before.reused_active_paths;
            trace.push(json!({"elapsed_seconds":started.elapsed().as_secs_f64(),"revision":session.revisions().len(),"record":record}));
            if record.step % 100 == 0 {
                save(&session, &calc, control.as_ref(), out)?;
                write(out.join("trace.json"), &trace)?;
                write(out.join("exact-monitor.json"), &monitors)?;
                write(out.join("audits.json"), &audits)?;
                write(
                    out.join("progress.json"),
                    &json!({"mode":mode,"seed":seed,"status":"running","attempts":record.step,"elapsed_seconds":started.elapsed().as_secs_f64(),"best_approximate_score":session.best().evaluation.score,"best_exact_seen":best_exact_seen.as_ref().map(|s|s.evaluation.score),"work":work,"exact_fallback":control.as_ref().is_some_and(|c|c.uses_exact_paths()),"convergence":residual_trend(session.history(),&ResidualTrendSettings::default())?}),
                )?;
                println!(
                    "{mode} {seed}: {} attempts, {:.1}s, best {:.8}",
                    record.step,
                    started.elapsed().as_secs_f64(),
                    record.best_score
                );
            }
        }
        Ok(())
    })();
    if let Err(e) = search_result {
        search_error = Some(e.to_string());
    }
    let budget_used_seconds = started.elapsed().as_secs_f64();
    save(&session, &calc, control.as_ref(), out)?;
    if search_error.is_none() {
        if let Some(c) = &mut control {
            let start = Instant::now();
            c.audit_rmc(&mut session, &mut calc)?;
            terminal_audit_seconds = start.elapsed().as_secs_f64();
        }
    }
    let verify_start = Instant::now();
    let mut final_checks = Vec::new();
    let mut selected = Vec::new();
    for (name, state) in [
        ("initial", session.initial()),
        ("current", session.current()),
        ("best", session.best()),
    ] {
        let exact = evaluate(&job, state, &mut monitor)?;
        final_checks.push(json!({"state":name,"errors":error(&job,state,&exact)?}));
        selected.push(exact);
    }
    if let Some(state) = best_exact_seen {
        selected.push(state);
    }
    selected.sort_by(|a, b| a.evaluation.score.total_cmp(&b.evaluation.score));
    let best_exact = selected.remove(0);
    let final_verification_seconds = verify_start.elapsed().as_secs_f64();
    save(&session, &calc, control.as_ref(), out)?;
    write(out.join("best-exact.json"), &best_exact)?;
    std::fs::write(
        out.join("best-exact.xyz"),
        best_exact.structures[0].configuration.to_xyz()?,
    )?;
    write(out.join("trace.json"), &trace)?;
    write(out.join("exact-monitor.json"), &monitors)?;
    write(out.join("audits.json"), &audits)?;
    write(out.join("history.json"), &session.history())?;
    let trend = residual_trend(session.history(), &ResidualTrendSettings::default())?;
    write(out.join("convergence.json"), &trend)?;
    let report = json!({"mode":mode,"seed":seed,"status":if search_error.is_some(){"failed"}else{"complete"},"error":search_error,
        "budget_seconds":seconds,"budget_used_seconds":budget_used_seconds,"setup_seconds":setup_seconds,
        "terminal_audit_seconds":terminal_audit_seconds,"final_verification_seconds":final_verification_seconds,"total_seconds":started.elapsed().as_secs_f64(),
        "attempts":session.completed(),"initial_score":session.initial().evaluation.score,"best_score":session.best().evaluation.score,"best_exact_score":best_exact.evaluation.score,
        "exact_fallback":control.as_ref().is_some_and(|c|c.uses_exact_paths()),"controller_reports":control.as_ref().map(|c|c.reports()),"final_checks":final_checks,
        "work":work,"final_stage_stats":calc.stats(),"exact_monitor_stats":monitor.stats(),"convergence":trend,
        "counter_scope":"work path/request counters cover proposals only; wall time includes audits, discarded stages, monitoring and checkpoint I/O",
        "selection":"best exact score among periodically checked best states and terminal initial/current/best; not a claim about every visited state"});
    write(out.join("summary.json"), &report)?;
    write(out.join("progress.json"), &report)?;
    println!("{report:#}");
    if search_error.is_some() {
        return Err("search failed; recovery checkpoint and report saved".into());
    }
    Ok(())
}
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("import") if args.len()==5 => import(&args[1],&args[2],&args[3],Path::new(&args[4])),
        Some("check") if args.len()==5 => check(read(&args[1])?,read(&args[2])?,read(&args[3])?,Path::new(&args[4])),
        Some("run") if args.len()==7 && matches!(args[1].as_str(),"exact"|"adaptive") => run(&args[1],read(&args[2])?,read(&args[3])?,args[4].parse()?,args[5].parse()?,Path::new(&args[6])),
        _=>Err("usage: rmc_qualify import SPECTRUM R_JOB STATE NEW_DIR | check JOB ACCEL STATES NEW_DIR | run exact|adaptive JOB ACCEL SEED SECONDS NEW_DIR".into()),
    }
}
