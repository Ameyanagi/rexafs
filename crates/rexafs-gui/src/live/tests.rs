use super::*;
use rexafs::prelude::Measurement as MetricMeasurement;
use std::time::Duration;

fn spectrum(value: f64) -> String {
    format!(
        "# XDI/1.0 test\n# Column.1: energy eV\n# Column.2: mu\n# ///\n# ----\n# energy mu\n7100 {value}\n7101 {}\n7102 {}\n",
        value + 1.,
        value + 2.
    )
}
fn config(folder: &Path) -> LiveConfig {
    let document = rexafs::xafs::io::reader::parse_measurement(spectrum(1.).as_bytes()).unwrap();
    LiveConfig {
        processing_reference: None,
        schema: 1,
        id: GroupId::new_result(),
        name: "Synthetic Live".into(),
        folder: folder.into(),
        pattern: "*.xdi".into(),
        recursive: false,
        policy: CompletionPolicy::Quiet {
            checks: 2,
            interval_ms: 1,
        },
        settings: PipelineParams::default(),
        definition: MetricDefinition {
            id: GroupId::new_result(),
            revision: 1,
            name: "Mean".into(),
            measurement: MetricMeasurement::mean(7100.0..=7102.0).raw_mu().absolute(),
            edge_energy: false,
            wavelet: None,
        },
        recipe: None,
        peak_model: None,
        exafs: None,
        merge: LiveMerge::Individual,
        layouts: vec![
            ScanLayout::capture(
                &document,
                &document.scans[0],
                document.scans[0].signals[0].mapping.clone(),
            )
            .unwrap(),
        ],
        created: chrono::Utc::now().to_rfc3339(),
    }
}
fn tick(engine: &mut LiveEngine, start: Instant) -> Vec<LiveRecord> {
    let cancel = AtomicBool::new(false);
    engine.poll(start, 10, &cancel).unwrap();
    engine
        .poll(start + Duration::from_millis(2), 10, &cancel)
        .unwrap()
}

#[test]
fn live_commits_before_publication_and_restart_is_idempotent() {
    let tmp = tempfile::tempdir().unwrap();
    let folder = tmp.path().join("input");
    std::fs::create_dir(&folder).unwrap();
    std::fs::write(folder.join("scan.xdi"), spectrum(1.)).unwrap();
    let store =
        LiveStore::create(&tmp.path().join("cache"), config(&folder), BTreeMap::new()).unwrap();
    let directory = store.directory.clone();
    let mut engine = LiveEngine::open(store).unwrap();
    let records = tick(&mut engine, Instant::now());
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].frames[0].row.result.as_ref().unwrap().value, 2.);
    assert_eq!(engine.store.records().unwrap().len(), 1);
    assert!(LiveStore::open(&directory).is_err());
    drop(engine);
    let mut restored = LiveEngine::open(LiveStore::open(&directory).unwrap()).unwrap();
    assert!(tick(&mut restored, Instant::now()).is_empty());
    assert_eq!(restored.store.records().unwrap()[0].key, records[0].key);
    let group = &records[0].frames[0].group;
    assert!(group.energy.is_empty() && group.mu.is_empty());
    assert_eq!(
        group.raw(group.params.as_ref().unwrap()).unwrap().1,
        vec![1., 2., 3.]
    );
}

