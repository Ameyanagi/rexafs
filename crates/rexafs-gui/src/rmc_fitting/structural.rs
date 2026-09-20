//! Structural diagnostics introduced in 0.2.12, sampled separately from scattering.
//! Optimization steps are not physical time or an uncertainty ensemble.
use rexafs::rmc::{Configuration, RadialDistribution, radial_distribution};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    path::Path,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
        mpsc,
    },
};

/// Fixed for a run so every retained sample has the same interpretation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Inclusive lower and exclusive upper distance in Å. Moments cover this interval.
    pub range: [f64; 2],
    /// Equal-width spherical bins (recommended 160; at most 512).
    pub bins: usize,
    /// Attempt interval; evolutionary runs sample each generation instead.
    pub stride: usize,
    /// Maximum history samples, excluding the initial/current/best overlays.
    pub capacity: usize,
    /// True for generation indices; then stride is one generation.
    pub generations: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            range: [0., 8.],
            bins: 160,
            stride: 100,
            capacity: 256,
            generations: false,
        }
    }
}
impl Settings {
    pub fn validate(&self) -> Result<(), String> {
        if !self.range.iter().all(|r| r.is_finite())
            || self.range[0] < 0.
            || self.range[1] <= self.range[0]
            || self.range[1] > 50.
            || !(2..=512).contains(&self.bins)
            || !(1..=1_000_000).contains(&self.stride)
            || !(2..=256).contains(&self.capacity)
        {
            return Err("Structural tracking needs 0 ≤ r min < r max ≤ 50 Å, 2–512 bins, a stride of 1–1,000,000 attempts, and 2–256 retained samples.".into());
        }
        Ok(())
    }
    pub fn edges(&self) -> Vec<f64> {
        (0..=self.bins)
            .map(|i| self.range[0] + (self.range[1] - self.range[0]) * i as f64 / self.bins as f64)
            .collect()
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Pair {
    pub neighbor: u8,
    pub radial: RadialDistribution,
}
impl Pair {
    pub fn values(&self) -> &[f64] {
        self.radial
            .g_r
            .as_deref()
            .unwrap_or(&self.radial.neighbors.counts_per_absorber)
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Sample {
    /// Actual completed attempt/generation when the coordinates were sampled.
    pub step: usize,
    pub pairs: Vec<Pair>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct History {
    pub settings: Settings,
    /// Explicit absorber indices, equally weighted; the central self is excluded.
    pub centers: Vec<usize>,
    pub center_element: u8,
    pub initial: Sample,
    pub current: Sample,
    pub best: Sample,
    /// Actual sample coordinates; gaps are retained and never interpolated in data.
    pub samples: Vec<Sample>,
    pub skipped_samples: usize,
    /// Number removed by the bounded rolling history.
    pub discarded_samples: usize,
}
impl History {
    pub fn validate(&self, configuration: &Configuration, completed: usize) -> Result<(), String> {
        self.settings.validate()?;
        if self.centers.is_empty()
            || self.centers.len() > configuration.atoms.len()
            || self.centers.iter().collect::<BTreeSet<_>>().len() != self.centers.len()
            || self.centers.iter().any(|&i| {
                configuration
                    .atoms
                    .get(i)
                    .is_none_or(|a| a.atomic_number != self.center_element)
            })
            || self.samples.len() > self.settings.capacity
            || self.samples.windows(2).any(|s| s[0].step >= s[1].step)
            || self.initial.step != 0
        {
            return Err("Invalid structural history centers or sampling order.".into());
        }
        let species: Vec<_> = configuration
            .atoms
            .iter()
            .map(|a| a.atomic_number)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let edges = self.settings.edges();
        for sample in [&self.initial, &self.current, &self.best]
            .into_iter()
            .chain(&self.samples)
        {
            if sample.step > completed || sample.pairs.len() != species.len() {
                return Err("Invalid structural sample dimensions or step.".into());
            }
            for (pair, &z) in sample.pairs.iter().zip(&species) {
                let radial = &pair.radial;
                let n = &radial.neighbors;
                if pair.neighbor != z
                    || n.edges != edges
                    || n.counts_per_absorber.len() != self.settings.bins
                    || radial
                        .g_r
                        .as_ref()
                        .is_some_and(|g| g.len() != self.settings.bins)
                    || radial.g_r.is_some() != configuration.cell.is_some()
                    || radial.density.is_some() != configuration.cell.is_some()
                    || radial.independent_radius.is_some() != configuration.cell.is_some()
                    || n.counts_per_absorber
                        .iter()
                        .chain(pair.values())
                        .any(|v| !v.is_finite() || *v < 0.)
                    || !n.coordination.is_finite()
                    || n.coordination < 0.
                    || n.mean.is_some() != (n.coordination > 0.)
                    || n.variance.is_some() != (n.coordination > 0.)
                    || n.mean
                        .into_iter()
                        .chain(n.variance)
                        .any(|v| !v.is_finite() || v < 0.)
                    || radial.density.is_some_and(|v| !v.is_finite() || v <= 0.)
                    || radial
                        .independent_radius
                        .is_some_and(|v| !v.is_finite() || v <= 0.)
                {
                    return Err("Invalid structural distribution values or normalization.".into());
                }
            }
        }
        Ok(())
    }
    pub fn export(&self, directory: &Path) -> Result<(), String> {
        use std::io::Write;
        std::fs::write(
            directory.join("structural-evolution.json"),
            serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        let mut curves = std::fs::File::create(directory.join("structural-curves.csv"))
            .map_err(|e| e.to_string())?;
        let mut metrics = std::fs::File::create(directory.join("structural-history.csv"))
            .map_err(|e| e.to_string())?;
        writeln!(curves, "state,step,center_z,neighbor_z,r_lower_angstrom,r_upper_angstrom,neighbors_per_center,g_r_dimensionless").map_err(|e| e.to_string())?;
        writeln!(metrics, "state,step,center_z,neighbor_z,r_lower_angstrom,r_upper_angstrom,coordination,mean_angstrom,variance_angstrom2").map_err(|e| e.to_string())?;
        let optional = |value: Option<f64>| value.map(|v| format!("{v:.17}")).unwrap_or_default();
        for (state, sample) in [
            ("initial", &self.initial),
            ("current", &self.current),
            ("best", &self.best),
        ]
        .into_iter()
        .chain(self.samples.iter().map(|s| ("history", s)))
        {
            for pair in &sample.pairs {
                let n = &pair.radial.neighbors;
                writeln!(
                    metrics,
                    "{state},{},{},{},{:.17},{:.17},{:.17},{},{}",
                    sample.step,
                    self.center_element,
                    pair.neighbor,
                    self.settings.range[0],
                    self.settings.range[1],
                    n.coordination,
                    optional(n.mean),
                    optional(n.variance)
                )
                .map_err(|e| e.to_string())?;
                for (i, &count) in n.counts_per_absorber.iter().enumerate() {
                    writeln!(
                        curves,
                        "{state},{},{},{},{:.17},{:.17},{:.17},{}",
                        sample.step,
                        self.center_element,
                        pair.neighbor,
                        n.edges[i],
                        n.edges[i + 1],
                        count,
                        optional(pair.radial.g_r.as_ref().map(|g| g[i]))
                    )
                    .map_err(|e| e.to_string())?;
                }
            }
        }
        Ok(())
    }
}

fn sample(
    settings: &Settings,
    centers: &[usize],
    step: usize,
    configuration: &Configuration,
) -> Result<Sample, String> {
    let edges = settings.edges();
    let pairs = configuration
        .atoms
        .iter()
        .map(|a| a.atomic_number)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(|neighbor| {
            Ok(Pair {
                neighbor,
                radial: radial_distribution(configuration, centers, Some(neighbor), &edges)
                    .map_err(|e| e.to_string())?,
            })
        })
        .collect::<Result<_, String>>()?;
    Ok(Sample { step, pairs })
}
struct Frame {
    step: usize,
    current: Configuration,
    best: Configuration,
}
enum Message {
    Frame(Frame),
    Flush(mpsc::Sender<()>),
}
/// One queued frame and one active frame bound work. A saturated worker drops
/// intermediate samples and records the gap; final/pause snapshots are flushed
/// by the optimizer thread, never by the UI thread.
pub struct Worker {
    tx: Option<mpsc::SyncSender<Message>>,
    result: Arc<Mutex<Result<Option<Arc<History>>, String>>>,
    skipped: Arc<AtomicUsize>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl Worker {
    pub fn new(
        settings: Settings,
        centers: Vec<usize>,
        initial: Configuration,
        previous: Option<Arc<History>>,
    ) -> Result<Self, String> {
        settings.validate()?;
        let center_element = centers
            .first()
            .and_then(|&i| initial.atoms.get(i))
            .ok_or("Structural tracking needs selected centers.")?
            .atomic_number;
        if centers.iter().any(|&i| {
            initial
                .atoms
                .get(i)
                .is_none_or(|a| a.atomic_number != center_element)
        }) {
            return Err("Structural tracking centers must have one element identity.".into());
        }
        if previous
            .as_ref()
            .is_some_and(|h| h.settings != settings || h.centers != centers)
        {
            return Err("Structural sampling settings differ from the saved history.".into());
        }
        let (tx, rx) = mpsc::sync_channel(1);
        let result = Arc::new(Mutex::new(Ok(previous.clone())));
        let output = result.clone();
        let skipped = Arc::new(AtomicUsize::new(
            previous.as_ref().map_or(0, |h| h.skipped_samples),
        ));
        let dropped = skipped.clone();
        let thread = std::thread::spawn(move || {
            let mut history = previous.map(|h| (*h).clone());
            let initial_sample = sample(&settings, &centers, 0, &initial);
            let mut last_best: Option<Configuration> = None;
            for message in rx {
                match message {
                    Message::Flush(done) => {
                        let _ = done.send(());
                    }
                    Message::Frame(frame) => {
                        let updated = (|| {
                            let current = sample(&settings, &centers, frame.step, &frame.current)?;
                            let best = if last_best.as_ref() == Some(&frame.best) {
                                history.as_ref().unwrap().best.clone()
                            } else {
                                sample(&settings, &centers, frame.step, &frame.best)?
                            };
                            let h = history.get_or_insert(History {
                                settings: settings.clone(),
                                centers: centers.clone(),
                                center_element,
                                initial: initial_sample.clone()?,
                                current: current.clone(),
                                best: best.clone(),
                                samples: Vec::new(),
                                skipped_samples: 0,
                                discarded_samples: 0,
                            });
                            h.current = current.clone();
                            h.best = best;
                            if h.samples.last().is_some_and(|s| s.step == frame.step) {
                                h.samples.pop();
                            }
                            h.samples.push(current);
                            if h.samples.len() > settings.capacity {
                                h.samples.remove(0);
                                h.discarded_samples += 1;
                            }
                            h.skipped_samples = dropped.load(Ordering::Relaxed);
                            last_best = Some(frame.best);
                            Ok(Some(Arc::new(h.clone())))
                        })();
                        *output.lock().unwrap_or_else(|p| p.into_inner()) = updated;
                    }
                }
            }
        });
        Ok(Self {
            tx: Some(tx),
            result,
            skipped,
            thread: Some(thread),
        })
    }
    pub fn submit(
        &self,
        step: usize,
        current: &Configuration,
        best: &Configuration,
        flush: bool,
    ) -> Result<(), String> {
        let frame = Message::Frame(Frame {
            step,
            current: current.clone(),
            best: best.clone(),
        });
        let tx = self.tx.as_ref().unwrap();
        if flush {
            tx.send(frame).map_err(|_| "Structural worker stopped.")?;
            let (done, ack) = mpsc::channel();
            tx.send(Message::Flush(done))
                .map_err(|_| "Structural worker stopped.")?;
            ack.recv().map_err(|_| "Structural worker stopped.")?;
            self.snapshot()?;
        } else if let Err(error) = tx.try_send(frame) {
            match error {
                mpsc::TrySendError::Full(_) => {
                    self.skipped.fetch_add(1, Ordering::Relaxed);
                }
                mpsc::TrySendError::Disconnected(_) => {
                    return Err("Structural worker stopped.".into());
                }
            }
        }
        Ok(())
    }
    pub fn snapshot(&self) -> Result<Option<Arc<History>>, String> {
        self.result
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .clone()
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        self.tx.take();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rexafs::rmc::Atom;
    #[test]
    fn background_history_is_bounded_resumable_and_exports_actual_steps() {
        let initial = Configuration {
            atoms: vec![
                Atom {
                    atomic_number: 29,
                    position: [0.; 3],
                },
                Atom {
                    atomic_number: 8,
                    position: [2., 0., 0.],
                },
            ],
            cell: None,
        };
        let settings = Settings {
            range: [1., 3.],
            bins: 4,
            capacity: 2,
            ..Default::default()
        };
        let worker = Worker::new(settings.clone(), vec![0], initial.clone(), None).unwrap();
        for step in [0, 100, 300] {
            worker.submit(step, &initial, &initial, true).unwrap();
        }
        let h = worker.snapshot().unwrap().unwrap();
        assert_eq!(
            h.samples.iter().map(|s| s.step).collect::<Vec<_>>(),
            vec![100, 300]
        );
        assert_eq!(h.discarded_samples, 1);
        h.validate(&initial, 300).unwrap();
        let decoded: History = serde_json::from_slice(&serde_json::to_vec(&h).unwrap()).unwrap();
        let resumed =
            Worker::new(settings, vec![0], initial.clone(), Some(Arc::new(decoded))).unwrap();
        let mut moved = initial.clone();
        moved.atoms[1].position[0] = 2.2;
        resumed.submit(400, &moved, &initial, true).unwrap();
        let h = resumed.snapshot().unwrap().unwrap();
        let oxygen = h.current.pairs.iter().find(|p| p.neighbor == 8).unwrap();
        assert_eq!(oxygen.radial.neighbors.mean, Some(2.2));
        assert_eq!(h.initial.pairs[0].radial.neighbors.mean, Some(2.));
        let directory = tempfile::tempdir().unwrap();
        h.export(directory.path()).unwrap();
        let csv = std::fs::read_to_string(directory.path().join("structural-history.csv")).unwrap();
        assert!(csv.contains("history,400,29,8,"));
        assert!(!csv.contains("history,100,"));
    }
}
