use super::*;

const DATA: &[u8] = b"# energy mu\n7100 0.1\n7101 0.2\n7102 0.3\n";
fn source(dir: &tempfile::TempDir, name: &str) -> PathBuf {
    let path = dir.path().join(name);
    fs::write(&path, DATA).unwrap();
    path
}
fn ready(observation: Observation) -> Snapshot {
    match observation {
        Observation::Ready(value) => value,
        Observation::NeedsReview(e) | Observation::Unavailable(e) => panic!("{e}"),
        _ => panic!("Expected a completed snapshot"),
    }
}
fn quiet_ready(tracker: &mut CompletionTracker, path: &Path, now: Instant) -> Snapshot {
    assert!(matches!(
        tracker.observe(path, now),
        Observation::Waiting {
            observed: 1,
            required: 3
        }
    ));
    assert!(matches!(
        tracker.observe(path, now + Duration::from_secs(1)),
        Observation::Waiting {
            observed: 2,
            required: 3
        }
    ));
    ready(tracker.observe(path, now + Duration::from_secs(2)))
}
fn marker(path: &Path, bytes: &[u8]) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(".ready");
    let path = PathBuf::from(name);
    fs::write(
        &path,
        serde_json::to_vec(&DigestMarker {
            bytes: bytes.len() as u64,
            sha256: hash(bytes),
        })
        .unwrap(),
    )
    .unwrap();
    path
}

#[test]
fn quiet_completion_counts_spaced_observations_and_parses_the_same_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let path = source(&dir, "scan 日本語.qd");
    let now = Instant::now();
    let mut tracker = CompletionTracker::new(Default::default()).unwrap();
    assert!(matches!(
        tracker.observe(&path, now),
        Observation::Waiting { observed: 1, .. }
    ));
    for _ in 0..10 {
        assert!(matches!(
            tracker.observe(&path, now + Duration::from_millis(999)),
            Observation::Waiting { observed: 1, .. }
        ));
    }
    assert!(matches!(
        tracker.observe(&path, now + Duration::from_secs(1)),
        Observation::Waiting { observed: 2, .. }
    ));
    let snapshot = ready(tracker.observe(&path, now + Duration::from_secs(2)));
    assert_eq!(&*snapshot.bytes, DATA);
    assert_eq!(snapshot.revision, hash(DATA));
    assert_eq!(snapshot.measurement.scans.len(), 1);
    assert_eq!(
        snapshot.completion,
        CompletionEvidence::InferredQuiet {
            checks: 3,
            interval_ms: 1000
        }
    );
    // Completion and computation are separate: no acknowledge means retry.
    assert!(matches!(
        tracker.observe(&path, now + Duration::from_secs(3)),
        Observation::Ready(_)
    ));
    tracker.acknowledge(&snapshot).unwrap();
    tracker.acknowledge(&snapshot).unwrap(); // A repeated durable acknowledgement is idempotent.
    assert!(matches!(
        tracker.observe(&path, now + Duration::from_secs(4)),
        Observation::AlreadyAccepted { .. }
    ));
}

#[test]
fn partial_writes_reset_quiet_counts_and_malformed_rows_need_review() {
    let dir = tempfile::tempdir().unwrap();
    let path = source(&dir, "scan.dat");
    let now = Instant::now();
    let mut tracker = CompletionTracker::new(Default::default()).unwrap();
    tracker.observe(&path, now);
    fs::write(&path, b"# energy mu\n7100 0.1\n7101 bad\n7102 0.3\n").unwrap();
    assert!(matches!(
        tracker.observe(&path, now + Duration::from_secs(1)),
        Observation::Waiting { observed: 1, .. }
    ));
    tracker.observe(&path, now + Duration::from_secs(2));
    assert!(matches!(
        tracker.observe(&path, now + Duration::from_secs(3)),
        Observation::NeedsReview(_)
    ));
    fs::write(&path, DATA).unwrap();
    quiet_ready(&mut tracker, &path, now + Duration::from_secs(4));
}

