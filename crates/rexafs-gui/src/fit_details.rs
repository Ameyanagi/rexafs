//! Archived physical path values and first-order covariance propagation.
use crate::fitting::FitPathSpec;
use rexafs::xafs::fitting::{
    FeffFitResult, FeffFlavor, FitVariables, FittingError, expression::eval_expression_with,
    feffpath,
};

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct Estimate {
    pub value: f64,
    pub stderr: Option<f64>,
    pub fixed: bool,
}
impl Estimate {
    pub fn label(&self, digits: usize) -> String {
        if self.fixed {
            format!("{:.*} (fixed)", digits, self.value)
        } else if let Some(e) = self.stderr {
            format!("{:.*} ± {:.*}", digits, self.value, digits, e)
        } else {
            format!("{:.*} ± unavailable", digits, self.value)
        }
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct PathFitDetails {
    pub file: std::path::PathBuf,
    pub label: String,
    pub reff: Option<f64>,
    pub nleg: Option<usize>,
    pub degeneracy: Option<f64>,
    /// Coordination number used by the fit (FEFF degeneracy unless overridden).
    #[serde(default)]
    pub effective_degen: Option<Estimate>,
    pub distance: Option<Estimate>,
    pub deltar: Option<Estimate>,
    pub sigma2: Option<Estimate>,
    pub s02: Option<Estimate>,
    pub e0: Option<Estimate>,
    /// N·S₀², the amplitude the fit actually resolves.
    #[serde(default)]
    pub n_s02: Option<Estimate>,
    #[serde(default)]
    pub ei: Option<Estimate>,
    #[serde(default)]
    pub third: Option<Estimate>,
    #[serde(default)]
    pub fourth: Option<Estimate>,
}

fn evaluate(expr: &str, reff: Option<f64>, degen: Option<f64>, vars: &FitVariables) -> Option<f64> {
    let globals = vars.resolve_values().ok()?;
    eval_expression_with(if expr.trim().is_empty() { "0" } else { expr }, |symbol| {
        (if symbol == "reff" {
            reff
        } else if symbol == "degen" {
            degen
        } else {
            globals.get(symbol).copied()
        })
        .ok_or_else(|| FittingError::UndefinedSymbol {
            symbol: symbol.into(),
        })
    })
    .ok()
}

/// J C Jᵀ, including shared-variable correlations and constrained expressions.
/// The FEFF reference geometry is treated as exact; this is fit uncertainty.
fn estimate(
    expr: &str,
    reff: Option<f64>,
    degen: Option<f64>,
    result: &FeffFitResult,
) -> Option<Estimate> {
    let value = evaluate(expr, reff, degen, &result.variables)?;
    let gradient: Option<Vec<f64>> = result
        .varying_names
        .iter()
        .map(|name| {
            let var = result.variables.get(name)?;
            let step = 1e-5 * var.value.abs().max(0.01);
            let lo = var
                .min
                .map_or(var.value - step, |b| (var.value - step).max(b));
            let hi = var
                .max
                .map_or(var.value + step, |b| (var.value + step).min(b));
            if hi <= lo {
                return None;
            }
            let mut vars = result.variables.clone();
            vars.get_mut(name)?.value = hi;
            let a = evaluate(expr, reff, degen, &vars)?;
            vars.get_mut(name)?.value = lo;
            let b = evaluate(expr, reff, degen, &vars)?;
            Some((a - b) / (hi - lo))
        })
        .collect();
    fn depends(
        expr: &str,
        result: &FeffFitResult,
        seen: &mut std::collections::BTreeSet<String>,
    ) -> bool {
        rexafs::xafs::fitting::expression::extract_symbols(expr)
            .iter()
            .any(|s| {
                result.varying_names.contains(s)
                    || (seen.insert(s.clone())
                        && result
                            .variables
                            .get(s)
                            .and_then(|v| v.expr.as_deref())
                            .is_some_and(|expr| depends(expr, result, seen)))
            })
    }
    let fixed = !depends(expr, result, &mut Default::default());
    let stderr = gradient.as_ref().and_then(|g| {
        if fixed {
            return Some(0.);
        }
        let c = result.covariance.as_ref()?;
        if c.len() != g.len()
            || c.iter()
                .any(|row| row.len() != g.len() || row.iter().any(|v| !v.is_finite()))
        {
            return None;
        }
        let variance: f64 = g
            .iter()
            .enumerate()
            .map(|(i, a)| {
                g.iter()
                    .enumerate()
                    .map(|(j, b)| a * c[i][j] * b)
                    .sum::<f64>()
            })
            .sum();
        (variance.is_finite() && variance >= 0.).then(|| variance.sqrt())
    });
    Some(Estimate {
        value,
        stderr,
        fixed,
    })
}

/// Cumulant / lifetime cells are optional: an empty cell is the core default
/// of zero and is left out of reports rather than shown as a fitted value.
fn optional(
    expr: &str,
    reff: Option<f64>,
    degen: Option<f64>,
    result: &FeffFitResult,
) -> Option<Estimate> {
    if expr.trim().is_empty() {
        None
    } else {
        estimate(expr, reff, degen, result)
    }
}

pub(crate) fn snapshot(paths: &[FitPathSpec], result: &FeffFitResult) -> Vec<PathFitDetails> {
    paths
        .iter()
        .filter(|p| p.enabled)
        .map(|p| {
            let feff = feffpath(&p.file.to_string_lossy(), FeffFlavor::Feff85L)
                .ok()
                .map(|p| p.feff);
            let reff = feff.as_ref().map(|f| f.reff);
            let degen = feff.as_ref().map(|f| f.degen);
            let deltar = estimate(&p.deltar, reff, degen, result);
            let distance = reff.zip(deltar.as_ref()).map(|(r, dr)| Estimate {
                value: r + dr.value,
                ..dr.clone()
            });
            PathFitDetails {
                file: p.file.clone(),
                label: p.label.clone(),
                reff,
                nleg: feff.as_ref().map(|f| f.nleg),
                degeneracy: degen,
                effective_degen: estimate(
                    if p.degen.trim().is_empty() {
                        "degen"
                    } else {
                        &p.degen
                    },
                    reff,
                    degen,
                    result,
                ),
                distance,
                deltar,
                sigma2: estimate(&p.sigma2, reff, degen, result),
                s02: estimate(
                    if p.s02.trim().is_empty() { "1" } else { &p.s02 },
                    reff,
                    degen,
                    result,
                ),
                e0: estimate(&p.e0, reff, degen, result),
                n_s02: estimate(
                    &format!(
                        "({}) * ({})",
                        if p.degen.trim().is_empty() {
                            "degen"
                        } else {
                            p.degen.trim()
                        },
                        if p.s02.trim().is_empty() {
                            "1"
                        } else {
                            p.s02.trim()
                        }
                    ),
                    reff,
                    degen,
                    result,
                ),
                ei: optional(&p.ei, reff, degen, result),
                third: optional(&p.third, reff, degen, result),
                fourth: optional(&p.fourth, reff, degen, result),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rexafs::xafs::fitting::FitVariable;
    #[test]
    fn effective_coordination_number_uses_the_feff_degeneracy_local() {
        let r = FeffFitResult::default();
        let n = estimate("degen", None, Some(6.0), &r).unwrap();
        assert_eq!(n.value, 6.0);
        assert!(n.fixed);
        let half = estimate("degen / 2", None, Some(6.0), &r).unwrap();
        assert_eq!(half.value, 3.0);
        assert!(estimate("degen", None, None, &r).is_none());
    }

    #[test]
    fn propagates_shared_constraints_and_covariance() {
        let mut r = FeffFitResult::default();
        r.variables.insert(
            "a",
            FitVariable {
                value: 0.01,
                vary: true,
                ..Default::default()
            },
        );
        r.variables.insert(
            "b",
            FitVariable {
                value: 0.02,
                vary: true,
                ..Default::default()
            },
        );
        r.variables.insert(
            "dr",
            FitVariable {
                expr: Some("2*a+b".into()),
                ..Default::default()
            },
        );
        r.varying_names = vec!["a".into(), "b".into()];
        r.covariance = Some(vec![vec![0.0001, 0.00005], vec![0.00005, 0.0004]]);
        let e = estimate("reff+dr", Some(2.5), None, &r).unwrap();
        assert!((e.value - 2.54).abs() < 1e-10);
        assert!((e.stderr.unwrap() - 0.001_f64.sqrt()).abs() < 1e-9);
        assert!(!e.fixed);
        r.covariance = None;
        assert!(estimate("dr", None, None, &r).unwrap().stderr.is_none());
        assert!(estimate("2.5", None, None, &r).unwrap().fixed);
        assert!(estimate("unknown", None, None, &r).is_none());
        r.variables.get_mut("a").unwrap().value = 0.;
        assert!(!estimate("a*a", None, None, &r).unwrap().fixed);
    }
    #[test]
    fn respects_bounds_when_differentiating() {
        let mut r = FeffFitResult::default();
        r.variables.insert(
            "a",
            FitVariable {
                value: 0.,
                vary: true,
                min: Some(0.),
                ..Default::default()
            },
        );
        r.varying_names = vec!["a".into()];
        r.covariance = Some(vec![vec![0.0004]]);
        assert!((estimate("a", None, None, &r).unwrap().stderr.unwrap() - 0.02).abs() < 1e-10);
    }
}
