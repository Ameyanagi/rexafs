//! Completed-file acquisition. Producers are read-only; durable records are
//! committed before the UI sees them. Spectra live in a private disk cache.
use crate::{
    group_identity::GroupId,
    live_intake::{CompletionEvidence, CompletionPolicy, CompletionTracker, Observation, Snapshot},
    params::{DerivedSpectrum, Operation, PipelineParams},
    series_measurements::{
        FrameInput, FrameStatus, MetricDefinition, MetricRow, SeriesFrame, calculate_row,
    },
};
use rexafs::xafs::io::reader::{Measurement, MeasurementScan, SpectrumMapping};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Instant,
};

mod store;
pub use store::{LiveStore, discover, root};

#[derive(Clone, Serialize, Deserialize)]
pub struct LiveConfig {
    pub schema: u32,
    pub id: GroupId,
    pub name: String,
    pub folder: PathBuf,
    /// A case-insensitive filename glob; no path traversal or producer writes.
    pub pattern: String,
    pub recursive: bool,
    pub policy: CompletionPolicy,
    pub settings: PipelineParams,
    pub definition: MetricDefinition,
    pub layouts: Vec<ScanLayout>,
    #[serde(default)]
    pub recipe: Option<Arc<crate::series_measurements::AnalysisRecipe>>,
    pub created: String,
}

/// Compare column identities, units, header-supported arithmetic and warnings.
/// A differing scan ID/name or point count does not change its interpretation.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct ScanLayout {
    format: String,
    columns: Vec<(String, Option<String>)>,
    candidates: Vec<rexafs::xafs::io::reader::SignalCandidate>,
    warnings: Vec<String>,
    document_warnings: Vec<String>,
    modes: Option<String>,
    crystal_spacing: Option<String>,
    mapping: SpectrumMapping,
}
impl ScanLayout {
    pub fn capture(
        document: &Measurement,
        scan: &MeasurementScan,
        mapping: SpectrumMapping,
    ) -> Result<Self, String> {
        scan.arrays(Some(&mapping)).map_err(|e| e.to_string())?;
        if scan.columns.iter().any(|c| c.name.starts_with("column_")) {
            return Err(
                "Live mapping needs named columns; review and label the producer's layout first"
                    .into(),
            );
        }
        Ok(Self {
            format: document.format.clone(),
            columns: scan
                .columns
                .iter()
                .map(|c| (c.name.clone(), c.units.clone()))
                .collect(),
            candidates: scan.signals.clone(),
            warnings: scan.warnings.clone(),
            document_warnings: document.warnings.clone(),
            modes: scan.metadata.get("9809_modes").cloned(),
            crystal_spacing: crystal_spacing(scan),
            mapping,
        })
    }
    fn matches(&self, document: &Measurement, scan: &MeasurementScan) -> bool {
        self.format == document.format
            && self.document_warnings == document.warnings
            && self.warnings == scan.warnings
            && self.candidates == scan.signals
            && self.modes.as_ref() == scan.metadata.get("9809_modes")
            && self.crystal_spacing == crystal_spacing(scan)
            && self.columns
                == scan
                    .columns
                    .iter()
                    .map(|c| (c.name.clone(), c.units.clone()))
                    .collect::<Vec<_>>()
    }
}

fn crystal_spacing(scan: &MeasurementScan) -> Option<String> {
    scan.header
        .lines()
        .find(|l| l.contains("Mono"))?
        .split_once("D=")?
        .1
        .split_whitespace()
        .next()
        .map(str::to_owned)
}

