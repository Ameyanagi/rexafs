use super::*;
use crate::{
    group_identity::GroupId,
    params::{PipelineParams, RequiredStage},
};
fn sample() -> (FrameInput, FluorescenceCorrection) {
    let energy = (0..701).map(|i| 8579. + 2. * i as f64).collect::<Vec<_>>();
    let mu = energy
        .iter()
        .map(|e| {
            0.2 + 0.00001 * (e - 8979.)
                + 1. / (1. + (-(e - 8979.) / 1.5).exp())
                + 0.15 * (-((e - 8990.) / 8.).powi(2)).exp()
        })
        .collect();
    let input = FrameInput {
        group: GroupId::new_result(),
        label: "Synthetic Cu fluorescence".into(),
        path: Default::default(),
        recipe: None,
        settings: PipelineParams {
            e0: Some(8979.),
            ..Default::default()
        },
        derived: Some(Arc::new(DerivedSpectrum {
            energy,
            mu,
            ..Default::default()
        })),
    };
    (
        input,
        FluorescenceCorrection::new("CuO", "Cu", "K")
            .line("Ka1")
            .angles(45., 45.),
    )
}
fn record(input: &FrameInput, model: &FluorescenceCorrection) -> CorrectionRecord {
    let corrected = prepare_input(input)
        .unwrap()
        .correct_fluorescence(model)
        .unwrap();
    CorrectionRecord {
        schema: 1,
        input: crate::params::OperationInput {
            group_id: Some(input.group.clone()),
            label: input.label.clone(),
            fingerprint: input.settings.fingerprint(),
            path: Default::default(),
            derived_id: None,
            size: None,
        },
        source_digest: input.revision().unwrap().0,
        settings: input.settings.clone(),
        result: corrected.fluorescence_correction().unwrap().clone(),
    }
}
#[test]
fn fluorescence_embedded_history_normalization_and_domain_survive_reopen() {
    let (input, model) = sample();
    let r = record(&input, &model);
    let tmp = tempfile::tempdir().unwrap();
    let cache = tmp.path().join("history");
    let receipt = history::retain(&cache, &r).unwrap();
    let mut group = corrected_group(&r, receipt.clone());
    group.group_id = Some(GroupId::new_result());
    assert_eq!(input.derived.as_ref().unwrap().mu, r.result.original_mu);
    assert_eq!(group.mu, r.result.corrected_mu);
    let mut sp = group.for_display(group.params.as_ref().unwrap()).unwrap();
    assert!(sp.norm().is_some() && sp.flat().is_some() && sp.is_xanes_only());
    assert!(sp.chi().is_none());
    assert!(sp.calc_background().is_err());
    assert!(sp.correct_fluorescence(&model).is_err());
    assert!(
        group
            .prepare(group.params.as_ref().unwrap(), RequiredStage::Fourier)
            .unwrap_err()
            .contains("XANES")
    );
    let mut mb = group.params.clone().unwrap();
    mb.mback = Some(rexafs::MBack::for_edge("Cu", "K").options);
    assert!(
        group
            .prepare(&mb, RequiredStage::Normalized)
            .unwrap()
            .norm()
            .is_some()
    );
    let project = crate::project::ProjectFile {
        derived: vec![group],
        ..Default::default()
    };
    let file = tmp.path().join("correction.rxs");
    crate::project::save_with_storage(&file, &project, crate::project::DataStorage::Embedded)
        .unwrap();
    std::fs::remove_dir_all(cache).unwrap();
    let restored =
        crate::project::load_with_cache_root(&file, || Ok(tmp.path().join("restored"))).unwrap();
    assert!(restored.raw_files.is_empty());
    let group = &restored.derived[0];
    let historical = history::read(&group.corrections[0]).unwrap();
    assert_eq!(historical.result, r.result);
    assert!(
        group
            .for_display(group.params.as_ref().unwrap())
            .unwrap()
            .is_xanes_only()
    );
    std::fs::write(&group.corrections[0].path, b"broken").unwrap();
    assert!(
        history::read(&group.corrections[0])
            .err()
            .unwrap()
            .contains("checksum")
    );
}
#[test]
fn fluorescence_rejects_transmission_prepared_and_repeated_inputs() {
    let (mut input, model) = sample();
    Arc::make_mut(input.derived.as_mut().unwrap()).absorption_mode = AbsorptionMode::Transmission;
    assert!(
        prepare_input(&input)
            .unwrap()
            .correct_fluorescence(&model)
            .is_err()
    );
    let d = Arc::make_mut(input.derived.as_mut().unwrap());
    d.absorption_mode = AbsorptionMode::Unknown;
    d.operation = Some(Operation {
        tool: "Measurement import".into(),
        inputs: vec![],
        applied_energy_shift_ev: 0.,
        parameters: serde_json::json!({
            "mapping":{"energy_column":0,"energy":"Ev","signal":{"Transmission":{"incident":1,"transmitted":2}}}
        }),
    });
    // Use the actual serialized core mapping, including its default fields.
    let mapping = rexafs::io::SpectrumMapping {
        energy_column: 0,
        energy: rexafs::io::EnergyConversion::Ev,
        signal: rexafs::io::SignalConversion::Transmission {
            incident: 1,
            transmitted: 2,
        },
    };
    d.operation.as_mut().unwrap().parameters["mapping"] = serde_json::to_value(mapping).unwrap();
    assert_eq!(
        prepare_input(&input).unwrap().absorption_mode(),
        AbsorptionMode::Transmission
    );
    Arc::make_mut(input.derived.as_mut().unwrap()).quantity = Quantity::FlattenedMu;
    assert!(
        prepare_input(&input)
            .err()
            .unwrap()
            .contains("before normalization")
    );
    Arc::make_mut(input.derived.as_mut().unwrap())
        .corrections
        .push(CorrectionReceipt {
            path: "missing".into(),
            digest: "retained".into(),
        });
    assert!(
        prepare_input(&input)
            .err()
            .unwrap()
            .contains("Already corrected")
    );
}
#[test]
fn fluorescence_ancestor_receipts_follow_lcf_outputs_and_prepared_processing() {
    let (input, model) = sample();
    let r = record(&input, &model);
    let tmp = tempfile::tempdir().unwrap();
    let receipt = history::retain(tmp.path(), &r).unwrap();
    let d = corrected_group(&r, receipt.clone());
    let sp = d.for_display(d.params.as_ref().unwrap()).unwrap();
    let cfg = rexafs::prelude::LcfConfig {
        range: Some((-20., 100.)),
        space: rexafs::prelude::AnalysisSpace::Flat,
        ..Default::default()
    };
    let fit = rexafs::prelude::lcf(&sp, &[&sp], &cfg).unwrap();
    let inputs = vec![crate::project::AnalysisInput {
        corrections: vec![receipt],
        group_id: Some(input.group),
        label: input.label,
        fingerprint: 0,
    }];
    let groups = super::super::tools::result_groups::lcf_groups(&fit, &inputs, Some(&cfg)).unwrap();
    assert!(!groups[0].corrections.is_empty());
    assert!(
        groups[0]
            .prepare(&PipelineParams::default(), RequiredStage::Fourier)
            .is_err()
    );
    assert!(
        groups[0]
            .for_display(&PipelineParams::default())
            .unwrap()
            .is_xanes_only()
    );
    // A retained analysis still owns its ancestor evidence after source groups
    // have been removed, and its outputs retain the limit after project reopen.
    let project = crate::project::ProjectFile {
        lcf_analysis: Some(crate::project::LcfAnalysis {
            config: Some(cfg.clone()),
            result: fit,
            inputs,
        }),
        ..Default::default()
    };
    let file = tmp.path().join("analysis-only.rxs");
    crate::project::save_with_storage(&file, &project, crate::project::DataStorage::Embedded)
        .unwrap();
    std::fs::remove_file(&d.corrections[0].path).unwrap();
    let restored =
        crate::project::load_with_cache_root(&file, || Ok(tmp.path().join("reopened"))).unwrap();
    assert!(restored.derived.is_empty() && restored.raw_files.is_empty());
    let analysis = restored.lcf_analysis.as_ref().unwrap();
    assert_eq!(
        history::read(&analysis.inputs[0].corrections[0])
            .unwrap()
            .result,
        r.result
    );
    let outputs = super::super::tools::result_groups::lcf_groups(
        &analysis.result,
        &analysis.inputs,
        Some(&cfg),
    )
    .unwrap();
    assert!(
        outputs[0]
            .for_display(&PipelineParams::default())
            .unwrap()
            .is_xanes_only()
    );
}