#[test]
fn live_peak_recipe_keeps_initial_values_failures_and_durable_results() {
    use crate::peak_fits::{self, PeakSavedModel};
    use rexafs::prelude::PeakFit;
    let tmp = tempfile::tempdir().unwrap();
    let folder = tmp.path().join("input");
    std::fs::create_dir(&folder).unwrap();
    let mut config = config(&folder);
    let model = PeakFit::new(7100.0..=7102.0)
        .raw_mu()
        .absolute()
        .constant_baseline(0.);
    config.peak_model = Some(PeakSavedModel {
        id: GroupId::new_result(),
        revision: 3,
        name: "Synthetic constant".into(),
        model,
    });
    std::fs::write(folder.join("a.xdi"), spectrum(1.)).unwrap();
    let mut engine = LiveEngine::open(
        LiveStore::create(&tmp.path().join("cache"), config, BTreeMap::new()).unwrap(),
    )
    .unwrap();
    let directory = engine.store.directory.clone();
    let first = tick(&mut engine, Instant::now());
    let row = first[0].frames[0].peak.as_ref().unwrap();
    assert_eq!(row.status, FrameStatus::Succeeded);
    let record = peak_fits::read(row).unwrap();
    assert_eq!(
        record.result.definition.parameters.vars["baseline_offset"].value,
        0.
    );
    assert!((record.result.parameters.vars["baseline_offset"].value - 2.).abs() < 1e-8);
    assert_eq!(record.result.data, vec![1., 2., 3.]);
    assert!(record.settings.is_some());
    // A compatible but shorter source has a retained failure, not a missing frame.
    std::fs::write(folder.join("b.xdi"), spectrum(4.).replace("7102 6\n", "")).unwrap();
    let second = tick(&mut engine, Instant::now() + Duration::from_secs(1));
    assert_eq!(second.len(), 1);
    assert_eq!(
        second[0].frames[0].peak.as_ref().unwrap().status,
        FrameStatus::Failed
    );
    drop(engine);
    std::fs::remove_dir_all(folder).unwrap();
    let store = LiveStore::open(&directory).unwrap();
    assert_eq!(store.config.peak_model.as_ref().unwrap().revision, 3);
    assert_eq!(
        store
            .config
            .peak_model
            .as_ref()
            .unwrap()
            .model
            .parameters
            .vars["baseline_offset"]
            .value,
        0.
    );
    let records = store.records().unwrap();
    assert_eq!(records.len(), 2);
    assert_eq!(
        peak_fits::read(records[0].frames[0].peak.as_ref().unwrap())
            .unwrap()
            .result
            .model,
        record.result.model
    );
    assert_eq!(
        records[1].frames[0].peak.as_ref().unwrap().status,
        FrameStatus::Failed
    );
}

#[test]
fn excluded_initial_revision_does_not_exclude_future_revision() {
    let tmp = tempfile::tempdir().unwrap();
    let folder = tmp.path().join("input");
    std::fs::create_dir(&folder).unwrap();
    let path = folder.join("first.xdi");
    std::fs::write(&path, spectrum(1.)).unwrap();
    let baseline = BTreeMap::from([(path.clone(), digest(spectrum(1.).as_bytes()))]);
    let mut engine = LiveEngine::open(
        LiveStore::create(&tmp.path().join("cache"), config(&folder), baseline).unwrap(),
    )
    .unwrap();
    let now = Instant::now();
    assert!(tick(&mut engine, now).is_empty());
    assert_eq!(engine.states[&path], SourceState::Excluded);
    std::fs::write(&path, spectrum(10.)).unwrap();
    assert_eq!(tick(&mut engine, now + Duration::from_secs(1)).len(), 1);
}

#[test]
fn changed_layout_waits_for_review_while_other_files_continue() {
    let tmp = tempfile::tempdir().unwrap();
    let folder = tmp.path().join("input");
    std::fs::create_dir(&folder).unwrap();
    let mut engine = LiveEngine::open(
        LiveStore::create(&tmp.path().join("cache"), config(&folder), BTreeMap::new()).unwrap(),
    )
    .unwrap();
    std::fs::write(folder.join("good.xdi"), spectrum(1.)).unwrap();
    std::fs::write(
        folder.join("other.xdi"),
        spectrum(2.).replace("energy eV", "energy keV"),
    )
    .unwrap();
    let records = tick(&mut engine, Instant::now());
    assert_eq!(records.len(), 1);
    assert_eq!(engine.progress().review.len(), 1);
    std::fs::write(folder.join("other.xdi"), spectrum(2.)).unwrap();
    engine.retry();
    assert_eq!(
        tick(&mut engine, Instant::now() + Duration::from_secs(2)).len(),
        1
    );
}

#[test]
fn bounded_poll_visits_every_path_and_cancellation_publishes_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    let folder = tmp.path().join("input");
    std::fs::create_dir(&folder).unwrap();
    for i in 0..23 {
        std::fs::write(folder.join(format!("{i:03}.XDI")), spectrum(i as f64)).unwrap();
    }
    let mut engine = LiveEngine::open(
        LiveStore::create(&tmp.path().join("cache"), config(&folder), BTreeMap::new()).unwrap(),
    )
    .unwrap();
    let now = Instant::now();
    let mut total = 0;
    for i in 0..40 {
        let records = engine
            .poll(
                now + Duration::from_millis(i * 2),
                3,
                &AtomicBool::new(false),
            )
            .unwrap();
        assert!(records.len() <= 3);
        total += records.len();
    }
    assert_eq!(total, 23);
    std::fs::write(folder.join("new.xdi"), spectrum(25.)).unwrap();
    assert!(
        engine
            .poll(now + Duration::from_secs(1), 100, &AtomicBool::new(true))
            .unwrap()
            .is_empty()
    );
    assert_eq!(engine.store.records().unwrap().len(), 23);
}

