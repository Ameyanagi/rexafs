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
        },
        recipe: None,
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
