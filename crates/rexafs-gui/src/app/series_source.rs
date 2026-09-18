//! Map overview frame positions to file-backed or materialized project groups.
//! Missing members retain their position; they are never replaced by neighbors.
use super::*;
use crate::group_identity::{GroupId, GroupRegistry};

pub(super) fn prepare_overview_frame(
    path: &std::path::Path,
    group: Option<&DerivedSpectrum>,
    params: &PipelineParams,
) -> Result<XASSpectrum, String> {
    match group {
        Some(group) => group.process(params),
        None => process_file(path, params).map_err(|error| error.to_string()),
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(super) enum OverviewSource {
    Scan {
        index: usize,
        start: usize,
        len: usize,
    },
    Series {
        id: GroupId,
        revision: u64,
        entries: Arc<Vec<Option<usize>>>,
    },
}

impl OverviewSource {
    pub(super) fn from_series(
        series: &crate::series_measurements::SeriesDefinition,
        registry: &GroupRegistry,
    ) -> Self {
        Self::Series {
            id: series.id.clone(),
            revision: series.revision,
            entries: Arc::new(
                series
                    .frames
                    .iter()
                    .map(|frame| registry.index(&frame.group))
                    .collect(),
            ),
        }
    }
    pub(super) fn len(&self) -> usize {
        match self {
            Self::Scan { len, .. } => *len,
            Self::Series { entries, .. } => entries.len(),
        }
    }

    pub(super) fn scan_index(&self) -> Option<usize> {
        match self {
            Self::Scan { index, .. } => Some(*index),
            _ => None,
        }
    }

    pub(super) fn entry(&self, frame: usize) -> Option<usize> {
        match self {
            Self::Scan { start, len, .. } => (frame < *len).then_some(start + frame),
            Self::Series { entries, .. } => entries.get(frame).copied().flatten(),
        }
    }

    pub(super) fn position(&self, entry: usize) -> Option<usize> {
        match self {
            Self::Scan { start, len, .. } => scan_entry_offset(*start, *len, entry),
            Self::Series { entries, .. } => entries.iter().position(|&ix| ix == Some(entry)),
        }
    }

    pub(super) fn available(&self, frame: usize, registry: &GroupRegistry) -> bool {
        self.entry(frame)
            .is_some_and(|ix| !registry.index_excluded(ix))
    }

    pub(super) fn samples(&self, registry: &GroupRegistry, cap: usize) -> Vec<usize> {
        if let Self::Scan { start, len, .. } = self {
            return active_scan_indices(registry, *start, *len, cap)
                .into_iter()
                .map(|ix| ix - start)
                .collect();
        }
        let available: Vec<_> = (0..self.len())
            .filter(|&p| self.available(p, registry))
            .collect();
        sample_scan_indices(0, available.len(), cap)
            .into_iter()
            .map(|i| available[i])
            .collect()
    }

    pub(super) fn surviving(
        &self,
        registry: &GroupRegistry,
        pos: usize,
        backwards: bool,
    ) -> Option<usize> {
        if let Self::Scan { start, len, .. } = self {
            return surviving_frame(registry, *start, *len, pos, backwards);
        }
        let pos = pos.min(self.len().saturating_sub(1));
        let before = || (0..=pos).rev().find(|&p| self.available(p, registry));
        let after = || (pos..self.len()).find(|&p| self.available(p, registry));
        if backwards {
            before().or_else(after)
        } else {
            after().or_else(before)
        }
    }

    pub(super) fn heatmap_rows(
        &self,
        matrix: &[Vec<f64>],
        samples: &[usize],
        registry: &GroupRegistry,
    ) -> Vec<Vec<f64>> {
        if let Self::Scan { start, len, .. } = self {
            return overview_heatmap_rows(matrix, samples, registry, *start, *len);
        }
        sample_scan_indices(0, self.len(), MAX_FRAMES)
            .into_iter()
            .map(|frame| {
                let row = samples
                    .iter()
                    .enumerate()
                    .min_by_key(|(_, p)| p.abs_diff(frame))
                    .and_then(|(i, _)| matrix.get(i));
                row.filter(|_| self.available(frame, registry))
                    .cloned()
                    .unwrap_or_else(|| vec![f64::NAN; matrix.first().map_or(0, Vec::len)])
            })
            .collect()
    }
}

impl StudioApp {
    pub(super) fn overview_source(&self) -> Option<OverviewSource> {
        if let Some(id) = &self.overview_series {
            let series = self
                .measurements
                .archive
                .series
                .iter()
                .find(|s| &s.id == id)?;
            return Some(OverviewSource::from_series(series, &self.group_registry));
        }
        let index = self.active_scan?;
        let scan = self.catalog.scans.get(index)?;
        Some(OverviewSource::Scan {
            index,
            start: scan.start,
            len: scan.len,
        })
    }

    pub(super) fn overview_fingerprint(&self, source: &OverviewSource) -> u64 {
        if let OverviewSource::Scan { start, len, .. } = source {
            return scan_fingerprint(
                &self.params,
                &self.overrides,
                &self.group_registry,
                *start,
                *len,
            );
        }
        let mut hash = DefaultHasher::new();
        source.hash(&mut hash);
        for frame in 0..source.len() {
            if let Some(ix) = source.entry(frame) {
                self.group_registry.index_excluded(ix).hash(&mut hash);
                self.effective_fingerprint(ix).hash(&mut hash);
            }
        }
        hash.finish()
    }

    pub(crate) fn choose_overview_series(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(series) = self.measurements.archive.series.get(index) else {
            return;
        };
        self.overview_series = Some(series.id.clone());
        self.active_scan = None;
        self.operando = None;
        self.operando_plots = None;
        self.time_pos = 0;
        self.pending_time_pos = None;
        self.series_trend = TrendSource::WhiteLine;
        self.ui.scan_picker = false;
        self.ensure_operando(cx);
        cx.notify();
    }

    pub(crate) fn open_series_overview(&mut self, cx: &mut Context<Self>) {
        self.measurements.overview = true;
        self.ensure_operando(cx);
        cx.notify();
    }

    pub(crate) fn overview_label(&self) -> String {
        self.overview_series
            .as_ref()
            .and_then(|id| {
                self.measurements
                    .archive
                    .series
                    .iter()
                    .find(|s| &s.id == id)
                    .map(|s| s.name.clone())
            })
            .or_else(|| self.active_scan.map(|i| self.series_scan_label(i)))
            .unwrap_or_else(|| "Select series or scan".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_series_retains_order_missing_members_and_exact_cursor_mapping() {
        let source = OverviewSource::Series {
            id: GroupId::new_result(),
            revision: 2,
            entries: Arc::new(vec![
                Some(DERIVED_BASE + 9),
                None,
                Some(4),
                Some(DERIVED_BASE),
            ]),
        };
        let registry = GroupRegistry::default();
        assert_eq!(source.samples(&registry, 192), vec![0, 2, 3]);
        assert_eq!(source.samples(&registry, 2), vec![0, 3]);
        assert_eq!(source.position(4), Some(2));
        assert_eq!(source.entry(0), Some(DERIVED_BASE + 9));
        assert_eq!(source.entry(1), None);
        assert_eq!(source.surviving(&registry, 1, false), Some(2));
        assert_eq!(source.surviving(&registry, 1, true), Some(0));
        let rows = source.heatmap_rows(&[vec![9.], vec![4.], vec![0.]], &[0, 2, 3], &registry);
        assert!(rows[1][0].is_nan());
        assert_eq!(rows[2], vec![4.]);
        assert_eq!(source.scan_index(), None);
        // Frame 1 was not sampled; never replace its exact preview with frame 0.
        assert!(exact_overview_row(&[0, 2], &[vec![9.], vec![4.]], 1, 1)[0].is_nan());
    }

    #[test]
    fn directory_scan_keeps_its_original_coordinates() {
        let source = OverviewSource::Scan {
            index: 2,
            start: 20,
            len: 513,
        };
        let registry = GroupRegistry::default();
        assert_eq!(
            source.samples(&registry, MAX_FRAMES),
            sample_scan_indices(0, 513, MAX_FRAMES)
        );
        assert_eq!(source.entry(257), Some(277));
        assert_eq!(source.position(277), Some(257));
        assert_eq!(source.position(19), None);
        assert_eq!(source.scan_index(), Some(2));
    }

    #[test]
    fn restored_series_resolves_identities_after_group_reordering_and_removal() {
        use crate::series_measurements::{SeriesDefinition, SeriesFrame};
        let a = GroupId::new_result();
        let b = GroupId::new_result();
        let series = SeriesDefinition {
            id: GroupId::new_result(),
            revision: 3,
            name: "Test".into(),
            ordering: "Manual".into(),
            coordinate: Default::default(),
            frames: [a.clone(), b.clone()]
                .into_iter()
                .enumerate()
                .map(|(i, group)| SeriesFrame {
                    id: GroupId::new_result(),
                    group,
                    label: format!("frame {i}"),
                    sequence: i + 1,
                    coordinate: None,
                    acquired_at: None,
                })
                .collect(),
        };
        let saved = serde_json::to_vec(&series).unwrap();
        let restored = serde_json::from_slice(&saved).unwrap();
        let registry = GroupRegistry::default();
        registry.replace_derived(&[
            DerivedSpectrum {
                group_id: Some(b.clone()),
                ..Default::default()
            },
            DerivedSpectrum {
                group_id: Some(a.clone()),
                ..Default::default()
            },
        ]);
        let source = OverviewSource::from_series(&restored, &registry);
        assert_eq!(source.entry(0), Some(DERIVED_BASE + 1));
        assert_eq!(source.entry(1), Some(DERIVED_BASE));
        registry.set_excluded(&BTreeSet::from([a]));
        assert_eq!(source.samples(&registry, 192), vec![1]);
        assert_eq!(source.surviving(&registry, 0, false), Some(1));
        registry.replace_derived(&[]);
        let missing = OverviewSource::from_series(&restored, &registry);
        assert!(missing.samples(&registry, 192).is_empty());
        assert_eq!(missing.len(), 2);
        assert_eq!(missing.surviving(&registry, 0, false), None);
    }

    #[test]
    fn materialized_overview_matches_file_input_and_keeps_flat_components_typed() {
        let file = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../rexafs/tests/fixtures/analysis/cu-mixtures/standards/cufoil_abs.xdi");
        let params = PipelineParams {
            e0: Some(8979.),
            ..Default::default()
        };
        let original = prepare_overview_frame(&file, None, &params).unwrap();
        let mut group = DerivedSpectrum {
            energy: original.energy.as_ref().unwrap().as_slice().to_vec(),
            mu: original.mu.as_ref().unwrap().as_slice().to_vec(),
            ..Default::default()
        };
        let grid = vec![0., 1., 2.];
        let expected = frame_sample(&original, &grid).unwrap();
        let result =
            prepare_overview_frame(std::path::Path::new(""), Some(&group), &params).unwrap();
        let actual = frame_sample(&result, &grid).unwrap();
        assert_eq!(actual.norm, expected.norm);
        assert_eq!(actual.flat, expected.flat);
        assert_eq!(actual.k_row, expected.k_row);
        group.quantity = crate::params::Quantity::FlattenedMu;
        group.mu = expected.flat;
        let retained = group.mu.clone();
        let result =
            prepare_overview_frame(std::path::Path::new(""), Some(&group), &params).unwrap();
        assert_eq!(result.flat().unwrap().as_slice(), retained);
        assert_eq!(group.mu, retained);
    }
}