#[test]
fn multiple_scans_remain_distinct_and_storage_cannot_be_watched() {
    let tmp = tempfile::tempdir().unwrap();
    let folder = tmp.path().join("input");
    std::fs::create_dir(&folder).unwrap();
    let mut config = config(&folder);
    assert!(LiveStore::create(&folder.join("output"), config.clone(), BTreeMap::new()).is_err());
    assert!(!folder.join("output").exists());
    let text = "#F test\n#S 1 first\n#N 2\n#L energy mu\n7100 1\n7101 2\n7102 3\n#S 2 second\n#N 2\n#L energy mu\n7100 4\n7101 5\n7102 6\n";
    let document = rexafs::xafs::io::reader::parse_measurement(text.as_bytes()).unwrap();
    config.pattern = "*.spec".into();
    config.layouts = vec![
        ScanLayout::capture(
            &document,
            &document.scans[0],
            document.scans[0].signals[0].mapping.clone(),
        )
        .unwrap(),
    ];
    std::fs::write(folder.join("two.spec"), text).unwrap();
    let mut engine = LiveEngine::open(
        LiveStore::create(&tmp.path().join("cache"), config, BTreeMap::new()).unwrap(),
    )
    .unwrap();
    let records = tick(&mut engine, Instant::now());
    assert_eq!(records[0].frames.len(), 2);
    let frames = &records[0].frames;
    assert_ne!(frames[0].row.frame.id, frames[1].row.frame.id);
    assert_eq!(
        frames
            .iter()
            .map(|f| f.row.result.as_ref().unwrap().value)
            .collect::<Vec<_>>(),
        vec![2., 5.]
    );
}

#[test]
fn live_revisions_and_identical_files_remain_distinct() {
    let tmp = tempfile::tempdir().unwrap();
    let folder = tmp.path().join("input");
    std::fs::create_dir(&folder).unwrap();
    let first = folder.join("a.xdi");
    let bytes = spectrum(1.);
    std::fs::write(&first, &bytes).unwrap();
    std::fs::write(folder.join("b.xdi"), &bytes).unwrap();
    let mut engine = LiveEngine::open(
        LiveStore::create(&tmp.path().join("cache"), config(&folder), BTreeMap::new()).unwrap(),
    )
    .unwrap();
    let now = Instant::now();
    let records = tick(&mut engine, now);
    assert_eq!(records.len(), 2);
    assert_ne!(records[0].key, records[1].key);
    assert_ne!(
        records[0].frames[0].group.group_id,
        records[1].frames[0].group.group_id
    );
    assert_eq!(std::fs::read(&first).unwrap(), bytes.as_bytes());
    let original = records[0].frames[0].group.source.as_ref().unwrap();
    let original_bytes = std::fs::read(original).unwrap();
    std::fs::write(&first, spectrum(100.)).unwrap();
    let revised = tick(&mut engine, now + Duration::from_secs(1));
    assert_eq!(revised.len(), 1);
    assert_ne!(revised[0].revision, records[0].revision);
    assert_ne!(
        revised[0].frames[0].group.label,
        records[0].frames[0].group.label
    );
    assert_eq!(std::fs::read(original).unwrap(), original_bytes);
    let preview = crate::params::preview_import(original, &cache_import()).unwrap();
    assert!(preview.diagnostics.warnings().is_empty());
    assert_eq!(engine.store.records().unwrap().len(), 3);
}

#[test]
fn live_failed_metric_retains_source_and_failure_row() {
    let tmp = tempfile::tempdir().unwrap();
    let folder = tmp.path().join("input");
    std::fs::create_dir(&folder).unwrap();
    std::fs::write(folder.join("a.xdi"), spectrum(1.)).unwrap();
    let mut config = config(&folder);
    config.definition.measurement = MetricMeasurement::mean(7200.0..=7300.0).raw_mu().absolute();
    let mut engine = LiveEngine::open(
        LiveStore::create(&tmp.path().join("cache"), config, BTreeMap::new()).unwrap(),
    )
    .unwrap();
    let records = tick(&mut engine, Instant::now());
    let frame = &records[0].frames[0];
    assert_eq!(frame.row.status, FrameStatus::Failed);
    assert!(frame.row.result.is_none() && frame.row.reason.is_some());
    assert!(frame.group.source.as_ref().unwrap().exists());
    assert_eq!(engine.store.records().unwrap().len(), 1);
    assert!(tick(&mut engine, Instant::now() + Duration::from_secs(1)).is_empty());
}

