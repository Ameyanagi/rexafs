//! Render the synthetic-copper tutorial with ruviz, without refitting data.
//!
//! Run `cu_reduction_check <inputs> --desktop-defaults` first. Then pass its
//! `tutorial-results.json` and an output directory to this example with the
//! `plotting` feature. Fractions are dimensionless raw-mu coefficients, errors
//! are percentage points, and PCA contributions are uncentered squared signal.
//! The report explicitly identifies the approximate edge-step conversion.

#[cfg(not(feature = "plotting"))]
fn main() {
    eprintln!("Enable ruviz with --features plotting");
    std::process::exit(1);
}

#[cfg(feature = "plotting")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use ruviz::{
        core::{TextAlign, TextStyle},
        prelude::*,
    };
    use serde_json::Value;
    use std::{fs, path::PathBuf};

    let input = std::env::args()
        .nth(1)
        .ok_or("Pass tutorial-results.json")?;
    let output = PathBuf::from(std::env::args().nth(2).ok_or("Pass an output directory")?);
    let report: Value = serde_json::from_slice(&fs::read(input)?)?;
    let truth: Vec<Vec<f64>> = serde_json::from_value(report["raw_truth"].clone())?;
    let variance: Vec<f64> = serde_json::from_value(report["pca"]["variance_explained"].clone())?;
    let frames: Vec<f64> = (1..=truth.len()).map(|i| i as f64).collect();
    let colors = [
        Color::from_hex("#b64b35")?,
        Color::from_hex("#bd8818")?,
        Color::from_hex("#2477a6")?,
    ];
    let names = ["CuO", "Cu₂O", "Cu"];
    fs::create_dir_all(&output)?;
    let width = 180. / 25.4 * 72.;
    let recipe_height = 252.;
    let mut recipe = publication_panel(width, recipe_height - 27., 27., 34.)
        .title("Known synthetic recipe")
        .xlabel("Synthetic frame (no time unit)")
        .ylabel("Raw μ mixing weight (%)")
        .xlim(0., 51.)
        .ylim(-2., 103.)
        .major_ticks_x(6)
        .major_ticks_y(6);
    let styles = [LineStyle::Solid, LineStyle::Dashed, LineStyle::Dotted];
    let mut recipe_legend = publication_strip(width, 27.);
    for j in 0..3 {
        let values: Vec<f64> = truth.iter().map(|r| 100. * r[j]).collect();
        recipe = recipe
            .line(&frames, &values)
            .color(colors[j])
            .line_width(1.1)
            .line_style(styles[j].clone())
            .into();
        let x = 122. + 105. * j as f64;
        recipe_legend = recipe_legend
            .line(&[x, x + 20.], &[13., 13.])
            .color(colors[j])
            .line_width(1.1)
            .line_style(styles[j].clone())
            .into();
        recipe_legend = recipe_legend.text_styled(
            x + 27.,
            13.,
            names[j],
            TextStyle::new().font_size(8.5).align(TextAlign::Left),
        );
    }
    save_publication_panels(
        &output,
        "recipe",
        "Known raw-absorption fractions across 50 synthetic frames",
        width,
        recipe_height,
        vec![(0., 0., recipe_legend), (0., 27., recipe)],
    )?;

    let pc: Vec<f64> = (1..=6).map(|i| i as f64).collect();
    let values: Vec<f64> = variance.iter().take(6).map(|x| 100. * x).collect();
    let pca_height = 216.;
    let panel_width = width / 2.;
    let scree = publication_panel(panel_width, pca_height, 31., 35.)
        .title("a   Uncentered PCA")
        .xlabel("Principal component")
        .ylabel("Squared signal (%)")
        .xlim(0.8, 6.2)
        .ylim(-2., 103.)
        .major_ticks_x(6)
        .major_ticks_y(6)
        .line(&pc, &values)
        .color(colors[2])
        .line_width(1.)
        .scatter(&pc, &values)
        .marker(MarkerStyle::Circle)
        .marker_size(3.5)
        .color(Color::WHITE)
        .edge_color(colors[2])
        .edge_width(0.65)
        .into();
    let detail_pc = &pc[1..];
    let detail_values = &values[1..];
    let zoom = publication_panel(panel_width, pca_height, 31., 35.)
        .title("b   PC2–PC6 detail")
        .xlabel("Principal component")
        .ylabel("Squared signal (%)")
        .xlim(1.8, 6.2)
        .ylim(-0.005, 0.25)
        .major_ticks_x(5)
        .major_ticks_y(6)
        .line(detail_pc, detail_values)
        .color(colors[2])
        .line_width(1.)
        .scatter(&detail_pc, &detail_values)
        .marker(MarkerStyle::Circle)
        .marker_size(3.5)
        .color(Color::WHITE)
        .edge_color(colors[2])
        .edge_width(0.65)
        .into();
    save_publication_panels(
        &output,
        "pca",
        "Uncentered PCA with linear axes and a separate PC2–PC6 detail panel",
        width,
        pca_height,
        vec![(0., 0., scree), (panel_width, 0., zoom)],
    )?;

    comparison_figure(&report, &truth, &frames, &output)?;
    println!("Rendered recipe, pca and comparison as SVG and 600 dpi PNG with ruviz 0.14.2");
    Ok(())
}

