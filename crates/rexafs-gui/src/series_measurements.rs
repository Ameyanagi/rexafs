//! Retained full-frame measurements, independent of the sampled overview/cache.
use crate::{
    group_identity::GroupId,
    params::{self, DerivedSpectrum, PipelineParams, RequiredStage},
};
use rexafs::prelude::{Measurement, MeasurementResult, MeasurementSpace, XASSpectrum};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{path::PathBuf, sync::Arc};

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SeriesArchive {
    pub series: Vec<SeriesDefinition>,
    pub runs: Vec<SeriesRun>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SeriesDefinition {
    pub id: GroupId,
    pub revision: u64,
    pub name: String,
    pub ordering: String,
    pub frames: Vec<SeriesFrame>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SeriesFrame {
    pub id: GroupId,
    pub group: GroupId,
    pub label: String,
    pub sequence: usize,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct MetricDefinition {
    pub id: GroupId,
    pub revision: u64,
    pub name: String,
    pub measurement: Measurement,
    /// Absolute edge energy determined by the recorded preparation; not a shift.
    pub edge_energy: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FrameStatus {
    Pending,
    Succeeded,
    Failed,
    Unavailable,
    Cancelled,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct MetricRow {
    pub frame: SeriesFrame,
    pub source_digest: Option<String>,
    pub input_revision: Option<String>,
    pub settings: PipelineParams,
    pub status: FrameStatus,
    pub result: Option<MeasurementResult>,
    pub reason: Option<String>,
    /// Exact resolved preparation settings, excluding large processed arrays.
    pub preparation: serde_json::Value,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SeriesRun {
    pub id: GroupId,
    pub series_id: GroupId,
    pub series_revision: u64,
    pub definition: MetricDefinition,
    pub created: String,
    pub software: String,
    pub rows: Vec<MetricRow>,
    pub complete: bool,
    pub cancelled: bool,
}

/// Worker input: file sources remain lazy. Materialized groups were already
/// resident in the project; no processed spectrum is stored in this record.
#[derive(Clone)]
pub struct FrameInput {
    pub group: GroupId,
    pub label: String,
    pub path: PathBuf,
    pub derived: Option<Arc<DerivedSpectrum>>,
    pub settings: PipelineParams,
}

fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

impl FrameInput {
    fn source_bytes(&self) -> Result<Vec<u8>, String> {
        if let Some(group) = &self.derived
            && group.source.is_none()
        {
            // Names and catalog positions are presentation, not science identity.
            return serde_json::to_vec(&(
                &group.energy,
                &group.mu,
                group.quantity,
                group.quantity_unconfirmed,
                &group.operation,
                &group.declared_edge,
            ))
            .map_err(|e| e.to_string());
        }
        use std::io::Read;
        let path = self
            .derived
            .as_ref()
            .and_then(|g| g.source.as_ref())
            .unwrap_or(&self.path);
        if path.as_os_str().is_empty() {
            return Err("Source group unavailable".into());
        }
        let file = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let mut bytes = Vec::new();
        file.take(256 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if bytes.len() > 256 * 1024 * 1024 {
            return Err("Source exceeds the 256 MiB measurement limit".into());
        }
        Ok(bytes)
    }

    pub fn revision(&self) -> Result<(String, String), String> {
        let bytes = self.source_bytes()?;
        Ok(self.revision_of(&bytes))
    }

    fn revision_of(&self, bytes: &[u8]) -> (String, String) {
        let source = digest(bytes);
        let parameters = serde_json::to_vec(&self.settings).expect("finite pipeline settings");
        let mut hash = Sha256::new();
        hash.update(source.as_bytes());
        hash.update(parameters);
        (
            source,
            hash.finalize().iter().map(|b| format!("{b:02x}")).collect(),
        )
    }

    pub fn prepare(
        &self,
        definition: &MetricDefinition,
        expected: Option<&str>,
    ) -> Result<(XASSpectrum, String, String), String> {
        let bytes = self.source_bytes()?;
        let (source, revision) = self.revision_of(&bytes);
        if expected.is_some_and(|old| old != revision) {
            return Err("Inputs changed; calculate a new run".into());
        }
        let stage = required_stage(definition);
        let sp = if let Some(group) = &self.derived
            && group.source.is_none()
        {
            group.prepare(&self.settings, stage)?
        } else {
            let path = self
                .derived
                .as_ref()
                .and_then(|g| g.source.as_ref())
                .unwrap_or(&self.path);
            let (x, y) = params::load_raw_snapshot(&bytes, path, &self.settings)?;
            params::prepare_arrays(x, y, &self.settings, stage)?
        };
        Ok((sp, source, revision))
    }
}

pub fn required_stage(definition: &MetricDefinition) -> RequiredStage {
    if definition.edge_energy {
        return RequiredStage::Normalized;
    }
    match definition.measurement.space {
        MeasurementSpace::Mu => RequiredStage::Raw,
        MeasurementSpace::Norm | MeasurementSpace::Flat => RequiredStage::Normalized,
        MeasurementSpace::Chi { .. } => RequiredStage::Background,
        MeasurementSpace::Fourier => RequiredStage::Fourier,
    }
}

impl SeriesRun {
    pub fn new(
        series: &SeriesDefinition,
        definition: MetricDefinition,
        inputs: &[FrameInput],
    ) -> Self {
        Self {
            id: GroupId::new_result(),
            series_id: series.id.clone(),
            series_revision: series.revision,
            definition,
            created: chrono::Utc::now().to_rfc3339(),
            software: env!("CARGO_PKG_VERSION").into(),
            rows: series
                .frames
                .iter()
                .enumerate()
                .map(|(index, frame)| MetricRow {
                    frame: frame.clone(),
                    source_digest: None,
                    input_revision: None,
                    settings: inputs
                        .get(index)
                        .map(|input| input.settings.clone())
                        .unwrap_or_default(),
                    status: if inputs
                        .get(index)
                        .is_some_and(|input| input.group == frame.group)
                    {
                        FrameStatus::Pending
                    } else {
                        FrameStatus::Unavailable
                    },
                    result: None,
                    reason: (!inputs
                        .get(index)
                        .is_some_and(|input| input.group == frame.group))
                    .then(|| "Source group unavailable".into()),
                    preparation: serde_json::Value::Null,
                })
                .collect(),
            complete: false,
            cancelled: false,
        }
    }

    /// Resolve revisions before calculation. The coordinator retains these
    /// small records; source spectra are read one at a time, then released.
    pub fn freeze(&mut self, inputs: &[FrameInput], cancelled: impl Fn() -> bool) {
        for (row, input) in self.rows.iter_mut().zip(inputs) {
            if cancelled() {
                break;
            }
            if row.input_revision.is_some() {
                continue;
            }
            if row.frame.group != input.group {
                continue;
            }
            match input.revision() {
                Ok((source, revision)) => {
                    row.source_digest = Some(source);
                    row.input_revision = Some(revision);
                }
                Err(error) => {
                    row.status = FrameStatus::Unavailable;
                    row.reason = Some(error);
                }
            }
        }
    }

    pub fn finish(&mut self, cancelled: bool) {
        self.cancelled = cancelled;
        if cancelled {
            for row in &mut self.rows {
                if row.status == FrameStatus::Pending {
                    row.status = FrameStatus::Cancelled;
                    row.reason = Some("Cancelled before calculation".into());
                }
            }
        }
        self.complete = !self
            .rows
            .iter()
            .any(|r| matches!(r.status, FrameStatus::Pending | FrameStatus::Cancelled));
    }

    /// CSV includes every requested frame, including gaps. JSON export supplies
    /// the complete definition, settings and resolved preparation alongside it.
    pub fn csv(&self) -> String {
        fn cell(value: impl ToString) -> String {
            format!("\"{}\"", value.to_string().replace('"', "\"\""))
        }
        let mut csv="frame,frame_id,group_id,label,value,unit,status,reason,input_revision,source_digest,run_id,definition_id,definition_revision,range_start,range_end,e0_ev,uncertainty\n".to_string();
        for row in &self.rows {
            let result = row.result.as_ref();
            let cells = vec![
                row.frame.sequence.to_string(),
                serde_json::to_string(&row.frame.id).unwrap(),
                serde_json::to_string(&row.frame.group).unwrap(),
                row.frame.label.clone(),
                result.map(|r| r.value.to_string()).unwrap_or_default(),
                result.map(|r| r.unit.clone()).unwrap_or_default(),
                format!("{:?}", row.status),
                row.reason.clone().unwrap_or_default(),
                row.input_revision.clone().unwrap_or_default(),
                row.source_digest.clone().unwrap_or_default(),
                serde_json::to_string(&self.id).unwrap(),
                serde_json::to_string(&self.definition.id).unwrap(),
                self.definition.revision.to_string(),
                result.map(|r| r.range[0].to_string()).unwrap_or_default(),
                result.map(|r| r.range[1].to_string()).unwrap_or_default(),
                result
                    .and_then(|r| r.e0_ev)
                    .map(|e| e.to_string())
                    .unwrap_or_default(),
                "Unavailable: no input error model".into(),
            ];
            csv.push_str(&cells.into_iter().map(cell).collect::<Vec<_>>().join(","));
            csv.push('\n');
        }
        csv
    }
}

/// Evaluate a frozen row without mutating its source, project or display cache.
pub fn calculate_row(
    input: &FrameInput,
    row: &MetricRow,
    definition: &MetricDefinition,
) -> MetricRow {
    let mut output = row.clone();
    if input.group != row.frame.group {
        output.status = FrameStatus::Unavailable;
        output.reason = Some("The source group does not match this retained frame".into());
        output.result = None;
        return output;
    }
    if row.input_revision.is_none() {
        output.status = FrameStatus::Unavailable;
        output.reason = Some("Input was not snapshotted; start a new run".into());
        return output;
    }
    let result = (|| {
        let (sp, _, _) = input.prepare(definition, row.input_revision.as_deref())?;
        let result = if definition.edge_energy {
            let e0 = sp.e0().ok_or("Edge energy unavailable")?;
            MeasurementResult {
                measurement: definition.measurement.clone(),
                value: e0,
                position: Some(e0),
                range: [e0, e0],
                standard_error: None,
                unit: "eV".into(),
                e0_ev: Some(e0),
            }
        } else {
            sp.measure(&definition.measurement)
                .map_err(|e| e.to_string())?
        };
        output.preparation = resolved_preparation(&sp);
        Ok::<_, String>(result)
    })();
    match result {
        Ok(value) => {
            output.status = FrameStatus::Succeeded;
            output.result = Some(value);
            output.reason = None;
        }
        Err(error) => {
            output.status = FrameStatus::Failed;
            output.result = None;
            output.reason = Some(error);
        }
    }
    output
}

fn resolved_preparation(sp: &XASSpectrum) -> serde_json::Value {
    use rexafs::prelude::NormalizationMethod;
    let normalization = match &sp.normalization {
        Some(NormalizationMethod::PrePostEdge(n)) => {
            serde_json::json!({"method":"PrePostEdge","e0_ev":n.e0,"edge_step":n.edge_step,"pre_start_ev":n.pre_edge_start,"pre_end_ev":n.pre_edge_end,"post_start_ev":n.norm_start,"post_end_ev":n.norm_end,"polynomial_order":n.norm_polyorder,"victoreen":n.n_victoreen})
        }
        _ => serde_json::Value::Null,
    };
    let mut fft = sp.xftf.clone();
    if let Some(f) = &mut fft {
        f.chir = None;
        f.chir_mag = None;
        f.r = None;
        f.kwin = None;
    }
    serde_json::json!({"normalization":normalization,"fourier":fft,"e0_ev":sp.e0()})
}

#[cfg(test)]
mod tests;
