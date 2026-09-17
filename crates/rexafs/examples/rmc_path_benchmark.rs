//! Unreleased: reproducible one-absorber Cu₂O path benchmark. Run with
//! `cargo run --release -p rexafs --features refeff-runner --example rmc_path_benchmark -- NEW_OUTPUT_JSON`.
//! Uses the built-in 2×2×2 cuprite structure, no experimental data. Compares
//! unscreened path physics and records setup/update times separately.
use rexafs::rmc::*;
use rexafs::structure::{BuiltinLibrary, Edge};
use serde_json::json;
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = std::env::args()
        .nth(1)
        .ok_or("supply a new output JSON path")?;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(out)?;
    let library = BuiltinLibrary::get()?;
    let c = Configuration::from_structure(&library.structure("cu2o_cuprite")?, [2, 2, 2])?;
    let absorber = c.atoms.iter().position(|a| a.atomic_number == 29).unwrap();
    let k: Vec<_> = (0..191).map(|i| 2.5 + i as f64 * 0.05).collect();
    let options = RefeffOptions {
        path_radius: 4.5,
        path_criteria: [0., 0.],
        ..Default::default()
    };
    let acceleration = AccelerationSettings {
        catalogue: PathCatalogueSettings {
            radius: 4.5,
            max_legs: 4,
            displacement: 0.4,
            ..Default::default()
        },
        basis: ScatteringBasis::Frozen {
            max_leg_change: 0.1,
            max_angle_change: 0.2,
        },
        ..Default::default()
    };
    let mut fast =
        PreparedRefeffCalculator::new(options.clone(), vec![c.clone()], acceleration.clone())?;
    let start = Instant::now();
    let initial = fast.calculate(&c, absorber, Edge::K, &k)?;
    let setup_seconds = start.elapsed().as_secs_f64();
    let mut changed = c.clone();
    // Move a nearest oxygen in the explicit cell; every periodic image moves with it.
    let oxygen = fast
        .catalogue(CalculationRequest {
            structure: 0,
            configuration: &c,
            absorber,
            edge: Edge::K,
            k: &k,
            options: None,
            paths: false,
        })?
        .paths()
        .iter()
        .filter(|p| p.scatterers.len() == 1 && c.atoms[p.scatterers[0].atom].atomic_number == 8)
        .min_by(|a, b| {
            a.half_length(&c)
                .unwrap()
                .total_cmp(&b.half_length(&c).unwrap())
        })
        .unwrap()
        .scatterers[0]
        .atom;
    changed.atoms[oxygen].position[0] += 0.025;
    changed.atoms[oxygen].position[1] -= 0.012;
    let start = Instant::now();
    let updated = fast.calculate(&changed, absorber, Edge::K, &k)?;
    let update_seconds = start.elapsed().as_secs_f64();
    let fast_stats = fast.stats();
    let start = Instant::now();
    let agreement = fast.compare_reference(CalculationRequest {
        structure: 0,
        configuration: &changed,
        absorber,
        edge: Edge::K,
        k: &k,
        options: None,
        paths: false,
    })?;
    let exact_seconds = start.elapsed().as_secs_f64();
    let mut baseline =
        RefeffCalculator::new(options.clone())?.with_frozen_potentials(vec![c.clone()])?;
    let start = Instant::now();
    let pipeline_initial = baseline.calculate(&c, absorber, Edge::K, &k)?;
    let pipeline_setup_seconds = start.elapsed().as_secs_f64();
    let start = Instant::now();
    let pipeline_updated = baseline.calculate(&changed, absorber, Edge::K, &k)?;
    let pipeline_update_seconds = start.elapsed().as_secs_f64();
    let compare = |a: &[f64], b: &[f64]| {
        (a.iter().zip(b).map(|(x, y)| (x - y).powi(2)).sum::<f64>()
            / b.iter().map(|x| x * x).sum::<f64>())
        .sqrt()
    };
    let report = json!({"reference":c,"changed":changed,"absorber":absorber,"moved_atom":oxygen,"options":options,"acceleration":acceleration,
        "k":k,"initial":initial,"updated":updated,"pipeline_initial":pipeline_initial,"pipeline_updated":pipeline_updated,
        "basis_accuracy":agreement,"typed_vs_pipeline_relative_l2":compare(&agreement.reference,&pipeline_updated),
        "initial_vs_pipeline_relative_l2":compare(&initial,&pipeline_initial),
        "setup_seconds":setup_seconds,"update_seconds":update_seconds,"direct_typed_check_seconds":exact_seconds,
        "pipeline_setup_seconds":pipeline_setup_seconds,"pipeline_update_seconds":pipeline_update_seconds,
        "warm_speed_ratio":pipeline_update_seconds/update_seconds,"prepared_stats":fast_stats,"pipeline_stats":baseline.stats(),
        "note":"One absorber, one local move; no EVAX timing and no inference of end-to-end RMC speed or experimental accuracy."});
    serde_json::to_writer_pretty(&mut file, &report)?;
    println!("prepared setup {setup_seconds:.3} s; update {update_seconds:.6} s; pinned pipeline update {pipeline_update_seconds:.6} s");
    println!(
        "basis relative L2 {:?}; typed vs pipeline {:.6}",
        agreement.relative_l2,
        compare(&agreement.reference, &pipeline_updated)
    );
    Ok(())
}