/// Lay out the comparison at 180 mm width, with point-sized publication text.
///
/// All 50 estimates are shown. Species have redundant color and marker encodings;
/// both error panels use the same limits. No data are interpolated or decimated.
#[cfg(feature = "plotting")]
fn comparison_figure(
    report: &serde_json::Value,
    truth: &[Vec<f64>],
    frames: &[f64],
    output: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    use ruviz::{
        core::{TextAlign, TextStyle},
        prelude::*,
    };

    let width = 180. / 25.4 * 72.;
    let height = 414.;
    let panel_width = width / 2.;
    let colors = [
        Color::from_hex("#b64b35")?,
        Color::from_hex("#bd8818")?,
        Color::from_hex("#2477a6")?,
    ];
    let markers = [
        MarkerStyle::Circle,
        MarkerStyle::Square,
        MarkerStyle::Triangle,
    ];
    let names = ["CuO", "Cu₂O", "Cu"];
    let text = TextStyle::new().font_size(8.).align(TextAlign::Left);
    let panel = |panel_height: f32, bottom: f32| {
        publication_panel(panel_width, panel_height, 27., bottom)
            .major_ticks_x(6)
            .xlim(0., 51.)
    };
    let strip = |strip_height| publication_strip(width, strip_height);
    let mut legend = strip(27.);
    for j in 0..3 {
        let x = 43. + j as f64 * 61.;
        legend = legend
            .line(&[x], &[13.])
            .color(Color::WHITE)
            .marker(markers[j])
            .marker_edge_color(colors[j])
            .marker_edge_width(0.65)
            .marker_size(3.6)
            .into();
        legend = legend.text_styled(x + 8., 13., names[j], text.clone());
    }
    legend = legend
        .line(&[262., 280.], &[13., 13.])
        .color(Color::BLACK)
        .line_width(0.9)
        .into();
    legend = legend.text_styled(286., 13., "Recipe", text.clone());
    legend = legend
        .line(&[358.], &[13.])
        .color(Color::WHITE)
        .marker(MarkerStyle::Circle)
        .marker_edge_color(Color::BLACK)
        .marker_edge_width(0.65)
        .marker_size(3.6)
        .into();
    legend = legend.text_styled(366., 13., "Recovered", text.clone());
    let mut panels = vec![(0., 0., legend)];

    for (col, method) in ["MCR", "LCF"].iter().enumerate() {
        let result = &report["methods"][method];
        let estimated: Vec<Vec<f64>> =
            serde_json::from_value(result["raw_basis_estimates"].clone())?;
        let mut fractions = panel(193., 19.)
            .title(if col == 0 {
                "a   MCR–ALS (blind)"
            } else {
                "b   LCF (known references)"
            })
            .ylabel("Fraction (%)")
            .ylim(-3., 104.)
            .major_ticks_y(6);
        let mut errors = panel(169., 34.)
            .title(if col == 0 {
                "c   MCR–ALS error"
            } else {
                "d   LCF error"
            })
            .xlabel("Synthetic frame")
            .ylabel("Estimate − recipe (pp)")
            .ylim(-3.1, 3.1)
            .major_ticks_y(7);
        // A light zero reference makes the signed errors readable without a grid.
        errors = errors
            .line(&[0., 51.], &[0., 0.])
            .color(Color::from_hex("#949494")?)
            .line_width(0.5)
            .into();
        for j in 0..3 {
            let recipe: Vec<f64> = truth.iter().map(|r| 100. * r[j]).collect();
            let recovered: Vec<f64> = estimated.iter().map(|r| 100. * r[j]).collect();
            let delta: Vec<f64> = recovered.iter().zip(&recipe).map(|(a, b)| a - b).collect();
            fractions = fractions
                .line(frames, &recipe)
                .color(colors[j])
                .line_width(0.9)
                .scatter(&frames, &recovered)
                .color(Color::WHITE)
                .marker(markers[j])
                .edge_color(colors[j])
                .edge_width(0.5)
                .marker_size(2.6)
                .into();
            errors = errors
                .line(frames, &delta)
                .color(colors[j])
                .line_width(0.8)
                .scatter(&frames, &delta)
                .color(Color::WHITE)
                .marker(markers[j])
                .edge_color(colors[j])
                .edge_width(0.5)
                .marker_size(2.3)
                .into();
        }
        let mean = result["mean_absolute_error_pp"]
            .as_f64()
            .ok_or("Missing mean error")?;
        let max = result["max_absolute_error_pp"]
            .as_f64()
            .ok_or("Missing maximum error")?;
        let stats = text.clone().font_size(7.5);
        errors = errors
            .text_styled(
                2.,
                2.6,
                format!("Mean absolute error: {mean:.3} pp"),
                stats.clone(),
            )
            .text_styled(
                2.,
                2.02,
                format!("Maximum absolute error: {max:.3} pp"),
                stats,
            );
        let x = col as f32 * panel_width;
        panels.push((x, 27., fractions));
        panels.push((x, 229., errors));
    }
    let footer = strip(16.).text_styled(
        39.,
        8.,
        "Approximate raw μ basis after edge-step conversion; pp = percentage points.",
        text.font_size(7.5),
    );
    panels.push((0., 398., footer));
    save_publication_panels(
        output,
        "comparison",
        "Recovery of known fractions in 50 synthetic copper spectra",
        width,
        height,
        panels,
    )
}

