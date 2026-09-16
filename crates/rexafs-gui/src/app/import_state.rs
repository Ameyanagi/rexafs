//! Persisted intake ledger. Counts describe sources, never list rows.
use crate::group_identity::GroupId;
use std::collections::{BTreeMap, VecDeque};
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

pub type BatchId = usize;

/// Identifies one pending source in one import, never future imports of its path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PendingTarget {
    pub batch: BatchId,
    pub path: PathBuf,
}

impl PendingTarget {
    pub(crate) fn extension(&self) -> Option<String> {
        self.path
            .extension()
            .map(|s| s.to_string_lossy().to_ascii_lowercase())
    }
}

/// Retain the original review evidence when the user skips a source.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct SkippedReview {
    pending: super::import_review::PendingSource,
    previous_reason: Option<String>,
}

#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct SourceOutcome {
    pub pending: Option<super::import_review::PendingSource>,
    pub created: Vec<GroupId>,
    pub existing: Vec<GroupId>,
    pub changed: bool,
    pub freshness_unknown: bool,
    pub retained: bool,
    pub cached: bool,
    pub failed: Option<String>,
    pub skipped: Option<String>,
    pub skipped_review: Option<SkippedReview>,
    pub warnings: Vec<String>,
}

impl SourceOutcome {
    pub fn summary(&self) -> String {
        let mut clauses = Vec::new();
        if let Some(pending) = &self.pending {
            clauses.push(format!("Needs mapping · {}", pending.reason));
        }
        if !self.created.is_empty() {
            clauses.push(format!("{} groups created", self.created.len()));
        }
        if !self.existing.is_empty() {
            clauses.push(if self.changed {
                "Source changed".into()
            } else {
                "Duplicate".into()
            });
        }
        if self.retained {
            clauses.push("Existing current group retained".into());
        }
        if self.freshness_unknown {
            clauses.push("Freshness not recorded; reload to check".into());
        }
        if self.failed.is_some() {
            clauses.push("Failed".into());
        }
        if let Some(reason) = &self.skipped {
            clauses.push(reason.clone());
        }
        if !self.warnings.is_empty() {
            clauses.push(format!("{} warnings", self.warnings.len()));
        }
        clauses.join(" · ")
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct IntakeBatch {
    pub paths: Vec<PathBuf>,
    pub sources: BTreeMap<PathBuf, SourceOutcome>,
    pub stopped: bool,
    pub finished: bool,
    pub dropped_queue: usize,
}

impl IntakeBatch {
    pub fn receipt(&self) -> String {
        let mut clauses = Vec::new();
        if self.stopped {
            clauses.push("Import stopped".into());
        }
        let files = self
            .sources
            .values()
            .filter(|s| !s.created.is_empty())
            .count();
        let groups: usize = self.sources.values().map(|s| s.created.len()).sum();
        if files > 0 {
            clauses.push(format!("Added {files} files → {groups} groups"));
        }
        for (count, label) in [
            (
                self.sources
                    .values()
                    .filter(|s| s.pending.is_some())
                    .count(),
                "needs mapping",
            ),
            (
                self.sources
                    .values()
                    .filter(|s| !s.existing.is_empty() && !s.changed)
                    .count(),
                "duplicates skipped",
            ),
            (
                self.sources.values().filter(|s| s.changed).count(),
                "source changed",
            ),
            (
                self.sources
                    .values()
                    .filter(|s| s.freshness_unknown)
                    .count(),
                "freshness unverified",
            ),
            (
                self.sources.values().filter(|s| s.failed.is_some()).count(),
                "failed",
            ),
            (
                self.sources.values().map(|s| s.warnings.len()).sum(),
                "warnings",
            ),
            (
                self.sources
                    .values()
                    .filter(|s| s.skipped.is_some())
                    .count(),
                "skipped",
            ),
            (self.dropped_queue, "queued imports dropped"),
        ] {
            if count > 0 {
                clauses.push(format!("{count} {label}"));
            }
        }
        if clauses.is_empty() {
            clauses.push(
                if self.finished {
                    "No files added"
                } else {
                    "Reading sources…"
                }
                .into(),
            );
        }
        clauses.join(" · ")
    }

    pub fn created(&self) -> impl Iterator<Item = &GroupId> {
        self.sources.values().flat_map(|s| &s.created)
    }
}

pub struct IntakeRequest {
    pub id: BatchId,
    pub restore: bool,
    pub recent_folders: Vec<PathBuf>,
    pub reviewed_paths: Option<Vec<PathBuf>>,
}

#[derive(Default)]
pub struct IntakeState {
    pub approved: super::import_review::Approvals,
    pub history: Vec<IntakeBatch>,
    pub queue: VecDeque<IntakeRequest>,
    pub active: Option<BatchId>,
    active_restore: bool,
    pub receipt: Option<BatchId>,
    pub history_open: bool,
    pub reveal: Vec<GroupId>,
    pub(crate) last_skip: Vec<PendingTarget>,
    // Session metadata complements the catalog's stored size. Older indexes
    // have no mtime; report that separately from a detected change.
    pub modified: BTreeMap<PathBuf, SystemTime>,
}

#[derive(Clone)]
pub struct IntakeOrigin {
    pub batch: BatchId,
    pub path: PathBuf,
    pub group: GroupId,
}

impl IntakeState {
    pub(crate) fn pending_targets(&self) -> Vec<PendingTarget> {
        self.history
            .iter()
            .enumerate()
            .flat_map(|(batch, state)| {
                state
                    .sources
                    .iter()
                    .filter(|(path, source)| {
                        source.pending.is_some() && !self.approved.contains_key(*path)
                    })
                    .map(move |(path, _)| PendingTarget {
                        batch,
                        path: path.clone(),
                    })
            })
            .collect()
    }

    /// Skip only captured sources that still await review. Files and groups are
    /// unchanged. Later imports are independent; the original evidence is saved.
    pub(crate) fn skip_pending(&mut self, targets: &[PendingTarget]) -> usize {
        let mut skipped = Vec::new();
        for target in targets {
            if self.approved.contains_key(&target.path) {
                continue;
            }
            let Some(source) = self
                .history
                .get_mut(target.batch)
                .and_then(|batch| batch.sources.get_mut(&target.path))
            else {
                continue;
            };
            let Some(pending) = source.pending.take() else {
                continue;
            };
            source.skipped_review = Some(SkippedReview {
                pending,
                previous_reason: source.skipped.replace("Skipped by user".into()),
            });
            skipped.push(target.clone());
        }
        let count = skipped.len();
        if count > 0 {
            self.last_skip = skipped;
        }
        count
    }

    /// Restore the latest skipped selection, unless the same source has since
    /// been imported or entered another pending review. No source is reread.
    pub(crate) fn undo_pending_skip(&mut self) -> usize {
        let mut restored = 0;
        for target in std::mem::take(&mut self.last_skip) {
            if self.approved.contains_key(&target.path)
                || self.history.iter().enumerate().any(|(id, batch)| {
                    id > target.batch
                        && batch.sources.get(&target.path).is_some_and(|s| {
                            s.pending.is_some() || !s.created.is_empty() || !s.existing.is_empty()
                        })
                })
            {
                continue;
            }
            let Some(source) = self
                .history
                .get_mut(target.batch)
                .and_then(|batch| batch.sources.get_mut(&target.path))
            else {
                continue;
            };
            if source.pending.is_some() {
                continue;
            }
            if let Some(review) = source.skipped_review.take() {
                source.pending = Some(review.pending);
                source.skipped = review.previous_reason;
                self.receipt = Some(target.batch);
                restored += 1;
            }
        }
        restored
    }

    /// A previously opened measurement preview cannot accept skipped or already
    /// accepted sources. Check the complete selection before creating any groups.
    pub(crate) fn measurement_review_is_current(&self, batch: BatchId, paths: &[PathBuf]) -> bool {
        !paths.is_empty()
            && self.history.get(batch).is_some_and(|batch| {
                paths.iter().all(|path| {
                    batch.sources.get(path).is_some_and(|source| {
                        source
                            .pending
                            .as_ref()
                            .is_some_and(|pending| pending.measurement_reader)
                    })
                })
            })
    }

    pub fn from_history(mut history: Vec<IntakeBatch>) -> Self {
        for batch in &mut history {
            if !batch.finished {
                batch.stopped = true;
                batch.finished = true;
            }
        }
        let receipt = history
            .iter()
            .rposition(|b| b.sources.values().any(|s| s.pending.is_some()));
        Self {
            history,
            receipt,
            ..Default::default()
        }
    }

    pub fn is_pending(&self, path: &std::path::Path) -> bool {
        self.history.iter().any(|batch| {
            batch
                .sources
                .get(path)
                .is_some_and(|source| source.pending.is_some())
        })
    }

    /// Capture when scheduling work: a later reimport of a sibling at this
    /// path must not acquire the original group's diagnostics.
    pub fn origin(&self, path: &std::path::Path, group: &GroupId) -> Option<IntakeOrigin> {
        let batch = self.history.iter().rposition(|batch| {
            batch
                .sources
                .get(path)
                .is_some_and(|s| s.created.contains(group))
        })?;
        Some(IntakeOrigin {
            batch,
            path: path.into(),
            group: group.clone(),
        })
    }

    pub fn outcome(&mut self, origin: &IntakeOrigin) -> Option<&mut SourceOutcome> {
        self.history
            .get_mut(origin.batch)?
            .sources
            .get_mut(&origin.path)
            .filter(|s| s.created.contains(&origin.group))
    }

    pub fn enqueue(
        &mut self,
        paths: Vec<PathBuf>,
        restore: bool,
        recent_folders: Vec<PathBuf>,
    ) -> BatchId {
        let id = self.history.len();
        self.history.push(IntakeBatch {
            paths,
            sources: BTreeMap::new(),
            stopped: false,
            finished: false,
            dropped_queue: 0,
        });
        self.queue.push_back(IntakeRequest {
            id,
            restore,
            recent_folders,
            reviewed_paths: None,
        });
        if !restore {
            self.receipt = Some(id);
        }
        id
    }

    pub fn enqueue_review(&mut self, id: BatchId, paths: Vec<PathBuf>) {
        if let Some(batch) = self.history.get_mut(id) {
            batch.stopped = false;
            batch.finished = false;
            self.queue.push_back(IntakeRequest {
                id,
                restore: false,
                recent_folders: vec![],
                reviewed_paths: Some(paths),
            });
            self.receipt = Some(id);
        }
    }

    pub fn start_next(&mut self) -> Option<IntakeRequest> {
        if self.active.is_some() {
            return None;
        }
        let request = self.queue.pop_front()?;
        self.active = Some(request.id);
        self.active_restore = request.restore;
        if !request.restore {
            self.receipt = Some(request.id);
        }
        Some(request)
    }

    pub fn finish(&mut self, id: BatchId) {
        self.history[id].finished = true;
        let restoring = self.active == Some(id) && self.active_restore;
        if self.active == Some(id) {
            self.active = None;
            self.active_restore = false;
        }
        if self.receipt.is_some() && !restoring {
            self.receipt = Some(id);
        }
    }

    pub fn stop(&mut self) {
        let dropped = self.queue.len();
        let ids: Vec<_> = self
            .queue
            .drain(..)
            .map(|r| r.id)
            .chain(self.active)
            .collect();
        self.approved
            .retain(|_, approval| !ids.contains(&approval.batch));
        for id in ids {
            let batch = &mut self.history[id];
            batch.stopped = true;
            // Unknown directory contents are explicitly unvisited, not invented file counts.
            for path in &batch.paths {
                batch
                    .sources
                    .entry(path.clone())
                    .or_insert_with(|| SourceOutcome {
                        skipped: Some("Discovery stopped; remaining contents not visited".into()),
                        ..Default::default()
                    });
            }
            batch.finished = self.active != Some(id);
            self.receipt = Some(id);
        }
        if let Some(id) = self.active {
            self.history[id].dropped_queue += dropped;
        }
    }

    pub fn queued_text(&self) -> Option<String> {
        let request = self.queue.front()?;
        let batch = &self.history[request.id];
        Some(format!(
            "Queued: {} files/folders, starts after current import ({} queued)",
            batch.paths.len(),
            self.queue.len()
        ))
    }
}

pub fn source_changed(
    stored_size: u64,
    stored_modified: Option<SystemTime>,
    size: u64,
    modified: Option<SystemTime>,
) -> bool {
    stored_size != size || matches!((stored_modified, modified), (Some(a), Some(b)) if a != b)
}

pub fn flush_due(first: bool, count: usize, elapsed: Duration) -> bool {
    count >= if first { 16 } else { 128 }
        || (first && count > 0 && elapsed >= Duration::from_millis(200))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pending_source() -> SourceOutcome {
        SourceOutcome {
            pending: Some(super::super::import_review::PendingSource {
                measurement_reader: true,
                detection: None,
                suggestion: None,
                reason: "Choose the signals".into(),
            }),
            ..Default::default()
        }
    }

    #[test]
    fn skip_pending_types_is_case_insensitive_scoped_and_preserves_groups() {
        let mut state = IntakeState::default();
        let batch = state.enqueue(vec!["/run".into()], false, vec![]);
        for name in ["a.prj", "b.PRJ", "c.xts", "d.QD", "no_extension"] {
            state.history[batch]
                .sources
                .insert(name.into(), pending_source());
        }
        state.history[batch].sources.insert(
            "already.prj".into(),
            SourceOutcome {
                created: vec![id(4)],
                ..Default::default()
            },
        );
        let captured: Vec<_> = state
            .pending_targets()
            .into_iter()
            .filter(|t| t.extension().as_deref() == Some("prj"))
            .collect();
        let later = state.enqueue(vec!["later.prj".into()], false, vec![]);
        state.history[later]
            .sources
            .insert("later.prj".into(), pending_source());
        assert_eq!(state.skip_pending(&captured), 2);
        assert_eq!(state.pending_targets().len(), 4);
        assert_eq!(
            state.history[batch].created().cloned().collect::<Vec<_>>(),
            vec![id(4)]
        );
        assert!(state.is_pending(std::path::Path::new("later.prj")));
        assert!(state.history[batch].receipt().contains("2 skipped"));
        assert_eq!(state.skip_pending(&captured), 0);
        assert_eq!(state.undo_pending_skip(), 2);
        assert_eq!(state.pending_targets().len(), 6);
        assert!(
            state.history[batch].sources[&PathBuf::from("a.prj")]
                .skipped
                .is_none()
        );
    }

    #[test]
    fn skipped_review_survives_project_roundtrip_without_reentering_pending() {
        let mut state = IntakeState::default();
        let batch = state.enqueue(vec!["sample.prj".into()], false, vec![]);
        let mut source = pending_source();
        source.failed = Some("historical read error".into());
        source.skipped = Some("historical note".into());
        state.history[batch]
            .sources
            .insert("sample.prj".into(), source);
        let captured = state.pending_targets();
        let paths = vec![PathBuf::from("sample.prj")];
        assert!(state.measurement_review_is_current(batch, &paths));
        assert_eq!(state.skip_pending(&captured), 1);
        assert!(!state.measurement_review_is_current(batch, &paths));
        let history =
            serde_json::from_str(&serde_json::to_string(&state.history).unwrap()).unwrap();
        let restored = IntakeState::from_history(history);
        assert!(restored.pending_targets().is_empty());
        let source = &restored.history[batch].sources[&paths[0]];
        assert_eq!(
            source.skipped_review.as_ref().unwrap().pending.reason,
            "Choose the signals"
        );
        assert_eq!(source.failed.as_deref(), Some("historical read error"));
        assert_eq!(state.undo_pending_skip(), 1);
        assert_eq!(
            state.history[batch].sources[&paths[0]].skipped.as_deref(),
            Some("historical note")
        );
        assert!(state.measurement_review_is_current(batch, &paths));
    }

    #[test]
    fn undo_skip_does_not_duplicate_a_later_import_and_stale_reviews_are_rejected() {
        let mut state = IntakeState::default();
        let batch = state.enqueue(vec!["a.qd".into(), "b.qd".into()], false, vec![]);
        for name in ["a.qd", "b.qd"] {
            state.history[batch]
                .sources
                .insert(name.into(), pending_source());
        }
        let paths = vec![PathBuf::from("a.qd"), PathBuf::from("b.qd")];
        let captured = state.pending_targets();
        assert_eq!(state.skip_pending(&captured[..1]), 1);
        assert!(!state.measurement_review_is_current(batch, &paths));
        assert!(state.measurement_review_is_current(batch, &paths[1..]));
        let later = state.enqueue(vec![paths[0].clone()], false, vec![]);
        state.history[later]
            .sources
            .insert(paths[0].clone(), pending_source());
        assert!(state.measurement_review_is_current(later, &paths[..1]));
        assert_eq!(state.undo_pending_skip(), 0);
        state.history[batch]
            .sources
            .get_mut(&paths[1])
            .unwrap()
            .pending = None;
        assert_eq!(state.skip_pending(&captured[1..]), 0);
        assert!(!state.measurement_review_is_current(batch, &paths[1..]));
    }

    #[test]
    fn project_restore_keeps_the_saved_intake_receipt_available() {
        let mut state = IntakeState::default();
        let original = state.enqueue(vec!["/run".into()], false, vec![]);
        state.start_next().unwrap();
        state.finish(original);
        let restored = state.enqueue(vec!["/run/a.dat".into()], true, vec![]);
        assert_eq!(state.receipt, Some(original));
        assert!(state.start_next().unwrap().restore);
        assert_eq!(state.receipt, Some(original));
        state.finish(restored);
        assert_eq!(state.receipt, Some(original));
    }
    fn id(n: u64) -> GroupId {
        GroupId::legacy_result(n)
    }
    #[test]
    fn import_state_cancel_preserves_added_sources_and_dismissal_survives_finish() {
        let mut state = IntakeState::default();
        let n = state.enqueue(vec!["added".into(), "unvisited".into()], false, vec![]);
        state.start_next();
        state.history[n].sources.insert(
            "added".into(),
            SourceOutcome {
                created: vec![id(1)],
                failed: Some("later load failed".into()),
                ..Default::default()
            },
        );
        state.enqueue(vec!["queued".into()], false, vec![]);
        state.stop();
        state.stop();
        assert!(
            state.history[n].sources[&PathBuf::from("added")]
                .skipped
                .is_none()
        );
        assert_eq!(state.history[n].dropped_queue, 1);
        assert_eq!(
            state.history[n].receipt(),
            "Import stopped · Added 1 files → 1 groups · 1 failed · 1 skipped · 1 queued imports dropped"
        );
        state.receipt = None;
        state.finish(n);
        assert!(state.receipt.is_none());
        assert_eq!(state.history[n].created().count(), 1);
    }

    #[test]
    fn import_state_receipt_elides_zeros_and_keeps_source_group_counts() {
        let mut state = IntakeState::default();
        let batch = state.enqueue(vec![], false, vec![]);
        let b = &mut state.history[batch];
        b.sources.insert(
            "a".into(),
            SourceOutcome {
                created: vec![id(1), id(2)],
                warnings: vec!["short rows".into()],
                ..Default::default()
            },
        );
        b.sources.insert(
            "b".into(),
            SourceOutcome {
                existing: vec![id(3)],
                ..Default::default()
            },
        );
        b.sources.insert(
            "c".into(),
            SourceOutcome {
                failed: Some("unreadable".into()),
                ..Default::default()
            },
        );
        assert_eq!(
            b.receipt(),
            "Added 1 files → 2 groups · 1 duplicates skipped · 1 failed · 1 warnings"
        );
        b.stopped = true;
        assert!(b.receipt().starts_with("Import stopped · Added"));
        assert_eq!(b.created().count(), 2);
    }
    #[test]
    fn import_state_queue_finishes_in_order_and_cancel_retains_every_batch() {
        let mut state = IntakeState::default();
        state.enqueue(vec!["first".into()], false, vec![]);
        assert_eq!(state.start_next().unwrap().id, 0);
        state.enqueue(vec!["second".into()], false, vec![]);
        assert!(state.start_next().is_none());
        assert!(
            state
                .queued_text()
                .unwrap()
                .contains("starts after current import")
        );
        state.finish(0);
        assert_eq!(state.start_next().unwrap().id, 1);
        state.history[1]
            .sources
            .entry("added".into())
            .or_default()
            .created
            .push(id(1));
        state.enqueue(vec!["third".into()], false, vec![]);
        state.stop();
        state.finish(1);
        assert!(state.queue.is_empty());
        assert_eq!(state.history[1].created().count(), 1);
        assert!(state.history[2].stopped && state.history[2].finished);
        assert_eq!(state.history[1].dropped_queue, 1);
        assert_eq!(state.receipt, Some(1));
    }
    #[test]
    fn import_state_timing_and_changed_source() {
        assert!(!flush_due(true, 15, Duration::from_millis(199)));
        assert!(flush_due(true, 16, Duration::ZERO));
        assert!(flush_due(true, 1, Duration::from_millis(200)));
        assert!(!flush_due(false, 127, Duration::from_secs(1)));
        assert!(flush_due(false, 128, Duration::ZERO));
        let t = Some(SystemTime::UNIX_EPOCH);
        assert!(!source_changed(10, t, 10, t));
        assert!(source_changed(10, t, 11, t));
        assert!(source_changed(
            10,
            t,
            10,
            Some(SystemTime::UNIX_EPOCH + Duration::from_secs(1))
        ));
        assert!(!source_changed(10, None, 10, t));
    }

    #[test]
    fn diagnostics_follow_the_created_group_not_the_latest_sibling_import() {
        let mut state = IntakeState::default();
        let first = state.enqueue(vec!["source".into()], false, vec![]);
        state.history[first]
            .sources
            .entry("source".into())
            .or_default()
            .created = vec![id(1), id(2)];
        let origin = state
            .origin(std::path::Path::new("source"), &id(2))
            .unwrap();
        let second = state.enqueue(vec!["source".into()], false, vec![]);
        state.history[second]
            .sources
            .entry("source".into())
            .or_default()
            .created
            .push(id(3));
        state.outcome(&origin).unwrap().failed = Some("reference failed".into());
        assert!(
            state.history[first].sources[&PathBuf::from("source")]
                .failed
                .is_some()
        );
        assert!(
            state.history[second].sources[&PathBuf::from("source")]
                .failed
                .is_none()
        );
        assert_eq!(
            state
                .origin(std::path::Path::new("source"), &id(3))
                .unwrap()
                .batch,
            second
        );
    }
}