#[test]
fn producer_digest_requires_the_complete_matching_source_and_valid_structure() {
    let dir = tempfile::tempdir().unwrap();
    let path = source(&dir, "scan.dat");
    let now = Instant::now();
    let mut tracker = CompletionTracker::new(CompletionPolicy::DigestMarker {
        suffix: ".ready".into(),
    })
    .unwrap();
    assert!(matches!(
        tracker.observe(&path, now),
        Observation::Waiting {
            observed: 0,
            required: 1
        }
    ));
    let marker_path = marker(&path, b"wrong source");
    assert!(matches!(
        tracker.observe(&path, now),
        Observation::NeedsReview(_)
    ));
    marker(&path, DATA);
    let snapshot = ready(tracker.observe(&path, now));
    assert_eq!(
        snapshot.completion,
        CompletionEvidence::ProducerDigest {
            marker: marker_path.clone()
        }
    );
    tracker.acknowledge(&snapshot).unwrap();
    let damaged = b"# energy mu\n7100 bad\n7101 0.2\n7102 0.3\n";
    fs::write(&path, damaged).unwrap();
    marker(&path, damaged);
    assert!(matches!(
        tracker.observe(&path, now),
        Observation::NeedsReview(_)
    ));
    fs::write(&marker_path, b"not json").unwrap();
    assert!(matches!(
        tracker.observe(&path, now),
        Observation::NeedsReview(_)
    ));
}

#[test]
fn changed_during_capture_never_publishes_mixed_or_replaced_input() {
    let dir = tempfile::tempdir().unwrap();
    let path = source(&dir, "scan.dat");
    let before = Fingerprint::at(&path).unwrap();
    let result = capture(&path, &before, &CompletionPolicy::default(), || {
        fs::write(&path, b"# energy mu\n7100 1\n7101 2\n7102 3\n7103 4\n").unwrap();
    });
    assert!(result.err().unwrap().contains("changed during"));
    // The source bytes retained by an earlier accepted snapshot never change.
    let mut tracker = CompletionTracker::new(Default::default()).unwrap();
    let snapshot = quiet_ready(&mut tracker, &path, Instant::now());
    let original = snapshot.bytes.clone();
    fs::remove_file(&path).unwrap();
    fs::write(&path, DATA).unwrap();
    assert_eq!(snapshot.bytes, original);
}

#[test]
fn distinct_paths_same_bytes_and_reused_names_keep_their_identities() {
    let dir = tempfile::tempdir().unwrap();
    let a = source(&dir, "a.dat");
    let b = source(&dir, "b.dat");
    let now = Instant::now();
    let mut tracker = CompletionTracker::new(Default::default()).unwrap();
    let first = quiet_ready(&mut tracker, &a, now);
    tracker.acknowledge(&first).unwrap();
    let second = quiet_ready(&mut tracker, &b, now);
    tracker.acknowledge(&second).unwrap();
    assert_eq!(first.revision, second.revision);
    assert_ne!(first.source, second.source);
    let next = dir.path().join("producer.partial");
    fs::write(&next, b"# energy mu\n7100 1\n7101 2\n7102 3\n7103 4\n").unwrap();
    // Explicit remove/rename works on Windows too. A folder reconciliation must
    // observe the final locator, not assume that every rename means completion.
    fs::remove_file(&a).unwrap();
    fs::rename(next, &a).unwrap();
    let newer = quiet_ready(&mut tracker, &a, now + Duration::from_secs(4));
    assert_eq!(first.source, newer.source);
    assert_ne!(first.revision, newer.revision);
    assert!(tracker.acknowledge(&first).is_err());
    tracker.acknowledge(&newer).unwrap();
}

