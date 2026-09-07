//! Append-only import. Detect channels from bounded prefixes; retain lazy sources.
use super::*;
use crate::catalog::FileMeta;
use crate::params::{ImportConfig, detect_import};
use futures::{SinkExt, channel::mpsc};

struct ImportFile {
    meta: FileMeta,
    reference: bool,
}
enum ImportEvent {
    Batch(Vec<ImportFile>),
    Error(String),
    Done,
}

fn start_import(
    paths: Vec<PathBuf>,
    import: ImportConfig,
    detect_channels: bool,
) -> mpsc::Receiver<ImportEvent> {
    let (mut tx, rx) = mpsc::channel(2);
    std::thread::spawn(move || {
        let send = |tx: &mut mpsc::Sender<ImportEvent>, event| {
            futures::executor::block_on(tx.send(event)).is_ok()
        };
        let mut batch = Vec::new();
        for path in paths {
            let root = match path.canonicalize() {
                Ok(root) => root,
                Err(e) => {
                    if !send(
                        &mut tx,
                        ImportEvent::Error(format!("{}: {e}", path.display())),
                    ) {
                        return;
                    }
                    continue;
                }
            };
            let explicit_file = root.is_file();
            for entry in walkdir::WalkDir::new(&root)
                .follow_links(false)
                .sort_by_file_name()
            {
                let entry = match entry {
                    Ok(entry) => entry,
                    Err(e) => {
                        if !send(&mut tx, ImportEvent::Error(e.to_string())) {
                            return;
                        }
                        continue;
                    }
                };
                if !entry.file_type().is_file()
                    || (!explicit_file
                        && !entry.path().extension().is_some_and(|ext| {
                            crate::catalog::SPECTRUM_EXTENSIONS
                                .iter()
                                .any(|e| ext.eq_ignore_ascii_case(e))
                        }))
                {
                    continue;
                }
                let reference = if detect_channels {
                    match detect_import(entry.path(), &import) {
                        Ok(Some(preview)) => {
                            if let Some(error) = &preview.mapping_error
                                && !send(&mut tx, ImportEvent::Error(error.clone()))
                            {
                                return;
                            }
                            preview.resolved.mode != DetectionMode::Reference
                                && preview
                                    .available_channels()
                                    .contains(&DetectionMode::Reference)
                        }
                        Ok(None) => false, // The bounded prefix ended before data.
                        Err(error) => {
                            if !send(&mut tx, ImportEvent::Error(error)) {
                                return;
                            }
                            // Retain sources that need manual mapping.
                            false
                        }
                    }
                } else {
                    // Restore stays lazy. load_spectrum diagnoses each source with
                    // its effective mapping after resolve_pending_overrides.
                    false
                };
                let meta = FileMeta {
                    dir: Arc::from(entry.path().parent().unwrap().to_string_lossy().as_ref()),
                    name: entry
                        .file_name()
                        .to_string_lossy()
                        .into_owned()
                        .into_boxed_str(),
                    size: entry.metadata().map(|m| m.len()).unwrap_or(0),
                };
                // No prefix diagnostics are stored as source totals. The inspector
                // and load_spectrum diagnose the full source with its mapping.
                batch.push(ImportFile { meta, reference });
                if batch.len() == 128
                    && !send(&mut tx, ImportEvent::Batch(std::mem::take(&mut batch)))
                {
                    return;
                }
                if tx.is_closed() {
                    return;
                }
            }
        }
        if !batch.is_empty() && !send(&mut tx, ImportEvent::Batch(batch)) {
            return;
        }
        send(&mut tx, ImportEvent::Done);
    });
    rx
}

impl StudioApp {
    pub(super) fn set_chi_standard(
        &mut self,
        standard: Option<crate::params::ChiStandard>,
        cx: &mut Context<Self>,
    ) {
        let target = self.override_target();
        if target.is_some_and(|ix| self.frozen.contains(&ix)) {
            self.status = "This group is frozen — thaw it to edit its parameters.".into();
            cx.notify();
            return;
        }
        let before = self.ui_params().clone();
        self.edit_params().bkg_standard = standard;
        self.record_param_edit(
            target,
            None,
            before,
            self.ui_params().clone(),
            "Set AUTOBK standard χ(k)".into(),
        );
        self.schedule_recompute(cx);
        cx.notify();
    }

