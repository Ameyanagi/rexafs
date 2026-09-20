//! Display-only subtraction on the overview's common grid. The reference is
//! processed from its exact source, even when the overview omits that frame.
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HeatmapPalette {
    Viridis,
    Plasma,
    Inferno,
    Coolwarm,
    RdBu,
    Gray,
}

impl HeatmapPalette {
    pub const ALL: [Self; 6] = [
        Self::Viridis,
        Self::Plasma,
        Self::Inferno,
        Self::Coolwarm,
        Self::RdBu,
        Self::Gray,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Viridis => "Viridis",
            Self::Plasma => "Plasma",
            Self::Inferno => "Inferno",
            Self::Coolwarm => "Blue–red",
            Self::RdBu => "Red–blue",
            Self::Gray => "Gray",
        }
    }
    pub fn map(self, reversed: bool) -> ruviz::render::ColorMap {
        use ruviz::render::ColorMap;
        let map = match self {
            Self::Viridis => ColorMap::viridis(),
            Self::Plasma => ColorMap::plasma(),
            Self::Inferno => ColorMap::inferno(),
            Self::Coolwarm => ColorMap::coolwarm(),
            Self::RdBu => ColorMap::rdbu(),
            Self::Gray => ColorMap::gray(),
        };
        if reversed {
            ColorMap::new(
                format!("{} reversed", self.label()),
                (0..256).map(|i| map.sample(1. - i as f64 / 255.)).collect(),
            )
        } else {
            map
        }
    }
}

#[derive(Clone, Copy, Default)]
pub(crate) struct SeriesSupport {
    energy: Option<(f64, f64)>,
    k: Option<(f64, f64)>,
    r: Option<(f64, f64)>,
    kweight: f64,
}

impl SeriesSupport {
    pub fn from_spectrum(sp: &XASSpectrum) -> Self {
        fn extent(axis: &[f64]) -> Option<(f64, f64)> {
            Some((*axis.first()?, *axis.last()?))
        }
        Self {
            energy: sp.energy.as_ref().and_then(|v| extent(v.as_slice())),
            k: sp.k().and_then(extent),
            r: sp.r().as_ref().and_then(|v| extent(v.as_slice())),
            kweight: sp.kweight().copied().unwrap_or(f64::NAN),
        }
    }
    fn extent(self, space: SeriesSpace) -> Option<(f64, f64)> {
        match space {
            SeriesSpace::Energy | SeriesSpace::Flat => self.energy,
            SeriesSpace::K => self.k,
            SeriesSpace::R => self.r,
        }
    }
}

pub(crate) struct DifferenceReference {
    source: OverviewSource,
    fingerprint: u64,
    project: u64,
    pub frame: usize,
    spectrum: Arc<XASSpectrum>,
}

#[derive(Default)]
pub(crate) struct SeriesDisplay {
    pub difference: bool,
    pub reference: Option<DifferenceReference>,
    pub pending_frame: Option<usize>,
    pub request: u64,
    pub palette: Option<HeatmapPalette>,
    pub reversed: bool,
    pub colors_open: bool,
    pub reference_open: bool,
    pub reference_field: Option<Entity<NumericField>>,
    pub frame_field: Option<Entity<crate::widgets::text_input::TextInput>>,
    pub frame_field_position: usize,
    pub frame_error: Option<String>,
    pub menu_position: gpui::Point<gpui::Pixels>,
    pub menu_focus: Option<gpui::FocusHandle>,
}

impl SeriesDisplay {
    pub fn palette(&self) -> HeatmapPalette {
        self.palette.unwrap_or(if self.difference {
            HeatmapPalette::Coolwarm
        } else {
            HeatmapPalette::Viridis
        })
    }
}

/// Missing coverage, failed samples and incompatible k weights are gaps,
/// never differences against an extrapolated zero. Inputs are unchanged.
fn subtract_row(
    row: &[f64],
    reference: &[f64],
    grid: &[f64],
    space: SeriesSpace,
    support: SeriesSupport,
    reference_support: SeriesSupport,
) -> Vec<f64> {
    let overlap = support.extent(space).zip(reference_support.extent(space));
    let weights_match = !matches!(space, SeriesSpace::K | SeriesSpace::R)
        || support.kweight == reference_support.kweight;
    grid.iter()
        .enumerate()
        .map(|(i, x)| {
            let covered = overlap
                .is_some_and(|((lo, hi), (rlo, rhi))| *x >= lo.max(rlo) && *x <= hi.min(rhi));
            match (row.get(i), reference.get(i)) {
                (Some(y), Some(r))
                    if weights_match && covered && y.is_finite() && r.is_finite() =>
                {
                    y - r
                }
                _ => f64::NAN,
            }
        })
        .collect()
}

impl StudioApp {
    pub(crate) fn active_series_reference(&self) -> Option<&DifferenceReference> {
        let data = self.operando.as_ref()?;
        self.series_display.reference.as_ref().filter(|r| {
            self.series_display.difference
                && r.project == self.project_generation
                && r.source == data.source
                && r.fingerprint == data.fingerprint
        })
    }

