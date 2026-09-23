//! Rebuild derived averages from committed source revisions. Rebuilding uses
//! one spectrum at a time; original records remain immutable recovery evidence.
use super::*;
use crate::params::{OperationInput, StreamingAverage};

#[derive(Clone)]
pub struct LiveAverage {
    pub channel: String,
    pub batch: usize,
    pub count: usize,
    pub group: Result<DerivedSpectrum, String>,
}

/// Calculate equal-weight means of raw absorption on the first contributor's
/// energy grid, restricted to common coverage. Each source/scan contributes
/// only its latest committed revision. Fixed batches use first-accepted scan
/// order; file rewrites retain their batch and replace their earlier weight.
/// Cached means are immutable, while each displayed output has a stable ID.
/// No alignment, noise estimate or automatic quality threshold is inferred.
pub fn build_averages(store: &LiveStore, cancel: &AtomicBool) -> Result<Vec<LiveAverage>, String> {
    if store.config.merge == LiveMerge::Individual {
        return Ok(Vec::new());
    }
    let records = store.records()?;
    let mut latest = BTreeMap::new();
    let mut ordinals: BTreeMap<(PathBuf, String), usize> = BTreeMap::new();
    for (i, record) in records.iter().enumerate() {
        if cancel.load(Ordering::Relaxed) {
            return Err("Average cancelled".into());
        }
        latest.insert(record.source.clone(), i);
        for frame in &record.frames {
            let scan = frame
                .group
                .operation
                .as_ref()
                .and_then(|o| o.parameters["scan_id"].as_str())
                .unwrap_or("scan");
            let next = ordinals.len();
            ordinals
                .entry((record.source.clone(), scan.into()))
                .or_insert(next);
        }
    }
    let mut inputs: BTreeMap<(String, usize), Vec<(usize, &LiveFrame)>> = BTreeMap::new();
    for i in latest.values() {
        let record = &records[*i];
        for frame in &record.frames {
            let scan = frame
                .group
                .operation
                .as_ref()
                .and_then(|o| o.parameters["scan_id"].as_str())
                .unwrap_or("scan");
            let ordinal = ordinals[&(record.source.clone(), scan.into())];
            let batch = match store.config.merge {
                LiveMerge::Batches { scans } => ordinal / scans,
                _ => 0,
            };
            let channel = if frame.channel.is_empty() {
                "Signal"
            } else {
                &frame.channel
            };
            inputs
                .entry((channel.into(), batch))
                .or_default()
                .push((ordinal, frame));
        }
    }
    let mut outputs = Vec::new();
    for ((channel, batch), mut frames) in inputs {
        if cancel.load(Ordering::Relaxed) {
            return Err("Average cancelled".into());
        }
        frames.sort_by_key(|(ordinal, _)| *ordinal);
        let count = frames.len();
        let group = average(store, &channel, batch, &frames, cancel);
        outputs.push(LiveAverage {
            channel,
            batch,
            count,
            group,
        });
    }
    Ok(outputs)
}

fn average(
    store: &LiveStore,
    channel: &str,
    batch: usize,
    frames: &[(usize, &LiveFrame)],
    cancel: &AtomicBool,
) -> Result<DerivedSpectrum, String> {
    let mut accumulator = None;
    let mut single = None;
    let mut inputs = Vec::new();
    let mut revisions = Vec::new();
    let mut mode = None;
    for (_, frame) in frames {
        if cancel.load(Ordering::Relaxed) {
            return Err("Average cancelled".into());
        }
        let group = &frame.group;
        let mut raw = group.params.clone().unwrap_or_default();
        // Apply frozen offsets once when processing the averaged raw signal.
        raw.energy_offset_ev = 0.;
        let (energy, mu) = group.raw(&raw)?;
        if energy.len() < 2
            || energy.len() != mu.len()
            || energy.iter().chain(&mu).any(|v| !v.is_finite())
            || energy.windows(2).any(|e| e[0] >= e[1])
        {
            return Err("Average needs finite signals on strictly increasing energy grids".into());
        }
        if mode.is_some_and(|m| m != group.acquisition_mode()) {
            return Err("Detector interpretation differs within one average".into());
        }
        mode = Some(group.acquisition_mode());
        if frames.len() == 1 {
            single = Some((energy, mu));
        } else if let Some(acc) = &mut accumulator {
            StreamingAverage::add(acc, &energy, &mu);
        } else {
            accumulator = Some(StreamingAverage::new(energy, mu));
        }
        let path = group
            .source
            .clone()
            .ok_or("A retained scan is missing its source")?;
        revisions.push(path.clone());
        inputs.push(OperationInput {
            group_id: group.group_id.clone(),
            label: group.label.clone(),
            path,
            derived_id: None,
            fingerprint: group.fingerprint(group.params.as_ref().unwrap_or(&store.config.settings)),
            size: None,
        });
    }
    let (energy, mu) = match single {
        Some(arrays) => arrays,
        None => accumulator.ok_or("No scans to average")?.finish()?,
    };
    let key =
        digest(&serde_json::to_vec(&(channel, batch, &revisions)).map_err(|e| e.to_string())?);
    let source = store.spectrum(&format!("average-{key}"), &energy, &mu,
        "Equal-weight raw absorption mean; original source revisions are retained in the acquisition ledger.")?;
    let identity = digest(&serde_json::to_vec(&(channel, batch)).map_err(|e| e.to_string())?);
    let mut settings = store.config.settings.clone();
    settings.import = cache_import();
    let suffix = match store.config.merge {
        LiveMerge::Batches { scans } => {
            format!(" · batch {} · {}/{scans} scans", batch + 1, frames.len())
        }
        _ => format!(
            " · {} {}",
            frames.len(),
            if frames.len() == 1 { "scan" } else { "scans" }
        ),
    };
    Ok(DerivedSpectrum {
        group_id: Some(GroupId::source(
            &store.directory.join(format!("average-output-{identity}")),
            crate::params::DetectionMode::MuColumn,
        )),
        label: format!("{channel} average{suffix}"),
        source: Some(source),
        params: Some(settings),
        absorption_mode: mode.unwrap_or_default(),
        operation: Some(Operation {
            tool: "Live average".into(),
            applied_energy_shift_ev: 0.,
            inputs,
            parameters: serde_json::json!({"session_id":store.config.id,"channel":channel,"batch":batch,
                "count":frames.len(),"merge":store.config.merge,"revision":key,
                "policy":"Equal weight; latest source revision; first accepted grid; common overlap"}),
        }),
        ..Default::default()
    })
}
