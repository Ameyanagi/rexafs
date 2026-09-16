use super::*;
use rexafs::prelude::{AxisOrigin, Metric};

fn input(label: &str, height: f64) -> FrameInput {
    let group = GroupId::new_result();
    FrameInput {
        group: group.clone(),
        label: label.into(),
        path: PathBuf::new(),
        settings: PipelineParams::default(),
        derived: Some(Arc::new(DerivedSpectrum {
            group_id: Some(group),
            label: label.into(),
            energy: vec![0., 0.2, 1., 3.],
            mu: vec![height; 4],
            ..Default::default()
        })),
    }
}
fn definition() -> MetricDefinition {
    MetricDefinition {
        id: GroupId::new_result(),
        revision: 1,
        name: "Mean".into(),
        edge_energy: false,
        measurement: Measurement {
            space: MeasurementSpace::Mu,
            origin: AxisOrigin::Absolute,
            metric: Metric::Mean {
                start: 0.1,
                end: 2.5,
            },
        },
    }
}
fn series(inputs: &[FrameInput]) -> SeriesDefinition {
    SeriesDefinition {
        id: GroupId::new_result(),
        revision: 1,
        name: "Synthetic series".into(),
        ordering: "explicit".into(),
        frames: inputs
            .iter()
            .enumerate()
            .map(|(i, input)| SeriesFrame {
                id: GroupId::new_result(),
                group: input.group.clone(),
                label: input.label.clone(),
                sequence: i + 1,
            })
            .collect(),
    }
}

#[test]
fn every_frame_including_overview_omissions_is_measured() {
    let inputs: Vec<_> = (0..513)
        .map(|i| input(&format!("frame {i}"), if i == 257 { 19. } else { 1. }))
        .collect();
    let mut run = SeriesRun::new(&series(&inputs), definition(), &inputs);
    run.freeze(&inputs, || false);
    for (row, input) in run.rows.iter_mut().zip(&inputs) {
        *row = calculate_row(input, row, &run.definition);
    }
    run.finish(false);
    assert!(run.complete);
    assert!(run.rows.iter().all(|r| r.status == FrameStatus::Succeeded));
    assert_eq!(run.rows[257].result.as_ref().unwrap().value, 19.);
    assert_eq!(run.csv().lines().count(), 514);
    assert!(run.rows.iter().all(|r| r.preparation["fourier"].is_null()));
}

#[test]
fn source_and_settings_changes_fail_without_rewriting_history() {
    let inputs = vec![input("one", 2.)];
    let mut run = SeriesRun::new(&series(&inputs), definition(), &inputs);
    run.freeze(&inputs, || false);
    let old = calculate_row(&inputs[0], &run.rows[0], &run.definition);
    assert_eq!(old.status, FrameStatus::Succeeded);
    let mut edited = inputs[0].clone();
    Arc::make_mut(edited.derived.as_mut().unwrap()).mu[0] = 20.;
    let fail = calculate_row(&edited, &run.rows[0], &run.definition);
    assert_eq!(fail.status, FrameStatus::Failed);
    assert!(fail.reason.unwrap().contains("Inputs changed"));
    edited = inputs[0].clone();
    edited.settings.e0 = Some(1.);
    assert_eq!(
        calculate_row(&edited, &run.rows[0], &run.definition).status,
        FrameStatus::Failed
    );
    assert_eq!(old.result.unwrap().value, 2.);
    edited = inputs[0].clone();
    edited.label = "new name".into();
    Arc::make_mut(edited.derived.as_mut().unwrap()).label = "new name".into();
    assert_eq!(edited.revision().unwrap(), inputs[0].revision().unwrap());
}

#[test]
fn cancellation_keeps_success_and_gaps_resume_with_same_revision() {
    let inputs = vec![input("a", 1.), input("b", 2.), input("c", 3.)];
    let mut run = SeriesRun::new(&series(&inputs), definition(), &inputs);
    run.freeze(&inputs, || false);
    run.rows[0] = calculate_row(&inputs[0], &run.rows[0], &run.definition);
    run.finish(true);
    assert_eq!(run.rows[0].status, FrameStatus::Succeeded);
    assert_eq!(run.rows[1].status, FrameStatus::Cancelled);
    let saved = serde_json::to_string(&run).unwrap();
    let mut restored: SeriesRun = serde_json::from_str(&saved).unwrap();
    for (row, input) in restored.rows.iter_mut().zip(&inputs).skip(1) {
        *row = calculate_row(input, row, &restored.definition);
    }
    restored.finish(false);
    assert!(restored.complete);
    assert!(!restored.cancelled);
    assert_eq!(
        serde_json::to_value(&restored.rows[0]).unwrap(),
        serde_json::to_value(&run.rows[0]).unwrap()
    );
}

#[test]
fn short_xanes_normalization_never_runs_invalid_exafs_stages() {
    let x: Vec<_> = (0..201).map(|i| 9900. + i as f64).collect();
    let y: Vec<_> = x
        .iter()
        .map(|e| 0.2 + 0.00001 * (e - 10000.) + 1. / (1. + (-(e - 10000.) / 1.5).exp()))
        .collect();
    let settings = PipelineParams {
        e0: Some(10000.),
        pre_edge_start: Some(-90.),
        pre_edge_end: Some(-20.),
        norm_start: Some(20.),
        norm_end: Some(90.),
        norm_polyorder: Some(1),
        bkg_nclamp: Some(-1),
        fft_nfft: Some(1),
        bft_nfft: Some(1),
        ..Default::default()
    };
    let normalized =
        params::prepare_arrays(x.clone(), y.clone(), &settings, RequiredStage::Normalized).unwrap();
    assert!(normalized.norm().is_some());
    assert!(normalized.k().is_none());
    assert!(normalized.xftf.is_none());
    assert!(params::process_arrays(x, y, &settings).is_err());
    let value = normalized.measure(&Measurement::mean(20.0..=70.0)).unwrap();
    assert!((value.value - 1.).abs() < 1e-5);
}

