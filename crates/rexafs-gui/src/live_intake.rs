//! Live intake: completion evidence and immutable bytes.
//!
//! This module does not start a watcher, publish results, or resume a session.
//! The coordinator journals each result before acknowledging its input.
use rexafs::xafs::io::reader::{self, Measurement};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::Read,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant, SystemTime},
};

const MAX_BYTES: u64 = 256 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum CompletionPolicy {
    /// Inferred only: a paused writer can look complete. Defaults to three
    /// matching observations at least one second apart, followed by parsing.
    Quiet { checks: u16, interval_ms: u64 },
    /// The producer publishes `<source><suffix>` atomically after closing the
    /// source. Its JSON supplies byte length and SHA-256 of the complete source.
    DigestMarker { suffix: String },
}
impl Default for CompletionPolicy {
    fn default() -> Self {
        Self::Quiet {
            checks: 3,
            interval_ms: 1000,
        }
    }
}
impl CompletionPolicy {
    pub fn validate(&self) -> Result<(), String> {
        match self {
            Self::Quiet { checks, interval_ms } if *checks < 2 || *checks > 100 || *interval_ms == 0 =>
                Err("Quiet-file completion needs 2–100 observations and a positive interval".into()),
            Self::DigestMarker { suffix } if suffix.is_empty() || suffix.len() > 64 || !suffix.starts_with('.')
                || suffix.chars().any(|c| !c.is_ascii_alphanumeric() && !matches!(c, '.' | '-' | '_')) =>
                Err("Completion-marker suffix must start with a dot and contain only letters, digits, dots, hyphens or underscores".into()),
            _ => Ok(()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum CompletionEvidence {
    InferredQuiet { checks: u16, interval_ms: u64 },
    ProducerDigest { marker: PathBuf },
}

/// Exact accepted bytes and their parsed scans travel together. Process these
/// arrays instead of reopening `source`. Queue paths, not a backlog of payloads.
pub struct Snapshot {
    pub source: PathBuf,
    pub revision: String,
    pub bytes: Arc<[u8]>,
    pub measurement: Measurement,
    pub completion: CompletionEvidence,
}

pub enum Observation {
    Waiting { observed: u16, required: u16 },
    Ready(Snapshot),
    AlreadyAccepted { revision: String },
    NeedsReview(String),
    Unavailable(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Fingerprint {
    len: u64,
    modified: SystemTime,
    #[cfg(unix)]
    identity: (u64, u64),
}
impl Fingerprint {
    fn of(metadata: &fs::Metadata) -> Result<Self, String> {
        if !metadata.is_file() {
            return Err("Live inputs must be regular files".into());
        }
        #[cfg(unix)]
        use std::os::unix::fs::MetadataExt;
        Ok(Self {
            len: metadata.len(),
            modified: metadata.modified().map_err(|e| e.to_string())?,
            #[cfg(unix)]
            identity: (metadata.dev(), metadata.ino()),
        })
    }
    pub(crate) fn at(path: &Path) -> Result<Self, String> {
        let metadata =
            fs::symlink_metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
        if metadata.file_type().is_symlink() {
            return Err("Live input symlinks need explicit source review".into());
        }
        Self::of(&metadata)
    }
}

#[derive(Default)]
struct Candidate {
    fingerprint: Option<Fingerprint>,
    last_check: Option<Instant>,
    stable: u16,
    accepted: Option<String>,
    accepted_fingerprint: Option<Fingerprint>,
    pending: Option<String>,
}

/// A deterministic per-source readiness tracker, driven by reconciliation polls.
/// It creates no thread and writes nothing to the watched directory. On restart,
/// construct a fresh tracker and restore only durably committed revisions.
pub struct CompletionTracker {
    policy: CompletionPolicy,
    candidates: BTreeMap<PathBuf, Candidate>,
}
impl CompletionTracker {
    pub fn new(policy: CompletionPolicy) -> Result<Self, String> {
        policy.validate()?;
        Ok(Self {
            policy,
            candidates: BTreeMap::new(),
        })
    }

    /// `path` must be an absolute locator discovered under the chosen folder.
    /// Distinct paths remain distinct even when they contain identical bytes.
    pub fn observe(&mut self, path: &Path, now: Instant) -> Observation {
        if !path.is_absolute() {
            return Observation::Unavailable("Live input paths must be absolute".into());
        }
        let fingerprint = match Fingerprint::at(path) {
            Ok(value) => value,
            Err(error) => {
                if let Some(candidate) = self.candidates.get_mut(path) {
                    candidate.fingerprint = None;
                    candidate.last_check = None;
                    candidate.stable = 0;
                    candidate.pending = None;
                    candidate.accepted_fingerprint = None;
                }
                return Observation::Unavailable(error);
            }
        };
        let candidate = self.candidates.entry(path.to_path_buf()).or_default();
        if candidate.accepted_fingerprint.as_ref() == Some(&fingerprint)
            && let Some(revision) = &candidate.accepted
        {
            return Observation::AlreadyAccepted {
                revision: revision.clone(),
            };
        }
        if candidate.fingerprint.as_ref() != Some(&fingerprint) {
            candidate.fingerprint = Some(fingerprint.clone());
            candidate.last_check = Some(now);
            candidate.stable = 1;
            candidate.pending = None;
        } else if let CompletionPolicy::Quiet {
            checks,
            interval_ms,
        } = self.policy
        {
            if candidate.last_check.is_some_and(|last| {
                now.checked_duration_since(last)
                    .is_some_and(|d| d >= Duration::from_millis(interval_ms))
            }) {
                candidate.stable = candidate.stable.saturating_add(1).min(checks);
                candidate.last_check = Some(now);
            }
        }
        if let CompletionPolicy::Quiet { checks, .. } = self.policy {
            if candidate.stable < checks {
                return Observation::Waiting {
                    observed: candidate.stable,
                    required: checks,
                };
            }
        }
        let snapshot = match capture(path, &fingerprint, &self.policy, || {}) {
            Ok(Some(value)) => value,
            Ok(None) => {
                return Observation::Waiting {
                    observed: 0,
                    required: 1,
                };
            }
            Err(error) => return Observation::NeedsReview(error),
        };
        if candidate.accepted.as_ref() == Some(&snapshot.revision) {
            candidate.accepted_fingerprint = Some(fingerprint);
            return Observation::AlreadyAccepted {
                revision: snapshot.revision,
            };
        }
        candidate.pending = Some(snapshot.revision.clone());
        Observation::Ready(snapshot)
    }

    /// Call only after the result and input revision have been committed to the
    /// recovery ledger. A failed computation must not acknowledge the source.
    pub fn acknowledge(&mut self, snapshot: &Snapshot) -> Result<(), String> {
        let candidate = self
            .candidates
            .get_mut(&snapshot.source)
            .ok_or("Unknown Live input")?;
        if candidate.pending.is_none() && candidate.accepted.as_ref() == Some(&snapshot.revision) {
            return Ok(());
        }
        if candidate.pending.as_ref() != Some(&snapshot.revision) {
            return Err("Live input revision is no longer the pending snapshot".into());
        }
        candidate.accepted = candidate.pending.take();
        candidate.accepted_fingerprint = candidate.fingerprint.clone();
        Ok(())
    }

    /// Restore an explicit durable ledger entry without assuming readiness or
    /// restarting work. The next poll still checks the current source bytes.
    pub fn restore_accepted(&mut self, source: PathBuf, revision: String) -> Result<(), String> {
        if !source.is_absolute() || !is_digest(&revision) {
            return Err("Invalid retained Live source or SHA-256".into());
        }
        self.candidates.entry(source).or_default().accepted = Some(revision);
        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
struct DigestMarker {
    bytes: u64,
    sha256: String,
}
fn is_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}
fn hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn capture(
    path: &Path,
    expected: &Fingerprint,
    policy: &CompletionPolicy,
    after_read: impl FnOnce(),
) -> Result<Option<Snapshot>, String> {
    let marker = if let CompletionPolicy::DigestMarker { suffix } = policy {
        let mut name = path.as_os_str().to_owned();
        name.push(suffix);
        let marker_path = PathBuf::from(name);
        if !marker_path.try_exists().map_err(|e| e.to_string())? {
            return Ok(None);
        }
        Fingerprint::at(&marker_path)?;
        let mut bytes = Vec::new();
        fs::File::open(&marker_path)
            .map_err(|e| e.to_string())?
            .take(4097)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if bytes.len() > 4096 {
            return Err("Completion marker exceeds 4 KiB".into());
        }
        let marker: DigestMarker = serde_json::from_slice(&bytes)
            .map_err(|e| format!("Invalid completion marker: {e}"))?;
        if !is_digest(&marker.sha256) {
            return Err("Completion marker needs a lowercase SHA-256 digest".into());
        }
        Some((marker_path, marker, bytes))
    } else {
        None
    };
    if expected.len > MAX_BYTES {
        return Err("Live input exceeds the 256 MiB reader limit".into());
    }
    let file = fs::File::open(path).map_err(|e| e.to_string())?;
    if Fingerprint::of(&file.metadata().map_err(|e| e.to_string())?)? != *expected {
        return Err("Input changed before reading; wait for a completed revision".into());
    }
    let mut bytes = Vec::new();
    file.take(MAX_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    after_read();
    if bytes.len() as u64 != expected.len || Fingerprint::at(path)? != *expected {
        return Err("Input changed during reading; wait for a completed revision".into());
    }
    let revision = hash(&bytes);
    let completion = if let Some((path, marker, original)) = marker {
        if marker.bytes != bytes.len() as u64 || marker.sha256 != revision {
            return Err("Source does not match its producer completion marker".into());
        }
        let mut current = Vec::new();
        fs::File::open(&path)
            .map_err(|e| e.to_string())?
            .take(4097)
            .read_to_end(&mut current)
            .map_err(|e| e.to_string())?;
        if current != original {
            return Err("Completion marker changed during reading".into());
        }
        CompletionEvidence::ProducerDigest { marker: path }
    } else {
        let CompletionPolicy::Quiet {
            checks,
            interval_ms,
        } = *policy
        else {
            unreachable!()
        };
        CompletionEvidence::InferredQuiet {
            checks,
            interval_ms,
        }
    };
    let measurement = reader::parse_measurement(&bytes)
        .map_err(|e| format!("Input is not structurally readable: {e}"))?;
    Ok(Some(Snapshot {
        source: path.to_path_buf(),
        revision,
        bytes: bytes.into(),
        measurement,
        completion,
    }))
}

#[cfg(test)]
mod tests;
