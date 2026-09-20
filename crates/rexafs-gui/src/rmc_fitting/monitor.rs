//! Observe long calculations without blocking the optimizer's event channel.
use super::*;

pub(super) struct MonitorGuard {
    done: Arc<AtomicBool>,
    worker: Option<std::thread::JoinHandle<()>>,
}
impl MonitorGuard {
    pub(super) fn start(
        request: &Request,
        calculator: &PreparedRefeffCalculator,
        telemetry: Telemetry,
    ) -> Self {
        let done = Arc::new(AtomicBool::new(false));
        let stop = done.clone();
        let monitor = calculator.monitor();
        let manual = request.cache_mib;
        let mut memory = memory::available_memory();
        let initial = memory::budget(manual, memory, calculator.stats().cached_bytes);
        monitor.set_cache_bytes(initial.bytes);
        let worker = std::thread::spawn(move || {
            let started = Instant::now();
            let mut last_memory = Instant::now();
            let mut previous_misses = 0;
            let mut last_miss = None;
            while !stop.load(Ordering::Relaxed) {
                let progress = monitor.snapshot();
                if last_memory.elapsed() >= Duration::from_secs(2) {
                    memory = memory::available_memory();
                    let next = memory::budget(manual, memory, progress.stats.cached_bytes);
                    if memory::should_resize(monitor.cache_bytes(), next.bytes) {
                        if next.bytes > monitor.cache_bytes() {
                            last_miss = None;
                            previous_misses = progress.stats.repeat_misses;
                        }
                        monitor.set_cache_bytes(next.bytes);
                    }
                    last_memory = Instant::now();
                }
                let description = match progress.stage {
                    PreparedStage::Idle => "Waiting between calculations".to_string(),
                    PreparedStage::Paths => format!(
                        "Building paths for atom {} · {} candidate extensions · {} paths retained",
                        progress.absorber.unwrap_or(0) + 1,
                        progress.search.extensions,
                        progress.search.paths
                    ),
                    PreparedStage::Electronics => format!(
                        "Preparing electronic tables for atom {} · {} absorber contexts ready",
                        progress.absorber.unwrap_or(0) + 1,
                        progress.stats.contexts
                    ),
                    PreparedStage::Scattering => format!(
                        "Calculating scattering · {} path entries visited in this batch · {} absorber contexts ready",
                        progress.paths_processed, progress.stats.contexts
                    ),
                };
                if progress.stats.repeat_misses > previous_misses {
                    last_miss = Some(Instant::now());
                    previous_misses = progress.stats.repeat_misses;
                }
                let mut cache = cache_stats(&progress.stats);
                cache.limit_bytes = monitor.cache_bytes();
                let cache_warning = cache.warning().or_else(|| {
                    last_miss.filter(|time| time.elapsed() < Duration::from_secs(30)).map(|_| {
                        format!("Repeated cache misses are causing recalculation ({} total; {} evictions). More cache memory may help; inspect Run details before changing the limit.", cache.repeat_misses, cache.evictions)
                    })
                });
                let budget = memory::CacheBudget {
                    bytes: monitor.cache_bytes(),
                    automatic: manual.is_none(),
                    memory,
                };
                *telemetry.lock().unwrap_or_else(|p| p.into_inner()) = Some(CalculationProgress {
                    description,
                    elapsed_seconds: started.elapsed().as_secs_f64(),
                    cache,
                    budget,
                    cache_warning,
                });
                std::thread::park_timeout(Duration::from_millis(250));
            }
        });
        Self {
            done,
            worker: Some(worker),
        }
    }
}
impl Drop for MonitorGuard {
    fn drop(&mut self) {
        self.done.store(true, Ordering::Relaxed);
        if let Some(worker) = self.worker.take() {
            worker.thread().unpark();
            let _ = worker.join();
        }
    }
}
