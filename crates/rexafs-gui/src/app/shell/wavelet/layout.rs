//! Aligned wavelet panels and bidirectional links for their shared physical axes.
use super::*;
use ruviz::core::{InteractiveChangeSubscription, InteractivePlotSession};

const GAP: f32 = 4.;

#[derive(Clone, Copy)]
pub(super) struct JointLayout {
    pub left: u32,
    pub bottom: u32,
    pub map: (u32, u32),
}

impl JointLayout {
    pub fn new((width, height): (u32, u32)) -> Self {
        let left = ((width as f32 * 0.24)
            .clamp(130., 190.)
            .min(width as f32 * 0.4))
        .round() as u32;
        let bottom = ((height as f32 * 0.28)
            .clamp(130., 180.)
            .min(height as f32 * 0.4))
        .round() as u32;
        Self {
            left,
            bottom,
            map: (
                width.saturating_sub(left + GAP as u32).max(1),
                height.saturating_sub(bottom + GAP as u32).max(1),
            ),
        }
    }
}

/// Link only physical coordinates: k is horizontal, R is vertical. The
/// marginal spectra retain independent amplitude axes. Tokens own the links;
/// dropping them disconnects old sessions before a plot replacement.
pub(super) fn link_views(
    map: InteractivePlotSession,
    k: InteractivePlotSession,
    r: InteractivePlotSession,
) -> Vec<InteractiveChangeSubscription> {
    let sessions = [map, k, r];
    synchronize(0, &sessions);
    let updating = Arc::new(AtomicBool::new(false));
    (0..3)
        .map(|source| {
            let peers = sessions.clone();
            let updating = updating.clone();
            sessions[source].subscribe_changes(move |_| {
                if updating.swap(true, Ordering::AcqRel) {
                    return;
                }
                synchronize(source, &peers);
                updating.store(false, Ordering::Release);
            })
        })
        .collect()
}

fn synchronize(source: usize, sessions: &[InteractivePlotSession; 3]) {
    let bounds = sessions[source].view_bounds_snapshot().visible_bounds;
    for (target, session) in sessions.iter().enumerate() {
        if source == target {
            continue;
        }
        let mut next = session.view_bounds_snapshot().visible_bounds;
        if source != 2 && target != 2 {
            next.min.x = bounds.min.x;
            next.max.x = bounds.max.x;
        }
        if source != 1 && target != 1 {
            next.min.y = bounds.min.y;
            next.max.y = bounds.max.y;
        }
        session.restore_visible_bounds(next);
    }
}