/// Draw a compact legend or note without axes, using the figure's point units.
#[cfg(feature = "plotting")]
fn publication_strip(width: f32, height: f32) -> ruviz::core::Plot {
    use ruviz::core::{MarginConfig, Plot, PlotConfig, SpineConfig};
    Plot::with_config(
        PlotConfig::builder()
            .figure(width / 72., height / 72.)
            .dpi(72.)
            .font_family("Helvetica")
            .margins(MarginConfig::fixed_uniform(0.))
            .spines(SpineConfig::none())
            .build(),
    )
    .grid(false)
    .ticks(false)
    .xlim(0., f64::from(width))
    .ylim(0., f64::from(height))
}

/// Use the same physical typography and axes across all tutorial figures.
#[cfg(feature = "plotting")]
fn publication_panel(width: f32, height: f32, top: f32, bottom: f32) -> ruviz::core::Plot {
    use ruviz::core::{MarginConfig, Plot, PlotConfig, SpineConfig};
    Plot::with_config(
        PlotConfig::builder()
            .figure(width / 72., height / 72.)
            .dpi(72.)
            .font_family("Helvetica")
            .font_size(8.5)
            .typography(|mut t| {
                t.title_scale = 1.18;
                t.label_scale = 1.;
                t.tick_scale = 0.94;
                t
            })
            .lines(|mut l| {
                l.axis_width = 0.6;
                l.tick_width = 0.6;
                l.tick_length = 3.;
                l
            })
            .spacing(|mut s| {
                s.title_pad = 8.;
                s.label_pad = 5.;
                s.tick_pad = 3.;
                s
            })
            .margins(MarginConfig::fixed(
                39. / 72.,
                10. / 72.,
                top / 72.,
                bottom / 72.,
            ))
            .spines(SpineConfig::minimal())
            .build(),
    )
    .grid(false)
    .ticks_bottom_left()
    .tick_direction_outside()
    .minor_ticks_x(0)
    .minor_ticks_y(0)
}

