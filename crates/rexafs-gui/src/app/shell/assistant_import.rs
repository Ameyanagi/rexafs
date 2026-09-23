//! Confirmed file intake keeps exact source bytes outside the temporary Codex
//! workspace, then uses the desktop's usual channel detection and review queue.
use serde::Deserialize;
use serde_json::Value;
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

const MAX_FILES: usize = 100;
const MAX_BYTES: u64 = 512 * 1024 * 1024;

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ImportRequest {
    pub paths: Vec<PathBuf>,
}

impl ImportRequest {
    /// Parse the explicit file selection without opening files. The confirmation
    /// shows these paths before a background worker reads any source bytes.
    pub fn from_args(args: &Value) -> Result<Self, String> {
        let request: Self = serde_json::from_value(args.clone()).map_err(|e| e.to_string())?;
        if request.paths.is_empty() || request.paths.len() > MAX_FILES {
            return Err(format!("Choose between 1 and {MAX_FILES} spectrum files."));
        }
        for path in &request.paths {
            if !path.is_absolute()
                || path
                    .components()
                    .any(|part| matches!(part, std::path::Component::ParentDir))
            {
                return Err(
                    "Use absolute file paths without '..'; download remote files first.".into(),
                );
            }
            if crate::project::is_project(path) {
                return Err(
                    "Use Open project for .rxs files; Assistant import adds spectra.".into(),
                );
            }
        }
        let mut unique = std::collections::BTreeSet::new();
        if !request.paths.iter().all(|path| unique.insert(path)) {
            return Err("Each import path must appear only once.".into());
        }
        Ok(request)
    }

    /// Copy at most 512 MiB in total, preserving filenames and byte contents.
    /// Dropping an uncommitted result removes every copy, including after Stop.
    pub fn stage(&self, root: &Path) -> Result<StagedImport, String> {
        fs::create_dir_all(root).map_err(|e| e.to_string())?;
        let directory = tempfile::Builder::new()
            .prefix("import-")
            .tempdir_in(root)
            .map_err(|e| e.to_string())?;
        let mut paths = Vec::new();
        let mut remaining = MAX_BYTES;
        let mut sources = Vec::new();
        for (index, path) in self.paths.iter().enumerate() {
            let canonical = path
                .canonicalize()
                .map_err(|e| format!("{}: {e}", path.display()))?;
            let before = fs::metadata(&canonical).map_err(|e| e.to_string())?;
            if !before.is_file() {
                return Err(format!(
                    "{} is not a regular file; select files individually.",
                    path.display()
                ));
            }
            if before.len() > remaining {
                return Err("Assistant import is limited to 512 MiB per request.".into());
            }
            let folder = directory.path().join(index.to_string());
            fs::create_dir(&folder).map_err(|e| e.to_string())?;
            let destination = folder.join(path.file_name().ok_or("Missing source filename")?);
            let source = fs::File::open(&canonical).map_err(|e| e.to_string())?;
            let opened = source.metadata().map_err(|e| e.to_string())?;
            if !opened.is_file()
                || opened.len() != before.len()
                || opened.modified().ok() != before.modified().ok()
            {
                return Err("The source changed while opening it; request import again.".into());
            }
            let mut output = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&destination)
                .map_err(|e| e.to_string())?;
            let copied = std::io::copy(&mut (&source).take(remaining + 1), &mut output)
                .map_err(|e| e.to_string())?;
            let after = source.metadata().map_err(|e| e.to_string())?;
            if copied > remaining
                || copied != before.len()
                || after.modified().ok() != before.modified().ok()
            {
                return Err(
                    "The source changed or exceeded 512 MiB during import; request it again."
                        .into(),
                );
            }
            output.flush().map_err(|e| e.to_string())?;
            remaining -= copied;
            sources.push(serde_json::json!({"requested_path":path,"resolved_path":canonical,"copy":destination,"bytes":copied}));
            paths.push(destination);
        }
        // Preserve where the untouched source copies came from even if the
        // downloaded originals disappear when the Assistant disconnects.
        fs::write(
            directory.path().join("sources.json"),
            serde_json::to_vec_pretty(&sources).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        Ok(StagedImport { directory, paths })
    }
}

