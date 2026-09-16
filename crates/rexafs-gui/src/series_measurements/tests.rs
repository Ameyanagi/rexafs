use super::*;
use rexafs::prelude::{AxisOrigin, Metric};

fn input(label: &str, height: f64) -> FrameInput {
    let group = GroupId::new_result();
    FrameInput {
        group: group.clone(),
        label: label.into(),
        path: PathBuf::new(),
        settings: PipelineParams::default(),
        recipe: None,
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
        coordinate: Default::default(),
        frames: inputs
            .iter()
            .enumerate()
            .map(|(i, input)| SeriesFrame {
                id: GroupId::new_result(),
                group: input.group.clone(),
                label: input.label.clone(),
                sequence: i + 1,
                coordinate: None,
                acquired_at: None,
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
        runs: vec![Arc::new(run)],
        presets: vec![],
        recipes: vec![],
        live_sessions: vec![],
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
            coordinate: None,
            acquired_at: None,
        },
        source_digest: Some(source),
        input_revision: Some(revision),
        settings: Arc::new(prototype.settings.clone()),
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

#[test]
fn membership_revisions_never_reassign_saved_results() {
    let inputs = vec![input("scan10", 10.), input("scan2", 2.), input("scan1", 1.)];
    let mut series = series(&inputs);
    let mut run = SeriesRun::new(&series, definition(), &inputs);
    run.freeze(&inputs, || false);
    for (row, input) in run.rows.iter_mut().zip(&inputs) {
        *row = calculate_row(input, row, &run.definition);
    }
    let original = serde_json::to_value(&run).unwrap();
    series.sort_naturally();
    assert_eq!(
        series
            .frames
            .iter()
            .map(|f| f.label.as_str())
            .collect::<Vec<_>>(),
        vec!["scan1", "scan2", "scan10"]
    );
    series.frames.remove(1);
    series.revise();
    series.name = "Renamed".into();
    series.revise();
    assert_eq!(serde_json::to_value(&run).unwrap(), original);
    assert_eq!(run.rows[0].result.as_ref().unwrap().value, 10.);
    assert!(series.revision > run.series_revision);
}

#[test]
fn coordinate_sidecars_are_atomic_id_keyed_and_timezone_aware() {
    let inputs = vec![input("a", 1.), input("b", 2.), input("c", 3.)];
    let mut series = series(&inputs);
    series.coordinate = CoordinateDefinition {
        label: "Temperature".into(),
        unit: "K".into(),
        source: "thermocouple".into(),
        timestamp_meaning: "start".into(),
    };
    series.frames[0].coordinate = Some(300.);
    series.frames[1].coordinate = Some(300.);
    series.frames[0].acquired_at = Some("2026-09-16T12:00:03+09:00".into());
    series.frames[1].acquired_at = Some("2026-09-16T03:00:01Z".into());
    let sidecar = series.coordinates_csv().unwrap();
    let mut unlabelled = series.clone();
    unlabelled.coordinate = Default::default();
    let before = unlabelled.frames.clone();
    assert!(unlabelled.import_coordinates(&sidecar).is_err());
    assert_eq!(
        serde_json::to_value(unlabelled.frames).unwrap(),
        serde_json::to_value(before).unwrap()
    );
    let mut restored = series.clone();
    restored.frames.reverse();
    restored.import_coordinates(&sidecar).unwrap();
    assert_eq!(restored.frames[0].coordinate, None);
    restored.sort_acquisition().unwrap();
    assert_eq!(restored.frames[0].group, inputs[1].group);
    let run = SeriesRun::new(&restored, definition(), &inputs);
    assert_eq!(
        run.plot_coordinates(TrendAxis::ElapsedAcquisition),
        vec![Some(0.), Some(2.), None]
    );
    assert_eq!(
        run.plot_coordinates(TrendAxis::Coordinate),
        vec![Some(300.), Some(300.), None]
    );
    let before = serde_json::to_value(&restored).unwrap();
    let malformed = String::from_utf8(sidecar)
        .unwrap()
        .replace("2026-09-16T12:00:03+09:00", "2026-09-16T12:00:03");
    assert!(restored.import_coordinates(malformed.as_bytes()).is_err());
    assert_eq!(serde_json::to_value(restored).unwrap(), before);
    assert!(
        super::catalogue::natural_cmp("scan9999999999999999999999", "scan10000000000000000000000")
            .is_lt()
    );
}

#[test]
fn unsaved_preset_edits_reserve_distinct_revisions_in_runs() {
    let inputs = vec![input("one", 1.)];
    let mut preset = definition();
    let mut archive = SeriesArchive {
        presets: vec![preset.clone()],
        ..Default::default()
    };
    preset.measurement.metric = Metric::Point { x: 0.5 };
    preset.revision = archive.definition_revision(&preset);
    assert_eq!(preset.revision, 2);
    archive.runs.push(Arc::new(SeriesRun::new(
        &series(&inputs),
        preset.clone(),
        &inputs,
    )));
    preset.measurement.metric = Metric::Point { x: 1.5 };
    assert_eq!(archive.definition_revision(&preset), 3);
    preset.measurement.metric = Metric::Point { x: 0.5 };
    assert_eq!(archive.definition_revision(&preset), 2);
}

#[test]
fn recovery_keeps_only_complete_chunks_and_never_changes_the_saved_project() {
    use std::io::Write;
    let inputs = vec![input("one", 1.), input("two", 2.), input("three", 3.)];
    let series = series(&inputs);
    let mut run = SeriesRun::new(&series, definition(), &inputs);
    run.freeze(&inputs, || false);
    let project = crate::project::ProjectFile {
        version: 1,
        series_measurements: SeriesArchive {
            series: vec![series],
            runs: vec![Arc::new(run.clone())],
            ..Default::default()
        },
        ..Default::default()
    };
    let dir = tempfile::tempdir().unwrap();
    let original = dir.path().join("original.rxs");
    crate::project::save(&original, &project).unwrap();
    let original_bytes = std::fs::read(&original).unwrap();
    let root = dir.path().join("recovery");
    let mut journal = recovery::RecoveryWriter::begin(&root, project, &run).unwrap();
    assert!(
        recovery::discover(&root).unwrap().is_empty(),
        "active checkpoint must not be offered"
    );
    let row = calculate_row(&inputs[0], &run.rows[0], &run.definition);
    journal.rows(0, &[row.clone()]).unwrap();
    drop(journal);
    let entry = recovery::discover(&root).unwrap().pop().unwrap();
    std::fs::OpenOptions::new()
        .append(true)
        .open(entry.directory.join("rows.jsonl"))
        .unwrap()
        .write_all(b"{\"Rows\":{\"begin\":1")
        .unwrap();
    let (recovered, committed) = recovery::recover(&entry).unwrap();
    assert_eq!(committed, 1);
    let restored = &recovered.series_measurements.runs[0];
    assert_eq!(restored.rows.len(), 3);
    assert_eq!(restored.rows[0].result.as_ref().unwrap().value, 1.);
    assert_eq!(restored.rows[1].status, FrameStatus::Cancelled);
    assert!(restored.cancelled);
    assert!(recovered.origin.is_none());
    assert_eq!(std::fs::read(&original).unwrap(), original_bytes);
    // A completed malformed line must fail, instead of silently hiding corruption.
    std::fs::OpenOptions::new()
        .append(true)
        .open(entry.directory.join("rows.jsonl"))
        .unwrap()
        .write_all(b"\n")
        .unwrap();
    assert!(recovery::recover(&entry).is_err());
    recovery::discard(&entry).unwrap();
    assert!(recovery::discover(&root).unwrap().is_empty());
}

#[test]
fn cancelled_before_snapshot_cannot_relabel_changed_settings() {
    let inputs = vec![input("one", 1.)];
    let mut run = SeriesRun::new(&series(&inputs), definition(), &inputs);
    run.freeze(&inputs, || true);
    run.finish(true);
    let mut changed = inputs;
    changed[0].settings.e0 = Some(1.);
    run.freeze(&changed, || false);
    assert_eq!(run.rows[0].status, FrameStatus::Failed);
    assert!(run.rows[0].input_revision.is_none());
}

#[test]
fn compact_runs_share_settings_and_retain_prototype_rows() {
    let inputs: Vec<_> = (0..1000)
        .map(|i| input(&format!("frame {i}"), 1.))
        .collect();
    let mut run = SeriesRun::new(&series(&inputs), definition(), &inputs);
    assert!(Arc::ptr_eq(&run.rows[0].settings, &run.rows[999].settings));
    Arc::make_mut(&mut run.rows[999].settings).e0 = Some(1234.);
    assert_eq!(run.rows[0].settings.e0, None);
    let compact = serde_json::to_value(&run).unwrap();
    assert_eq!(compact["rows"]["schema"], 1);
    assert_eq!(compact["rows"]["settings"].as_array().unwrap().len(), 2);
    let mut legacy = compact.clone();
    legacy["rows"] = serde_json::to_value(&run.rows).unwrap();
    assert!(
        serde_json::to_vec(&compact).unwrap().len()
            < serde_json::to_vec(&legacy).unwrap().len() / 2
    );
    for value in [compact.clone(), legacy] {
        let loaded: SeriesRun = serde_json::from_value(value).unwrap();
        assert_eq!(serde_json::to_value(&loaded).unwrap(), compact);
        assert!(Arc::ptr_eq(
            &loaded.rows[0].settings,
            &loaded.rows[998].settings
        ));
        assert_eq!(loaded.rows[999].settings.e0, Some(1234.));
    }
    let mut damaged = compact.clone();
    damaged["rows"]["values"][0]["settings"] = 999.into();
    assert!(serde_json::from_value::<SeriesRun>(damaged).is_err());
    let mut future = compact;
    future["rows"]["schema"] = 2.into();
    assert!(serde_json::from_value::<SeriesRun>(future).is_err());
    let archive = SeriesArchive {
        runs: vec![Arc::new(run)],
        ..Default::default()
    };
    let snapshot = archive.clone();
    assert!(Arc::ptr_eq(&archive.runs[0], &snapshot.runs[0]));
}

#[test]
fn recipes_replay_frozen_choices_and_reject_changed_quantities() {
    let original = input("representative", 2.);
    let mut archive = SeriesArchive::default();
    let recipe = AnalysisRecipe::capture("Raw mean".into(), definition(), &original).unwrap();
    let id = archive.save_recipe(recipe.clone());
    assert_eq!(archive.save_recipe(recipe.clone()), id);
    assert_eq!(archive.recipes.len(), 1);
    let mut changed = recipe.clone();
    changed.settings.e0 = Some(1.);
    archive.save_recipe(changed);
    assert_eq!(archive.recipes.len(), 2);
    assert_eq!(archive.recipes[0].revision, 1);
    assert_eq!(archive.recipes[1].revision, 2);
    assert_eq!(archive.recipes[0].settings.e0, None);
    let mut target = input("other acquisition", 5.);
    target.settings.e0 = Some(42.);
    let target = target.with_recipe(Some(archive.recipes[0].clone()));
    assert_eq!(target.settings.e0, None);
    assert_eq!(original.settings.e0, None);
    let mut run = SeriesRun::new(
        &series(std::slice::from_ref(&target)),
        recipe.definition.clone(),
        std::slice::from_ref(&target),
    );
    run.freeze(std::slice::from_ref(&target), || false);
    run.rows[0] = calculate_row(&target, &run.rows[0], &run.definition);
    assert_eq!(run.rows[0].status, FrameStatus::Succeeded);
    assert_eq!(run.rows[0].result.as_ref().unwrap().value, 5.);
    let saved = serde_json::to_vec(&run).unwrap();
    let retained: SeriesRun = serde_json::from_slice(&saved).unwrap();
    assert_eq!(retained.recipe.as_ref().unwrap().revision, 1);
    assert!(retained.csv().contains("recipe_revision,recipe_name"));
    let mut wrong = target.clone();
    Arc::make_mut(wrong.derived.as_mut().unwrap()).quantity = params::Quantity::FlattenedMu;
    assert!(wrong.revision().unwrap_err().contains("incompatible"));
    let failed = calculate_row(&wrong, &retained.rows[0], &retained.definition);
    assert_eq!(failed.status, FrameStatus::Failed);
    assert!(failed.result.is_none());
    assert_eq!(retained.rows[0].result.as_ref().unwrap().value, 5.);
}

#[test]
fn recipe_mapping_checks_exact_snapshot_layout_and_units() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("scan 日本語.dat");
    let bytes = b"# energy mu\n0 2\n0.2 2\n1 2\n3 2\n";
    std::fs::write(&path, bytes).unwrap();
    let mut source = input("table", 0.);
    source.derived = None;
    source.path = path.clone();
    let recipe = AnalysisRecipe::capture("Table mean".into(), definition(), &source).unwrap();
    let source = source.with_recipe(Some(Arc::new(recipe)));
    source.prepare(&definition(), None).unwrap();
    std::fs::write(&path, b"# mu energy\n0 2\n0.2 2\n1 2\n3 2\n").unwrap();
    assert!(
        source
            .prepare(&definition(), None)
            .unwrap_err()
            .contains("incompatible")
    );
    std::fs::write(&path, bytes).unwrap();
    source.prepare(&definition(), None).unwrap();
    let mut invalid = (*source.recipe.as_ref().unwrap()).as_ref().clone();
    invalid.settings.e0 = Some(f64::NAN);
    assert!(invalid.validate().unwrap_err().contains("finite"));
    invalid.settings.e0 = None;
    invalid.schema = 99;
    assert!(invalid.validate().unwrap_err().contains("schema"));
}

#[test]
fn recipe_runs_survive_project_and_locked_recovery_roundtrips() {
    let original = input("reference", 3.);
    let recipe = Arc::new(
        AnalysisRecipe::capture("Portable recipe".into(), definition(), &original).unwrap(),
    );
    let inputs = vec![original.with_recipe(Some(recipe.clone()))];
    let series = series(&inputs);
    let mut run = SeriesRun::new(&series, recipe.definition.clone(), &inputs);
    run.freeze(&inputs, || false);
    let project = crate::project::ProjectFile {
        version: 1,
        series_measurements: SeriesArchive {
            series: vec![series],
            runs: vec![Arc::new(run.clone())],
            recipes: vec![recipe],
            ..Default::default()
        },
        ..Default::default()
    };
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("replay 日本語.rxs");
    crate::project::save(&path, &project).unwrap();
    let loaded = crate::project::load(&path).unwrap();
    assert_eq!(
        serde_json::to_value(&project.series_measurements).unwrap(),
        serde_json::to_value(&loaded.series_measurements).unwrap()
    );
    let root = dir.path().join("recovery");
    let mut writer = recovery::RecoveryWriter::begin(&root, loaded, &run).unwrap();
    assert!(recovery::discover(&root).unwrap().is_empty());
    run.rows[0] = calculate_row(&inputs[0], &run.rows[0], &run.definition);
    writer.rows(0, &run.rows).unwrap();
    drop(writer);
    let entry = recovery::discover(&root).unwrap().remove(0);
    let (recovered, count) = recovery::recover(&entry).unwrap();
    assert_eq!(count, 1);
    let retained = &recovered.series_measurements.runs[0];
    assert_eq!(retained.recipe.as_ref().unwrap().name, "Portable recipe");
    assert_eq!(retained.rows[0].result.as_ref().unwrap().value, 3.);
    assert_eq!(
        crate::project::load(&path)
            .unwrap()
            .series_measurements
            .runs[0]
            .rows[0]
            .status,
        FrameStatus::Pending
    );
}
