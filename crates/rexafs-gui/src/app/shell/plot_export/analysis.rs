//! Export retained analysis arrays, with the same curves and display offsets as the plot.
use super::*;
use crate::plotting::analysis::{McrView, PcaView, analysis_axis_labels};

impl StudioApp {
    pub(super) fn analysis_export_curves(&self) -> Result<Vec<Curve>, String> {
        let mut curves = Vec::new();
        let mut add = |label: String, x_label: &str, y_label: &str, x: Vec<f64>, y: Vec<f64>| {
            curves.push(Curve {
                label,
                x_label: x_label.into(),
                y_label: y_label.into(),
                x,
                y,
            });
        };
        match self.analysis.shown {
            Some(super::super::tools::Tool::Lcf) => {
                let r = self.analysis.lcf.as_ref().ok_or("No LCF result")?;
                let (xl, yl) = analysis_axis_labels(r.space);
                let x = r.x.as_slice().to_vec();
                add(
                    "data".into(),
                    &xl,
                    &yl,
                    x.clone(),
                    r.data.as_slice().to_vec(),
                );
                add("fit".into(), &xl, &yl, x.clone(), r.fit.as_slice().to_vec());
                let span = r
                    .data
                    .iter()
                    .fold(0.0_f64, |m, v| m.max(v.abs()))
                    .max(1e-12);
                for (i, (component, weight)) in r.components.iter().zip(&r.weights).enumerate() {
                    add(
                        format!("{} × {:.2}", weight.name, weight.weight),
                        &xl,
                        &yl,
                        x.clone(),
                        component
                            .iter()
                            .map(|v| v - span * 0.25 * (i + 1) as f64)
                            .collect(),
                    );
                }
                add(
                    "residual (display offset)".into(),
                    &xl,
                    &yl,
                    x,
                    r.residual
                        .iter()
                        .map(|v| v - span * 0.25 * (r.components.len() + 1) as f64)
                        .collect(),
                );
            }
            Some(super::super::tools::Tool::Mcr) => {
                let saved = self.analysis.mcr.as_ref().ok_or("No MCR result")?;
                let r = &saved.result;
                let (xl, yl) = analysis_axis_labels(r.config.space);
                match self.analysis.mcr_view {
                    McrView::Spectra | McrView::References => {
                        if let Some(reference) = saved
                            .comparison
                            .as_ref()
                            .filter(|_| self.analysis.mcr_view == McrView::References)
                        {
                            for (i, &j) in reference.component_indices.iter().enumerate() {
                                add(
                                    format!("Component {}", j + 1),
                                    &xl,
                                    &yl,
                                    r.x.as_slice().to_vec(),
                                    r.spectra.row(j).iter().copied().collect(),
                                );
                                add(
                                    reference
                                        .inputs
                                        .get(i)
                                        .map(|s| s.label.clone())
                                        .unwrap_or_else(|| format!("Reference {}", i + 1)),
                                    &xl,
                                    &yl,
                                    r.x.as_slice().to_vec(),
                                    reference.spectra.row(i).iter().copied().collect(),
                                );
                            }
                        } else {
                            for j in 0..r.spectra.nrows() {
                                add(
                                    format!("Component {}", j + 1),
                                    &xl,
                                    &yl,
                                    r.x.as_slice().to_vec(),
                                    r.spectra.row(j).iter().copied().collect(),
                                );
                            }
                        }
                    }
                    McrView::Fractions => {
                        for j in 0..r.concentrations.ncols() {
                            add(
                                format!("Component {}", j + 1),
                                "Sample number (input order)",
                                if r.config.sum_to_one {
                                    "Component fraction"
                                } else {
                                    "Component coefficient"
                                },
                                (1..=r.labels.len()).map(|i| i as f64).collect(),
                                r.concentrations.column(j).iter().copied().collect(),
                            );
                        }
                    }
                    McrView::Convergence => add(
                        "ALS objective".into(),
                        "Complete ALS iteration",
                        "Relative squared residual",
                        (1..=r.objective_history.len()).map(|i| i as f64).collect(),
                        r.objective_history
                            .iter()
                            .map(|v| v / r.data.norm_squared())
                            .collect(),
                    ),
                    McrView::Residuals => add(
                        "Residual RMS".into(),
                        "Sample number (input order)",
                        "Residual RMS (absorption units)",
                        (1..=r.labels.len()).map(|i| i as f64).collect(),
                        (0..r.data.nrows())
                            .map(|i| {
                                (r.residual.row(i).norm_squared() / r.data.ncols() as f64).sqrt()
                            })
                            .collect(),
                    ),
                }
            }
            Some(super::super::tools::Tool::Pca) => {
                let m = self.analysis.pca.as_ref().ok_or("No PCA result")?;
                let n = if self.analysis.pca_all_components {
                    m.n_components()
                } else {
                    m.n_components().min(12)
                };
                let count: Vec<_> = (1..=n).map(|i| i as f64).collect();
                let (xl, yl) = analysis_axis_labels(m.space);
                let retained = self
                    .analysis
                    .pca_fit
                    .as_ref()
                    .map_or(self.tools.pca_components, |f| f.n_components);
                match self.analysis.pca_view {
                    PcaView::Scree => add(
                        "Explained fraction".into(),
                        "Principal component",
                        if m.centered {
                            "Explained variance fraction"
                        } else {
                            "Explained squared signal fraction"
                        },
                        count,
                        m.variance_explained.iter().take(n).copied().collect(),
                    ),
                    PcaView::Cumulative => add(
                        "Cumulative fraction".into(),
                        "Retained components",
                        "Cumulative explained fraction (%)",
                        count,
                        m.cumulative_variance
                            .iter()
                            .take(n)
                            .map(|v| 100. * v)
                            .collect(),
                    ),
                    PcaView::Indicator => {
                        let x: Vec<_> = (1..=n.min(m.ind.len().saturating_sub(1)))
                            .filter(|&i| m.ind[i].is_finite() && m.ind[i] >= 0.)
                            .map(|i| i as f64)
                            .collect();
                        let y = x.iter().map(|&i| m.ind[i as usize]).collect();
                        add(
                            "IND".into(),
                            "Retained components",
                            "Malinowski IND(k)",
                            x,
                            y,
                        );
                    }
                    PcaView::Scores12 | PcaView::Scores13 | PcaView::Scores23 => {
                        let (a, b) = match self.analysis.pca_view {
                            PcaView::Scores13 => (0, 2),
                            PcaView::Scores23 => (1, 2),
                            _ => (0, 1),
                        };
                        if b < m.scores.ncols() {
                            for i in 0..m.n_spectra() {
                                add(
                                    m.labels[i].clone(),
                                    &format!("PC{} score", a + 1),
                                    &format!("PC{} score", b + 1),
                                    vec![m.scores[(i, a)]],
                                    vec![m.scores[(i, b)]],
                                );
                            }
                        }
                    }
                    PcaView::Loadings => {
                        for i in 0..retained.min(m.n_components()) {
                            add(
                                format!("PC{}", i + 1),
                                &xl,
                                "Orthonormal loading (dimensionless)",
                                m.x.as_slice().to_vec(),
                                m.components.row(i).iter().copied().collect(),
                            );
                        }
                    }
                    PcaView::Residual => {
                        let x: Vec<_> = (0..=n).map(|i| i as f64).collect();
                        add(
                            "Training collection".into(),
                            "Retained components",
                            "Relative squared reconstruction error",
                            x.clone(),
                            m.reconstruction_errors()
                                .iter()
                                .take(n + 1)
                                .map(|v| v.relative_error)
                                .collect(),
                        );
                        if let Some(target) = &self.analysis.pca_fit {
                            add(
                                "Target reconstruction".into(),
                                "Retained components",
                                "Relative squared reconstruction error",
                                x,
                                (0..=n)
                                    .map(|k| {
                                        m.reconstruct(&target.data, k)
                                            .map(|r| r.r_factor)
                                            .unwrap_or(f64::NAN)
                                    })
                                    .collect(),
                            );
                        }
                    }
                    PcaView::Target => {
                        if let Some(r) = &self.analysis.pca_fit {
                            add(
                                "data".into(),
                                &xl,
                                &yl,
                                r.x.as_slice().to_vec(),
                                r.data.as_slice().to_vec(),
                            );
                            add(
                                format!("{} components", r.n_components),
                                &xl,
                                &yl,
                                r.x.as_slice().to_vec(),
                                r.fit.as_slice().to_vec(),
                            );
                            let span = r
                                .data
                                .iter()
                                .fold(0.0_f64, |m, v| m.max(v.abs()))
                                .max(1e-12);
                            add(
                                "residual (display offset)".into(),
                                &xl,
                                &yl,
                                r.x.as_slice().to_vec(),
                                r.residual.iter().map(|v| v - span * 0.3).collect(),
                            );
                        }
                    }
                    PcaView::Similarity => {
                        let matrix = crate::plotting::analysis::pca_similarity(m, retained);
                        for (i, row) in matrix.into_iter().enumerate() {
                            add(
                                m.labels[i].clone(),
                                "Sample number",
                                "Cosine similarity",
                                (1..=m.n_spectra()).map(|i| i as f64).collect(),
                                row,
                            );
                        }
                    }
                }
            }
            _ => return Err("No displayed analysis result".into()),
        }
        Ok(curves)
    }
}
