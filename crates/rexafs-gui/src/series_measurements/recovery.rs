//! Private, append-only result checkpoints. A chunk is durably written before
//! the GUI publishes it. Recovery never overwrites the user-requested project.
use super::*;
use crate::project::ProjectFile;
use std::{
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, BufWriter, Read, Write},
    path::{Path, PathBuf},
};

#[derive(Clone, Serialize, Deserialize)]
pub struct RecoveryEntry {
    #[serde(skip)]
    pub directory: PathBuf,
    pub run: GroupId,
    pub title: String,
    pub created: String,
}

#[derive(Serialize, Deserialize)]
enum Record {
    Rows {
        begin: usize,
        #[serde(with = "super::storage")]
        rows: Vec<MetricRow>,
    },
    Finished {
        cancelled: bool,
    },
}

pub struct RecoveryWriter {
    log: File,
    _lock: File,
}

pub fn recovery_root() -> Result<PathBuf, String> {
    let parent = crate::settings::env_var_os("SETTINGS")
        .and_then(|p| PathBuf::from(p).parent().map(Path::to_path_buf))
        .or_else(crate::settings::app_dir)
        .ok_or("Recovery directory unavailable")?;
    Ok(parent.join("series-recovery"))
}

fn private_directory(path: &Path) -> Result<(), String> {
    let mut builder = std::fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(path).map_err(|e| e.to_string())?;
    let meta = std::fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if !meta.is_dir() || meta.file_type().is_symlink() {
        return Err("Recovery path must be a real directory".into());
    }
    Ok(())
}

fn create_private(path: &Path) -> Result<File, String> {
    let mut options = OpenOptions::new();
    options.write(true).read(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path).map_err(|e| e.to_string())
}

fn atomic_json(path: &Path, value: &impl Serialize) -> Result<(), String> {
    let temp = path.with_extension("pending");
    let mut writer = BufWriter::new(create_private(&temp)?);
    serde_json::to_writer(&mut writer, value).map_err(|e| e.to_string())?;
    writer.flush().map_err(|e| e.to_string())?;
    writer.get_ref().sync_all().map_err(|e| e.to_string())?;
    drop(writer);
    std::fs::rename(temp, path).map_err(|e| e.to_string())
}

impl RecoveryWriter {
    pub fn begin(root: &Path, mut project: ProjectFile, run: &SeriesRun) -> Result<Self, String> {
        private_directory(root)?;
        let directory = root.join(digest(
            serde_json::to_string(&(run.id.clone(), GroupId::new_result()))
                .unwrap()
                .as_bytes(),
        ));
        private_directory(&directory)?;
        let lock = create_private(&directory.join("owner.lock"))?;
        lock.try_lock().map_err(|e| e.to_string())?;
        // Run metadata has its own streaming file so the project snapshot does
        // not duplicate this run or impose the project document's size limit.
        project.series_measurements.runs.retain(|r| r.id != run.id);
        crate::project::save_with_storage(
            &directory.join("workspace.rxs"),
            &project,
            crate::project::DataStorage::Paths,
        )?;
        atomic_json(&directory.join("run.json"), run)?;
        let log = create_private(&directory.join("rows.jsonl"))?;
        let entry = RecoveryEntry {
            directory: directory.clone(),
            run: run.id.clone(),
            title: run.definition.name.clone(),
            created: run.created.clone(),
        };
        // Publish discovery only after the complete snapshot and frozen run exist.
        atomic_json(&directory.join("ready.json"), &entry)?;
        for old in discover(root)
            .unwrap_or_default()
            .into_iter()
            .filter(|e| e.run == run.id)
        {
            let _ = discard(&old);
        }
        Ok(Self { log, _lock: lock })
    }

    fn append(&mut self, record: &Record) -> Result<(), String> {
        let mut bytes = serde_json::to_vec(record).map_err(|e| e.to_string())?;
        bytes.push(b'\n');
        self.log
            .write_all(&bytes)
            .and_then(|_| self.log.sync_data())
            .map_err(|e| e.to_string())
    }
    pub fn rows(&mut self, begin: usize, rows: &[MetricRow]) -> Result<(), String> {
        self.append(&Record::Rows {
            begin,
            rows: rows.to_vec(),
        })
    }
    pub fn finish(&mut self, cancelled: bool) -> Result<(), String> {
        self.append(&Record::Finished { cancelled })
    }
}

