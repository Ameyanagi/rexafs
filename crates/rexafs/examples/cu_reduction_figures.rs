//! Render the synthetic-copper tutorial with ruviz, without refitting data.
//!
//! Run `cu_reduction_check <inputs> --desktop-defaults` first. Then pass its
//! `tutorial-results.json` and an output directory to this example with the
//! `plotting` feature. Fractions are dimensionless raw-mu coefficients, errors
//! are percentage points, and PCA contributions are uncentered squared signal.
//! The report explicitly identifies the approximate edge-step conversion.
//!
//! Every panel uses `Plot::new()` styling, with a compact export canvas. Scientific
//! content, axis ranges and panel arrangement are specified; labeled series use
//! ruviz's automatic legend placement. Export resolution is 600 dpi.

#[cfg(not(feature = "plotting"))]
fn main() {
    eprintln!("Enable ruviz with --features plotting");
    std::process::exit(1);
}

#[cfg(feature = "plotting")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
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
    let names = ["CuO", "Cu2O", "Cu"];
    fs::create_dir_all(&output)?;

    let (width, height) = panel_size_points();
    let mut recipe = figure_panel()
        .title("Known synthetic recipe")
        .xlabel("Synthetic frame (no time unit)")
        .ylabel("Raw μ mixing weight (%)")
        .xlim(0., 51.)
        .ylim(-2., 103.)
        .legend_best();
    for j in 0..3 {
        let values: Vec<f64> = truth.iter().map(|r| 100. * r[j]).collect();
        recipe = recipe.line(&frames, &values).label(names[j]).into();
    }
    save_figure_panels(
        &output,
        "recipe",
        "Known raw-absorption fractions across 50 synthetic frames",
        width,
        height,
        vec![(0., 0., recipe)],
    )?;

    let pc: Vec<f64> = (1..=6).map(|i| i as f64).collect();
    let values: Vec<f64> = variance.iter().take(6).map(|x| 100. * x).collect();
    let scree = figure_panel()
        .title("a   Uncentered PCA")
        .xlabel("Principal component")
        .ylabel("Squared signal (%)")
        .xlim(0.8, 6.2)
        .ylim(-2., 103.)
        .group(|g| g.line(&pc, &values).scatter(&pc, &values));
    let detail_pc = &pc[1..];
    let detail_values = &values[1..];
    let zoom = figure_panel()
        .title("b   PC2–PC6 detail")
        .xlabel("Principal component")
        .ylabel("Squared signal (%)")
        .xlim(1.8, 6.2)
        .major_ticks_x(5)
        .ylim(-0.005, 0.25)
        .group(|g| {
            g.line(&detail_pc, &detail_values)
                .scatter(&detail_pc, &detail_values)
        });
    save_figure_panels(
        &output,
        "pca",
        "Uncentered PCA with linear axes and a separate PC2–PC6 detail panel",
        2. * width,
        height,
        vec![(0., 0., scree), (width, 0., zoom)],
    )?;

    comparison_figure(&report, &truth, &frames, &output)?;
    println!("Rendered recipe, pca and comparison with ruviz 0.14.2 default styling as SVG and 600 dpi PNG");
    Ok(())
}

/// Compare all 50 estimates with the recipe using ruviz's default visual styles.
///
/// Grouping gives each species one automatic palette color for its recipe line
/// and recovered points. Both error panels use the same limits. No data are
/// interpolated or decimated. Errors are measured after the approximate
/// reference edge-step conversion to raw-absorption mixing weights.
#[cfg(feature = "plotting")]
fn comparison_figure(
    report: &serde_json::Value,
    truth: &[Vec<f64>],
    frames: &[f64],
    output: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let (width, height) = panel_size_points();
    let names = ["CuO", "Cu2O", "Cu"];
    let mut panels = Vec::new();
    for (col, method) in ["MCR", "LCF"].iter().enumerate() {
        let result = &report["methods"][method];
        let estimated: Vec<Vec<f64>> =
            serde_json::from_value(result["raw_basis_estimates"].clone())?;
        let mut fractions = figure_panel()
            .title(if col == 0 {
                "a   MCR–ALS (blind)"
            } else {
                "b   LCF (known references)"
            })
            .xlabel("Synthetic frame (line: recipe; points: estimate)")
            .ylabel("Approximate raw μ fraction (%)")
            .xlim(0., 51.)
            .ylim(-3., 104.)
            .legend_best();
        let mut errors = figure_panel()
            .title(if col == 0 {
                "c   MCR–ALS error"
            } else {
                "d   LCF error"
            })
            .xlabel("Synthetic frame")
            .ylabel("Estimate − recipe (percentage points)")
            .xlim(0., 51.)
            .ylim(-3.1, 3.1)
            .legend_best();
        for j in 0..3 {
            let recipe: Vec<f64> = truth.iter().map(|r| 100. * r[j]).collect();
            let recovered: Vec<f64> = estimated.iter().map(|r| 100. * r[j]).collect();
            let delta: Vec<f64> = recovered.iter().zip(&recipe).map(|(a, b)| a - b).collect();
            fractions = fractions.group(|g| {
                g.group_label(names[j])
                    .line(&frames, &recipe)
                    .scatter(&frames, &recovered)
            });
            errors = errors.group(|g| {
                g.group_label(names[j])
                    .line(&frames, &delta)
                    .scatter(&frames, &delta)
            });
        }
        let mean = result["mean_absolute_error_pp"]
            .as_f64()
            .ok_or("Missing mean error")?;
        let max = result["max_absolute_error_pp"]
            .as_f64()
            .ok_or("Missing maximum error")?;
        errors = errors
            .text(25., 2.6, format!("Mean absolute error: {mean:.3} pp"))
            .text(25., 2.15, format!("Maximum absolute error: {max:.3} pp"));
        let x = col as f32 * width;
        panels.push((x, 0., fractions));
        panels.push((x, height, errors));
    }
    save_figure_panels(
        output,
        "comparison",
        "Recovery of known fractions in 50 synthetic copper spectra; approximate raw μ basis after edge-step conversion; pp = percentage points",
        2. * width,
        2. * height,
        panels,
    )
}

/// Keep default styling and aspect ratio, with a compact canvas for downloads.
#[cfg(feature = "plotting")]
fn figure_panel() -> ruviz::core::Plot {
    ruviz::core::Plot::new().size(5.4, 4.05)
}

/// Read the export panel dimensions, converting inches to points (1/72 inch).
#[cfg(feature = "plotting")]
fn panel_size_points() -> (f32, f32) {
    let plot = figure_panel();
    let figure = &plot.get_config().figure;
    (figure.width * 72., figure.height * 72.)
}

/// Compose the same ruviz panels into a vector SVG and a 600 dpi PNG.
///
/// The layout uses points (1/72 inch). SVG IDs are scoped per panel so clipping
/// remains correct when their independently rendered vector contents are joined.
/// Raster panels are copied at their final resolution, without resampling.
#[cfg(feature = "plotting")]
fn save_figure_panels(
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
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}mm" height="{}mm" viewBox="0 0 {width_pt} {height_pt}">
<title>{title}</title>
<rect width="100%" height="100%" fill="white"/>
"#,
        width_pt / 72. * 25.4,
        height_pt / 72. * 25.4
    );
    for (i, (x, y, plot)) in panels.into_iter().enumerate() {
        // Use the same resolution for both renderers, then map SVG units to points.
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
            return Err("Figure panel exceeds canvas".into());
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
