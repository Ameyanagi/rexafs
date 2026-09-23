//! A SQLite transaction is the publication boundary. Source and spectrum files
//! are synced before that transaction; abandoned files are safe to reuse.
use super::*;
use rusqlite::{Connection, OptionalExtension, params};
use std::{
    fs::{File, OpenOptions},
    io::Write,
};

pub fn root() -> Result<PathBuf, String> {
    let parent = crate::settings::env_var_os("SETTINGS")
        .and_then(|p| PathBuf::from(p).parent().map(Path::to_path_buf))
        .or_else(crate::settings::app_dir)
        .ok_or("Application storage unavailable")?;
    Ok(parent.join("live-sessions"))
}

pub struct LiveStore {
    pub config: LiveConfig,
    pub directory: PathBuf,
    connection: Connection,
    lock: File,
}
impl Drop for LiveStore {
    fn drop(&mut self) {
        let _ = self.lock.unlock();
    }
}

fn error(e: impl std::fmt::Display) -> String {
    e.to_string()
}
fn private_dir(path: &Path) -> Result<(), String> {
    let mut builder = std::fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(path).map_err(error)?;
    if std::fs::symlink_metadata(path)
        .map_err(error)?
        .file_type()
        .is_symlink()
    {
        return Err("Live storage cannot be a symlink".into());
    }
    Ok(())
}

/// Create immutable files atomically. A repeated write must have identical bytes.
pub(super) fn publish(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if path.exists() {
        if std::fs::read(path).map_err(error)? == bytes {
            return Ok(());
        }
        return Err("Retained Live artifact changed; choose a new session".into());
    }
    let mut temp =
        tempfile::NamedTempFile::new_in(path.parent().ok_or("Missing artifact directory")?)
            .map_err(error)?;
    temp.write_all(bytes)
        .and_then(|_| temp.as_file().sync_all())
        .map_err(error)?;
    temp.persist_noclobber(path).map_err(error)?;
    #[cfg(unix)]
    {
        File::open(path.parent().unwrap())
            .and_then(|f| f.sync_all())
            .map_err(error)?;
    }
    Ok(())
}

