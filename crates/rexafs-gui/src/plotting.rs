//! Builders turning a processed `XASSpectrum` into ruviz `Plot`s for the four
//! Explore quadrants: mu(E), normalized mu(E), k-weighted chi(k), |chi(R)|.
//!
//! ruviz 0.5 retains native rotated y labels and adds the interactive APIs used
//! by the operando plots. Card headers remain plot titles rather than serving as
//! a workaround for broken vertical text.

use rexafs::prelude::BackgroundMethod;
use rexafs::prelude::XASSpectrum;
use ruviz::core::LegendPosition;
use ruviz::data::{BatchUpdate, Observable};
use ruviz::plots::heatmap::{HeatmapConfig, HeatmapOrigin};
use ruviz::prelude::Plot;
use ruviz::render::{Color, LineStyle};

use crate::theme::Theme;

mod alignment;
pub(crate) mod analysis;

fn vecs(v: &nalgebra::DVector<f64>) -> Vec<f64> {
    v.iter().copied().collect()
}

/// One spectrum in a comparison overlay.
pub struct QuadTrace {
    pub color_index: usize,
    pub color: Option<Color>,
    pub label: String,
    pub sp: std::sync::Arc<XASSpectrum>,
    pub active: bool,
}

pub(crate) fn mixed_kweights(traces: &[QuadTrace]) -> bool {
    // Calculated absorption groups have no Fourier transform. An absent
    // transform is not a different Fourier weight from the reference spectra.
    let mut weights = traces.iter().filter_map(|t| t.sp.kweight());
    let Some(first) = weights.next() else {
        return false;
    };
    weights.any(|weight| weight != first)
}

pub(crate) fn ft_trace_label(trace: &QuadTrace) -> String {
    format!(
        "FT kw={} · {}",
        trace.sp.kweight().copied().unwrap_or(2.0),
        trace.label
    )
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TraceLayout {
    Overlay,
    Waterfall,
}

/// Explore-view display options (see doc/gui-ux-design.md).
#[derive(Clone, Copy)]
pub struct ViewOptions {
    /// Preview samples at most 12 traces; false plots the full comparison set.
    pub sample_overlay: bool,
    /// Continuous Viridis colors in comparison order, without changing group colors.
    pub gradient: bool,
    /// Display-only energy bounds relative to the current E0; None shows all data.
    pub energy_view_range: Option<(f64, f64)>,
    pub layout: TraceLayout,
    /// Waterfall offset as a fraction of the first trace's peak-to-peak.
    pub offset_frac: f64,
    pub legend: bool,
    pub grid: bool,
    /// Diagnostics on the mu(E) quadrant (drawn for the active trace only).
    pub show_pre: bool,
    pub show_post: bool,
    pub show_e0: bool,
    pub show_ranges: bool,
    /// Normalized quadrant shows flattened (true) or plain normalized mu(E).
    pub flat: bool,
    /// chi(k) quadrant: FFT k-range lines and the FT window curve.
    pub show_krange: bool,
    pub show_kwin: bool,
    /// |chi(R)| quadrant: also plot Re[chi(R)] of the active trace.
    pub show_re: bool,
    pub show_im: bool,
    /// AUTOBK background spline over mu(E) (Background stage).
    pub show_bkg: bool,
    /// Overlay the scaled derivative dμ/dE on the normalized plot (Athena's
    /// "normalized + scaled derivative").
    pub show_deriv: bool,
}

impl Default for ViewOptions {
    fn default() -> Self {
        Self {
            sample_overlay: false,
            gradient: false,
            energy_view_range: None,
            layout: TraceLayout::Overlay,
            offset_frac: 0.6,
            legend: true,
            grid: true,
            show_pre: false,
            show_post: false,
            show_e0: false,
            show_ranges: false,
            flat: true,
            show_krange: false,
            show_kwin: false,
            show_re: false,
            show_im: false,
            show_bkg: false,
            show_deriv: false,
        }
    }
}

/// Legends beyond this many traces are clutter.
const MAX_LEGEND_TRACES: usize = 8;

/// Stable per-trace color: keyed by the trace's position in the sorted
/// selection so draw order (active drawn last) never reshuffles colors.
pub fn trace_color(theme: &Theme, index: usize) -> Color {
    theme.plot_theme().get_color(index)
}

/// The same stable trace color as a gpui color, for the shared legend strip.
pub fn trace_rgba(theme: &Theme, index: usize) -> gpui::Rgba {
    let c = trace_color(theme, index);
    gpui::Rgba {
        r: c.r as f32 / 255.0,
        g: c.g as f32 / 255.0,
        b: c.b as f32 / 255.0,
        a: c.a as f32 / 255.0,
    }
}

/// Middle-truncate `name` to at most `max` characters ("frame_0…003.dat").
pub fn middle_truncate(name: &str, max: usize) -> String {
    let count = name.chars().count();
    if count <= max || max < 2 {
        return name.to_string();
    }
    let head = (max - 1) / 2;
    let tail = max - 1 - head;
    let front: String = name.chars().take(head).collect();
    let back: String = name.chars().skip(count - tail).collect();
    format!("{front}…{back}")
}

/// Axis label for the wavenumber axis.
pub const K_AXIS: &str = "k (Å⁻¹)";
/// Axis label for the radial-distance axis.
pub const R_AXIS: &str = "R (Å)";

/// Label for |chi(R)|, whose unit depends on the k-weight used for the
/// transform: k-weight n gives Å^-(n+1). Hardcoding Å⁻³ was only correct for
/// the default k-weight of 2.
pub fn chir_label(kw: f64) -> String {
    let n = kw.round();
    if (kw - n).abs() > 1e-9 {
        return format!("|χ(R)| (Å^-{:.1})", kw + 1.0);
    }
    match n as i64 + 1 {
        1 => "|χ(R)| (Å⁻¹)".to_string(),
        2 => "|χ(R)| (Å⁻²)".to_string(),
        3 => "|χ(R)| (Å⁻³)".to_string(),
        4 => "|χ(R)| (Å⁻⁴)".to_string(),
        m => format!("|χ(R)| (Å^-{m})"),
    }
}

/// Symmetric display limits include every finite amplitude, with 5% headroom.
/// This changes only the view: outliers and the original arrays are retained.
/// Empty or all-zero signals use ±1 so the axis remains well-defined.
pub(crate) fn symmetric_y_limits(values: impl IntoIterator<Item = f64>) -> (f64, f64) {
    let peak = values
        .into_iter()
        .filter(|v| v.is_finite())
        .fold(0.0_f64, |peak, value| peak.max(value.abs()));
    let limit = if peak == 0. {
        1.
    } else {
        (peak * 1.05).min(f64::MAX)
    };
    (-limit, limit)
}

pub(crate) fn centered_y(plot: Plot, values: impl IntoIterator<Item = f64>) -> Plot {
    let (lo, hi) = symmetric_y_limits(values);
    plot.ylim(lo, hi)
        .hline_styled(0., Color::from_gray(120), 0.8, LineStyle::Dashed)
}

/// Label for a k-weighted chi(k) quantity.
pub fn chik_label(kw: f64) -> String {
    let n = kw.round();
    if (kw - n).abs() > 1e-9 {
        return format!("k^{kw:.1} χ(k)");
    }
    match n as i64 {
        0 => "χ(k)".to_string(),
        1 => "k·χ(k)".to_string(),
        2 => "k²χ(k)".to_string(),
        3 => "k³χ(k)".to_string(),
        _ => format!("k^{n:.0} χ(k)"),
    }
}

/// Identity of one series inside a quadrant. Together with its style it
/// forms the quadrant *structure*; the values behind it are reactive.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum SeriesKey {
    /// Data trace `i` (position in the compare set).
    Trace(usize),
    PreEdge,
    PostEdge,
    /// MBACK's complete fitted atomic model, converted to input mu units.
    MbackFit,
    Bkg,
    Deriv,
    Kwin,
    Re,
    Im,
    /// k-weighted chi(k) drawn under chi(q).
    ChiKOnQ,
}

/// One series: identity + style (structure) and its current values.
#[derive(Clone)]
pub struct SeriesSpec {
    pub key: SeriesKey,
    pub x: Vec<f64>,
    pub y: Vec<f64>,
    pub width: f32,
    pub color: Color,
    pub dashed: bool,
    pub label: Option<String>,
}

/// The observables behind one series of a live quadrant.
#[derive(Clone)]
pub struct SeriesSource {
    pub x: Observable<Vec<f64>>,
    pub y: Observable<Vec<f64>>,
}

