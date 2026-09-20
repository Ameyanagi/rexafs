// Retained local measurement harness, not a production API or convergence benchmark.
#[path = "../../../crates/rexafs-gui/src/rmc_fitting/memory.rs"]
mod memory;
use rexafs::rmc::*;
use std::{path::Path, time::Instant};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    let saved: serde_json::Value = serde_json::from_slice(&std::fs::read(&args[1])?)?;
    let request = &saved["request"];
    let problem: EnsembleProblem = serde_json::from_value(request["problem"].clone())?;
    let options: RefeffOptions = serde_json::from_value(request["calculator"].clone())?;
    let catalogue: PathCatalogueSettings = serde_json::from_value(request["catalogue"].clone())?;
    let mut settings: SessionSettings = serde_json::from_value(request["settings"].clone())?;
    settings.moves.steps = 1;
    let mut reference = None;
    let mut results = Vec::new();
    for manual in [Some(256usize), None] {
        let acceleration = AccelerationSettings {
            catalogue: catalogue.clone(),
            workers: request["workers"].as_u64().unwrap() as usize,
            parallel_paths: request["parallel_paths"].as_bool().unwrap(),
            reuse_electronic_inputs: request["reuse_electronic_inputs"].as_bool().unwrap(),
            cache_bytes: 256 * 1024 * 1024,
            ..Default::default()
        };
        let mut calc = PreparedRefeffCalculator::new(
            options.clone(),
            problem
                .structures
                .iter()
                .map(|s| s.configuration.clone())
                .collect(),
            acceleration,
        )?;
        let monitor = calc.monitor();
        let detected = memory::available_memory();
        let budget = memory::budget(manual, detected, 0);
        let mib = budget.bytes / memory::MIB;
        monitor.set_cache_bytes(budget.bytes);
        let stopping = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let stop = stopping.clone();
        let sampler = std::thread::spawn(move || {
            while !stop.load(std::sync::atomic::Ordering::Relaxed) {
                let next = memory::budget(
                    manual,
                    memory::available_memory(),
                    monitor.snapshot().stats.cached_bytes,
                );
                if memory::should_resize(monitor.cache_bytes(), next.bytes) {
                    monitor.set_cache_bytes(next.bytes);
                }
                std::thread::park_timeout(std::time::Duration::from_secs(2));
            }
        });
        let start = Instant::now();
        let mut session = RmcSession::new(&problem, &settings, &mut calc)?;
        let setup_seconds = start.elapsed().as_secs_f64();
        let setup_stats = calc.stats();
        eprintln!("{mib} MiB initial setup: {setup_seconds:.3} seconds");
        let start = Instant::now();
        session.step(&mut calc)?;
        let step_seconds = start.elapsed().as_secs_f64();
        let stats = calc.stats();
        let states = serde_json::to_vec(&(session.initial(), session.current(), session.best()))?;
        if let Some(previous) = &reference {
            assert_eq!(&states, previous);
        } else {
            reference = Some(states);
        }
        stopping.store(true, std::sync::atomic::Ordering::Relaxed);
        sampler.thread().unpark();
        sampler.join().unwrap();
        let report = serde_json::json!({"automatic":manual.is_none(),"initial_memory":detected,"initial_cache_mib":mib,"setup_seconds":setup_seconds,"step_seconds":step_seconds,"setup_stats":setup_stats,"final_stats":stats});
        eprintln!(
            "{mib} MiB step: {step_seconds:.3} seconds; reused {}",
            stats.reused_active_paths
        );
        results.push(report);
    }
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(Path::new(&args[2]))?;
    serde_json::to_writer_pretty(
        file,
        &serde_json::json!({"source_checkpoint":args[1],"bitwise_equal":true,"note":"One initial evaluation and one seeded attempt; timings exclude compilation. Every initial/current/best state was compared as full round-trip JSON bytes; automatic policy resampled memory every two seconds. Local diagnostic, not a converged refinement.","results":results}),
    )?;
    Ok(())
}
