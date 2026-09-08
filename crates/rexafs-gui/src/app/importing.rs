//! Append-only import. Detect channels from bounded prefixes; retain lazy sources.
use super::import_review::{Approval, Approvals, PendingSource};
use super::import_state::{flush_due, source_changed};
use super::*;
use crate::catalog::FileMeta;
use crate::params::{ImportConfig, detect_import};
use futures::{SinkExt, channel::mpsc};

struct ImportFile {
    meta: FileMeta,
    reference: bool,
    modified: Option<std::time::SystemTime>,
    cached: bool,
    pending: Option<PendingSource>,
    approval: Option<Approval>,
}
enum ImportEvent {
    Root {
        requested: PathBuf,
        canonical: PathBuf,
        folder: bool,
        index: Option<PathBuf>,
    },
    Batch(Vec<ImportFile>),
    Error(PathBuf, String),
    Skipped(PathBuf, String),
    Done,
}

fn skip_existing_import(
    existing: Option<usize>,
    restore: bool,
    registry: &crate::group_identity::GroupRegistry,
) -> bool {
    existing.is_some_and(|ix| restore || !registry.index_excluded(ix))
}

// A separate timer can flush while discovery is blocked reading the next header.
fn batch_import_events(
    worker_rx: std::sync::mpsc::Receiver<ImportEvent>,
    mut tx: mpsc::Sender<Vec<ImportEvent>>,
) {
    let started = Instant::now();
    let mut last_flush = started;
    let mut first = true;
    let mut batch = Vec::new();
    let mut notices = Vec::new();
    loop {
        let timeout = if (first && !batch.is_empty()) || !notices.is_empty() {
            Duration::from_millis(200).saturating_sub(last_flush.elapsed())
        } else {
            Duration::from_millis(200)
        };
        let event = worker_rx.recv_timeout(timeout);
        match event {
            Ok(ImportEvent::Batch(files)) => batch.extend(files),
            Ok(event) => {
                if matches!(event, ImportEvent::Done) {
                    if !batch.is_empty() {
                        notices.push(ImportEvent::Batch(std::mem::take(&mut batch)));
                    }
                    notices.push(event);
                    let _ = futures::executor::block_on(tx.send(notices));
                    return;
                }
                notices.push(event);
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => return,
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
        }
        if flush_due(first, batch.len(), started.elapsed())
            || notices.len() >= 128
            || (!notices.is_empty() && last_flush.elapsed() >= Duration::from_millis(200))
        {
            if !batch.is_empty() {
                notices.push(ImportEvent::Batch(std::mem::take(&mut batch)));
                first = false;
            }
            if futures::executor::block_on(tx.send(std::mem::take(&mut notices))).is_err() {
                return;
            }
            last_flush = Instant::now();
        }
        if tx.is_closed() {
            return;
        }
    }
}

#[cfg(test)]
fn start_import(
    paths: Vec<PathBuf>,
    import: ImportConfig,
    detect_channels: bool,
    cancel: Arc<AtomicBool>,
    use_index: bool,
) -> mpsc::Receiver<Vec<ImportEvent>> {
    start_import_reviewed(
        paths,
        import,
        detect_channels,
        cancel,
        use_index,
        false,
        Approvals::new(),
    )
}

fn start_import_reviewed(
    paths: Vec<PathBuf>,
    import: ImportConfig,
    detect_channels: bool,
    cancel: Arc<AtomicBool>,
    use_index: bool,
    review_new: bool,
    approvals: Approvals,
) -> mpsc::Receiver<Vec<ImportEvent>> {
    let (tx, rx) = mpsc::channel(2);
    let (worker_tx, worker_rx) = std::sync::mpsc::sync_channel(2);
    std::thread::spawn(move || batch_import_events(worker_rx, tx));
    std::thread::spawn(move || {
        let mut tx = worker_tx;
        let send =
            |tx: &mut std::sync::mpsc::SyncSender<ImportEvent>, event| tx.send(event).is_ok();
        let single_root = paths.len() == 1;
        for path in paths {
            if cancel.load(Ordering::Relaxed) {
                break;
            }
            let root = match path.canonicalize() {
                Ok(root) => root,
                Err(e) => {
                    if !send(
                        &mut tx,
                        ImportEvent::Error(path.clone(), format!("{}: {e}", path.display())),
                    ) {
                        return;
                    }
                    continue;
                }
            };
            let folder = single_root && root.is_dir();
            let index = folder.then(|| index_cache_path(&root)).flatten();
            if !send(
                &mut tx,
                ImportEvent::Root {
                    requested: path.clone(),
                    canonical: root.clone(),
                    folder: use_index && folder,
                    index: index.clone(),
                },
            ) {
                return;
            }
            // Seed visible rows from the persisted index before opening source
            // headers. The normal walk then validates them and discovers channels.
            if use_index
                && !review_new
                && let Some(index) = index
                && let Ok(catalog) = load_index(&index, &root)
            {
                for ix in 0..catalog.len() {
                    if cancel.load(Ordering::Relaxed) {
                        break;
                    }
                    let cached_path = catalog.path(ix);
                    if !send(
                        &mut tx,
                        ImportEvent::Batch(vec![ImportFile {
                            meta: FileMeta {
                                dir: Arc::from(
                                    cached_path
                                        .parent()
                                        .unwrap_or(&root)
                                        .to_string_lossy()
                                        .as_ref(),
                                ),
                                name: catalog.name(ix).to_string().into_boxed_str(),
                                size: catalog.entry_size(ix),
                            },
                            reference: false,
                            modified: None,
                            cached: true,
                            pending: None,
                            approval: None,
                        }]),
                    ) {
                        return;
                    }
                }
            }
            let explicit_file = root.is_file();
            for entry in walkdir::WalkDir::new(&root)
                .follow_links(false)
                .sort_by_file_name()
            {
                if cancel.load(Ordering::Relaxed) {
                    break;
                }
                let entry = match entry {
                    Ok(entry) => entry,
                    Err(e) => {
                        if !send(
                            &mut tx,
                            ImportEvent::Error(
                                e.path().unwrap_or(&root).to_path_buf(),
                                e.to_string(),
                            ),
                        ) {
                            return;
                        }
                        continue;
                    }
                };
                if entry.file_type().is_file() && crate::project::is_project(entry.path()) {
                    if !send(
                        &mut tx,
                        ImportEvent::Skipped(
                            entry.path().to_path_buf(),
                            "Project file skipped; use Open project…".into(),
                        ),
                    ) {
                        return;
                    }
                    continue;
                }
                if !entry.file_type().is_file() {
                    continue;
                }
                if !explicit_file
                    && !entry.path().extension().is_some_and(|ext| {
                        crate::catalog::SPECTRUM_EXTENSIONS
                            .iter()
                            .any(|e| ext.eq_ignore_ascii_case(e))
                    })
                {
                    if !send(
                        &mut tx,
                        ImportEvent::Skipped(
                            entry.path().to_path_buf(),
                            "Non-spectrum file skipped".into(),
                        ),
                    ) {
                        return;
                    }
                    continue;
                }
                let approval = approvals.get(entry.path()).cloned();
                let file_import = approval
                    .as_ref()
                    .and_then(|a| a.choice.primary_config())
                    .unwrap_or(&import);
                let mut pending = None;
                let approved_unchanged = approval.as_ref().is_some_and(|approval| {
                    super::import_preview::SourceRevision::read(entry.path())
                        .ok()
                        .as_ref()
                        == Some(&approval.source)
                });
                let reference = if detect_channels && approved_unchanged {
                    // The reviewed full source is stronger evidence than a prefix
                    // that may end inside a long header. It was validated in full.
                    false
                } else if detect_channels {
                    match detect_import(entry.path(), file_import) {
                        Ok(Some(preview)) => {
                            if review_new {
                                let reason = if let Some(approved) = &approval {
                                    (super::import_preview::SourceRevision::read(entry.path())
                                        .ok()
                                        .as_ref()
                                        != Some(&approved.source)
                                        || crate::import_mapping::LayoutKey::from_detection(
                                            &preview,
                                        ) != approved.layout)
                                        .then(|| {
                                            "Source changed after review; confirm its new layout."
                                                .into()
                                        })
                                } else {
                                    preview.review_reason()
                                };
                                if let Some(reason) = reason {
                                    pending = Some(PendingSource {
                                        detection: Some(preview.clone()),
                                        reason,
                                    });
                                }
                            }
                            if !review_new
                                && let Some(error) = &preview.mapping_error
                                && !send(
                                    &mut tx,
                                    ImportEvent::Error(entry.path().to_path_buf(), error.clone()),
                                )
                            {
                                return;
                            }
                            approval.is_none()
                                && preview.resolved.mode != DetectionMode::Reference
                                && preview
                                    .available_channels()
                                    .contains(&DetectionMode::Reference)
                        }
                        Ok(None) => {
                            if review_new {
                                pending = Some(PendingSource { detection: None, reason: "The bounded header read did not reach numeric data; review the full source.".into() });
                            }
                            false
                        }
                        Err(error) => {
                            if !send(
                                &mut tx,
                                ImportEvent::Error(entry.path().to_path_buf(), error),
                            ) {
                                return;
                            }
                            if review_new {
                                pending = Some(PendingSource { detection: None, reason: "Source could not be parsed. Repair or locate it, then reload its preview.".into() });
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
                let metadata = match entry.metadata() {
                    Ok(metadata) => metadata,
                    Err(error) => {
                        if !send(
                            &mut tx,
                            ImportEvent::Error(entry.path().to_path_buf(), error.to_string()),
                        ) {
                            return;
                        }
                        continue;
                    }
                };
                let meta = FileMeta {
                    dir: Arc::from(
                        entry
                            .path()
                            .parent()
                            .unwrap_or(&root)
                            .to_string_lossy()
                            .as_ref(),
                    ),
                    name: entry
                        .file_name()
                        .to_string_lossy()
                        .into_owned()
                        .into_boxed_str(),
                    size: metadata.len(),
                };
                // No prefix diagnostics are stored as source totals. The inspector
                // and load_spectrum diagnose the full source with its mapping.
                let modified = metadata.modified().ok();
                if !send(
                    &mut tx,
                    ImportEvent::Batch(vec![ImportFile {
                        meta,
                        reference,
                        modified,
                        cached: false,
                        pending,
                        approval,
                    }]),
                ) {
                    return;
                }
            }
        }
        send(&mut tx, ImportEvent::Done);
    });
    rx
}

impl StudioApp {
    pub(crate) fn intake_group_index(
        &self,
        path: &std::path::Path,
        group: &crate::group_identity::GroupId,
    ) -> Option<usize> {
        self.group_registry
            .index(group)
            .filter(|&ix| self.valid_group_index(ix))
            .or_else(|| {
                self.catalog
                    .find_by_canonical_path(path)
                    .filter(|&ix| self.peek_group_id(ix).as_ref() == Some(group))
            })
    }

    pub(super) fn set_chi_standard(
        &mut self,
        standard: Option<crate::params::ChiStandard>,
        cx: &mut Context<Self>,
    ) {
        let target = self.override_target();
        if target.is_some_and(|ix| self.frozen.contains(&ix)) {
            self.status = "Processing is locked — unlock it to edit its parameters.".into();
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
        paths: Vec<PathBuf>,
        restore: bool,
        cx: &mut Context<Self>,
    ) {
        self.append_import_with_recents(paths, restore, Vec::new(), cx);
    }

    pub(super) fn append_import_with_recents(
        &mut self,
        paths: Vec<PathBuf>,
        restore: bool,
        recent_folders: Vec<PathBuf>,
        cx: &mut Context<Self>,
    ) {
        self.intake.enqueue(paths, restore, recent_folders);
        if let Some(text) = self.intake.queued_text() {
            self.status = text.into();
        }
        self.start_queued_import(cx);
        cx.notify();
    }

    pub(crate) fn start_queued_import(&mut self, cx: &mut Context<Self>) {
        if self.catalog.scanning || self.verify_running {
            return;
        }
        let Some(request) = self.intake.start_next() else {
            return;
        };
        let id = request.id;
        let restore = request.restore;
        let recent_folders = request.recent_folders;
        let mut paths = request
            .reviewed_paths
            .unwrap_or_else(|| self.intake.history[id].paths.clone());
        let activate_first = self.catalog.is_empty()
            && self.derived.is_empty()
            && self.current_path.as_os_str().is_empty();
        let migrating_source =
            (!restore && self.selected.is_none() && !self.current_path.as_os_str().is_empty())
                .then(|| self.current_path.clone());
        if let Some(path) = &migrating_source {
            paths.insert(0, path.clone());
            self.pending_project_spectrum = Some(path.clone());
        }
        // The worker determines canonical paths and index location. Enqueueing
        // never waits for filesystem metadata, including network mounts.
        self.source_dir = None;
        self.catalog_index_path = None;
        self.catalog.scanning = true;
        let generation = self.catalog_gen;
        let params = self.params.clone();
        let mut existing_references: BTreeSet<_> = self
            .derived
            .iter()
            .filter(|d| {
                d.params
                    .as_ref()
                    .is_some_and(|p| p.import.mode == DetectionMode::Reference)
            })
            .filter_map(|d| d.source.clone())
            .collect();
        let cancel = Arc::new(AtomicBool::new(false));
        self.intake_cancel = Some(cancel.clone());
        let mut rx = start_import_reviewed(
            paths,
            if restore {
                params.import.clone()
            } else {
                ImportConfig::default()
            },
            !restore,
            cancel,
            activate_first,
            !restore,
            self.intake.approved.clone(),
        );
        self.status = "Importing files and detecting reference channels…".into();
        cx.spawn(async move |this, cx| {
            let mut migrating_source = migrating_source;
            let mut reimported = false;
            let mut imported_folders = BTreeSet::new();
            while let Some(events) = rx.next().await {
                let done = events.iter().any(|event| matches!(event, ImportEvent::Done));
                let current = this.update(cx, |app, cx| {
                    if app.catalog_gen != generation { return false; }
                    for event in events {
                    match event {
                        ImportEvent::Root { requested, canonical, folder, index } => {
                            if migrating_source.as_ref() == Some(&requested) {
                                migrating_source = Some(canonical.clone());
                                app.pending_project_spectrum = Some(canonical.clone());
                            }
                            let batch = &mut app.intake.history[id];
                            for path in &mut batch.paths {
                                if *path == requested { *path = canonical.clone(); }
                            }
                            if requested != canonical && let Some(outcome) = batch.sources.remove(&requested) {
                                batch.sources.entry(canonical.clone()).or_insert(outcome);
                            }
                            if folder { app.source_dir = Some(canonical); app.catalog_index_path = index; }
                        }
                        ImportEvent::Batch(batch) => {
                            let catalog_start = app.catalog.len();
                            let derived_start = app.derived.len();
                            for file in batch {
                                let path = PathBuf::from(file.meta.dir.as_ref()).join(file.meta.name.as_ref());
                                for folder in &recent_folders {
                                    if path.starts_with(folder) { imported_folders.insert(folder.clone()); }
                                }
                                if app.intake.history[id].stopped {
                                    app.intake.history[id].sources.entry(path).or_default().skipped = Some("Stopped before adding groups".into());
                                    continue;
                                }
                                if restore && app.intake.is_pending(&path) { continue; }
                                let existing = app.catalog.find_by_canonical_path(&path);
                                let in_batch = app.intake.history[id].sources.get(&path)
                                    .is_some_and(|s| !s.created.is_empty() || s.retained);
                                if skip_existing_import(existing, restore, &app.group_registry) && !in_batch {
                                    if let Some(ix) = existing {
                                        let changed = source_changed(app.catalog.entry_size(ix), app.intake.modified.get(&path).copied(), file.meta.size, file.modified);
                                        let mut ids: Vec<_> = app.peek_group_id(ix).into_iter().collect();
                                        ids.extend(app.derived.iter().filter(|d| d.source.as_ref() == Some(&path)).filter_map(|d| d.group_id.clone()));
                                        let outcome = app.intake.history[id].sources.entry(path.clone()).or_default();
                                        // A repeated argument within this intake is still one source.
                                        if outcome.created.is_empty() { outcome.existing = ids; outcome.changed = changed; outcome.freshness_unknown = !app.intake.modified.contains_key(&path) || file.modified.is_none(); }
                                        if changed { app.record_intake_problem(id, &path, "Source changed: stored size or modification time differs. Review and reload the existing group.".into(), ProblemSeverity::Warning); }
                                    }
                                    continue;
                                }
                                let retained = migrating_source.as_ref() == Some(&path);
                                if !retained && file.approval.as_ref().is_some_and(|approval| {
                                    super::import_preview::SourceRevision::read(&path).ok().as_ref() != Some(&approval.source)
                                }) {
                                    app.intake.approved.remove(&path);
                                    app.intake.history[id].sources.entry(path).or_default().pending = Some(PendingSource {
                                        detection: None, reason: "Source changed while adding reviewed files; reload and confirm it again.".into()
                                    });
                                    continue;
                                }
                                if !retained && let Some(pending) = file.pending {
                                    app.intake.approved.remove(&path);
                                    app.intake.history[id].sources.entry(path).or_default().pending = Some(pending);
                                    continue;
                                }
                                if !file.cached {
                                    app.intake.history[id].sources.entry(path.clone()).or_default().cached = false;
                                    if let Some(ix) = existing { app.catalog.update_size(ix, file.meta.size); }
                                }
                                if let Some(modified) = file.modified { app.intake.modified.insert(path.clone(), modified); }
                                let primary = existing.unwrap_or(app.catalog.len());
                                let mut restored_ids = BTreeSet::new();
                                if !restore && !in_batch {
                                    app.bind_joint_sources();
                                    if let Some(id) = app.group_registry.reimport_source(&path, existing) {
                                        restored_ids.insert(id);
                                        reimported = true;
                                    }
                                }
                                if existing.is_none() { app.catalog.extend(vec![file.meta]); }
                                if !restore && existing.is_none() {
                                    let mut source_params = if retained { params.clone() } else { PipelineParams::default() };
                                    if !retained {
                                        source_params.import = file.approval.as_ref()
                                            .and_then(|a| a.choice.primary_config()).cloned().unwrap_or_default();
                                    }
                                    app.set_custom_params(primary, (source_params != app.params).then_some(source_params));
                                }
                                let primary_id = app.peek_group_id(primary);
                                if retained {
                                    app.intake.history[id].sources.entry(path.clone()).or_default().retained = true;
                                } else if !in_batch {
                                    let outcome = app.intake.history[id].sources.entry(path.clone()).or_default();
                                    outcome.cached = file.cached;
                                    outcome.created.extend(primary_id);
                                }
                                if file.reference && existing_references.insert(path.clone()) {
                                    let reference_params = super::import_channels::channel_params(ImportConfig { mode: DetectionMode::Reference, ..Default::default() });
                                    let mut group = DerivedSpectrum {
                                        id: app.next_group_id(),
                                        label: path.file_name().unwrap_or_default().to_string_lossy().into_owned(),
                                        source: Some(path.clone()), params: Some(reference_params), ..Default::default()
                                    };
                                    app.group_registry.assign_group(&mut group, &app.project_source_origins);
                                    if !restored_ids.is_empty() { restored_ids.extend(group.group_id.clone()); }
                                    let outcome = app.intake.history[id].sources.entry(path.clone()).or_default();
                                    outcome.existing.clear();
                                    outcome.created.extend(group.group_id.clone());
                                    app.derived.push(group);
                                }
                                if let Some(approval) = &file.approval {
                                    for mapping in approval.choice.channels.iter().filter(|c| c.mode != approval.choice.primary) {
                                        if app.source_has_channel(&path, mapping.mode) { continue; }
                                        let mut group = DerivedSpectrum {
                                            id: app.next_group_id(), label: path.file_name().unwrap_or_default().to_string_lossy().into_owned(),
                                            source: Some(path.clone()), params: Some(super::import_channels::channel_params(mapping.clone())), ..Default::default()
                                        };
                                        app.group_registry.assign_group(&mut group, &app.project_source_origins);
                                        app.intake.history[id].sources.entry(path.clone()).or_default().created.extend(group.group_id.clone());
                                        app.derived.push(group);
                                    }
                                    let created = app.intake.history[id].sources.get(&path).map(|s| s.created.clone()).unwrap_or_default();
                                    if let Some(batch) = app.intake.history.get_mut(approval.batch) {
                                        let source = batch.sources.entry(path.clone()).or_default();
                                        source.pending = None;
                                        source.failed = None;
                                        if approval.batch != id { source.created.extend(created); }
                                    }
                                    if let Some(application_id) = &approval.application {
                                        let source_id = app.group_id(primary).expect("new primary has an identity");
                                        let members = app.intake.history[id].sources.get(&path).into_iter().flat_map(|s| &s.created)
                                            .filter_map(|group| {
                                                let mapping = if *group == source_id { &app.effective_params(primary).import } else {
                                                    &app.derived.iter().find(|d| d.group_id.as_ref() == Some(group))?.params.as_ref()?.import
                                                };
                                                Some(crate::import_recipes::ApplicationMember {
                                                    source_id: source_id.clone(), path: path.clone(), group: group.clone(),
                                                    channel: mapping.mode, mapping_revision: crate::import_recipes::mapping_revision(mapping),
                                                })
                                            }).collect::<Vec<_>>();
                                        if let Some(application) = app.imports.applications.iter_mut().find(|a| &a.id == application_id) {
                                            for member in members {
                                                if !application.members.iter().any(|m| m.group == member.group) { application.members.push(member); }
                                            }
                                        }
                                    }
                                    app.intake.approved.remove(&path);
                                }
                                if !restored_ids.is_empty() { app.record_reimport(restored_ids); }
                            }
                            app.register_appended_groups(catalog_start, derived_start);
                            if activate_first && app.selected.is_none() && app.current_path.as_os_str().is_empty() && app.pending_project_spectrum.is_none() && app.pending_derived.is_none() && !app.catalog.is_empty() { app.select_entry(0, cx); }
                            app.status = app.intake.history[id].receipt().into();
                        }
                        ImportEvent::Error(path, e) => {
                            app.intake.history[id].sources.entry(path.clone()).or_default().failed = Some(e.clone());
                            if !restore && app.catalog.find_by_canonical_path(&path).is_none() {
                                app.intake.history[id].sources.entry(path.clone()).or_default().pending.get_or_insert_with(|| PendingSource { detection: None, reason: e.clone() });
                            }
                            app.record_intake_problem(id, &path, e, ProblemSeverity::Error);
                        }
                        ImportEvent::Skipped(path, reason) => {
                            app.intake.history[id].sources.entry(path).or_default().skipped = Some(reason);
                        }
                        ImportEvent::Done => {
                            if !app.intake.history[id].stopped {
                                let absent: Vec<_> = app.intake.history[id].sources.iter()
                                    .filter(|(_, outcome)| outcome.cached).map(|(path, _)| path.clone()).collect();
                                for path in absent {
                                    let message = "Indexed source was not found during the folder check".to_string();
                                    app.intake.history[id].sources.get_mut(&path).expect("known source").failed = Some(message.clone());
                                    app.record_intake_problem(id, &path, message, ProblemSeverity::Error);
                                }
                            }
                            app.catalog.scanning = false;
                            app.resolve_pending_overrides(cx);
                            if restore || app.pending_project_spectrum.is_some() { app.restore_project_selection(cx); }
                            if reimported && app.workspace == Workspace::Operando { app.ensure_operando(cx); }
                            if !app.filter_text.is_empty() { app.apply_filter(cx); }
                            app.intake_cancel = None;
                            app.intake.finish(id);
                            if !app.intake.history[id].stopped { app.persist_catalog_index(cx); }
                            app.status = app.intake.history[id].receipt().into();
                            app.record(app.status.to_string(), None);
                            for folder in recent_folders.iter().rev().filter(|folder| imported_folders.contains(*folder)) {
                                crate::settings::push_recent(&mut app.structure.settings.recent_import_folders, folder.clone(), 8);
                            }
                            if !imported_folders.is_empty() { app.persist_recent_locations(); }
                            app.finish_routed_import(cx);
                            app.start_queued_import(cx);
                        }
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

    pub(super) fn record_intake_problem(
        &mut self,
        id: import_state::BatchId,
        path: &std::path::Path,
        message: String,
        severity: ProblemSeverity,
    ) {
        push_problem(
            &mut self.job_errors,
            JobError {
                batch: Some(id),
                severity,
                label: path.display().to_string(),
                message,
            },
        );
    }
}

#[cfg(test)]
mod tests {
    fn start_import(
        paths: Vec<PathBuf>,
        import: ImportConfig,
        detect: bool,
    ) -> impl futures::Stream<Item = ImportEvent> + Unpin {
        super::start_import(paths, import, detect, Default::default(), false)
            .flat_map(futures::stream::iter)
            .filter(|event| futures::future::ready(!matches!(event, ImportEvent::Root { .. })))
    }

    #[test]
    fn intake_batches_non_spectrum_notices_without_losing_sources() {
        let (worker_tx, worker_rx) = std::sync::mpsc::channel();
        let (tx, mut rx) = mpsc::channel(2);
        for n in 0..4096 {
            worker_tx
                .send(ImportEvent::Skipped(
                    format!("notes-{n}").into(),
                    "Non-spectrum".into(),
                ))
                .unwrap();
        }
        worker_tx.send(ImportEvent::Done).unwrap();
        let pump = std::thread::spawn(move || batch_import_events(worker_rx, tx));
        let packets = futures::executor::block_on(async move {
            let mut packets = Vec::new();
            while let Some(events) = rx.next().await {
                packets.push(events);
            }
            packets
        });
        assert!(
            packets.len() < 100,
            "one UI delivery per skipped file would stall large folders"
        );
        assert_eq!(
            packets
                .iter()
                .flatten()
                .filter(|e| matches!(e, ImportEvent::Skipped(..)))
                .count(),
            4096
        );
        assert!(matches!(
            packets.last().unwrap().last(),
            Some(ImportEvent::Done)
        ));
        pump.join().unwrap();
    }

    #[test]
    fn intake_reopens_cached_sources_before_header_detection() {
        let root = std::env::temp_dir().join(format!("rexafs-intake-index-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("good.dat");
        std::fs::write(&path, "# energy i0 it\n100 10 5\n101 10 4\n").unwrap();
        let root = root.canonicalize().unwrap();
        let mut catalog = Catalog::default();
        catalog.extend(vec![FileMeta {
            dir: Arc::from(root.to_string_lossy().as_ref()),
            name: "good.dat".into(),
            size: 1,
        }]);
        let index = index_cache_path(&root).unwrap();
        crate::catalog::write_index(&index, &root, &catalog.index_parts()).unwrap();
        let events = futures::executor::block_on(
            super::start_import(
                vec![root.clone()],
                ImportConfig::default(),
                true,
                Default::default(),
                true,
            )
            .flat_map(futures::stream::iter)
            .collect::<Vec<_>>(),
        );
        let files: Vec<_> = events
            .iter()
            .filter_map(|event| match event {
                ImportEvent::Batch(files) => Some(files),
                _ => None,
            })
            .flatten()
            .collect();
        assert_eq!(files.len(), 2);
        assert!(files[0].cached);
        assert!(!files[1].cached);
        assert_eq!(files[0].meta.size, 1);
        assert!(files[1].meta.size > 1);
        assert!(files[1].modified.is_some());
        std::fs::remove_file(index).unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn intake_timer_flushes_while_discovery_is_waiting() {
        let (worker_tx, worker_rx) = std::sync::mpsc::channel();
        let (tx, rx) = mpsc::channel(2);
        let pump = std::thread::spawn(move || batch_import_events(worker_rx, tx));
        worker_tx
            .send(ImportEvent::Batch(vec![ImportFile {
                meta: FileMeta {
                    dir: Arc::from("/data"),
                    name: "one.dat".into(),
                    size: 1,
                },
                reference: false,
                modified: None,
                cached: false,
                pending: None,
                approval: None,
            }]))
            .unwrap();
        let (observed_tx, observed_rx) = std::sync::mpsc::channel();
        let consumer = std::thread::spawn(move || {
            futures::executor::block_on(async move {
                let mut rx = rx.flat_map(futures::stream::iter);
                while let Some(event) = rx.next().await {
                    observed_tx.send(event).unwrap();
                }
            })
        });
        // Discovery has not sent Done or a second file: only the timer can deliver this.
        assert!(
            matches!(observed_rx.recv_timeout(Duration::from_secs(2)).unwrap(), ImportEvent::Batch(files) if files.len() == 1)
        );
        worker_tx.send(ImportEvent::Done).unwrap();
        assert!(matches!(
            observed_rx.recv_timeout(Duration::from_secs(2)).unwrap(),
            ImportEvent::Done
        ));
        pump.join().unwrap();
        consumer.join().unwrap();
    }

    #[test]
    fn intake_flushes_sixteen_then_128_and_cancels_remaining_discovery() {
        let root =
            std::env::temp_dir().join(format!("rexafs-intake-stream-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        for i in 0..2000 {
            std::fs::write(root.join(format!("{i:03}.dat")), "100 1\n101 2\n").unwrap();
        }
        let cancel = Arc::new(AtomicBool::new(false));
        let mut rx = super::start_import(
            vec![root.clone()],
            ImportConfig::default(),
            false,
            cancel.clone(),
            false,
        )
        .flat_map(futures::stream::iter)
        .filter(|event| futures::future::ready(!matches!(event, ImportEvent::Root { .. })));
        futures::executor::block_on(async {
            assert!(
                matches!(rx.next().await, Some(ImportEvent::Batch(files)) if !files.is_empty() && files.len() <= 16)
            );
            assert!(
                matches!(rx.next().await, Some(ImportEvent::Batch(files)) if files.len() == 128)
            );
            cancel.store(true, Ordering::Relaxed);
            let mut delivered = 144;
            let mut done = false;
            while let Some(event) = rx.next().await {
                match event {
                    ImportEvent::Batch(files) => delivered += files.len(),
                    ImportEvent::Done => done = true,
                    _ => panic!("unexpected intake event"),
                }
            }
            assert!(done);
            assert!(delivered < 2000, "cancellation did not stop discovery");
        });
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn intake_mixed_sources_report_failures_skips_and_canonical_identity() {
        let root = std::env::temp_dir().join(format!("rexafs-intake-mixed-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join("good.dat"),
            "# energy i0 it\n100 10 5\n101 10 4\n",
        )
        .unwrap();
        std::fs::write(root.join("bad.dat"), [0xff]).unwrap();
        std::fs::write(root.join("notes.md"), "notes").unwrap();
        std::fs::write(root.join("nested.rxs"), "project").unwrap();
        let events = futures::executor::block_on(
            start_import(
                vec![
                    root.clone(),
                    root.join("./good.dat"),
                    root.join("missing.dat"),
                ],
                ImportConfig::default(),
                true,
            )
            .collect::<Vec<_>>(),
        );
        let errors: Vec<_> = events
            .iter()
            .filter_map(|e| {
                if let ImportEvent::Error(p, _) = e {
                    Some(p)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(errors.len(), 2);
        assert_eq!(
            events
                .iter()
                .filter(|e| matches!(e, ImportEvent::Skipped(_, _)))
                .count(),
            2
        );
        let good: Vec<_> = events
            .iter()
            .filter_map(|e| {
                if let ImportEvent::Batch(files) = e {
                    Some(files)
                } else {
                    None
                }
            })
            .flatten()
            .filter(|f| f.meta.name.as_ref() == "good.dat")
            .collect();
        assert_eq!(good.len(), 2);
        assert_eq!(good[0].meta.dir, good[1].meta.dir);
        assert_eq!(good[0].modified, good[1].modified);
        assert!(matches!(events.last(), Some(ImportEvent::Done)));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn intake_problems_survive_recent_capacity_and_remain_batch_filtered() {
        let mut problems = Vec::new();
        let tagged = JobError {
            batch: Some(3),
            severity: ProblemSeverity::Error,
            label: "source".into(),
            message: "unreadable".into(),
        };
        push_problem(&mut problems, tagged.clone());
        for _ in 0..JOB_ERROR_CAPACITY + 1 {
            push_problem(
                &mut problems,
                JobError {
                    batch: None,
                    ..tagged.clone()
                },
            );
        }
        assert_eq!(
            problems
                .iter()
                .filter(|p| p.batch == Some(3))
                .collect::<Vec<_>>(),
            vec![&tagged]
        );
        assert_eq!(
            problems.iter().filter(|p| p.batch.is_none()).count(),
            JOB_ERROR_CAPACITY
        );
    }

    #[test]
    fn route_directory_projects_are_skipped_and_reported_by_intake() {
        let root = std::env::temp_dir().join(format!("rexafs-drop-walk-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("nested.RXS"), "not spectrum data").unwrap();
        std::fs::write(root.join("mu.dat"), "100 1\n101 2\n").unwrap();
        assert_eq!(
            super::shell::path_routing::route_paths(vec![root.clone()], std::path::Path::is_dir),
            super::shell::path_routing::DropRoute::ImportFolder(root.clone())
        );
        let events = futures::executor::block_on(
            start_import(vec![root.clone()], ImportConfig::default(), false).collect::<Vec<_>>(),
        );
        assert_eq!(
            events
                .iter()
                .filter(|e| matches!(e, ImportEvent::Skipped(p, _) if p.ends_with("nested.RXS")))
                .count(),
            1
        );
        let files: Vec<_> = events
            .iter()
            .filter_map(|e| match e {
                ImportEvent::Batch(files) => Some(files),
                _ => None,
            })
            .flatten()
            .collect();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].meta.name.as_ref(), "mu.dat");
        assert!(matches!(events.last(), Some(ImportEvent::Done)));
        assert!(!events.iter().any(|e| matches!(e, ImportEvent::Error(_, _))));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn explicit_import_accepts_excluded_catalog_entries_but_restore_keeps_them_hidden() {
        let registry = crate::group_identity::GroupRegistry::default();
        let id = registry.register_source(
            Some(0),
            "/data/a.dat".into(),
            DetectionMode::Auto,
            &Default::default(),
        );
        assert!(skip_existing_import(Some(0), false, &registry));
        registry.set_excluded(&BTreeSet::from([id]));
        assert!(!skip_existing_import(Some(0), false, &registry));
        assert!(skip_existing_import(Some(0), true, &registry));
        assert!(!skip_existing_import(None, true, &registry));
    }

    use super::*;

    #[test]
    fn fresh_review_intake_keeps_guesses_pending_and_accepts_a_full_reviewed_long_header() {
        use crate::app::import_review::ReviewChoice;
        use crate::import_mapping::LayoutKey;
        let root =
            std::env::temp_dir().join(format!("rexafs-reviewed-intake-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        for (name, text) in [
            ("known.dat", "# energy mu\n8900 1\n9000 2\n9100 3\n"),
            ("unknown.dat", "8900 10 5\n9000 12 5\n9100 14 5\n"),
            ("broken.dat", "broken file\n"),
        ] {
            std::fs::write(root.join(name), text).unwrap();
        }
        let events = futures::executor::block_on(
            super::start_import_reviewed(
                vec![root.clone()],
                ImportConfig::default(),
                true,
                Default::default(),
                false,
                true,
                Approvals::new(),
            )
            .collect::<Vec<_>>(),
        );
        let files = events
            .iter()
            .flatten()
            .filter_map(|event| match event {
                ImportEvent::Batch(files) => Some(files),
                _ => None,
            })
            .flatten()
            .collect::<Vec<_>>();
        assert_eq!(files.len(), 3);
        assert!(
            files
                .iter()
                .find(|f| f.meta.name.as_ref() == "known.dat")
                .unwrap()
                .pending
                .is_none()
        );
        assert!(files.iter().filter(|f| f.pending.is_some()).count() == 2);
        let path = root.join("long.dat");
        std::fs::write(
            &path,
            format!(
                "{}# energy i0 it\n8900 10 5\n9000 12 5\n9100 14 5\n",
                "# header padding\n".repeat(5000)
            ),
        )
        .unwrap();
        let path = path.canonicalize().unwrap();
        let mapping = ImportConfig {
            mode: DetectionMode::Transmission,
            ..Default::default()
        };
        let table = crate::params::preview_import(&path, &mapping).unwrap();
        let approvals = Approvals::from([(
            path.clone(),
            Approval {
                batch: 0,
                recipe: None,
                application: None,
                layout: LayoutKey::from_preview(&table),
                choice: ReviewChoice {
                    primary: DetectionMode::Transmission,
                    channels: vec![mapping.clone()],
                },
                source: super::super::import_preview::SourceRevision::read(&path).unwrap(),
            },
        )]);
        let events = futures::executor::block_on(
            super::start_import_reviewed(
                vec![path],
                ImportConfig::default(),
                true,
                Default::default(),
                false,
                true,
                approvals,
            )
            .collect::<Vec<_>>(),
        );
        let files = events
            .iter()
            .flatten()
            .filter_map(|event| match event {
                ImportEvent::Batch(files) => Some(files),
                _ => None,
            })
            .flatten()
            .collect::<Vec<_>>();
        assert_eq!(files.len(), 1);
        assert!(files[0].pending.is_none());
        assert!(files[0].approval.as_ref().unwrap().choice.primary_config() == Some(&mapping));
        std::fs::remove_dir_all(root).unwrap();
    }
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
                        for ImportFile {
                            meta, reference, ..
                        } in files
                        {
                            files_seen += 1;
                            assert_eq!(
                                PathBuf::from(meta.dir.as_ref()).join(meta.name.as_ref()),
                                path.canonicalize().unwrap()
                            );
                            assert!(*reference);
                        }
                    }
                    ImportEvent::Error(_, error) => errors.push(error),
                    ImportEvent::Skipped(path, _) => {
                        panic!("unexpected project: {}", path.display())
                    }
                    ImportEvent::Done | ImportEvent::Root { .. } => {}
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
                    ImportEvent::Error(_, error) => {
                        panic!("incomplete prefix caused an error: {error}")
                    }
                    ImportEvent::Skipped(path, _) => {
                        panic!("unexpected project: {}", path.display())
                    }
                    ImportEvent::Done | ImportEvent::Root { .. } => {}
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
                ImportEvent::Error(_, error) => {
                    errors += 1;
                    assert!(error.contains("UTF-8"));
                }
                ImportEvent::Skipped(path, _) => {
                    panic!("unexpected project: {}", path.display())
                }
                ImportEvent::Done | ImportEvent::Root { .. } => {}
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
            batch: None,
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
                ImportEvent::Error(_, error) => panic!("restore previewed a source: {error}"),
                ImportEvent::Skipped(path, _) => {
                    panic!("unexpected project: {}", path.display())
                }
                ImportEvent::Done | ImportEvent::Root { .. } => {}
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
            assert!(!events.iter().any(|e| matches!(e, ImportEvent::Error(_, _))));
        }
    }
}
