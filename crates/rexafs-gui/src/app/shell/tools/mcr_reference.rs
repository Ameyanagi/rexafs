//! Compare retained MCR factors with independently selected standards.
//! Matching changes labels in the comparison only; it never changes the fit.

use super::*;
use nalgebra::{DMatrix, DVector};
use rexafs::prelude::McrResult;

/// Minimum-cost one-to-one matching by the Hungarian assignment algorithm.
/// Rows are references; columns are recovered components. Costs are squared
/// spectral errors divided by each reference's squared norm, without scaling.
fn match_components(cost: &DMatrix<f64>) -> Vec<usize> {
    let n = cost.nrows();
    let (mut u, mut v) = (vec![0.0; n + 1], vec![0.0; n + 1]);
    let (mut p, mut way) = (vec![0; n + 1], vec![0; n + 1]);
    for i in 1..=n {
        p[0] = i;
        let mut j0 = 0;
        let mut min = vec![f64::INFINITY; n + 1];
        let mut used = vec![false; n + 1];
        loop {
            used[j0] = true;
            let i0 = p[j0];
            let (mut delta, mut j1) = (f64::INFINITY, 0);
            for j in 1..=n {
                if !used[j] {
                    let value = cost[(i0 - 1, j - 1)] - u[i0] - v[j];
                    if value < min[j] {
                        min[j] = value;
                        way[j] = j0;
                    }
                    if min[j] < delta {
                        delta = min[j];
                        j1 = j;
                    }
                }
            }
            for j in 0..=n {
                if used[j] {
                    u[p[j]] += delta;
                    v[j] -= delta;
                } else {
                    min[j] -= delta;
                }
            }
            j0 = j1;
            if p[j0] == 0 {
                break;
            }
        }
        loop {
            let j1 = way[j0];
            p[j0] = p[j1];
            j0 = j1;
            if j0 == 0 {
                break;
            }
        }
    }
    let mut assignment = vec![0; n];
    for j in 1..=n {
        assignment[p[j] - 1] = j - 1;
    }
    assignment
}

fn compare_references(
    result: &McrResult,
    standards: &[Arc<XASSpectrum>],
) -> Result<crate::project::McrReferenceComparison, String> {
    let n = result.spectra.nrows();
    if standards.len() != n {
        return Err(format!(
            "Mark exactly {n} reference standards to compare with the retained components"
        ));
    }
    let mut spectra = DMatrix::zeros(n, result.x.len());
    for (i, standard) in standards.iter().enumerate() {
        let x = standard
            .energy
            .as_ref()
            .ok_or("Reference has no energy axis")?;
        let y = match result.config.space {
            AnalysisSpace::Flat => standard.flat(),
            _ => standard.norm(),
        }
        .ok_or("Reference needs the same normalization or flattening as the MCR inputs")?;
        if x.len() != y.len()
            || x.len() < 2
            || x.iter().chain(y.iter()).any(|v| !v.is_finite())
            || x.as_slice().windows(2).any(|w| w[0] >= w[1])
            || x[0] > result.x[0]
            || x[x.len() - 1] < result.x[result.x.len() - 1]
        {
            return Err(
                "Reference needs finite increasing energy and full measured coverage".into(),
            );
        }
        let values = DVector::from_iterator(
            result.x.len(),
            result.x.iter().map(|&energy| {
                let j = x
                    .as_slice()
                    .partition_point(|&v| v < energy)
                    .clamp(1, x.len() - 1);
                y[j - 1] + (y[j] - y[j - 1]) * (energy - x[j - 1]) / (x[j] - x[j - 1])
            }),
        );
        if values.norm_squared() <= 0.0 {
            return Err("A reference has zero spectral norm".into());
        }
        spectra.set_row(i, &values.transpose());
    }
    let cost = DMatrix::from_fn(n, n, |i, j| {
        (spectra.row(i) - result.spectra.row(j)).norm_squared() / spectra.row(i).norm_squared()
    });
    if cost.iter().any(|v| !v.is_finite()) {
        return Err("Reference comparison produced a nonfinite error".into());
    }
    let component_indices = match_components(&cost);
    let relative_errors = component_indices
        .iter()
        .enumerate()
        .map(|(i, &j)| cost[(i, j)].sqrt())
        .collect();
    Ok(crate::project::McrReferenceComparison {
        spectra,
        component_indices,
        relative_errors,
        inputs: Vec::new(),
    })
}

impl StudioApp {
    pub(crate) fn compare_mcr_references(&mut self, cx: &mut Context<Self>) -> Result<(), String> {
        let saved = self
            .analysis
            .mcr
            .as_ref()
            .ok_or("Run MCR before comparing references")?;
        let mut inputs = Vec::new();
        let mut standards = Vec::new();
        for ix in marked_group_indices(&self.selection) {
            let source = self.tool_target(ix).ok_or("Reference is unavailable")?;
            let spectrum = self.cache.peek(&(ix, source.fingerprint)).ok_or(
                "References are still loading; try again after their comparison plot is ready",
            )?;
            standards.push(spectrum.clone());
            inputs.push(crate::project::AnalysisInput {
                corrections: self.correction_sources(source.ix),
                group_id: source.group_id,
                label: source.label,
                fingerprint: source.fingerprint,
            });
        }
        let mut comparison = compare_references(&saved.result, &standards)?;
        comparison.inputs = inputs;
        self.analysis.mcr.as_mut().unwrap().comparison = Some(comparison);
        self.analysis.mcr_view = crate::plotting::analysis::McrView::References;
        self.analysis.shown = Some(Tool::Mcr);
        self.tools.message =
            "References matched by spectral error; the blind MCR fit is unchanged.".into();
        self.record(
            "Compared MCR components with marked reference standards",
            None,
        );
        self.rebuild_analysis_plot(cx);
        cx.notify();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn assignment_is_global_and_handles_permuted_components() {
        let cost = DMatrix::from_row_slice(3, 3, &[5., 1., 8., 2., 4., 7., 5., 6., 0.]);
        assert_eq!(match_components(&cost), vec![1, 0, 2]);
        let greedy_trap = DMatrix::from_row_slice(2, 2, &[1., 2., 1., 100.]);
        assert_eq!(match_components(&greedy_trap), vec![1, 0]);
    }
}
