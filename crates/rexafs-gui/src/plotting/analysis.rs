//! Collection diagnostics. Plot values are derived from the retained result,
//! never from current form settings or a differently processed spectrum.

use super::*;
use rexafs::prelude::{AnalysisSpace, McrResult, PcaModel};

/// Display scaling only; retained PCA values and exported results are unchanged.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum PcaScale {
    Linear,
    #[default]
    Log,
}

impl PcaScale {
    pub fn label(self) -> &'static str {
        match self {
            Self::Linear => "Linear",
            Self::Log => "Log",
        }
    }

    fn axis(self) -> ruviz::axes::AxisScale {
        match self {
            Self::Linear => ruviz::axes::AxisScale::Linear,
            Self::Log => ruviz::axes::AxisScale::Log,
        }
    }

    fn display_value(self, value: f64) -> f64 {
        match self {
            Self::Linear => value,
            Self::Log => value.max(1e-32),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum PcaView {
    #[default]
    Scree,
    Cumulative,
    Residual,
    Indicator,
    Scores12,
    Scores13,
    Scores23,
    Loadings,
    Similarity,
    Target,
}

impl PcaView {
    pub const ALL: [Self; 10] = [
        Self::Scree,
        Self::Cumulative,
        Self::Residual,
        Self::Indicator,
        Self::Scores12,
        Self::Scores13,
        Self::Scores23,
        Self::Loadings,
        Self::Similarity,
        Self::Target,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Scree => "Scree",
            Self::Cumulative => "Cumulative",
            Self::Residual => "Error vs count",
            Self::Indicator => "IND",
            Self::Scores12 => "PC1 / PC2",
            Self::Scores13 => "PC1 / PC3",
            Self::Scores23 => "PC2 / PC3",
            Self::Loadings => "Loadings",
            Self::Similarity => "Similarity",
            Self::Target => "Reconstruction",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum McrView {
    #[default]
    Spectra,
    Fractions,
    Convergence,
    Residuals,
    References,
}

impl McrView {
    pub const ALL: [Self; 5] = [
        Self::Spectra,
        Self::Fractions,
        Self::Convergence,
        Self::Residuals,
        Self::References,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Spectra => "Spectra",
            Self::Fractions => "Fractions",
            Self::Convergence => "Convergence",
            Self::Residuals => "Residuals",
            Self::References => "References",
        }
    }
}

/// Presentation labels for the core's shared count suggestion.
pub(crate) fn pca_count_suggestion(model: &PcaModel) -> Option<(usize, &'static str)> {
    model.component_count_suggestion().map(|suggestion| {
        (
            suggestion.count,
            match suggestion.basis {
                rexafs::prelude::PcaCountBasis::NumericalRank => "numerical rank",
                rexafs::prelude::PcaCountBasis::IndicatorMinimum => "IND heuristic",
            },
        )
    })
}

pub(crate) fn analysis_axis_labels(space: AnalysisSpace) -> (String, String) {
    match space {
        AnalysisSpace::Norm => ("Energy (eV)".into(), "normalized μ(E)".into()),
        AnalysisSpace::Flat => ("Energy (eV)".into(), "flattened μ(E)".into()),
        AnalysisSpace::Deriv => ("Energy (eV)".into(), "dμ/dE (eV⁻¹)".into()),
        AnalysisSpace::Chi { kweight } => (K_AXIS.into(), chik_label(kweight)),
    }
}

pub(crate) fn build_pca_diagnostic(
    model: &PcaModel,
    view: PcaView,
    retained: usize,
    all: bool,
    scale: PcaScale,
    theme: &Theme,
) -> Plot {
    let base = || Plot::new().theme(theme.plot_theme());
    let count = if all {
        model.n_components()
    } else {
        model.n_components().min(12)
    };
    let x: Vec<_> = (1..=count).map(|i| i as f64).collect();
    let fraction_label = if model.centered {
        "Explained variance"
    } else {
        "Explained squared signal"
    };
    match view {
        PcaView::Scree => {
            // Floor is for display only. Raw eigenvalues remain in the model/export.
            let y: Vec<_> = model
                .variance_explained
                .iter()
                .take(count)
                .map(|&v| scale.display_value(v))
                .collect();
            let p: Plot = base()
                .line(&x, &y)
                .color(trace_color(theme, 0))
                .scatter(&x, &y)
                .color(trace_color(theme, 0))
                .into();
            p.xlabel("Principal component")
                .ylabel(format!("{fraction_label} fraction"))
                .yscale(scale.axis())
        }
        PcaView::Cumulative => {
            let y: Vec<_> = model
                .cumulative_variance
                .iter()
                .take(count)
                .map(|v| 100.0 * v)
                .collect();
            let p: Plot = base()
                .line(&x, &y)
                .color(trace_color(theme, 0))
                .scatter(&x, &y)
                .color(trace_color(theme, 0))
                .into();
            p.xlabel("Retained components")
                .ylabel(format!("Cumulative {fraction_label} (%)"))
                .ylim(0.0, 100.1)
        }
        PcaView::Indicator => {
            let x: Vec<_> = (1..=count.min(model.ind.len().saturating_sub(1)))
                .filter(|&k| model.ind[k].is_finite() && model.ind[k] >= 0.0)
                .map(|k| k as f64)
                .collect();
            let y: Vec<_> = x
                .iter()
                .map(|&k| scale.display_value(model.ind[k as usize]))
                .collect();
            let p: Plot = base()
                .line(&x, &y)
                .color(trace_color(theme, 0))
                .scatter(&x, &y)
                .color(trace_color(theme, 0))
                .into();
            p.xlabel("Retained components k")
                .ylabel("Malinowski IND(k)")
                .yscale(scale.axis())
        }
        PcaView::Scores12 | PcaView::Scores13 | PcaView::Scores23 => {
            let (a, b) = match view {
                PcaView::Scores13 => (0, 2),
                PcaView::Scores23 => (1, 2),
                _ => (0, 1),
            };
            let mut p = base()
                .xlabel(format!("PC{} score", a + 1))
                .ylabel(format!("PC{} score", b + 1));
            if b < model.scores.ncols() {
                // Each trace is one sample, preserving its label in hover inspection.
                for i in 0..model.n_spectra() {
                    p = p
                        .scatter(&[model.scores[(i, a)]], &[model.scores[(i, b)]])
                        .color(trace_color(theme, i))
                        .label(model.labels[i].clone())
                        .into();
                }
            }
            p
        }
        PcaView::Loadings => {
            let (xlabel, _) = analysis_axis_labels(model.space);
            let mut p = base()
                .xlabel(xlabel)
                .ylabel("Orthonormal loading (dimensionless)");
            for i in 0..retained.min(model.n_components()) {
                let y: Vec<_> = model.components.row(i).iter().copied().collect();
                p = p
                    .line(model.x.as_slice(), &y)
                    .color(trace_color(theme, i))
                    .label(format!("PC{}", i + 1))
                    .into();
            }
            p.legend_position(LegendPosition::UpperRight)
        }
        PcaView::Similarity => {
            let matrix = pca_similarity(model, retained);
            let n = model.n_spectra() as f64;
            let p: Plot = base()
                .heatmap_with(
                    &matrix,
                    HeatmapConfig::new()
                        .colorbar(true)
                        .origin(HeatmapOrigin::Lower)
                        .extent(0.5, n + 0.5, 0.5, n + 0.5)
                        .vmin(-1.0)
                        .vmax(1.0),
                )
                .into();
            p.xlabel("Sample number")
                .ylabel("Sample number · cosine similarity")
        }
        PcaView::Target | PcaView::Residual => base(), // Target overlay is constructed from the retained PcaFit.
    }
}

/// Cosine similarity in the retained score subspace. Undefined zero vectors
/// remain NaN (missing cells), rather than appearing to match another sample.
fn pca_similarity(model: &PcaModel, retained: usize) -> Vec<Vec<f64>> {
    let scores = model.scores.columns(0, retained.min(model.scores.ncols()));
    (0..scores.nrows())
        .map(|i| {
            (0..scores.nrows())
                .map(|j| {
                    let a = scores.row(i);
                    let b = scores.row(j);
                    let scale = a.norm() * b.norm();
                    if scale > 0.0 {
                        (a.dot(&b) / scale).clamp(-1.0, 1.0)
                    } else {
                        f64::NAN
                    }
                })
                .collect()
        })
        .collect()
}

pub(crate) fn build_mcr_diagnostic(result: &McrResult, view: McrView, theme: &Theme) -> Plot {
    match view {
        McrView::Spectra | McrView::References => build_mcr_plot(result, theme),
        McrView::Fractions => {
            let x: Vec<_> = (1..=result.labels.len()).map(|i| i as f64).collect();
            let mut p = Plot::new()
                .theme(theme.plot_theme())
                .xlabel("Sample number (input order)")
                .ylabel(if result.config.sum_to_one {
                    "Component fraction"
                } else {
                    "Component coefficient"
                });
            for j in 0..result.concentrations.ncols() {
                let y: Vec<_> = result.concentrations.column(j).iter().copied().collect();
                // Random mixtures have no temporal trajectory: points avoid suggesting one.
                p = p
                    .scatter(&x, &y)
                    .color(trace_color(theme, j))
                    .label(format!("Component {}", j + 1))
                    .into();
            }
            p.legend_position(LegendPosition::UpperRight)
        }
        McrView::Convergence => {
            let x: Vec<_> = (1..=result.objective_history.len())
                .map(|i| i as f64)
                .collect();
            let norm = result.data.norm_squared();
            let y: Vec<_> = result
                .objective_history
                .iter()
                .map(|v| (v / norm).max(1e-32))
                .collect();
            let p: Plot = Plot::new()
                .theme(theme.plot_theme())
                .line(&x, &y)
                .color(trace_color(theme, 0))
                .into();
            p.xlabel("Complete ALS iteration")
                .ylabel("Relative squared residual")
                .yscale(ruviz::axes::AxisScale::Log)
        }
        McrView::Residuals => {
            let x: Vec<_> = (1..=result.labels.len()).map(|i| i as f64).collect();
            let y: Vec<_> = (0..result.data.nrows())
                .map(|i| {
                    (result.residual.row(i).norm_squared() / result.data.ncols() as f64).sqrt()
                })
                .collect();
            let p: Plot = Plot::new()
                .theme(theme.plot_theme())
                .scatter(&x, &y)
                .color(trace_color(theme, 0))
                .into();
            p.xlabel("Sample number (input order)")
                .ylabel("Residual RMS (absorption units)")
        }
    }
}

/// Training residuals use a sum of the discarded eigenvalues, avoiding the
/// cancellation of 1 − cumulative_fraction. Target errors are reconstructed
/// directly; neither curve is a noise-weighted statistic or cross-validation.
pub(crate) fn build_pca_residual_plot(
    model: &PcaModel,
    target: Option<&rexafs::prelude::PcaFit>,
    all: bool,
    scale: PcaScale,
    theme: &Theme,
) -> Plot {
    let n = if all {
        model.n_components()
    } else {
        model.n_components().min(12)
    };
    let x: Vec<_> = (0..=n).map(|k| k as f64).collect();
    let training: Vec<_> = model
        .reconstruction_errors()
        .iter()
        .take(n + 1)
        .map(|point| scale.display_value(point.relative_error))
        .collect();
    let mut plot: Plot = Plot::new()
        .theme(theme.plot_theme())
        .line(&x, &training)
        .color(trace_color(theme, 0))
        .label("Training collection")
        .scatter(&x, &training)
        .color(trace_color(theme, 0))
        .into();
    if let Some(target) = target {
        let residual: Vec<_> = (0..=n)
            .map(|k| {
                model
                    .reconstruct(&target.data, k)
                    .map(|r| scale.display_value(r.r_factor))
                    .unwrap_or(f64::NAN)
            })
            .collect();
        plot = plot
            .line(&x, &residual)
            .color(trace_color(theme, 1))
            .line_style(LineStyle::Dashed)
            .label("Target reconstruction")
            .into();
    }
    plot.xlabel("Retained components")
        .ylabel("Relative squared reconstruction error")
        .yscale(scale.axis())
        .legend_position(LegendPosition::UpperRight)
}

/// Matched references and factors share colors; dashed lines are references.
/// No shifts, offsets, scale factors or chemical assignments alter the fit.
pub(crate) fn build_mcr_reference_plot(saved: &crate::project::McrAnalysis, theme: &Theme) -> Plot {
    let Some(reference) = &saved.comparison else {
        return build_mcr_plot(&saved.result, theme);
    };
    let mut plot = Plot::new().theme(theme.plot_theme());
    for (i, &j) in reference.component_indices.iter().enumerate() {
        let truth: Vec<_> = reference.spectra.row(i).iter().copied().collect();
        let estimated: Vec<_> = saved.result.spectra.row(j).iter().copied().collect();
        let name = reference
            .inputs
            .get(i)
            .map(|s| s.label.clone())
            .unwrap_or_else(|| format!("Reference {}", i + 1));
        plot = plot
            .line(saved.result.x.as_slice(), &estimated)
            .color(trace_color(theme, i))
            .label(format!("Component {}", j + 1))
            .line(&saved.result.x.as_slice().to_vec(), &truth)
            .color(trace_color(theme, i))
            .line_style(LineStyle::Dashed)
            .label(name)
            .into();
    }
    let (x, y) = analysis_axis_labels(saved.result.config.space);
    plot.xlabel(x)
        .ylabel(y)
        .legend_position(LegendPosition::UpperRight)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rexafs::prelude::{PcaConfig, pca_train};
    #[test]
    fn rank_and_similarity_distinguish_composition_variation_from_roundoff() {
        let spectra: Vec<_> = (0..20)
            .map(|i| {
                let t = i as f64 / 19.0;
                let mut s = XASSpectrum::new();
                s.energy = Some(nalgebra::DVector::from_vec(vec![
                    8970., 8980., 8990., 9000.,
                ]));
                s.normalization = Some(rexafs::prelude::NormalizationMethod::PrePostEdge(
                    rexafs::prelude::PrePostEdge {
                        e0: Some(8980.),
                        norm: Some(nalgebra::DVector::from_vec(vec![
                            0.1 + t,
                            1.0 - t,
                            0.3 + 2.0 * t,
                            1.0,
                        ])),
                        ..Default::default()
                    },
                ));
                s.e0 = Some(8980.);
                s
            })
            .collect();
        for (center, rank) in [(false, 2), (true, 1)] {
            let m = pca_train(
                &spectra,
                &PcaConfig {
                    center,
                    range: Some((-10., 20.)),
                    ..Default::default()
                },
            )
            .unwrap();
            assert_eq!(m.numerical_rank(), rank);
            assert_eq!(pca_count_suggestion(&m), Some((rank, "numerical rank")));
            let similarity = pca_similarity(&m, rank);
            assert!((similarity[0][0] - 1.0).abs() < 1e-12);
            if center {
                assert!((similarity[0][19] + 1.0).abs() < 1e-12);
            }
        }
        let equal = vec![spectra[0].clone(); 20];
        let m = pca_train(
            &equal,
            &PcaConfig {
                center: true,
                range: Some((-10., 20.)),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(pca_count_suggestion(&m), None);
    }
}