#[test]
fn live_embedded_project_preserves_original_bytes_and_converted_spectrum() {
    use crate::project::{self, DataStorage, ProjectFile};
    let tmp = tempfile::tempdir().unwrap();
    let folder = tmp.path().join("input");
    std::fs::create_dir(&folder).unwrap();
    let bytes = spectrum(1.);
    std::fs::write(folder.join("a.xdi"), &bytes).unwrap();
    let cache = tmp.path().join("cache");
    let mut engine =
        LiveEngine::open(LiveStore::create(&cache, config(&folder), BTreeMap::new()).unwrap())
            .unwrap();
    let record = tick(&mut engine, Instant::now()).pop().unwrap();
    let mut session = engine.store.session();
    session.published.insert(record.key);
    session.snapshots.insert(
        engine
            .store
            .directory
            .join(format!("{}.raw", record.revision)),
    );
    let mut original = ProjectFile::default();
    original.spectrum_file = record.frames[0].group.source.clone();
    original.derived.push(record.frames[0].group.clone());
    original.series_measurements.live_sessions.push(session);
    let path = tmp.path().join("portable.rxs");
    project::save_with_storage(&path, &original, DataStorage::Embedded).unwrap();
    drop(engine);
    std::fs::remove_dir_all(&cache).unwrap();
    std::fs::remove_dir_all(&folder).unwrap();
    let reopened = project::load_with_cache_root(&path, || Ok(tmp.path().join("restore"))).unwrap();
    assert!(
        reopened.raw_files.is_empty(),
        "recovery artifacts must not become catalog imports"
    );
    assert!(reopened.spectrum_file.is_none());
    assert_eq!(reopened.derived.len(), 1);
    let session = &reopened.series_measurements.live_sessions[0];
    assert_eq!(
        std::fs::read(session.snapshots.first().unwrap()).unwrap(),
        bytes.as_bytes()
    );
    let group = &reopened.derived[0];
    assert_eq!(
        group.raw(group.params.as_ref().unwrap()).unwrap().1,
        vec![1., 2., 3.]
    );
}

#[test]
fn live_reviewed_mapping_rejects_changed_units_and_ambiguous_role_guessing() {
    let bytes = b"# energy I0 It If\n7100 10 5 2\n7101 10 4 3\n7102 10 3 4\n";
    let document = rexafs::xafs::io::reader::parse_measurement(bytes).unwrap();
    let scan = &document.scans[0];
    assert!(scan.signals.len() > 1);
    assert!(preview_mapping(scan, None).is_err());
    let mapping = scan.signals[0].mapping.clone();
    let evidence = serde_json::json!({"source_record": scan, "mapping": mapping});
    assert_eq!(preview_mapping(scan, Some(&evidence)).unwrap(), mapping);
    let mut changed = scan.clone();
    changed.columns[0].units = Some("keV".into());
    assert!(preview_mapping(&changed, Some(&evidence)).is_err());
}

#[test]
fn live_kek_qd_uses_observed_angles_and_preserves_original_counts() {
    // Existing academic/nonmilitary fixture; attribution stays in its collection.
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../rexafs/tests/fixtures/xas/candidates/kek-pf/bl-9c/cu_foil_0.qd");
    let bytes = std::fs::read(source).unwrap();
    let document = rexafs::xafs::io::reader::parse_measurement(&bytes).unwrap();
    let scan = &document.scans[0];
    let mapping = preview_mapping(scan, None).unwrap();
    let expected = scan.arrays(Some(&mapping)).unwrap();
    let tmp = tempfile::tempdir().unwrap();
    let folder = tmp.path().join("input");
    std::fs::create_dir(&folder).unwrap();
    std::fs::write(folder.join("foil.qd"), &bytes).unwrap();
    let mut config = config(&folder);
    config.pattern = "*.qd".into();
    config.layouts = vec![ScanLayout::capture(&document, scan, mapping).unwrap()];
    config.definition.measurement = MetricMeasurement::mean(8990.0..=9050.0).raw_mu().absolute();
    let mut engine = LiveEngine::open(
        LiveStore::create(&tmp.path().join("cache"), config, BTreeMap::new()).unwrap(),
    )
    .unwrap();
    let record = tick(&mut engine, Instant::now()).pop().unwrap();
    let group = &record.frames[0].group;
    let actual = group.raw(group.params.as_ref().unwrap()).unwrap();
    assert_eq!(actual.0, expected.0);
    assert_eq!(actual.1, expected.1);
    assert_eq!(record.frames[0].row.status, FrameStatus::Succeeded);
    assert_eq!(
        std::fs::read(
            engine
                .store
                .directory
                .join(format!("{}.raw", record.revision))
        )
        .unwrap(),
        bytes
    );
}

