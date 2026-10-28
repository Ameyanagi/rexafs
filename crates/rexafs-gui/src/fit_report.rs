//! Markdown export of one fit (Larch `feffit_report` layout): statistics,
//! fit ranges, variables, correlations and one block per path with the
//! physical values, their uncertainties and the expressions behind them.
use crate::fit_details::Estimate;
use crate::fitting::{
    FitHistoryEntry, FitPathSpec, FitRanges, fit_result_notices, high_correlations,
};
use rexafs::prelude::FTWindow;
use rexafs::xafs::fitting::{FeffFitResult, FeffFitTransform};

/// Correlations weaker than this are left out, as in Larch.
pub(crate) const CORRELATION_MIN: f64 = 0.1;

/// Readable name of a Fourier-transform window family.
pub(crate) fn window_name(window: FTWindow) -> &'static str {
    match window {
        FTWindow::Hanning => "Hanning",
        FTWindow::Parzen => "Parzen",
        FTWindow::Welch => "Welch",
        FTWindow::Gaussian => "Gaussian",
        FTWindow::Sine => "sine",
        FTWindow::KaiserBessel => "Kaiser–Bessel",
        FTWindow::FHanning => "fractional Hanning",
    }
}

/// Display unit for a window parameter: dimensionless fractions for fractional
/// Hanning, otherwise the k or R axis unit. Kaiser–Bessel also reuses the first
/// parameter's numerical value for its dimensionless shape.
pub(crate) fn window_parameter_unit(window: FTWindow, axis: &'static str) -> &'static str {
    if window == FTWindow::FHanning {
        "fraction"
    } else {
        axis
    }
}

/// The transforms a fit used, one per dataset. Fits recorded before rexafs
/// stored them (`transforms` empty) used their ranges with the then-default
/// taper: Kaiser–Bessel k window, dk = 4 Å⁻¹; Hanning R window, dR = 0 Å.
/// A joint fit uses each dataset's recorded ranges, or the shared ranges when
/// that dataset had no override.
/// The flag is true for that reconstructed fallback.
pub(crate) fn fit_transforms(entry: &FitHistoryEntry) -> (Vec<FeffFitTransform>, bool) {
    if !entry.transforms.is_empty() {
        return (entry.transforms.clone(), false);
    }
    let legacy = |ranges: &FitRanges| FeffFitTransform {
        window: FTWindow::KaiserBessel,
        dk: 4.0,
        dk2: None,
        rwindow: FTWindow::Hanning,
        dr: 0.0,
        dr2: None,
        ..ranges.transform()
    };
    let transforms = match entry.joint.as_ref().filter(|j| !j.datasets.is_empty()) {
        Some(joint) => joint
            .datasets
            .iter()
            .map(|d| legacy(d.ranges.as_ref().unwrap_or(&entry.ranges)))
            .collect(),
        None => vec![legacy(&entry.ranges)],
    };
    (transforms, true)
}

/// Fit window description, e.g. "Kaiser–Bessel k window (dk = 4 Å⁻¹);
/// Hanning R window (dR = 0 Å)". Unequal second parameters are named `dk2`
/// and `dR2`, so the low- and high-end settings remain distinguishable.
/// Fractional Hanning parameters are dimensionless fractions; other families
/// use the axis units (and Kaiser–Bessel also reuses dk as its shape parameter).
/// A k-space fit applies no k window to its
/// residual (Larch semantics), but the transform still records one.
pub(crate) fn window_text(t: &FeffFitTransform) -> String {
    let taper = |name: &str, a: f64, b: Option<f64>, unit: &str| match b {
        Some(b) if b != a => format!("{name} = {a} {unit}, {name}2 = {b} {unit}"),
        _ => format!("{name} = {a} {unit}"),
    };
    format!(
        "{} k window ({}); {} R window ({})",
        window_name(t.window),
        taper("dk", t.dk, t.dk2, window_parameter_unit(t.window, "Å⁻¹")),
        window_name(t.rwindow),
        taper("dR", t.dr, t.dr2, window_parameter_unit(t.rwindow, "Å"))
    )
}

/// k-weights of one transform: the list, or the single weight when empty.
pub(crate) fn kweights_text(t: &FeffFitTransform) -> String {
    let weights = if t.kweights.is_empty() {
        vec![t.kweight]
    } else {
        t.kweights.clone()
    };
    weights
        .iter()
        .map(|k| format!("{k}"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Fit statistics and the settings that define them, as (quantity, value)
/// rows for reports: R-factor, χ², reduced χ² (only with a noise estimate),
/// N_idp / N_var, fit space, k and R ranges, k-weights, windows and the
/// correlations with |r| ≥ [`CORRELATION_MIN`]. Correlations need the full
/// `result`; without it the row says they are unavailable. Joint fits with
/// several transforms list ranges and windows for each dataset.
pub(crate) fn summary_rows(
    entry: &FitHistoryEntry,
    result: Option<&FeffFitResult>,
) -> Vec<(String, String)> {
    let joint = entry.joint.as_ref();
    let name = |n: &str| crate::joint_fitting::display_name(n, joint);
    let mut rows = vec![
        ("R-factor".to_string(), format!("{:.5}", entry.r_factor)),
        ("χ²".into(), format!("{:.4e}", entry.chi_square)),
        ("Reduced χ²".into(), entry.reduced_chi_square_text()),
        (
            "N_idp / N_var".into(),
            format!("{:.1} / {}", entry.n_idp, entry.n_vary),
        ),
        ("Fit space".into(), entry.ranges.fitspace.label().into()),
    ];
    let (transforms, reconstructed) = fit_transforms(entry);
    for (index, t) in transforms.iter().enumerate() {
        let prefix = match joint.and_then(|j| j.datasets.get(index)) {
            Some(d) if transforms.len() > 1 => format!("{}: ", d.label),
            _ if transforms.len() > 1 => format!("Dataset {}: ", index + 1),
            _ => String::new(),
        };
        rows.push((
            format!("{prefix}k range (Å⁻¹)"),
            format!("{:.2} – {:.2}", t.kmin, t.kmax),
        ));
        rows.push((
            format!("{prefix}R range (Å)"),
            format!("{:.2} – {:.2}", t.rmin, t.rmax),
        ));
        rows.push((format!("{prefix}k-weights"), kweights_text(t)));
        let mut window = window_text(t);
        if reconstructed {
            window.push_str(" (not recorded; release default at fit time)");
        }
        rows.push((format!("{prefix}Window"), window));
    }
    let correlations = match result {
        None => "Unavailable (full fit result not loaded)".to_string(),
        Some(r) => {
            let corr = high_correlations(r, CORRELATION_MIN);
            if corr.is_empty() {
                "None".into()
            } else {
                corr.iter()
                    .map(|(a, b, c)| format!("{} / {}: {c:+.3}", name(a), name(b)))
                    .collect::<Vec<_>>()
                    .join("; ")
            }
        }
    };
    rows.push((
        format!("Correlations (|r| ≥ {CORRELATION_MIN:.2})"),
        correlations,
    ));
    rows
}

fn est(e: &Option<Estimate>, digits: usize, unit: &str) -> String {
    match e {
        Some(e) => {
            let unit = if unit.is_empty() {
                String::new()
            } else {
                format!(" {unit}")
            };
            if e.fixed {
                format!("{:.*}{unit} (fixed)", digits, e.value)
            } else if let Some(err) = e.stderr {
                format!("{:.*} ± {:.*}{unit}", digits, e.value, digits, err)
            } else {
                format!("{:.*}{unit} (± unavailable)", digits, e.value)
            }
        }
        None => "unavailable".into(),
    }
}

fn opt(v: Option<f64>, digits: usize) -> String {
    v.map(|v| format!("{v:.*}", digits))
        .unwrap_or_else(|| "?".into())
}

fn cell(text: &str) -> String {
    text.replace('|', "\\|")
}

fn expr_cell(expr: &str) -> String {
    let expr = expr.trim();
    if expr.is_empty() {
        String::new()
    } else {
        format!("`{}`", cell(expr))
    }
}

/// Builds the report. `result` adds the noise estimate, correlations and
/// warnings that the archived history entry does not carry; the report is
/// still complete without it (older projects).
pub(crate) fn markdown(entry: &FitHistoryEntry, result: Option<&FeffFitResult>) -> String {
    let joint = entry.joint.as_ref();
    let name = |n: &str| crate::joint_fitting::display_name(n, joint);
    let mut out = String::new();
    out.push_str(&format!("# Fit {} · {}\n\n", entry.id, cell(&entry.group)));

    // ---- statistics ---------------------------------------------------
    out.push_str("## Statistics\n\n| quantity | value |\n|---|---:|\n");
    out.push_str(&format!("| R-factor | {:.5} |\n", entry.r_factor));
    out.push_str(&format!("| χ² | {:.4e} |\n", entry.chi_square));
    out.push_str(&format!(
        "| reduced χ² | {} |\n",
        entry.reduced_chi_square_text()
    ));
    out.push_str(&format!(
        "| N idp / N var | {:.1} / {} |\n",
        entry.n_idp, entry.n_vary
    ));
    if let Some(r) = result {
        out.push_str(&format!("| N data | {} |\n", r.n_data));
        if !entry.noise_estimated() {
            out.push_str("| ε k (noise) | not estimated (unit scale) |\n");
        } else if let Some(k) = r.kweight_results.first() {
            out.push_str(&format!("| ε k (noise) | {:.3e} |\n", k.epsilon_k));
            out.push_str(&format!("| ε R (noise) | {:.3e} |\n", k.epsilon_r));
        }
    }
    if let Some(report) = &entry.solver_report {
        out.push_str(&format!(
            "| optimizer | {:?} · {} |\n",
            report.method,
            if report.converged {
                "converged"
            } else {
                "stopped"
            }
        ));
        if let Some(n) = report.iterations {
            out.push_str(&format!("| iterations | {n} |\n"));
        }
        if let Some(n) = report.evaluations {
            out.push_str(&format!("| evaluations | {n} |\n"));
        }
        out.push_str(&format!(
            "| objective | {:.3e} → {:.3e} |\n",
            report.initial_cost, report.final_cost
        ));
        out.push_str(&format!(
            "| termination | {} |\n",
            cell(&report.termination)
        ));
    }

    // ---- ranges ---------------------------------------------------------
    let (transforms, reconstructed) = fit_transforms(entry);
    let multiple = transforms.len() > 1;
    let dataset_column = if multiple { " dataset |" } else { "" };
    let dataset_rule = if multiple { "---|" } else { "" };
    out.push_str(&format!(
        "\n## Fit ranges\n\n|{dataset_column} space | k-range (Å⁻¹) | k-weights | R-range (Å) | window | noise weighting |\n|{dataset_rule}---|---|---|---|---|---|\n"
    ));
    for (index, t) in transforms.iter().enumerate() {
        let dataset = if multiple {
            let label = joint
                .and_then(|j| j.datasets.get(index))
                .map(|d| d.label.clone())
                .unwrap_or_else(|| format!("Dataset {}", index + 1));
            format!(" {} |", cell(&label))
        } else {
            String::new()
        };
        let space = match t.fitspace {
            rexafs::xafs::fitting::FitSpace::R => "R",
            rexafs::xafs::fitting::FitSpace::K => "k",
            rexafs::xafs::fitting::FitSpace::Q => "q",
        };
        let mut window = window_text(t);
        if reconstructed {
            window.push_str(" (not recorded; release default at fit time)");
        }
        out.push_str(&format!(
            "|{dataset} {space} | {:.3} – {:.3} | {} | {:.3} – {:.3} | {} | {} |\n",
            t.kmin,
            t.kmax,
            kweights_text(t),
            t.rmin,
            t.rmax,
            window,
            if joint
                .and_then(|j| j.datasets.get(index))
                .and_then(|d| d.ranges.as_ref())
                .unwrap_or(&entry.ranges)
                .noise
            {
                "on"
            } else {
                "off"
            }
        ));
    }

    // ---- variables ------------------------------------------------------
    out.push_str("\n## Variables\n\n| name | kind | value | ± | initial | bounds | expression |\n|---|---|---:|---:|---:|---|---|\n");
    for v in &entry.vars {
        let fitted = entry.values.iter().find(|(n, _, _)| n == &v.name);
        let live = result.and_then(|r| r.variables.get(&v.name));
        let (kind, value, err) = if let Some(expr) = &v.expr {
            let _ = expr;
            ("expr", live.map(|x| x.value), None)
        } else if v.vary {
            (
                "vary",
                fitted.map(|(_, x, _)| *x).or(live.map(|x| x.value)),
                fitted
                    .and_then(|(_, _, e)| *e)
                    .or(live.and_then(|x| x.stderr)),
            )
        } else {
            ("fixed", Some(v.value), None)
        };
        let bounds = match (v.min, v.max) {
            (None, None) => String::new(),
            (lo, hi) => format!(
                "{} … {}",
                lo.map(|x| format!("{x}")).unwrap_or_else(|| "−∞".into()),
                hi.map(|x| format!("{x}")).unwrap_or_else(|| "∞".into())
            ),
        };
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} |\n",
            cell(&name(&v.name)),
            kind,
            value
                .map(|x| format!("{x:.5}"))
                .unwrap_or_else(|| "—".into()),
            err.map(|e| format!("{e:.5}")).unwrap_or_default(),
            v.value,
            bounds,
            v.expr.as_deref().map(expr_cell).unwrap_or_default()
        ));
    }

    // ---- correlations ---------------------------------------------------
    if let Some(r) = result {
        let corr = high_correlations(r, CORRELATION_MIN);
        if !corr.is_empty() {
            out.push_str(&format!(
                "\n## Correlations (|r| ≥ {CORRELATION_MIN:.2})\n\n| a | b | r |\n|---|---|---:|\n"
            ));
            for (a, b, c) in corr {
                out.push_str(&format!(
                    "| {} | {} | {c:+.3} |\n",
                    cell(&name(&a)),
                    cell(&name(&b))
                ));
            }
        }
    }

    // ---- paths ----------------------------------------------------------
    out.push_str("\n## Paths\n");
    let specs: Vec<&FitPathSpec> = entry.paths.iter().filter(|p| p.enabled).collect();
    if entry.path_details.is_empty() {
        out.push_str("\nPath values were not recorded for this fit (older history entry).\n");
    }
    for (i, p) in entry.path_details.iter().enumerate() {
        let spec = specs.get(i).copied();
        let e =
            |f: fn(&FitPathSpec) -> &String| spec.map(f).map(|s| expr_cell(s)).unwrap_or_default();
        let source = p
            .file
            .parent()
            .and_then(|d| d.file_name())
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let file = p
            .file
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| p.file.display().to_string());
        out.push_str(&format!(
            "\n### {} · {} · {}\n\n",
            i + 1,
            cell(&p.label),
            cell(&file)
        ));
        if !source.is_empty() {
            out.push_str(&format!("FEFF run `{}`", cell(&source)));
            if let Some(n) = p.nleg {
                out.push_str(&format!(" · {n} legs"));
            }
            out.push_str("\n\n");
        }
        out.push_str("| quantity | value | expression |\n|---|---:|---|\n");
        out.push_str(&format!("| R_eff | {} Å | |\n", opt(p.reff, 4)));
        out.push_str(&format!(
            "| FEFF degeneracy | {} | |\n",
            opt(p.degeneracy, 0)
        ));
        let n_expr = spec
            .map(|s| {
                if s.degen.trim().is_empty() {
                    "`degen`".to_string()
                } else {
                    expr_cell(&s.degen)
                }
            })
            .unwrap_or_default();
        out.push_str(&format!(
            "| N | {} | {} |\n",
            est(&p.effective_degen, 2, ""),
            n_expr
        ));
        out.push_str(&format!("| N·S₀² | {} | |\n", est(&p.n_s02, 3, "")));
        out.push_str(&format!(
            "| S₀² | {} | {} |\n",
            est(&p.s02, 3, ""),
            e(|s| &s.s02)
        ));
        out.push_str(&format!(
            "| ΔE₀ | {} | {} |\n",
            est(&p.e0, 3, "eV"),
            e(|s| &s.e0)
        ));
        let r_expr = spec
            .map(|s| expr_cell(&format!("reff + {}", s.deltar.trim())))
            .unwrap_or_default();
        out.push_str(&format!(
            "| R | {} | {} |\n",
            est(&p.distance, 4, "Å"),
            r_expr
        ));
        out.push_str(&format!(
            "| ΔR | {} | {} |\n",
            est(&p.deltar, 4, "Å"),
            e(|s| &s.deltar)
        ));
        out.push_str(&format!(
            "| σ² | {} | {} |\n",
            est(&p.sigma2, 5, "Å²"),
            e(|s| &s.sigma2)
        ));
        if p.ei.is_some() || spec.is_some_and(|s| !s.ei.trim().is_empty()) {
            out.push_str(&format!(
                "| Ei | {} | {} |\n",
                est(&p.ei, 3, "eV"),
                e(|s| &s.ei)
            ));
        }
        if p.third.is_some() || spec.is_some_and(|s| !s.third.trim().is_empty()) {
            out.push_str(&format!(
                "| C₃ | {} | {} |\n",
                est(&p.third, 6, "Å³"),
                e(|s| &s.third)
            ));
        }
        if p.fourth.is_some() || spec.is_some_and(|s| !s.fourth.trim().is_empty()) {
            out.push_str(&format!(
                "| C₄ | {} | {} |\n",
                est(&p.fourth, 6, "Å⁴"),
                e(|s| &s.fourth)
            ));
        }
    }

    // ---- notices --------------------------------------------------------
    if let Some(r) = result {
        let notices = fit_result_notices(r);
        if !notices.is_empty() || !r.warnings.is_empty() {
            out.push_str("\n## Notes\n\n");
            for n in notices {
                out.push_str(&format!("- {n}\n"));
            }
            for w in &r.warnings {
                out.push_str(&format!("- {}\n", cell(&w.message)));
            }
        }
    }
    out.push_str("\nUncertainties are one standard error propagated from the fit covariance; the FEFF reference geometry is treated as exact.\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fitting::{FitPathSpec, FitRanges, FitVarSpec};
    use rexafs::xafs::fitting::FitVariable;
    use std::path::PathBuf;

    fn entry() -> (FitHistoryEntry, FeffFitResult) {
        let file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../rexafs/tests/testfiles/feffcu01.dat");
        let mut spec = FitPathSpec::standard(file, 1);
        spec.degen = "n1".into();
        let vars = vec![
            FitVarSpec {
                name: "amp".into(),
                value: 0.9,
                vary: false,
                min: None,
                max: None,
                expr: None,
            },
            FitVarSpec {
                name: "n1".into(),
                value: 12.0,
                vary: true,
                min: Some(0.0),
                max: None,
                expr: None,
            },
            FitVarSpec {
                name: "de0".into(),
                value: 0.0,
                vary: true,
                min: None,
                max: None,
                expr: None,
            },
            FitVarSpec {
                name: "dr_1".into(),
                value: 0.0,
                vary: true,
                min: None,
                max: None,
                expr: None,
            },
            FitVarSpec {
                name: "sig2_1".into(),
                value: 0.003,
                vary: true,
                min: None,
                max: None,
                expr: None,
            },
        ];
        let mut result = FeffFitResult::default();
        for (n, v, e) in [
            ("amp", 0.9, None),
            ("n1", 11.4, Some(0.8)),
            ("de0", 4.2, Some(0.5)),
            ("dr_1", -0.006, Some(0.004)),
            ("sig2_1", 0.0087, Some(0.0004)),
        ] {
            result.variables.insert(
                n,
                FitVariable {
                    value: v,
                    vary: e.is_some(),
                    stderr: e,
                    ..Default::default()
                },
            );
        }
        result.varying_names = vec!["n1".into(), "de0".into(), "dr_1".into(), "sig2_1".into()];
        result.n_vary = 4;
        result.r_factor = 0.0123;
        result.n_idp = 11.2;
        let entry = FitHistoryEntry::from_result(
            3,
            "cu_150k.xmu".into(),
            vec![spec],
            vars,
            FitRanges::default(),
            &result,
        );
        (entry, result)
    }

    #[test]
    fn report_lists_statistics_variables_and_path_values() {
        let (entry, result) = entry();
        let md = markdown(&entry, Some(&result));
        assert!(md.starts_with("# Fit 3 · cu_150k.xmu\n"));
        assert!(md.contains("| R-factor | 0.01230 |"));
        assert!(md.contains("| n1 | vary | 11.40000 | 0.80000 | 12 | 0 … ∞ |  |"));
        assert!(md.contains("| amp | fixed | 0.90000 |  | 0.9 |  |  |"));
        assert!(md.contains("### 1 · feffcu01.dat · feffcu01.dat"));
        // FeffFitResult::default() carries no covariance, so the archived
        // estimates have values but no propagated uncertainty.
        assert!(
            md.contains("| N | 11.40") && md.contains("| `n1` |"),
            "{md}"
        );
        assert!(md.contains("| N·S₀² | 10.260"), "{md}");
        assert!(
            md.contains("| S₀² | 0.900") && md.contains("| `amp` |"),
            "{md}"
        );
        assert!(
            md.contains("| ΔR | -0.0060") && md.contains("| `dr_1` |"),
            "{md}"
        );
        assert!(md.contains("| R | ") && md.contains("`reff + dr_1`"));
        assert!(
            !md.contains("| Ei |"),
            "unused cumulants stay out of the report"
        );
    }

    #[test]
    fn reduced_chi_square_needs_a_noise_estimate() {
        let (mut entry, mut result) = entry();
        entry.reduced_chi_square = 4.5e-5;
        result.reduced_chi_square = 4.5e-5;
        let md = markdown(&entry, Some(&result));
        assert!(
            md.contains("| reduced χ² | n/a (noise not estimated) |"),
            "{md}"
        );
        assert!(
            md.contains("| ε k (noise) | not estimated (unit scale) |"),
            "{md}"
        );
        assert!(!md.contains("4.5000e-5"), "{md}");

        entry.ranges.noise = true;
        result.kweight_results = vec![Default::default()];
        result.kweight_results[0].epsilon_k = 2.5e-4;
        let md = markdown(&entry, Some(&result));
        assert!(md.contains("| reduced χ² | 4.5000e-5 |"), "{md}");
        assert!(md.contains("| ε k (noise) | 2.500e-4 |"), "{md}");
    }

    #[test]
    fn report_without_live_result_still_renders() {
        let (entry, _) = entry();
        let md = markdown(&entry, None);
        assert!(md.contains("## Paths"));
        assert!(!md.contains("## Correlations"));
        assert!(md.contains("| de0 | vary | 4.20000 | 0.50000 |"));
    }

    #[test]
    fn report_uses_archived_ranges_weights_and_asymmetric_windows() {
        let (mut entry, _) = entry();
        let transform = FeffFitTransform {
            kmin: 3.5,
            kmax: 13.5,
            kweights: vec![3.0, 1.0],
            rmin: 1.5,
            rmax: 3.5,
            window: FTWindow::Parzen,
            dk: 1.5,
            dk2: Some(2.5),
            rwindow: FTWindow::Welch,
            dr: 0.25,
            dr2: Some(0.75),
            ..Default::default()
        };
        entry.transforms = vec![transform.clone()];
        let expected = "Parzen k window (dk = 1.5 Å⁻¹, dk2 = 2.5 Å⁻¹); Welch R window (dR = 0.25 Å, dR2 = 0.75 Å)";
        assert_eq!(fit_transforms(&entry), (vec![transform], false));
        let rows = summary_rows(&entry, None);
        assert!(rows.contains(&("Window".into(), expected.into())));
        assert!(rows.contains(&("k-weights".into(), "3, 1".into())));
        let md = markdown(&entry, None);
        assert!(
            md.contains(&format!(
                "| R | 3.500 – 13.500 | 3, 1 | 1.500 – 3.500 | {expected} | off |"
            )),
            "{md}"
        );
        assert!(!md.contains("release default at fit time"), "{md}");
    }

    #[test]
    fn window_description_uses_fractional_and_gaussian_parameter_units() {
        let mut transform = FeffFitTransform {
            window: FTWindow::FHanning,
            dk: 0.2,
            dk2: Some(0.4),
            rwindow: FTWindow::Gaussian,
            dr: 0.3,
            dr2: Some(0.6),
            ..Default::default()
        };
        assert_eq!(
            window_text(&transform),
            "fractional Hanning k window (dk = 0.2 fraction, dk2 = 0.4 fraction); Gaussian R window (dR = 0.3 Å, dR2 = 0.6 Å)"
        );
        transform.window = FTWindow::Gaussian;
        transform.rwindow = FTWindow::FHanning;
        assert_eq!(
            window_text(&transform),
            "Gaussian k window (dk = 0.2 Å⁻¹, dk2 = 0.4 Å⁻¹); fractional Hanning R window (dR = 0.3 fraction, dR2 = 0.6 fraction)"
        );
    }

    #[test]
    fn joint_report_lists_each_archived_transform() {
        let (mut entry, _) = entry();
        entry.joint = Some(crate::joint_fitting::JointConfig {
            datasets: ["Cu | cold", "Cu warm"]
                .iter()
                .enumerate()
                .map(|(id, label)| crate::joint_fitting::JointDataset {
                    id,
                    label: (*label).into(),
                    ..Default::default()
                })
                .collect(),
            ..Default::default()
        });
        entry.transforms = vec![
            FeffFitTransform {
                window: FTWindow::Hanning,
                dk: 1.0,
                dk2: Some(2.0),
                ..Default::default()
            },
            FeffFitTransform {
                kmin: 4.0,
                kmax: 14.0,
                kweight: 3.0,
                window: FTWindow::Sine,
                dk: 3.0,
                ..Default::default()
            },
        ];
        let md = markdown(&entry, None);
        assert!(md.contains("| dataset | space |"), "{md}");
        assert!(
            md.contains("| Cu \\| cold | R | 0.000 – 20.000 | 2 | 1.000 – 3.000 | Hanning k window (dk = 1 Å⁻¹, dk2 = 2 Å⁻¹)"),
            "{md}"
        );
        assert!(
            md.contains(
                "| Cu warm | R | 4.000 – 14.000 | 3 | 1.000 – 3.000 | sine k window (dk = 3 Å⁻¹)"
            ),
            "{md}"
        );
        let rows = summary_rows(&entry, None);
        assert!(rows.contains(&("Cu warm: k-weights".into(), "3".into())));
    }

    #[test]
    fn unrecorded_history_keeps_the_legacy_window_fallback() {
        let (mut entry, _) = entry();
        entry.transforms.clear();
        entry.ranges.follow_window = false;
        // The fallback describes the historical calculation even if newer
        // range fields contain a different requested window.
        entry.ranges.window = FTWindow::Parzen;
        entry.ranges.dk = 1.5;
        entry.ranges.dk2 = Some(2.5);
        entry.ranges.rwindow = FTWindow::Welch;
        entry.ranges.dr = 0.25;
        entry.ranges.dr2 = Some(0.75);
        let (transforms, reconstructed) = fit_transforms(&entry);
        assert!(reconstructed);
        let transform = &transforms[0];
        assert_eq!(transform.window, FTWindow::KaiserBessel);
        assert_eq!((transform.dk, transform.dk2), (4.0, None));
        assert_eq!(transform.rwindow, FTWindow::Hanning);
        assert_eq!((transform.dr, transform.dr2), (0.0, None));
        assert_eq!(
            (transform.kmin, transform.kmax),
            (entry.ranges.kmin, entry.ranges.kmax)
        );
        let expected = "Kaiser–Bessel k window (dk = 4 Å⁻¹); Hanning R window (dR = 0 Å) (not recorded; release default at fit time)";
        let rows = summary_rows(&entry, None);
        assert!(rows.contains(&("Window".into(), expected.into())));
        let md = markdown(&entry, None);
        assert!(md.contains(expected), "{md}");
        assert!(!md.contains("Parzen k window"), "{md}");
    }

    #[test]
    fn old_joint_history_reconstructs_each_datasets_ranges_with_legacy_windows() {
        let (mut entry, _) = entry();
        entry.transforms.clear();
        let ranges = FitRanges {
            follow_transform: false,
            follow_window: false,
            kmin: 3.0,
            kmax: 11.0,
            rmin: 1.5,
            rmax: 2.5,
            kweight: 1.0,
            kweights: vec![1.0],
            window: FTWindow::Parzen,
            dk: 2.0,
            noise: true,
            ..Default::default()
        };
        entry.joint = Some(crate::joint_fitting::JointConfig {
            datasets: vec![
                crate::joint_fitting::JointDataset {
                    id: 0,
                    label: "Cold".into(),
                    ranges: Some(ranges),
                    ..Default::default()
                },
                crate::joint_fitting::JointDataset {
                    id: 1,
                    label: "Warm".into(),
                    // No override: this dataset used the shared fit range.
                    ..Default::default()
                },
            ],
            ..Default::default()
        });
        let (transforms, reconstructed) = fit_transforms(&entry);
        assert!(reconstructed);
        assert_eq!(transforms.len(), 2);
        assert_eq!((transforms[0].kmin, transforms[0].kmax), (3.0, 11.0));
        assert_eq!((transforms[0].rmin, transforms[0].rmax), (1.5, 2.5));
        assert_eq!(transforms[0].effective_kweights(), vec![1.0]);
        assert_eq!((transforms[1].kmin, transforms[1].kmax), (2.0, 12.0));
        for transform in &transforms {
            assert_eq!(transform.window, FTWindow::KaiserBessel);
            assert_eq!((transform.dk, transform.dk2), (4.0, None));
            assert_eq!(transform.rwindow, FTWindow::Hanning);
            assert_eq!((transform.dr, transform.dr2), (0.0, None));
        }
        let md = markdown(&entry, None);
        assert!(
            md.contains("| Cold | R | 3.000 – 11.000 | 1 | 1.500 – 2.500 | Kaiser–Bessel k window"),
            "{md}"
        );
        assert!(
            md.contains(
                "| Warm | R | 2.000 – 12.000 | 2, 1, 3 | 1.000 – 3.000 | Kaiser–Bessel k window"
            ),
            "{md}"
        );
        assert_eq!(
            md.matches("not recorded; release default at fit time")
                .count(),
            2
        );
        assert!(md.contains("release default at fit time) | on |"), "{md}");
        assert!(md.contains("release default at fit time) | off |"), "{md}");
        let rows = summary_rows(&entry, None);
        assert!(rows.contains(&("Cold: k range (Å⁻¹)".into(), "3.00 – 11.00".into())));
        assert!(rows.contains(&("Warm: k range (Å⁻¹)".into(), "2.00 – 12.00".into())));
    }
}