/// Everything a quadrant shows, split into structure (labels, series
/// identities and styles, guide lines) and reactive values.
#[derive(Clone)]
pub struct QuadrantSpec {
    pub title: String,
    pub xlabel: String,
    pub ylabel: String,
    pub series: Vec<SeriesSpec>,
    /// Static vertical guide lines: (x, color, width, dashed).
    pub vlines: Vec<(f64, Color, f32, bool)>,
    pub legend_columns: Option<usize>,
    pub grid: bool,
    pub xlim: Option<(f64, f64)>,
    pub ylim: Option<(f64, f64)>,
}

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub(crate) struct PlotCoverage {
    plotted: usize,
    total: usize,
    sampled_out: usize,
    unavailable: usize,
    incompatible: usize,
}

impl PlotCoverage {
    pub fn new(total: usize, sampled: usize, loaded: usize, plotted: usize) -> Self {
        Self {
            plotted,
            total,
            sampled_out: total.saturating_sub(sampled),
            unavailable: sampled.saturating_sub(loaded),
            incompatible: loaded.saturating_sub(plotted),
        }
    }

    pub fn disclosure(self) -> Option<String> {
        if self.plotted == self.total {
            return None;
        }
        let mut text = format!(
            "{} of {} {} plotted",
            self.plotted,
            self.total,
            crate::text::noun_for(self.total, "spectrum")
        );
        for (count, reason) in [
            (self.sampled_out, "omitted by sampling"),
            (self.incompatible, "without data for this plot"),
            (self.unavailable, "failed or not loaded"),
        ] {
            if count > 0 {
                text.push_str(&format!(" · {count} {reason}"));
            }
        }
        Some(text)
    }
}

impl QuadrantSpec {
    /// Keep only appearance metadata; live arrays remain in their existing observables.
    pub(crate) fn export_template(&self) -> Self {
        Self {
            title: self.title.clone(),
            xlabel: self.xlabel.clone(),
            ylabel: self.ylabel.clone(),
            series: self
                .series
                .iter()
                .map(|s| SeriesSpec {
                    key: s.key,
                    x: Vec::new(),
                    y: Vec::new(),
                    width: s.width,
                    color: s.color,
                    dashed: s.dashed,
                    label: s.label.clone(),
                })
                .collect(),
            vlines: self.vlines.clone(),
            legend_columns: self.legend_columns,
            grid: self.grid,
            xlim: self.xlim,
            ylim: self.ylim,
        }
    }

    pub(crate) fn coverage(&self, total: usize, sampled: usize, loaded: usize) -> PlotCoverage {
        let plotted = self
            .series
            .iter()
            .filter_map(|s| match s.key {
                SeriesKey::Trace(i) if !s.x.is_empty() && !s.y.is_empty() => Some(i),
                _ => None,
            })
            .collect::<std::collections::BTreeSet<_>>()
            .len();
        PlotCoverage::new(total, sampled, loaded, plotted)
    }

    /// Hash of the structure only (never the values). Two specs with equal
    /// keys can share one ruviz session: a refresh just replaces the
    /// observables. `salt` folds in host-side structure (figure size, theme).
    pub fn structure_key(&self, salt: u64) -> u64 {
        use std::hash::{DefaultHasher, Hash, Hasher};
        let mut h = DefaultHasher::new();
        salt.hash(&mut h);
        self.title.hash(&mut h);
        self.xlabel.hash(&mut h);
        self.ylabel.hash(&mut h);
        self.legend_columns.hash(&mut h);
        self.grid.hash(&mut h);
        self.xlim
            .map(|(lo, hi)| (lo.to_bits(), hi.to_bits()))
            .hash(&mut h);
        self.ylim
            .map(|(lo, hi)| (lo.to_bits(), hi.to_bits()))
            .hash(&mut h);
        for s in &self.series {
            s.key.hash(&mut h);
            s.width.to_bits().hash(&mut h);
            (s.color.r, s.color.g, s.color.b, s.color.a).hash(&mut h);
            s.dashed.hash(&mut h);
            s.label.hash(&mut h);
        }
        for (x, c, w, d) in &self.vlines {
            x.to_bits().hash(&mut h);
            (c.r, c.g, c.b, c.a).hash(&mut h);
            w.to_bits().hash(&mut h);
            d.hash(&mut h);
        }
        h.finish()
    }

    /// A ruviz plot whose series read from fresh observables (returned in
    /// series order, for later [`QuadrantSpec::apply`] calls).
    pub fn to_plot(&self, theme: &Theme) -> (Plot, Vec<SeriesSource>) {
        let mut plot = Plot::new()
            .theme(theme.plot_theme())
            .grid(self.grid)
            .xlabel(&self.xlabel)
            .ylabel(&self.ylabel);
        let mut sources = Vec::with_capacity(self.series.len());
        if let Some((lo, hi)) = self.xlim {
            plot = plot.xlim(lo, hi);
        }
        if let Some((lo, hi)) = self.ylim {
            plot =
                plot.ylim(lo, hi)
                    .hline_styled(0., Color::from_gray(120), 0.8, LineStyle::Dashed);
        }
        for s in &self.series {
            let src = SeriesSource {
                x: Observable::new(s.x.clone()),
                y: Observable::new(s.y.clone()),
            };
            let mut sb = plot
                .line_source(src.x.clone(), src.y.clone())
                .line_width(s.width)
                .color(s.color);
            if s.dashed {
                sb = sb.line_style(LineStyle::Dashed);
            }
            if let Some(label) = &s.label {
                sb = sb.label(label.clone());
            }
            plot = sb.into();
            sources.push(src);
        }
        for &(x, color, width, dashed) in &self.vlines {
            let style = if dashed {
                LineStyle::Dashed
            } else {
                LineStyle::Solid
            };
            plot = plot.vline_styled(x, color, width, style);
        }
        if let Some(columns) = self.legend_columns {
            plot = plot
                .legend_position(LegendPosition::OutsideLower)
                .legend_columns(columns);
        }
        (plot, sources)
    }

    /// Push this spec's values into an existing session's observables. All
    /// notifications are deferred to one batch so a render never sees a new
    /// x with an old y.
    /// Unchanged vectors are left alone, so a refresh that lands on the
    /// same values (a cache hit while dragging back and forth) costs a
    /// comparison and no re-raster.
    pub fn apply(self, sources: &[SeriesSource]) {
        let mut batch = BatchUpdate::new();
        for src in sources {
            batch.add(&src.x);
            batch.add(&src.y);
        }
        for (s, src) in self.series.into_iter().zip(sources) {
            if *src.x.read() != s.x {
                src.x.set(s.x);
            }
            if *src.y.read() != s.y {
                src.y.set(s.y);
            }
        }
        drop(batch);
    }
}

const TREND: Color = Color {
    r: 150,
    g: 150,
    b: 150,
    a: 255,
};
const GUIDE: Color = Color {
    r: 140,
    g: 140,
    b: 140,
    a: 255,
};
const E0_COLOR: Color = Color::ORANGE;
const PRE_COLOR: Color = Color {
    r: 90,
    g: 140,
    b: 200,
    a: 255,
};
const NORM_COLOR: Color = Color {
    r: 90,
    g: 180,
    b: 120,
    a: 255,
};
const BKG_COLOR: Color = Color {
    r: 232,
    g: 121,
    b: 47,
    a: 255,
};