pub fn discover(root: &Path) -> Result<Vec<RecoveryEntry>, String> {
    if !root.exists() {
        return Ok(vec![]);
    }
    let mut entries = vec![];
    for item in std::fs::read_dir(root).map_err(|e| e.to_string())? {
        let item = item.map_err(|e| e.to_string())?;
        if !item.file_type().map_err(|e| e.to_string())?.is_dir() {
            continue;
        }
        let dir = item.path();
        let Ok(lock) = OpenOptions::new()
            .read(true)
            .write(true)
            .open(dir.join("owner.lock"))
        else {
            continue;
        };
        if lock.try_lock().is_err() {
            continue;
        }
        let Ok(bytes) = std::fs::read(dir.join("ready.json")) else {
            continue;
        };
        if let Ok(mut entry) = serde_json::from_slice::<RecoveryEntry>(&bytes) {
            entry.directory = dir;
            entries.push(entry);
        }
    }
    entries.sort_by(|a, b| b.created.cmp(&a.created));
    Ok(entries)
}

fn locked_entry(entry: &RecoveryEntry) -> Result<File, String> {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(entry.directory.join("owner.lock"))
        .map_err(|e| e.to_string())?;
    file.try_lock()
        .map_err(|_| "This checkpoint is still in use by another calculation".to_string())?;
    Ok(file)
}

/// Replay complete records only. A torn final line was never committed to the
/// GUI. Corruption in a complete record fails instead of silently dropping rows.
pub fn recover(entry: &RecoveryEntry) -> Result<(ProjectFile, usize), String> {
    let _lock = locked_entry(entry)?;
    let mut project = crate::project::load(&entry.directory.join("workspace.rxs"))?;
    let run_file = File::open(entry.directory.join("run.json")).map_err(|e| e.to_string())?;
    if run_file.metadata().map_err(|e| e.to_string())?.len() > 1024 * 1024 * 1024 {
        return Err("Recovery run exceeds 1 GiB".into());
    }
    let mut run: SeriesRun =
        serde_json::from_reader(BufReader::new(run_file)).map_err(|e| e.to_string())?;
    if run.id != entry.run {
        return Err("Checkpoint run identity does not match".into());
    }
    let mut reader =
        BufReader::new(File::open(entry.directory.join("rows.jsonl")).map_err(|e| e.to_string())?);
    let mut line = Vec::new();
    let mut finished = None;
    let mut committed = 0;
    loop {
        line.clear();
        let size = reader
            .by_ref()
            .take(8 * 1024 * 1024 + 1)
            .read_until(b'\n', &mut line)
            .map_err(|e| e.to_string())?;
        if size == 0 {
            break;
        }
        if size > 8 * 1024 * 1024 {
            return Err("Recovery record exceeds 8 MiB".into());
        }
        if !line.ends_with(b"\n") {
            break;
        }
        let record: Record =
            serde_json::from_slice(&line).map_err(|e| format!("Damaged checkpoint record: {e}"))?;
        match record {
            Record::Rows { begin, rows } => {
                let end = begin
                    .checked_add(rows.len())
                    .filter(|&e| e <= run.rows.len())
                    .ok_or("Checkpoint row bounds invalid")?;
                for (old, new) in run.rows[begin..end].iter().zip(&rows) {
                    if old.frame.id != new.frame.id
                        || old.frame.group != new.frame.group
                        || old.input_revision != new.input_revision
                    {
                        return Err("Checkpoint row identity or input revision changed".into());
                    }
                }
                committed += rows.len();
                run.rows[begin..end].clone_from_slice(&rows);
            }
            Record::Finished { cancelled } => finished = Some(cancelled),
        }
    }
    run.finish(finished.unwrap_or(true));
    project.series_measurements.runs.retain(|r| r.id != run.id);
    project.series_measurements.runs.push(Arc::new(run));
    // A recovered workspace is a new save target; its original stays untouched.
    project.origin = None;
    Ok((project, committed))
}

pub fn discard(entry: &RecoveryEntry) -> Result<(), String> {
    let lock = locked_entry(entry)?;
    // Windows cannot remove an open locked file. Remove readiness first while
    // holding ownership, then release the handle before deleting this directory.
    std::fs::remove_file(entry.directory.join("ready.json")).map_err(|e| e.to_string())?;
    drop(lock);
    std::fs::remove_dir_all(&entry.directory).map_err(|e| e.to_string())
}

/// Forget recovery copies only after these complete runs were saved successfully.
pub fn acknowledge_saved(project: &ProjectFile) {
    let Ok(root) = recovery_root() else { return };
    for entry in discover(&root).unwrap_or_default() {
        if project
            .series_measurements
            .runs
            .iter()
            .any(|run| run.id == entry.run && run.complete && !run.cancelled)
        {
            let _ = discard(&entry);
        }
    }
}