impl LiveStore {
    pub fn create(
        root: &Path,
        config: LiveConfig,
        baseline: BTreeMap<PathBuf, String>,
    ) -> Result<Self, String> {
        config.validate()?;
        if !root.is_absolute()
            || root
                .components()
                .any(|c| c == std::path::Component::ParentDir)
        {
            return Err(
                "Live recovery storage needs an absolute path without parent traversal".into(),
            );
        }
        let ancestor = root
            .ancestors()
            .find(|p| p.exists())
            .ok_or("Recovery directory has no existing parent")?;
        let resolved = ancestor
            .canonicalize()
            .map_err(error)?
            .join(root.strip_prefix(ancestor).map_err(error)?);
        let folder = config.folder.canonicalize().map_err(error)?;
        if resolved.starts_with(&folder) {
            return Err("Live recovery storage must be outside the watched folder".into());
        }
        private_dir(root)?;
        let root = root.canonicalize().map_err(error)?;
        if root.starts_with(&folder) {
            return Err("Live recovery storage must be outside the watched folder".into());
        }
        let directory = root.join(digest(&serde_json::to_vec(&config.id).map_err(error)?));
        std::fs::create_dir(&directory).map_err(error)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700))
                .map_err(error)?;
        }
        let store = Self::connect(directory, config)?;
        store.connection.execute_batch("CREATE TABLE records (key TEXT PRIMARY KEY, source TEXT NOT NULL, revision TEXT NOT NULL, payload TEXT NOT NULL); CREATE TABLE baseline (source TEXT PRIMARY KEY, revision TEXT NOT NULL);").map_err(error)?;
        for (path, revision) in baseline {
            store
                .connection
                .execute(
                    "INSERT INTO baseline VALUES (?1, ?2)",
                    params![serde_json::to_string(&path).map_err(error)?, revision],
                )
                .map_err(error)?;
        }
        publish(
            &store.directory.join("session.json"),
            &serde_json::to_vec(&store.config).map_err(error)?,
        )?;
        Ok(store)
    }
    fn connect(directory: PathBuf, config: LiveConfig) -> Result<Self, String> {
        // Recovery may reach the same storage through an alias (for example
        // macOS /tmp and /private/tmp). Keep output identities and fit cache
        // keys identical to those written when the session was created.
        let directory = directory.canonicalize().map_err(error)?;
        let mut options = OpenOptions::new();
        options.read(true).write(true).create(true).truncate(false);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let lock = options.open(directory.join("owner.lock")).map_err(error)?;
        lock.try_lock()
            .map_err(|_| "This Live session is already open in another window".to_string())?;
        let connection = Connection::open(directory.join("ledger.sqlite")).map_err(error)?;
        connection
            .execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;")
            .map_err(error)?;
        Ok(Self {
            config,
            directory,
            connection,
            lock,
        })
    }
    pub fn open(directory: &Path) -> Result<Self, String> {
        let config: LiveConfig =
            serde_json::from_slice(&std::fs::read(directory.join("session.json")).map_err(error)?)
                .map_err(error)?;
        config.validate()?;
        let store = Self::connect(directory.to_path_buf(), config)?;
        store
            .connection
            .query_row("SELECT count(*) FROM records", [], |r| r.get::<_, i64>(0))
            .map_err(error)?;
        Ok(store)
    }
    pub fn contains(&self, key: &str) -> Result<bool, String> {
        self.connection
            .query_row("SELECT 1 FROM records WHERE key=?1", [key], |_| Ok(()))
            .optional()
            .map(|v| v.is_some())
            .map_err(error)
    }
    pub fn commit(&mut self, record: &LiveRecord) -> Result<(), String> {
        let tx = self.connection.transaction().map_err(error)?;
        tx.execute(
            "INSERT INTO records VALUES (?1, ?2, ?3, ?4)",
            params![
                record.key,
                serde_json::to_string(&record.source).map_err(error)?,
                record.revision,
                serde_json::to_string(record).map_err(error)?
            ],
        )
        .map_err(error)?;
        tx.commit().map_err(error)
    }
    pub fn records(&self) -> Result<Vec<LiveRecord>, String> {
        let mut statement = self
            .connection
            .prepare("SELECT payload FROM records ORDER BY rowid")
            .map_err(error)?;
        statement
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(error)?
            .map(|row| serde_json::from_str(&row.map_err(error)?).map_err(error))
            .collect()
    }
    pub fn latest(&self) -> Result<BTreeMap<PathBuf, String>, String> {
        let mut statement = self
            .connection
            .prepare("SELECT source, revision FROM records ORDER BY rowid")
            .map_err(error)?;
        statement
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(error)?
            .map(|row| {
                let (path, revision) = row.map_err(error)?;
                Ok((serde_json::from_str(&path).map_err(error)?, revision))
            })
            .collect()
    }
    pub fn baseline(&self) -> Result<BTreeMap<PathBuf, String>, String> {
        let mut statement = self
            .connection
            .prepare("SELECT source, revision FROM baseline")
            .map_err(error)?;
        statement
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(error)?
            .map(|row| {
                let (path, revision) = row.map_err(error)?;
                Ok((serde_json::from_str(&path).map_err(error)?, revision))
            })
            .collect()
    }
    pub fn snapshot(&self, revision: &str, bytes: &[u8]) -> Result<(), String> {
        if digest(bytes) != revision {
            return Err("Snapshot digest does not match".into());
        }
        publish(&self.directory.join(format!("{revision}.raw")), bytes)
    }
    pub fn spectrum(
        &self,
        key: &str,
        x: &[f64],
        y: &[f64],
        header: &str,
    ) -> Result<PathBuf, String> {
        use std::fmt::Write;
        let path = self.directory.join(format!("{key}.dat"));
        let mut text = "# rexafs Live converted cache: energy in eV, stored mu\n".to_string();
        for line in header.lines() {
            writeln!(text, "# source: {line}").map_err(error)?;
        }
        text.push_str("# -----\n# energy mu\n");
        for (x, y) in x.iter().zip(y) {
            writeln!(text, "{x} {y}").map_err(error)?;
        }
        publish(&path, text.as_bytes())?;
        Ok(path)
    }
    pub fn session(&self) -> LiveSession {
        LiveSession {
            config: self.config.clone(),
            directory: self.directory.clone(),
            series: GroupId::source(
                &self.directory.join("series"),
                crate::params::DetectionMode::Auto,
            ),
            run: GroupId::source(
                &self.directory.join("run"),
                crate::params::DetectionMode::Auto,
            ),
            published: BTreeSet::new(),
            stopped: false,
            snapshots: BTreeSet::new(),
            exafs: Vec::new(),
            exafs_trends: Vec::new(),
        }
    }
}

pub fn discover(root: &Path) -> Result<Vec<(PathBuf, LiveConfig)>, String> {
    if !root.exists() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in std::fs::read_dir(root).map_err(error)? {
        let entry = entry.map_err(error)?;
        if !entry.file_type().map_err(error)?.is_dir() {
            continue;
        }
        if let Ok(bytes) = std::fs::read(entry.path().join("session.json"))
            && let Ok(config) = serde_json::from_slice::<LiveConfig>(&bytes)
        {
            out.push((entry.path(), config));
        }
    }
    out.sort_by(|a, b| b.1.created.cmp(&a.1.created));
    Ok(out)
}