/// One quadrant's data traces from per-trace (x, y) extractions. Inactive
/// traces draw first (thin), the active trace last (thick, on top); every
/// trace keeps the colour of its original position so the reordering never
/// recolors anything. Waterfall mode offsets successive traces by
/// `offset_frac` x the first trace's range. Returns the spec plus the
/// waterfall shift of the active (or first) trace so diagnostic overlays can
/// be aligned to the curve they annotate.
fn build_multi(
    traces: &[QuadTrace],
    view: &ViewOptions,
    theme: &Theme,
    in_plot_legend: bool,
    (title, xlabel, ylabel): (&str, &str, &str),
    extract: impl Fn(&XASSpectrum) -> Option<(Vec<f64>, Vec<f64>)>,
) -> (QuadrantSpec, f64) {
    let mut series: Vec<(usize, &QuadTrace, Vec<f64>, Vec<f64>)> = traces
        .iter()
        .enumerate()
        .filter_map(|(i, t)| extract(&t.sp).map(|(x, y)| (i, t, x, y)))
        .collect();

    let offset = if view.layout == TraceLayout::Waterfall {
        let span = series
            .first()
            .map(|(_, _, _, y)| {
                let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
                for v in y {
                    lo = lo.min(*v);
                    hi = hi.max(*v);
                }
                (hi - lo).abs()
            })
            .filter(|s| s.is_finite() && *s > 0.0)
            .unwrap_or(1.0);
        span * view.offset_frac
    } else {
        0.0
    };
    if offset != 0.0 {
        for (i, _, _, y) in series.iter_mut() {
            let shift = *i as f64 * offset;
            for v in y.iter_mut() {
                *v += shift;
            }
        }
    }
    let active_shift = offset * traces.iter().position(|t| t.active).unwrap_or(0) as f64;
    // Active trace drawn last so it sits on top.
    series.sort_by_key(|(_, t, _, _)| t.active);

    let n = series.len();
    // In the grid the shared GPUI legend strip identifies traces; in-plot
    // legends only when a quadrant is maximized.
    let with_legend = in_plot_legend && view.legend && n > 1 && n <= MAX_LEGEND_TRACES;
    let specs = series
        .into_iter()
        .map(|(i, trace, x, y)| SeriesSpec {
            key: SeriesKey::Trace(i),
            x,
            y,
            width: if trace.active && n > 1 { 2.2 } else { 1.4 },
            color: trace
                .color
                .unwrap_or_else(|| trace_color(theme, trace.color_index)),
            dashed: false,
            label: with_legend.then(|| middle_truncate(&trace.label, 24)),
        })
        .collect();
    (
        QuadrantSpec {
            xlim: None,
            ylim: None,
            title: title.to_string(),
            xlabel: xlabel.to_string(),
            ylabel: ylabel.to_string(),
            series: specs,
            vlines: Vec::new(),
            legend_columns: with_legend.then_some(n.min(4)),
            grid: view.grid,
        },
        active_shift,
    )
}

fn dashed(key: SeriesKey, x: Vec<f64>, y: Vec<f64>, color: Color, label: &str) -> SeriesSpec {
    SeriesSpec {
        key,
        x,
        y,
        width: 1.0,
        color,
        dashed: true,
        label: Some(label.to_string()),
    }
}

/// Normalization-check overlays for the active trace on the mu(E) plot:
/// the selected method's fit (pre/post-edge trendlines or the complete MBACK model),
/// the AUTOBK spline, the E0 line, and
/// the fit-window lines (pre-edge range muted, norm range in a second hue;
/// values are stored relative to E0). `shift` is the active trace's
/// waterfall offset so the trendlines sit on the curve they belong to.
fn add_mu_diagnostics(spec: &mut QuadrantSpec, sp: &XASSpectrum, view: &ViewOptions, shift: f64) {
    let mback = match sp.normalization.as_ref() {
        Some(rexafs::NormalizationMethod::MBack(m)) => m.result.as_deref(),
        _ => None,
    };
    if let Some(result) = mback {
        if view.show_pre
            && view.show_post
            && let Some(energy) = &sp.energy
        {
            // MBACK fits s * mu(E) to f₂(E) + B(E). Show the complete fitted
            // model in the input absorption units, not just its background.
            // Keep the auxiliary MBACK baselines internal; display pre/post
            // lines only when the polynomial method is selected.
            spec.series.push(SeriesSpec {
                key: SeriesKey::MbackFit,
                x: vecs(energy),
                y: result
                    .f2
                    .iter()
                    .zip(&result.background)
                    .map(|(f2, b)| (f2 + b) / result.scale + shift)
                    .collect(),
                width: 1.5,
                color: BKG_COLOR,
                dashed: false,
                label: Some("MBACK fit".into()),
            });
        }
    } else {
        for (show, key, label, values) in [
            (view.show_pre, SeriesKey::PreEdge, "pre-edge", sp.pre_edge()),
            (
                view.show_post,
                SeriesKey::PostEdge,
                "post-edge",
                sp.post_edge(),
            ),
        ] {
            if show && let (Some(energy), Some(values)) = (&sp.energy, values) {
                let y = values.iter().map(|v| v + shift).collect();
                spec.series.push(dashed(key, vecs(energy), y, TREND, label));
            }
        }
    }
    if view.show_bkg
        && let (Some(energy), Some(BackgroundMethod::AUTOBK(autobk))) =
            (sp.energy.as_ref(), sp.background.as_ref())
        && let Some(bkg) = autobk.get_bkg()
    {
        let x = vecs(energy);
        let n = x.len().min(bkg.len());
        let y: Vec<f64> = bkg.iter().take(n).map(|v| v + shift).collect();
        spec.series.push(SeriesSpec {
            key: SeriesKey::Bkg,
            x: x[..n].to_vec(),
            y,
            width: 1.5,
            color: BKG_COLOR,
            dashed: false,
            label: Some("background μ₀(E)".to_string()),
        });
    }
    let e0 = sp.e0();
    if view.show_e0
        && let Some(e0) = e0
    {
        spec.vlines.push((e0, E0_COLOR, 1.2, true));
    }
    if view.show_ranges
        && let (Some(e0), Some(r)) = (e0, crate::params::normalization_ranges(sp))
    {
        for (rel, color) in [
            (r[0], PRE_COLOR),
            (r[1], PRE_COLOR),
            (r[2], NORM_COLOR),
            (r[3], NORM_COLOR),
        ] {
            spec.vlines.push((e0 + rel, color, 1.0, true));
        }
    }
}

/// Specs for all five Explore quadrants (mu(E), normalized mu(E), k-weighted
/// chi(k), |chi(R)|, chi(q)) for a set of traces. `in_plot_legend` opts into
/// ruviz legends (maximized quadrant); the grid uses the shared GPUI strip.
pub(crate) fn quantity_quadrant_specs(
    traces: &[QuadTrace],
    view: &ViewOptions,
    theme: &Theme,
    in_plot_legend: bool,
    quantity: crate::params::Quantity,
) -> [QuadrantSpec; 5] {
    let mut adjusted = *view;
    if matches!(
        quantity,
        crate::params::Quantity::FlattenedMu | crate::params::Quantity::FlattenedDifference
    ) {
        adjusted.flat = true;
    }
    if quantity == crate::params::Quantity::NormalizedMu {
        adjusted.flat = false;
    }
    let mut specs = build_quadrant_specs(traces, &adjusted, theme, in_plot_legend);
    if matches!(
        quantity,
        crate::params::Quantity::NormalizedMu | crate::params::Quantity::NormalizedDifference
    ) {
        specs[0].title = quantity.label().into();
        specs[0].ylabel = format!("{} (dimensionless)", quantity.label());
    }
    specs
}

