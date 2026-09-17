//! Retained peak fits: bounded spectrum processing and immutable disk artifacts.
use crate::{
    group_identity::GroupId,
    params::PipelineParams,
    series_measurements::{
        FrameInput, FrameStatus, MetricDefinition, SeriesDefinition, resolved_preparation,
    },
};
use rexafs::prelude::{
    FitVariables, Measurement, PeakContribution, PeakFit, PeakFitResult, PeakTermination,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::Arc,
};
mod trends;
pub use trends::PeakMetric;

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PeakArchive {
    pub models: Vec<PeakSavedModel>,
    pub runs: Vec<Arc<PeakRun>>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PeakSavedModel {
    pub id: GroupId,
    pub revision: u64,
    pub name: String,
    pub model: PeakFit,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PeakRun {
    pub id: GroupId,
    pub created: String,
    pub software: String,
    pub name: String,
    pub model: Arc<PeakFit>,
    pub rows: Vec<PeakRow>,
    pub complete: bool,
    pub cancelled: bool,
    /// Frozen catalogue, including physical coordinates and timestamp provenance.
    /// Absent for current/marked fits and older prototype runs.
    #[serde(default)]
    pub series: Option<SeriesDefinition>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PeakRow {
    pub group: GroupId,
    pub label: String,
    pub sequence: usize,
    pub settings: Arc<PipelineParams>,
    pub source_digest: Option<String>,
    pub input_revision: Option<String>,
    pub status: FrameStatus,
    pub reason: Option<String>,
    /// Small summaries stay resident. Curves and full covariance are loaded only
    /// for the selected row from the digest-verified artifact.
    pub summary: Option<PeakSummary>,
    pub artifact: Option<PathBuf>,
    pub artifact_digest: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PeakSummary {
    /// Resolved origin added to all relative model coordinates, in eV.
    #[serde(default)]
    pub origin_ev: Option<f64>,
    pub parameters: FitVariables,
    /// Component summaries; curve arrays are intentionally empty in this record.
    pub components: Vec<PeakContribution>,
    pub termination: PeakTermination,
    pub objective: f64,
    pub peak_center_ev: Option<f64>,
    pub peak_center_standard_error_ev: Option<f64>,
    pub uncertainty_unavailable: Option<String>,
    pub warnings: Vec<String>,
}
impl From<&PeakFitResult> for PeakSummary {
    fn from(r: &PeakFitResult) -> Self {
        Self {
            origin_ev: Some(r.origin_ev),
            parameters: r.parameters.clone(),
            components: r
                .components
                .iter()
                .cloned()
                .map(|mut c| {
                    c.curve.clear();
                    c
                })
                .collect(),
            termination: r.termination.clone(),
            objective: r.objective,
            peak_center_ev: r.peak_center_ev,
            peak_center_standard_error_ev: r.peak_center_standard_error_ev,
            uncertainty_unavailable: r.uncertainty_unavailable.clone(),
            warnings: r.warnings.clone(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PeakRecord {
    pub schema: u32,
    pub group: GroupId,
    pub label: String,
    pub source_digest: String,
    pub input_revision: String,
    /// Requested processing/import settings. Absent only in historical prototype
    /// artifacts, where the containing run row retains them instead.
    #[serde(default)]
    pub settings: Option<Arc<PipelineParams>>,
    pub preparation: serde_json::Value,
    pub result: PeakFitResult,
}

pub fn root() -> Result<PathBuf, String> {
    let parent = crate::settings::env_var_os("SETTINGS")
        .and_then(|p| PathBuf::from(p).parent().map(Path::to_path_buf))
        .or_else(crate::settings::app_dir)
        .ok_or("Application storage unavailable")?;
    Ok(parent.join("peak-fits"))
}
fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn retain(root: &Path, record: &PeakRecord) -> Result<(PathBuf, String), String> {
    let mut dirs = std::fs::DirBuilder::new();
    dirs.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        dirs.mode(0o700);
    }
    dirs.create(root).map_err(|e| e.to_string())?;
    if std::fs::symlink_metadata(root)
        .map_err(|e| e.to_string())?
        .file_type()
        .is_symlink()
    {
        return Err("Peak-fit storage cannot be a symlink".into());
    }
    let compressed = LimitedWriter {
        inner: Vec::new(),
        remaining: 128 * 1024 * 1024,
    };
    let gzip = flate2::write::GzEncoder::new(compressed, flate2::Compression::default());
    let mut expanded = LimitedWriter {
        inner: gzip,
        remaining: 256 * 1024 * 1024,
    };
    serde_json::to_writer(&mut expanded, record).map_err(|e| e.to_string())?;
    let bytes = expanded.inner.finish().map_err(|e| e.to_string())?.inner;
    let hash = digest(&bytes);
    let path = root.join(format!("{hash}.json.gz"));
    let mut temporary = tempfile::NamedTempFile::new_in(root).map_err(|e| e.to_string())?;
    temporary.write_all(&bytes).map_err(|e| e.to_string())?;
    temporary.as_file().sync_all().map_err(|e| e.to_string())?;
    match temporary.persist_noclobber(&path) {
        Ok(_) => {}
        Err(error) if error.error.kind() == std::io::ErrorKind::AlreadyExists => {
            if digest(&std::fs::read(&path).map_err(|e| e.to_string())?) != hash {
                return Err("Historical peak-fit artifact changed".into());
            }
        }
        Err(error) => return Err(error.to_string()),
    }
    Ok((path, hash))
}

/// The same compressed/expanded limits apply while writing and reading artifacts.
struct LimitedWriter<W> {
    inner: W,
    remaining: usize,
}
impl<W: Write> Write for LimitedWriter<W> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.len() > self.remaining {
            return Err(std::io::Error::other(
                "Peak-fit artifact exceeds its storage limit",
            ));
        }
        let written = self.inner.write(bytes)?;
        self.remaining -= written;
        Ok(written)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

pub fn read(row: &PeakRow) -> Result<PeakRecord, String> {
    let path = row
        .artifact
        .as_ref()
        .ok_or("No retained fit for this frame")?;
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut bytes = Vec::new();
    file.take(128 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 128 * 1024 * 1024 {
        return Err("Peak artifact exceeds 128 MiB".into());
    }
    if row.artifact_digest.as_deref() != Some(digest(&bytes).as_str()) {
        return Err("Retained peak-fit checksum mismatch".into());
    }
    let mut json = Vec::new();
    flate2::read::GzDecoder::new(&bytes[..])
        .take(256 * 1024 * 1024 + 1)
        .read_to_end(&mut json)
        .map_err(|e| e.to_string())?;
    if json.len() > 256 * 1024 * 1024 {
        return Err("Expanded peak artifact exceeds 256 MiB".into());
    }
    let record: PeakRecord = serde_json::from_slice(&json).map_err(|e| e.to_string())?;
    if record.schema != 1
        || record.group != row.group
        || Some(&record.input_revision) != row.input_revision.as_ref()
    {
        return Err("Retained peak-fit identity mismatch".into());
    }
    Ok(record)
}

pub fn preparation_definition(model: &PeakFit) -> MetricDefinition {
    let mut measurement = Measurement::mean(model.range[0]..=model.range[1]);
    measurement.space = model.space;
    measurement.origin = model.origin;
    MetricDefinition {
        id: GroupId::new_result(),
        revision: 1,
        name: "Peak fit".into(),
        measurement,
        edge_energy: false,
    }
}

impl PeakRun {
    pub fn new(name: String, model: PeakFit, inputs: &[FrameInput]) -> Self {
        Self {
            id: GroupId::new_result(),
            created: chrono::Utc::now().to_rfc3339(),
            software: env!("CARGO_PKG_VERSION").into(),
            name,
            model: Arc::new(model),
            complete: false,
            cancelled: false,
            series: None,
            rows: inputs
                .iter()
                .enumerate()
                .map(|(i, input)| PeakRow {
                    group: input.group.clone(),
                    label: input.label.clone(),
                    sequence: i + 1,
                    settings: Arc::new(input.settings.clone()),
                    source_digest: None,
                    input_revision: None,
                    status: FrameStatus::Pending,
                    reason: None,
                    summary: None,
                    artifact: None,
                    artifact_digest: None,
                })
                .collect(),
        }
    }
    pub fn freeze(&mut self, inputs: &[FrameInput], cancelled: impl Fn() -> bool) {
        for (row, input) in self.rows.iter_mut().zip(inputs) {
            if cancelled() {
                break;
            }
            match input.revision() {
                Ok((source, revision)) => {
                    row.source_digest = Some(source);
                    row.input_revision = Some(revision);
                }
                Err(e) => {
                    row.status = FrameStatus::Unavailable;
                    row.reason = Some(e);
                }
            }
        }
    }
    pub fn finish(&mut self, cancelled: bool) {
        self.cancelled = cancelled;
        for row in &mut self.rows {
            if row.status == FrameStatus::Pending {
                row.status = if cancelled {
                    FrameStatus::Cancelled
                } else {
                    FrameStatus::Failed
                };
                row.reason = Some(
                    if cancelled {
                        "Cancelled"
                    } else {
                        "Frame was not calculated"
                    }
                    .into(),
                );
            }
        }
        self.complete = !cancelled;
    }
}

pub fn calculate(
    model: &PeakFit,
    row: &PeakRow,
    input: &FrameInput,
    root: &Path,
    cancelled: impl Fn() -> bool,
) -> PeakRow {
    let mut output = row.clone();
    if row.status != FrameStatus::Pending {
        return output;
    }
    let compute = || {
        if row.group != input.group || *row.settings != input.settings {
            return Err("Frozen peak-fit input/settings changed".into());
        }
        let revision = row
            .input_revision
            .as_deref()
            .ok_or("Frame was not frozen")?;
        let (sp, source_digest, input_revision) =
            input.prepare(&preparation_definition(model), Some(revision))?;
        let result = model
            .fit_with_progress(&sp, |_, _| !cancelled())
            .map_err(|e| e.to_string())?;
        let summary = PeakSummary::from(&result);
        let status = match result.termination {
            PeakTermination::Converged | PeakTermination::FixedModel => FrameStatus::Succeeded,
            PeakTermination::Cancelled => FrameStatus::Cancelled,
            PeakTermination::NotConverged => FrameStatus::Failed,
        };
        let reason = (status != FrameStatus::Succeeded).then(|| result.termination_detail.clone());
        let record = PeakRecord {
            schema: 1,
            group: row.group.clone(),
            label: row.label.clone(),
            source_digest,
            input_revision,
            settings: Some(row.settings.clone()),
            preparation: resolved_preparation(&sp),
            result,
        };
        let (path, hash) = retain(root, &record)?;
        Ok::<_, String>((summary, status, reason, path, hash))
    };
    match compute() {
        Ok((summary, status, reason, path, hash)) => {
            output.summary = Some(summary);
            output.status = status;
            output.reason = reason;
            output.artifact = Some(path);
            output.artifact_digest = Some(hash);
        }
        Err(e) => {
            output.status = if cancelled() {
                FrameStatus::Cancelled
            } else {
                FrameStatus::Failed
            };
            output.reason = Some(e);
        }
    }
    output
}

/// CSV exports keep native point indices, component curves, selected quantity
/// and source identity. The JSON export preserves the complete scientific record.
pub fn write_curves(record: &PeakRecord, writer: impl Write) -> Result<(), String> {
    let mut csv = csv::Writer::from_writer(writer);
    let r = &record.result;
    let mut header = vec![
        "energy_eV".into(),
        "source_index".into(),
        "data".into(),
        "model".into(),
        "residual".into(),
    ];
    header.extend(r.components.iter().map(|c| format!("component_{}", c.name)));
    header.extend([
        "representation".into(),
        "source_sha256".into(),
        "input_revision".into(),
        "range_start".into(),
        "range_end".into(),
        "axis_origin".into(),
        "resolved_origin_eV".into(),
        "excluded_ranges".into(),
    ]);
    csv.write_record(&header).map_err(|e| e.to_string())?;
    for i in 0..r.energy.len() {
        let mut row = vec![
            r.energy[i].to_string(),
            r.source_indices[i].to_string(),
            r.data[i].to_string(),
            r.model[i].to_string(),
            r.residual[i].to_string(),
        ];
        row.extend(r.components.iter().map(|c| c.curve[i].to_string()));
        row.extend([
            format!("{:?}", r.definition.space),
            record.source_digest.clone(),
            record.input_revision.clone(),
            r.definition.range[0].to_string(),
            r.definition.range[1].to_string(),
            serde_json::to_string(&r.definition.origin).map_err(|e| e.to_string())?,
            r.origin_ev.to_string(),
            serde_json::to_string(&r.definition.exclude).map_err(|e| e.to_string())?,
        ]);
        csv.write_record(&row).map_err(|e| e.to_string())?;
    }
    csv.flush().map_err(|e| e.to_string())
}

pub fn write_trend(run: &PeakRun, writer: impl Write) -> Result<(), String> {
    let mut csv = csv::Writer::from_writer(writer);
    let metrics = PeakMetric::choices(&run.model);
    let mut header: Vec<String> = [
        "frame",
        "label",
        "status",
        "reason",
        "source_sha256",
        "input_revision",
        "series_id",
        "series_revision",
        "series_ordering",
        "coordinate",
        "coordinate_name",
        "coordinate_unit",
        "coordinate_source",
        "acquired_at",
        "timestamp_meaning",
        "representation",
        "range_start",
        "range_end",
        "axis_origin",
        "resolved_origin_eV",
        "absolute_start_eV",
        "absolute_end_eV",
        "excluded_ranges",
        "run_id",
        "software",
        "termination",
        "warnings",
        "uncertainty_unavailable",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    for m in &metrics {
        header.extend([
            m.key(),
            format!("{}_stderr", m.key()),
            format!("{}_unit", m.key()),
        ]);
    }
    csv.write_record(header).map_err(|e| e.to_string())?;
    let number = |n: Option<f64>| n.map(|n| n.to_string()).unwrap_or_default();
    for (i, r) in run.rows.iter().enumerate() {
        let series = run.series.as_ref();
        let frame = series.and_then(|s| s.frames.get(i));
        let origin = r.summary.as_ref().and_then(|s| s.origin_ev);
        let mut row = vec![
            r.sequence.to_string(),
            r.label.clone(),
            format!("{:?}", r.status),
            r.reason.clone().unwrap_or_default(),
            r.source_digest.clone().unwrap_or_default(),
            r.input_revision.clone().unwrap_or_default(),
            series
                .map(|s| serde_json::to_string(&s.id).unwrap())
                .unwrap_or_default(),
            series.map(|s| s.revision.to_string()).unwrap_or_default(),
            series.map(|s| s.ordering.clone()).unwrap_or_default(),
            number(frame.and_then(|f| f.coordinate)),
            series
                .map(|s| s.coordinate.label.clone())
                .unwrap_or_default(),
            series
                .map(|s| s.coordinate.unit.clone())
                .unwrap_or_default(),
            series
                .map(|s| s.coordinate.source.clone())
                .unwrap_or_default(),
            frame
                .and_then(|f| f.acquired_at.clone())
                .unwrap_or_default(),
            series
                .map(|s| s.coordinate.timestamp_meaning.clone())
                .unwrap_or_default(),
            format!("{:?}", run.model.space),
            run.model.range[0].to_string(),
            run.model.range[1].to_string(),
            serde_json::to_string(&run.model.origin).map_err(|e| e.to_string())?,
            number(origin),
            number(origin.map(|v| v + run.model.range[0])),
            number(origin.map(|v| v + run.model.range[1])),
            serde_json::to_string(&run.model.exclude).map_err(|e| e.to_string())?,
            serde_json::to_string(&run.id).unwrap(),
            run.software.clone(),
            r.summary
                .as_ref()
                .map(|s| format!("{:?}", s.termination))
                .unwrap_or_default(),
            r.summary
                .as_ref()
                .map(|s| s.warnings.join("; "))
                .unwrap_or_default(),
            r.summary
                .as_ref()
                .and_then(|s| s.uncertainty_unavailable.clone())
                .unwrap_or_default(),
        ];
        for m in &metrics {
            let (value, error) = r.summary.as_ref().map(|s| m.value(s)).unwrap_or_default();
            row.extend([number(value), number(error), m.unit(&run.model)]);
        }
        csv.write_record(row).map_err(|e| e.to_string())?;
    }
    csv.flush().map_err(|e| e.to_string())
}

impl PeakArchive {
    pub fn artifacts(&self) -> impl Iterator<Item = &PathBuf> {
        self.runs
            .iter()
            .flat_map(|run| run.rows.iter().filter_map(|r| r.artifact.as_ref()))
    }
    pub fn relocate(
        &mut self,
        map: &mut impl FnMut(&Path) -> Result<PathBuf, String>,
    ) -> Result<(), String> {
        for run in &mut self.runs {
            for row in &mut Arc::make_mut(run).rows {
                if let Some(path) = &mut row.artifact {
                    *path = map(path)?;
                }
            }
        }
        Ok(())
    }
    pub fn save_model(&mut self, name: String, model: PeakFit) {
        let previous = self.models.iter().rev().find(|m| m.name == name);
        let id = previous
            .map(|m| m.id.clone())
            .unwrap_or_else(GroupId::new_result);
        let revision = previous.map_or(1, |m| m.revision + 1);
        self.models.push(PeakSavedModel {
            id,
            revision,
            name,
            model,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn input(root: &Path, name: &str) -> FrameInput {
        let path = root.join(name);
        let mut data = String::from("# energy mu\n");
        for i in 0..=200 {
            let x = 8900. + i as f64 / 20.;
            let y = 0.2 + (-0.5 * ((x - 8905.) / 0.8).powi(2)).exp();
            data.push_str(&format!("{x:.6} {y:.15}\n"));
        }
        std::fs::write(&path, data).unwrap();
        let settings = PipelineParams {
            import: crate::params::ImportConfig {
                mode: crate::params::DetectionMode::MuColumn,
                energy_col: Some(0),
                mu_col: Some(1),
                ..Default::default()
            },
            ..Default::default()
        };
        FrameInput {
            group: GroupId::source(&path, crate::params::DetectionMode::MuColumn),
            label: name.into(),
            path,
            derived: None,
            settings,
            recipe: None,
        }
    }
    fn model() -> PeakFit {
        PeakFit::new(8900.0..=8910.0)
            .raw_mu()
            .absolute()
            .gaussian("p", 8904.5, 1.8, 2.)
            .constant_baseline(0.1)
    }

    #[test]
    fn peak_fit_batch_retains_failures_without_resident_curves_and_detects_changed_inputs() {
        let tmp = tempfile::tempdir().unwrap();
        let inputs = vec![input(tmp.path(), "a.dat"), input(tmp.path(), "b.dat")];
        let mut run = PeakRun::new("synthetic".into(), model(), &inputs);
        run.freeze(&inputs, || false);
        std::fs::write(&inputs[1].path, "# energy mu\n8900 1\n8901 2\n").unwrap();
        let root = tmp.path().join("artifacts");
        for (row, input) in run.rows.iter_mut().zip(&inputs) {
            *row = calculate(&run.model, row, input, &root, || false);
        }
        run.finish(false);
        assert_eq!(run.rows.len(), 2);
        assert_eq!(
            run.rows[0].status,
            FrameStatus::Succeeded,
            "{:?}",
            run.rows[0].reason
        );
        assert_eq!(run.rows[1].status, FrameStatus::Failed);
        assert!(
            run.rows[1]
                .reason
                .as_ref()
                .unwrap()
                .contains("Inputs changed")
        );
        assert!(
            run.rows[0]
                .summary
                .as_ref()
                .unwrap()
                .components
                .iter()
                .all(|c| c.curve.is_empty())
        );
        let record = read(&run.rows[0]).unwrap();
        assert_eq!(record.result.points, 201);
        assert!((record.result.parameters.vars["p_center"].value - 8905.).abs() < 1e-6);
        assert!(
            (record.result.parameters.vars["p_area"].value
                - 0.8 * (2. * std::f64::consts::PI).sqrt())
            .abs()
                < 1e-6
        );
        assert_eq!(
            record.result.definition.parameters.vars["p_center"].value,
            8904.5
        );
        std::fs::write(run.rows[0].artifact.as_ref().unwrap(), b"damaged").unwrap();
        assert!(read(&run.rows[0]).err().unwrap().contains("checksum"));
    }

    #[test]
    fn peak_fit_embedded_project_relocates_history_without_importing_artifacts() {
        let tmp = tempfile::tempdir().unwrap();
        let inputs = vec![input(tmp.path(), "a.dat")];
        let mut run = PeakRun::new("synthetic".into(), model(), &inputs);
        run.freeze(&inputs, || false);
        let root = tmp.path().join("artifacts");
        run.rows[0] = calculate(&run.model, &run.rows[0], &inputs[0], &root, || false);
        run.finish(false);
        assert_eq!(
            run.rows[0].status,
            FrameStatus::Succeeded,
            "{:?}",
            run.rows[0].reason
        );
        let expected = read(&run.rows[0]).unwrap().result.model;
        let archive = PeakArchive {
            runs: vec![Arc::new(run)],
            ..Default::default()
        };
        let project = crate::project::ProjectFile {
            peak_fits: archive,
            ..Default::default()
        };
        let path = tmp.path().join("fits.rxs");
        crate::project::save_with_storage(&path, &project, crate::project::DataStorage::Embedded)
            .unwrap();
        std::fs::remove_dir_all(root).unwrap();
        std::fs::remove_file(&inputs[0].path).unwrap();
        let restored =
            crate::project::load_with_cache_root(&path, || Ok(tmp.path().join("relocated")))
                .unwrap();
        assert!(restored.raw_files.is_empty());
        assert!(restored.spectrum_file.is_none());
        assert_eq!(
            read(&restored.peak_fits.runs[0].rows[0])
                .unwrap()
                .result
                .model,
            expected
        );
    }
    #[test]
    fn peak_fit_cancellation_and_model_versions_keep_history() {
        let tmp = tempfile::tempdir().unwrap();
        let inputs = vec![input(tmp.path(), "a.dat")];
        let mut run = PeakRun::new("synthetic".into(), model(), &inputs);
        run.freeze(&inputs, || false);
        run.rows[0] = calculate(
            &run.model,
            &run.rows[0],
            &inputs[0],
            &tmp.path().join("artifacts"),
            || true,
        );
        run.finish(true);
        assert!(run.cancelled);
        assert!(!run.complete);
        assert_eq!(run.rows[0].status, FrameStatus::Cancelled);
        let record = read(&run.rows[0]).unwrap();
        assert_eq!(record.result.termination, PeakTermination::Cancelled);
        assert!(record.result.covariance.is_none());
        let mut archive = PeakArchive::default();
        archive.save_model("test".into(), model());
        let mut edited = model();
        edited.range = [8901., 8909.];
        archive.save_model("test".into(), edited);
        assert_eq!(archive.models[0].revision, 1);
        assert_eq!(archive.models[1].revision, 2);
        assert_eq!(archive.models[0].id, archive.models[1].id);
        assert_eq!(archive.models[0].model.range, [8900., 8910.]);
    }

    #[test]
    fn peak_fit_exports_keep_coordinates_masks_units_and_failed_frames() {
        use crate::series_measurements::{CoordinateDefinition, SeriesFrame};
        let tmp = tempfile::tempdir().unwrap();
        let inputs = vec![input(tmp.path(), "a.dat"), input(tmp.path(), "b.dat")];
        let mut run = PeakRun::new("test".into(), model().exclude(8903.0..=8904.0), &inputs);
        run.series = Some(SeriesDefinition {
            id: GroupId::new_result(),
            revision: 2,
            name: "Temperature".into(),
            ordering: "recorded".into(),
            frames: inputs
                .iter()
                .enumerate()
                .map(|(i, input)| SeriesFrame {
                    id: input.group.clone(),
                    group: input.group.clone(),
                    label: input.label.clone(),
                    sequence: i + 1,
                    coordinate: Some(300. + i as f64 * 50.),
                    acquired_at: Some(format!("2026-09-17T00:00:0{i}Z")),
                })
                .collect(),
            coordinate: CoordinateDefinition {
                label: "Temperature".into(),
                unit: "K".into(),
                source: "synthetic".into(),
                timestamp_meaning: "scan start".into(),
            },
        });
        run.freeze(&inputs, || false);
        run.rows[0] = calculate(
            &run.model,
            &run.rows[0],
            &inputs[0],
            &tmp.path().join("artifacts"),
            || false,
        );
        run.finish(false);
        assert_eq!(
            run.plot_coordinates(),
            (vec![300., 350.], "Temperature (K)".into())
        );
        let record = read(&run.rows[0]).unwrap();
        assert!(
            !record
                .result
                .energy
                .iter()
                .any(|x| (8903.0..=8904.0).contains(x))
        );
        let mut bytes = Vec::new();
        write_curves(&record, &mut bytes).unwrap();
        let mut csv = csv::Reader::from_reader(&bytes[..]);
        let header = csv.headers().unwrap().clone();
        let column = |name: &str| header.iter().position(|s| s == name).unwrap();
        let rows: Vec<_> = csv.records().map(Result::unwrap).collect();
        assert_eq!(rows.len(), 180);
        assert_eq!(&rows[0][column("excluded_ranges")], "[[8903.0,8904.0]]");
        bytes.clear();
        write_trend(&run, &mut bytes).unwrap();
        let mut csv = csv::Reader::from_reader(&bytes[..]);
        let header = csv.headers().unwrap().clone();
        let column = |name: &str| header.iter().position(|s| s == name).unwrap();
        let rows: Vec<_> = csv.records().map(Result::unwrap).collect();
        assert_eq!(rows.len(), 2);
        assert_eq!(&rows[1][column("status")], "Failed");
        assert_eq!(&rows[1][column("p_center_absolute_eV")], "");
        assert_eq!(&rows[1][column("coordinate")], "350");
        assert_eq!(&rows[0][column("coordinate_unit")], "K");
        assert_eq!(&rows[0][column("p_whole_axis_area_unit")], "μ units × eV");
        assert_eq!(&rows[0][column("resolved_origin_eV")], "0");
        assert_eq!(&rows[0][column("absolute_start_eV")], "8900");
        assert_eq!(&rows[0][column("timestamp_meaning")], "scan start");
        let center: f64 = rows[0][column("p_center_absolute_eV")].parse().unwrap();
        assert!((center - 8905.).abs() < 1e-6);
        run.series.as_mut().unwrap().frames[1].coordinate = None;
        assert_eq!(
            run.plot_coordinates(),
            (vec![1., 2.], "Frame sequence".into())
        );
        let mut writer = LimitedWriter {
            inner: Vec::new(),
            remaining: 3,
        };
        writer.write_all(b"abc").unwrap();
        assert!(writer.write_all(b"d").is_err());
        assert_eq!(writer.inner, b"abc");
    }
}