/// Compose the same ruviz panels into a vector SVG and a 600 dpi PNG.
///
/// The layout uses points (1/72 inch). SVG IDs are scoped per panel so clipping
/// remains correct when their independently rendered vector contents are joined.
/// Raster panels are copied at their final resolution, without resampling.
#[cfg(feature = "plotting")]
fn save_publication_panels(
    output: &std::path::Path,
    name: &str,
    title: &str,
    width_pt: f32,
    height_pt: f32,
    panels: Vec<(f32, f32, ruviz::core::Plot)>,
) -> Result<(), Box<dyn std::error::Error>> {
    use ruviz::core::plot::Image;
    use std::{fmt::Write as _, fs};

    const DPI: u32 = 600;
    let px = |points: f32| (points * DPI as f32 / 72.).round() as u32;
    let width = px(width_pt);
    let height = px(height_pt);
    let mut image = Image::new(
        width,
        height,
        vec![255; width as usize * height as usize * 4],
    );
    let mut svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="180mm" height="{}mm" viewBox="0 0 {width_pt} {height_pt}">
<title>{title}</title>
<rect width="100%" height="100%" fill="white"/>
"#,
        height_pt / 72. * 25.4
    );
    for (i, (x, y, plot)) in panels.into_iter().enumerate() {
        // Render in a high-resolution coordinate system so short legend strips
        // also meet ruviz's minimum canvas dimensions, then map back to points.
        let plot = plot.dpi(DPI);
        let source = plot.render_to_svg()?;
        let start = source.find("<svg ").ok_or("Missing SVG element")?;
        let content_start = start + source[start..].find('>').ok_or("Unclosed SVG element")? + 1;
        let content_end = source
            .rfind("</svg>")
            .ok_or("Missing SVG closing element")?;
        let content = source[content_start..content_end]
            .replace("id=\"", &format!("id=\"panel{i}_"))
            .replace("url(#", &format!("url(#panel{i}_"))
            .replace("href=\"#", &format!("href=\"#panel{i}_"));
        writeln!(
            svg,
            "<g transform=\"translate({x} {y}) scale({})\">{content}</g>",
            72. / DPI as f32
        )?;

        let raster = plot.render()?;
        let (left, top) = (px(x), px(y));
        if left + raster.width > width || top + raster.height > height {
            return Err("Publication panel exceeds canvas".into());
        }
        for row in 0..raster.height as usize {
            let destination = ((top as usize + row) * width as usize + left as usize) * 4;
            let source = row * raster.width as usize * 4;
            image.pixels[destination..destination + raster.width as usize * 4]
                .copy_from_slice(&raster.pixels[source..source + raster.width as usize * 4]);
        }
    }
    svg.push_str("</svg>\n");
    fs::write(output.join(format!("{name}.svg")), svg)?;
    fs::write(
        output.join(format!("{name}.png")),
        image.encode_png_with_dpi(DPI as f32)?,
    )?;
    Ok(())
}