pub fn build_quadrant_specs(
    traces: &[QuadTrace],
    view: &ViewOptions,
    theme: &Theme,
    in_plot_legend: bool,
) -> [QuadrantSpec; 5] {
    let kw = traces
        .iter()
        .find(|t| t.active)
        .or_else(|| traces.first())
        .and_then(|t| t.sp.kweight().copied())
        .unwrap_or(2.0);
    let mixed_weights = mixed_kweights(traces);
    let weighted_traces: Vec<QuadTrace> = if mixed_weights {
        traces
            .iter()
            .map(|trace| QuadTrace {
                label: ft_trace_label(trace),
                sp: trace.sp.clone(),
                active: trace.active,
                color_index: trace.color_index,
                color: trace.color,
            })
            .collect()
    } else {
        Vec::new()
    };
    let ft_traces = if mixed_weights {
        &weighted_traces
    } else {
        traces
    };

    let (mut mu_e, mu_shift) = build_multi(
        traces,
        view,
        theme,
        in_plot_legend,
        ("μ(E)", "Energy (eV)", "μ(E)"),
        |sp| Some((sp.energy.as_ref().map(vecs)?, sp.mu.as_ref().map(vecs)?)),
    );
    let flat = view.flat;
    let norm_label = if flat {
        "flat μ(E)"
    } else {
        "normalized μ(E)"
    };
    let (mut norm, _) = build_multi(
        traces,
        view,
        theme,
        in_plot_legend,
        (norm_label, "Energy (eV)", norm_label),
        move |sp| {
            let y = if flat {
                sp.flat().or_else(|| sp.norm())
            } else {
                sp.norm().or_else(|| sp.flat())
            };
            Some((sp.energy.as_ref().map(vecs)?, y.map(|v| vecs(&v))?))
        },
    );
    let chik_label = chik_label(kw);
    let (mut chi_k, chik_shift) = build_multi(
        traces,
        view,
        theme,
        in_plot_legend,
        (&chik_label, K_AXIS, &chik_label),
        |sp| {
            // One display weight makes the common axis true for every curve.
            // Each spectrum retains its own processing and Fourier transform.
            let k = sp.k()?;
            let chi = sp.chi()?;
            (k.len() == chi.len()).then(|| {
                (
                    k.to_vec(),
                    k.iter().zip(chi).map(|(k, chi)| chi * k.powf(kw)).collect(),
                )
            })
        },
    );
    let chir_title = match (view.show_re, view.show_im) {
        (true, true) => "|χ(R)| + Re + Im",
        (true, false) => "|χ(R)| + Re",
        (false, true) => "|χ(R)| + Im",
        _ => "|χ(R)|",
    };
    let (mut chi_r, chir_shift) = build_multi(
        ft_traces,
        view,
        theme,
        in_plot_legend,
        (
            chir_title,
            R_AXIS,
            &if mixed_weights {
                "|χ(R)| (mixed k weights)".into()
            } else {
                chir_label(kw)
            },
        ),
        |sp| {
            let r = sp.r().map(|v| vecs(&v))?;
            let m = sp.chir_mag().map(|v| vecs(&v))?;
            let n = r.len().min(m.len());
            Some((r[..n].to_vec(), m[..n].to_vec()))
        },
    );

    if let Some(active) = traces.iter().find(|t| t.active).or_else(|| traces.first()) {
        add_mu_diagnostics(&mut mu_e, &active.sp, view, mu_shift);
        if view.show_e0
            && let Some(e0) = active.sp.e0()
        {
            norm.vlines.push((e0, E0_COLOR, 1.2, true));
        }
        if view.show_deriv
            && let (Some(e), Some(n)) = (
                active.sp.energy.as_ref(),
                active.sp.norm().or_else(|| active.sp.flat()),
            )
        {
            // dμ/dE scaled so its peak reaches half the edge, as Athena does.
            let e = vecs(e);
            let n = vecs(&n);
            let m = e.len().min(n.len());
            let d: Vec<f64> = (0..m)
                .map(|i| {
                    let a = i.saturating_sub(1);
                    let b = (i + 1).min(m - 1);
                    let de = e[b] - e[a];
                    if de > 0.0 { (n[b] - n[a]) / de } else { 0.0 }
                })
                .collect();
            let peak = d.iter().fold(0.0f64, |acc, v| acc.max(v.abs())).max(1e-12);
            let y: Vec<f64> = d.iter().map(|v| v / peak * 0.5).collect();
            norm.series.push(dashed(
                SeriesKey::Deriv,
                e[..m].to_vec(),
                y,
                GUIDE,
                "dμ/dE (scaled)",
            ));
        }
        // FT window diagnostics on chi(k), scaled to the data amplitude and
        // shifted onto the active trace in waterfall mode.
        if view.show_kwin
            && let (Some(k), Some(kwin), Some(chi)) = (
                active.sp.kwin_k(),
                active.sp.kwin(),
                active.sp.chi_kweighted(),
            )
        {
            let peak = chi.iter().fold(0.0f64, |m, v| m.max(v.abs())).max(1e-12);
            let x = vecs(&k);
            let n = x.len().min(kwin.len());
            let y: Vec<f64> = kwin.iter().take(n).map(|w| w * peak + chik_shift).collect();
            chi_k
                .series
                .push(dashed(SeriesKey::Kwin, x[..n].to_vec(), y, TREND, "window"));
        }
        if view.show_krange
            && let Some(xftf) = active.sp.xftf.as_ref()
        {
            for v in [xftf.kmin, xftf.kmax].into_iter().flatten() {
                chi_k.vlines.push((v, PRE_COLOR, 1.0, true));
            }
        }
        // Re part of chi(R) for phase-agreement checks (doc: "|χ(R)| (+Re
        // part toggle)"), aligned to the active trace's waterfall offset.
        if view.show_re
            && let (Some(r), Some(re)) = (active.sp.r(), active.sp.chir_real())
        {
            let r = vecs(&r);
            let n = r.len().min(re.len());
            let y: Vec<f64> = re.iter().take(n).map(|v| v + chir_shift).collect();
            chi_r
                .series
                .push(dashed(SeriesKey::Re, r[..n].to_vec(), y, TREND, "Re"));
        }
        if view.show_im
            && let (Some(r), Some(im)) = (active.sp.r(), active.sp.chir_imag())
        {
            let n = r.len().min(im.len());
            let y = im.iter().take(n).map(|v| v + chir_shift).collect();
            chi_r.series.push(dashed(
                SeriesKey::Im,
                r.iter().take(n).copied().collect(),
                y,
                FIT_COLOR,
                "Im",
            ));
        }
    }
    let (mut chi_q, chiq_shift) = build_multi(
        ft_traces,
        view,
        theme,
        in_plot_legend,
        (
            "χ(q)",
            "q (Å⁻¹)",
            if mixed_weights {
                "χ(q) (mixed k weights)"
            } else {
                "χ(q)"
            },
        ),
        |sp| {
            let q = sp.q().map(|v| vecs(&v))?;
            let c = sp.chiq().map(|v| vecs(&v))?;
            let n = q.len().min(c.len());
            Some((q[..n].to_vec(), c[..n].to_vec()))
        },
    );
    if let Some(active) = traces.iter().find(|t| t.active).or_else(|| traces.first())
        && let (Some(k), Some(chi)) = (
            active.sp.k().map(nalgebra::DVector::from_column_slice),
            active.sp.chi_kweighted(),
        )
    {
        let x = vecs(&k);
        let n = x.len().min(chi.len());
        let y: Vec<f64> = chi.iter().take(n).map(|v| v + chiq_shift).collect();
        chi_q.series.push(dashed(
            SeriesKey::ChiKOnQ,
            x[..n].to_vec(),
            y,
            TREND,
            &chik_label,
        ));
    }
    // Stacked traces have deliberate vertical offsets; retain their full extent.
    if view.layout != TraceLayout::Waterfall {
        chi_k.ylim = Some(symmetric_y_limits(
            chi_k.series.iter().flat_map(|s| s.y.iter().copied()),
        ));
    }
    [mu_e, norm, chi_k, chi_r, chi_q]
}

/// Tick count and axis limits that make ruviz label an integer axis (frame
/// numbers, component numbers) with whole numbers only, with at most ten ticks.
///
/// See [`integer_axis_max`]; ten is the ceiling ruviz allows a linear axis.
pub(crate) fn integer_axis(first: usize, last: usize) -> (usize, (f64, f64)) {
    integer_axis_max(first, last, 10)
}

/// Tick count and axis limits that make ruviz label an integer axis with whole
/// numbers only, using at most `max_ticks` ticks (clamped to 3..=10, the range
/// ruviz honours; a narrow panel passes a lower ceiling so its labels fit).
///
/// `first..=last` are the integer positions on the axis; at least two are kept
/// so a one-frame axis still reads "0 1". The limits pad each end by half a
/// unit so ticks sit on the values and heatmap rows are not cropped. ruviz's
/// tick locator (matplotlib's `MaxNLocator`) treats the count as a ceiling and
/// keeps the densest "nice" step (1, 2, 2.5 or 5 × 10ⁿ) that fits, so the count
/// is exactly the number of ticks the finest integer step (1, 2, 5, 10, 20,
/// 50, …) with at most `max_ticks` ticks produces: the next finer rung about
/// doubles the count and is rejected, and the 2.5 rung only fits beside a
/// rounder step with more ticks, which the locator prefers. The count must be
/// the plot's final `major_ticks_x`; a later override with a width-only count
/// lets the 2.5 rung back in. ruviz also never accepts a ceiling below three,
/// so when the fitting step would leave only two ticks (0 and 5 of seven
/// frames), the previous, denser integer step is kept: a slightly crowded
/// axis beats a fractional one.
pub(crate) fn integer_axis_max(first: usize, last: usize, max_ticks: usize) -> (usize, (f64, f64)) {
    let last = last.max(first + 1);
    let max_ticks = max_ticks.clamp(3, 10);
    let mut previous = None;
    let mut count = 2;
    for step in (0..).flat_map(|exponent| [1, 2, 5].map(|m| m * 10usize.pow(exponent))) {
        let n = last / step + 1 - first.div_ceil(step);
        if n <= max_ticks {
            count = if n >= 3 { n } else { previous.unwrap_or(n) };
            break;
        }
        previous = Some(n);
    }
    (count, (first as f64 - 0.5, last as f64 + 0.5))
}

fn heatmap_y_extent(scan_len: usize, row_count: usize) -> (f64, f64) {
    let last_frame = scan_len.saturating_sub(1) as f64;
    let row_step = if row_count > 1 {
        last_frame / (row_count - 1) as f64
    } else {
        1.0
    };
    let half_step = row_step / 2.0;
    (-half_step, last_frame + half_step)
}