#[test]
fn unchanged_prepared_flat_is_measured_without_refitting() {
    let mut source = input("flat", 0.75);
    let group = Arc::make_mut(source.derived.as_mut().unwrap());
    group.quantity = params::Quantity::FlattenedMu;
    source.settings.e0 = Some(1.);
    source.settings.bkg_nclamp = Some(-1);
    let mut def = definition();
    def.measurement.space = MeasurementSpace::Flat;
    let (sp, _, _) = source.prepare(&def, None).unwrap();
    assert!(sp.k().is_none());
    assert_eq!(sp.measure(&def.measurement).unwrap().value, 0.75);
}

#[test]
fn project_roundtrip_preserves_ids_definitions_and_failed_rows() {
    let mut missing = input("missing", 1.);
    missing.derived = None;
    missing.path = PathBuf::from("/missing/synthetic-data.xdi");
    let inputs = vec![input("good", 2.), missing];
    let series = series(&inputs);
    let mut run = SeriesRun::new(&series, definition(), &inputs);
    run.freeze(&inputs, || false);
    run.rows[0] = calculate_row(&inputs[0], &run.rows[0], &run.definition);
    run.finish(false);
    assert!(run.complete);
    assert_eq!(run.rows[1].status, FrameStatus::Unavailable);
    assert!(run.rows[1].result.is_none());
    let archive = SeriesArchive {
        series: vec![series],
        runs: vec![run],
    };
    let project = crate::project::ProjectFile {
        version: crate::project::PROJECT_VERSION,
        series_measurements: archive.clone(),
        ..Default::default()
    };
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("measurements.rxs");
    crate::project::save(&path, &project).unwrap();
    let loaded = crate::project::load(&path).unwrap();
    assert_eq!(
        serde_json::to_value(loaded.series_measurements).unwrap(),
        serde_json::to_value(archive).unwrap()
    );
}

#[test]
fn immutable_file_snapshots_do_not_bridge_skipped_rows() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("spectrum.dat");
    std::fs::write(&path, "# energy mu\n0 1\n1 2\n2 3\n3 4\n").unwrap();
    let mut source = input("file", 1.);
    source.derived = None;
    source.path = path.clone();
    let inputs = vec![source.clone()];
    let mut run = SeriesRun::new(&series(&inputs), definition(), &inputs);
    run.freeze(&inputs, || false);
    assert_eq!(
        calculate_row(&source, &run.rows[0], &run.definition).status,
        FrameStatus::Succeeded
    );
    std::fs::write(&path, "# energy mu\n0 1\n1 bad\n2 3\n3 4\n").unwrap();
    assert!(source.prepare(&run.definition, None).is_err());
    assert_eq!(
        calculate_row(&source, &run.rows[0], &run.definition).status,
        FrameStatus::Failed
    );
}

/// Explicit resource qualification, not part of routine unit-test latency.
#[test]
#[ignore = "100,000-frame synthetic resource qualification"]
fn hundred_thousand_frames_use_one_prepared_spectrum_at_a_time() {
    let mut prototype = input("synthetic", 1.);
    let group = Arc::make_mut(prototype.derived.as_mut().unwrap());
    group.energy = (0..256).map(|i| i as f64).collect();
    group.mu = vec![1.; 256];
    let definition = definition();
    let (source, revision) = prototype.revision().unwrap();
    let row = MetricRow {
        frame: SeriesFrame {
            id: GroupId::new_result(),
            group: prototype.group.clone(),
            label: "synthetic".into(),
            sequence: 1,
        },
        source_digest: Some(source),
        input_revision: Some(revision),
        settings: prototype.settings.clone(),
        status: FrameStatus::Pending,
        result: None,
        reason: None,
        preparation: serde_json::Value::Null,
    };
    let start = std::time::Instant::now();
    let mut values = Vec::with_capacity(100_000);
    for _ in 0..100_000 {
        let output = calculate_row(&prototype, &row, &definition);
        assert_eq!(output.status, FrameStatus::Succeeded);
        values.push(output.result.unwrap().value);
    }
    eprintln!(
        "100000 synthetic frames; 256 points/frame; serial; {:.3}s; {} scalar bytes",
        start.elapsed().as_secs_f64(),
        values.len() * 8
    );
    assert!(values.iter().all(|v| *v == 1.));
}

#[test]
fn missing_worker_inputs_keep_requested_rows_visible() {
    let inputs = vec![input("first", 1.), input("second", 2.)];
    let requested = series(&inputs);
    let mut run = SeriesRun::new(&requested, definition(), &inputs[..1]);
    run.freeze(&inputs[..1], || false);
    assert_eq!(run.rows.len(), 2);
    assert_eq!(run.rows[1].status, FrameStatus::Unavailable);
    assert!(run.rows[1].result.is_none());
    assert_eq!(
        calculate_row(&inputs[1], &run.rows[0], &run.definition).status,
        FrameStatus::Unavailable
    );
}
