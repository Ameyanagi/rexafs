//! Runtime resource control and progress; never part of numerical identity.
use super::{PathSearchProgress, PreparedRefeffStats};
use serde::{Deserialize, Serialize};
use std::sync::{
    atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
    Arc, Mutex,
};

/// Unreleased: current prepared-scattering operation. A stage is not a percentage.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum PreparedStage {
    /// No calculation is currently in progress.
    #[default]
    Idle,
    /// Enumerating directed scattering paths within the displacement envelope.
    Paths,
    /// Preparing fixed electronic potentials and phase tables.
    Electronics,
    /// Evaluating or reusing explicit scattering paths.
    Scattering,
}

/// Unreleased: inexpensive progress snapshot for another thread to display.
/// Counters can advance during an attempted move that is later rejected or fails.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct PreparedProgress {
    /// Current operation, independent of optimizer attempts or generations.
    pub stage: PreparedStage,
    /// Zero-based absorber index during preparation; None for a calculation batch.
    pub absorber: Option<usize>,
    /// Geometric-search counters for the current absorber.
    pub search: PathSearchProgress,
    /// Catalogue entries visited in the current calculation batch, including hits.
    pub paths_processed: u64,
    /// Completed work and retained cache payload; excludes transient allocations.
    pub stats: PreparedRefeffStats,
}

struct Inner {
    observed: AtomicBool,
    budget: AtomicUsize,
    paths: AtomicU64,
    progress: Mutex<PreparedProgress>,
}

/// Unreleased: thread-safe progress and cache-budget handle. Read [`Self::snapshot`]
/// periodically; it never runs scattering. Set [`Self::set_cache_bytes`] to adjust
/// memory without changing spectra, random streams, or checkpoint identity.
#[derive(Clone)]
pub struct PreparedRefeffMonitor(Arc<Inner>);

impl PreparedRefeffMonitor {
    pub(super) fn new(bytes: usize) -> Self {
        Self(Arc::new(Inner {
            observed: AtomicBool::new(false),
            budget: AtomicUsize::new(bytes),
            paths: AtomicU64::new(0),
            progress: Mutex::new(PreparedProgress::default()),
        }))
    }

    pub(super) fn observe(&self) {
        self.0.observed.store(true, Ordering::Relaxed);
    }

    /// Current requested cache-payload limit in bytes. This is not a process
    /// memory limit: electronic tables, catalogues and active work are separate.
    pub fn cache_bytes(&self) -> usize {
        self.0.budget.load(Ordering::Relaxed)
    }

    /// Change the cache-payload limit at the next absorber-batch boundary.
    /// Decreasing it evicts the oldest snapshots; zero disables retention.
    /// Increasing it permits later results to be cached but allocates nothing
    /// immediately. Neither operation changes the calculator's stored identity.
    /// The caller is responsible for reserving memory for the rest of the process.
    pub fn set_cache_bytes(&self, bytes: usize) {
        self.0.budget.store(bytes, Ordering::Relaxed);
    }

    /// Clone the most recent completed counters and the live stage/path count.
    /// During a batch the retained-memory counters describe the previous boundary.
    pub fn snapshot(&self) -> PreparedProgress {
        let mut progress = self
            .0
            .progress
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .clone();
        progress.paths_processed = self.0.paths.load(Ordering::Relaxed);
        progress.stats.cache_limit_bytes = self.cache_bytes();
        progress
    }

    pub(super) fn stage(&self, stage: PreparedStage, absorber: Option<usize>) {
        if self.0.observed.load(Ordering::Relaxed) {
            self.0.paths.store(0, Ordering::Relaxed);
            let mut p = self.0.progress.lock().unwrap_or_else(|p| p.into_inner());
            p.stage = stage;
            p.absorber = absorber;
            p.search = PathSearchProgress::default();
        }
    }

    pub(super) fn search(&self, search: PathSearchProgress) {
        if self.0.observed.load(Ordering::Relaxed) {
            self.0
                .progress
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .search = search;
        }
    }

    pub(super) fn path(&self) {
        if self.0.observed.load(Ordering::Relaxed) {
            self.0.paths.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub(super) fn publish(&self, stats: PreparedRefeffStats) {
        if self.0.observed.load(Ordering::Relaxed) {
            self.0
                .progress
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .stats = stats;
        }
    }
}