    pub(super) fn choose_chi_standard(&mut self, cx: &mut Context<Self>) {
        let generation = self.project_generation;
        let path = self.current_path.clone();
        let group = self.active_group_id();
        let fingerprint = self.ui_params().fingerprint();
        let rx = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: None,
        });
        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(paths))) = rx.await && let Some(file) = paths.into_iter().next() {
                let result = cx.background_executor().spawn(async move {
                    crate::params::ChiStandard::load(&file)
                }).await;
                this.update(cx, |app, cx| {
                    if app.project_generation != generation || app.current_path != path
                        || app.active_group_id() != group || app.ui_params().fingerprint() != fingerprint {
                        app.status = "Standard not applied because the active group or its settings changed. Load it again for the intended group.".into();
                        cx.notify();
                        return;
                    }
                    match result {
                        Ok(standard) => app.set_chi_standard(Some(standard), cx),
                        Err(e) => { app.status = format!("Standard χ(k): {e}").into(); cx.notify(); }
                    }
                }).ok();
            }
        }).detach();
    }

    pub(super) fn append_import(
        &mut self,
        mut paths: Vec<PathBuf>,
        restore: bool,
        cx: &mut Context<Self>,
    ) {
        if self.catalog.scanning {
            self.status = "Wait for the current import to finish before adding more files.".into();
            cx.notify();
            return;
        }
        if !restore && self.selected.is_none() && !self.current_path.as_os_str().is_empty() {
            paths.insert(0, self.current_path.clone());
            self.pending_project_spectrum = Some(self.current_path.clone());
        }
        self.source_dir = None;
        self.catalog_index_path = None;
        self.catalog.scanning = true;
        let generation = self.catalog_gen;
        let params = self.params.clone();
        let existing_references: BTreeSet<_> = self
            .derived
            .iter()
            .filter(|d| {
                d.params
                    .as_ref()
                    .is_some_and(|p| p.import.mode == DetectionMode::Reference)
            })
            .filter_map(|d| d.source.clone())
            .collect();
        let mut rx = start_import(paths, params.import.clone(), !restore);
        self.status = "Importing files and detecting reference channels…".into();
        cx.spawn(async move |this, cx| {
            let (mut added, mut channels, mut notices) = (0, 0, 0);
            while let Some(event) = rx.next().await {
                let done = matches!(event, ImportEvent::Done);
                let current = this.update(cx, |app, cx| {
                    if app.catalog_gen != generation { return false; }
                    match event {
                        ImportEvent::Batch(batch) => {
                            for file in batch {
                                let path = PathBuf::from(file.meta.dir.as_ref()).join(file.meta.name.as_ref());
                                if app.catalog.find_by_canonical_path(&path).is_some() { continue; }
                                app.catalog.extend(vec![file.meta]);
                                added += 1;
                                if file.reference && !existing_references.contains(&path) {
                                    let mut reference_params = params.clone();
                                    reference_params.import.mode = DetectionMode::Reference;
                                    // A reference edge must resolve independently of sample overrides.
                                    reference_params.e0 = None;
                                    reference_params.edge_step = None;
                                    reference_params.bkg_ek0 = None;
                                    reference_params.bkg_standard = None;
                                    let group = DerivedSpectrum {
                                        id: app.next_group_id(),
                                        label: path.file_name().unwrap_or_default().to_string_lossy().into_owned(),
                                        source: Some(path), params: Some(reference_params), ..Default::default()
                                    };
                                    app.derived.push(group);
                                    channels += 1;
                                }
                            }
                            if app.selected.is_none() && app.pending_project_spectrum.is_none() && app.pending_derived.is_none() && !app.catalog.is_empty() { app.select_entry(0, cx); }
                            app.status = format!("Importing · {added} files · {channels} reference channels").into();
                        }
                        ImportEvent::Error(e) => { notices += 1; app.record_job_error("import", e); }
                        ImportEvent::Done => {
                            app.catalog.scanning = false;
                            app.resolve_pending_overrides(cx);
                            app.restore_project_selection(cx);
                            if !app.filter_text.is_empty() { app.apply_filter(cx); }
                            app.status = format!("Imported {added} files + {channels} reference channels · {notices} notices").into();
                            app.record(app.status.to_string(), None);
                        }
                    }
                    cx.notify();
                    true
                }).unwrap_or(false);
                if !current || done { break; }
            }
        }).detach();
        cx.notify();
    }

    pub(super) fn add_import_channel(&mut self, mode: DetectionMode, cx: &mut Context<Self>) {
        let path = self.current_path.clone();
        if path.as_os_str().is_empty() {
            return;
        }
        let mut params = self.ui_params().clone();
        params.import.mode = mode;
        params.e0 = None;
        params.edge_step = None;
        params.bkg_ek0 = None;
        params.bkg_standard = None;
        if let Some(i) = self.derived.iter().position(|d| {
            d.source.as_ref() == Some(&path)
                && d.params.as_ref().is_some_and(|p| p.import == params.import)
        }) {
            self.select_entry(DERIVED_BASE + i, cx);
            return;
        }
        let group = DerivedSpectrum {
            id: self.next_group_id(),
            label: path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
            source: Some(path),
            params: Some(params),
            ..Default::default()
        };
        let index = self.derived.len();
        self.record(
            format!("Add {} channel", mode.label()),
            Some(shell::journal::UndoOp::DerivedAdd {
                index,
                spectrum: group.clone(),
            }),
        );
        self.derived.push(group);
        self.select_entry(DERIVED_BASE + index, cx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn import_thread_uses_bounded_detection_and_defers_full_source_diagnostics() {
        let path = std::env::temp_dir().join("rexafs-intake-bounded.dat");
        let mut bytes = b"# energy i0 it ir\n100 10 0 1\n101 bad 0 1\n".to_vec();
        // The prefix has no finite transmission points. Later rows may be
        // valid; intake must not emit a full-signal error or row warnings.
        for _ in 0..60_000 {
            bytes.extend_from_slice(b"102 10 0 1\n");
        }
        bytes.resize(1_000_000, b' ');
        bytes.extend_from_slice(b"\n103 malformed 0 1\n");
        bytes.push(0xff); // A full read_to_string would fail.
        std::fs::write(&path, bytes).unwrap();
        for import in [
            ImportConfig::default(),
            ImportConfig {
                i0_col: Some(99),
                ..Default::default()
            },
        ] {
            let events = futures::executor::block_on(
                start_import(vec![path.clone()], import.clone(), true).collect::<Vec<_>>(),
            );
            let mut files_seen = 0;
            let mut errors = Vec::new();
            for event in &events {
                match event {
                    ImportEvent::Batch(files) => {
                        for ImportFile { meta, reference } in files {
                            files_seen += 1;
                            assert_eq!(
                                PathBuf::from(meta.dir.as_ref()).join(meta.name.as_ref()),
                                path.canonicalize().unwrap()
                            );
                            assert!(*reference);
                        }
                    }
                    ImportEvent::Error(error) => errors.push(error),
                    ImportEvent::Done => {}
                }
            }
            assert_eq!(files_seen, 1);
            assert!(matches!(events.last(), Some(ImportEvent::Done)));
            if import.i0_col.is_some() {
                assert_eq!(errors.len(), 1);
                assert!(errors[0].contains("I0 column 99 out of range"));
            } else {
                assert!(errors.is_empty(), "{errors:?}");
            }
        }
        assert!(preview_import(&path, &ImportConfig::default()).is_err());
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn import_thread_retains_long_header_sources_without_errors_or_references() {
        for (extension, header, tail) in [
            ("dat", "", "# energy i0 it ir\n100 10 5 2\n101 10 4 2\n"),
            (
                "xdi",
                "# XDI/1.0\n# Column.1: energy eV\n# Column.2: i0\n# Column.3: it\n# Column.4: ir\n# ///\n",
                "# ---\n100 10 5 2\n101 10 4 2\n",
            ),
        ] {
            let path = std::env::temp_dir().join(format!("rexafs-intake-long-header.{extension}"));
            std::fs::write(
                &path,
                format!("{header}{}{tail}", "# metadata\n".repeat(7000)),
            )
            .unwrap();
            let events = futures::executor::block_on(
                start_import(vec![path.clone()], ImportConfig::default(), true).collect::<Vec<_>>(),
            );
            let mut files_seen = 0;
            for event in &events {
                match event {
                    ImportEvent::Batch(files) => {
                        files_seen += files.len();
                        assert!(files.iter().all(|file| !file.reference));
                    }
                    ImportEvent::Error(error) => {
                        panic!("incomplete prefix caused an error: {error}")
                    }
                    ImportEvent::Done => {}
                }
            }
            assert_eq!(files_seen, 1);
            assert!(matches!(events.last(), Some(ImportEvent::Done)));
            std::fs::remove_file(path).unwrap();
        }
    }

    #[test]
    fn import_thread_reports_invalid_utf8_header_and_retains_source() {
        let path = std::env::temp_dir().join("rexafs-intake-latin1.dat");
        std::fs::write(
            &path,
            b"# units: \xb5\n# energy i0 it ir\n100 10 5 2\n101 10 4 2\n",
        )
        .unwrap();
        let events = futures::executor::block_on(
            start_import(vec![path.clone()], ImportConfig::default(), true).collect::<Vec<_>>(),
        );
        let (mut sources, mut errors) = (0, 0);
        for event in &events {
            match event {
                ImportEvent::Batch(files) => {
                    sources += files.len();
                    assert!(files.iter().all(|file| !file.reference));
                }
                ImportEvent::Error(error) => {
                    errors += 1;
                    assert!(error.contains("UTF-8"));
                }
                ImportEvent::Done => {}
            }
        }
        assert_eq!((sources, errors), (1, 1));
        assert!(matches!(events.last(), Some(ImportEvent::Done)));
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn full_preview_warnings_keep_source_and_severity_without_duplicate_records() {
        let path = std::env::temp_dir().join("rexafs-import-diagnostics.dat");
        std::fs::write(
            &path,
            "# energy i0 it\n100 10 5\n101 bad 5\n102 10\n103 10 5 9\n104 10 0\n105 10 2\n",
        )
        .unwrap();
        let preview = preview_import(&path, &ImportConfig::default()).unwrap();
        let mut problems = Vec::new();
        let warnings = preview.diagnostics.warnings();
        for message in &warnings {
            assert!(message.contains("example lines:"));
            let warning = JobError::warning(&path, message.clone());
            assert_eq!(warning.severity, ProblemSeverity::Warning);
            assert!(warning.label.contains("rexafs-import-diagnostics.dat"));
            push_problem(&mut problems, warning.clone());
            push_problem(&mut problems, warning);
        }
        assert_eq!(warnings.len(), 4);
        assert_eq!(problem_counts(&problems), (0, 4));
        let error = JobError {
            severity: ProblemSeverity::Error,
            label: "error".into(),
            message: "failure".into(),
        };
        for _ in 0..JOB_ERROR_CAPACITY {
            push_problem(&mut problems, error.clone());
        }
        assert_eq!(problem_counts(&problems), (JOB_ERROR_CAPACITY, 0));
        let warning = JobError::warning(&path, "new warning".into());
        let full_errors = problems.clone();
        push_problem(&mut problems, warning.clone());
        assert_eq!(problems, full_errors);
        // Even an error older than every warning must survive a new warning.
        problems[1] = warning.clone();
        let newer_warning = JobError::warning(&path, "newer warning".into());
        push_problem(&mut problems, newer_warning.clone());
        assert_eq!(problems[0], error);
        assert!(!problems.contains(&warning));
        assert_eq!(problems.last(), Some(&newer_warning));
        assert_eq!(problem_counts(&problems), (JOB_ERROR_CAPACITY - 1, 1));
        push_problem(&mut problems, error.clone());
        assert_eq!(problem_counts(&problems), (JOB_ERROR_CAPACITY, 0));
        let newest_error = JobError {
            message: "new failure".into(),
            ..error
        };
        push_problem(&mut problems, newest_error.clone());
        assert_eq!(problems.len(), JOB_ERROR_CAPACITY);
        assert_eq!(problems.last(), Some(&newest_error));
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn restore_defers_diagnostics_until_saved_mappings_are_available() {
        use crate::project::{ParamOverride, ProjectFile, load, save};
        let dir = std::env::temp_dir().join("rexafs-restore-diagnostics");
        std::fs::create_dir_all(&dir).unwrap();
        let mu_path = dir.join("mu.dat");
        let roi_path = dir.join("roi.dat");
        let unreadable = dir.join("invalid-utf8.dat");
        std::fs::write(&mu_path, "100 1\n101 2\n").unwrap();
        std::fs::write(&roi_path, "100 1 0 10\n101 NaN 0 10\n102 3 0 10\n").unwrap();
        std::fs::write(&unreadable, [0xff]).unwrap();
        let overrides = [
            (
                mu_path,
                ImportConfig {
                    mode: DetectionMode::MuColumn,
                    mu_col: Some(1),
                    ..Default::default()
                },
            ),
            (
                roi_path,
                ImportConfig {
                    mode: DetectionMode::Fluorescence,
                    i0_col: Some(3),
                    fluor_cols: Some(vec![1]),
                    ..Default::default()
                },
            ),
        ]
        .into_iter()
        .map(|(path, import)| ParamOverride {
            path,
            params: PipelineParams {
                import,
                ..Default::default()
            },
        })
        .collect();
        let project_path = dir.join("restore.rxs");
        save(
            &project_path,
            &ProjectFile {
                overrides,
                ..Default::default()
            },
        )
        .unwrap();
        let restored = load(&project_path).unwrap();
        let mut paths: Vec<_> = restored.overrides.iter().map(|o| o.path.clone()).collect();
        paths.push(unreadable);
        let events = futures::executor::block_on(
            start_import(paths, restored.params.import.clone(), false).collect::<Vec<_>>(),
        );
        let mut sources = 0;
        for event in &events {
            match event {
                ImportEvent::Batch(files) => {
                    sources += files.len();
                    assert!(files.iter().all(|f| !f.reference));
                }
                ImportEvent::Error(error) => panic!("restore previewed a source: {error}"),
                ImportEvent::Done => {}
            }
        }
        assert_eq!(sources, 3);
        assert!(matches!(events.last(), Some(ImportEvent::Done)));
        for (index, saved) in restored.overrides.iter().enumerate() {
            assert!(crate::params::load_mu(&saved.path, &restored.params.import).is_err());
            let raw = load_group_raw_with_diagnostics(&saved.path, &saved.params, None).unwrap();
            assert_eq!(raw.diagnostics.valid_points, 2);
            assert_eq!(raw.diagnostics.excluded_signal_points.count, index);
            if index == 1 {
                assert_eq!(raw.diagnostics.excluded_signal_points.examples, vec![2]);
                assert_eq!(raw.mu, vec![0.1, 0.3]);
            } else {
                assert!(raw.diagnostics.warnings().is_empty());
                assert_eq!(raw.mu, vec![1., 2.]);
            }
        }
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn raw_cache_retains_arrays_and_diagnostics() {
        let path = std::env::temp_dir().join("rexafs-cache-diagnostics.dat");
        std::fs::write(&path, "# energy i0 it\n100 10 5\n101 bad 5\n102 10 2\n").unwrap();
        let raw = Arc::new(
            crate::params::load_raw_with_diagnostics(&path, &PipelineParams::default()).unwrap(),
        );
        let mut cache: LruCache<(usize, u64), RawArrays> =
            LruCache::new(NonZeroUsize::new(2).unwrap());
        cache.put((0, 42), raw.clone());
        let cached = cache.get(&(0, 42)).unwrap().clone();
        assert_eq!(cached, raw);
        assert_eq!(cached.diagnostics.malformed_rows.examples, vec![3]);
        assert_eq!(cached.diagnostics.summary(), "2 points · 1 rows skipped");
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn qas_import_detects_reference_but_project_restore_preserves_saved_groups() {
        let source =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../rexafs/tests/testfiles/Ru_QAS.dat");
        for detect in [true, false] {
            let events = futures::executor::block_on(
                start_import(vec![source.clone()], ImportConfig::default(), detect)
                    .collect::<Vec<_>>(),
            );
            let files: Vec<_> = events
                .iter()
                .filter_map(|e| {
                    if let ImportEvent::Batch(files) = e {
                        Some(files.as_slice())
                    } else {
                        None
                    }
                })
                .flatten()
                .collect();
            assert_eq!(files.len(), 1);
            assert_eq!(files[0].reference, detect);
            assert!(matches!(events.last(), Some(ImportEvent::Done)));
            assert!(!events.iter().any(|e| matches!(e, ImportEvent::Error(_))));
        }
    }
}
