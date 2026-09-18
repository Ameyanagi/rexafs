//! Desktop RMC input snapshots, incremental execution and durable recovery.
//! The numerical algorithm remains in rexafs::rmc.
pub mod diagnostics;
use crate::{
    fitting::{FitRanges, FitSpaceSpec},
    group_identity::GroupId,
    params::PipelineParams,
};
use rexafs::{
    Spectrum,
    rmc::*,
    structure::{Edge, Structure},
};
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
};

#[cfg(feature = "refeff-runner")]
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum FitMode {
    #[default]
    Path,
    Rmc,
}
impl FitMode {
    pub const ALL: [Self; 2] = [Self::Path, Self::Rmc];
    pub fn label(self) -> &'static str {
        match self {
            Self::Path => "Path fitting",
            Self::Rmc => "RMC",
        }
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Draft {
    pub repeats: [usize; 3],
    pub steps: usize,
    pub step_size: f64,
    /// New drafts use bounded feedback and cooling; old projects stay fixed.
    #[serde(default)]
    pub auto_moves: bool,
    pub temperature: f64,
    pub max_displacement: f64,
    pub min_distance: f64,
    pub seed: u64,
    pub absorber: u8,
    pub edge: Edge,
    pub fixed_atoms: String,
    pub absorber_atoms: String,
    pub s02: f64,
    pub delta_e0: f64,
    /// Unreleased: opt in to fixed-S₀² energy refinement. Old projects stay fixed.
    pub refine_energy: bool,
    pub energy_refinement: EnergyRefinement,
    pub calibration_range: [f64; 2],
    pub calibration_step: f64,
    pub calibration_amplitude: [f64; 2],
    pub options: RefeffOptions,
    pub ranges: FitRanges,
    /// Copied Transform window and sampling settings. Older projects retain the
    /// historical fitting defaults until Use spectrum ranges is selected.
    pub transform_settings: Option<rexafs::xafs::fitting::FeffFitTransform>,
    pub constraints: Constraints,
}
impl Default for Draft {
    fn default() -> Self {
        Self {
            repeats: [2; 3],
            steps: 10_000,
            step_size: 0.05,
            auto_moves: true,
            temperature: 0.001,
            max_displacement: 0.2,
            min_distance: 1.,
            seed: 20260918,
            absorber: 29,
            edge: Edge::K,
            fixed_atoms: "0".into(),
            absorber_atoms: String::new(),
            s02: 1.,
            delta_e0: 0.,
            refine_energy: false,
            energy_refinement: EnergyRefinement::default(),
            calibration_range: [-15., 15.],
            calibration_step: 0.5,
            calibration_amplitude: [0.5, 1.2],
            options: RefeffOptions {
                path_criteria: [0., 0.],
                ..Default::default()
            },
            ranges: FitRanges {
                follow_transform: false,
                kweights: vec![2.],
                ..Default::default()
            },
            transform_settings: None,
            constraints: Constraints::default(),
        }
    }
}
impl Draft {
    fn transform(&self) -> rexafs::xafs::fitting::FeffFitTransform {
        let ranges = self.ranges.transform();
        let Some(mut transform) = self.transform_settings.clone() else {
            return ranges;
        };
        transform.kmin = ranges.kmin;
        transform.kmax = ranges.kmax;
        transform.rmin = ranges.rmin;
        transform.rmax = ranges.rmax;
        transform.kweight = ranges.kweight;
        transform.kweights = ranges.kweights;
        transform.fitspace = ranges.fitspace;
        transform
    }

    /// Fast form validation without preparing scattering or copying a spectrum.
    /// The native Spectrum adapter and session still validate the full problem
    /// at submission; this supplies actionable errors while editing the form.
    pub fn validate_form(
        &self,
        spectrum: &Spectrum,
        configuration: &Configuration,
    ) -> Result<(), String> {
        if spectrum.k().is_none() || spectrum.chi().is_none() {
            return Err("Prepare χ(k) in Background before starting RMC.".into());
        }
        let rbkg = crate::fitting::spectrum_rbkg(spectrum)
            .ok_or("Prepare the spectrum with AUTOBK in Background first.")?;
        self.ranges.validate_background(rbkg)?;
        if !self.s02.is_finite() || self.s02 <= 0. {
            return Err("S₀² must be positive. Use an independently calibrated amplitude.".into());
        }
        if !self.delta_e0.is_finite() {
            return Err("Set a finite fit ΔE₀ in eV.".into());
        }
        if self.refine_energy {
            self.energy_refinement
                .validate()
                .map_err(|e| e.to_string())?;
            let [lo, hi] = self.energy_refinement.bounds;
            if self.delta_e0 < lo || self.delta_e0 > hi {
                return Err("Starting ΔE₀ must lie inside the refinement bounds.".into());
            }
        }
        for (name, value) in [
            ("Move size", self.step_size),
            ("Maximum displacement", self.max_displacement),
            ("Minimum distance", self.min_distance),
        ] {
            if !value.is_finite() || value <= 0. {
                return Err(format!("{name} must be positive, in Å."));
            }
        }
        if !self.temperature.is_finite() || self.temperature < 0. {
            return Err("Metropolis tolerance must be zero or positive.".into());
        }
        if !(1..=1_000_000_000).contains(&self.steps) {
            return Err("Set an attempt budget between 1 and 1,000,000,000.".into());
        }
        let weights = self.transform().effective_kweights();
        if weights.len() != 1 || !(0. ..=3.).contains(&weights[0]) || weights[0].fract() != 0. {
            return Err("Choose one integer k weight from 0 through 3.".into());
        }
        self.options.validate().map_err(|e| e.to_string())?;
        self.k_support(spectrum)?;
        selected_absorbers(configuration, self)?;
        if atom_indices(&self.fixed_atoms, configuration.atoms.len())?.len()
            == configuration.atoms.len()
        {
            return Err(
                "At least one atom must remain movable; reduce the fixed atom selection.".into(),
            );
        }
        Ok(())
    }

    /// Cover the native fitting window (half a dk beyond each bound), with
    /// one grid point of interpolation margin. Reject unresolved shifted k
    /// before scattering; do not silently clip a nonzero Fourier window.
    fn k_support(&self, spectrum: &Spectrum) -> Result<[f64; 2], String> {
        use rexafs::xafs::xafsutils::constants::ETOK;
        let k = spectrum
            .k()
            .filter(|k| k.len() >= 2)
            .ok_or("Prepare χ(k) in Background first.")?;
        let transform = self.transform();
        let step = transform.kstep.unwrap_or(k[1] - k[0]);
        let lo = (transform.kmin - transform.dk / 2. - step).max(k[0]);
        let hi = (transform.kmax + transform.dk2.unwrap_or(transform.dk) / 2. + step)
            .min(k[k.len() - 1]);
        let selected: Vec<_> = k.iter().copied().filter(|x| *x >= lo && *x <= hi).collect();
        if selected.len() < 2
            || selected[0] > transform.kmin
            || selected[selected.len() - 1] < transform.kmax
        {
            return Err("The k fit range must lie inside the measured spectrum.".into());
        }
        let [shift_min, shift_max] = if self.refine_energy {
            self.energy_refinement
                .validate()
                .map_err(|e| e.to_string())?;
            self.energy_refinement.bounds
        } else {
            [self.delta_e0; 2]
        };
        let shifted = selected[0].powi(2) - ETOK * shift_max;
        if shifted < 0. {
            let minimum = (ETOK * shift_max).max(0.).sqrt() + transform.dk / 2. + step;
            return Err(format!(
                "This ΔE₀ needs k min above {minimum:.2} Å⁻¹ to retain the Fourier taper. Increase k min or review the calibration."
            ));
        }
        let required = (selected[selected.len() - 1].powi(2) - ETOK * shift_min).sqrt();
        if !required.is_finite() || required > 30. {
            return Err("The fit window and ΔE₀ require ReFEFF support above 30 Å⁻¹. Reduce k max or review ΔE₀.".into());
        }
        Ok([lo, hi])
    }