/// Operando heatmap in physical units: x in k (1/Angstrom) from the resample
/// grid, y = frame index over the FULL scan (rows are the evenly sampled
/// overview). Rows remain in chronological order; the lower heatmap origin maps
/// row 0 to frame 0, while the displayed y axis is reversed so frame 0 appears
/// at the top and tick values remain truthful. Frame ticks are whole numbers
/// (see [`integer_axis`]).
pub fn build_heatmap(
    matrix: &[Vec<f64>],
    grid: &[f64],
    scan_len: usize,
    xlabel: &str,
    theme: &Theme,
    display: &crate::app::series_display::SeriesDisplay,
) -> Plot {
    let kmin = grid.first().copied().unwrap_or(0.0);
    let kmax = grid.last().copied().unwrap_or(1.0).max(kmin + 1e-9);
    let (ymin, ymax) = heatmap_y_extent(scan_len, matrix.len());
    let (frame_ticks, (view_min, view_max)) = integer_axis(0, scan_len.saturating_sub(1));
    let mut config = HeatmapConfig::new()
        .colorbar(true)
        .cmap(display.palette().map(display.reversed))
        .origin(HeatmapOrigin::Lower)
        .extent(kmin, kmax, ymin, ymax);
    if display.difference {
        let (lo, hi) = symmetric_y_limits(matrix.iter().flat_map(|row| row.iter().copied()));
        config = config.vmin(lo).vmax(hi);
    }
    let plot: Plot = Plot::new()
        .theme(theme.plot_theme())
        .xlabel(xlabel)
        .ylabel("Frame")
        .heatmap_with(matrix, config)
        .ylim(view_max, view_min)
        .into();
    plot.major_ticks_y(frame_ticks)
}

/// Source-backed energy or R-space cursor frame. Replacing the observable
/// redraws the selected frame without rebuilding its interactive viewport.
pub fn build_frame_row_source(
    grid: &[f64],
    values: Observable<Vec<f64>>,
    xlabel: &str,
    ylabel: &str,
    theme: &Theme,
) -> Plot {
    Plot::new()
        .theme(theme.plot_theme())
        .line_source(grid, values)
        .color(trace_color(theme, 0))
        .xlabel(xlabel)
        .ylabel(ylabel)
        .into()
}

/// Source-backed chi(k) of one operando frame with symmetric display limits.
/// Rebuild when the amplitude extent changes; otherwise replace `values` in place.
pub fn build_frame_chik_source(
    grid: &[f64],
    values: Observable<Vec<f64>>,
    kweight: f64,
    theme: &Theme,
) -> Plot {
    let (lo, hi) = symmetric_y_limits(values.read().iter().copied());
    let plot: Plot = Plot::new()
        .theme(theme.plot_theme())
        .line_source(grid, values)
        .xlabel(K_AXIS)
        .ylabel(chik_label(kweight))
        .into();
    plot.ylim(lo, hi)
        .hline_styled(0., Color::from_gray(120), 0.8, LineStyle::Dashed)
}

/// k-space fit overlay: k-weighted data vs model and, optionally, the
/// per-path contributions. The fit range is drawn by the handle decor.
pub fn build_fit_k(
    result: &rexafs::prelude::FeffFitResult,
    theme: &Theme,
    show_paths: bool,
    highlight: Option<&str>,
) -> Plot {
    let k = vecs(&result.k);
    let kw = result.kweight;
    let weight = |chi: &nalgebra::DVector<f64>| -> Vec<f64> {
        chi.iter()
            .zip(k.iter())
            .map(|(c, kk)| c * kk.powf(kw))
            .collect()
    };
    let data = weight(&result.data_chi);
    let model = weight(&result.model_chi);
    let mut amplitudes: Vec<f64> = data.iter().chain(&model).copied().collect();
    let mut plot: Plot = Plot::new()
        .theme(theme.plot_theme())
        .line(&k, &data)
        .color(trace_color(theme, 0))
        .label("data")
        .line(&k, &model)
        .color(FIT_COLOR)
        .line_width(1.6)
        .label("fit")
        .into();
    if show_paths && result.path_contributions.len() > 1 {
        for (i, path) in result.path_contributions.iter().enumerate() {
            let y = weight(&path.chi);
            amplitudes.extend_from_slice(&y);
            plot = plot
                .line(&k, &y)
                .color(trace_color(theme, i + 2))
                .line_width(1.0)
                .line_style(LineStyle::Dashed)
                .label(&path.label)
                .into();
        }
    }
    if let Some(h) = highlight
        && let Some(path) = result.path_contributions.iter().find(|p| p.label == h)
    {
        let y = weight(&path.chi);
        amplitudes.extend_from_slice(&y);
        plot = plot
            .line(&k, &y)
            .color(HOVER_COLOR)
            .line_width(2.2)
            .label(format!("{h} (hover)"))
            .into();
    }
    centered_y(plot, amplitudes)
        .legend_position(ruviz::core::LegendPosition::UpperRight)
        .xlabel(K_AXIS)
        .ylabel(chik_label(kw))
}

/// Hovered-path preview trace (amber).
const HOVER_COLOR: Color = Color {
    r: 224,
    g: 179,
    b: 90,
    a: 255,
};

/// Fit trace colour (orange in both themes, distinct from the data trace).
const FIT_COLOR: Color = Color {
    r: 232,
    g: 121,
    b: 47,
    a: 255,
};

/// R-space fit overlay: |χ(R)| of data vs model, optionally Re[χ(R)] and
/// the per-path contributions stacked below the data (Artemis style).
pub fn build_fit_r(
    result: &rexafs::prelude::FeffFitResult,
    theme: &Theme,
    show_paths: bool,
    show_re: bool,
    show_im: bool,
) -> Plot {
    let r = vecs(&result.r);
    let data_mag = fit_data_chir_mag(result);
    let model_mag = vecs(&result.model_chir_mag);
    let n = r.len().min(data_mag.len()).min(model_mag.len());
    let r = r[..n].to_vec();
    let data_mag = data_mag[..n].to_vec();
    let model_mag = model_mag[..n].to_vec();
    let peak = data_mag.iter().fold(0.0f64, |m, v| m.max(*v)).max(1e-12);
    let mut plot: Plot = Plot::new()
        .theme(theme.plot_theme())
        .line(&r, &data_mag)
        .color(trace_color(theme, 0))
        .label("|χ(R)| data")
        .line(&r, &model_mag)
        .color(FIT_COLOR)
        .line_width(1.6)
        .label("fit")
        .into();
    if show_re {
        let re_d: Vec<f64> = result.data_chir_re.iter().take(n).copied().collect();
        let re_m: Vec<f64> = result.model_chir_re.iter().take(n).copied().collect();
        let r_d = r[..re_d.len()].to_vec();
        let r_m = r[..re_m.len()].to_vec();
        plot = plot
            .line(&r_d, &re_d)
            .color(trace_color(theme, 0))
            .line_width(1.0)
            .line_style(LineStyle::Dotted)
            .label("Re data")
            .line(&r_m, &re_m)
            .color(FIT_COLOR)
            .line_width(1.0)
            .line_style(LineStyle::Dotted)
            .label("Re fit")
            .into();
    }
    if show_im {
        for (values, color, label) in [
            (&result.data_chir_im, trace_color(theme, 0), "Im data"),
            (&result.model_chir_im, FIT_COLOR, "Im fit"),
        ] {
            let y: Vec<_> = values.iter().take(n).copied().collect();
            plot = plot
                .line(&r[..y.len()], &y)
                .color(color)
                .line_width(1.2)
                .line_style(LineStyle::Dashed)
                .label(label)
                .into();
        }
    }
    if show_paths && !result.path_contributions.is_empty() {
        // Each path sits on its own baseline below zero so the shells read
        // as a stack rather than a tangle.
        for (i, path) in result.path_contributions.iter().enumerate() {
            let offset = -peak * (0.35 + 0.3 * i as f64);
            let m: Vec<f64> = path.chir_mag.iter().take(n).map(|v| v + offset).collect();
            let x = r[..m.len()].to_vec();
            plot = plot
                .line(&x, &m)
                .color(trace_color(theme, i + 2))
                .line_width(1.0)
                .label(&path.label)
                .into();
        }
    }
    plot.legend_position(ruviz::core::LegendPosition::UpperRight)
        .xlabel(R_AXIS)
        .ylabel(if show_re || show_im {
            chir_label(result.kweight).replace("|χ(R)|", "χ(R)")
        } else {
            chir_label(result.kweight)
        })
}