#[test]
fn mback_live_and_series_use_identical_pinned_preparation() {
    let tmp = tempfile::tempdir().unwrap();
    let folder = tmp.path().join("input");
    std::fs::create_dir(&folder).unwrap();
    let (energy, mu, settings) = crate::normalization_history::tests::synthetic();
    let expected = crate::params::prepare_arrays(
        energy.clone(),
        mu.clone(),
        &settings,
        crate::params::RequiredStage::Normalized,
    )
    .unwrap();
    let measurement = MetricMeasurement::mean(-20.0..=40.0).flat();
    let expected_value = expected.measure(&measurement).unwrap().value;
    let mut text =
        "# XDI/1.0 synthetic\n# Column.1: energy eV\n# Column.2: mu\n# ///\n# ----\n# energy mu\n"
            .to_owned();
    for (e, m) in energy.iter().zip(&mu) {
        text.push_str(&format!("{e:.15} {m:.15}\n"));
    }
    let path = folder.join("synthetic-cu.xdi");
    std::fs::write(&path, text).unwrap();
    let mut config = config(&folder);
    config.settings = settings.clone();
    config.definition.measurement = measurement;
    let definition = config.definition.clone();
    let input = crate::series_measurements::FrameInput {
        group: GroupId::new_result(),
        label: "Synthetic Cu".into(),
        path,
        derived: None,
        settings,
        recipe: None,
    };
    let (series, _, _) = input.prepare(&definition, None).unwrap();
    assert!(
        (series.measure(&definition.measurement).unwrap().value - expected_value).abs() < 1e-10
    );
    let mut engine = LiveEngine::open(
        LiveStore::create(&tmp.path().join("cache"), config, BTreeMap::new()).unwrap(),
    )
    .unwrap();
    let records = tick(&mut engine, Instant::now());
    assert_eq!(records.len(), 1);
    let row = &records[0].frames[0].row;
    assert_eq!(row.status, FrameStatus::Succeeded, "{:?}", row.reason);
    assert!((row.result.as_ref().unwrap().value - expected_value).abs() < 1e-10);
    assert_eq!(
        row.preparation,
        crate::series_measurements::resolved_preparation(&series)
    );
    assert_eq!(
        row.preparation["normalization"]["resolved"]["reference"],
        serde_json::to_value(&input.settings.mback.as_ref().unwrap().reference).unwrap()
    );
    let directory = engine.store.directory.clone();
    drop(engine);
    let store = LiveStore::open(&directory).unwrap();
    assert_eq!(
        store.records().unwrap()[0].frames[0].row.preparation,
        row.preparation
    );
}

