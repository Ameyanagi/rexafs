//! Validate a local raw-mu synthetic copper collection with the desktop's core.
//!
//! Run with the directory written by `scripts/generate-cu-reduction.py`.
//! Reads 50 raw mixtures and three measured references without changing them;
//! writes `native-validation.json`. Normalization uses a common E0 (8979 eV),
//! pre-edge offsets -150..-75 eV and quadratic post-edge offsets 150..650 eV.
//! Compare fitted normalized coefficients with edge-step-weighted truth, not
//! the raw-mu generating coefficients. No truth or references enter blind MCR.
//! Check uncentered rank and compare two- and three-component MCR fits; the
//! three-component arrays are retained for independent plotting and review.
//!
//! To compare a saved desktop MCR run with known-reference LCF, append
//! `--compare-defaults <saved-project.rxs>`. This mode uses desktop automatic
//! normalization, verifies every fitted input against the saved MCR arrays,
//! and fits the same absolute energy interval separately for each target E0.
//! It writes `native-default-lcf.json` and compact `tutorial-results.json`,
//! without changing the saved project. `--desktop-defaults` instead computes
//! PCA and MCR with the desktop defaults before performing this comparison.
//! Inputs can be extracted from the bundle with `scripts/extract-cu-reduction.py`.
use rexafs::prelude::*;
use rexafs::xafs::normalization::{NormalizationMethod, PrePostEdge};
use serde_json::{json, Value};
use std::{error::Error, fs, path::Path};

/// Reproduce the desktop default run without requiring a graphical session.
fn desktop_defaults(root: &Path) -> Result<(), Box<dyn Error>> {
    let mixtures = (1..=50)
        .map(|i| prepare_default(&root.join(format!("data/series/frame_{i:02}.xdi"))))
        .collect::<Result<Vec<_>, _>>()?;
    let first = &mixtures[0];
    let energy = first.energy.as_ref().ok_or("Missing energy")?;
    let e0 = first.e0().ok_or("Missing E0")?;
    let mcr = mcr_als(
        &mixtures,
        &McrConfig {
            space: AnalysisSpace::Flat,
            range: Some((energy[0] - e0, energy[energy.len() - 1] - e0)),
            ..Default::default()
        },
    )?;
    let pca = pca_train(
        &mixtures,
        &PcaConfig {
            space: AnalysisSpace::Flat,
            center: false,
            ..Default::default()
        },
    )?;
    let path = root.join("native-default-analysis.json");
    fs::write(
        &path,
        serde_json::to_vec(&json!({
            "mcr_analysis": {"result": mcr}, "pca_analysis": {"model": pca}
        }))?,
    )?;
    compare_defaults(root, &path)
}