/// q-space view: back-transformed data vs model over the fit's R-window.
pub fn build_fit_q(result: &rexafs::prelude::FeffFitResult, theme: &Theme) -> Plot {
    let q = vecs(&result.q);
    let n = q
        .len()
        .min(result.data_chiq.len())
        .min(result.model_chiq.len());
    let q = q[..n].to_vec();
    let data: Vec<f64> = result.data_chiq.iter().take(n).copied().collect();
    let model: Vec<f64> = result.model_chiq.iter().take(n).copied().collect();
    let plot: Plot = Plot::new()
        .theme(theme.plot_theme())
        .line(&q, &data)
        .color(trace_color(theme, 0))
        .label("data")
        .line(&q, &model)
        .color(FIT_COLOR)
        .line_width(1.6)
        .label("fit")
        .into();
    plot.legend_position(ruviz::core::LegendPosition::UpperRight)
        .xlabel("q (Å⁻¹)")
        .ylabel(format!("Re χ(q) · kw {:.0}", result.kweight))
}

fn fit_data_chir_mag(result: &rexafs::prelude::FeffFitResult) -> Vec<f64> {
    result
        .data_chir_re
        .iter()
        .zip(result.data_chir_im.iter())
        .map(|(re, im)| (re * re + im * im).sqrt())
        .collect()
}

/// Residual strip under the k-space fit: k-weighted (data - model).
pub fn build_fit_residual_k(result: &rexafs::prelude::FeffFitResult, theme: &Theme) -> Plot {
    let k = vecs(&result.k);
    let kw = result.kweight;
    let res: Vec<f64> = result
        .data_chi
        .iter()
        .zip(result.model_chi.iter())
        .zip(k.iter())
        .map(|((d, m), kk)| (d - m) * kk.powf(kw))
        .collect();
    let plot: Plot = Plot::new()
        .theme(theme.plot_theme())
        .line(&k, &res)
        .line_width(1.0)
        .color(Color::from_gray(150))
        .into();
    centered_y(plot, res).xlabel("").ylabel("").major_ticks_y(3)
}

/// Residual strip under the R-space fit: |chi(R)| data - model.
pub fn build_fit_residual_r(result: &rexafs::prelude::FeffFitResult, theme: &Theme) -> Plot {
    let r = vecs(&result.r);
    let data_mag = fit_data_chir_mag(result);
    let model_mag = vecs(&result.model_chir_mag);
    let n = r.len().min(data_mag.len()).min(model_mag.len());
    let res: Vec<f64> = (0..n).map(|i| data_mag[i] - model_mag[i]).collect();
    let r = r[..n].to_vec();
    let plot: Plot = Plot::new()
        .theme(theme.plot_theme())
        .line(&r, &res)
        .line_width(1.0)
        .color(Color::from_gray(150))
        .into();
    plot.hline_styled(0.0, Color::from_gray(120), 0.8, LineStyle::Dashed)
        .xlabel("")
        .ylabel("")
        .major_ticks_y(3)
}

/// LCF overlay: data, fit, residual (offset below) and the weighted
/// components stacked underneath.
pub fn build_lcf_plot(
    result: &rexafs::prelude::LcfResult,
    xlabel: &str,
    ylabel: &str,
    theme: &Theme,
) -> Plot {
    let x = vecs(&result.x);
    let data = vecs(&result.data);
    let fit = vecs(&result.fit);
    let span = data.iter().fold(0.0f64, |m, v| m.max(v.abs())).max(1e-12);
    let mut plot: Plot = Plot::new()
        .theme(theme.plot_theme())
        .line(&x, &data)
        .color(trace_color(theme, 0))
        .label("data")
        .line(&x, &fit)
        .color(FIT_COLOR)
        .line_width(1.6)
        .label("fit")
        .into();
    for (i, (comp, weight)) in result.components.iter().zip(&result.weights).enumerate() {
        let y: Vec<f64> = comp
            .iter()
            .map(|v| v - span * 0.25 * (i + 1) as f64)
            .collect();
        let n = y.len().min(x.len());
        let (xs, ys) = (x[..n].to_vec(), y[..n].to_vec());
        plot = plot
            .line(&xs, &ys)
            .color(trace_color(theme, i + 2))
            .line_width(1.0)
            .label(format!("{} × {:.2}", weight.name, weight.weight))
            .into();
    }
    let n = result.components.len();
    let res: Vec<f64> = result
        .residual
        .iter()
        .map(|v| v - span * 0.25 * (n + 1) as f64)
        .collect();
    let n = res.len().min(x.len());
    let (xs, rs) = (x[..n].to_vec(), res[..n].to_vec());
    plot = plot
        .line(&xs, &rs)
        .color(Color::from_gray(150))
        .line_width(1.0)
        .line_style(LineStyle::Dashed)
        .label("residual")
        .into();
    plot.legend_position(ruviz::core::LegendPosition::UpperRight)
        .xlabel(xlabel)
        .ylabel(ylabel)
}

/// Estimated MCR spectra, without arbitrary offsets or inferred chemical labels.
pub fn build_mcr_plot(result: &rexafs::prelude::McrResult, theme: &Theme) -> Plot {
    let x = vecs(&result.x);
    let mut plot = Plot::new().theme(theme.plot_theme());
    for i in 0..result.spectra.nrows() {
        let y: Vec<_> = result.spectra.row(i).iter().copied().collect();
        plot = plot
            .line(&x, &y)
            .color(trace_color(theme, i))
            .label(format!("Component {}", i + 1))
            .into();
    }
    plot.xlabel("Energy (eV)")
        .ylabel(
            if result.config.space == rexafs::prelude::AnalysisSpace::Flat {
                "flattened μ(E)"
            } else {
                "normalized μ(E)"
            },
        )
        .legend_position(ruviz::core::LegendPosition::UpperRight)
}

/// PCA target transform: data vs reconstruction with the residual below.
pub fn build_pca_plot(
    fit: &rexafs::prelude::PcaFit,
    xlabel: &str,
    ylabel: &str,
    theme: &Theme,
) -> Plot {
    let x = vecs(&fit.x);
    let data = vecs(&fit.data);
    let recon = vecs(&fit.fit);
    let span = data.iter().fold(0.0f64, |m, v| m.max(v.abs())).max(1e-12);
    let res: Vec<f64> = fit.residual.iter().map(|v| v - span * 0.3).collect();
    let n = res.len().min(x.len());
    let (xs, rs) = (x[..n].to_vec(), res[..n].to_vec());
    let plot: Plot = Plot::new()
        .theme(theme.plot_theme())
        .line(&x, &data)
        .color(trace_color(theme, 0))
        .label("data")
        .line(&x, &recon)
        .color(FIT_COLOR)
        .line_width(1.6)
        .label(crate::text::plural(fit.n_components, "component"))
        .line(&xs, &rs)
        .color(Color::from_gray(150))
        .line_width(1.0)
        .line_style(LineStyle::Dashed)
        .label("residual")
        .into();
    plot.legend_position(ruviz::core::LegendPosition::UpperRight)
        .xlabel(xlabel)
        .ylabel(ylabel)
}

/// Parameter-vs-frame trend. The moving cursor is a dynamic annotation owned
/// by the interactive plot session, so scrubbing does not rebuild this data.
/// The frame axis is ticked at whole frames using at most `max_ticks` ticks
/// (see [`integer_axis_max`]); the caller derives the ceiling from the panel
/// width, so callers must not override `major_ticks_x` afterwards.
pub fn build_trend(
    values: &[f64],
    frames: &[f64],
    ylabel: &str,
    max_ticks: usize,
    theme: &Theme,
) -> Plot {
    let mut plot = Plot::new()
        .theme(theme.plot_theme())
        .xlabel("Frame")
        .ylabel(ylabel);
    // Missing/failed frames remain real gaps: each contiguous finite run is
    // its own series, located at its true frame position.
    let n = values.len().min(frames.len());
    let mut start = 0;
    while start < n {
        while start < n && !values[start].is_finite() {
            start += 1;
        }
        let mut end = start;
        while end < n && values[end].is_finite() {
            end += 1;
        }
        if start < end {
            let xs: Vec<f64> = frames[start..end].to_vec();
            let ys: Vec<f64> = values[start..end].to_vec();
            plot = if end - start == 1 {
                plot.scatter(&xs, &ys).color(trace_color(theme, 0)).into()
            } else {
                plot.line(&xs, &ys).color(trace_color(theme, 0)).into()
            };
        }
        start = end.saturating_add(1);
    }
    if n == 0 {
        return plot;
    }
    let last = frames[..n].iter().copied().fold(0.0f64, f64::max);
    let (ticks, (lo, hi)) = integer_axis_max(0, last.round() as usize, max_ticks);
    plot.xlim(lo, hi).major_ticks_x(ticks)
}