fn three_signals(transmission: f64, fluorescence: f64, reference: f64) -> String {
    let i0 = 10000.;
    let it = i0 * (-transmission).exp();
    let ir = it * (-reference).exp();
    let fluor = i0 * fluorescence;
    format!(
        "# XDI/1.0 synthetic\n# Column.1: energy eV\n# Column.2: i0 counts\n# Column.3: it counts\n# Column.4: if counts\n# Column.5: ir counts\n# ///\n# ----\n# energy i0 it if ir\n7100 {i0} {it} {fluor} {ir}\n7101 {i0} {it} {fluor} {ir}\n7102 {i0} {it} {fluor} {ir}\n"
    )
}
fn channel_config(folder: &Path, merge: LiveMerge) -> LiveConfig {
    let document = rexafs::io::parse_measurement(three_signals(1., 2., 3.).as_bytes()).unwrap();
    let scan = &document.scans[0];
    assert_eq!(scan.signals.len(), 3);
    let selected = BTreeSet::from([
        "transmission".into(),
        "fluorescence".into(),
        "reference".into(),
    ]);
    assert!(preview_channels(scan, &BTreeSet::new(), None).is_err());
    let signals = preview_channels(scan, &selected, None).unwrap();
    let mut config = config(folder);
    config.schema = 2;
    config.merge = merge;
    config.layouts = signals
        .iter()
        .map(|s| ScanLayout::capture_channel(&document, scan, s).unwrap())
        .collect();
    config
}
fn average_values(
    engine: &LiveEngine,
) -> BTreeMap<(String, usize), (usize, GroupId, PathBuf, f64)> {
    build_averages(&engine.store, &AtomicBool::new(false))
        .unwrap()
        .into_iter()
        .map(|a| {
            let g = a.group.unwrap();
            let raw = g.raw(g.params.as_ref().unwrap()).unwrap();
            (
                (a.channel, a.batch),
                (a.count, g.group_id.unwrap(), g.source.unwrap(), raw.1[0]),
            )
        })
        .collect()
}
#[test]
fn three_running_outputs_replace_revisions_without_double_weight_after_resume() {
    let tmp = tempfile::tempdir().unwrap();
    let folder = tmp.path().join("input");
    std::fs::create_dir(&folder).unwrap();
    let config = channel_config(&folder, LiveMerge::Running);
    let mut engine = LiveEngine::open(
        LiveStore::create(&tmp.path().join("cache"), config, BTreeMap::new()).unwrap(),
    )
    .unwrap();
    let directory = engine.store.directory.clone();
    std::fs::write(folder.join("a.xdi"), three_signals(1., 2., 3.)).unwrap();
    let now = Instant::now();
    let records = tick(&mut engine, now);
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].frames.len(), 3);
    assert_eq!(
        records[0]
            .frames
            .iter()
            .filter_map(|f| f.group.group_id.clone())
            .collect::<BTreeSet<_>>()
            .len(),
        3
    );
    let first = average_values(&engine);
    assert_eq!(first.len(), 3);
    std::fs::write(folder.join("b.xdi"), three_signals(3., 4., 5.)).unwrap();
    tick(&mut engine, now + Duration::from_secs(1));
    let second = average_values(&engine);
    for (channel, expected) in [
        ("transmission", 2.),
        ("fluorescence", 3.),
        ("reference", 4.),
    ] {
        let key = (channel.into(), 0);
        assert_eq!(second[&key].0, 2);
        assert_eq!(second[&key].1, first[&key].1);
        assert_ne!(second[&key].2, first[&key].2);
        assert!((second[&key].3 - expected).abs() < 1e-10);
    }
    // Cancellation retains all committed channels and leaves the new source eligible.
    std::fs::write(folder.join("a.xdi"), three_signals(5., 6., 7.)).unwrap();
    assert!(
        engine
            .poll(now + Duration::from_secs(2), 10, &AtomicBool::new(true))
            .unwrap()
            .is_empty()
    );
    assert_eq!(average_values(&engine), second);
    tick(&mut engine, now + Duration::from_secs(3));
    let revised = average_values(&engine);
    assert_eq!(revised.len(), 3);
    for (channel, expected) in [
        ("transmission", 4.),
        ("fluorescence", 5.),
        ("reference", 6.),
    ] {
        let key = (channel.into(), 0);
        assert_eq!(revised[&key].0, 2);
        assert_eq!(revised[&key].1, first[&key].1);
        assert!((revised[&key].3 - expected).abs() < 1e-10);
    }
    assert_eq!(engine.store.records().unwrap().len(), 3);
    drop(engine);
    let mut restored = LiveEngine::open(LiveStore::open(&directory).unwrap()).unwrap();
    assert!(tick(&mut restored, Instant::now()).is_empty());
    assert_eq!(average_values(&restored), revised);
    assert_eq!(restored.store.records().unwrap().len(), 3);
}
#[test]
fn fixed_sets_keep_all_channels_together_and_do_not_renumber_on_rewrite() {
    let tmp = tempfile::tempdir().unwrap();
    let folder = tmp.path().join("input");
    std::fs::create_dir(&folder).unwrap();
    let config = channel_config(&folder, LiveMerge::Batches { scans: 2 });
    let mut engine = LiveEngine::open(
        LiveStore::create(&tmp.path().join("cache"), config, BTreeMap::new()).unwrap(),
    )
    .unwrap();
    let now = Instant::now();
    for (i, name) in ["z.xdi", "a.xdi", "b.xdi"].into_iter().enumerate() {
        std::fs::write(folder.join(name), three_signals(i as f64 + 1., 2., 3.)).unwrap();
        tick(&mut engine, now + Duration::from_secs(i as u64));
    }
    let before = average_values(&engine);
    assert_eq!(before.len(), 6);
    assert_eq!(before[&("transmission".into(), 0)].0, 2);
    assert!((before[&("transmission".into(), 0)].3 - 1.5).abs() < 1e-10);
    assert_eq!(before[&("transmission".into(), 1)].0, 1);
    std::fs::write(folder.join("z.xdi"), three_signals(9., 2., 3.)).unwrap();
    tick(&mut engine, now + Duration::from_secs(4));
    let after = average_values(&engine);
    assert_eq!(after.len(), 6);
    for (key, value) in &after {
        assert_eq!(value.1, before[key].1);
    }
    assert!((after[&("transmission".into(), 0)].3 - 5.5).abs() < 1e-10);
    assert_eq!(
        after[&("transmission".into(), 1)],
        before[&("transmission".into(), 1)]
    );
}
#[test]
fn bad_selected_channel_cannot_publish_a_partial_detector_set() {
    let tmp = tempfile::tempdir().unwrap();
    let folder = tmp.path().join("input");
    std::fs::create_dir(&folder).unwrap();
    let config = channel_config(&folder, LiveMerge::Running);
    let mut engine = LiveEngine::open(
        LiveStore::create(&tmp.path().join("cache"), config, BTreeMap::new()).unwrap(),
    )
    .unwrap();
    let good = three_signals(1., 2., 3.);
    std::fs::write(
        folder.join("a.xdi"),
        good.replace("# Column.5: ir counts", "# Column.5: unknown counts"),
    )
    .unwrap();
    std::fs::write(folder.join("b.xdi"), good).unwrap();
    let records = tick(&mut engine, Instant::now());
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].frames.len(), 3);
    assert_eq!(engine.progress().review.len(), 1);
    assert_eq!(average_values(&engine).len(), 3);
}
#[test]
fn old_sessions_default_to_individual_scans_and_batches_validate() {
    let mut value = serde_json::to_value(config(Path::new("/tmp"))).unwrap();
    value.as_object_mut().unwrap().remove("merge");
    let mut restored: LiveConfig = serde_json::from_value(value).unwrap();
    assert_eq!(restored.merge, LiveMerge::Individual);
    restored.merge = LiveMerge::Batches { scans: 0 };
    assert!(restored.validate().is_err());
    restored.merge = LiveMerge::Batches { scans: 2 };
    assert!(restored.validate().is_ok());
}

