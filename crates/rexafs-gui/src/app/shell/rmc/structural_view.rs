//! Plots use prepared structural samples; no neighbor enumeration on the UI thread.
use super::*;
use engine::structural::History;
use ruviz::plots::heatmap::{HeatmapConfig, HeatmapOrigin};

impl StudioApp {
    pub(super) fn rmc_structural_view(
        &mut self,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<gpui::Div> {
        let t = self.theme;
        let mut out = div()
            .id("rmc-structural-view")
            .overflow_y_scroll()
            .flex_1()
            .min_h_0()
            .min_w_0()
            .flex()
            .flex_col()
            .gap_2()
            .p_3();
        let Some(progress) = &self.rmc.live else {
            return out.child(hint(
                &t,
                "Structural samples appear after the initial scattering calculation.",
            ));
        };
        if let Some(error) = &progress.structural_error {
            out = out.child(
                div()
                    .text_color(t.warn)
                    .child(format!("Structural diagnostics: {error}")),
            );
        }
        let Some(history) = progress.structural.clone() else {
            return out.child(note(&t, "No structural history is available. Historical checkpoints retain their original sampling settings; start a new run to track structural evolution."));
        };
        self.rmc.structural_pair = self
            .rmc
            .structural_pair
            .min(history.current.pairs.len().saturating_sub(1));
        let pair = self.rmc.structural_pair;
        let unit = if history.settings.generations {
            "generation"
        } else {
            "attempt"
        };
        let symbol = |z| Element::from_z(z).map_or("?", |e| e.symbol);
        let mut pairs = div().flex().flex_wrap().gap_2();
        for (i, p) in history.current.pairs.iter().enumerate() {
            pairs = pairs.child(
                chip(
                    &t,
                    ("structural-pair", i),
                    format!("{}–{}", symbol(history.center_element), symbol(p.neighbor)),
                    i == pair,
                )
                .on_click(cx.listener(move |app, _, _, cx| {
                    app.rmc.structural_pair = i;
                    cx.notify();
                })),
            );
        }
        out = out.child(pairs);
        let mut views = div().flex().flex_wrap().gap_2();
        for (i, label) in [
            "Distributions",
            "Distance vs step",
            "Coordination",
            "Mean distance",
            "Variance",
        ]
        .into_iter()
        .enumerate()
        {
            views = views.child(
                chip(
                    &t,
                    ("structural-view", i),
                    label,
                    self.rmc.structural_view == i,
                )
                .on_click(cx.listener(move |app, _, _, cx| {
                    app.rmc.structural_view = i;
                    cx.notify();
                })),
            );
        }
        out = out.child(views);
        let radial = &history.current.pairs[pair].radial;
        out = out.child(note(
            &t,
            format!(
                "{} · {} · interval [{:.2}, {:.2}) Å",
                if radial.g_r.is_some() {
                    "g(r): species-density and shell-volume normalized"
                } else {
                    "Finite cluster: neighbors per absorber per bin"
                },
                plural(history.centers.len(), "center"),
                history.settings.range[0],
                history.settings.range[1]
            ),
        ));
        out = out.child(note(
            &t,
            format!(
                "Sampled at {unit} {} · {} retained · {} skipped · {} older samples removed",
                history.current.step,
                history.samples.len(),
                history.skipped_samples,
                history.discarded_samples
            ),
        ));
        if let Some(radius) = radial
            .independent_radius
            .filter(|&radius| history.settings.range[1] > radius)
        {
            out = out.child(note(&t, format!("Above {radius:.2} Å, distances include repeated-cell correlations. The cell does not supply independent bulk information at those radii.")));
        }
        out = out.child(note(&t, "Optimizer steps are not physical time or uncertainty samples. Moments cover the selected distance interval."));
        if history.settings.generations {
            out = out.child(note(
                &t,
                "For EA, Current and Best both show the current population's best individual.",
            ));
        }
        if radial.neighbors.mean.is_none() {
            out = out.child(note(&t, "The current sample has no neighbors in this interval; its mean and variance are undefined."));
        }
        if self.rmc.structural_view == 0 {
            out = out.child(note(&t, "Overlays: Initial · Current · Best"));
        } else if self.rmc.structural_view == 1 {
            out = out.child(note(&t, "Each colored strip is a saved sample at its actual step. Blank gaps have no sample; values are not interpolated."));
        }
        let key = (Arc::as_ptr(&history) as usize, pair);
        if self.rmc.structural_key != Some(key) || self.rmc.structural_plots.len() != 5 {
            self.rmc.structural_plots = plots(&history, pair, t)
                .into_iter()
                .map(|plot| plot_builder(plot).interactive().build(cx))
                .collect();
            self.rmc.structural_key = Some(key);
            self.rmc.structural_source = Some(history.clone());
        }
        if let Some(plot) = self.rmc.structural_plots.get(self.rmc.structural_view) {
            out = out.child(div().flex_1().min_h(px(220.)).min_w_0().child(plot.clone()));
        }
        out
    }
}

fn note(t: &crate::theme::Theme, text: impl Into<gpui::SharedString>) -> gpui::Div {
    hint(t, text)
        .w_full()
        .min_w_0()
        .flex_none()
        .whitespace_normal()
}

fn plots(history: &History, pair: usize, theme: crate::theme::Theme) -> Vec<Plot> {
    let settings = &history.settings;
    let step_label = if settings.generations {
        "Generation"
    } else {
        "Attempt"
    };
    let edges = settings.edges();
    let centers: Vec<_> = edges.windows(2).map(|r| (r[0] + r[1]) / 2.).collect();
    let ylabel = if history.current.pairs[pair].radial.g_r.is_some() {
        "g(r) (dimensionless)"
    } else {
        "Neighbors per absorber per bin"
    };
    let mut overlay = Plot::new()
        .theme(theme.plot_theme())
        .xlabel("Distance r (Å)")
        .ylabel(ylabel);
    for (label, sample) in [
        ("Initial", &history.initial),
        ("Current", &history.current),
        ("Best", &history.best),
    ] {
        overlay = overlay
            .line(&centers, &sample.pairs[pair].values().to_vec())
            .label(label)
            .into();
    }
    let mut heatmap = Plot::new()
        .theme(theme.plot_theme())
        .xlabel("Distance r (Å)")
        .ylabel(step_label);
    let maximum = history
        .samples
        .iter()
        .flat_map(|s| s.pairs[pair].values())
        .copied()
        .fold(0., f64::max)
        .max(1e-12);
    // Separate single-row layers preserve irregular coordinates. Width is limited
    // to the configured stride and neighboring half-distances, leaving skipped
    // intervals blank rather than stretching a sample across missing attempts.
    for (i, sample) in history.samples.iter().enumerate() {
        let step = sample.step as f64;
        let half = settings.stride as f64 / 2.;
        let before = i
            .checked_sub(1)
            .map(|j| (step - history.samples[j].step as f64) / 2.)
            .unwrap_or(half)
            .min(half);
        let after = history
            .samples
            .get(i + 1)
            .map(|s| (s.step as f64 - step) / 2.)
            .unwrap_or(half)
            .min(half);
        heatmap = heatmap
            .heatmap_with(
                &vec![sample.pairs[pair].values().to_vec()],
                HeatmapConfig::new()
                    .origin(HeatmapOrigin::Lower)
                    .vmin(0.)
                    .vmax(maximum)
                    .colorbar(i == history.samples.len() - 1)
                    .colorbar_label(ylabel)
                    .extent(
                        settings.range[0],
                        settings.range[1],
                        step - before,
                        step + after,
                    ),
            )
            .into();
    }
    let mut result = vec![overlay.legend_best(), heatmap];
    for (metric, label) in [
        "Coordination (neighbors per absorber)",
        "Mean distance (Å)",
        "Distance variance (Å²)",
    ]
    .into_iter()
    .enumerate()
    {
        let points: Vec<_> = history
            .samples
            .iter()
            .filter_map(|s| {
                let n = &s.pairs[pair].radial.neighbors;
                let value = match metric {
                    0 => Some(n.coordination),
                    1 => n.mean,
                    _ => n.variance,
                }?;
                Some((s.step as f64, value))
            })
            .collect();
        let (x, y): (Vec<_>, Vec<_>) = points.into_iter().unzip();
        let mut plot = Plot::new()
            .theme(theme.plot_theme())
            .xlabel(step_label)
            .ylabel(label);
        if !x.is_empty() {
            plot = plot.scatter(&x, &y).into();
        }
        result.push(plot);
    }
    result
}
