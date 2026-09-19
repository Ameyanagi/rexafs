//! Explicit, cancellable calibration previews. Existing RMC checkpoints are unchanged.
use super::*;
use sha2::{Digest, Sha256};

#[derive(Clone, Serialize, Deserialize)]
pub struct CalibrationPreview {
    pub input: String,
    pub settings: CalibrationSettings,
    pub result: CalibrationResult,
}

/// Scientific input identity used to reject stale calibration after form/source edits.
pub fn input_key(request: &Request) -> Result<String, String> {
    serde_json::to_vec(request)
        .map(|bytes| {
            Sha256::digest(bytes)
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect()
        })
        .map_err(|e| e.to_string())
}

pub fn calibration_settings(draft: &Draft) -> Result<CalibrationSettings, String> {
    let [lo, hi] = draft.calibration_range;
    let step = draft.calibration_step;
    let [amin, amax] = draft.calibration_amplitude;
    if !lo.is_finite()
        || !hi.is_finite()
        || lo >= hi
        || !step.is_finite()
        || step <= 0.
        || !amin.is_finite()
        || !amax.is_finite()
        || amin <= 0.
        || amax < amin
    {
        return Err(
            "Set an increasing ΔE₀ interval, positive grid step and positive S₀² bounds.".into(),
        );
    }
    let intervals = ((hi - lo) / step).ceil();
    if !intervals.is_finite() || intervals > 999. {
        return Err(
            "Calibration needs at most 1,000 energy candidates; increase the grid step.".into(),
        );
    }
    let mut grid: Vec<_> = (0..intervals as usize)
        .map(|i| lo + i as f64 * step)
        .collect();
    grid.push(hi);
    if grid.windows(2).any(|w| w[0] >= w[1]) {
        return Err("Calibration grid step is too small for this energy interval.".into());
    }
    Ok(CalibrationSettings {
        delta_e0: grid,
        s02_bounds: [amin, amax],
    })
}

pub fn bound_warning(preview: &CalibrationPreview) -> Option<&'static str> {
    let best = &preview.result.best;
    let at_energy_bound = preview.settings.delta_e0.first() == Some(&best.delta_e0)
        || preview.settings.delta_e0.last() == Some(&best.delta_e0);
    if preview.result.report.amplitude_at_bound == Some(true) || at_energy_bound {
        Some(
            "Estimate reached a search bound. Review the structure, normalization and search interval.",
        )
    } else {
        None
    }
}

pub struct DiagnosticControl {
    interrupt: Interrupt,
    stopping: Arc<AtomicBool>,
}

pub fn refinement_progress(
    saved: &SavedRun,
    result: &LocalRefinementResult,
) -> Result<Progress, String> {
    saved.validate()?;
    result.validate().map_err(|e| e.to_string())?;
    let checkpoint = serde_json::to_value(&saved.checkpoint).map_err(|e| e.to_string())?;
    if serde_json::to_value(&result.problem).map_err(|e| e.to_string())?
        != serde_json::to_value(&saved.request.problem).map_err(|e| e.to_string())?
        || checkpoint["calculator"].as_str() != Some(result.calculator.as_str())
        || serde_json::to_value(&result.session_settings).map_err(|e| e.to_string())?
            != checkpoint["settings"]
        || result.source_attempt != saved.progress.completed
        || result.initial.structures != saved.progress.best.structures
        || result.initial.delta_e0 != saved.progress.best.delta_e0
        || !result.best.evaluation.score.is_finite()
        || result.best.evaluation.score > result.initial.evaluation.score
    {
        return Err("Local refinement does not match this RMC checkpoint.".into());
    }
    let mut display = saved.progress.clone();
    display.initial = Arc::new(result.initial.clone());
    display.best = Arc::new(result.best.clone());
    validate_progress(&display, &saved.request)?;
    Ok(display)
}