    /// Copy the spectrum's Transform bounds, windows, widths and forward sampling.
    /// Clip k bounds only to measured support. Explicit Back FT R bounds are used;
    /// absent bounds start above AUTOBK Rbkg and at 4 Å. Leave calibration explicit.
    pub fn use_spectrum_ranges(
        &mut self,
        spectrum: &Spectrum,
        recipe: &PipelineParams,
    ) -> Result<(), String> {
        let k = spectrum
            .k()
            .ok_or("Prepare the spectrum in Background first.")?;
        let rbkg = crate::fitting::spectrum_rbkg(spectrum)
            .ok_or("The processed spectrum needs a saved AUTOBK Rbkg.")?;
        if k.len() < 2 || !rbkg.is_finite() {
            return Err("Incomplete processed spectrum.".into());
        }
        let mut forward = crate::params::forward_settings(recipe)?;
        forward.fill_parameter(&nalgebra::DVector::from_column_slice(k));
        let kmin = forward.kmin.unwrap().max(k[0]);
        let kmax = forward.kmax.unwrap().min(k[k.len() - 1]);
        if !kmin.is_finite() || !kmax.is_finite() || kmax <= kmin {
            return Err(
                "The selected spectrum has insufficient k support for these starting ranges."
                    .into(),
            );
        }
        let weight = forward.kweight.unwrap();
        let back = rexafs::xafs::xrayfft::XrayFFTR::default();
        let transform = rexafs::xafs::fitting::FeffFitTransform {
            dk: forward.dk.unwrap(),
            dk2: forward.dk2,
            window: forward.window.unwrap(),
            nfft: forward.nfft.unwrap(),
            kstep: forward.kstep,
            dr: recipe.bft_dr.or(back.dr).unwrap(),
            dr2: recipe.bft_dr2,
            rwindow: recipe.bft_window.or(back.window).unwrap(),
            ..Default::default()
        };
        self.ranges = FitRanges {
            follow_transform: false,
            kmin,
            kmax,
            rmin: recipe.bft_rmin.unwrap_or(rbkg + 0.15),
            rmax: recipe.bft_rmax.unwrap_or(4_f64.max(rbkg + 1.15)),
            kweight: weight,
            kweights: vec![weight],
            fitspace: FitSpaceSpec::R,
            noise: false,
        };
        self.transform_settings = Some(transform);
        Ok(())
    }
}

/// Geometry-only preview. No ReFEFF, potentials or path enumeration are run.
pub fn build_preview(structure: &Structure, repeats: [usize; 3]) -> Result<Configuration, String> {
    Configuration::from_structure(structure, repeats).map_err(|e| e.to_string())
}

/// Flag obvious missing path coverage without equating Fourier R with a bond
/// length. A shorter fit interval still needs scientific model validation.
pub fn path_coverage_warning(rmax: f64, path_radius: f64) -> Option<String> {
    (rmax.is_finite() && path_radius.is_finite() && rmax > path_radius).then(|| {
        format!("R fit ends at {rmax:.1} Å; paths stop at {path_radius:.1} Å. Review the path radius or fit range.")
    })
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Project {
    pub draft: Draft,
    pub structure: Option<Structure>,
    pub configuration: Option<Configuration>,
    pub label: String,
    pub spectrum_defaults_applied: bool,
    pub saved: Option<Box<SavedRun>>,
    pub calibration: Option<Box<diagnostics::CalibrationPreview>>,
    pub refinement: Option<Box<LocalRefinementResult>>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Source {
    pub group_id: Option<GroupId>,
    pub label: String,
    pub path: PathBuf,
    pub fingerprint: u64,
    pub recipe: PipelineParams,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Request {
    pub problem: EnsembleProblem,
    pub settings: SessionSettings,
    pub calculator: RefeffOptions,
    pub catalogue: PathCatalogueSettings,
    /// New jobs share exactly matching canonical electronic inputs. Older saved
    /// requests retain historical ordering and calculator identity on resume.
    #[serde(default)]
    pub reuse_electronic_inputs: bool,
    pub source: Source,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CacheStats {
    pub exact: u64,
    pub reused_active: u64,
    pub requests: u64,
    pub geometry_hits: u64,
    pub bytes: usize,
    pub setup_seconds: f64,
    pub evaluation_seconds: f64,
    #[serde(default)]
    pub electronic_preparations: usize,
    #[serde(default)]
    pub shared_electronic_contexts: usize,
}
impl CacheStats {
    pub fn active_reuse(&self) -> Option<f64> {
        let total = self.exact + self.reused_active;
        (total > 0).then(|| self.reused_active as f64 / total as f64)
    }
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Progress {
    pub initial: Arc<EnsembleState>,
    pub best: Arc<EnsembleState>,
    pub current_score: f64,
    pub completed: usize,
    pub limit: usize,
    pub accepted: usize,
    pub constraint_rejected: usize,
    pub history: Vec<SessionStep>,
    pub trend: ResidualTrendReport,
    pub cache: CacheStats,
    pub setup_seconds: f64,
    pub elapsed_seconds: f64,
    #[serde(default)]
    pub move_scale: Option<f64>,
    #[serde(default)]
    pub recent_acceptance: Option<f64>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct SavedRun {
    pub version: u32,
    pub request: Request,
    pub checkpoint: RmcCheckpoint,
    pub progress: Progress,
    pub status: String,
}
impl SavedRun {
    /// Check result dimensions before displaying/exporting untrusted project files.
    /// Numerical checkpoint validation is also performed by RmcSession::resume.
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1 {
            return Err("Unsupported desktop RMC checkpoint version.".into());
        }
        validate_progress(&self.progress, &self.request)?;
        let checkpoint = serde_json::to_value(&self.checkpoint).map_err(|e| e.to_string())?;
        for (key, value) in [
            ("problem", serde_json::to_value(&self.request.problem)),
            ("initial", serde_json::to_value(&self.progress.initial)),
            ("best", serde_json::to_value(&self.progress.best)),
            ("completed", serde_json::to_value(self.progress.completed)),
        ] {
            if checkpoint.get(key) != Some(&value.map_err(|e| e.to_string())?) {
                return Err(format!(
                    "Saved RMC {key} differs from its numerical checkpoint."
                ));
            }
        }
        // The total attempt limit is the only input a continuation may change.
        let mut settings =
            serde_json::to_value(&self.request.settings).map_err(|e| e.to_string())?;
        settings["moves"]["steps"] = checkpoint["settings"]["moves"]["steps"].clone();
        if checkpoint.get("settings") != Some(&settings) {
            return Err("Saved RMC settings differ from their numerical checkpoint.".into());
        }
        Ok(())
    }
}
pub fn validate_progress(p: &Progress, request: &Request) -> Result<(), String> {
    let problem = &request.problem;
    if problem.datasets.len() != 1 || problem.structures.len() != 1 {
        return Err("The desktop RMC viewer requires one spectrum and one structure.".into());
    }
    let data = &problem.datasets[0];
    let n = data.exafs.k.len();
    if n < 2
        || data.exafs.chi.len() != n
        || !matches!(data.objective, Objective::R(_))
        || !data
            .exafs
            .k
            .iter()
            .chain(&data.exafs.chi)
            .all(|x| x.is_finite())
        || !data.exafs.k.windows(2).all(|x| x[1] > x[0])
    {
        return Err("Invalid saved RMC spectrum or objective.".into());
    }
    for state in [&p.initial, &p.best] {
        let shifts = state.energy_shifts(problem).map_err(|e| e.to_string())?;
        if let Some(policy) = &request.settings.energy_refinement {
            policy.validate().map_err(|e| e.to_string())?;
            if state.delta_e0.len() != problem.datasets.len()
                || shifts
                    .iter()
                    .any(|v| *v < policy.bounds[0] || *v > policy.bounds[1])
            {
                return Err("Saved RMC ΔE₀ is missing or outside its bounds.".into());
            }
        } else if !state.delta_e0.is_empty() {
            return Err("Fixed-energy RMC result contains refined shifts.".into());
        }
        if state.structures.len() != 1
            || state.evaluation.datasets.len() != 1
            || state.evaluation.datasets[0].chi.len() != n
            || !state.evaluation.datasets[0]
                .chi
                .iter()
                .all(|x| x.is_finite())
            || !state.evaluation.score.is_finite()
            || !state.penalty.is_finite()
        {
            return Err("Invalid saved RMC result dimensions or values.".into());
        }
        state.structures[0]
            .configuration
            .validate()
            .map_err(|e| e.to_string())?;
    }
    if p.completed > p.limit
        || p.accepted > p.completed
        || p.constraint_rejected > p.completed
        || !p.current_score.is_finite()
        || !p.elapsed_seconds.is_finite()
        || p.elapsed_seconds < 0.
        || p.history
            .iter()
            .any(|h| h.step > p.completed || !h.score.is_finite() || !h.best_score.is_finite())
    {
        return Err("Invalid saved RMC progress.".into());
    }
    Ok(())
}

/// Most recent durable checkpoint, discoverable after an unexpected app close.
pub fn latest_recovery(root: &Path) -> Result<PathBuf, String> {
    std::fs::read_dir(root)
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .map(|entry| entry.path().join("checkpoint.json"))
        .filter_map(|path| Some((path.metadata().ok()?.modified().ok()?, path)))
        .max_by_key(|(modified, _)| *modified)
        .map(|(_, path)| path)
        .ok_or_else(|| "No saved RMC recovery checkpoint was found.".into())
}

pub enum Event {
    Progress(Box<Progress>),
    Saved(Box<SavedRun>),
    Paused,
    Finished(Box<SavedRun>),
    Error(String),
}
enum Command {
    Resume(usize),
    Pause,
    Stop,
}
type Interrupt = Arc<Mutex<Option<Box<dyn Fn() + Send + Sync>>>>;
pub struct Control {
    tx: mpsc::Sender<Command>,
    interrupt: Interrupt,
    stopping: Arc<AtomicBool>,
}
impl Control {
    pub fn pause(&self) {
        let _ = self.tx.send(Command::Pause);
    }
    pub fn resume(&self, total: usize) {
        let _ = self.tx.send(Command::Resume(total));
    }
    pub fn stop(&self) {
        self.stopping.store(true, Ordering::Relaxed);
        let _ = self.tx.send(Command::Stop);
        if let Ok(guard) = self.interrupt.lock()
            && let Some(cancel) = guard.as_ref()
        {
            cancel();
        }
    }
}
impl Drop for Control {
    fn drop(&mut self) {
        self.stop();
    }
}

pub fn atom_indices(text: &str, count: usize) -> Result<Vec<usize>, String> {
    let mut out = Vec::new();
    for value in text
        .split(|c: char| c == ',' || c.is_whitespace())
        .filter(|s| !s.is_empty())
    {
        let index = value.parse::<usize>().map_err(|_| {
            format!("Invalid atom index '{value}'; use zero-based numbers separated by spaces.")
        })?;
        if index >= count || out.contains(&index) {
            return Err(format!(
                "Atom index {index} is duplicated or outside 0…{}.",
                count.saturating_sub(1)
            ));
        }
        out.push(index);
    }
    Ok(out)
}
/// Resolve preview and calculation absorbers using the same checked atom indices.
pub fn selected_absorbers(
    configuration: &Configuration,
    draft: &Draft,
) -> Result<Vec<usize>, String> {
    let absorbers = if draft.absorber_atoms.trim().is_empty() {
        configuration
            .atoms
            .iter()
            .enumerate()
            .filter(|(_, a)| a.atomic_number == draft.absorber)
            .map(|(i, _)| i)
            .collect::<Vec<_>>()
    } else {
        atom_indices(&draft.absorber_atoms, configuration.atoms.len())?
    };
    if absorbers.is_empty()
        || absorbers
            .iter()
            .any(|&i| configuration.atoms[i].atomic_number != draft.absorber)
    {
        return Err("Select absorbing atoms of the chosen element.".into());
    }
    Ok(absorbers)
}
impl Request {
    pub fn new(
        spectrum: &Spectrum,
        configuration: Configuration,
        draft: &Draft,
        source: Source,
    ) -> Result<Self, String> {
        configuration.validate().map_err(|e| e.to_string())?;
        draft.validate_form(spectrum, &configuration)?;
        let ranges = draft.ranges.resolved(source.recipe.fft_kweight);
        if ranges.fitspace != FitSpaceSpec::R || ranges.noise {
            return Err("Desktop RMC currently requires R space with explicit unit χ noise scales; automatic noise estimation is not connected.".into());
        }
        if !draft.step_size.is_finite()
            || draft.step_size <= 0.
            || !draft.temperature.is_finite()
            || draft.temperature < 0.
            || !draft.max_displacement.is_finite()
            || draft.max_displacement <= 0.
            || !draft.min_distance.is_finite()
            || draft.min_distance <= 0.
            || draft.steps == 0
            || draft.steps > 1_000_000_000
        {
            return Err("Set positive move/displacement/distance bounds, nonnegative tolerance and 1…1,000,000,000 attempts.".into());
        }
        let fixed = atom_indices(&draft.fixed_atoms, configuration.atoms.len())?;
        let movable_atoms = (0..configuration.atoms.len())
            .filter(|i| !fixed.contains(i))
            .collect::<Vec<_>>();
        if movable_atoms.is_empty() {
            return Err("At least one atom must remain movable.".into());
        }
        let absorbers = selected_absorbers(&configuration, draft)?;
        draft.options.validate().map_err(|e| e.to_string())?;
        if draft.options.path_criteria != [0., 0.] {
            return Err("Exact cached RMC requires zero path amplitude criteria.".into());
        }
        let transform = Draft {
            ranges,
            ..draft.clone()
        }
        .transform();
        let mut input = RmcSpectrumOptions::new(absorbers, draft.edge, transform.clone());
        input.k_range = Some(draft.k_support(spectrum)?);
        input.s02 = draft.s02;
        input.delta_e0 = draft.delta_e0;
        let mut dataset = RmcDataset::from_spectrum(spectrum, input).map_err(|e| e.to_string())?;
        dataset.exafs.name = source.label.clone();
        let mut coverage = dataset.exafs.clone();
        if draft.refine_energy {
            coverage.delta_e0 = draft.energy_refinement.bounds[0];
        }
        let theoretical_k = coverage.theoretical_k().map_err(|e| e.to_string())?;
        let mut calculator = draft.options.clone();
        // Include the measured taper and shifted theoretical grid. Round upward
        // to a whole Å⁻¹; never extrapolate returned scattering tables.
        calculator.kmax = calculator.kmax.max(theoretical_k.last().unwrap().ceil());
        calculator.validate().map_err(|e| e.to_string())?;
        let problem = EnsembleProblem::single(configuration, dataset);
        let settings = SessionSettings {
            moves: RmcSettings {
                steps: draft.steps,
                step_size: draft.step_size,
                temperature: draft.temperature,
                min_distance: draft.min_distance,
                max_displacement: Some(draft.max_displacement),
                seed: draft.seed,
                movable_atoms,
            },
            constraints: draft.constraints.clone(),
            energy_refinement: draft.refine_energy.then(|| draft.energy_refinement.clone()),
            history_capacity: 10_000,
            ..Default::default()
        };
        let settings = if draft.auto_moves {
            settings.with_auto_moves()
        } else {
            settings
        };
        let catalogue = PathCatalogueSettings {
            radius: draft.options.path_radius,
            max_legs: draft.options.max_legs as usize,
            displacement: draft.max_displacement,
            ..Default::default()
        };
        Ok(Self {
            problem,
            settings,
            calculator,
            catalogue,
            reuse_electronic_inputs: true,
            source,
        })
    }

    /// Allocate one context slot for each requested structure, absorber, edge
    /// and settings combination. This is a resource choice, not site sampling.
    /// Path-count and byte budgets remain independently enforced by the core.
    #[cfg(feature = "refeff-runner")]
    fn acceleration_settings(&self) -> Result<AccelerationSettings, String> {
        let mut contexts = std::collections::BTreeSet::new();
        for dataset in &self.problem.datasets {
            let options =
                serde_json::to_string(dataset.refeff.as_ref().unwrap_or(&self.calculator))
                    .map_err(|e| e.to_string())?;
            for structure in 0..self.problem.structures.len() {
                let absorbers = if dataset.absorbers_by_structure.is_empty() {
                    &dataset.exafs.absorbers
                } else {
                    dataset
                        .absorbers_by_structure
                        .get(structure)
                        .ok_or("Missing absorber selection for an RMC structure.")?
                };
                for &absorber in absorbers {
                    contexts.insert((
                        structure,
                        absorber,
                        dataset.exafs.edge.hole_index(),
                        options.clone(),
                    ));
                }
            }
        }
        Ok(AccelerationSettings {
            catalogue: self.catalogue.clone(),
            max_contexts: contexts
                .len()
                .max(AccelerationSettings::default().max_contexts),
            reuse_electronic_inputs: self.reuse_electronic_inputs,
            ..Default::default()
        })
    }
}

/// Resume the existing budget, or extend a completed run by an explicit number
/// of additional attempts. This changes no inputs, calibration or random state.
pub fn continuation_limit(
    completed: usize,
    limit: usize,
    additional: usize,
) -> Result<usize, String> {
    if completed < limit {
        return Ok(limit);
    }
    completed
        .checked_add(additional)
        .filter(|total| additional > 0 && *total <= 1_000_000_000)
        .ok_or_else(|| {
            "Additional attempts must be positive and keep the total at or below 1,000,000,000."
                .into()
        })
}

pub fn save_run(path: &Path, run: &SavedRun) -> Result<(), String> {
    use std::io::Write;
    let parent = path
        .parent()
        .ok_or("Recovery file has no parent directory")?;
    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let mut file = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    serde_json::to_writer(file.as_file_mut(), run).map_err(|e| e.to_string())?;
    file.flush().map_err(|e| e.to_string())?;
    file.as_file().sync_all().map_err(|e| e.to_string())?;
    file.persist(path).map_err(|e| e.to_string())?;
    Ok(())
}
pub fn load_run(path: &Path) -> Result<SavedRun, String> {
    if std::fs::metadata(path).map_err(|e| e.to_string())?.len() > 512 * 1024 * 1024 {
        return Err("RMC checkpoint exceeds 512 MiB.".into());
    }
    let run: SavedRun =
        serde_json::from_reader(std::fs::File::open(path).map_err(|e| e.to_string())?)
            .map_err(|e| format!("RMC checkpoint: {e}"))?;
    run.validate()?;
    Ok(run)
}

#[cfg(feature = "refeff-runner")]
pub fn spawn(
    request: Request,
    resume: Option<Box<SavedRun>>,
    path: PathBuf,
    prepare_only: bool,
) -> (Control, mpsc::Receiver<Event>) {
    let (tx, commands) = mpsc::channel();
    let (events, rx) = mpsc::sync_channel(2);
    let interrupt: Interrupt = Default::default();
    let stopping = Arc::new(AtomicBool::new(false));
    let worker_interrupt = interrupt.clone();
    let worker_stopping = stopping.clone();
    std::thread::spawn(move || {
        let result = (|| -> Result<(), String> {
            if let Some(saved) = &resume {
                saved.validate()?;
            }
            let settings = request.acceleration_settings()?;
            let mut calculator = PreparedRefeffCalculator::new(
                request.calculator.clone(),
                request
                    .problem
                    .structures
                    .iter()
                    .map(|s| s.configuration.clone())
                    .collect(),
                settings,
            )
            .map_err(|e| e.to_string())?;
            let token = calculator.cancellation_token();
            *worker_interrupt
                .lock()
                .map_err(|_| "RMC cancellation state failed")? =
                Some(Box::new(move || token.cancel()));
            if worker_stopping.load(Ordering::Relaxed) {
                return Err("Stopped before preparation.".into());
            }
            drive(
                request,
                resume,
                &mut calculator,
                commands,
                events.clone(),
                &path,
                prepare_only,
                &worker_stopping,
            )
        })();
        if let Err(e) = result {
            let _ = events.send(Event::Error(e));
        }
    });
    (
        Control {
            tx,
            interrupt,
            stopping,
        },
        rx,
    )
}
#[cfg(not(feature = "refeff-runner"))]
pub fn spawn(
    _request: Request,
    _resume: Option<Box<SavedRun>>,
    _path: PathBuf,
    _prepare_only: bool,
) -> (Control, mpsc::Receiver<Event>) {
    let (tx, _) = mpsc::channel();
    let (events, rx) = mpsc::channel();
    let _ = events.send(Event::Error("This build has no ReFEFF backend.".into()));
    (
        Control {
            tx,
            interrupt: Default::default(),
            stopping: Default::default(),
        },
        rx,
    )
}

#[cfg(feature = "refeff-runner")]
fn cache(calculator: &PreparedRefeffCalculator) -> CacheStats {
    let s = calculator.stats();
    CacheStats {
        exact: s.exact_paths,
        reused_active: s.reused_active_paths,
        requests: s.requests,
        geometry_hits: s.identical_geometry_hits,
        bytes: s.cached_bytes,
        setup_seconds: s.setup_seconds,
        evaluation_seconds: s.evaluation_seconds,
        electronic_preparations: s.electronic_preparations,
        shared_electronic_contexts: s.shared_electronic_contexts,
    }
}
#[cfg(feature = "refeff-runner")]
#[allow(clippy::too_many_arguments)]
fn drive(
    mut request: Request,
    resume: Option<Box<SavedRun>>,
    calculator: &mut PreparedRefeffCalculator,
    commands: mpsc::Receiver<Command>,
    events: mpsc::SyncSender<Event>,
    path: &Path,
    prepare_only: bool,
    stopping: &AtomicBool,
) -> Result<(), String> {
    let started = Instant::now();
    let (mut session, prior_elapsed, mut accepted, mut rejected) = if let Some(saved) = resume {
        // A saved request supplies the original reference and calculator settings.
        if serde_json::to_value(&request.problem).map_err(|e| e.to_string())?
            != serde_json::to_value(&saved.request.problem).map_err(|e| e.to_string())?
        {
            return Err("Resume inputs differ from the saved problem.".into());
        }
        (
            RmcSession::resume(saved.checkpoint, calculator).map_err(|e| e.to_string())?,
            saved.progress.elapsed_seconds,
            saved.progress.accepted,
            saved.progress.constraint_rejected,
        )
    } else {
        (
            RmcSession::new(&request.problem, &request.settings, calculator)
                .map_err(|e| e.to_string())?,
            0.,
            0,
            0,
        )
    };
    session
        .set_step_limit(request.settings.moves.steps)
        .map_err(|e| e.to_string())?;
    let setup_seconds = started.elapsed().as_secs_f64();
    let mut paused = prepare_only;
    let mut last_update = Instant::now();
    let mut last_save = Instant::now();
    let mut working_seconds = setup_seconds;
    let mut active_started = Instant::now();
    let make_progress = |session: &RmcSession,
                         calculator: &PreparedRefeffCalculator,
                         accepted,
                         rejected,
                         elapsed,
                         limit|
     -> Result<Progress, String> {
        Ok(Progress {
            initial: Arc::new(session.initial().clone()),
            best: Arc::new(session.best().clone()),
            current_score: session.current().evaluation.score,
            completed: session.completed(),
            limit,
            accepted,
            constraint_rejected: rejected,
            history: session.history().to_vec(),
            trend: residual_trend(session.history(), &ResidualTrendSettings::default())
                .map_err(|e| e.to_string())?,
            cache: cache(calculator),
            setup_seconds,
            elapsed_seconds: prior_elapsed + elapsed,
            move_scale: request
                .settings
                .adaptation
                .as_ref()
                .map(|_| session.adaptation().scale),
            recent_acceptance: session.adaptation().last_acceptance,
        })
    };
    let initial = make_progress(
        &session,
        calculator,
        accepted,
        rejected,
        working_seconds,
        request.settings.moves.steps,
    )?;
    let saved = SavedRun {
        version: 1,
        request: request.clone(),
        checkpoint: session.checkpoint(),
        progress: initial.clone(),
        status: if paused { "Prepared" } else { "Running" }.into(),
    };
    let persisted = save_run(path, &saved);
    let _ = events.send(Event::Saved(Box::new(saved)));
    persisted?;
    let _ = events.send(Event::Progress(Box::new(initial)));
    if paused {
        let _ = events.send(Event::Paused);
    }
    loop {
        let command = if paused {
            commands.recv().ok()
        } else {
            commands.try_recv().ok()
        };
        if let Some(command) = command {
            match command {
                Command::Stop => {
                    stopping.store(true, Ordering::Relaxed);
                }
                Command::Pause => {
                    if !paused {
                        working_seconds += active_started.elapsed().as_secs_f64();
                        paused = true;
                    }
                }
                Command::Resume(total) => {
                    session.set_step_limit(total).map_err(|e| e.to_string())?;
                    request.settings.moves.steps = total;
                    if paused {
                        active_started = Instant::now();
                        paused = false;
                    }
                }
            }
            if paused && !stopping.load(Ordering::Relaxed) {
                let mut p = make_progress(
                    &session,
                    calculator,
                    accepted,
                    rejected,
                    working_seconds,
                    request.settings.moves.steps,
                )?;
                p.limit = request.settings.moves.steps;
                let saved = SavedRun {
                    version: 1,
                    request: request.clone(),
                    checkpoint: session.checkpoint(),
                    progress: p,
                    status: "Paused".into(),
                };
                let persisted = save_run(path, &saved);
                let _ = events.send(Event::Saved(Box::new(saved)));
                persisted?;
                let _ = events.send(Event::Paused);
                continue;
            }
        } else if paused {
            return Err(
                "RMC control channel closed while paused; recovery checkpoint retained.".into(),
            );
        }
        let mut failure = None;
        let mut done = stopping.load(Ordering::Relaxed);
        if !done {
            match session.step(calculator) {
                Ok(Some(step)) => {
                    accepted += usize::from(step.accepted);
                    rejected += usize::from(step.constraint_rejected);
                }
                Ok(None) => done = true,
                Err(e) => {
                    done = true;
                    if !stopping.load(Ordering::Relaxed) {
                        failure = Some(e.to_string());
                    }
                }
            }
        }
        let elapsed = working_seconds
            + if paused {
                0.
            } else {
                active_started.elapsed().as_secs_f64()
            };
        if done
            || last_update.elapsed() >= Duration::from_millis(500)
            || last_save.elapsed() >= Duration::from_secs(30)
        {
            let mut progress = make_progress(
                &session,
                calculator,
                accepted,
                rejected,
                elapsed,
                request.settings.moves.steps,
            )?;
            progress.limit = request.settings.moves.steps;
            if done || last_save.elapsed() >= Duration::from_secs(30) {
                let status = if failure.is_some() {
                    "Failed"
                } else if stopping.load(Ordering::Relaxed) {
                    "Stopped"
                } else if done {
                    match session.stop_reason() {
                        Some(StopReason::StepLimit) => "Attempt limit reached",
                        Some(StopReason::TargetScore) => "Target score reached",
                        Some(StopReason::Stagnation) => "Stagnation stopping rule",
                        Some(StopReason::LowAcceptance) => "Low-acceptance stopping rule",
                        None => "Finished",
                    }
                } else {
                    "Running"
                };
                let saved = SavedRun {
                    version: 1,
                    request: request.clone(),
                    checkpoint: session.checkpoint(),
                    progress: progress.clone(),
                    status: status.into(),
                };
                let persisted = save_run(path, &saved);
                if done {
                    let _ = events.send(Event::Finished(Box::new(saved)));
                    persisted?;
                    if let Some(error) = failure {
                        return Err(error);
                    }
                    return Ok(());
                }
                let _ = events.send(Event::Saved(Box::new(saved)));
                persisted?;
                last_save = Instant::now();
            }
            if let Err(mpsc::TrySendError::Disconnected(_)) =
                events.try_send(Event::Progress(Box::new(progress)))
            {
                return Err("RMC window closed; use the recovery checkpoint.".into());
            }
            last_update = Instant::now();
        }
    }
}

pub fn pair_distances(text: &str) -> Result<Vec<PairDistance>, String> {
    let mut pairs = Vec::new();
    for entry in text.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        let (elements, minimum) = entry
            .split_once('=')
            .ok_or("Use element pairs such as Cu-O=1.5, Cu-Cu=2.0 (Å).")?;
        let (a, b) = elements
            .trim()
            .split_once('-')
            .ok_or("A pair needs two element symbols separated by '-'.")?;
        let element = |s: &str| {
            rexafs::structure::Element::from_symbol(s.trim())
                .map(|e| e.z)
                .ok_or_else(|| format!("Unknown element '{s}'."))
        };
        let minimum = minimum
            .trim()
            .parse::<f64>()
            .map_err(|_| "Invalid minimum pair distance.")?;
        if !minimum.is_finite() || minimum <= 0. {
            return Err("Pair distances must be finite and positive.".into());
        }
        let mut elements = [element(a)?, element(b)?];
        elements.sort();
        if pairs.iter().any(|p: &PairDistance| p.elements == elements) {
            return Err("Each element pair should be specified once.".into());
        }
        pairs.push(PairDistance { elements, minimum });
    }
    Ok(pairs)
}
pub fn export(parent: &Path, saved: &SavedRun) -> Result<PathBuf, String> {
    use std::io::Write;
    saved.validate()?;
    let directory = tempfile::Builder::new()
        .prefix("rexafs-rmc-")
        .tempdir_in(parent)
        .map_err(|e| e.to_string())?
        .keep();
    save_run(&directory.join("checkpoint.json"), saved)?;
    for (name, state) in [
        ("initial", &saved.progress.initial),
        ("best", &saved.progress.best),
    ] {
        let configuration = &state
            .structures
            .first()
            .ok_or("Missing saved structure")?
            .configuration;
        std::fs::write(
            directory.join(format!("{name}.xyz")),
            configuration.to_xyz().map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        std::fs::write(
            directory.join(format!("{name}-configuration.json")),
            serde_json::to_vec_pretty(configuration).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
    }
    let parameters = serde_json::json!({
        "delta_e0_units": "eV",
        "convention": "theoretical q = sqrt(k^2 - ETOK * delta_e0); experimental energies unchanged",
        "datasets": saved.request.problem.datasets.iter().map(|d| &d.exafs.name).collect::<Vec<_>>(),
        "s02_fixed": saved.request.problem.datasets.iter().map(|d| d.exafs.s02).collect::<Vec<_>>(),
        "initial_delta_e0": saved.progress.initial.energy_shifts(&saved.request.problem).map_err(|e| e.to_string())?,
        "best_delta_e0": saved.progress.best.energy_shifts(&saved.request.problem).map_err(|e| e.to_string())?,
        "energy_refinement": saved.request.settings.energy_refinement,
    });
    std::fs::write(
        directory.join("fit-parameters.json"),
        serde_json::to_vec_pretty(&parameters).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    let data = saved
        .request
        .problem
        .datasets
        .first()
        .ok_or("Missing saved spectrum")?;
    let mut csv = std::fs::File::create(directory.join("fit-k.csv")).map_err(|e| e.to_string())?;
    writeln!(csv, "k,experimental,initial,best").map_err(|e| e.to_string())?;
    for i in 0..data.exafs.k.len() {
        writeln!(
            csv,
            "{:.17},{:.17},{:.17},{:.17}",
            data.exafs.k[i],
            data.exafs.chi[i],
            saved.progress.initial.evaluation.datasets[0].chi[i],
            saved.progress.best.evaluation.datasets[0].chi[i]
        )
        .map_err(|e| e.to_string())?;
    }
    if let Objective::R(transform) = &data.objective {
        let transformed = [
            &data.exafs.chi,
            &saved.progress.initial.evaluation.datasets[0].chi,
            &saved.progress.best.evaluation.datasets[0].chi,
        ]
        .into_iter()
        .map(|chi| {
            transform_spectrum_fourier(&data.exafs.k, chi, data.exafs.kweight, transform)
                .map_err(|e| e.to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
        let mut csv =
            std::fs::File::create(directory.join("fit-r.csv")).map_err(|e| e.to_string())?;
        writeln!(csv, "r,experimental_real,experimental_imag,experimental_magnitude,initial_real,initial_imag,initial_magnitude,best_real,best_imag,best_magnitude").map_err(|e| e.to_string())?;
        for i in 0..transformed[0].r_space.r.len() {
            write!(csv, "{:.17}", transformed[0].r_space.r[i]).map_err(|e| e.to_string())?;
            for curve in &transformed {
                write!(
                    csv,
                    ",{:.17},{:.17},{:.17}",
                    curve.r_space.chir_re[i], curve.r_space.chir_im[i], curve.r_space.chir_mag[i]
                )
                .map_err(|e| e.to_string())?;
            }
            writeln!(csv).map_err(|e| e.to_string())?;
        }
    }
    std::fs::write(
        directory.join("convergence.json"),
        serde_json::to_vec_pretty(&saved.progress.trend).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    let p = &saved.progress;
    let mut report = format!(
        "# RMC result\n\nSource: {}\n\nTermination: {}. Residual trend: {:?}.\n\nInitial objective: {:.10}. Best objective: {:.10}. Structural penalty: {:.10}.\n\nCompleted attempts: {} / {}. Accepted: {}. Constraint rejected: {}.\n\nActive elapsed time: {:.3} s. Current-session setup: {:.3} s.\n\nThe objective is the normalized squared real-plus-imaginary R residual plus configured structural penalties. A completed budget does not establish convergence or structural uniqueness. Exact ReFEFF path updates use fixed reference electronic potentials.\n\nThe checkpoint contains the original processing snapshot, job settings, RNG and best/current states. Configuration JSON preserves the periodic cell; ordinary XYZ does not.\n",
        saved.request.source.label,
        saved.status,
        p.trend.status,
        p.initial.evaluation.score,
        p.best.evaluation.score,
        p.best.penalty,
        p.completed,
        p.limit,
        p.accepted,
        p.constraint_rejected,
        p.elapsed_seconds,
        p.setup_seconds
    );
    report.push_str(&format!("\nFixed S₀²: {:.6}. Initial theoretical ΔE₀: {:+.6} eV. Best theoretical ΔE₀: {:+.6} eV. These parameters match the exported initial/best curves and structures; full precision is in fit-parameters.json. Experimental energy alignment is unchanged.\n",
        data.exafs.s02, saved.progress.initial.energy_shifts(&saved.request.problem).map_err(|e| e.to_string())?[0],
        saved.progress.best.energy_shifts(&saved.request.problem).map_err(|e| e.to_string())?[0]));
    std::fs::write(directory.join("REPORT.md"), report).map_err(|e| e.to_string())?;
    Ok(directory)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    pub(crate) fn spectrum() -> Spectrum {
        let energy: Vec<_> = (0..=650).map(|i| 8800. + 2. * i as f64).collect();
        let mu: Vec<_> = energy
            .iter()
            .map(|e| {
                let above = (e - 8980.).max(0.);
                let k = (above / 3.81).sqrt();
                1. / (1. + (-(e - 8980.) / 2.).exp())
                    + 0.06 * (4. * k).sin() * (-above / 800.).exp() * (above / 20.).min(1.)
            })
            .collect();
        let mut s =
            Spectrum::from_prepared(&energy, &mu, rexafs::analysis::AnalysisSpace::Norm, 8980.)
                .unwrap();
        s.set_name("desktop synthetic Cu")
            .calc_background()
            .unwrap();
        s
    }
    fn request() -> Request {
        let spectrum = spectrum();
        let mut draft = Draft::default();
        draft.ranges.kmin = 3.;
        draft.ranges.kmax = 10.;
        draft.ranges.rmin = 1.2;
        draft.ranges.rmax = 3.;
        draft.options.max_legs = 2;
        draft.options.path_radius = 3.;
        draft.steps = 8;
        let config = Configuration {
            atoms: vec![
                Atom {
                    atomic_number: 29,
                    position: [0.; 3],
                },
                Atom {
                    atomic_number: 8,
                    position: [1.9, 0., 0.],
                },
            ],
            cell: None,
        };
        Request::new(
            &spectrum,
            config,
            &draft,
            Source {
                group_id: None,
                label: "desktop synthetic Cu".into(),
                path: PathBuf::new(),
                fingerprint: 7,
                recipe: PipelineParams::default(),
            },
        )
        .unwrap()
    }
    #[test]
    fn energy_refinement_is_optional_and_checks_full_shifted_coverage() {
        let spectrum = spectrum();
        let reference = request();
        let configuration = reference.problem.structures[0].configuration.clone();
        let mut draft = Draft::default();
        draft.ranges.rmin = 1.2;
        draft.ranges.kmin = 3.;
        draft.ranges.kmax = 10.;
        draft.refine_energy = true;
        draft.s02 = 0.8;
        draft.energy_refinement.bounds = [-600., 15.];
        draft.energy_refinement.initial_step = 1.;
        draft.transform_settings = Some(rexafs::fitting::FeffFitTransform {
            dk: 1.,
            ..Default::default()
        });
        let built = Request::new(
            &spectrum,
            configuration.clone(),
            &draft,
            reference.source.clone(),
        )
        .unwrap();
        assert!(built.settings.energy_refinement.is_some());
        assert_eq!(built.problem.datasets[0].exafs.s02, 0.8);
        assert!(built.calculator.kmax > draft.options.kmax);
        draft.energy_refinement.bounds[1] = 50.;
        assert!(
            Request::new(
                &spectrum,
                configuration.clone(),
                &draft,
                reference.source.clone()
            )
            .is_err()
        );
        draft.refine_energy = false;
        assert!(
            Request::new(&spectrum, configuration, &draft, reference.source)
                .unwrap()
                .settings
                .energy_refinement
                .is_none()
        );
        let old: Draft = serde_json::from_str("{}").unwrap();
        assert!(!old.refine_energy);
    }

    #[cfg(feature = "refeff-runner")]
    #[test]
    fn energy_worker_exports_shifts_matching_saved_spectra_and_preserves_fixed_amplitude() {
        let mut request = request();
        request.settings.energy_refinement = Some(EnergyRefinement {
            bounds: [-2., 2.],
            interval: 2,
            ..Default::default()
        });
        request.settings.moves.steps = 4;
        request.problem.datasets[0].exafs.s02 = 0.8;
        let original = serde_json::to_value(&request.problem).unwrap();
        let directory = tempfile::tempdir().unwrap();
        let (control, events) = spawn(
            request,
            None,
            directory.path().join("energy-run.json"),
            false,
        );
        let saved = loop {
            match events.recv_timeout(Duration::from_secs(60)).unwrap() {
                Event::Finished(saved) => break saved,
                Event::Error(e) => panic!("{e}"),
                _ => {}
            }
        };
        drop(control);
        saved.validate().unwrap();
        assert_eq!(
            serde_json::to_value(&saved.request.problem).unwrap(),
            original
        );
        assert_eq!(
            saved
                .progress
                .history
                .iter()
                .filter(|r| r.energy.is_some())
                .map(|r| r.step)
                .collect::<Vec<_>>(),
            [2, 4]
        );
        let loaded = load_run(&directory.path().join("energy-run.json")).unwrap();
        assert_eq!(loaded.progress.best.delta_e0, saved.progress.best.delta_e0);
        let exported = export(directory.path(), &loaded).unwrap();
        let parameters: serde_json::Value =
            serde_json::from_slice(&std::fs::read(exported.join("fit-parameters.json")).unwrap())
                .unwrap();
        assert_eq!(parameters["s02_fixed"], serde_json::json!([0.8]));
        assert_eq!(
            parameters["best_delta_e0"],
            serde_json::to_value(&loaded.progress.best.delta_e0).unwrap()
        );
        let mut calc = PreparedRefeffCalculator::new(
            loaded.request.calculator.clone(),
            loaded
                .request
                .problem
                .structures
                .iter()
                .map(|s| s.configuration.clone())
                .collect(),
            loaded.request.acceleration_settings().unwrap(),
        )
        .unwrap();
        let resumed = RmcSession::resume(loaded.checkpoint.clone(), &mut calc).unwrap();
        assert_eq!(resumed.best(), loaded.progress.best.as_ref());
    }

    #[test]
    fn path_coverage_message_uses_the_requested_fit_range() {
        let warning = path_coverage_warning(6., 4.).unwrap();
        assert!(warning.contains("6.0 Å"));
        assert!(warning.contains("4.0 Å"));
        assert!(path_coverage_warning(4., 4.).is_none());
        assert!(path_coverage_warning(3., 5.).is_none());
        assert!(path_coverage_warning(f64::NAN, 4.).is_none());
    }

    #[test]
    fn form_validation_and_continuation_preserve_the_saved_budget() {
        let spectrum = spectrum();
        let request = request();
        let configuration = &request.problem.structures[0].configuration;
        let mut draft = Draft::default();
        draft.ranges.rmin = 1.2;
        assert!(draft.validate_form(&spectrum, configuration).is_ok());
        draft.ranges.rmin = 0.1;
        assert!(
            draft
                .validate_form(&spectrum, configuration)
                .unwrap_err()
                .contains("Rbkg")
        );
        draft.ranges.rmin = 1.2;
        draft.step_size = 0.;
        assert!(
            draft
                .validate_form(&spectrum, configuration)
                .unwrap_err()
                .contains("Move size")
        );
        draft.step_size = 0.03;
        draft.fixed_atoms = "0,1".into();
        assert!(
            draft
                .validate_form(&spectrum, configuration)
                .unwrap_err()
                .contains("movable")
        );
        draft.fixed_atoms = "0".into();
        draft.s02 = 0.;
        assert!(
            draft
                .validate_form(&spectrum, configuration)
                .unwrap_err()
                .contains("S₀²")
        );
        assert_eq!(continuation_limit(80, 100, 0).unwrap(), 100);
        assert_eq!(continuation_limit(100, 100, 25).unwrap(), 125);
        assert!(continuation_limit(100, 100, 0).is_err());
        assert!(continuation_limit(1_000_000_000, 1_000_000_000, 1).is_err());
        assert!(continuation_limit(usize::MAX, usize::MAX, 1).is_err());
    }

    #[test]
    fn input_snapshot_retains_processing_and_rejects_unsupported_modes() {
        let spectrum = spectrum();
        let before = serde_json::to_value(&spectrum).unwrap();
        let request = request();
        let configuration = request.problem.structures[0].configuration.clone();
        let mut draft = Draft::default();
        draft.ranges.rmin = 1.2;
        let good = Request::new(
            &spectrum,
            configuration.clone(),
            &draft,
            request.source.clone(),
        )
        .unwrap();
        assert_eq!(serde_json::to_value(&spectrum).unwrap(), before);
        assert_eq!(
            serde_json::to_value(good.problem.datasets[0].source.as_ref().unwrap().spectrum())
                .unwrap(),
            before
        );
        draft.ranges.rmin = 0.1;
        assert!(
            Request::new(
                &spectrum,
                configuration.clone(),
                &draft,
                request.source.clone()
            )
            .is_err()
        );
        draft.ranges.rmin = 1.2;
        draft.ranges.fitspace = FitSpaceSpec::K;
        assert!(
            Request::new(
                &spectrum,
                configuration.clone(),
                &draft,
                request.source.clone()
            )
            .is_err()
        );
        draft.ranges.fitspace = FitSpaceSpec::R;
        draft.ranges.kweights = vec![1., 2.];
        assert!(Request::new(&spectrum, configuration, &draft, request.source).is_err());
    }
    #[test]
    fn positive_calibration_retains_native_window_without_unresolved_low_k() {
        let spectrum = spectrum();
        let reference = request();
        let mut draft = Draft {
            delta_e0: 8.76,
            ..Default::default()
        };
        draft.ranges.kmin = 4.;
        draft.ranges.kmax = 10.;
        draft.ranges.rmin = 1.2;
        let request = Request::new(
            &spectrum,
            reference.problem.structures[0].configuration.clone(),
            &draft,
            reference.source.clone(),
        )
        .unwrap();
        let data = &request.problem.datasets[0].exafs;
        assert!(data.k[0] > 1.5 && data.k[0] <= 2.);
        assert!(data.k.last().unwrap() >= &12.);
        assert!(data.theoretical_k().unwrap().iter().all(|k| k.is_finite()));
        draft.delta_e0 = 40.;
        assert!(
            draft
                .validate_form(&spectrum, &reference.problem.structures[0].configuration)
                .unwrap_err()
                .contains("k min above")
        );
    }
    #[test]
    fn spectrum_defaults_and_live_builder_preserve_data_and_bound_resources() {
        let spectrum = spectrum();
        let before = serde_json::to_value(&spectrum).unwrap();
        let mut draft = Draft {
            s02: 0.85,
            ..Default::default()
        };
        let recipe = PipelineParams {
            fft_kmin: Some(3.),
            fft_kmax: Some(30.),
            fft_kweight: Some(2.),
            ..Default::default()
        };
        draft.use_spectrum_ranges(&spectrum, &recipe).unwrap();
        assert_eq!(draft.s02, 0.85);
        assert_eq!(draft.ranges.kmin, 3.);
        assert_eq!(
            draft.ranges.kmax,
            *spectrum.k().unwrap().iter().last().unwrap()
        );
        assert_eq!(draft.transform().dk, 1.); // Processing default, not fit default 4.
        assert!(
            (draft.ranges.rmin - crate::fitting::spectrum_rbkg(&spectrum).unwrap() - 0.15).abs()
                < 1e-12
        );
        assert_eq!(draft.options.path_criteria, [0., 0.]);
        assert_eq!(serde_json::to_value(&spectrum).unwrap(), before);
        let structure = rexafs::structure::BuiltinLibrary::get()
            .unwrap()
            .structure("cu2o_cuprite")
            .unwrap();
        let small = build_preview(&structure, [2; 3]).unwrap();
        let large = build_preview(&structure, [3; 3]).unwrap();
        assert_eq!(small.atoms.len(), 48);
        assert_eq!(large.atoms.len(), 162);
        assert!(large.cell.unwrap()[0][0] > small.cell.unwrap()[0][0]);
        assert!(build_preview(&structure, [0, 2, 2]).is_err());
        assert!(build_preview(&structure, [10_000; 3]).is_err());
        assert_eq!(small.atoms.len(), 48); // Last valid preview remains owned.
        let request = Request::new(&spectrum, small, &draft, request().source).unwrap();
        assert_eq!(request.settings.moves.steps, 10_000);
        assert_eq!(request.settings.moves.step_size, 0.05);
    }

    #[test]
    fn copied_transform_and_extended_calculation_support_are_retained() {
        use rexafs::xafs::xafsutils::FTWindow;
        let spectrum = spectrum();
        let recipe = PipelineParams {
            fft_kmin: Some(3.5),
            fft_kmax: Some(16.),
            fft_dk: Some(1.4),
            fft_dk2: Some(0.8),
            fft_window: Some(FTWindow::Hanning),
            fft_kweight: Some(3.),
            fft_kstep: Some(0.05),
            fft_nfft: Some(4096),
            bft_rmin: Some(1.4),
            bft_rmax: Some(4.5),
            bft_dr: Some(0.3),
            bft_dr2: Some(0.7),
            bft_window: Some(FTWindow::Parzen),
            ..Default::default()
        };
        let mut draft = Draft {
            delta_e0: -10.,
            ..Default::default()
        };
        draft.use_spectrum_ranges(&spectrum, &recipe).unwrap();
        let expected = draft.transform();
        assert_eq!(
            (expected.kmin, expected.kmax, expected.rmin, expected.rmax),
            (3.5, 16., 1.4, 4.5)
        );
        assert_eq!(
            (expected.dk, expected.dk2, expected.window),
            (1.4, Some(0.8), FTWindow::Hanning)
        );
        assert_eq!(
            (expected.dr, expected.dr2, expected.rwindow),
            (0.3, Some(0.7), FTWindow::Parzen)
        );
        assert_eq!(
            (expected.kstep, expected.nfft, expected.primary_kweight()),
            (Some(0.05), 4096, 3.)
        );
        let reference = request();
        let mut source = reference.source;
        source.recipe = recipe;
        let request = Request::new(
            &spectrum,
            reference.problem.structures[0].configuration.clone(),
            &draft,
            source,
        )
        .unwrap();
        assert_eq!(
            request.problem.datasets[0].objective,
            Objective::R(expected)
        );
        assert!(request.calculator.kmax > 16.);
        assert!(
            request.problem.datasets[0]
                .exafs
                .theoretical_k()
                .unwrap()
                .iter()
                .all(|k| *k <= request.calculator.kmax)
        );
        let restored: Request =
            serde_json::from_value(serde_json::to_value(&request).unwrap()).unwrap();
        assert_eq!(restored.problem, request.problem);
        // A manual range edit keeps the copied taper; old projects keep their original default.
        draft.ranges.kmax = 14.;
        assert_eq!(draft.transform().kmax, 14.);
        assert_eq!(draft.transform().dk, 1.4);
        let legacy: Draft = serde_json::from_str("{}").unwrap();
        assert_eq!(legacy.transform().dk, 4.);
    }

    #[cfg(feature = "refeff-runner")]
    #[test]
    fn older_requests_keep_legacy_electronic_order_and_resource_identity() {
        let request = request();
        assert!(request.reuse_electronic_inputs);
        let mut value = serde_json::to_value(request).unwrap();
        value
            .as_object_mut()
            .unwrap()
            .remove("reuse_electronic_inputs");
        let restored: Request = serde_json::from_value(value).unwrap();
        let settings = restored.acceleration_settings().unwrap();
        assert!(!settings.reuse_electronic_inputs);
        assert_eq!(
            settings.max_contexts,
            AccelerationSettings::default().max_contexts
        );
    }

    #[cfg(feature = "refeff-runner")]
    #[test]
    fn all_256_absorbers_receive_contexts_without_repeating_shared_datasets() {
        let mut request = request();
        let structure = rexafs::structure::BuiltinLibrary::get()
            .unwrap()
            .structure("cu")
            .unwrap();
        let configuration = build_preview(&structure, [4; 3]).unwrap();
        assert_eq!(configuration.atoms.len(), 256);
        request.problem.structures[0].configuration = configuration;
        request.problem.datasets[0].exafs.absorbers = (0..256).collect();
        request
            .problem
            .datasets
            .push(request.problem.datasets[0].clone());
        let settings = request.acceleration_settings().unwrap();
        assert_eq!(settings.max_contexts, 256);
        assert_eq!(
            settings.max_total_paths,
            AccelerationSettings::default().max_total_paths
        );
        request.problem.datasets[1].exafs.edge = Edge::L3;
        assert_eq!(request.acceleration_settings().unwrap().max_contexts, 512);
    }

    #[cfg(feature = "refeff-runner")]
    #[test]
    #[ignore = "manual native qualification: prepares electronic contexts for all 256 Cu sites"]
    fn native_256_site_cell_prepares_every_selected_absorber() {
        let structure = rexafs::structure::BuiltinLibrary::get()
            .unwrap()
            .structure("cu")
            .unwrap();
        let configuration = build_preview(&structure, [4; 3]).unwrap();
        let mut request = request();
        request.problem.structures[0].configuration = configuration.clone();
        request.problem.datasets[0].exafs.absorbers = (0..256).collect();
        // Qualify allocation and real electronic preparation with a short,
        // single-scattering calculation; this is not an experimental fit.
        request.calculator.cluster_radius = 4.;
        request.calculator.path_radius = 3.;
        request.calculator.max_legs = 2;
        request.calculator.kmax = 20.;
        let settings = request.acceleration_settings().unwrap();
        let mut calculator = PreparedRefeffCalculator::new(
            request.calculator.clone(),
            vec![configuration.clone()],
            settings,
        )
        .unwrap();
        for absorber in 0..256 {
            let result = calculator
                .calculate_request(CalculationRequest {
                    structure: 0,
                    configuration: &configuration,
                    absorber,
                    edge: Edge::K,
                    k: &[3., 10., 16., 18.],
                    options: None,
                    paths: false,
                })
                .unwrap();
            assert_eq!(result.chi.len(), 4);
            assert!(result.chi.iter().all(|v| v.is_finite()));
            assert!(result.chi.iter().any(|v| v.abs() > 1e-8));
            if absorber % 16 == 15 {
                eprintln!("prepared {} of 256 Cu sites", absorber + 1);
            }
        }
        assert_eq!(calculator.stats().contexts, 256);
    }
    #[test]
    fn atom_and_pair_rules_are_explicit_and_checked() {
        assert_eq!(atom_indices("0, 2 4", 5).unwrap(), vec![0, 2, 4]);
        for value in ["-1", "0 0", "5", "1.2"] {
            assert!(atom_indices(value, 5).is_err());
        }
        let pairs = pair_distances("Cu-O=1.5, Cu-Cu=2.0").unwrap();
        assert_eq!(pairs[0].elements, [8, 29]);
        assert_eq!(pairs[1].minimum, 2.);
        for value in ["Cu-O=NaN", "Cu-O=-1", "Un-O=2", "Cu-O=1,O-Cu=2"] {
            assert!(pair_distances(value).is_err());
        }
        let legacy: crate::project::ProjectFile = serde_json::from_str("{}").unwrap();
        assert_eq!(legacy.fit_mode, FitMode::Path);
    }
    #[cfg(feature = "refeff-runner")]
    #[test]
    fn desktop_worker_prepares_pauses_resumes_and_cold_continues_exactly() {
        let request = request();
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("recovery.json");
        let (control, events) = spawn(request.clone(), None, path.clone(), true);
        let mut saw_initial = false;
        loop {
            match events.recv_timeout(Duration::from_secs(60)).unwrap() {
                Event::Progress(p) => {
                    assert_eq!(p.completed, 0);
                    saw_initial = true;
                }
                Event::Paused => break,
                Event::Error(e) => panic!("{e}"),
                _ => {}
            }
        }
        assert!(saw_initial);
        let prepared = load_run(&path).unwrap();
        assert_eq!(prepared.progress.completed, 0);
        control.resume(4);
        let saved = loop {
            match events.recv_timeout(Duration::from_secs(60)).unwrap() {
                Event::Finished(s) => break s,
                Event::Error(e) => panic!("{e}"),
                _ => {}
            }
        };
        assert_eq!(saved.progress.completed, 4);
        assert!(saved.progress.cache.reused_active + saved.progress.cache.exact > 0);
        drop(control);
        let mut continuation = saved.clone();
        continuation.request.settings.moves.steps = 8;
        let (control, events) = spawn(
            continuation.request.clone(),
            Some(continuation),
            directory.path().join("continued.json"),
            false,
        );
        let continued = loop {
            match events.recv_timeout(Duration::from_secs(60)).unwrap() {
                Event::Finished(s) => break s,
                Event::Error(e) => panic!("{e}"),
                _ => {}
            }
        };
        drop(control);
        let mut calculator = PreparedRefeffCalculator::new(
            request.calculator.clone(),
            vec![request.problem.structures[0].configuration.clone()],
            request.acceleration_settings().unwrap(),
        )
        .unwrap();
        let mut uninterrupted =
            RmcSession::new(&request.problem, &request.settings, &mut calculator).unwrap();
        uninterrupted.run(&mut calculator).unwrap();
        assert_eq!(continued.progress.completed, 8);
        assert_eq!(continued.progress.best.as_ref(), uninterrupted.best());
        assert_eq!(continued.progress.history, uninterrupted.history());
        let loaded = load_run(&directory.path().join("continued.json")).unwrap();
        assert_eq!(loaded.progress.history, continued.progress.history);
        let mut project = crate::project::ProjectFile {
            fit_mode: FitMode::Rmc,
            ..Default::default()
        };
        project.rmc.saved = Some(continued);
        let restored: crate::project::ProjectFile =
            serde_json::from_slice(&serde_json::to_vec(&project).unwrap()).unwrap();
        assert_eq!(restored.fit_mode, FitMode::Rmc);
        assert_eq!(
            restored.rmc.saved.as_ref().unwrap().progress.best.as_ref(),
            uninterrupted.best()
        );
        let exported = export(directory.path(), restored.rmc.saved.as_ref().unwrap()).unwrap();
        assert!(exported.join("best-configuration.json").is_file());
        assert!(exported.join("REPORT.md").is_file());
        assert!(exported.join("fit-r.csv").is_file());
        // Shape and provenance errors are rejected before display or resume.
        let good = restored.rmc.saved.as_ref().unwrap();
        let original = serde_json::to_value(uninterrupted.checkpoint()).unwrap();
        let local = uninterrupted
            .refine_best(
                &LocalRefinementSettings {
                    evaluations: 10,
                    ..Default::default()
                },
                &mut calculator,
            )
            .unwrap();
        assert_eq!(
            original,
            serde_json::to_value(uninterrupted.checkpoint()).unwrap()
        );
        diagnostics::export_refinement(&exported, good, &local).unwrap();
        let csv = std::fs::read_to_string(exported.join("refined-fit-k.csv")).unwrap();
        let values: Vec<f64> = csv
            .lines()
            .skip(1)
            .map(|row| row.split(',').nth(3).unwrap().parse().unwrap())
            .collect();
        assert_eq!(values, local.best.evaluation.datasets[0].chi);
        let mut invalid_local = local.clone();
        invalid_local.calculator.push_str("-different");
        assert!(diagnostics::refinement_progress(good, &invalid_local).is_err());
        let mut invalid_local = local;
        invalid_local.session_settings.moves.seed += 1;
        assert!(diagnostics::refinement_progress(good, &invalid_local).is_err());
        let mut invalid = (**good).clone();
        Arc::make_mut(&mut invalid.progress.best)
            .evaluation
            .datasets
            .clear();
        assert!(invalid.validate().is_err());
        let mut invalid = (**good).clone();
        invalid.request.problem.datasets[0].exafs.chi[0] += 1.;
        assert!(invalid.validate().is_err());
        let mut invalid = (**good).clone();
        invalid.request.settings.moves.step_size *= 2.;
        assert!(invalid.validate().is_err());
    }
    #[cfg(feature = "refeff-runner")]
    #[test]
    fn running_worker_emits_live_progress_and_saves_pause_and_stop() {
        let mut request = request();
        request.settings.moves.steps = 100_000;
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("run-live/checkpoint.json");
        let (control, events) = spawn(request, None, path.clone(), false);
        let live_completed = loop {
            match events.recv_timeout(Duration::from_secs(60)).unwrap() {
                Event::Progress(p) if p.completed > 0 => break p.completed,
                Event::Error(e) => panic!("{e}"),
                Event::Finished(_) => panic!("Budget exhausted before live update"),
                _ => {}
            }
        };
        control.pause();
        loop {
            match events.recv_timeout(Duration::from_secs(60)).unwrap() {
                Event::Paused => break,
                Event::Error(e) => panic!("{e}"),
                _ => {}
            }
        }
        let paused = load_run(&path).unwrap();
        assert!(paused.progress.completed >= live_completed);
        assert_eq!(paused.status, "Paused");
        assert_eq!(latest_recovery(directory.path()).unwrap(), path);
        control.stop();
        let stopped = loop {
            match events.recv_timeout(Duration::from_secs(60)).unwrap() {
                Event::Finished(s) => break s,
                Event::Error(e) => panic!("{e}"),
                _ => {}
            }
        };
        assert_eq!(stopped.status, "Stopped");
        assert_eq!(stopped.progress.history, paused.progress.history);
        assert_eq!(
            load_run(&path).unwrap().progress.completed,
            paused.progress.completed
        );
    }
}
