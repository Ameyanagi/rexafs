//! Materialize calculated spectra without passing them through normalization
//! again. Provenance is retained in the normal group operation metadata.

use super::*;

fn inputs(records: &[crate::project::AnalysisInput]) -> Vec<OperationInput> {
    records
        .iter()
        .map(|input| OperationInput {
            group_id: input.group_id.clone(),
            label: input.label.clone(),
            fingerprint: input.fingerprint,
            path: PathBuf::new(),
            derived_id: None,
            size: None,
        })
        .collect()
}

fn quantity(space: AnalysisSpace, residual: bool) -> Result<Quantity, String> {
    match (space,residual) {
        (AnalysisSpace::Flat,false)=>Ok(Quantity::FlattenedMu),
        (AnalysisSpace::Flat,true)=>Ok(Quantity::FlattenedDifference),
        (AnalysisSpace::Norm,false)=>Ok(Quantity::NormalizedMu),
        (AnalysisSpace::Norm,true)=>Ok(Quantity::NormalizedDifference),
        _=>Err("Add to Groups currently supports norm and flat energy spectra; other analysis arrays remain available in the analysis export".into()),
    }
}

fn calculated_group(
    label: String,
    x: &[f64],
    y: &[f64],
    quantity: Quantity,
    operation: Operation,
    e0: Option<f64>,
) -> DerivedSpectrum {
    DerivedSpectrum {
        label,
        energy: x.to_vec(),
        mu: y.to_vec(),
        quantity,
        operation: Some(operation),
        params: Some(PipelineParams {
            e0,
            ..Default::default()
        }),
        ..Default::default()
    }
}

pub(super) fn mcr_groups(
    saved: &crate::project::McrAnalysis,
) -> Result<Vec<DerivedSpectrum>, String> {
    let r = &saved.result;
    let q = quantity(r.config.space, false)?;
    let mut out = Vec::new();
    for j in 0..r.spectra.nrows() {
        let matched = saved.comparison.as_ref().and_then(|c| {
            c.component_indices
                .iter()
                .position(|&index| index == j)
                .map(|i| (c, i))
        });
        let reference=matched.and_then(|(c,i)| c.inputs.get(i).map(|input| serde_json::json!({
            "label":input.label,"group_id":input.group_id,"relative_spectral_error":c.relative_errors[i],"interpretation":"closest matched reference; not a unique chemical identification"
        })));
        let label = match matched.and_then(|(c, i)| c.inputs.get(i)) {
            Some(input) => format!("MCR {} · {} (matched)", j + 1, input.label),
            None => format!("MCR component {}", j + 1),
        };
        let parameters = serde_json::json!({
            "rexafs_version":env!("CARGO_PKG_VERSION"),"role":"estimated component spectrum",
            "space":r.config.space,"energy_range_ev":[r.x[0],r.x[r.x.len()-1]],
            "component_index":j+1,"component_count":r.config.components,
            "sum_to_one":r.config.sum_to_one,"nonnegative_spectra":r.config.nonnegative_spectra,
            "seed":r.config.seed,"tolerance":r.config.tolerance,"max_iterations":r.config.max_iterations,
            "anchors":r.config.anchors,"initial_samples":r.initial_samples,
            "initialization":if r.config.initial_spectra.is_some() {"supplied spectra; full initialization retained in project MCR result"} else {"selected input spectra"},
            "termination":r.termination,"best_iteration":r.best_iteration,
            "relative_squared_residual":r.relative_error,"matched_reference":reference,
            "coefficients_in_input_order":r.concentrations.column(j).iter().copied().collect::<Vec<_>>(),
            "array_processing":"retained calculated values; default unit edge step; explicit pre/post-edge refitting is a separate processing choice",
            "energy_origin_ev":r.e0
        });
        out.push(calculated_group(
            label,
            r.x.as_slice(),
            &r.spectra.row(j).iter().copied().collect::<Vec<_>>(),
            q,
            Operation {
                tool: "MCR-ALS".into(),
                parameters,
                inputs: inputs(&saved.inputs),
                applied_energy_shift_ev: 0.0,
            },
            r.e0,
        ));
    }
    let corrections =
        crate::fluorescence_history::combine(saved.inputs.iter().map(|i| i.corrections.as_slice()));
    for group in &mut out {
        group.corrections = corrections.clone();
    }
    Ok(out)
}