#[cfg(test)]
mod tests {
    use super::{chik_label, chir_label, heatmap_y_extent, middle_truncate};

    #[test]
    fn normalization_fit_overlay_follows_method_units_toggle_and_active_trace() {
        use super::*;
        use crate::params::{PipelineParams, process_file};
        use std::{path::Path, sync::Arc};
        let file =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/projects/data/cu_150k.xmu");
        let polynomial = Arc::new(process_file(&file, &PipelineParams::default()).unwrap());
        let mut mback = (*polynomial).clone();
        mback
            .set_normalization_method(rexafs::MBack::for_edge("Cu", "K"))
            .unwrap();
        mback.normalize().unwrap();
        let mback = Arc::new(mback);
        let before = serde_json::to_string(&*mback).unwrap();
        let mut traces = vec![
            QuadTrace {
                color_index: 0,
                color: None,
                label: "Polynomial".into(),
                sp: polynomial,
                active: false,
            },
            QuadTrace {
                color_index: 1,
                color: None,
                label: "MBACK".into(),
                sp: mback.clone(),
                active: true,
            },
        ];
        let view = ViewOptions {
            show_pre: true,
            show_post: true,
            layout: TraceLayout::Waterfall,
            offset_frac: 0.2,
            ..Default::default()
        };
        let specs = build_quadrant_specs(&traces, &view, &Theme::dark(), true);
        let overlay = specs[0]
            .series
            .iter()
            .find(|s| s.key == SeriesKey::MbackFit)
            .unwrap();
        let rexafs::NormalizationMethod::MBack(model) = mback.normalization.as_ref().unwrap()
        else {
            panic!()
        };
        let result = model.result.as_ref().unwrap();
        let mu = traces[0].sp.mu.as_ref().unwrap();
        let shift = (mu.max() - mu.min()) * view.offset_frac;
        assert_eq!(overlay.label.as_deref(), Some("MBACK fit"));
        assert_eq!(overlay.x, result.energy);
        for ((shown, f2), background) in overlay.y.iter().zip(&result.f2).zip(&result.background) {
            assert!((shown - ((f2 + background) / result.scale + shift)).abs() < 1e-12);
        }
        assert!(
            !specs[0]
                .series
                .iter()
                .any(|s| matches!(s.key, SeriesKey::PreEdge | SeriesKey::PostEdge))
        );
        assert_eq!(specs[0].series.len(), 3);
        // Fits in input units never appear on the normalized/flattened axis.
        assert!(!specs[1].series.iter().any(|s| s.key == SeriesKey::MbackFit));
        let hidden = build_quadrant_specs(&traces, &ViewOptions::default(), &Theme::dark(), true);
        assert_eq!(hidden[0].series.len(), 2);
        // An inactive MBACK comparison must not annotate the polynomial trace.
        traces[0].active = true;
        traces[1].active = false;
        let polynomial_active = build_quadrant_specs(&traces, &view, &Theme::dark(), true);
        assert!(
            !polynomial_active[0]
                .series
                .iter()
                .any(|s| s.key == SeriesKey::MbackFit)
        );
        for key in [SeriesKey::PreEdge, SeriesKey::PostEdge] {
            assert!(polynomial_active[0].series.iter().any(|s| s.key == key));
        }
        assert_eq!(serde_json::to_string(&*mback).unwrap(), before);
    }

    #[test]
    fn symmetric_k_limits_retain_both_glitch_polarities_and_zero_signals() {
        use super::symmetric_y_limits;
        assert_eq!(symmetric_y_limits([-2., 100., 1.]), (-105., 105.));
        assert_eq!(symmetric_y_limits([-100., 2., 1.]), (-105., 105.));
        assert_eq!(symmetric_y_limits([0., f64::NAN]), (-1., 1.));
        assert_eq!(symmetric_y_limits([]), (-1., 1.));
    }

    #[test]
    fn k_frame_plot_has_symmetric_visible_bounds_for_each_new_amplitude() {
        use super::*;
        use ruviz::core::plot::SurfaceTarget;
        let values = Observable::new(vec![-2., 100., 1.]);
        for signal in [vec![-2., 100., 1.], vec![-200., 1., 3.]] {
            values.set(signal.clone());
            let plot = build_frame_chik_source(&[0., 1., 2.], values.clone(), 2., &Theme::dark());
            let session = plot.prepare_interactive();
            session
                .render_to_surface(SurfaceTarget {
                    size_px: (600, 300),
                    scale_factor: 1.,
                    time_seconds: 0.,
                })
                .unwrap();
            let bounds = session.viewport_snapshot().unwrap().visible_bounds;
            assert_eq!(bounds.min.y, -bounds.max.y);
            assert!(
                signal
                    .iter()
                    .all(|v| *v >= bounds.min.y && *v <= bounds.max.y)
            );
            assert_eq!(*values.read(), signal);
        }
    }

    #[test]
    fn compare_coverage_counts_extracted_spectra_per_plot_and_separates_sampling() {
        use super::*;
        use crate::params::{DerivedSpectrum, PipelineParams, Quantity, process_file};
        use std::{path::Path, sync::Arc};
        let params = PipelineParams::default();
        let file =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/projects/data/cu_150k.xmu");
        let current = Arc::new(process_file(&file, &params).unwrap());
        let difference = DerivedSpectrum {
            energy: current.energy.as_ref().unwrap().as_slice().to_vec(),
            mu: vec![0.0; current.energy.as_ref().unwrap().len()],
            quantity: Quantity::NormalizedDifference,
            ..Default::default()
        };
        let difference = Arc::new(difference.for_display(&params).unwrap());
        let traces = (0..12)
            .map(|i| QuadTrace {
                color_index: i,
                color: None,
                label: i.to_string(),
                sp: if i == 0 {
                    current.clone()
                } else {
                    difference.clone()
                },
                active: i == 0,
            })
            .collect::<Vec<_>>();
        let specs = quantity_quadrant_specs(
            &traces,
            &ViewOptions::default(),
            &Theme::dark(),
            false,
            Quantity::default(),
        );
        assert_eq!(specs[0].coverage(13, 12, 12).plotted, 12);
        let coverage = specs[2].coverage(13, 12, 12);
        assert_eq!(coverage, PlotCoverage::new(13, 12, 12, 1));
        assert_eq!(
            coverage.disclosure().unwrap(),
            "1 of 13 spectra plotted · 1 omitted by sampling · 11 without data for this plot"
        );
        assert_eq!(
            PlotCoverage::new(13, 12, 10, 1).disclosure().unwrap(),
            "1 of 13 spectra plotted · 1 omitted by sampling · 9 without data for this plot · 2 failed or not loaded"
        );
        assert_eq!(PlotCoverage::new(1, 1, 1, 1).disclosure(), None);
    }

    #[test]
    fn mixed_weight_overlays_share_the_k_axis_and_identify_fourier_weights() {
        use super::*;
        use crate::params::{PipelineParams, process_file};
        use std::{path::Path, sync::Arc};
        let file =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/projects/data/cu_150k.xmu");
        let spectra: Vec<_> = [1.0, 3.0]
            .into_iter()
            .map(|weight| {
                Arc::new(
                    process_file(
                        &file,
                        &PipelineParams {
                            fft_kweight: Some(weight),
                            ..Default::default()
                        },
                    )
                    .unwrap(),
                )
            })
            .collect();
        assert_eq!(spectra[0].chi(), spectra[1].chi());
        let original_r: Vec<_> = spectra.iter().map(|sp| sp.chir_mag().unwrap()).collect();
        for active in 0..2 {
            let weight = [1.0, 3.0][active];
            let traces: Vec<_> = spectra
                .iter()
                .enumerate()
                .map(|(i, sp)| QuadTrace {
                    label: format!("same spectrum {i}"),
                    sp: sp.clone(),
                    active: i == active,
                    color_index: i + 4,
                    color: (i == 1).then(|| {
                        crate::spectrum_colors::Assignment {
                            palette: crate::spectrum_colors::Palette::Viridis,
                            index: 1,
                            count: 2,
                            reversed: false,
                        }
                        .color(&Theme::dark())
                    }),
                })
                .collect();
            let specs =
                build_quadrant_specs(&traces, &ViewOptions::default(), &Theme::dark(), true);
            assert_eq!(specs[2].ylabel, chik_label(weight));
            for index in 0..2 {
                let plotted = specs[2]
                    .series
                    .iter()
                    .find(|s| s.key == SeriesKey::Trace(index))
                    .unwrap();
                let expected: Vec<_> = spectra[index]
                    .k()
                    .unwrap()
                    .iter()
                    .zip(spectra[index].chi().unwrap())
                    .map(|(k, chi)| chi * k.powf(weight))
                    .collect();
                assert_eq!(plotted.y, expected);
                let expected_color = traces[index]
                    .color
                    .unwrap_or_else(|| trace_color(&Theme::dark(), index + 4));
                assert_eq!(plotted.color, expected_color);
                for spec in &specs {
                    if let Some(series) = spec
                        .series
                        .iter()
                        .find(|s| s.key == SeriesKey::Trace(index))
                    {
                        assert_eq!(series.color, expected_color);
                    }
                }
                let fourier = specs[3]
                    .series
                    .iter()
                    .find(|s| s.key == SeriesKey::Trace(index))
                    .unwrap();
                assert_eq!(fourier.color, expected_color);
                assert_eq!(
                    fourier.y,
                    original_r[index].iter().copied().collect::<Vec<_>>()
                );
                assert!(
                    fourier
                        .label
                        .as_ref()
                        .unwrap()
                        .starts_with(&format!("FT kw={}", [1, 3][index]))
                );
            }
            assert!(specs[3].ylabel.contains("mixed k weights"));
            assert!(specs[4].ylabel.contains("mixed k weights"));
            assert_eq!(spectra[0].kweight(), Some(&1.0));
            assert_eq!(spectra[1].kweight(), Some(&3.0));
        }
    }

