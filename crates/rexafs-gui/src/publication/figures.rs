//! Publication presets share the same data and renderer with preview and export.
use super::*;
use ruviz::prelude::{LineStyle, Plot};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

fn weighted_unit(weight: f64) -> String {
    if weight.abs() < 1e-9 {
        "(dimensionless)".into()
    } else {
        plotting::chir_label(weight - 1.).replace("|χ(R)| ", "")
    }
}
fn weighted_chi_label(weight: f64) -> String {
    format!("{} {}", plotting::chik_label(weight), weighted_unit(weight))
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct FigureOptions {
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub dpi: Option<f64>,
    pub font_size: Option<f64>,
    pub line_width: Option<f64>,
    pub title: Option<String>,
    pub caption: Option<String>,
    pub xlabel: Option<String>,
    pub ylabel: Option<String>,
    pub xmin: Option<f64>,
    pub xmax: Option<f64>,
    pub ymin: Option<f64>,
    pub ymax: Option<f64>,
    pub legend: bool,
    pub grid: Option<bool>,
    pub guides: bool,
    pub hidden: BTreeSet<String>,
    pub shown: BTreeSet<String>,
    /// Energy figures default to the library's flattened array.
    pub normalized: bool,
}

impl Default for FigureOptions {
    fn default() -> Self {
        Self {
            width: None,
            height: None,
            dpi: None,
            font_size: None,
            line_width: None,
            title: None,
            caption: None,
            xlabel: None,
            ylabel: None,
            xmin: None,
            xmax: None,
            ymin: None,
            ymax: None,
            legend: true,
            grid: None,
            guides: false,
            hidden: BTreeSet::new(),
            shown: BTreeSet::new(),
            normalized: false,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct FigureSettings {
    /// Settings are shared by figure type across a multi-spectrum export.
    pub figures: BTreeMap<String, FigureOptions>,
    pub table_captions: BTreeMap<String, String>,
}
impl FigureSettings {
    pub fn options(&self, key: &str) -> FigureOptions {
        self.figures.get(key).cloned().unwrap_or_default()
    }
}

impl FigureOptions {
    pub fn dimensions(&self) -> (f64, f64, f64) {
        let default = Plot::new();
        let figure = &default.get_config().figure;
        (
            self.width.unwrap_or(figure.width as f64),
            self.height.unwrap_or(figure.height as f64),
            self.dpi.unwrap_or(300.),
        )
    }

    pub fn validate(&self) -> Result<(), String> {
        let (width, height, dpi) = self.dimensions();
        if ![width, height, dpi].iter().all(|v| v.is_finite())
            || !(1.0..=30.0).contains(&width)
            || !(1.0..=30.0).contains(&height)
            || !(72.0..=1200.0).contains(&dpi)
            || dpi.fract() != 0.0
        {
            return Err("Use a width/height of 1–30 inches and an integer DPI of 72–1200.".into());
        }
        if width * height * dpi * dpi > 25_000_000.0 {
            return Err("Reduce size or DPI to keep the figure below 25 million pixels.".into());
        }
        for (name, value, low, high) in [
            ("Font size", self.font_size, 4.0, 48.0),
            ("Line width", self.line_width, 0.1, 12.0),
        ] {
            if value.is_some_and(|v| !v.is_finite() || !(low..=high).contains(&v)) {
                return Err(format!("{name} must be {low}–{high} points, or Auto."));
            }
        }
        for (axis, min, max) in [("X", self.xmin, self.xmax), ("Y", self.ymin, self.ymax)] {
            match (min, max) {
                (None, None) => (),
                (Some(a), Some(b)) if a.is_finite() && b.is_finite() && a < b => (),
                _ => {
                    return Err(format!(
                        "Set both {axis} limits with min < max, or clear both for Auto."
                    ));
                }
            }
        }
        Ok(())
    }

    fn apply(&self, mut plot: Plot) -> Result<Plot, String> {
        self.validate()?;
        let (width, height, dpi) = self.dimensions();
        // Keep the library's physical size, with publication output at 300 DPI.
        if self.width.is_some() || self.height.is_some() {
            plot = plot.size(width as f32, height as f32);
        }
        plot = plot.dpi(dpi as u32).grid(self.grid.unwrap_or(true));
        if let Some(size) = self.font_size {
            plot = plot.font_size(size as f32);
        }
        if let Some(title) = &self.title {
            plot = plot.title(title.clone());
        }
        if let Some(label) = &self.xlabel {
            plot = plot.xlabel(label.clone());
        }
        if let Some(label) = &self.ylabel {
            plot = plot.ylabel(label.clone());
        }
        if let (Some(min), Some(max)) = (self.xmin, self.xmax) {
            plot = plot.xlim(min, max);
        }
        if let (Some(min), Some(max)) = (self.ymin, self.ymax) {
            plot = plot.ylim(min, max);
        }
        if let Some(grid) = self.grid {
            plot = plot.grid(grid);
        }
        if self.legend {
            plot = plot.legend_best();
        }
        Ok(plot)
    }
}

#[derive(Clone)]
pub(crate) struct FigureSeries {
    pub key: String,
    pub label: String,
    pub x: Vec<f64>,
    pub y: Vec<f64>,
    pub dashed: bool,
    pub optional: bool,
}

impl FigureSeries {
    pub(crate) fn visible(&self, options: &FigureOptions) -> bool {
        !options.hidden.contains(&self.key) && (!self.optional || options.shown.contains(&self.key))
    }
}

#[derive(Clone)]
pub(crate) struct FigureData {
    pub key: &'static str,
    pub label: String,
    pub xlabel: String,
    pub ylabel: String,
    pub math_ylabel: String,
    pub series: Vec<FigureSeries>,
    pub normalized_series: Option<Vec<FigureSeries>>,
    pub guides: Vec<f64>,
    pub default_xlim: Option<(f64, f64)>,
}

impl FigureData {
    pub(crate) fn common(&self) -> bool {
        matches!(
            self.key,
            "xanes" | "flattened-mu" | "chi-k" | "chi-r" | "fit-r"
        )
    }

    pub(crate) fn series(&self, options: &FigureOptions) -> &[FigureSeries] {
        if options.normalized
            && let Some(series) = &self.normalized_series
        {
            return series;
        }
        &self.series
    }

    pub(crate) fn ylabel(&self, options: &FigureOptions) -> &str {
        if options.normalized && self.normalized_series.is_some() {
            "Normalized μ(E) (dimensionless)"
        } else {
            &self.ylabel
        }
    }

    pub(crate) fn math_xlabel(&self) -> &str {
        match self.key {
            "mu-energy" | "xanes" | "flattened-mu" => "$E$ (eV)",
            "chi-k" | "fit-k" | "residual-k" => "$k$ ($\"Å\"^(-1)$)",
            "chi-r" | "fit-r" | "residual-r" => "$R$ (Å)",
            "chi-q" | "fit-q" => "$q$ ($\"Å\"^(-1)$)",
            _ => &self.xlabel,
        }
    }

    pub(crate) fn math_ylabel(&self, options: &FigureOptions) -> &str {
        if options.normalized && self.normalized_series.is_some() {
            "Normalized $mu(E)$ (dimensionless)"
        } else {
            &self.math_ylabel
        }
    }

    pub(crate) fn xlim(&self, options: &FigureOptions) -> Option<(f64, f64)> {
        options.xmin.zip(options.xmax).or(self.default_xlim)
    }

    /// Autoscale only the displayed interval; a noisy high-k tail must not
    /// flatten the useful signal. The series and exported CSV keep every point.
    fn visible_ylim(&self, options: &FigureOptions) -> Option<(f64, f64)> {
        let (xmin, xmax) = self.xlim(options)?;
        let mut ymin = f64::INFINITY;
        let mut ymax = f64::NEG_INFINITY;
        let mut include = |y: f64| {
            if y.is_finite() {
                ymin = ymin.min(y);
                ymax = ymax.max(y);
            }
        };
        for s in self.series(options).iter().filter(|s| s.visible(options)) {
            for (&x, &y) in s.x.iter().zip(&s.y) {
                if (xmin..=xmax).contains(&x) {
                    include(y);
                }
            }
            for (x, y) in s.x.windows(2).zip(s.y.windows(2)) {
                for limit in [xmin, xmax] {
                    if x[0] < limit && limit < x[1] {
                        include(y[0] + (y[1] - y[0]) * (limit - x[0]) / (x[1] - x[0]));
                    }
                }
            }
        }
        if !ymin.is_finite() || !ymax.is_finite() {
            return None;
        }
        let pad = if ymax > ymin {
            (ymax - ymin) * 0.05
        } else {
            ymin.abs().max(1.) * 0.05
        };
        Some((ymin - pad, ymax + pad))
    }

    /// One x/y pair per visible curve preserves distinct grids without
    /// interpolation. Seventeen significant digits round-trip f64 values.
    pub(crate) fn csv(&self, options: &FigureOptions) -> Result<String, String> {
        let series: Vec<_> = self
            .series(options)
            .iter()
            .filter(|s| s.visible(options) && !s.x.is_empty())
            .collect();
        if series.is_empty() {
            return Err("Select at least one curve with available data.".into());
        }
        if series.iter().any(|s| s.x.len() != s.y.len()) {
            return Err("A curve has mismatched x and y arrays.".into());
        }
        let quote = crate::fitting::csv_field;
        let mut csv = series
            .iter()
            .flat_map(|s| {
                [
                    quote(&format!("{}: {}", s.label, self.xlabel)),
                    quote(&format!("{}: {}", s.label, self.ylabel(options))),
                ]
            })
            .collect::<Vec<_>>()
            .join(",");
        csv.push('\n');
        for row in 0..series.iter().map(|s| s.x.len()).max().unwrap_or(0) {
            let cells: Vec<_> = series
                .iter()
                .flat_map(|s| {
                    if row < s.x.len() {
                        [format!("{:.16e}", s.x[row]), format!("{:.16e}", s.y[row])]
                    } else {
                        [String::new(), String::new()]
                    }
                })
                .collect();
            csv.push_str(&cells.join(","));
            csv.push('\n');
        }
        Ok(csv)
    }

    /// A factual starting caption, derived only from the curves actually shown.
    pub fn caption(&self, options: &FigureOptions) -> String {
        if let Some(caption) = &options.caption {
            return caption.clone();
        }
        let description = match self.key {
            "mu-energy" => "X-ray absorption spectrum and selected normalization/background curves",
            "xanes" | "flattened-mu" if options.normalized => {
                "Edge-step-normalized X-ray absorption spectrum"
            }
            "xanes" | "flattened-mu" => {
                "Flattened, edge-step-normalized X-ray absorption spectrum with the fitted post-edge trend removed"
            }
            "chi-k" => "Background-subtracted EXAFS as a function of photoelectron wave number",
            "chi-r" => "Fourier-transformed EXAFS; radial coordinates are not phase corrected",
            "chi-q" => "Back-transformed EXAFS",
            "fit-k" => "EXAFS data, fitted model and selected path contributions in k space",
            "fit-r" => {
                "EXAFS data, fitted model and selected components in R space; radial coordinates are not phase corrected"
            }
            "fit-q" => "Back-transformed EXAFS data and fitted model",
            "residual-k" => "Weighted EXAFS residual, defined as data minus fitted model",
            "residual-r" => {
                "Difference between the magnitudes of the data and fitted Fourier transforms; this is not the magnitude of the complex residual"
            }
            _ => &self.label,
        };
        let curves = self
            .series(options)
            .iter()
            .filter(|s| !s.x.is_empty() && s.visible(options))
            .map(|s| {
                format!(
                    "{} ({})",
                    s.label,
                    if s.dashed { "dashed" } else { "solid" }
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        let mut caption = format!(
            "{description}. Horizontal axis: {}; vertical axis: {}. Curves, in plotting order: {curves}.",
            options.xlabel.as_deref().unwrap_or(&self.xlabel),
            options.ylabel.as_deref().unwrap_or(self.ylabel(options))
        );
        if options.guides && !self.guides.is_empty() {
            caption.push_str(&format!(
                " Vertical guides at {} (horizontal-axis units).",
                self.guides
                    .iter()
                    .map(|v| format!("{v:.4}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        caption
    }

    pub fn plot(&self, options: &FigureOptions) -> Result<Plot, String> {
        let mut plot = Plot::new()
            .typst(true)
            .xlabel(self.math_xlabel())
            .ylabel(self.math_ylabel(options));
        if let Some((min, max)) = self.xlim(options) {
            plot = plot.xlim(min, max);
        }
        if let Some((min, max)) = self.visible_ylim(options) {
            plot = plot.ylim(min, max);
        }
        let mut plot = options.apply(plot)?;
        let mut count = 0;
        for series in self.series(options) {
            if !series.visible(options) || series.x.is_empty() {
                continue;
            }
            let mut line = plot.line(&series.x, &series.y).label(
                ruviz::render::typst_text::literal_text_snippet(&series.label),
            );
            if let Some(width) = options.line_width {
                line = line.line_width(width as f32);
            }
            if series.dashed {
                line = line.line_style(LineStyle::Dashed);
            }
            plot = line.into();
            count += 1;
        }
        if count == 0 {
            return Err("Select at least one curve with available data.".into());
        }
        if options.guides {
            for &x in &self.guides {
                plot = plot.vline_styled(
                    x,
                    ruviz::prelude::Color::from_gray(140),
                    0.8,
                    LineStyle::Dashed,
                );
            }
        }
        Ok(plot)
    }
}

/// Capture the exact PNG displayed in the editor; saving it cannot change layout.
pub(crate) struct RenderedFigure {
    pub png: Vec<u8>,
    pub svg: String,
}
pub(crate) fn render_figure(
    data: &FigureData,
    options: &FigureOptions,
) -> Result<RenderedFigure, String> {
    let plot = data.plot(options)?;
    let mut svg = plot.render_to_svg().map_err(|e| e.to_string())?;
    if let Some(start) = svg.find("<svg")
        && let Some(end) = svg[start..].find('>')
    {
        svg.insert_str(
            start + end + 1,
            &format!(
                "<desc>{}</desc>",
                super::report::html(&data.caption(options))
            ),
        );
        if let Some(title) = &options.title {
            svg.insert_str(
                start + end + 1,
                &format!("<title>{}</title>", super::report::html(title)),
            );
        }
    }
    Ok(RenderedFigure {
        png: plot.render_png_bytes().map_err(|e| e.to_string())?,
        svg,
    })
}

pub(crate) fn quantity_figures(
    sp: Arc<XASSpectrum>,
    label: &str,
    quantity: Option<crate::params::Quantity>,
) -> Vec<FigureData> {
    let mut figures = spectrum_figures(sp, label);
    if let Some(quantity) = quantity.filter(|q| !q.is_absorption()) {
        figures.retain(|figure| !figure.series.is_empty());
        for figure in &mut figures {
            if figure.key == "mu-energy" {
                figure.label = quantity.label().into();
                figure.ylabel = format!("{} (dimensionless)", quantity.label());
                figure.math_ylabel =
                    ruviz::render::typst_text::literal_text_snippet(&figure.ylabel);
            }
        }
    }
    figures
}

fn math_unit(power: f64) -> String {
    if power.abs() < 1e-9 {
        "(dimensionless)".into()
    } else {
        format!("($\"Å\"^({})$)", -power)
    }
}

fn math_chik_label(weight: f64) -> String {
    let chi = if weight.abs() < 1e-9 {
        "chi(k)".into()
    } else {
        format!("k^({weight}) chi(k)")
    };
    format!("${chi}$ {}", math_unit(weight))
}

fn math_chir_label(weight: f64) -> String {
    format!("$abs(chi(R))$ {}", math_unit(weight + 1.))
}

fn publication_kmax(kmax: Option<f64>, data: Option<&[f64]>) -> Option<f64> {
    kmax.filter(|v| v.is_finite() && *v > 0.)
        .or_else(|| {
            data?
                .iter()
                .copied()
                .filter(|v| v.is_finite())
                .reduce(f64::max)
        })
        .filter(|v| *v > 0.)
        .map(|v| v + 1.)
}

fn publication_rmax(fit_rmax: Option<f64>) -> f64 {
    match fit_rmax.filter(|v| v.is_finite() && *v > 6.) {
        Some(max) => (max + 0.5).ceil(),
        None => 6.,
    }
}

pub(crate) fn spectrum_figures(sp: Arc<XASSpectrum>, label: &str) -> Vec<FigureData> {
    let weight = sp.kweight().copied().unwrap_or(2.);
    let kmax = publication_kmax(sp.xftf.as_ref().and_then(|ft| ft.kmax), sp.k());
    let e0 = sp
        .normalization
        .as_ref()
        .and_then(|n| n.get_e0())
        .or(sp.e0());
    let norm_end = match sp.normalization.as_ref() {
        Some(rexafs::xafs::normalization::NormalizationMethod::PrePostEdge(p)) => p.norm_end,
        _ => None,
    };
    // Flattened and normalized are exact, distinct library outputs. Missing
    // flattened data must never be relabeled normalized data.
    let energy_figure = sp
        .energy
        .as_ref()
        .zip(sp.flat())
        .and_then(|(energy, flat)| {
            if energy.is_empty() || energy.len() != flat.len() {
                return None;
            }
            let series = |y: Vec<f64>| FigureSeries {
                key: "series-0".into(),
                label: label.into(),
                x: energy.as_slice().to_vec(),
                y,
                dashed: false,
                optional: false,
            };
            let normalized_series = sp
                .norm()
                .filter(|n| n.len() == energy.len())
                .map(|n| vec![series(n.as_slice().to_vec())]);
            let end = e0
                .zip(norm_end)
                .map(|(edge, end)| edge + end)
                .filter(|v| v.is_finite() && *v > energy[0])
                .unwrap_or(energy[energy.len() - 1])
                .min(energy[energy.len() - 1]);
            Some(FigureData {
                key: "flattened-mu",
                label: "Energy · μ(E)".into(),
                xlabel: "Energy (eV)".into(),
                ylabel: "Flattened μ(E) (dimensionless)".into(),
                math_ylabel: "Flattened $mu(E)$ (dimensionless)".into(),
                series: vec![series(flat.as_slice().to_vec())],
                normalized_series,
                guides: e0.into_iter().collect(),
                default_xlim: (energy[0] < end).then_some((energy[0], end)),
            })
        });
    let opts = plotting::ViewOptions {
        flat: false,
        show_re: false,
        show_im: false,
        show_bkg: true,
        show_pre: true,
        show_post: true,
        show_e0: true,
        show_ranges: true,
        show_kwin: false,
        ..Default::default()
    };
    let traces = [plotting::QuadTrace {
        color_index: 0,
        color: None,
        label: label.into(),
        sp,
        active: true,
    }];
    let mut figures: Vec<_> = ["mu-energy", "normalized-mu", "chi-k", "chi-r", "chi-q"]
        .into_iter()
        .zip(plotting::build_quadrant_specs(
            &traces,
            &opts,
            &Theme::light(),
            true,
        ))
        .filter(|(key, _)| *key != "normalized-mu")
        .map(|(key, spec)| FigureData {
            key,
            label: spec.title,
            xlabel: spec.xlabel,
            ylabel: match key {
                "mu-energy" => "μ(E) (arb. units)".into(),
                "chi-k" => weighted_chi_label(weight),
                "chi-r" => plotting::chir_label(weight),
                "chi-q" => format!("Re χ(q) {}", weighted_unit(weight)),
                _ => spec.ylabel,
            },
            math_ylabel: match key {
                "mu-energy" => "$mu(E)$ (arb. units)".into(),
                "chi-k" => math_chik_label(weight),
                "chi-r" => math_chir_label(weight),
                "chi-q" => format!("$op(\"Re\") chi(q)$ {}", math_unit(weight)),
                _ => String::new(),
            },
            default_xlim: match key {
                "chi-k" => kmax.map(|max| (0., max)),
                "chi-r" => Some((0., 6.)),
                _ => None,
            },
            guides: spec.vlines.into_iter().map(|(x, _, _, _)| x).collect(),
            normalized_series: None,
            series: spec
                .series
                .into_iter()
                .enumerate()
                .map(|(i, s)| FigureSeries {
                    key: format!("series-{i}"),
                    label: s.label.unwrap_or_else(|| {
                        if i == 0 {
                            label.into()
                        } else {
                            format!("Curve {}", i + 1)
                        }
                    }),
                    x: s.x,
                    y: s.y,
                    dashed: s.dashed,
                    optional: false,
                })
                .collect(),
        })
        .collect();
    if let Some(energy) = energy_figure {
        let mut xanes = energy.clone();
        xanes.key = "xanes";
        xanes.label = "XANES · μ(E)".into();
        xanes.default_xlim = e0.map(|edge| (edge - 20., edge + 80.));
        figures.extend([xanes, energy]);
    }
    figures.sort_by_key(|f| match f.key {
        "xanes" => 0,
        "flattened-mu" => 1,
        "chi-k" => 2,
        "chi-r" => 3,
        "mu-energy" => 4,
        _ => 5,
    });
    figures
}

pub(crate) fn fit_figures(result: &FeffFitResult) -> Vec<FigureData> {
    let kw = result.kweight;
    let k: Vec<_> = result.k.iter().copied().collect();
    let r: Vec<_> = result.r.iter().copied().collect();
    let q: Vec<_> = result.q.iter().copied().collect();
    let weighted = |values: &nalgebra::DVector<f64>| {
        values
            .iter()
            .zip(&k)
            .map(|(y, k)| y * k.powf(kw))
            .collect::<Vec<_>>()
    };
    let magnitude = result
        .data_chir_re
        .iter()
        .zip(result.data_chir_im.iter())
        .map(|(re, im)| re.hypot(*im))
        .collect::<Vec<_>>();
    let data_k = weighted(&result.data_chi);
    let model_k = weighted(&result.model_chi);
    let model_r: Vec<_> = result.model_chir_mag.iter().copied().collect();
    let series = |key: &str, label: &str, x: &[f64], y: &[f64], dashed| {
        let n = x.len().min(y.len());
        FigureSeries {
            key: key.into(),
            label: label.into(),
            x: x[..n].to_vec(),
            y: y[..n].to_vec(),
            dashed,
            optional: key != "data" && key != "fit" && key != "residual",
        }
    };
    let figure = |key, label: &str, xlabel: &str, ylabel: String, series| FigureData {
        key,
        label: label.into(),
        xlabel: xlabel.into(),
        ylabel,
        math_ylabel: match key {
            "fit-k" => math_chik_label(kw),
            "fit-r" => math_chir_label(kw),
            "fit-q" => format!("$op(\"Re\") chi(q)$ {}", math_unit(kw)),
            "residual-k" => format!("Residual {}", math_chik_label(kw)),
            "residual-r" => format!("Residual {}", math_chir_label(kw)),
            _ => String::new(),
        },
        series,
        normalized_series: None,
        guides: vec![],
        default_xlim: match key {
            "fit-r" | "residual-r" => Some((0., publication_rmax(result.rmax))),
            "fit-k" | "residual-k" => publication_kmax(result.kmax, Some(&k)).map(|max| (0., max)),
            _ => None,
        },
    };
    let mut output = vec![
        figure(
            "fit-k",
            "Fit · χ(k)",
            "k (Å⁻¹)",
            weighted_chi_label(kw),
            vec![
                series("data", "Data", &k, &data_k, false),
                series("fit", "Fit", &k, &model_k, true),
            ],
        ),
        figure(
            "fit-r",
            "Fit · |χ(R)|",
            "R (Å)",
            plotting::chir_label(kw),
            vec![
                series("data", "Data |χ(R)|", &r, &magnitude, false),
                series("fit", "Fit |χ(R)|", &r, &model_r, true),
            ],
        ),
        figure(
            "fit-q",
            "Fit · χ(q)",
            "q (Å⁻¹)",
            format!("Re χ(q) {}", weighted_unit(kw)),
            vec![
                series("data", "Data", &q, result.data_chiq.as_slice(), false),
                series("fit", "Fit", &q, result.model_chiq.as_slice(), true),
            ],
        ),
        figure(
            "residual-k",
            "Residual · k",
            "k (Å⁻¹)",
            format!("Residual {}", weighted_chi_label(kw)),
            vec![series(
                "residual",
                "Residual",
                &k,
                &data_k
                    .iter()
                    .zip(&model_k)
                    .map(|(a, b)| a - b)
                    .collect::<Vec<_>>(),
                false,
            )],
        ),
        figure(
            "residual-r",
            "Residual · R",
            "R (Å)",
            format!("Residual {}", plotting::chir_label(kw)),
            vec![series(
                "residual",
                "Residual",
                &r,
                &magnitude
                    .iter()
                    .zip(&model_r)
                    .map(|(a, b)| a - b)
                    .collect::<Vec<_>>(),
                false,
            )],
        ),
    ];
    for (key, label, values) in [
        ("real-data", "Re data", &result.data_chir_re),
        ("real-fit", "Re fit", &result.model_chir_re),
        ("imag-data", "Im data", &result.data_chir_im),
        ("imag-fit", "Im fit", &result.model_chir_im),
    ] {
        output[1]
            .series
            .push(series(key, label, &r, values.as_slice(), true));
    }
    for (index, path) in result.path_contributions.iter().enumerate() {
        let key = format!("path-{index}");
        output[0]
            .series
            .push(series(&key, &path.label, &k, &weighted(&path.chi), true));
        output[1].series.push(series(
            &key,
            &path.label,
            &r,
            path.chir_mag.as_slice(),
            true,
        ));
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn publication_presets_follow_processing_ranges_and_keep_full_csv_data() {
        let file = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../rexafs/tests/testfiles/xraylarch_d867/xafsdata/cu_150k.xmu");
        let sp = Arc::new(
            crate::params::process_file(
                &file,
                &PipelineParams {
                    fft_kmin: Some(3.),
                    fft_kmax: Some(10.),
                    fft_kweight: Some(3.),
                    ..Default::default()
                },
            )
            .unwrap(),
        );
        let figures = spectrum_figures(sp.clone(), "Cu_foil #1 [test] $sample");
        let options = FigureOptions::default();
        assert!(options.legend && options.grid.unwrap_or(true));
        assert!(!options.guides);
        assert_eq!(options.dimensions().2, 300.);
        assert_eq!(
            figures
                .iter()
                .filter(|f| f.common())
                .map(|f| f.key)
                .collect::<Vec<_>>(),
            ["xanes", "flattened-mu", "chi-k", "chi-r"]
        );
        let xanes = &figures[0];
        let e0 = sp.e0().unwrap();
        assert_eq!(xanes.default_xlim, Some((e0 - 20., e0 + 80.)));
        assert_eq!(
            figures[1].default_xlim,
            Some((
                sp.energy.as_ref().unwrap()[0],
                sp.energy.as_ref().unwrap().max()
            ))
        );
        let chik = &figures[2];
        assert_eq!(chik.default_xlim, Some((0., 11.)));
        assert_eq!(
            chik.series.len(),
            1,
            "FFT window is not a publication curve"
        );
        assert_eq!(
            chik.series[0].y,
            sp.k()
                .unwrap()
                .iter()
                .zip(sp.chi().unwrap())
                .map(|(k, chi)| chi * k.powf(3.))
                .collect::<Vec<_>>()
        );
        assert!(chik.ylabel.contains("k³"));
        assert_eq!(figures[3].default_xlim, Some((0., 6.)));
        assert_eq!(figures[3].series.len(), 1);
        let normalized = FigureOptions {
            normalized: true,
            ..Default::default()
        };
        assert_eq!(
            xanes.series(&normalized)[0].y,
            sp.norm().unwrap().as_slice()
        );
        assert!(xanes.csv(&normalized).unwrap().contains("Normalized μ(E)"));
        assert_eq!(
            xanes.csv(&options).unwrap().lines().count(),
            sp.energy.as_ref().unwrap().len() + 1
        );
        if let Some(dir) = std::env::var_os("REXAFS_PUBLICATION_QA_DIR") {
            let dir = Path::new(&dir);
            fs::create_dir_all(dir).unwrap();
            for f in &figures {
                let rendered = render_figure(f, &options).unwrap();
                assert!(rendered.svg.contains("data-ruviz-text-engine=\"typst\""));
                fs::write(dir.join(format!("{}.png", f.key)), rendered.png).unwrap();
                fs::write(dir.join(format!("{}.svg", f.key)), rendered.svg).unwrap();
            }
            fs::write(
                dir.join("xanes-normalized.png"),
                render_figure(xanes, &normalized).unwrap().png,
            )
            .unwrap();
        }
    }

    #[test]
    fn visible_interval_sets_y_scale_without_cropping_export_or_user_limits() {
        let mut data = sample();
        data.default_xlim = Some((0., 1.));
        data.series[0].y[2] = 1000.;
        let options = FigureOptions::default();
        assert_eq!(data.visible_ylim(&options), Some((-0.05, 1.05)));
        assert_eq!(data.csv(&options).unwrap().lines().count(), 4);
        let options = FigureOptions {
            xmin: Some(1.),
            xmax: Some(2.),
            ..Default::default()
        };
        assert_eq!(data.xlim(&options), Some((1., 2.)));
        assert!(data.visible_ylim(&options).unwrap().1 > 1000.);
    }

    #[test]
    fn fit_r_defaults_show_magnitudes_and_expand_only_for_longer_fits() {
        let mut result = FeffFitResult {
            r: nalgebra::DVector::from_vec(vec![0., 2., 4., 6., 8.]),
            data_chir_re: nalgebra::DVector::from_vec(vec![0., 1., 0.5, 0.1, 0.]),
            data_chir_im: nalgebra::DVector::from_vec(vec![0., 0.2, 0.1, 0., 0.]),
            model_chir_mag: nalgebra::DVector::from_vec(vec![0., 0.95, 0.4, 0.1, 0.]),
            ..Default::default()
        };
        for (fit_max, display_max) in [
            (None, 6.),
            (Some(4.), 6.),
            (Some(6.), 6.),
            (Some(6.5), 7.),
            (Some(7.), 8.),
        ] {
            result.rmax = fit_max;
            let figures = fit_figures(&result);
            let fit = figures.iter().find(|f| f.key == "fit-r").unwrap();
            let options = FigureOptions::default();
            assert_eq!(fit.default_xlim, Some((0., display_max)));
            assert_eq!(
                fit.series
                    .iter()
                    .filter(|s| s.visible(&options))
                    .map(|s| s.key.as_str())
                    .collect::<Vec<_>>(),
                ["data", "fit"]
            );
            let options = FigureOptions {
                shown: BTreeSet::from(["real-data".into()]),
                ..Default::default()
            };
            assert!(
                fit.series
                    .iter()
                    .find(|s| s.key == "real-data")
                    .unwrap()
                    .visible(&options)
            );
            if fit_max == Some(4.)
                && let Some(dir) = std::env::var_os("REXAFS_PUBLICATION_QA_DIR")
            {
                fs::create_dir_all(&dir).unwrap();
                fs::write(
                    Path::new(&dir).join("fit-r.png"),
                    render_figure(fit, &FigureOptions::default()).unwrap().png,
                )
                .unwrap();
            }
        }
    }
    #[test]
    fn flattened_publication_preserves_library_values_in_figure_and_csv() {
        let file = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../rexafs/tests/testfiles/xraylarch_d867/xafsdata/cu_150k.xmu");
        let sp = Arc::new(crate::params::process_file(&file, &PipelineParams::default()).unwrap());
        let figures = spectrum_figures(sp.clone(), "Cu, \"reference\"");
        let flat = figures.iter().find(|f| f.key == "flattened-mu").unwrap();
        let norm = &flat.normalized_series.as_ref().unwrap()[0];
        assert_eq!(flat.series[0].y, sp.flat().unwrap().as_slice());
        assert_eq!(norm.y, sp.norm().unwrap().as_slice());
        assert_ne!(flat.series[0].y, norm.y);
        let csv = flat.csv(&FigureOptions::default()).unwrap();
        assert!(csv.starts_with("\"Cu, \"\"reference\"\""));
        for (row, (energy, value)) in csv
            .lines()
            .skip(1)
            .zip(flat.series[0].x.iter().zip(&flat.series[0].y))
        {
            let values: Vec<f64> = row.split(',').map(|s| s.parse().unwrap()).collect();
            assert_eq!(values, [*energy, *value]);
        }
        assert_eq!(csv.lines().count(), sp.energy.as_ref().unwrap().len() + 1);
        let empty = spectrum_figures(Arc::new(XASSpectrum::default()), "unprocessed");
        assert!(empty.iter().all(|f| f.key != "flattened-mu"));
    }

    #[test]
    fn csv_preserves_distinct_grids_and_respects_hidden_curves() {
        let mut figure = sample();
        figure.series.push(FigureSeries {
            key: "short".into(),
            label: "Other".into(),
            x: vec![0.5],
            y: vec![0.25],
            dashed: false,
            optional: false,
        });
        let csv = figure.csv(&FigureOptions::default()).unwrap();
        assert_eq!(csv.lines().count(), 4);
        assert!(csv.lines().nth(2).unwrap().ends_with(",,"));
        let hidden = FigureOptions {
            hidden: BTreeSet::from(["short".into()]),
            ..Default::default()
        };
        assert!(!figure.csv(&hidden).unwrap().contains("Other"));
        let hidden = FigureOptions {
            hidden: BTreeSet::from(["short".into(), "data".into()]),
            ..Default::default()
        };
        assert!(figure.csv(&hidden).is_err());
    }

    fn sample() -> FigureData {
        FigureData {
            key: "test",
            label: "Test".into(),
            xlabel: "Energy (eV)".into(),
            ylabel: "μ(E)".into(),
            math_ylabel: "$mu(E)$".into(),
            normalized_series: None,
            default_xlim: None,
            guides: vec![],
            series: vec![FigureSeries {
                key: "data".into(),
                label: "Data".into(),
                x: vec![0., 1., 2.],
                y: vec![0., 1., 0.],
                dashed: false,
                optional: false,
            }],
        }
    }
    fn png_size(bytes: &[u8]) -> (u32, u32) {
        (
            u32::from_be_bytes(bytes[16..20].try_into().unwrap()),
            u32::from_be_bytes(bytes[20..24].try_into().unwrap()),
        )
    }
    #[test]
    fn publication_defaults_to_300_dpi_and_custom_dpi_preserves_shape() {
        let defaults = Plot::new().dpi(300);
        let canvas = defaults.get_config().canvas_size();
        let rendered = render_figure(&sample(), &FigureOptions::default()).unwrap();
        assert_eq!(png_size(&rendered.png), canvas);
        assert!(rendered.svg.contains("Energy"));
        let options = FigureOptions {
            width: Some(4.),
            height: Some(3.),
            dpi: Some(200.),
            title: Some("Copper · $k^2 chi(k)$".into()),
            xlabel: Some("Photon energy".into()),
            ..Default::default()
        };
        let rendered = render_figure(&sample(), &options).unwrap();
        assert_eq!(png_size(&rendered.png), (800, 600));
        assert!(rendered.svg.contains("data-ruviz-text-engine=\"typst\""));
        assert!(rendered.svg.contains("class=\"typst-doc\""));
        assert!(rendered.svg.contains("Photon energy"));
    }
    #[test]
    fn invalid_output_dimensions_and_limits_are_rejected_before_rendering() {
        for options in [
            FigureOptions {
                width: Some(f64::NAN),
                ..Default::default()
            },
            FigureOptions {
                dpi: Some(0.),
                ..Default::default()
            },
            FigureOptions {
                width: Some(30.),
                height: Some(30.),
                dpi: Some(1200.),
                ..Default::default()
            },
            FigureOptions {
                xmin: Some(2.),
                xmax: Some(1.),
                ..Default::default()
            },
            FigureOptions {
                xmin: Some(0.),
                ..Default::default()
            },
        ] {
            assert!(render_figure(&sample(), &options).is_err());
        }
    }
}