pub(in crate::app::shell) fn lcf_groups(
    result: &rexafs::prelude::LcfResult,
    records: &[crate::project::AnalysisInput],
    config: Option<&LcfConfig>,
) -> Result<Vec<DerivedSpectrum>, String> {
    let q = quantity(result.space, false)?;
    let residual_quantity = quantity(result.space, true)?;
    let name = records.first().map_or("target", |r| r.label.as_str());
    let mut series = vec![
        (
            format!("LCF fit · {name}"),
            result.fit.as_slice().to_vec(),
            q,
            "fitted sum",
            None,
        ),
        (
            format!("LCF residual · {name}"),
            result.residual.as_slice().to_vec(),
            residual_quantity,
            "data minus fit",
            None,
        ),
    ];
    for (i, (weights, component)) in result.weights.iter().zip(&result.components).enumerate() {
        series.push((
            format!("LCF contribution · {} · {name}", weights.name),
            component.as_slice().to_vec(),
            q,
            "weighted reference contribution",
            Some(i),
        ));
    }
    let corrections =
        crate::fluorescence_history::combine(records.iter().map(|i| i.corrections.as_slice()));
    let mut groups: Vec<_> = series.into_iter().map(|(label,y,q,role,index)| calculated_group(label,result.x.as_slice(),&y,q,
        Operation {tool:"Linear combination fit".into(),inputs:inputs(records),applied_energy_shift_ev:0.0,
            parameters:serde_json::json!({"rexafs_version":env!("CARGO_PKG_VERSION"),"role":role,"space":result.space,
                "energy_range_ev":[result.x[0],result.x[result.x.len()-1]],"weights":result.weights,
                "configuration":config,"contribution_index":index,"r_factor":result.r_factor,
                "array_processing":"retained calculated values; preserve scale by default; optional pre/post-edge refitting for absorption outputs is a separate processing choice"})},None)).collect();
    for group in &mut groups {
        group.corrections = corrections.clone();
    }
    Ok(groups)
}

impl StudioApp {
    pub(crate) fn add_analysis_groups(&mut self, tool: Tool, cx: &mut Context<Self>) {
        let groups = match tool {
            Tool::Mcr => self
                .analysis
                .mcr
                .as_ref()
                .ok_or("Run MCR first".to_owned())
                .and_then(mcr_groups),
            Tool::Lcf => self
                .analysis
                .lcf
                .as_ref()
                .ok_or("Run LCF first".to_owned())
                .and_then(|r| {
                    lcf_groups(
                        r,
                        &self.analysis.lcf_inputs,
                        self.analysis.lcf_config.as_ref(),
                    )
                }),
            _ => Err("This analysis has no spectrum export".into()),
        };
        let groups = match groups {
            Ok(groups) => groups,
            Err(error) => {
                self.tools.message = error.into();
                cx.notify();
                return;
            }
        };
        let count = groups.len();
        let first = self.derived.len();
        for mut group in groups {
            let input = group.operation.as_ref().and_then(|op| op.inputs.first());
            let origin = input.and_then(|input| {
                input
                    .group_id
                    .as_ref()
                    .and_then(|id| self.group_registry.index(id))
                    .filter(|&ix| self.effective_fingerprint(ix) == input.fingerprint)
            });
            let e0 = origin.and_then(|ix| {
                self.cache
                    .peek(&(ix, self.effective_fingerprint(ix)))
                    .and_then(|s| s.e0())
            });
            if let Some(params) = &mut group.params {
                params.e0 = params.e0.or(e0);
            }
            if let Some(operation) = &mut group.operation {
                operation.parameters["energy_origin_ev"] =
                    serde_json::json!(group.params.as_ref().and_then(|p| p.e0));
            }
            group.declared_edge = origin.and_then(|ix| self.group_declared_edge(ix));
            group.id = self.next_group_id();
            group.group_id = Some(crate::group_identity::GroupId::new_result());
            let index = self.derived.len();
            self.record(
                format!("Added calculated spectrum: {}", group.label),
                Some(super::super::journal::UndoOp::DerivedAdd {
                    index,
                    spectrum: group.clone(),
                }),
            );
            self.derived.push(group);
        }
        self.group_registry.append_derived(&self.derived, first);
        self.rekey_after_catalog_change();
        self.tools.message = format!(
            "Added {} calculated {} with analysis metadata. Mark them with references to compare.",
            count,
            crate::text::noun_for(count, "group")
        )
        .into();
        cx.notify();
    }
}
