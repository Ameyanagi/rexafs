//! Desktop orchestration for native MCR. Only complete, unchanged input
//! snapshots may replace the retained result; calculation runs off the UI thread.

use futures::StreamExt;
use gpui::Context;
use rexafs::prelude::{McrConfig, mcr_als_with_progress};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use super::{AnalysisState, StudioApp, Tool, ToolField};

struct McrRequest {
    inputs: Vec<Result<Arc<rexafs::prelude::XASSpectrum>, crate::app::CompareLoad>>,
    sources: Vec<super::ToolTarget>,
    config: McrConfig,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::{DetectionMode, ImportConfig, PipelineParams};
    use rexafs::prelude::{AnalysisSpace, LcfConfig, PcaConfig, lcf, mcr_als, pca_train};

    #[test]
    fn all_copper_files_survive_desktop_processing_and_mcr_project_roundtrip() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../rexafs/tests/fixtures/analysis/cu-mixtures");
        let params = PipelineParams {
            import: ImportConfig {
                mode: DetectionMode::MuColumn,
                ..Default::default()
            },
            e0: Some(8979.),
            edge_step: Some(1.),
            pre_edge_start: Some(-150.),
            pre_edge_end: Some(-75.),
            norm_start: Some(150.),
            norm_end: Some(650.),
            norm_polyorder: Some(2),
            n_victoreen: Some(0),
            ..Default::default()
        };
        let load = |path: &str| {
            let request = crate::app::CompareLoad {
                ix: 0,
                fingerprint: params.fingerprint(),
                source: Ok(root.join(path)),
                params: params.clone(),
            };
            Arc::new(request.process().2.unwrap().0)
        };
        let spectra: Vec<_> = (1..=100)
            .map(|i| load(&format!("mixtures/mix_{i:03}.xdi")))
            .collect();
        let standards: Vec<_> = ["cufoil_abs", "cu2o_abs", "cuo_abs"]
            .iter()
            .map(|name| load(&format!("standards/{name}.xdi")))
            .collect();
        let truth: serde_json::Value =
            serde_json::from_slice(&std::fs::read(root.join("truth.json")).unwrap()).unwrap();
        let mut flat_rows = std::collections::BTreeMap::new();
        for space in [AnalysisSpace::Norm, AnalysisSpace::Flat] {
            let cfg = LcfConfig {
                space,
                range: Some((-29., 171.)),
                ..Default::default()
            };
            for (i, spectrum) in spectra.iter().enumerate() {
                let fit = lcf(spectrum, &standards, &cfg).unwrap();
                if space == AnalysisSpace::Flat {
                    flat_rows.insert(
                        i,
                        fit.weights
                            .iter()
                            .map(|w| w.weight)
                            .chain(std::iter::once(fit.r_factor))
                            .collect::<Vec<_>>(),
                    );
                }
                for (j, w) in fit.weights.iter().enumerate() {
                    assert!((w.weight - truth[i]["weights"][j].as_f64().unwrap()).abs() < 1e-7);
                }
            }
            let pca = pca_train(
                &spectra,
                &PcaConfig {
                    space,
                    range: cfg.range,
                    ..Default::default()
                },
            )
            .unwrap();
            assert!(pca.variance_explained.iter().skip(3).sum::<f64>() < 1e-20);
        }
        let result = mcr_als(
            &spectra,
            &McrConfig {
                space: AnalysisSpace::Flat,
                range: Some((-29., 171.)),
                max_iterations: 1,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(result.concentrations.nrows(), 100);
        assert_eq!(
            result.termination,
            rexafs::prelude::McrTermination::IterationLimit
        );
        let mut project = crate::project::ProjectFile::default();
        project.mcr_analysis = Some(crate::project::McrAnalysis {
            result: result.clone(),
            inputs: (0..100)
                .map(|i| crate::project::AnalysisInput {
                    corrections: Vec::new(),
                    group_id: Some(crate::group_identity::GroupId::new_result()),
                    label: format!("mix_{:03}", i + 1),
                    fingerprint: params.fingerprint(),
                })
                .collect(),
            comparison: None,
        });
        project.derived =
            super::super::result_groups::mcr_groups(project.mcr_analysis.as_ref().unwrap())
                .unwrap();
        let cfg = LcfConfig {
            space: AnalysisSpace::Flat,
            range: Some((-29., 171.)),
            ..Default::default()
        };
        let fit = lcf(&spectra[0], &standards, &cfg).unwrap();
        let records = project.mcr_analysis.as_ref().unwrap().inputs[..4].to_vec();
        let groups = super::super::result_groups::lcf_groups(&fit, &records, Some(&cfg)).unwrap();
        assert_eq!(groups.len(), 5);
        for k in 0..fit.x.len() {
            assert!(
                (groups[2..].iter().map(|g| g.mu[k]).sum::<f64>() - groups[0].mu[k]).abs() < 1e-12
            );
            assert!((fit.data[k] - groups[0].mu[k] - groups[1].mu[k]).abs() < 1e-12);
        }
        project.derived.extend(groups);
        project.lcf_series_analysis = Some(crate::project::LcfSeriesAnalysis {
            config: cfg.clone(),
            inputs: project
                .mcr_analysis
                .as_ref()
                .unwrap()
                .inputs
                .iter()
                .cloned()
                .enumerate()
                .collect(),
            standards: ["cufoil_abs", "cu2o_abs", "cuo_abs"]
                .into_iter()
                .map(|label| crate::project::AnalysisInput {
                    corrections: Vec::new(),
                    group_id: None,
                    label: label.into(),
                    fingerprint: params.fingerprint(),
                })
                .collect(),
            rows: flat_rows,
            errors: Default::default(),
            complete: true,
            cancelled: false,
        });
        project.lcf_analysis = Some(crate::project::LcfAnalysis {
            result: fit,
            inputs: records,
            config: Some(cfg),
        });
        project.pca_analysis = Some(crate::project::PcaAnalysis {
            model: pca_train(
                &spectra,
                &PcaConfig {
                    space: AnalysisSpace::Flat,
                    range: Some((-29., 171.)),
                    center: true,
                    ..Default::default()
                },
            )
            .unwrap(),
            target: None,
            inputs: project.mcr_analysis.as_ref().unwrap().inputs.clone(),
        });
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("copper-mcr.rxs");
        crate::project::save(&path, &project).unwrap();
        let saved = crate::project::load(&path).unwrap();
        assert_eq!(saved.derived.len(), 8);
        let series = saved.lcf_series_analysis.as_ref().unwrap();
        assert_eq!(series.rows.len(), 100);
        assert_eq!(series.config.space, AnalysisSpace::Flat);
        for (i, row) in &series.rows {
            for (j, w) in row[..3].iter().enumerate() {
                assert!((w - truth[*i]["weights"][j].as_f64().unwrap()).abs() < 1e-7);
            }
        }
        assert_eq!(
            saved
                .lcf_analysis
                .as_ref()
                .unwrap()
                .config
                .as_ref()
                .unwrap()
                .space,
            AnalysisSpace::Flat
        );
        assert_eq!(saved.pca_analysis.as_ref().unwrap().inputs.len(), 100);
        for (group, original) in saved.derived.iter().zip(&project.derived) {
            assert_eq!(group.mu, original.mu);
            assert_eq!(group.quantity, original.quantity);
            assert_eq!(
                serde_json::to_value(&group.operation).unwrap(),
                serde_json::to_value(&original.operation).unwrap()
            );
            assert_eq!(
                group
                    .for_display(&params)
                    .unwrap()
                    .flat()
                    .unwrap()
                    .as_slice(),
                original.mu
            );
        }
        assert_eq!(
            saved.derived[0].operation.as_ref().unwrap().inputs.len(),
            100
        );
        assert_eq!(
            saved.derived[0].operation.as_ref().unwrap().parameters["space"],
            "Flat"
        );
        let restored = saved.mcr_analysis.unwrap();
        assert_eq!(restored.result.data, result.data);
        assert_eq!(restored.result.spectra, result.spectra);
        assert_eq!(restored.result.concentrations, result.concentrations);
        assert_eq!(restored.result.config.components, 3);
    }
}

impl AnalysisState {
    pub(crate) fn mcr_generation_advance(&mut self) {
        self.mcr_generation += 1;
        self.mcr_sources.clear();
        self.collection_generation += 1;
        self.collection_loading = false;
        self.pca_sources.clear();
        self.lcf_sources.clear();
    }
}

impl StudioApp {
    pub(super) fn mcr_result_current(&self) -> bool {
        self.analysis.mcr.as_ref().is_some_and(|saved| {
            !saved.inputs.is_empty()
                && saved.inputs.iter().all(|input| {
                    input
                        .group_id
                        .as_ref()
                        .and_then(|id| self.group_registry.index(id))
                        .is_some_and(|ix| self.effective_fingerprint(ix) == input.fingerprint)
                })
        })
    }