/// Retain the plotted arrays and metrics without the large spectral matrices.
fn write_tutorial_report(
    root: &Path,
    project: &Value,
    mcr: &McrResult,
    references: &[XASSpectrum],
    fits: &[LcfResult],
) -> Result<(), Box<dyn Error>> {
    let manifest: Value = serde_json::from_slice(&fs::read(root.join("manifest.json"))?)?;
    let truth: Vec<Vec<f64>> = manifest["truth"]
        .as_array()
        .ok_or("Missing truth")?
        .iter()
        .map(|r| serde_json::from_value(r["raw_mu_weights"].clone()))
        .collect::<Result<_, _>>()?;
    let steps: Vec<f64> = references
        .iter()
        .map(|s| {
            s.normalization
                .as_ref()
                .and_then(|n| n.get_edge_step())
                .ok_or("Missing reference edge step")
        })
        .collect::<Result<_, _>>()?;
    let reference_data = pca_train(
        references,
        &PcaConfig {
            space: AnalysisSpace::Flat,
            center: false,
            range: Some((
                mcr.x[0] - references[0].e0().unwrap(),
                mcr.x[mcr.x.len() - 1] - references[0].e0().unwrap(),
            )),
        },
    )?;
    assert_eq!(reference_data.x, mcr.x);
    let permutations = [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ];
    let cost = |p: &[usize; 3]| {
        (0..3)
            .map(|j| (mcr.spectra.row(p[j]) - reference_data.data.row(j)).norm_squared())
            .sum::<f64>()
    };
    let matched = permutations
        .iter()
        .min_by(|a, b| cost(a).total_cmp(&cost(b)))
        .unwrap();
    let mcr_weights: Vec<Vec<f64>> = (0..50)
        .map(|i| {
            matched
                .iter()
                .map(|&j| mcr.concentrations[(i, j)])
                .collect()
        })
        .collect();
    let lcf_weights: Vec<Vec<f64>> = fits
        .iter()
        .map(|f| f.weights.iter().map(|w| w.weight).collect())
        .collect();
    let metrics = |weights: &[Vec<f64>], residual: f64| {
        let raw: Vec<Vec<f64>> = weights
            .iter()
            .map(|row| {
                let mut scaled: Vec<f64> = row.iter().zip(&steps).map(|(w, s)| w / s).collect();
                let sum: f64 = scaled.iter().sum();
                scaled.iter_mut().for_each(|v| *v /= sum);
                scaled
            })
            .collect();
        let errors: Vec<f64> = raw
            .iter()
            .zip(&truth)
            .flat_map(|(a, b)| a.iter().zip(b).map(|(x, y)| 100. * (x - y).abs()))
            .collect();
        json!({"normalized_weights": weights, "raw_basis_estimates": raw,
            "mean_absolute_error_pp": errors.iter().sum::<f64>() / errors.len() as f64,
            "max_absolute_error_pp": errors.iter().copied().fold(0., f64::max),
            "relative_squared_residual": residual})
    };
    let lcf_residual = fits.iter().map(|f| f.chi_square).sum::<f64>()
        / fits.iter().map(|f| f.data.norm_squared()).sum::<f64>();
    let report = json!({
        "schema_version": 1, "rexafs_version": env!("CARGO_PKG_VERSION"),
        "release_status": "unreleased source checkout", "synthetic": true,
        "source": manifest["source"], "species": ["CuO", "Cu2O", "Cu"],
        "frames": 50, "points_per_frame": mcr.x.len(),
        "energy_ev": [mcr.x[0], mcr.x[mcr.x.len()-1]],
        "processing": "Desktop automatic E0; pre-edge -200..-30 eV; post-edge +150 eV to measured end; order 2; Victoreen 0",
        "fraction_comparison": "Approximate raw-mu coefficients after reference edge-step conversion; not calibrated mass or atomic fractions",
        "raw_truth": truth, "reference_edge_steps": steps,
        "mcr_config": mcr.config, "mcr_iterations": mcr.iterations,
        "mcr_termination": mcr.termination,
        "matched_mcr_indices_one_based": matched.map(|i| i+1),
        "lcf_config": {"space":"Flat", "sum_to_one":true, "weight_bounds":[0,1], "fit_e0_shift":false,
            "range_policy":"Same absolute endpoints as MCR; offsets calculated separately for each target E0"},
        "pca": {"space":"Flat", "center":false, "range_ev_relative_to_first_e0":[-20,30],
            "variance_explained":project["pca_analysis"]["model"]["variance_explained"]},
        "methods": {"MCR":metrics(&mcr_weights, mcr.relative_error), "LCF":metrics(&lcf_weights,lcf_residual)}
    });
    fs::write(
        root.join("tutorial-results.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    Ok(())
}

fn prepare_default(path: &Path) -> Result<XASSpectrum, Box<dyn Error>> {
    let source = rexafs::io::read_measurement(path)?;
    let (energy, mu) = source.scans[0].arrays(None)?;
    let mut sp = XASSpectrum::from_arrays(&energy, &mu)?;
    sp.set_name(path.file_stem().unwrap().to_string_lossy().as_ref());
    sp.find_e0()?;
    let e0 = sp.e0().ok_or("Missing automatic E0")?;
    // These are PipelineParams::default() resolutions in the desktop.
    sp.set_normalization_method(NormalizationMethod::PrePostEdge(PrePostEdge {
        e0: Some(e0),
        pre_edge_start: Some(-200.),
        pre_edge_end: Some(-30.),
        norm_start: Some(150.),
        norm_end: Some(energy[energy.len() - 1] - e0),
        norm_polyorder: Some(2),
        n_victoreen: Some(0),
        ..PrePostEdge::new()
    }))?;
    sp.normalize()?;
    Ok(sp)
}

fn compare_defaults(root: &Path, project_path: &Path) -> Result<(), Box<dyn Error>> {
    let project: Value = serde_json::from_slice(&fs::read(project_path)?)?;
    let mcr: McrResult = serde_json::from_value(project["mcr_analysis"]["result"].clone())?;
    if mcr.config.space != AnalysisSpace::Flat {
        return Err("The comparison expects the desktop's default Flat representation".into());
    }
    let references = ["CuO", "Cu2O", "Cu"]
        .iter()
        .map(|name| prepare_default(&root.join(format!("data/references/{name}.xdi"))))
        .collect::<Result<Vec<_>, _>>()?;
    let mut fits = Vec::new();
    let mut elapsed_seconds = 0.0;
    let mut max_input_difference = 0.0_f64;
    assert_eq!(mcr.data.nrows(), 50);
    for i in 0..50 {
        let target = prepare_default(&root.join(format!("data/series/frame_{:02}.xdi", i + 1)))?;
        let e0 = target.e0().ok_or("Missing target E0")?;
        let config = LcfConfig {
            space: AnalysisSpace::Flat,
            range: Some((mcr.x[0] - e0, mcr.x[mcr.x.len() - 1] - e0)),
            ..Default::default()
        };
        let started = std::time::Instant::now();
        let fit = lcf(&target, &references, &config)?;
        elapsed_seconds += started.elapsed().as_secs_f64();
        assert_eq!(fit.x, mcr.x, "LCF and MCR must use identical energy points");
        for j in 0..fit.data.len() {
            max_input_difference = max_input_difference.max((fit.data[j] - mcr.data[(i, j)]).abs());
        }
        fits.push(fit);
    }
    assert!(
        max_input_difference < 1e-12,
        "Desktop preprocessing differs"
    );
    let squared_residual: f64 = fits.iter().map(|f| f.chi_square).sum();
    let squared_data: f64 = fits.iter().map(|f| f.data.norm_squared()).sum();
    let result = json!({
        "scope": "Native LCF with desktop automatic preprocessing and the saved MCR energy grid",
        "species": ["CuO", "Cu2O", "Cu"],
        "frames": fits.len(),
        "points_per_frame": mcr.x.len(),
        "sum_to_one": true,
        "weight_bounds": [0., 1.],
        "fit_energy_shifts": false,
        "fit_only_seconds": elapsed_seconds,
        "max_input_difference_from_saved_mcr": max_input_difference,
        "relative_squared_residual": squared_residual / squared_data,
        "fits": fits,
    });
    write_tutorial_report(root, &project, &mcr, &references, &fits)?;
    fs::write(
        root.join("native-default-lcf.json"),
        serde_json::to_vec(&result)?,
    )?;
    println!("LCF: 50 frames, {} points each; relative squared residual {:.8e}; fit-only time {:.4} s; max input difference {:.3e}",
        mcr.x.len(), squared_residual / squared_data, elapsed_seconds, max_input_difference);
    Ok(())
}

fn spectrum(path: &Path) -> Result<XASSpectrum, Box<dyn Error>> {
    let source = rexafs::io::read_measurement(path)?;
    let (energy, mu) = source.scans[0].arrays(None)?;
    let mut spectrum = XASSpectrum::from_arrays(&energy, &mu)?;
    spectrum.set_name(path.file_stem().unwrap().to_string_lossy().as_ref());
    spectrum.set_normalization_method(NormalizationMethod::PrePostEdge(PrePostEdge {
        e0: Some(8979.),
        pre_edge_start: Some(-150.),
        pre_edge_end: Some(-75.),
        norm_start: Some(150.),
        norm_end: Some(650.),
        norm_polyorder: Some(2),
        n_victoreen: Some(0),
        ..PrePostEdge::new()
    }))?;
    spectrum.normalize()?;
    Ok(spectrum)
}

fn main() -> Result<(), Box<dyn Error>> {
    let arg = std::env::args()
        .nth(1)
        .ok_or("Pass the generated copper collection folder")?;
    let root = Path::new(&arg);
    if std::env::args().nth(2).as_deref() == Some("--desktop-defaults") {
        return desktop_defaults(root);
    }
    if std::env::args().nth(2).as_deref() == Some("--compare-defaults") {
        let saved = std::env::args()
            .nth(3)
            .ok_or("Pass the saved desktop MCR project")?;
        return compare_defaults(root, Path::new(&saved));
    }
    let manifest: Value = serde_json::from_slice(&fs::read(root.join("manifest.json"))?)?;
    let references = ["CuO", "Cu2O", "Cu"]
        .iter()
        .map(|name| spectrum(&root.join(format!("data/references/{name}.xdi"))))
        .collect::<Result<Vec<_>, _>>()?;
    let mixtures = (1..=50)
        .map(|i| spectrum(&root.join(format!("data/series/frame_{i:02}.xdi"))))
        .collect::<Result<Vec<_>, _>>()?;
    let mut report = serde_json::Map::new();
    for (name, space) in [("norm", AnalysisSpace::Norm), ("flat", AnalysisSpace::Flat)] {
        let config = LcfConfig {
            space,
            range: Some((-29., 171.)),
            ..Default::default()
        };
        let fits = lcf_batch(&mixtures, &references, &config);
        let mut error = 0.0_f64;
        for (i, fit) in fits.into_iter().enumerate() {
            for (j, weight) in fit?.weights.iter().enumerate() {
                error = error.max(
                    (weight.weight
                        - manifest["truth"][i]["expected_normalized_weights"][j]
                            .as_f64()
                            .unwrap())
                    .abs(),
                );
            }
        }
        assert!(error < 1e-8, "{name}: LCF fraction error {error}");
        let pca = pca_train(
            &mixtures,
            &PcaConfig {
                space,
                range: config.range,
                center: true,
            },
        )?;
        assert!(pca.variance_explained.iter().skip(2).sum::<f64>() < 1e-20);
        let uncentered = pca_train(
            &mixtures,
            &PcaConfig {
                space,
                range: config.range,
                center: false,
            },
        )?;
        assert!(uncentered.variance_explained[2] > 1e-6);
        assert!(uncentered.variance_explained.iter().skip(3).sum::<f64>() < 1e-20);
        report.insert(
            name.into(),
            json!({"lcf_max_absolute_fraction_error":error,
            "centered_pca_variance":pca.variance_explained.iter().take(4).collect::<Vec<_>>(),
            "uncentered_pca_squared_signal_fraction":uncentered.variance_explained.iter().take(5).collect::<Vec<_>>(),
            "uncentered_rank":3 }),
        );
    }
    let two = mcr_als(
        &mixtures,
        &McrConfig {
            space: AnalysisSpace::Flat,
            range: Some((-29., 171.)),
            components: 2,
            max_iterations: 2000,
            ..Default::default()
        },
    )?;
    report.insert(
        "blind_mcr_flat_two_components".into(),
        json!({"iterations":two.iterations,"termination":two.termination,
            "relative_squared_residual":two.relative_error}),
    );
    let mcr = mcr_als(
        &mixtures,
        &McrConfig {
            space: AnalysisSpace::Flat,
            range: Some((-29., 171.)),
            max_iterations: 2000,
            ..Default::default()
        },
    )?;
    assert!(mcr.relative_error < 1e-10);
    assert!(two.relative_error > 1e-6);
    assert!(mcr.config.anchors.is_empty() && mcr.config.initial_spectra.is_none());
    let reference_data = pca_train(
        &references,
        &PcaConfig {
            space: AnalysisSpace::Flat,
            range: Some((-29., 171.)),
            center: false,
        },
    )?;
    assert_eq!(reference_data.x, mcr.x);
    let permutations = [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ];
    let spectral_error = |p: &[usize; 3]| {
        (0..3)
            .map(|j| (mcr.spectra.row(p[j]) - reference_data.data.row(j)).norm_squared())
            .sum::<f64>()
    };
    let matched = permutations
        .iter()
        .min_by(|a, b| spectral_error(a).total_cmp(&spectral_error(b)))
        .unwrap();
    let fraction_error = (0..50)
        .flat_map(|i| (0..3).map(move |j| (i, j)))
        .map(|(i, j)| {
            (mcr.concentrations[(i, matched[j])]
                - manifest["truth"][i]["expected_normalized_weights"][j]
                    .as_f64()
                    .unwrap())
            .abs()
        })
        .fold(0.0_f64, f64::max);
    report.insert(
        "blind_mcr_flat".into(),
        json!({"iterations":mcr.iterations,
        "termination":mcr.termination,"relative_squared_residual":mcr.relative_error,
        "matched_component_indices_CuO_Cu2O_Cu":matched,
        "max_normalized_fraction_error":fraction_error,
        "matched_spectral_relative_errors":(0..3).map(|j| {
            (mcr.spectra.row(matched[j]) - reference_data.data.row(j)).norm() / reference_data.data.row(j).norm()
        }).collect::<Vec<_>>(),
        "initialization_frames":mcr.initial_samples.iter().map(|i| i+1).collect::<Vec<_>>(),
        "note":"Native MCR used spectra only; fractions and measured references were withheld."}),
    );
    fs::write(
        root.join("native-mcr-three-components.json"),
        serde_json::to_vec(&mcr)?,
    )?;
    let text = serde_json::to_string_pretty(&report)?;
    fs::write(root.join("native-validation.json"), &text)?;
    println!("{text}");
    Ok(())
}