/// Use the selected, previously reviewed import only when its column identities,
/// units and header-supported choices match the preview. Otherwise require one
/// unambiguous detected signal; column order alone never chooses a detector.
pub fn preview_mapping(
    scan: &MeasurementScan,
    reviewed: Option<&serde_json::Value>,
) -> Result<SpectrumMapping, String> {
    if let Some(parameters) = reviewed {
        let source: Option<MeasurementScan> = parameters
            .get("source_record")
            .and_then(|v| serde_json::from_value(v.clone()).ok());
        let mapping: Option<SpectrumMapping> = parameters
            .get("mapping")
            .and_then(|v| serde_json::from_value(v.clone()).ok());
        if let (Some(source), Some(mapping)) = (source, mapping)
            && source
                .columns
                .iter()
                .map(|c| (&c.name, &c.units))
                .eq(scan.columns.iter().map(|c| (&c.name, &c.units)))
            && source.signals == scan.signals
            && source.warnings == scan.warnings
            && source.metadata.get("9809_modes") == scan.metadata.get("9809_modes")
            && crystal_spacing(&source) == crystal_spacing(scan)
        {
            scan.arrays(Some(&mapping)).map_err(|e| e.to_string())?;
            return Ok(mapping);
        }
    }
    match scan.signals.as_slice() {
        [signal] => Ok(signal.mapping.clone()),
        _ => Err("Select a previously reviewed spectrum with this layout, or import this sample and choose its signal first".into()),
    }
}

