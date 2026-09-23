//! Reuse a reviewed single-spectrum EXAFS model on Live outputs. FEFF files are
//! frozen with the recipe; each fit starts from the same recorded variables.
use super::*;
use crate::{
    fitting::{FitHistoryEntry, FitPathSpec, FitRanges, FitVarSpec},
    params::RequiredStage,
};
use rexafs::prelude::FeffFitResult;

#[derive(Clone, Serialize, Deserialize)]
pub struct ExafsRecipe {
    pub name: String,
    pub paths: Vec<(FitPathSpec, Vec<u8>)>,
    pub variables: Vec<FitVarSpec>,
    pub ranges: FitRanges,
}
impl ExafsRecipe {
    pub fn capture(fit: &FitHistoryEntry) -> Result<Self, String> {
        if fit.joint.is_some() {
            return Err("Choose a saved single-spectrum EXAFS fit".into());
        }
        let paths: Vec<_> = fit
            .paths
            .iter()
            .filter(|p| p.enabled)
            .map(|p| (p.clone(), Vec::new()))
            .collect();
        if paths.is_empty() {
            return Err("The saved fit has no enabled scattering paths".into());
        }
        Ok(Self {
            name: format!("{} · fit #{}", fit.group, fit.id),
            paths,
            variables: fit.vars.clone(),
            ranges: fit.ranges.clone(),
        })
    }
    /// Freeze path bytes on the preview worker before the session can start.
    pub fn freeze_paths(&mut self) -> Result<(), String> {
        let mut total = 0usize;
        for (spec, bytes) in &mut self.paths {
            *bytes =
                std::fs::read(&spec.file).map_err(|e| format!("{}: {e}", spec.file.display()))?;
            total = total.saturating_add(bytes.len());
            if total > 64 * 1024 * 1024 {
                return Err("Live EXAFS paths exceed 64 MiB".into());
            }
        }
        Ok(())
    }
    pub fn evaluate(
        &self,
        spectrum: &rexafs::prelude::XASSpectrum,
        directory: &Path,
    ) -> Result<FeffFitResult, String> {
        let paths = self.materialize_paths(directory)?;
        crate::fitting::run_spectrum_fit(spectrum, &paths, &self.variables, &self.ranges)
    }

    /// Materialize the frozen FEFF bytes for fitting and geometry readout.
    pub fn materialize_paths(&self, directory: &Path) -> Result<Vec<FitPathSpec>, String> {
        std::fs::create_dir_all(directory).map_err(|e| e.to_string())?;
        let mut paths = Vec::new();
        for (spec, bytes) in &self.paths {
            let mut spec = spec.clone();
            spec.file = directory.join(format!("exafs-path-{}.dat", digest(bytes)));
            super::store::publish(&spec.file, bytes)?;
            paths.push(spec);
        }
        Ok(paths)
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ExafsRow {
    pub key: String,
    pub group: GroupId,
    pub source: PathBuf,
    pub channel: String,
    pub label: String,
    pub artifact: PathBuf,
    pub artifact_digest: String,
    pub r_factor: Option<f64>,
    pub converged: bool,
    pub notices: Vec<String>,
    pub error: Option<String>,
}
#[derive(Serialize, Deserialize)]
pub struct ExafsRecord {
    pub key: String,
    pub group: GroupId,
    pub label: String,
    pub result: Result<FeffFitResult, String>,
}
pub fn read_exafs(row: &ExafsRow) -> Result<ExafsRecord, String> {
    let record: ExafsRecord = crate::analysis_store::read(&row.artifact, &row.artifact_digest)?;
    if record.key != row.key || record.group != row.group {
        return Err("Retained EXAFS fit identity mismatch".into());
    }
    Ok(record)
}

/// Fit each individual scan, or each updated average when averaging is enabled.
/// Cache keys include the immutable input revision. Reopening does not refit;
/// `compute` is true only during an explicitly running acquisition.
pub fn build_exafs(
    store: &LiveStore,
    averages: &[LiveAverage],
    compute: bool,
    cancel: &AtomicBool,
) -> Result<Vec<ExafsRow>, String> {
    let Some(recipe) = &store.config.exafs else {
        return Ok(Vec::new());
    };
    let groups: Vec<_> = if store.config.merge == LiveMerge::Individual {
        store
            .records()?
            .into_iter()
            .flat_map(|r| r.frames.into_iter().map(|f| f.group))
            .collect()
    } else {
        averages
            .iter()
            .filter_map(|a| a.group.as_ref().ok().cloned())
            .collect()
    };
    let mut rows = Vec::new();
    for group in groups {
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        let source = group
            .source
            .as_ref()
            .ok_or("Live output has no retained arrays")?;
        let id = group
            .group_id
            .clone()
            .ok_or("Live output has no identity")?;
        let key = digest(
            &serde_json::to_vec(&(&store.config.id, &id, source)).map_err(|e| e.to_string())?,
        );
        let index = store.directory.join(format!("exafs-{key}.json"));
        if index.exists() {
            let row: ExafsRow =
                serde_json::from_slice(&std::fs::read(&index).map_err(|e| e.to_string())?)
                    .map_err(|e| e.to_string())?;
            if row.key != key || row.group != id {
                return Err("Retained EXAFS index identity mismatch".into());
            }
            rows.push(row);
            continue;
        }
        if !compute {
            continue;
        }
        let mut settings = store.config.settings.clone();
        if let Some(p) = &group.params {
            settings.import = p.import.clone();
        }
        let result = group
            .prepare(&settings, RequiredStage::Background)
            .and_then(|s| recipe.evaluate(&s, &store.directory));
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        let record = ExafsRecord {
            key: key.clone(),
            group: id.clone(),
            label: group.label.clone(),
            result,
        };
        let (artifact, artifact_digest) = crate::analysis_store::retain(&store.directory, &record)?;
        let row = ExafsRow {
            key,
            group: id,
            source: source.clone(),
            label: group.label.clone(),
            channel: group
                .operation
                .as_ref()
                .and_then(|o| o.parameters["channel"].as_str())
                .unwrap_or("Signal")
                .into(),
            artifact,
            artifact_digest,
            r_factor: record.result.as_ref().ok().map(|r| r.r_factor),
            converged: record
                .result
                .as_ref()
                .is_ok_and(|r| r.solver_report.as_ref().is_some_and(|r| r.converged)),
            notices: record
                .result
                .as_ref()
                .map(crate::fitting::fit_result_notices)
                .unwrap_or_default(),
            error: record.result.as_ref().err().cloned(),
        };
        super::store::publish(
            &index,
            &serde_json::to_vec(&row).map_err(|e| e.to_string())?,
        )?;
        rows.push(row);
    }
    Ok(rows)
}
