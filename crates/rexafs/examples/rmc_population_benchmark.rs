//! Unreleased: exact-scattering cache comparison on identical Cu₂O requests.
//! `cargo run --release --locked -p rexafs --features refeff-runner
//! --example rmc_population_benchmark -- NEW_OUTPUT_JSON`
//! One absorber and synthetic population proposals; this is not an experimental
//! fit or an end-to-end EA benchmark. Every pair of calculated spectra must match
//! bit for bit. Setup includes all initial parents and is timed separately.
use rexafs::rmc::*;
use rexafs::structure::{BuiltinLibrary, Edge};
use serde_json::json;
use std::time::Instant;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args_os()
        .nth(1)
        .ok_or("supply new output JSON path")?;
    let mut output = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    let c =
        Configuration::from_structure(&BuiltinLibrary::get()?.structure("cu2o_cuprite")?, [2; 3])?;
    let absorber = c.atoms.iter().position(|a| a.atomic_number == 29).unwrap();
    let movable: Vec<_> = (0..c.atoms.len()).filter(|&i| i != absorber).collect();
    let parents: Vec<_> = (0..12)
        .map(|i| seeded_disorder(&c, &movable, 0.025, 7100 + i))
        .collect::<Result<_, _>>()?;
    let k: Vec<_> = (0..191).map(|i| 2.5 + i as f64 * 0.05).collect();
    let options = RefeffOptions {
        path_radius: 4.5,
        path_criteria: [0., 0.],
        ..Default::default()
    };
    let base = AccelerationSettings {
        catalogue: PathCatalogueSettings {
            radius: 4.5,
            max_legs: 4,
            displacement: 0.4,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut sequence = Vec::new();
    for i in 0..120 {
        let parent = &parents[i % parents.len()];
        sequence.push(parent.clone());
        sequence.push(seeded_disorder(
            parent,
            &[movable[i % movable.len()]],
            0.02,
            9300 + i as u64,
        )?);
    }
    let mut spectra = Vec::new();
    let mut reports = Vec::new();
    let mut scientific_identity = None;
    for capacity in [1, 25] {
        let settings = AccelerationSettings {
            snapshots_per_context: capacity,
            ..base.clone()
        };
        let mut calc =
            PreparedRefeffCalculator::new(options.clone(), vec![c.clone()], settings.clone())?;
        if let Some(identity) = &scientific_identity {
            assert_eq!(&calc.identity(), identity);
        }
        scientific_identity = Some(calc.identity());
        let start = Instant::now();
        for parent in &parents {
            calc.calculate(parent, absorber, Edge::K, &k)?;
        }
        let setup_seconds = start.elapsed().as_secs_f64();
        let setup_stats = calc.stats();
        let start = Instant::now();
        for (i, candidate) in sequence.iter().enumerate() {
            let chi = calc.calculate(candidate, absorber, Edge::K, &k)?;
            if capacity == 1 {
                spectra.push(chi);
            } else {
                assert_eq!(
                    chi, spectra[i],
                    "cache capacity changed scientific output at {i}"
                );
            }
        }
        let seconds = start.elapsed().as_secs_f64();
        println!(
            "{capacity} snapshots: {seconds:.4}s, {:.6}s/request",
            seconds / sequence.len() as f64
        );
        reports.push(
            json!({"capacity":capacity,"settings":settings,"setup_seconds":setup_seconds,
            "setup_stats":setup_stats,"seconds":seconds,"stats":calc.stats()}),
        );
    }
    serde_json::to_writer_pretty(
        &mut output,
        &json!({"options":options,"reference":c,
        "parent_seed_start":7100,"child_seed_start":9300,"k":k,"absorber":absorber,
        "population":12,"requests":sequence.len(),"bitwise_equal":true,"reports":reports,
        "note":"One absorber, fixed synthetic parent/child replay; excludes setup. Not a final fit or whole EA speedup."}),
    )?;
    Ok(())
}