    pub(crate) fn series_difference_row(&self, row: Vec<f64>, support: SeriesSupport) -> Vec<f64> {
        if !self.series_display.difference {
            return row;
        }
        let Some(data) = &self.operando else {
            return row;
        };
        let space = self.stage_view.series_space;
        let grid = data.space(space).0;
        let Some(reference) = self.active_series_reference() else {
            return vec![f64::NAN; grid.len()];
        };
        let Some(values) = resample_series_frame(&reference.spectrum, space, grid) else {
            return vec![f64::NAN; grid.len()];
        };
        subtract_row(
            &row,
            &values,
            grid,
            space,
            support,
            SeriesSupport::from_spectrum(&reference.spectrum),
        )
    }

    pub(crate) fn series_difference_matrix(&self, matrix: &[Vec<f64>]) -> Vec<Vec<f64>> {
        let Some(data) = &self.operando else {
            return matrix.to_vec();
        };
        matrix
            .iter()
            .enumerate()
            .map(|(i, row)| {
                self.series_difference_row(
                    row.clone(),
                    data.support.get(i).copied().unwrap_or_default(),
                )
            })
            .collect()
    }

    pub(crate) fn series_display_kweight(&self) -> f64 {
        self.active_series_reference()
            .map(|r| SeriesSupport::from_spectrum(&r.spectrum).kweight)
            .or_else(|| self.operando.as_ref().map(|d| d.kweight))
            .unwrap_or(2.)
    }

    pub(crate) fn clear_series_difference(&mut self, cx: &mut Context<Self>) {
        self.series_display.request += 1;
        self.series_display.difference = false;
        self.series_display.reference = None;
        self.series_display.pending_frame = None;
        self.series_display.reference_open = false;
        self.rebuild_operando_plots(cx);
        cx.notify();
    }

    pub(crate) fn set_series_reference(&mut self, frame: usize, cx: &mut Context<Self>) {
        let Some(data) = &self.operando else {
            return;
        };
        let Some(ix) = data.source.entry(frame) else {
            return;
        };
        if !data.source.available(frame, &self.group_registry) {
            return;
        }
        let source = data.source.clone();
        let fingerprint = data.fingerprint;
        let project = self.project_generation;
        let derived = ix
            .checked_sub(DERIVED_BASE)
            .and_then(|i| self.derived.get(i))
            .cloned();
        let path = if ix < self.catalog.len() {
            self.catalog.path(ix)
        } else {
            PathBuf::new()
        };
        let params = self.effective_params(ix).clone();
        let cached = self
            .cache
            .peek(&(ix, self.effective_fingerprint(ix)))
            .cloned();
        self.series_display.request += 1;
        let request = self.series_display.request;
        self.series_display.difference = true;
        self.series_display.reference = None;
        self.series_display.pending_frame = Some(frame);
        self.series_display.reference_open = false;
        self.rebuild_operando_plots(cx);
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move {
                    cached.map(Ok).unwrap_or_else(|| {
                        series_source::prepare_overview_frame(&path, derived.as_ref(), &params)
                            .map(Arc::new)
                    })
                })
                .await;
            this.update(cx, |app, cx| {
                if app.project_generation != project
                    || app.series_display.request != request
                    || !app
                        .operando
                        .as_ref()
                        .is_some_and(|d| d.source == source && d.fingerprint == fingerprint)
                {
                    return;
                }
                app.series_display.pending_frame = None;
                match result {
                    Ok(spectrum) => {
                        app.series_display.reference = Some(DifferenceReference {
                            source,
                            fingerprint,
                            project,
                            frame,
                            spectrum,
                        });
                        app.status = format!("Difference from frame {}", frame + 1).into();
                    }
                    Err(error) => {
                        app.status = format!("Reference frame {}: {error}", frame + 1).into()
                    }
                }
                app.rebuild_operando_plots(cx);
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn differences_preserve_sign_and_mark_unsupported_regions() {
        let support = SeriesSupport {
            energy: Some((1., 4.)),
            ..Default::default()
        };
        let reference_support = SeriesSupport {
            energy: Some((2., 5.)),
            ..Default::default()
        };
        let row = [0., 1., 3., 1., f64::NAN, 0.];
        let reference = [0., 0., 2., 2., 2., 2.];
        let out = subtract_row(
            &row,
            &reference,
            &[0., 1., 2., 3., 4., 5.],
            SeriesSpace::Flat,
            support,
            reference_support,
        );
        assert_eq!(&out[2..4], &[1., -1.]);
        for i in [0, 1, 4, 5] {
            assert!(out[i].is_nan());
        }
        assert_eq!(row[2], 3.);
    }
    #[test]
    fn reference_subtracts_to_zero_and_mixed_weights_are_not_subtracted() {
        let support = SeriesSupport {
            k: Some((0., 2.)),
            kweight: 2.,
            ..Default::default()
        };
        let row = [3., -2., 5.];
        assert_eq!(
            subtract_row(&row, &row, &[0., 1., 2.], SeriesSpace::K, support, support),
            vec![0.; 3]
        );
        let other = SeriesSupport {
            kweight: 3.,
            ..support
        };
        assert!(
            subtract_row(&row, &row, &[0., 1., 2.], SeriesSpace::K, support, other)
                .iter()
                .all(|v| v.is_nan())
        );
    }
    #[test]
    fn palettes_reverse_without_changing_the_data_scale() {
        for palette in HeatmapPalette::ALL {
            let normal = palette.map(false);
            let reversed = palette.map(true);
            assert_eq!(normal.sample(0.), reversed.sample(1.));
            assert_eq!(normal.sample(1.), reversed.sample(0.));
        }
    }
}