pub(super) struct StagedImport {
    directory: tempfile::TempDir,
    paths: Vec<PathBuf>,
}

impl StagedImport {
    /// Retain sources before enqueuing lazy import. They survive disconnect and
    /// can be included in an embedded project using the existing Save workflow.
    pub fn keep(self) -> Vec<PathBuf> {
        let _ = self.directory.keep();
        self.paths
    }
}

/// Summarize intake without reporting queued files or mapping requests as
/// successful imports. Source comments and raw spectral arrays stay local.
pub(super) fn batch_status(
    history: &[crate::app::import_state::IntakeBatch],
    id: usize,
) -> Result<Value, String> {
    let batch = history.get(id).ok_or("Unknown import batch_id")?;
    let pending = batch
        .sources
        .values()
        .filter(|source| source.pending.is_some())
        .count();
    let failed = batch
        .sources
        .values()
        .filter(|source| source.failed.is_some())
        .count();
    let created: Vec<_> = batch
        .sources
        .values()
        .flat_map(|source| &source.created)
        .collect();
    Ok(serde_json::json!({
        "batch_id":id,"intake_finished":batch.finished,"stopped":batch.stopped,
        "needs_review":pending,"failed_files":failed,"created_groups":created.len(),
        "group_ids":created.into_iter().take(100).collect::<Vec<_>>(),
        "summary":batch.receipt(),
        "next_step":if pending > 0 { "Review channels and column mappings in the rexafs import panel." }
            else if !batch.finished { "Intake is still running or queued; check again before claiming completion." }
            else { "Use the overview to inspect available spectra and their processing state." }
    }))
}

impl super::AssistantWindow {
    pub(super) fn request_file_import(
        &mut self,
        id: Value,
        args: &Value,
        cx: &mut gpui::Context<Self>,
    ) {
        if !self.connected_apps {
            self.tool_response(
                id,
                Err("Enable Connected apps and files in Access before importing spectra.".into()),
                cx,
            );
            return;
        }
        match ImportRequest::from_args(args) {
            Ok(request) => {
                let mut lines: Vec<_> = request
                    .paths
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect();
                lines.push("Keep exact source copies in rexafs application data and open the normal import review. Original files remain unchanged.".into());
                self.queue_access(
                    id,
                    super::AccessAction::Import(request.clone()),
                    format!(
                        "Import {} spectrum file(s) into rexafs?",
                        request.paths.len()
                    ),
                    lines,
                    cx,
                );
            }
            Err(error) => self.tool_response(id, Err(error), cx),
        }
    }