pub fn export_refinement(
    directory: &Path,
    saved: &SavedRun,
    result: &LocalRefinementResult,
) -> Result<(), String> {
    use std::io::Write;
    refinement_progress(saved, result)?;
    std::fs::write(
        directory.join("local-refinement.json"),
        serde_json::to_vec_pretty(result).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    std::fs::write(
        directory.join("refined.xyz"),
        result.best.structures[0]
            .configuration
            .to_xyz()
            .map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    let data = &result.problem.datasets[0].exafs;
    let mut csv =
        std::fs::File::create(directory.join("refined-fit-k.csv")).map_err(|e| e.to_string())?;
    writeln!(csv, "k,experimental,rmc_best,refined").map_err(|e| e.to_string())?;
    for i in 0..data.k.len() {
        writeln!(
            csv,
            "{:.17e},{:.17e},{:.17e},{:.17e}",
            data.k[i],
            data.chi[i],
            result.initial.evaluation.datasets[0].chi[i],
            result.best.evaluation.datasets[0].chi[i]
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}
impl DiagnosticControl {
    pub fn stop(&self) {
        self.stopping.store(true, Ordering::Relaxed);
        if let Ok(guard) = self.interrupt.lock()
            && let Some(cancel) = guard.as_ref()
        {
            cancel();
        }
    }
}
impl Drop for DiagnosticControl {
    fn drop(&mut self) {
        self.stop();
    }
}

pub fn spawn_calibration(
    request: Request,
    settings: CalibrationSettings,
) -> (
    DiagnosticControl,
    mpsc::Receiver<Result<CalibrationPreview, String>>,
) {
    let (tx, rx) = mpsc::channel();
    let interrupt: Interrupt = Default::default();
    let stopping = Arc::new(AtomicBool::new(false));
    let control = DiagnosticControl {
        interrupt: interrupt.clone(),
        stopping: stopping.clone(),
    };
    std::thread::spawn(move || {
        let result = calculate_calibration(request, settings, interrupt, stopping);
        let _ = tx.send(result);
    });
    (control, rx)
}

#[cfg(feature = "refeff-runner")]
fn calculate_calibration(
    request: Request,
    settings: CalibrationSettings,
    interrupt: Interrupt,
    stopping: Arc<AtomicBool>,
) -> Result<CalibrationPreview, String> {
    if request.problem.datasets.len() != 1 || request.problem.structures.len() != 1 {
        return Err("Desktop calibration requires one dataset and one structure.".into());
    }
    let input = input_key(&request)?;
    let mut options = request.calculator.clone();
    // Cover every energy candidate before electronic preparation. Never clip
    // a candidate or extrapolate scattering tables beyond their supported k.
    for &shift in &settings.delta_e0 {
        let mut data = request.problem.datasets[0].exafs.clone();
        data.delta_e0 = shift;
        let grid = data
            .theoretical_k()
            .map_err(|e| format!("Calibration ΔE₀={shift:.1} eV: {e}"))?;
        options.kmax = options.kmax.max(grid.last().unwrap().ceil());
    }
    options.validate().map_err(|e| e.to_string())?;
    let mut calculator = PreparedRefeffCalculator::new(
        options,
        request
            .problem
            .structures
            .iter()
            .map(|s| s.configuration.clone())
            .collect(),
        request.acceleration_settings()?,
    )
    .map_err(|e| e.to_string())?;
    let token = calculator.cancellation_token();
    *interrupt
        .lock()
        .map_err(|_| "Calibration cancellation state failed")? =
        Some(Box::new(move || token.cancel()));
    if stopping.load(Ordering::Relaxed) {
        return Err("Calibration cancelled.".into());
    }
    let result = calibrate_dataset(&request.problem, 0, &settings, &mut calculator)
        .map_err(|e| e.to_string())?;
    if stopping.load(Ordering::Relaxed) {
        return Err("Calibration cancelled.".into());
    }
    Ok(CalibrationPreview {
        input,
        settings,
        result,
    })
}

pub fn spawn_refinement(
    saved: Box<SavedRun>,
    settings: LocalRefinementSettings,
) -> (
    DiagnosticControl,
    mpsc::Receiver<Result<LocalRefinementResult, String>>,
) {
    let (tx, rx) = mpsc::channel();
    let interrupt: Interrupt = Default::default();
    let stopping = Arc::new(AtomicBool::new(false));
    let control = DiagnosticControl {
        interrupt: interrupt.clone(),
        stopping: stopping.clone(),
    };
    std::thread::spawn(move || {
        let result = calculate_refinement(saved, settings, interrupt, stopping);
        let _ = tx.send(result);
    });
    (control, rx)
}

#[cfg(feature = "refeff-runner")]
fn calculate_refinement(
    saved: Box<SavedRun>,
    settings: LocalRefinementSettings,
    interrupt: Interrupt,
    stopping: Arc<AtomicBool>,
) -> Result<LocalRefinementResult, String> {
    saved.validate()?;
    let request = &saved.request;
    let mut calculator = PreparedRefeffCalculator::new(
        request.calculator.clone(),
        request
            .problem
            .structures
            .iter()
            .map(|s| s.configuration.clone())
            .collect(),
        request.acceleration_settings()?,
    )
    .map_err(|e| e.to_string())?;
    let token = calculator.cancellation_token();
    *interrupt
        .lock()
        .map_err(|_| "Refinement cancellation state failed")? =
        Some(Box::new(move || token.cancel()));
    if stopping.load(Ordering::Relaxed) {
        return Err("Refinement cancelled.".into());
    }
    let session =
        RmcSession::resume(saved.checkpoint, &mut calculator).map_err(|e| e.to_string())?;
    session
        .refine_best_with_progress(&settings, &mut calculator, |_| {
            if stopping.load(Ordering::Relaxed) {
                std::ops::ControlFlow::Break(())
            } else {
                std::ops::ControlFlow::Continue(())
            }
        })
        .map_err(|e| e.to_string())
}

#[cfg(not(feature = "refeff-runner"))]
fn calculate_refinement(
    _: Box<SavedRun>,
    _: LocalRefinementSettings,
    _: Interrupt,
    _: Arc<AtomicBool>,
) -> Result<LocalRefinementResult, String> {
    Err("This build has no ReFEFF backend.".into())
}

#[cfg(not(feature = "refeff-runner"))]
fn calculate_calibration(
    _: Request,
    _: CalibrationSettings,
    _: Interrupt,
    _: Arc<AtomicBool>,
) -> Result<CalibrationPreview, String> {
    Err("This build has no ReFEFF backend.".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn calibration_grid_is_bounded_and_retains_endpoints() {
        let mut draft = Draft::default();
        let settings = calibration_settings(&draft).unwrap();
        assert_eq!(settings.delta_e0.len(), 61);
        assert_eq!(settings.delta_e0[0], -15.);
        assert_eq!(*settings.delta_e0.last().unwrap(), 15.);
        draft.calibration_range = [0., 1.];
        draft.calibration_step = 0.3;
        assert_eq!(calibration_settings(&draft).unwrap().delta_e0.len(), 5);
        draft.calibration_step = 1e-10;
        assert!(calibration_settings(&draft).is_err());
        draft.calibration_step = f64::NAN;
        assert!(calibration_settings(&draft).is_err());
        draft.calibration_step = 0.5;
        draft.calibration_amplitude = [0., 1.];
        assert!(calibration_settings(&draft).is_err());
    }
    #[test]
    fn old_projects_retain_fixed_moves_and_new_requests_use_auto_moves() {
        let new = Draft::default();
        assert!(new.auto_moves);
        let mut old = serde_json::to_value(&new).unwrap();
        old.as_object_mut().unwrap().remove("auto_moves");
        old["step_size"] = serde_json::json!(0.03);
        let old: Draft = serde_json::from_value(old).unwrap();
        assert!(!old.auto_moves);
        assert_eq!(old.step_size, 0.03);
        let source = Source {
            group_id: None,
            label: "test".into(),
            path: "test.xmu".into(),
            fingerprint: 1,
            recipe: Default::default(),
        };
        let mut draft = new;
        draft.ranges.kmin = 3.;
        draft.ranges.kmax = 10.;
        let spectrum = super::super::tests::spectrum();
        let config = Configuration {
            atoms: vec![
                Atom {
                    atomic_number: 29,
                    position: [0.; 3],
                },
                Atom {
                    atomic_number: 29,
                    position: [2.5, 0., 0.],
                },
            ],
            cell: None,
        };
        let request = Request::new(&spectrum, config.clone(), &draft, source.clone()).unwrap();
        assert!(request.settings.adaptation.is_some());
        let key = input_key(&request).unwrap();
        let reloaded: Request =
            serde_json::from_value(serde_json::to_value(&request).unwrap()).unwrap();
        assert_eq!(key, input_key(&reloaded).unwrap());
        draft.delta_e0 = 1.;
        assert_ne!(
            key,
            input_key(&Request::new(&spectrum, config, &draft, source).unwrap()).unwrap()
        );
    }
}