impl LiveConfig {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != 1 || !self.folder.is_absolute() || self.name.trim().is_empty() {
            return Err(
                "Live needs a supported configuration, a name and an absolute folder".into(),
            );
        }
        if self.pattern.is_empty() || self.pattern.len() > 200 || self.pattern.contains(['/', '\\'])
        {
            return Err("Use a filename filter such as *.qd or *.xdi".into());
        }
        self.policy.validate()?;
        if self.layouts.is_empty() {
            return Err("Preview a representative completed file before Start".into());
        }
        if self.settings.align_to_ref {
            return Err("Live reference-channel alignment is not configured; turn it off before previewing this recipe".into());
        }
        Ok(())
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct LiveFrame {
    pub group: DerivedSpectrum,
    pub row: MetricRow,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct LiveRecord {
    pub key: String,
    pub source: PathBuf,
    pub revision: String,
    pub completion: CompletionEvidence,
    pub completed_at: String,
    pub frames: Vec<LiveFrame>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct LiveSession {
    pub config: LiveConfig,
    pub directory: PathBuf,
    pub series: GroupId,
    pub run: GroupId,
    /// Idempotent publication keys; session reopening always starts paused.
    pub published: BTreeSet<String>,
    pub stopped: bool,
    /// Current locators of retained original bytes; embedded projects relocate these.
    #[serde(default)]
    pub snapshots: BTreeSet<PathBuf>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SourceState {
    Waiting,
    Queued,
    Completed,
    Review(String),
    Missing(String),
    Excluded,
}
#[derive(Clone, Default)]
pub struct LiveProgress {
    pub discovered: usize,
    pub waiting: usize,
    pub queued: usize,
    pub completed: usize,
    pub review: Vec<(PathBuf, String)>,
    pub failed: Vec<(PathBuf, String)>,
}

/// One worker owns a session. Reconciliation finds missed arrivals after pause
/// or restart. At most one source payload is processed at once; backlog is paths.
pub struct LiveEngine {
    pub store: LiveStore,
    tracker: CompletionTracker,
    states: BTreeMap<PathBuf, SourceState>,
    observed: BTreeMap<PathBuf, crate::live_intake::Fingerprint>,
    cursor: Option<PathBuf>,
    baseline: BTreeMap<PathBuf, String>,
}

fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

pub fn matching_files(
    folder: &Path,
    pattern: &str,
    recursive: bool,
) -> Result<Vec<PathBuf>, String> {
    if !folder.is_dir() {
        return Err(format!("Live folder is unavailable: {}", folder.display()));
    }
    matching_files_until(folder, pattern, recursive, &AtomicBool::new(false))
}

fn matching_files_until(
    folder: &Path,
    pattern: &str,
    recursive: bool,
    cancel: &AtomicBool,
) -> Result<Vec<PathBuf>, String> {
    let mut paths = Vec::new();
    for entry in walkdir::WalkDir::new(folder)
        .follow_links(false)
        .max_depth(if recursive { usize::MAX } else { 1 })
    {
        if cancel.load(Ordering::Relaxed) {
            return Ok(Vec::new());
        }
        let entry = entry.map_err(|e| e.to_string())?;
        if entry.file_type().is_file()
            && filename_matches(&entry.file_name().to_string_lossy(), pattern)
        {
            paths.push(entry.path().to_path_buf());
        }
    }
    paths.sort();
    Ok(paths)
}

fn filename_matches(name: &str, pattern: &str) -> bool {
    let name = name.to_lowercase();
    let pattern = pattern.to_lowercase();
    let n: Vec<_> = name.chars().collect();
    let p: Vec<_> = pattern.chars().collect();
    let (mut i, mut j, mut star, mut resume) = (0, 0, None, 0);
    while i < n.len() {
        if j < p.len() && (p[j] == '?' || p[j] == n[i]) {
            i += 1;
            j += 1;
        } else if j < p.len() && p[j] == '*' {
            star = Some(j);
            resume = i;
            j += 1;
        } else if let Some(s) = star {
            resume += 1;
            i = resume;
            j = s + 1;
        } else {
            return false;
        }
    }
    while j < p.len() && p[j] == '*' {
        j += 1;
    }
    j == p.len()
}

impl LiveEngine {
    pub fn open(store: LiveStore) -> Result<Self, String> {
        let mut tracker = CompletionTracker::new(store.config.policy.clone())?;
        let mut states = BTreeMap::new();
        for (source, revision) in store.latest()? {
            states.insert(source.clone(), SourceState::Completed);
            tracker.restore_accepted(source, revision)?;
        }
        let baseline = store.baseline()?;
        Ok(Self {
            store,
            tracker,
            states,
            observed: BTreeMap::new(),
            cursor: None,
            baseline,
        })
    }
    pub fn retry(&mut self) {
        self.states
            .retain(|_, state| !matches!(state, SourceState::Review(_) | SourceState::Missing(_)));
    }
    pub fn progress(&self) -> LiveProgress {
        let mut out = LiveProgress {
            discovered: self.states.len(),
            ..Default::default()
        };
        for (path, state) in &self.states {
            match state {
                SourceState::Waiting => out.waiting += 1,
                SourceState::Queued => out.queued += 1,
                SourceState::Completed => out.completed += 1,
                SourceState::Review(reason) => out.review.push((path.clone(), reason.clone())),
                SourceState::Missing(reason) => out.failed.push((path.clone(), reason.clone())),
                SourceState::Excluded => {}
            }
        }
        out
    }
    /// Reconcile all locators, then observe a bounded fair slice. No scientific
    /// frame is dropped when the per-poll budget is reached.
    pub fn poll(
        &mut self,
        now: Instant,
        budget: usize,
        cancel: &AtomicBool,
    ) -> Result<Vec<LiveRecord>, String> {
        let config = &self.store.config;
        if cancel.load(Ordering::Relaxed) {
            return Ok(Vec::new());
        }
        if !config.folder.is_dir() {
            return Err("Live folder is unavailable".into());
        }
        let paths =
            matching_files_until(&config.folder, &config.pattern, config.recursive, cancel)?;
        if cancel.load(Ordering::Relaxed) {
            return Ok(Vec::new());
        }
        let found: BTreeSet<_> = paths.iter().cloned().collect();
        for (path, state) in &mut self.states {
            if !found.contains(path)
                && !matches!(state, SourceState::Completed | SourceState::Excluded)
            {
                *state =
                    SourceState::Missing("Source is unavailable; retry after it returns".into());
            }
        }
        for path in &paths {
            if let Ok(value) = crate::live_intake::Fingerprint::at(path) {
                if self.observed.get(path) != Some(&value) {
                    self.states.insert(path.clone(), SourceState::Queued);
                }
                self.observed.insert(path.clone(), value);
            }
            self.states
                .entry(path.clone())
                .or_insert(SourceState::Queued);
        }
        let paths: Vec<_> = paths
            .into_iter()
            .filter(|path| {
                matches!(
                    self.states.get(path),
                    Some(SourceState::Waiting | SourceState::Queued | SourceState::Missing(_))
                )
            })
            .collect();
        if paths.is_empty() {
            return Ok(Vec::new());
        }
        let start = self
            .cursor
            .as_ref()
            .map_or(0, |cursor| paths.partition_point(|p| p <= cursor))
            % paths.len();
        let mut records = Vec::new();
        for path in paths
            .iter()
            .cycle()
            .skip(start)
            .take(budget.max(1).min(paths.len()))
        {
            if cancel.load(Ordering::Relaxed) {
                break;
            }
            self.cursor = Some(path.clone());
            // Unchanged completed sources do not consume the pending-work budget.
            match self.tracker.observe(path, now) {
                Observation::Waiting { .. } => {
                    self.states.insert(path.clone(), SourceState::Waiting);
                }
                Observation::Unavailable(e) => {
                    self.states.insert(path.clone(), SourceState::Missing(e));
                }
                Observation::NeedsReview(e) => {
                    self.states.insert(path.clone(), SourceState::Review(e));
                }
                Observation::AlreadyAccepted { .. } => {
                    self.states.insert(path.clone(), SourceState::Completed);
                }
                Observation::Ready(snapshot) => {
                    if self.baseline.get(path) == Some(&snapshot.revision) {
                        self.states.insert(path.clone(), SourceState::Excluded);
                        continue;
                    }
                    let key = digest(
                        &serde_json::to_vec(&(path, &snapshot.revision, &self.store.config.id))
                            .map_err(|e| e.to_string())?,
                    );
                    if self.store.contains(&key)? {
                        self.tracker.acknowledge(&snapshot)?;
                        self.states.insert(path.clone(), SourceState::Completed);
                        continue;
                    }
                    match self.process(&snapshot, key, cancel) {
                        Ok(Some(record)) => {
                            self.store.commit(&record)?;
                            self.tracker.acknowledge(&snapshot)?;
                            self.states.insert(path.clone(), SourceState::Completed);
                            records.push(record);
                        }
                        Ok(None) => break,
                        Err(error) => {
                            self.states.insert(path.clone(), SourceState::Review(error));
                        }
                    }
                }
            }
        }
        Ok(records)
    }

    fn process(
        &self,
        snapshot: &Snapshot,
        key: String,
        cancel: &AtomicBool,
    ) -> Result<Option<LiveRecord>, String> {
        if snapshot.measurement.scans.is_empty() {
            return Err("No convertible scans; review the source datasets".into());
        }
        let config = &self.store.config;
        validate_recipe(config.recipe.as_deref(), &snapshot.bytes, &snapshot.source)?;
        let mut frames = Vec::new();
        let mut ids = BTreeSet::new();
        self.store.snapshot(&snapshot.revision, &snapshot.bytes)?;
        for scan in &snapshot.measurement.scans {
            if cancel.load(Ordering::Relaxed) {
                return Ok(None);
            }
            if !ids.insert(&scan.id) {
                return Err("Repeated scan identity in one source; review the source".into());
            }
            let layouts: Vec<_> = config
                .layouts
                .iter()
                .filter(|l| l.matches(&snapshot.measurement, scan))
                .collect();
            let mapping = layouts.first().ok_or("Layout, units or signal choices changed; review the input and start a revised recipe")?.mapping.clone();
            if layouts.iter().any(|l| l.mapping != mapping) {
                return Err("More than one mapping matches this scan; review the recipe".into());
            }
            let (energy, mu) = scan.arrays(Some(&mapping)).map_err(|e| e.to_string())?;
            let scan_key = digest(
                &serde_json::to_vec(&(&key, &scan.id, &mapping)).map_err(|e| e.to_string())?,
            );
            let source = self.store.spectrum(&scan_key, &energy, &mu, &scan.header)?;
            let group_id = GroupId::source(&source, crate::params::DetectionMode::MuColumn);
            let label = format!(
                "{} · {} · {}",
                snapshot
                    .source
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy(),
                scan.label,
                &snapshot.revision[..8]
            );
            let mut settings = config.settings.clone();
            settings.import = cache_import();
            let group = DerivedSpectrum {
                group_id: Some(group_id.clone()),
                label: label.clone(),
                source: Some(source.clone()),
                params: Some(settings.clone()),
                operation: Some(Operation {
                    tool: "Live acquisition".into(),
                    applied_energy_shift_ev: 0.,
                    inputs: Vec::new(),
                    parameters: serde_json::json!({"source_path":snapshot.source,"source_sha256":snapshot.revision,
                        "scan_id":scan.id,"mapping":mapping,"completion":snapshot.completion,"recipe":config.id,
                        "original_snapshot":self.store.directory.join(format!("{}.raw", snapshot.revision))}),
                }),
                ..Default::default()
            };
            let input = FrameInput {
                group: group_id.clone(),
                label: label.clone(),
                path: source,
                derived: Some(Arc::new(group.clone())),
                settings: settings.clone(),
                recipe: None,
            };
            let (source_digest, revision) = input.revision()?;
            let row = MetricRow {
                frame: SeriesFrame {
                    id: group_id.clone(),
                    group: group_id,
                    label,
                    sequence: 0,
                    coordinate: None,
                    acquired_at: None,
                },
                source_digest: Some(source_digest),
                input_revision: Some(revision),
                settings: Arc::new(settings),
                status: FrameStatus::Pending,
                result: None,
                reason: None,
                preparation: serde_json::Value::Null,
            };
            let row = calculate_row(&input, &row, &config.definition);
            frames.push(LiveFrame { group, row });
        }
        Ok(Some(LiveRecord {
            key,
            source: snapshot.source.clone(),
            revision: snapshot.revision.clone(),
            completion: snapshot.completion.clone(),
            completed_at: chrono::Utc::now().to_rfc3339(),
            frames,
        }))
    }
}

#[cfg(test)]
mod tests;

/// Explicit interpretation of the two-column, eV cache, independent of inference.
fn cache_import() -> crate::params::ImportConfig {
    crate::params::ImportConfig {
        mode: crate::params::DetectionMode::MuColumn,
        axis: crate::import_mapping::AxisConversion::EnergyEv,
        energy_col: Some(0),
        mu_col: Some(1),
        ..Default::default()
    }
}

pub fn validate_recipe(
    recipe: Option<&crate::series_measurements::AnalysisRecipe>,
    bytes: &[u8],
    path: &Path,
) -> Result<(), String> {
    use crate::series_measurements::InputContract;
    let Some(recipe) = recipe else {
        return Ok(());
    };
    recipe.validate()?;
    match &recipe.input {
        InputContract::Materialized(crate::params::Quantity::RawMu) => Ok(()),
        InputContract::Materialized(_) => Err("This recipe expects a processed result, not raw Live μ(E); save a recipe from a raw spectrum".into()),
        InputContract::Table(expected) => {
            let detection = crate::params::detect_import_reader(bytes, path, &recipe.settings.import)?
                .ok_or("Recipe layout could not be checked; review the representative input")?;
            if let Some(error) = detection.mapping_error.as_ref() { return Err(error.clone()); }
            if &crate::import_mapping::LayoutKey::from_detection(&detection) != expected {
                return Err("Saved recipe input is incompatible; review the layout and save a new recipe".into());
            }
            Ok(())
        }
    }
}