#[test]
fn preview_and_average_apply_frozen_energy_offset_once() {
    let tmp = tempfile::tempdir().unwrap();
    let folder = tmp.path().join("input");
    std::fs::create_dir(&folder).unwrap();
    let source = spectrum(1.);
    let document = rexafs::io::parse_measurement(source.as_bytes()).unwrap();
    let mut config = config(&folder);
    config.settings.energy_offset_ev = 5.;
    config.merge = LiveMerge::Running;
    config.definition.measurement = MetricMeasurement::mean(7105.0..=7107.0).raw_mu().absolute();
    let preview = prepare_preview(
        &document.scans[0],
        &config.layouts[0].mapping,
        &config.settings,
        crate::params::RequiredStage::Raw,
    )
    .unwrap();
    assert_eq!(
        preview
            .measure(&config.definition.measurement)
            .unwrap()
            .value,
        2.
    );
    std::fs::write(folder.join("scan.xdi"), source).unwrap();
    let mut engine = LiveEngine::open(
        LiveStore::create(&tmp.path().join("cache"), config, BTreeMap::new()).unwrap(),
    )
    .unwrap();
    let records = tick(&mut engine, Instant::now());
    assert_eq!(records[0].frames[0].row.result.as_ref().unwrap().value, 2.);
    let averages = build_averages(&engine.store, &AtomicBool::new(false)).unwrap();
    let group = averages[0].group.as_ref().unwrap();
    assert_eq!(
        group.raw(group.params.as_ref().unwrap()).unwrap().0,
        vec![7105., 7106., 7107.]
    );
    let source = &records[0].frames[0].group;
    assert_eq!(
        group.operation.as_ref().unwrap().inputs[0].fingerprint,
        source.fingerprint(source.params.as_ref().unwrap())
    );
    let params = group.params.as_ref().unwrap();
    let mut revised = group.clone();
    revised.source = Some(PathBuf::from("different-immutable-average.dat"));
    assert_ne!(group.fingerprint(params), revised.fingerprint(params));
}

