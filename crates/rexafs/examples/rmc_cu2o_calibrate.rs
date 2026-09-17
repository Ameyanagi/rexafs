//! Unreleased first-shell calibration check for a retained Cu2O RMC job.
//! `rmc_cu2o_calibrate SOURCE_JOB NEW_DIR` generates ReFEFF path files from the
//! fixed electronic reference and fits the nearest two-leg shell with the native
//! complex-R fitter. R-range/multistart variants are recorded; no calibration is
//! silently adopted and no atomic coordinates are changed.
use nalgebra::DVector;
use rexafs::fitting::{
    run_feff_and_load_paths, FeffExecutionMode, FeffFitDataset, FeffFitTransform, FeffFlavor,
    FeffRunRequest,
};
use rexafs::rmc::*;
use rexafs::xafs::xafsutils::FTWindow;
use serde_json::json;
use std::path::Path;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 2 {
        return Err("usage: rmc_cu2o_calibrate SOURCE_JOB NEW_DIR".into());
    }
    let source: serde_json::Value = serde_json::from_slice(&std::fs::read(&args[0])?)?;
    let problem: EnsembleProblem = serde_json::from_value(source["problem"].clone())?;
    let options: RefeffOptions = serde_json::from_value(source["refeff"].clone())?;
    let refs: Vec<Configuration> = serde_json::from_value(source["frozen_references"].clone())?;
    if problem.datasets.len() != 1 || refs.len() != 1 {
        return Err("this calibration example requires one dataset and one reference".into());
    }
    let data = &problem.datasets[0].exafs;
    if data.absorbers.is_empty()
        || data.k.len() < 2
        || data.k.len() != data.chi.len()
        || data.k.iter().any(|k| !k.is_finite() || *k < 0. || *k > 12.)
        || data.k.windows(2).any(|k| k[0] >= k[1])
        || data.chi.iter().any(|v| !v.is_finite())
    {
        return Err(
            "requires an absorber and finite matching chi/k arrays on increasing k in [0, 12]"
                .into(),
        );
    }
    let output = Path::new(&args[1]);
    std::fs::create_dir(output)?;
    let workspace = output.join("refeff");
    std::fs::create_dir(&workspace)?;
    let input = RefeffCalculator::new(options.clone())?.input_for(
        &refs[0],
        data.absorbers[0],
        data.edge,
    )?;
    std::fs::write(workspace.join("feff.inp"), &input)?;
    let paths = run_feff_and_load_paths(
        &FeffRunRequest {
            workspace_dir: workspace,
            mode: FeffExecutionMode::RefeffPipeline,
            timeout_sec: Some(300),
            keep_all_outputs: true,
            ..Default::default()
        },
        FeffFlavor::Feff85L,
    )?;
    let nearest = paths
        .iter()
        .filter(|p| p.feff.nleg == 2)
        .map(|p| p.feff.reff)
        .min_by(f64::total_cmp)
        .ok_or("no single-scattering paths")?;
    let shell: Vec<_> = paths
        .into_iter()
        .filter(|p| p.feff.nleg == 2 && (p.feff.reff - nearest).abs() < 0.05)
        .collect();
    let n = (data.k.last().ok_or("empty k")? / 0.05).round() as usize + 1;
    let k = DVector::from_iterator(n, (0..n).map(|i| i as f64 * 0.05));
    let mut chi = DVector::zeros(n);
    for (&q, &y) in data.k.iter().zip(&data.chi) {
        let i = (q / 0.05).round() as usize;
        if (k[i] - q).abs() > 1e-9 {
            return Err("requires 0.05 inverse-A grid".into());
        }
        chi[i] = y;
    }
    let mut summaries = Vec::new();
    for (rmin, rmax) in [(0.8, 2.1), (1.15, 2.1), (1.25, 2.05)] {
        for e0 in [0., 10.] {
            let transform = FeffFitTransform {
                kmin: 3.,
                kmax: 11.5,
                dk: 1.,
                dk2: Some(1.),
                kweight: 2.,
                window: FTWindow::Hanning,
                rmin,
                rmax,
                dr: 0.,
                dr2: Some(0.),
                kstep: Some(0.05),
                nfft: 2048,
                ..Default::default()
            };
            let dataset = FeffFitDataset {
                k: k.clone(),
                chi: chi.clone(),
                paths: shell.clone(),
                transform,
                epsilon_k: Some(1.),
                ..Default::default()
            };
            let settings = FirstShellSettings {
                initial: [data.s02, e0, 1., 0.003],
                ..Default::default()
            };
            let result = calibrate_first_shell(&dataset, &settings,
                format!("Cu2O experimental retained job {}; ReFEFF fixed reference; range sensitivity check",args[0]))?;
            let name = format!("fit-r{rmin:.2}-{rmax:.2}-e0-{e0:.0}.json");
            std::fs::write(output.join(&name), serde_json::to_vec_pretty(&result)?)?;
            let values: Vec<_> = [
                "rmc_s02",
                "rmc_delta_e0",
                "rmc_distance_scale",
                "rmc_sigma2",
            ]
            .iter()
            .map(|name| result.fit.variables.get(name).unwrap().value)
            .collect();
            let at_bound: Vec<_> = values
                .iter()
                .zip(settings.bounds)
                .map(|(v, b)| (v - b[0]).abs() <= 1e-5 || (v - b[1]).abs() <= 1e-5)
                .collect();
            println!("R {rmin:.2}..{rmax:.2}, start E0 {e0}: values {values:?}; Rfactor {:.7}; converged {:?}; bounds {at_bound:?}",result.fit.r_factor,result.fit.solver_report.as_ref().map(|r|r.converged));
            summaries.push(json!({"file":name,"rmin":rmin,"rmax":rmax,"initial_e0":e0,"values":values,
                "at_bound":at_bound,"r_factor":result.fit.r_factor,"solver":result.fit.solver_report,
                "n_idp":result.fit.n_idp,"varying_parameters":result.fit.n_vary}));
        }
    }
    std::fs::write(
        output.join("summary.json"),
        serde_json::to_vec_pretty(&json!({"source_job":args[0],
        "refeff":options,"shell_reff_A":nearest,"path_count":shell.len(),"fits":summaries,
        "note":"Native complex-R first-shell model; inspect bounds/range sensitivity before adopting. No job was changed."}))?,
    )?;
    Ok(())
}
