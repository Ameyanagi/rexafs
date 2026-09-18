//! Profile RMC startup with a local experimental copper Athena project.
//! Run with `cargo run --release --locked -p rexafs --features refeff-runner
//! --example rmc_startup_benchmark -- PROJECT cu|cu2o|cuo REPEATS legacy|reuse NEW_JSON`.
//! Reads the original project without modifying it. Times initial preparation and
//! one seeded coordinate proposal separately. The output contains experimental
//! data and must inherit the source's sharing restrictions. This is a performance
//! and numerical-regression check, not a converged material refinement.
use rexafs::{
    fitting::FeffFitTransform,
    io::AthenaProject,
    rmc::*,
    structure::{BuiltinLibrary, Edge},
};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 5 {
        return Err("expected PROJECT cu|cu2o|cuo REPEATS legacy|reuse NEW_JSON".into());
    }
    let (label, crystal) = match args[1].as_str() {
        "cu" => ("cufoil_abs", "cu"),
        "cu2o" => ("cu2o_abs", "cu2o_cuprite"),
        "cuo" => ("cuo_abs", "cuo_tenorite"),
        _ => return Err("choose cu, cu2o or cuo".into()),
    };
    let repeats: usize = args[2].parse()?;
    let reuse = match args[3].as_str() {
        "legacy" => false,
        "reuse" => true,
        _ => return Err("choose legacy or reuse".into()),
    };
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&args[4])?;
    let project = AthenaProject::read(&args[0])?;
    let mut group = project
        .group_by_label(label)
        .ok_or("missing experimental group")?
        .clone();
    // Preserve the source's fit settings, replacing its historical zero-width
    // Kaiser background taper with an explicit finite Hanning taper, as in rmc_cu2o.
    group.params.bkg_kwindow = Some("hanning".into());
    group.params.bkg_dk = Some(1.);
    let mut spectrum = group.to_spectrum()?;
    spectrum.calc_background()?;
    let library = BuiltinLibrary::get()?;
    let reference = Configuration::from_structure(&library.structure(crystal)?, [repeats; 3])?;
    let absorbers: Vec<_> = reference
        .atoms
        .iter()
        .enumerate()
        .filter_map(|(i, a)| (a.atomic_number == 29).then_some(i))
        .collect();
    let transform = FeffFitTransform {
        kmin: 3.,
        kmax: 12.,
        dk: 1.,
        rmin: 1.4,
        rmax: 4.5,
        ..Default::default()
    };
    let mut input = RmcSpectrumOptions::new(absorbers.clone(), Edge::K, transform);
    input.k_range = Some([2.45, 12.55]);
    let problem = EnsembleProblem::single(
        reference.clone(),
        RmcDataset::from_spectrum(&spectrum, input)?,
    );
    let options = RefeffOptions {
        kmax: 16.,
        path_criteria: [0., 0.],
        ..Default::default()
    };
    let acceleration = AccelerationSettings {
        max_contexts: absorbers.len().max(128),
        reuse_electronic_inputs: reuse,
        ..Default::default()
    };
    let settings = SessionSettings {
        moves: RmcSettings {
            steps: 1,
            seed: 20260918,
            step_size: 0.03,
            movable_atoms: (1..reference.atoms.len()).collect(),
            ..Default::default()
        },
        ..Default::default()
    };
    let mut calc =
        PreparedRefeffCalculator::new(options.clone(), vec![reference], acceleration.clone())?;
    let start = Instant::now();
    let mut session = RmcSession::new(&problem, &settings, &mut calc)?;
    let setup_seconds = start.elapsed().as_secs_f64();
    let setup_stats = calc.stats();
    eprintln!(
        "{} {}: initial {:.3}s, {} electronic preparations / {} contexts",
        args[1], args[3], setup_seconds, setup_stats.electronic_preparations, setup_stats.contexts
    );
    let start = Instant::now();
    session.step(&mut calc)?;
    let trial_seconds = start.elapsed().as_secs_f64();
    // Qualification is outside both timed regions. Resume from cold caches and
    // require bitwise agreement with the same mode, including the moved state.
    let mut cold = PreparedRefeffCalculator::new(
        options.clone(),
        problem
            .structures
            .iter()
            .map(|s| s.configuration.clone())
            .collect(),
        acceleration.clone(),
    )?;
    let resumed = RmcSession::resume(session.checkpoint(), &mut cold)?;
    assert_eq!(resumed.initial(), session.initial());
    assert_eq!(resumed.current(), session.current());
    assert_eq!(resumed.best(), session.best());
    let report = json!({"material":args[1],"mode":args[3],"repeats":repeats,"source_sha256":Sha256::digest(std::fs::read(&args[0])?).iter().map(|b| format!("{b:02x}")).collect::<String>(),"group":label,"structure":library.entry(crystal),"problem":problem,"settings":settings,"calculator":options,"acceleration":acceleration,"initial":session.initial(),"current":session.current(),"setup_seconds":setup_seconds,"trial_seconds":trial_seconds,"setup_stats":setup_stats,"final_stats":calc.stats(),"cold_resume_bitwise_equal":true});
    serde_json::to_writer_pretty(file, &report)?;
    Ok(())
}