    pub(super) fn start_mcr(&mut self, cx: &mut Context<Self>) -> Result<String, String> {
        if let Some(cancel) = &self.analysis.mcr_cancel {
            cancel.store(true, Ordering::Relaxed);
            self.tools.message = "Cancelling after the current iteration…".into();
            cx.notify();
            return Ok(self.tools.message.to_string());
        }
        let outcome = self.prepare_mcr(cx);
        let McrRequest {
            inputs,
            sources,
            config,
        } = match outcome {
            Ok(inputs) => inputs,
            Err(error) => {
                self.tools.message = error.clone().into();
                cx.notify();
                return Err(error);
            }
        };
        self.analysis.mcr_generation += 1;
        let generation = self.analysis.mcr_generation;
        let cancel = Arc::new(AtomicBool::new(false));
        self.analysis.mcr_cancel = Some(cancel.clone());
        self.tools.message = format!(
            "Preparing {}…",
            crate::text::plural(inputs.len(), "spectrum")
        )
        .into();
        let message = self.tools.message.to_string();
        let names: Vec<_> = sources.iter().map(|s| s.label.clone()).collect();
        let (tx, mut rx) = futures::channel::mpsc::unbounded();
        let task = cx.background_executor().spawn(async move {
            let mut loaded = Vec::new();
            let result = (|| {
                let mut spectra = Vec::new();
                for (i, input) in inputs.into_iter().enumerate() {
                    if cancel.load(Ordering::Relaxed) {
                        return Err("MCR cancelled while preparing spectra".to_owned());
                    }
                    match input {
                        Ok(spectrum) => spectra.push(spectrum),
                        Err(load) => {
                            let (ix, fingerprint, result) = load.process();
                            let (spectrum, _) = result.map_err(|e| format!("{}: {e}", names[i]))?;
                            let spectrum = Arc::new(spectrum);
                            loaded.push((ix, fingerprint, spectrum.clone()));
                            spectra.push(spectrum);
                        }
                    }
                    if i % 10 == 0 {
                        let _ = tx.unbounded_send(format!(
                            "Preparing spectra {}/{}…",
                            i + 1,
                            names.len()
                        ));
                    }
                }
                mcr_als_with_progress(&spectra, &config, |iteration, error| {
                    if iteration == 1 || iteration % 25 == 0 {
                        let _ = tx.unbounded_send(format!(
                            "Iteration {iteration} · residual {error:.2e}"
                        ));
                    }
                    !cancel.load(Ordering::Relaxed)
                })
                .map(|mut result| {
                    result.labels = names;
                    result
                })
                .map_err(|e| e.to_string())
            })();
            (result, loaded)
        });
        cx.spawn(async move |this, cx| {
            while let Some(message) = rx.next().await {
                if this
                    .update(cx, |app, cx| {
                        if app.analysis.mcr_generation == generation {
                            app.tools.message = message.into();
                            cx.notify();
                        }
                    })
                    .is_err()
                {
                    return;
                }
            }
            let (result, loaded) = task.await;
            this.update(cx, |app, cx| {
                if app.analysis.mcr_generation != generation {
                    return;
                }
                app.analysis.mcr_cancel = None;
                if !sources
                    .iter()
                    .all(|s| app.tool_target(s.ix).as_ref() == Some(s))
                {
                    app.tools.message =
                        "Inputs changed during MCR; rerun with the current spectra.".into();
                    cx.notify();
                    return;
                }
                for (ix, fingerprint, spectrum) in loaded {
                    app.cache.put((ix, fingerprint), spectrum);
                }
                match result {
                    Ok(result) => {
                        let message = format!(
                            "MCR {} · {} · residual {:.2e}",
                            super::mcr_termination_label(result.termination).to_lowercase(),
                            crate::text::plural(result.labels.len(), "spectrum"),
                            result.relative_error
                        );
                        app.analysis.mcr = Some(crate::project::McrAnalysis {
                            inputs: sources
                                .iter()
                                .map(|s| crate::project::AnalysisInput {
                                    corrections: app.correction_sources(s.ix),
                                    group_id: s.group_id.clone(),
                                    label: s.label.clone(),
                                    fingerprint: s.fingerprint,
                                })
                                .collect(),
                            result,
                            comparison: None,
                        });
                        app.analysis.mcr_sources = sources;
                        app.record(message.clone(), None);
                        app.tools.message = message.into();
                        if app.tools.open == Some(Tool::Mcr) {
                            app.analysis.shown = Some(Tool::Mcr);
                            app.rebuild_analysis_plot(cx);
                        }
                        app.invalidate_explore_plots(cx);
                    }
                    Err(error) => app.tools.message = error.to_string().into(),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
        Ok(message)
    }

    fn prepare_mcr(&mut self, cx: &Context<Self>) -> Result<McrRequest, String> {
        self.tools.sync_range(cx)?;
        let mut sources = Vec::new();
        let mut inputs = Vec::new();
        for ix in super::marked_group_indices(&self.selection) {
            let source = self
                .tool_target(ix)
                .ok_or("A marked group is unavailable")?;
            let input =
                if let Some(spectrum) = self.cache.peek(&(ix, self.effective_fingerprint(ix))) {
                    Ok(spectrum.clone())
                } else {
                    let mut pending = crate::app::missing_compare_loads(
                        &self.catalog,
                        &self.derived,
                        &self.params,
                        &self.overrides,
                        &[ix],
                        &self.cache,
                    );
                    Err(pending.pop().ok_or("A marked spectrum cannot be loaded")?)
                };
            sources.push(source);
            inputs.push(input);
        }
        if inputs.len() < 2 {
            return Err("Mark at least two spectra to resolve components".into());
        }
        let integer = |field, default: f64, maximum: f64| -> Result<usize, String> {
            let value = self.tools.field_value(field, cx).unwrap_or(default);
            if !value.is_finite() || value < 0.0 || value > maximum || value.fract() != 0.0 {
                return Err("Component count, iteration limit and seed must be nonnegative integers within their limits".into());
            }
            Ok(value as usize)
        };
        let components = integer(ToolField::McrComponents, 3.0, 20.0)?;
        let max_iterations = integer(ToolField::McrIterations, 500.0, 100_000.0)?;
        if components == 0 || max_iterations == 0 {
            return Err("Component count and iteration limit must be positive".into());
        }
        let seed = integer(ToolField::McrSeed, 0.0, u32::MAX as f64)? as u64;
        let config = McrConfig {
            components,
            max_iterations,
            seed,
            range: self.tools.lcf_range,
            space: self.tools.lcf_space.space(2.0),
            sum_to_one: self.tools.lcf_sum_to_one,
            nonnegative_spectra: self.tools.mcr_nonnegative_spectra,
            ..McrConfig::default()
        };
        Ok(McrRequest {
            inputs,
            sources,
            config,
        })
    }
}