    #[test]
    fn middle_truncate_passes_short_names_through() {
        assert_eq!(middle_truncate("short.dat", 28), "short.dat");
        assert_eq!(middle_truncate("", 8), "");
    }

    #[test]
    fn middle_truncate_keeps_head_and_tail() {
        let out = middle_truncate("frame_000000000000123456.dat", 15);
        assert_eq!(out.chars().count(), 15);
        assert!(out.starts_with("frame_0"));
        assert!(out.ends_with("456.dat"));
        assert!(out.contains('…'));
    }

    #[test]
    fn middle_truncate_respects_char_boundaries() {
        let out = middle_truncate("αβγδεζηθικλμνξοπρστυ", 9);
        assert_eq!(out.chars().count(), 9);
    }

    #[test]
    fn chik_labels_common_weights() {
        assert_eq!(chik_label(0.0), "χ(k)");
        assert_eq!(chik_label(1.0), "k·χ(k)");
        assert_eq!(chik_label(2.0), "k²χ(k)");
        assert_eq!(chik_label(3.0), "k³χ(k)");
        assert_eq!(chik_label(4.0), "k^4 χ(k)");
        assert_eq!(chik_label(2.5), "k^2.5 χ(k)");
    }

    #[test]
    fn chir_label_tracks_kweight() {
        // |chi(R)| for k-weight n carries units of Å^-(n+1); the old hardcoded
        // Å⁻³ was only right for the default k-weight of 2.
        assert_eq!(chir_label(0.0), "|χ(R)| (Å⁻¹)");
        assert_eq!(chir_label(1.0), "|χ(R)| (Å⁻²)");
        assert_eq!(chir_label(2.0), "|χ(R)| (Å⁻³)");
        assert_eq!(chir_label(3.0), "|χ(R)| (Å⁻⁴)");
        assert_eq!(chir_label(4.0), "|χ(R)| (Å^-5)");
    }

    #[test]
    fn heatmap_extent_centers_endpoint_rows_on_scan_frames() {
        let scan_len = 1_000;
        let row_count = 192;
        let (ymin, ymax) = heatmap_y_extent(scan_len, row_count);
        let row_step = (ymax - ymin) / row_count as f64;

        assert!((ymin + row_step / 2.0).abs() < 1e-12);
        assert!((ymax - row_step / 2.0 - 999.0).abs() < 1e-12);
    }

    #[test]
    fn heatmap_extent_handles_short_and_degenerate_inputs() {
        assert_eq!(heatmap_y_extent(3, 3), (-0.5, 2.5));
        assert_eq!(heatmap_y_extent(1, 1), (-0.5, 0.5));
        assert_eq!(heatmap_y_extent(0, 0), (-0.5, 0.5));
    }

    #[test]
    fn integer_axis_picks_the_finest_integer_step_with_at_most_ten_ticks() {
        use super::integer_axis;
        assert_eq!(integer_axis(0, 0), (2, (-0.5, 1.5)));
        assert_eq!(integer_axis(0, 1), (2, (-0.5, 1.5)));
        assert_eq!(integer_axis(0, 9), (10, (-0.5, 9.5)));
        assert_eq!(integer_axis(0, 11), (6, (-0.5, 11.5)));
        assert_eq!(integer_axis(1, 12), (6, (0.5, 12.5)));
        assert_eq!(integer_axis(0, 999), (10, (-0.5, 999.5)));
    }

    #[test]
    fn integer_axis_max_coarsens_the_step_for_narrow_panels() {
        use super::integer_axis_max;
        use ruviz::axes::generate_ticks;
        // Seven frames on a trend panel with room for five labels: step 2.
        let (count, (lo, hi)) = integer_axis_max(0, 6, 5);
        assert_eq!(count, 4);
        assert_eq!(generate_ticks(lo, hi, count), vec![0.0, 2.0, 4.0, 6.0]);
        // With room for three labels no integer step fits (step 5 leaves two,
        // which ruviz's floor of three would turn into 0 / 2.5 / 5), so the
        // step-2 axis is kept.
        assert_eq!(integer_axis_max(0, 6, 3), (4, (-0.5, 6.5)));
        assert_eq!(integer_axis_max(0, 6, 1), integer_axis_max(0, 6, 3));
    }

    #[test]
    fn integer_axis_ticks_are_whole_numbers_in_ruviz() {
        use super::integer_axis_max;
        use ruviz::axes::generate_ticks;
        for max_ticks in 3..=10 {
            for first in [0, 1] {
                for last in first..=3_000 {
                    let (count, (lo, hi)) = integer_axis_max(first, last, max_ticks);
                    // The ceiling is exceeded only to keep three or more ticks.
                    assert!(
                        count <= max_ticks || count <= 2 * max_ticks + 1,
                        "{first}..={last}: {count} ticks for a ceiling of {max_ticks}"
                    );
                    let ticks = generate_ticks(lo, hi, count);
                    assert!(ticks.len() >= 2, "{first}..={last}: {ticks:?}");
                    assert!(
                        ticks.iter().all(|t| t.fract() == 0.0),
                        "{first}..={last} with {count} of {max_ticks} ticks: {ticks:?}"
                    );
                }
            }
        }
    }
}

/// A tool preview uses full scientific inputs; display curves keep their own grids.
pub(crate) fn build_tool_preview(
    before: &XASSpectrum,
    after: &XASSpectrum,
    standard: Option<(&str, &XASSpectrum)>,
    difference: bool,
    alignment_window: Option<(f64, f64)>,
    theme: &Theme,
) -> Result<Plot, String> {
    if let Some(window) = alignment_window {
        return alignment::build(
            before,
            after,
            standard.ok_or("Choose a standard")?,
            window,
            theme,
        );
    }
    let bx = before.energy.as_ref().ok_or("Target has no energy grid")?;
    let by = if difference {
        before.norm()
    } else {
        before.mu.clone()
    }
    .ok_or("Target quantity unavailable")?;
    let ax = after.energy.as_ref().ok_or("Result has no energy grid")?;
    let ay = after.mu.as_ref().ok_or("Result quantity unavailable")?;
    let mut plot: Plot = Plot::new()
        .theme(theme.plot_theme())
        .line(&vecs(bx), &vecs(&by))
        .color(trace_color(theme, 0))
        .line_style(LineStyle::Dashed)
        .label("Original")
        .line(&vecs(ax), &vecs(ay))
        .color(trace_color(theme, 1))
        .line_width(1.8)
        .label(if difference {
            "Result: target − baseline"
        } else {
            "Result"
        })
        .into();
    if let Some((name, standard)) = standard {
        let sy = if difference {
            standard.norm()
        } else {
            standard.mu.clone()
        };
        if let (Some(x), Some(y)) = (&standard.energy, sy) {
            plot = plot
                .line(&vecs(x), &vecs(&y))
                .color(trace_color(theme, 2))
                .line_width(1.1)
                .label(format!("Standard: {name}"))
                .into();
        }
    }
    Ok(plot
        .xlabel("Energy (eV)")
        .ylabel(if difference {
            "normalized μ(E) / Δμ(E)"
        } else {
            "μ(E)"
        })
        .legend_position(LegendPosition::UpperRight))
}
