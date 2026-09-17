//! Display resampling is bounded and independent of native-grid region integrals.
use super::*;
use crate::app::series_display::HeatmapPalette;
use crate::app::shell::handles::HandleKey;
use ruviz::{
    plots::heatmap::{HeatmapConfig, HeatmapOrigin},
    prelude::*,
};
use ruviz_gpui::plot_builder;

pub(super) fn scale(map: &rexafs::WaveletMap, view: usize) -> (f64, f64) {
    if view == 3 {
        return (-std::f64::consts::PI, std::f64::consts::PI);
    }
    let maximum = map
        .real()
        .iter()
        .zip(map.imaginary())
        .map(|(&a, &b)| match view {
            1 => a.abs(),
            2 => b.abs(),
            _ => a.hypot(b),
        })
        .fold(0., f64::max);
    let maximum = if maximum == 0. { 1. } else { maximum };
    (if view == 0 { 0. } else { -maximum }, maximum)
}
fn bracket(axis: &[f64], x: f64) -> (usize, f64) {
    let i = axis
        .partition_point(|v| *v <= x)
        .saturating_sub(1)
        .min(axis.len() - 2);
    (i, ((x - axis[i]) / (axis[i + 1] - axis[i])).clamp(0., 1.))
}
/// Uniform physical centers, with bilinear interpolation on the scientific grid.
/// This also respects maps with deliberately nonuniform reference R coordinates.
pub(super) fn texture(map: &rexafs::WaveletMap, view: usize) -> (Vec<Vec<f64>>, [f64; 4]) {
    let nx = map.k().len().clamp(2, 512);
    let ny = map.r().len().clamp(2, 256);
    let [ka, kb] = map.settings().k_range;
    let ra = map.r()[0];
    let rb = *map.r().last().unwrap();
    let dx = (kb - ka) / (nx - 1) as f64;
    let dy = (rb - ra) / (ny - 1) as f64;
    let max = scale(map, 0).1;
    let nk = map.k().len();
    let rows = (0..ny)
        .map(|y| {
            let (r, b) = bracket(map.r(), ra + y as f64 * dy);
            (0..nx)
                .map(|x| {
                    let (k, a) = bracket(map.k(), ka + x as f64 * dx);
                    let samples = [
                        (r * nk + k, (1. - a) * (1. - b)),
                        (r * nk + k + 1, a * (1. - b)),
                        ((r + 1) * nk + k, (1. - a) * b),
                        ((r + 1) * nk + k + 1, a * b),
                    ];
                    let blend = |component: usize| {
                        samples
                            .iter()
                            .map(|&(i, w)| {
                                w * match component {
                                    1 => map.real()[i],
                                    2 => map.imaginary()[i],
                                    _ => map.real()[i].hypot(map.imaginary()[i]),
                                }
                            })
                            .sum::<f64>()
                    };
                    if view == 3 {
                        let re = blend(1);
                        let im = blend(2);
                        if re.hypot(im) < max * 0.01 || re.hypot(im) == 0. {
                            f64::NAN
                        } else {
                            im.atan2(re)
                        }
                    } else {
                        blend(view)
                    }
                })
                .collect()
        })
        .collect();
    (
        rows,
        [ka - dx / 2., kb + dx / 2., ra - dy / 2., rb + dy / 2.],
    )
}
impl StudioApp {
    pub(super) fn rebuild_wavelet_plots(&mut self, cx: &mut Context<Self>) {
        let Some(record) = self.wavelet.record.clone() else {
            return;
        };
        let map = &record.map;
        let (matrix, extent) = texture(map, self.wavelet.view);
        let (lo, hi) = self
            .wavelet
            .lock_scale
            .unwrap_or_else(|| scale(map, self.wavelet.view));
        let palette = self.wavelet.palette.unwrap_or(if self.wavelet.view == 0 {
            HeatmapPalette::Viridis
        } else {
            HeatmapPalette::Coolwarm
        });
        let config = HeatmapConfig::new()
            .colorbar(true)
            .cmap(palette.map(self.wavelet.reversed))
            .origin(HeatmapOrigin::Lower)
            .extent(extent[0], extent[1], extent[2], extent[3])
            .vmin(lo)
            .vmax(hi);
        let plot: Plot = if matrix.iter().flatten().any(|v| v.is_finite()) {
            Plot::new()
                .theme(self.theme.plot_theme())
                .xlabel("k (Å⁻¹)")
                .ylabel("R (Å)")
                .heatmap_with(&matrix, config)
                .xlim(map.settings().k_range[0], map.settings().k_range[1])
                .ylim(map.r()[0], *map.r().last().unwrap())
                .into()
        } else {
            // All phase samples can be masked. Draw empty physical axes,
            // never fabricate a phase value just to satisfy heatmap bounds.
            Plot::new()
                .theme(self.theme.plot_theme())
                .xlabel("k (Å⁻¹)")
                .ylabel("R (Å)")
                .line(
                    &map.settings().k_range,
                    &[map.r()[0], *map.r().last().unwrap()],
                )
                .color(Color::TRANSPARENT)
                .xlim(map.settings().k_range[0], map.settings().k_range[1])
                .ylim(map.r()[0], *map.r().last().unwrap())
                .into()
        };
        if let Some(entity) = &self.wavelet.plot_map {
            entity.update(cx, |p, cx| p.set_plot_keep_view(plot, cx));
        } else {
            let entity = plot_builder(plot)
                .interactive()
                .interaction_options(ruviz_gpui::InteractionOptions {
                    tooltips: false,
                    selection: false,
                    ..Default::default()
                })
                .build(cx);
            self.wavelet.subscription = Some(cx.subscribe(
                &entity,
                |app, _, event: &PlotPointerEvent, cx| {
                    if event.kind == PlotPointerEventKind::Click
                        && event.mouse_button == Some(gpui::MouseButton::Left)
                        && let Some(position) = event.data_position
                    {
                        let (k, r) = (position.x, position.y);
                        let Some(record) = &app.wavelet.record else {
                            return;
                        };
                        let bounds = record.map.settings().k_range;
                        app.wavelet.cursor = [
                            k.clamp(bounds[0], bounds[1]),
                            r.clamp(record.map.r()[0], *record.map.r().last().unwrap()),
                        ];
                        if !app.wavelet.slices {
                            app.wavelet.plot_k = None;
                            app.wavelet.plot_r = None;
                        }
                        app.wavelet.slices = true;
                        app.rebuild_wavelet_slices(cx);
                        cx.notify();
                    }
                },
            ));
            self.wavelet.plot_map = Some(entity);
        }
        self.rebuild_wavelet_slices(cx);
    }
    pub(super) fn wavelet_cursor_value(&self) -> Option<f64> {
        let record = self.wavelet.record.as_ref()?;
        let values = record.map.slice_at_r(self.wavelet.cursor[1]).ok()?;
        let (i, a) = bracket(record.map.k(), self.wavelet.cursor[0]);
        Some(values[i] * (1. - a) + values[i + 1] * a)
    }
    /// Lightweight overlay follows view transforms; changing a region never
    /// reconstructs the texture or changes its numerical color scale.
    pub(super) fn wavelet_map_overlay(&self) -> Option<gpui::AnyElement> {
        let entity = self.wavelet.plot_map.clone()?;
        let region = self
            .wavelet
            .region_value
            .as_ref()
            .map(|_| self.wavelet.region);
        let cursor = self.wavelet.cursor;
        let slices = self.wavelet.slices;
        let color: gpui::Hsla = self.theme.accent.into();
        Some(
            gpui::canvas(
                move |_, _, cx| {
                    use ruviz::core::plot::ViewportPoint;
                    let p = entity.read(cx);
                    let bounds = p
                        .interactive_session()
                        .viewport_snapshot()
                        .ok()?
                        .visible_bounds;
                    let xmin = bounds.min.x.min(bounds.max.x) + 1e-8;
                    let xmax = bounds.min.x.max(bounds.max.x) - 1e-8;
                    let ymin = bounds.min.y.min(bounds.max.y) + 1e-8;
                    let ymax = bounds.min.y.max(bounds.max.y) - 1e-8;
                    let screen = |x, y| p.screen_at(ViewportPoint { x, y }).ok().flatten();
                    let rect = region.and_then(|[ka, kb, ra, rb]| {
                        if kb < xmin || ka > xmax || rb < ymin || ra > ymax {
                            return None;
                        }
                        Some((
                            screen(ka.clamp(xmin, xmax), ra.clamp(ymin, ymax))?,
                            screen(kb.clamp(xmin, xmax), rb.clamp(ymin, ymax))?,
                        ))
                    });
                    let point = if slices {
                        screen(cursor[0], cursor[1])
                    } else {
                        None
                    };
                    Some((rect, point))
                },
                move |_, geometry, window, _| {
                    use gpui::{Bounds, fill, point, size};
                    let Some((rect, cursor)) = geometry else {
                        return;
                    };
                    if let Some((a, b)) = rect {
                        let left = a.x.min(b.x);
                        let top = a.y.min(b.y);
                        let width = (b.x - a.x).abs();
                        let height = (b.y - a.y).abs();
                        for bounds in [
                            Bounds::new(point(left, top), size(width, px(1.2))),
                            Bounds::new(point(left, top + height), size(width, px(1.2))),
                            Bounds::new(point(left, top), size(px(1.2), height)),
                            Bounds::new(point(left + width, top), size(px(1.2), height)),
                        ] {
                            window.paint_quad(fill(bounds, color));
                        }
                    }
                    if let Some(p) = cursor {
                        for bounds in [
                            Bounds::new(point(p.x - px(5.), p.y - px(0.7)), size(px(10.), px(1.4))),
                            Bounds::new(point(p.x - px(0.7), p.y - px(5.)), size(px(1.4), px(10.))),
                        ] {
                            window.paint_quad(fill(bounds, gpui::white()));
                        }
                    }
                },
            )
            .absolute()
            .inset_0()
            .into_any_element(),
        )
    }
    pub(super) fn rebuild_wavelet_slices(&mut self, cx: &mut Context<Self>) {
        let Some(record) = &self.wavelet.record else {
            return;
        };
        let map = &record.map;
        let (kx, ky, rx, ry) = if self.wavelet.slices {
            (
                map.k().to_vec(),
                map.slice_at_r(self.wavelet.cursor[1]).unwrap_or_default(),
                map.r().to_vec(),
                map.slice_at_k(self.wavelet.cursor[0]).unwrap_or_default(),
            )
        } else {
            let (r, y) = record
                .fourier
                .as_ref()
                .and_then(|f| f.r.as_ref().zip(f.chir_mag.as_ref()))
                .map(|(r, y)| (r.as_slice().to_vec(), y.as_slice().to_vec()))
                .unwrap_or_default();
            (
                map.input_k().to_vec(),
                map.input_k()
                    .iter()
                    .zip(map.input_chi())
                    .map(|(k, y)| k.powi(map.settings().kweight as i32) * y)
                    .collect(),
                r,
                y,
            )
        };
        let mut kp: Plot = Plot::new()
            .theme(self.theme.plot_theme())
            .line(&kx, &ky)
            .color(crate::plotting::trace_color(&self.theme, 0))
            .xlabel("k (Å⁻¹)")
            .ylabel(if self.wavelet.slices {
                "|W|"
            } else {
                "k-weighted χ"
            })
            .xlim(map.settings().k_range[0], map.settings().k_range[1])
            .into();
        if !self.wavelet.slices {
            let (lo, hi) = crate::plotting::symmetric_y_limits(ky.iter().copied());
            kp = kp.ylim(lo, hi);
        }
        let rp: Plot = Plot::new()
            .theme(self.theme.plot_theme())
            .line(&rx, &ry)
            .color(crate::plotting::trace_color(&self.theme, 0))
            .xlabel("R (Å)")
            .ylabel(if self.wavelet.slices {
                "|W|"
            } else {
                "|χ(R)|"
            })
            .xlim(map.r()[0], *map.r().last().unwrap())
            .into();
        for (slot, plot) in [
            (&mut self.wavelet.plot_k, kp),
            (&mut self.wavelet.plot_r, rp),
        ] {
            if let Some(entity) = slot {
                entity.update(cx, |p, cx| p.set_plot_keep_view(plot, cx));
            } else {
                *slot = Some(plot_builder(plot).interactive().build(cx));
            }
        }
        cx.notify();
    }
    pub(crate) fn wavelet_handle_specs(&self, plot: usize) -> Vec<(HandleKey, f64)> {
        if !self.wavelet.open || self.stage != Stage::Data || self.wavelet.record.is_none() {
            return vec![];
        }
        let start = if plot == PLOT_WAVELET_K { 0 } else { 2 };
        vec![
            (HandleKey::MeasurementStart, self.wavelet.region[start]),
            (HandleKey::MeasurementEnd, self.wavelet.region[start + 1]),
        ]
    }
    pub(crate) fn drag_wavelet_boundary(
        &mut self,
        plot: usize,
        key: HandleKey,
        x: f64,
        cx: &mut Context<Self>,
    ) {
        let Some(record) = &self.wavelet.record else {
            return;
        };
        if !x.is_finite() {
            return;
        }
        let start = if plot == PLOT_WAVELET_K { 0 } else { 2 };
        let end = key == HandleKey::MeasurementEnd;
        let bounds = if start == 0 {
            record.map.settings().k_range
        } else {
            [record.map.r()[0], *record.map.r().last().unwrap()]
        };
        // Round inward at coverage limits; text and numerical bounds stay identical.
        let (lo, hi) = (
            (bounds[0] * 100.).ceil() / 100.,
            (bounds[1] * 100.).floor() / 100.,
        );
        let x = if lo < hi {
            ((x * 100.).round() / 100.).clamp(lo, hi)
        } else {
            x.clamp(bounds[0], bounds[1])
        };
        if (end && x <= self.wavelet.region[start]) || (!end && x >= self.wavelet.region[start + 1])
        {
            return;
        }
        let i = start + usize::from(end);
        self.wavelet.region[i] = x;
        self.wavelet.region_fields[i].update(cx, |f, cx| {
            f.set_text(
                if lo < hi {
                    format!("{x:.2}")
                } else {
                    x.to_string()
                },
                cx,
            )
        });
        self.update_wavelet_region();
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn display_resampling_uses_physical_centers_and_keeps_native_integral() {
        let k = (0..401).map(|i| i as f64 * 0.025).collect::<Vec<_>>();
        let chi = k
            .iter()
            .map(|v| (4.6 * v).sin() * (-((v - 6.) / 2.).powi(2)).exp())
            .collect::<Vec<_>>();
        let map = rexafs::Wavelet::new(1. ..=9.)
            .kstep(0.025)
            .radii(vec![0.1, 0.2, 1., 2., 2.3, 3., 5.])
            .calculate(&k, &chi)
            .unwrap();
        let expected = map.integral(2. ..=8., 1. ..=3.).unwrap();
        let (image, extent) = texture(&map, 0);
        assert_eq!(image.len(), 7);
        assert_eq!(image[0].len(), 401);
        let dx = (extent[1] - extent[0]) / image[0].len() as f64;
        let dy = (extent[3] - extent[2]) / image.len() as f64;
        assert!((extent[0] + dx / 2. - 1.).abs() < 1e-12);
        assert!((extent[2] + dy / 2. - 0.1).abs() < 1e-12);
        let at_min = map.slice_at_r(0.1).unwrap();
        assert_eq!(image[0][0], at_min[40]);
        for view in 0..4 {
            let _ = texture(&map, view);
        }
        assert_eq!(map.integral(2. ..=8., 1. ..=3.).unwrap(), expected);
        let (lo, hi) = scale(&map, 1);
        assert_eq!(lo, -hi);
    }
}