#[test]
fn cu_exafs_outputs_freeze_paths_recover_and_embed_without_refitting() {
    use crate::fitting::{FitPathSpec, FitRanges, FitVarSpec};
    use crate::project::{self, DataStorage, ProjectFile};
    let tmp = tempfile::tempdir().unwrap();
    let folder = tmp.path().join("input");
    std::fs::create_dir(&folder).unwrap();
    let core = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../rexafs/tests");
    let bytes =
        std::fs::read(core.join("fixtures/analysis/cu-mixtures/standards/cufoil_abs.xdi")).unwrap();
    let document = rexafs::io::parse_measurement(&bytes).unwrap();
    let scan = &document.scans[0];
    let mut config = config(&folder);
    config.schema = 3;
    config.merge = LiveMerge::Running;
    config.layouts = vec![ScanLayout::capture_channel(&document, scan, &scan.signals[0]).unwrap()];
    config.definition.measurement = MetricMeasurement::mean(-20.0..=30.0).flat();
    let path = tmp.path().join("original-feff.dat");
    std::fs::copy(core.join("testfiles/feffcu01.dat"), &path).unwrap();
    let mut spec = FitPathSpec::blank(path.clone());
    spec.enabled = true;
    spec.s02 = "amp".into();
    spec.e0 = "e0".into();
    spec.deltar = "dr".into();
    spec.sigma2 = "ss".into();
    config.exafs = Some(ExafsRecipe {
        name: "Cu first-shell test".into(),
        paths: vec![(spec, std::fs::read(&path).unwrap())],
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
            vary: true,
            min,
            max,
            expr: None,
        })
        .collect(),
        ranges: FitRanges::default(),
    });
    std::fs::remove_file(path).unwrap();
    let cache = tmp.path().join("cache");
    let mut engine =
        LiveEngine::open(LiveStore::create(&cache, config, BTreeMap::new()).unwrap()).unwrap();
    std::fs::write(folder.join("one.xdi"), &bytes).unwrap();
    let now = Instant::now();
    assert_eq!(tick(&mut engine, now).len(), 1);
    let cancel = AtomicBool::new(false);
    let first = build_averages(&engine.store, &cancel).unwrap();
    assert!(
        build_exafs(&engine.store, &first, false, &cancel)
            .unwrap()
            .is_empty()
    );
    let rows = build_exafs(&engine.store, &first, true, &cancel).unwrap();
    assert_eq!(rows.len(), 1);
    let result = read_exafs(&rows[0]).unwrap().result.unwrap();
    assert!(result.r_factor.is_finite() && result.r_factor < 0.1);
    let first_modified = std::fs::metadata(&rows[0].artifact)
        .unwrap()
        .modified()
        .unwrap();
    let recovered = build_exafs(&engine.store, &first, false, &cancel).unwrap();
    assert_eq!(recovered[0].artifact, rows[0].artifact);
    assert_eq!(
        std::fs::metadata(&rows[0].artifact)
            .unwrap()
            .modified()
            .unwrap(),
        first_modified
    );
    std::fs::write(folder.join("two.xdi"), &bytes).unwrap();
    tick(&mut engine, now + Duration::from_secs(1));
    let second = build_averages(&engine.store, &cancel).unwrap();
    let updated = build_exafs(&engine.store, &second, true, &cancel).unwrap();
    assert_eq!(updated[0].group, rows[0].group);
    assert_ne!(updated[0].key, rows[0].key);
    let again = read_exafs(&updated[0]).unwrap().result.unwrap();
    assert!((again.r_factor - result.r_factor).abs() < 1e-8);
    let mut session = engine.store.session();
    session.exafs = rows.into_iter().chain(updated).collect();
    let group = second.into_iter().next().unwrap().group.unwrap();
    let mut project = ProjectFile {
        spectrum_file: group.source.clone(),
        derived: vec![group],
        ..Default::default()
    };
    project.series_measurements.live_sessions.push(session);
    let portable = tmp.path().join("portable.rxs");
    project::save_with_storage(&portable, &project, DataStorage::Embedded).unwrap();
    let directory = engine.store.directory.clone();
    drop(engine);
    #[cfg(unix)]
    {
        let alias = tmp.path().join("recovery-alias");
        std::os::unix::fs::symlink(&cache, &alias).unwrap();
        let store = LiveStore::open(&alias.join(directory.file_name().unwrap())).unwrap();
        assert_eq!(store.directory, directory);
        let averages = build_averages(&store, &cancel).unwrap();
        let recovered = build_exafs(&store, &averages, false, &cancel).unwrap();
        assert_eq!(recovered.len(), 1);
        let expected = &project.series_measurements.live_sessions[0].exafs[1];
        assert_eq!(recovered[0].key, expected.key);
        assert_eq!(recovered[0].group, expected.group);
    }
    std::fs::remove_dir_all(cache).unwrap();
    std::fs::remove_dir_all(folder).unwrap();
    let restored =
        project::load_with_cache_root(&portable, || Ok(tmp.path().join("restore"))).unwrap();
    let session = &restored.series_measurements.live_sessions[0];
    assert_eq!(session.exafs.len(), 2);
    for row in &session.exafs {
        assert!(read_exafs(row).unwrap().result.is_ok());
    }
    assert_eq!(
        Some(&session.exafs[1].source),
        restored.derived[0].source.as_ref()
    );
}