#[test]
fn durable_revision_restore_skips_only_committed_bytes_and_missing_resets_readiness() {
    let dir = tempfile::tempdir().unwrap();
    let path = source(&dir, "scan.dat");
    let now = Instant::now();
    let mut tracker = CompletionTracker::new(Default::default()).unwrap();
    tracker.restore_accepted(path.clone(), hash(DATA)).unwrap();
    tracker.observe(&path, now);
    tracker.observe(&path, now + Duration::from_secs(1));
    assert!(
        matches!(tracker.observe(&path, now + Duration::from_secs(2)), Observation::AlreadyAccepted { revision } if revision == hash(DATA))
    );
    fs::remove_file(&path).unwrap();
    assert!(matches!(
        tracker.observe(&path, now + Duration::from_secs(3)),
        Observation::Unavailable(_)
    ));
    fs::write(&path, DATA).unwrap();
    assert!(matches!(
        tracker.observe(&path, now + Duration::from_secs(4)),
        Observation::Waiting { observed: 1, .. }
    ));
    assert!(
        tracker
            .restore_accepted(path, "invalid digest".into())
            .is_err()
    );
}

#[test]
fn policies_reject_unbounded_or_ambiguous_readiness_parameters() {
    for policy in [
        CompletionPolicy::Quiet {
            checks: 1,
            interval_ms: 1000,
        },
        CompletionPolicy::Quiet {
            checks: 3,
            interval_ms: 0,
        },
        CompletionPolicy::DigestMarker {
            suffix: "/elsewhere".into(),
        },
    ] {
        assert!(CompletionTracker::new(policy).is_err());
    }
    assert!(CompletionTracker::new(Default::default()).is_ok());
}

#[cfg(unix)]
#[test]
fn watched_file_symlinks_are_not_silently_followed() {
    let dir = tempfile::tempdir().unwrap();
    let path = source(&dir, "source.dat");
    let link = dir.path().join("link.dat");
    std::os::unix::fs::symlink(path, &link).unwrap();
    let mut tracker = CompletionTracker::new(Default::default()).unwrap();
    assert!(matches!(
        tracker.observe(&link, Instant::now()),
        Observation::Unavailable(_)
    ));
}

#[test]
fn one_completed_source_retains_every_scan_and_its_original_columns() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("two-scans.spec");
    let bytes = b"#F file\n#S 1 scan\n#L energy  mu\n7100 1\n7101 2\n#S 2 scan\n#L energy  mu\n7100 3\n7101 4\n";
    fs::write(&path, bytes).unwrap();
    marker(&path, bytes);
    let mut tracker = CompletionTracker::new(CompletionPolicy::DigestMarker {
        suffix: ".ready".into(),
    })
    .unwrap();
    let snapshot = ready(tracker.observe(&path, Instant::now()));
    assert_eq!(&*snapshot.bytes, bytes);
    assert_eq!(snapshot.measurement.scans.len(), 2);
    assert_eq!(snapshot.measurement.scans[0].columns[1].values, [1., 2.]);
    assert_eq!(snapshot.measurement.scans[1].columns[1].values, [3., 4.]);
    assert_ne!(
        snapshot.measurement.scans[0].id,
        snapshot.measurement.scans[1].id
    );
}

#[test]
fn marker_changes_and_oversized_inputs_cannot_be_acknowledged() {
    let dir = tempfile::tempdir().unwrap();
    let path = source(&dir, "scan.dat");
    let marker_path = marker(&path, DATA);
    let policy = CompletionPolicy::DigestMarker {
        suffix: ".ready".into(),
    };
    let before = Fingerprint::at(&path).unwrap();
    let result = capture(&path, &before, &policy, || {
        fs::write(&marker_path, b"changed").unwrap();
    });
    assert!(result.err().unwrap().contains("marker changed"));
    fs::write(&marker_path, vec![b' '; 4097]).unwrap();
    assert!(
        capture(&path, &before, &policy, || {})
            .err()
            .unwrap()
            .contains("4 KiB")
    );
    // A sparse oversized source must fail before allocating or parsing payloads.
    let file = fs::File::create(&path).unwrap();
    file.set_len(MAX_BYTES + 1).unwrap();
    drop(file);
    let before = Fingerprint::at(&path).unwrap();
    assert!(
        capture(&path, &before, &CompletionPolicy::default(), || {})
            .err()
            .unwrap()
            .contains("256 MiB")
    );
}
