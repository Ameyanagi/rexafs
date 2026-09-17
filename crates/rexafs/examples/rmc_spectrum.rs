//! Unreleased Spectrum-input starter for a Cu K-edge RMC run.
//!
//! `rmc_spectrum SPECTRUM.json CONFIGURATION.json NEW_DIR [ATTEMPTS]` reads an
//! already processed rexafs Spectrum and explicit RMC Configuration (including
//! its periodic cell). Export these with serde_json. No preprocessing is rerun.
//! The visible settings below are illustrative, not a validated material recipe:
//! R=1.15–4 Å, k=3–11.5 Å⁻¹, S₀²=1, ΔE₀=0, unit numerical noise scales.
//! All Cu atoms are averaged; atom 0 is fixed. Adjust calibration/constraints for
//! the material. Exact prepared ReFEFF updates use fixed reference potentials.
//! The short default of 20 attempts demonstrates the API, not a converged fit.
use rexafs::fitting::FeffFitTransform;
use rexafs::rmc::*;
use rexafs::structure::Edge;
use rexafs::xafs::xafsutils::FTWindow;
use rexafs::Spectrum;
use serde::{de::DeserializeOwned, Serialize};
use std::path::{Path, PathBuf};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
fn read<T: DeserializeOwned>(path: impl AsRef<Path>) -> Result<T> {
    Ok(serde_json::from_slice(&std::fs::read(path)?)?)
}
fn write(path: impl AsRef<Path>, value: &impl Serialize) -> Result<()> {
    Ok(std::fs::write(path, serde_json::to_vec_pretty(value)?)?)
}

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if !(3..=4).contains(&args.len()) {
        return Err(
            "usage: rmc_spectrum SPECTRUM.json CONFIGURATION.json NEW_DIR [ATTEMPTS]".into(),
        );
    }
    let spectrum: Spectrum = read(&args[0])?;
    let configuration: Configuration = read(&args[1])?;
    let out = PathBuf::from(&args[2]);
    let steps = args
        .get(3)
        .map(|s| s.to_string_lossy().parse::<usize>())
        .transpose()?
        .unwrap_or(20);
    if steps == 0 {
        return Err("ATTEMPTS must be positive".into());
    }
    let absorbers = configuration
        .atoms
        .iter()
        .enumerate()
        .filter_map(|(i, atom)| (atom.atomic_number == 29).then_some(i))
        .collect();
    let mut input = RmcSpectrumOptions::new(
        absorbers,
        Edge::K,
        FeffFitTransform {
            kmin: 3.,
            kmax: 11.5,
            kweight: 2.,
            dk: 1.,
            dk2: Some(1.),
            window: FTWindow::Hanning,
            rmin: 1.15,
            rmax: 4.,
            ..Default::default()
        },
    );
    input.k_range = Some([2.5, 12.]); // Includes the k-window tapers.
    let dataset = RmcDataset::from_spectrum(&spectrum, input)?;
    let problem = EnsembleProblem::single(configuration.clone(), dataset);
    let settings = SessionSettings {
        moves: RmcSettings {
            steps,
            seed: 42,
            step_size: 0.03,
            temperature: 0.001,
            min_distance: 1.4,
            max_displacement: Some(0.2),
            movable_atoms: (1..configuration.atoms.len()).collect(),
        },
        ..Default::default()
    };
    let options = RefeffOptions {
        path_criteria: [0.; 2],
        ..Default::default()
    };
    let acceleration = AccelerationSettings::default(); // Exact paths; matching 4 Å / 4 legs / 0.2 Å envelope.
    let references = vec![configuration];
    let calculator =
        || PreparedRefeffCalculator::new(options.clone(), references.clone(), acceleration.clone());
    let mut calc = calculator()?;
    std::fs::create_dir(&out)?;
    write(
        out.join("job.json"),
        &serde_json::json!({"problem":problem, "settings":settings,
        "refeff":options, "acceleration":acceleration, "references":references}),
    )?;
    let mut session = RmcSession::new(&problem, &settings, &mut calc)?;
    let checkpoint = out.join("checkpoint.json");
    session.save_checkpoint(&checkpoint)?;
    for _ in 0..steps / 2 {
        if let Err(error) = session.step(&mut calc) {
            session.save_checkpoint(&checkpoint)?;
            return Err(error.into());
        }
    }
    session.save_checkpoint(&checkpoint)?;
    drop(session);
    drop(calc);
    // Cold resume: same reference/settings; saved preprocessing is not repeated.
    let mut calc = calculator()?;
    let mut session = RmcSession::load_checkpoint(&checkpoint, &mut calc)?;
    if let Err(error) = session.run(&mut calc) {
        session.save_checkpoint(&checkpoint)?;
        return Err(error.into());
    }
    session.save_checkpoint(&checkpoint)?;
    let trend = residual_trend(session.history(), &ResidualTrendSettings::default())?;
    write(out.join("convergence.json"), &trend)?;
    write(out.join("best.json"), session.best())?;
    std::fs::write(
        out.join("best.xyz"),
        session.best().structures[0].configuration.to_xyz()?,
    )?;
    println!(
        "{} attempts; normalized complex-R residual {:.8} -> {:.8}; {:?}",
        session.completed(),
        session.initial().evaluation.score,
        session.best().evaluation.score,
        trend.status
    );
    println!("Processing snapshot retained in job.json and checkpoint.json; short runs do not establish convergence.");
    Ok(())
}
