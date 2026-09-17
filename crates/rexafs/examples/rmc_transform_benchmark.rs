//! Unreleased numerical benchmark: identical local-map kernels through direct
//! sums and FFT convolution, and controlled moment sums versus direct phasors.
//! `cargo run --release -p rexafs --example rmc_transform_benchmark -- NEW_JSON`.
//! Synthetic inputs test numerical agreement/runtime, not experimental accuracy.
use num_complex::Complex64;
use rexafs::rmc::*;
use serde_json::json;
use std::{hint::black_box, time::Instant};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = std::env::args()
        .nth(1)
        .ok_or("supply a new output JSON path")?;
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(out)?;
    let k: Vec<_> = (0..191).map(|i| 2.5 + i as f64 * 0.05).collect();
    let values: Vec<_> = k
        .iter()
        .map(|k| (5. * k).sin() * (-0.004 * k * k).exp())
        .collect();
    let settings = LocalSpectrumSettings::morlet(WaveletSettings {
        k_centers: k.clone(),
        r: (0..48).map(|i| 0.5 + i as f64 * 0.1).collect(),
        omega0: 6.,
    });
    let mut maps = Vec::new();
    let mut timings = Vec::new();
    for algorithm in [LocalSpectrumAlgorithm::Direct, LocalSpectrumAlgorithm::Fft] {
        let start = Instant::now();
        let transform = LocalSpectrumTransform::new(
            &k,
            &LocalSpectrumSettings {
                algorithm: algorithm.clone(),
                ..settings.clone()
            },
        )?;
        let setup = start.elapsed().as_secs_f64();
        let reference = transform.transform(&values)?;
        let start = Instant::now();
        for _ in 0..100 {
            black_box(transform.transform(black_box(&values))?);
        }
        timings.push(json!({"algorithm":algorithm,"setup_seconds":setup,"100_maps_seconds":start.elapsed().as_secs_f64()}));
        maps.push(reference);
    }
    let error = maps[0]
        .iter()
        .zip(&maps[1])
        .map(|(a, b)| (*a - *b).norm())
        .fold(0., f64::max);
    let lengths: Vec<_> = (0..4000)
        .map(|i| 2.5 + (i as f64 - 1999.5) * 0.000001)
        .collect();
    let moment_settings = MomentSettings {
        tolerance: 1e-6,
        max_order: 24,
    };
    let start = Instant::now();
    let moments = PathMomentExpansion::new(&lengths, moment_settings.clone())?;
    let setup = start.elapsed().as_secs_f64();
    let start = Instant::now();
    let direct: Vec<Complex64> = k
        .iter()
        .map(|k| {
            lengths
                .iter()
                .map(|r| Complex64::from_polar(1., 2. * k * r))
                .sum()
        })
        .collect();
    let direct_seconds = start.elapsed().as_secs_f64();
    let start = Instant::now();
    let expanded = k
        .iter()
        .map(|&k| moments.evaluate(k))
        .collect::<Result<Vec<_>, _>>()?;
    let moment_seconds = start.elapsed().as_secs_f64();
    let moment_error = expanded
        .iter()
        .zip(&direct)
        .map(|(a, b)| (a.sum - b).norm())
        .fold(0., f64::max);
    serde_json::to_writer_pretty(
        file,
        &json!({"k":k,"map_settings":settings,"map_timings":timings,"maximum_complex_map_error":error,"lengths":lengths,"moment_settings":moment_settings,"moment_setup_seconds":setup,"direct_seconds":direct_seconds,"moment_seconds":moment_seconds,"moment_maximum_absolute_error":moment_error,"fallbacks":expanded.iter().filter(|e|e.order.is_none()).count(),"note":"One release-build workload; timings include owned output allocation, no scattering. Moment tolerance applies to the unscaled unit-phasor sum."}),
    )?;
    println!("maximum direct/FFT map error {error:.3e}; moment error {moment_error:.3e}; direct {direct_seconds:.6}s, moments {moment_seconds:.6}s");
    Ok(())
}