impl StudioApp {
    pub(super) fn wavelet_joint_plots(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let size = self.wavelet.layout_size.unwrap_or((680, 500));
        let layout = JointLayout::new(size);
        let weak = cx.entity().downgrade();
        let measure = gpui::canvas(
            move |bounds, window, _| {
                let next = (
                    f32::from(bounds.size.width).round() as u32,
                    f32::from(bounds.size.height).round() as u32,
                );
                if next != size && next.0 > 1 && next.1 > 1 {
                    let weak = weak.clone();
                    window.on_next_frame(move |_, cx| {
                        weak.update(cx, |app, cx| {
                            if app.wavelet.layout_size != Some(next) {
                                app.wavelet.layout_size = Some(next);
                                app.rebuild_wavelet_plots(cx);
                                cx.notify();
                            }
                        })
                        .ok();
                    });
                }
            },
            |_, _, _, _| {},
        )
        .absolute()
        .inset_0();
        div()
            .flex_1()
            .min_h(px(260.))
            .min_w_0()
            .relative()
            .flex()
            .flex_col()
            .gap(px(GAP))
            .child(measure)
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .min_w_0()
                    .flex()
                    .gap(px(GAP))
                    .child(
                        div()
                            .w(px(layout.left as f32))
                            .flex_none()
                            .h_full()
                            .flex()
                            .child(self.wavelet_plot_card(PLOT_WAVELET_R, cx)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .h_full()
                            .relative()
                            .children(self.wavelet.plot_map.clone())
                            .children(self.wavelet_map_overlay()),
                    ),
            )
            .child(
                div()
                    .h(px(layout.bottom as f32))
                    .flex_none()
                    .min_w_0()
                    .flex()
                    .gap(px(GAP))
                    .child(div().w(px(layout.left as f32)).flex_none())
                    .child(self.wavelet_plot_card(PLOT_WAVELET_K, cx)),
            )
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ruviz::{
        core::{ImageTarget, ViewportPoint, ViewportRect},
        prelude::*,
    };

    fn bounds(x: [f64; 2], y: [f64; 2]) -> ViewportRect {
        ViewportRect {
            min: ViewportPoint { x: x[0], y: y[0] },
            max: ViewportPoint { x: x[1], y: y[1] },
        }
    }
    fn session(x: [f64; 2], y: [f64; 2]) -> InteractivePlotSession {
        let plot: Plot = Plot::new()
            .line(&x, &y)
            .xlim(x[0], x[1])
            .ylim(y[0], y[1])
            .into();
        plot.prepare_interactive()
    }
    fn assert_bounds(plot: &InteractivePlotSession, expected: ViewportRect) {
        let actual = plot.view_bounds_snapshot().visible_bounds;
        for (a, b) in [
            (actual.min.x, expected.min.x),
            (actual.max.x, expected.max.x),
            (actual.min.y, expected.min.y),
            (actual.max.y, expected.max.y),
        ] {
            assert!((a - b).abs() < 1e-8, "{actual:?} != {expected:?}");
        }
    }

    #[test]
    fn zoom_and_pan_link_physical_axes_without_overwriting_amplitudes() {
        let map = session([2., 12.], [0., 6.]);
        let k = session([2., 12.], [-3., 3.]);
        let r = session([5., 0.], [0., 6.]);
        let _links = link_views(map.clone(), k.clone(), r.clone());
        map.restore_visible_bounds(bounds([3., 9.], [1., 4.]));
        assert_bounds(&k, bounds([3., 9.], [-3., 3.]));
        assert_bounds(&r, bounds([5., 0.], [1., 4.]));

        k.restore_visible_bounds(bounds([4., 10.], [-1., 2.]));
        assert_bounds(&map, bounds([4., 10.], [1., 4.]));
        assert_bounds(&r, bounds([5., 0.], [1., 4.]));
        r.restore_visible_bounds(bounds([4., 1.], [2., 5.]));
        assert_bounds(&map, bounds([4., 10.], [2., 5.]));
        assert_bounds(&k, bounds([4., 10.], [-1., 2.]));

        // Resetting the map restores both shared axes, but not amplitude zoom.
        map.restore_visible_bounds(bounds([2., 12.], [0., 6.]));
        assert_bounds(&k, bounds([2., 12.], [-1., 2.]));
        assert_bounds(&r, bounds([4., 1.], [0., 6.]));
    }

    #[test]
    fn rebuilt_marginals_inherit_map_zoom_and_old_links_disconnect() {
        let map = session([2., 12.], [0., 6.]);
        let old_k = session([2., 12.], [-3., 3.]);
        let old_r = session([5., 0.], [0., 6.]);
        let links = link_views(map.clone(), old_k.clone(), old_r);
        map.restore_visible_bounds(bounds([4., 8.], [1., 3.]));
        drop(links);
        let k = session([2., 12.], [0., 0.2]);
        let r = session([0.2, 0.], [0., 6.]);
        let _new_links = link_views(map.clone(), k.clone(), r.clone());
        assert_bounds(&k, bounds([4., 8.], [0., 0.2]));
        assert_bounds(&r, bounds([0.2, 0.], [1., 3.]));
        old_k.restore_visible_bounds(bounds([3., 11.], [-1., 1.]));
        assert_bounds(&map, bounds([4., 8.], [1., 3.]));
    }

    #[test]
    fn rendered_data_rectangles_align_at_different_panel_sizes() {
        let theme = crate::theme::Theme::dark();
        for size in [(680, 500), (920, 620)] {
            let layout = JointLayout::new(size);
            let map: Plot = plots::panel_plot(&theme, layout.map, false)
                .ticks(false)
                .heatmap_with(
                    &vec![vec![0., 0.2], vec![0.1, 0.15]],
                    ruviz::plots::heatmap::HeatmapConfig::new().colorbar(true),
                )
                .into();
            let k: Plot = plots::panel_plot(&theme, (layout.map.0, layout.bottom), false)
                .line(&[2., 12.], &[-1., 1.])
                .xlabel("k (Å⁻¹)")
                .ylabel("k-weighted χ")
                .into();
            let r: Plot = plots::panel_plot(&theme, (layout.left, layout.map.1), true)
                .line(&[1., 0.], &[0., 6.])
                .xlabel("|χ(R)|")
                .ylabel("R (Å)")
                .into();
            let area = |plot: Plot, size_px| {
                let session = plot.prepare_interactive();
                session
                    .render_to_image(ImageTarget {
                        size_px,
                        scale_factor: 1.,
                        time_seconds: 0.,
                    })
                    .unwrap();
                session.viewport_snapshot().unwrap().plot_area
            };
            let (ma, ka, ra) = (
                area(map, layout.map),
                area(k, (layout.map.0, layout.bottom)),
                area(r, (layout.left, layout.map.1)),
            );
            assert!((ma.min.x - ka.min.x).abs() < 0.01);
            assert!((ma.max.x - ka.max.x).abs() < 0.01);
            assert!((ma.min.y - ra.min.y).abs() < 0.01);
            assert!((ma.max.y - ra.max.y).abs() < 0.01);
        }
    }
}