    pub(super) fn import_approved_files(
        &mut self,
        key: &str,
        id: Value,
        request: ImportRequest,
        cx: &mut gpui::Context<Self>,
    ) {
        if !self.connected_apps
            || !super::changes_allowed(self.allow_changes, self.turn_edit, self.transcript.busy)
        {
            self.deny_access(key, "File import permission revoked");
            return;
        }
        let Some(root) = crate::settings::app_dir().map(|dir| dir.join("assistant-imports")) else {
            self.deny_access(key, "rexafs application data directory is unavailable");
            return;
        };
        if let Some(pending) = self.access.get_mut(key) {
            pending.approved = true;
        }
        self.access_decision(key, "Import spectrum files", "Allowed once");
        let key = key.to_owned();
        let generation = self.run_generation;
        let project = self.history_project_generation;
        cx.spawn(async move |this, cx| {
            let result = cx.background_executor().spawn(async move { request.stage(&root) }).await;
            this.update(cx, |app, cx| {
                // An uncommitted StagedImport drops its private directory when a
                // late response is rejected. Never enqueue into a new project.
                if !app.access.contains_key(&key) || app.run_generation != generation { return; }
                if !app.connected_apps || !super::changes_allowed(app.allow_changes, app.turn_edit, app.transcript.busy) {
                    app.deny_access(&key, "Import cancelled");
                    return;
                }
                let result = result.and_then(|staged| {
                    app.studio.update(cx, |studio, cx| {
                        if studio.project_generation != project {
                            return Err("The project changed; request the import again.".into());
                        }
                        let paths = staged.keep();
                        let saved_paths = paths.clone();
                        let batch = studio.intake.enqueue(paths, false, Vec::new());
                        studio.set_stage(super::super::Stage::Data, cx);
                        studio.start_queued_import(cx);
                        cx.notify();
                        Ok(serde_json::json!({"batch_id":batch,"status":"queued","saved_paths":saved_paths,
                            "next_step":"Use xray_get_import_status. Review channels and column mappings in rexafs when requested; these files are not yet confirmed as imported."}))
                    }).map_err(|e| e.to_string()).and_then(|result| result)
                });
                app.access.remove(&key);
                app.transcript.note(match &result {
                    Ok(value) => format!("Spectrum import queued · batch {} · review progress in Data", value["batch_id"]),
                    Err(error) => format!("Spectrum import failed · {error}"),
                });
                app.tool_response(id, result, cx);
            }).ok();
        }).detach();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn import_requires_explicit_files_and_never_opens_projects() {
        for value in [
            json!({"paths":[]}),
            json!({"paths":["relative.dat"]}),
            json!({"paths":["https://drive.google.com/file"]}),
            json!({"paths":["/tmp/../private.dat"]}),
            json!({"paths":["/tmp/data.rxs"]}),
            json!({"paths":["/tmp/a.dat","/tmp/a.dat"]}),
        ] {
            assert!(ImportRequest::from_args(&value).is_err());
        }
    }

    #[test]
    fn copies_preserve_bytes_and_survive_workspace_cleanup() {
        let workspace = tempfile::tempdir().unwrap();
        let library = tempfile::tempdir().unwrap();
        let path = workspace.path().join("spectrum.dat");
        let bytes = b"# Original source attribution\n1 2\n3 4\n";
        fs::write(&path, bytes).unwrap();
        let request = ImportRequest::from_args(&json!({"paths":[path]})).unwrap();
        let staged = request.stage(library.path()).unwrap();
        let copied = staged.paths[0].clone();
        assert_eq!(fs::read(&copied).unwrap(), bytes);
        drop(staged);
        assert!(!copied.exists(), "cancelled staging must be removed");
        let kept = request.stage(library.path()).unwrap().keep();
        drop(workspace);
        assert_eq!(fs::read(&kept[0]).unwrap(), bytes);
        assert_eq!(
            kept[0].file_name(),
            Some(std::ffi::OsStr::new("spectrum.dat"))
        );
        assert!(
            kept[0]
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .join("sources.json")
                .is_file()
        );
    }

    #[test]
    fn invalid_batch_leaves_no_partial_copies() {
        let source = tempfile::tempdir().unwrap();
        let library = tempfile::tempdir().unwrap();
        let path = source.path().join("good.dat");
        fs::write(&path, "1 2\n3 4\n").unwrap();
        let request = ImportRequest::from_args(&json!({"paths":[path,source.path()]})).unwrap();
        assert!(request.stage(library.path()).is_err());
        assert_eq!(fs::read_dir(library.path()).unwrap().count(), 0);
        let large = source.path().join("large.dat");
        fs::File::create(&large)
            .unwrap()
            .set_len(MAX_BYTES + 1)
            .unwrap();
        assert!(
            ImportRequest::from_args(&json!({"paths":[large]}))
                .unwrap()
                .stage(library.path())
                .is_err()
        );
    }

    #[test]
    fn queued_intake_is_not_reported_as_imported() {
        let mut intake = crate::app::import_state::IntakeState::default();
        let id = intake.enqueue(vec!["/tmp/spectrum.dat".into()], false, vec![]);
        let status = batch_status(&intake.history, id).unwrap();
        assert_eq!(status["intake_finished"], false);
        assert_eq!(status["created_groups"], 0);
        assert!(batch_status(&intake.history, id + 1).is_err());
    }
}
