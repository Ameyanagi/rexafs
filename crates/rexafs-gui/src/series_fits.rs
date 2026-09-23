//! Retained, independent EXAFS fits for every frame of a frozen named series.
//! Curves live in checksum-verified artifacts; small covariance summaries allow
//! expression trends without fitting again. See doc/series-fitting.md.
use crate::{
    fit_details::{Estimate, PathFitDetails},
    group_identity::GroupId,
    live::ExafsRecipe,
    params::PipelineParams,
    series_measurements::{
        FrameInput, FrameStatus, SeriesDefinition, SeriesFrame, resolved_preparation,
    },
};
use rexafs::prelude::{FeffFitResult, FitVariables};
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

#[derive(Clone, Serialize, Deserialize)]
pub struct FitTrend {
    pub name: String,
    pub expression: String,
    /// Display unit, supplied by the analyst; expressions do not infer units.
    pub unit: String,
    /// Enabled-path index supplying `reff` (Å) and `degen` (dimensionless).
    pub path: Option<usize>,
}
impl FitTrend {
    pub fn label(&self) -> String {
        if self.unit.is_empty() {
            self.name.clone()
        } else {
            format!("{} ({})", self.name, self.unit)
        }
    }
}

/// Plot choices include all model variables and each path's half-path length.
/// Arbitrary variable names do not imply physical units.
pub fn default_trends(model: &ExafsRecipe) -> Vec<FitTrend> {
    model
        .paths
        .iter()
        .enumerate()
        .map(|(i, (p, _))| FitTrend {
            name: format!("{} · R", p.label),
            expression: format!(
                "reff + ({})",
                if p.deltar.trim().is_empty() {
                    "0"
                } else {
                    &p.deltar
                }
            ),
            unit: "Å".into(),
            path: Some(i),
        })
        .chain(model.variables.iter().map(|v| FitTrend {
            name: v.name.clone(),
            expression: v.name.clone(),
            unit: String::new(),
            path: None,
        }))
        .collect()
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SeriesFitRun {
    pub id: GroupId,
    pub created: String,
    pub software: String,
    pub series: SeriesDefinition,
    pub model: ExafsRecipe,
    pub rows: Vec<SeriesFitRow>,
    pub trends: Vec<FitTrend>,
    pub complete: bool,
    pub cancelled: bool,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct SeriesFitRow {
    pub frame: SeriesFrame,
    pub settings: Arc<PipelineParams>,
    pub source_digest: Option<String>,
    pub input_revision: Option<String>,
    pub status: FrameStatus,
    pub reason: Option<String>,
    pub summary: Option<FitSummary>,
    pub artifact: Option<PathBuf>,
    pub artifact_digest: Option<String>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct FitSummary {
    pub variables: FitVariables,
    pub varying_names: Vec<String>,
    pub covariance: Option<Vec<Vec<f64>>>,
    pub paths: Vec<PathFitDetails>,
    pub r_factor: f64,
    pub converged: bool,
    pub notices: Vec<String>,
}
impl FitSummary {
    /// First-order J C Jᵀ propagation using all parameter correlations. FEFF
    /// geometry is held exact; absent covariance never becomes a zero error bar.
    pub fn estimate(&self, trend: &FitTrend) -> Option<Estimate> {
        let path = trend.path.and_then(|i| self.paths.get(i));
        let result = FeffFitResult {
            variables: self.variables.clone(),
            varying_names: self.varying_names.clone(),
            covariance: self.covariance.clone(),
            ..Default::default()
        };
        crate::fit_details::estimate(
            &trend.expression,
            path.and_then(|p| p.reff),
            path.and_then(|p| p.degeneracy),
            &result,
        )
    }
}

#[derive(Serialize, Deserialize)]
pub struct SeriesFitRecord {
    pub schema: u32,
    pub group: GroupId,
    pub input_revision: String,
    pub source_digest: String,
    pub settings: Arc<PipelineParams>,
    pub preparation: serde_json::Value,
    pub result: FeffFitResult,
}

pub fn root() -> Result<PathBuf, String> {
    let parent = crate::settings::env_var_os("SETTINGS")
        .and_then(|p| PathBuf::from(p).parent().map(Path::to_path_buf))
        .or_else(crate::settings::app_dir)
        .ok_or("Application storage unavailable")?;
    Ok(parent.join("series-fits"))
}
pub fn read(row: &SeriesFitRow) -> Result<SeriesFitRecord, String> {
    let record: SeriesFitRecord = crate::analysis_store::read(
        row.artifact.as_ref().ok_or("No fit for this frame")?,
        row.artifact_digest
            .as_deref()
            .ok_or("Missing fit checksum")?,
    )?;
    if record.schema != 1
        || record.group != row.frame.group
        || Some(&record.input_revision) != row.input_revision.as_ref()
        || Some(&record.source_digest) != row.source_digest.as_ref()
    {
        return Err("Retained series fit identity mismatch".into());
    }
    Ok(record)
}
impl SeriesFitRun {
    pub fn new(series: SeriesDefinition, model: ExafsRecipe, inputs: &[FrameInput]) -> Self {
        let trends = default_trends(&model);
        let rows = series
            .frames
            .iter()
            .enumerate()
            .map(|(i, frame)| SeriesFitRow {
                frame: frame.clone(),
                settings: Arc::new(
                    inputs
                        .get(i)
                        .map(|s| s.settings.clone())
                        .unwrap_or_default(),
                ),
                source_digest: None,
                input_revision: None,
                status: FrameStatus::Pending,
                reason: None,
                summary: None,
                artifact: None,
                artifact_digest: None,
            })
            .collect();
        Self {
            id: GroupId::new_result(),
            created: chrono::Utc::now().to_rfc3339(),
            software: env!("CARGO_PKG_VERSION").into(),
            series,
            model,
            rows,
            trends,
            complete: false,
            cancelled: false,
        }
    }
    /// Snapshot all identities before the first fit. Changed sources become
    /// failed frames rather than silently mixing input revisions within a run.
    pub fn freeze(
        &mut self,
        inputs: &[FrameInput],
        cancelled: impl Fn() -> bool,
    ) -> Result<(), String> {
        if inputs.len() != self.rows.len() {
            return Err("Series input count changed".into());
        }
        self.model.freeze_paths()?;
        for (row, input) in self.rows.iter_mut().zip(inputs) {
            if cancelled() {
                break;
            }
            if input.group != row.frame.group {
                return Err("Series input order changed".into());
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
        Ok(())
    }
    pub fn finish(&mut self, cancelled: bool) {
        self.cancelled = cancelled;
        self.complete = !cancelled;
        for row in &mut self.rows {
            if row.status == FrameStatus::Pending {
                row.status = FrameStatus::Cancelled;
                row.reason = Some("Stopped before this frame was fitted".into());
            }
        }
    }
}

pub fn calculate(
    input: &FrameInput,
    row: &SeriesFitRow,
    model: &ExafsRecipe,
    directory: &Path,
) -> SeriesFitRow {
    let mut row = row.clone();
    if row.status != FrameStatus::Pending {
        return row;
    }
    let result = (|| {
        if input.group != row.frame.group {
            return Err("Series input identity changed".into());
        }
        let (spectrum, source, revision) = input.prepare_for_fit(
            row.input_revision
                .as_deref()
                .ok_or("Input revision unavailable")?,
        )?;
        let paths = model.materialize_paths(directory)?;
        let result =
            crate::fitting::run_spectrum_fit(&spectrum, &paths, &model.variables, &model.ranges)?;
        let summary = FitSummary {
            variables: result.variables.clone(),
            varying_names: result.varying_names.clone(),
            covariance: result.covariance.clone(),
            paths: crate::fit_details::snapshot(&paths, &result),
            r_factor: result.r_factor,
            converged: result.solver_report.as_ref().is_some_and(|r| r.converged),
            notices: result.warnings.iter().map(|w| format!("{w:?}")).collect(),
        };
        let record = SeriesFitRecord {
            schema: 1,
            group: input.group.clone(),
            input_revision: revision,
            source_digest: source,
            settings: row.settings.clone(),
            preparation: resolved_preparation(&spectrum),
            result,
        };
        let (artifact, digest) = crate::analysis_store::retain(directory, &record)?;
        Ok::<_, String>((summary, artifact, digest))
    })();
    match result {
        Ok((summary, artifact, digest)) => {
            row.summary = Some(summary);
            row.artifact = Some(artifact);
            row.artifact_digest = Some(digest);
            row.status = FrameStatus::Succeeded;
        }
        Err(error) => {
            row.reason = Some(error);
            row.status = FrameStatus::Failed;
        }
    }
    row
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        fitting::{FitPathSpec, FitRanges, FitVarSpec},
        project::{self, DataStorage, ProjectFile},
    };

    #[test]
    fn cu_series_retains_curves_geometry_errors_and_portable_history() {
        let tmp = tempfile::tempdir().unwrap();
        let core = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../rexafs/tests");
        let source = tmp.path().join("cu.xdi");
        std::fs::copy(
            core.join("fixtures/analysis/cu-mixtures/standards/cufoil_abs.xdi"),
            &source,
        )
        .unwrap();
        let path = tmp.path().join("feff.dat");
        std::fs::copy(core.join("testfiles/feffcu01.dat"), &path).unwrap();
        let mut spec = FitPathSpec::standard(path.clone(), 1);
        spec.e0 = "e0".into();
        spec.sigma2 = "ss".into();
        spec.deltar = "dr".into();
        spec.label = "Cu–Cu".into();
        let model = ExafsRecipe {
            name: "Cu first shell".into(),
            paths: vec![(spec, vec![])],
            ranges: FitRanges::default(),
            variables: [
                ("amp", 0.9, Some(0.5), Some(1.5)),
                ("e0", 0., None, None),
                ("dr", 0., Some(-0.3), Some(0.3)),
                ("ss", 0.003, Some(0.), None),
            ]
            .into_iter()
            .map(|(name, value, min, max)| FitVarSpec {
                name: name.into(),
                value,
                min,
                max,
                vary: true,
                expr: None,
            })
            .collect(),
        };
        let inputs: Vec<_> = (0..3)
            .map(|i| FrameInput {
                group: GroupId::new_result(),
                label: format!("Cu {i}"),
                path: if i == 1 {
                    tmp.path().join("missing.xdi")
                } else {
                    source.clone()
                },
                derived: None,
                settings: Default::default(),
                recipe: None,
            })
            .collect();
        let series = SeriesDefinition {
            id: GroupId::new_result(),
            revision: 1,
            name: "Cu replay".into(),
            ordering: "explicit".into(),
            coordinate: Default::default(),
            frames: inputs
                .iter()
                .enumerate()
                .map(|(i, s)| SeriesFrame {
                    id: GroupId::new_result(),
                    group: s.group.clone(),
                    label: s.label.clone(),
                    sequence: i + 1,
                    coordinate: Some(i as f64 * 10.),
                    acquired_at: None,
                })
                .collect(),
        };
        let mut run = SeriesFitRun::new(series, model, &inputs);
        run.freeze(&inputs, || false).unwrap();
        std::fs::remove_file(&path).unwrap();
        let artifacts = tmp.path().join("fits");
        for (row, input) in run.rows.iter_mut().zip(&inputs) {
            *row = calculate(input, row, &run.model, &artifacts);
        }
        run.finish(false);
        assert_eq!(run.rows[1].status, FrameStatus::Unavailable);
        let first = run.rows[0].summary.as_ref().expect("Cu should fit");
        assert!(first.converged);
        assert!(first.r_factor < 0.02);
        let distance = first.estimate(&run.trends[0]).unwrap();
        let dr = first
            .estimate(&FitTrend {
                name: "dr".into(),
                expression: "dr".into(),
                unit: "Å".into(),
                path: None,
            })
            .unwrap();
        assert!((distance.value - first.paths[0].reff.unwrap() - dr.value).abs() < 1e-10);
        assert!((distance.stderr.unwrap() - dr.stderr.unwrap()).abs() < 1e-8);
        assert!(distance.stderr.unwrap() > 0.);
        assert_eq!(
            first.r_factor,
            run.rows[2].summary.as_ref().unwrap().r_factor
        );
        let record = read(&run.rows[0]).unwrap();
        assert!(!record.result.k.is_empty());
        assert!(!record.preparation.is_null());
        let mut changed = inputs[2].clone();
        changed.settings.e0 = Some(8990.);
        let mut pending = run.rows[2].clone();
        pending.status = FrameStatus::Pending;
        let rejected = calculate(&changed, &pending, &run.model, &artifacts);
        assert!(rejected.reason.unwrap().contains("Inputs changed"));
        let mut forged = run.rows[0].clone();
        forged.frame.group = GroupId::new_result();
        assert!(read(&forged).err().unwrap().contains("identity"));
        let mut project = ProjectFile::default();
        project.series_measurements.series.push(run.series.clone());
        project.series_measurements.fit_runs.push(Arc::new(run));
        let portable = tmp.path().join("series.rxs");
        project::save_with_storage(&portable, &project, DataStorage::Embedded).unwrap();
        std::fs::remove_dir_all(&artifacts).unwrap();
        std::fs::remove_file(&source).unwrap();
        let restored =
            project::load_with_cache_root(&portable, || Ok(tmp.path().join("restore"))).unwrap();
        let run = &restored.series_measurements.fit_runs[0];
        assert_eq!(run.rows.len(), 3);
        assert_eq!(run.rows[2].frame.sequence, 3);
        assert!(read(&run.rows[0]).is_ok());
        assert!(read(&run.rows[2]).is_ok());
        assert!(
            run.rows[0]
                .summary
                .as_ref()
                .unwrap()
                .estimate(&run.trends[0])
                .unwrap()
                .stderr
                .is_some()
        );
    }

    #[test]
    fn expressions_include_covariance_constraints_and_missing_errors() {
        use rexafs::prelude::FitVariable;
        let mut variables = FitVariables::default();
        for (name, value) in [("a", 1.), ("b", 2.)] {
            variables.insert(
                name,
                FitVariable {
                    value,
                    vary: true,
                    ..Default::default()
                },
            );
        }
        variables.insert(
            "sum",
            FitVariable {
                expr: Some("a+2*b".into()),
                ..Default::default()
            },
        );
        let mut summary = FitSummary {
            variables,
            varying_names: vec!["a".into(), "b".into()],
            covariance: Some(vec![vec![0.04, -0.01], vec![-0.01, 0.09]]),
            paths: vec![],
            r_factor: 0.,
            converged: true,
            notices: vec![],
        };
        let trend = FitTrend {
            name: "Derived".into(),
            expression: "sum".into(),
            unit: String::new(),
            path: None,
        };
        let estimate = summary.estimate(&trend).unwrap();
        assert_eq!(estimate.value, 5.);
        assert!((estimate.stderr.unwrap() - 0.6).abs() < 1e-9);
        summary.covariance = None;
        assert!(summary.estimate(&trend).unwrap().stderr.is_none());
        assert!(
            summary
                .estimate(&FitTrend {
                    expression: "reff+a".into(),
                    ..trend
                })
                .is_none()
        );
    }
}
