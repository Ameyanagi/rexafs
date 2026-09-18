//! Region-balanced MBACK with scaled SVD and bounded erfc variable projection.
use super::*;
use crate::atomic::AtomicTable;
use crate::xafs::normalization::PrePostEdge;
use errorfunctions::RealErrorFunctions;
use nalgebra::DMatrix;
use std::f64::consts::PI;

const RANK_TOL: f64 = 1e-10;
type Result<T> = std::result::Result<T, NormalizationError>;

struct LinearFit {
    coefficients: DVector<f64>,
    objective: f64,
    rank: usize,
    condition: f64,
}
fn scaled_solve(a: &DMatrix<f64>, b: &DVector<f64>) -> Result<LinearFit> {
    let (scaled, scales) = scale_columns(a)?;
    let svd = scaled.svd(true, true);
    let largest = svd.singular_values.max();
    let rank = svd
        .singular_values
        .iter()
        .filter(|&&s| s > largest * RANK_TOL)
        .count();
    if rank < a.ncols() {
        return Err(invalid(format!(
            "fit is not identifiable: rank {rank}/{}",
            a.ncols()
        )));
    }
    let condition = largest / svd.singular_values.min();
    let coefficients = svd
        .solve(b, largest * RANK_TOL)
        .map_err(invalid)?
        .component_div(&scales);
    if coefficients.iter().any(|v| !v.is_finite()) {
        return Err(invalid("linear fit overflow"));
    }
    let objective = (a * &coefficients - b).norm_squared();
    if !objective.is_finite() {
        return Err(invalid("objective overflow"));
    }
    Ok(LinearFit {
        coefficients,
        objective,
        rank,
        condition,
    })
}
fn scale_columns(a: &DMatrix<f64>) -> Result<(DMatrix<f64>, DVector<f64>)> {
    let scales = DVector::from_iterator(a.ncols(), (0..a.ncols()).map(|j| a.column(j).norm()));
    if scales
        .iter()
        .any(|s| !s.is_finite() || *s <= f64::MIN_POSITIVE)
    {
        return Err(invalid(
            "zero or nonfinite model column; reduce the model or revise the ranges",
        ));
    }
    let mut scaled = a.clone();
    for (j, &s) in scales.iter().enumerate() {
        scaled.column_mut(j).scale_mut(1. / s);
    }
    Ok((scaled, scales))
}
fn checked_region(name: &str, range: [f64; 2], energy: &[f64], e0: f64) -> Result<Vec<usize>> {
    if range.iter().any(|v| !v.is_finite()) || range[0] >= range[1] {
        return Err(invalid(format!(
            "{name} must have increasing finite offsets"
        )));
    }
    let (lo, hi) = (range[0] + e0, range[1] + e0);
    if lo < energy[0] || hi > *energy.last().unwrap() {
        return Err(invalid(format!(
            "{name} [{lo}, {hi}] eV is outside the scan; explicit intervals are not shortened"
        )));
    }
    let indices = energy
        .iter()
        .enumerate()
        .filter_map(|(i, &e)| (e >= lo && e <= hi).then_some(i))
        .collect::<Vec<_>>();
    // Auxiliary polynomial normalization excludes the upper endpoint, and needs
    // a line in pre-edge and a quadratic in post-edge. Do not let it expand a range.
    let minimum = if name == "pre-edge" { 3 } else { 4 };
    if indices.len() < minimum {
        return Err(invalid(format!(
            "{name} needs at least {minimum} measured points"
        )));
    }
    Ok(indices)
}

struct Problem<'a> {
    energy: &'a [f64],
    mu: &'a [f64],
    f2: &'a [f64],
    indices: &'a [usize],
    weights: &'a [f64],
    t: &'a [f64],
    degree: usize,
    emission: Option<f64>,
    erfc: Option<&'a MbackErfc>,
}
impl Problem<'_> {
    fn system(&self, width: Option<f64>) -> (DMatrix<f64>, DVector<f64>) {
        let columns = self.degree + 2 + usize::from(width.is_some());
        let mut a = DMatrix::zeros(self.indices.len(), columns);
        let mut b = DVector::zeros(self.indices.len());
        for (row, (&i, &w)) in self.indices.iter().zip(self.weights).enumerate() {
            a[(row, 0)] = self.mu[i] * w;
            for j in 0..=self.degree {
                a[(row, j + 1)] = -self.t[i].powi(j as i32) * w;
            }
            if let Some(width) = width {
                a[(row, columns - 1)] =
                    -RealErrorFunctions::erfc((self.energy[i] - self.emission.unwrap()) / width)
                        * w;
            }
            b[row] = self.f2[i] * w;
        }
        (a, b)
    }
    fn solve(&self, width: Option<f64>) -> Result<LinearFit> {
        let (a, b) = self.system(width);
        let mut fit = scaled_solve(&a, &b)?;
        if let Some(term) = self.erfc {
            let last = a.ncols() - 1;
            let amplitude = fit.coefficients[last].clamp(term.amplitude[0], term.amplitude[1]);
            if amplitude != fit.coefficients[last] {
                let reduced = a.columns(0, last).into_owned();
                let rhs = &b - a.column(last) * amplitude;
                let projected = scaled_solve(&reduced, &rhs)?;
                fit.coefficients
                    .rows_mut(0, last)
                    .copy_from(&projected.coefficients);
                fit.coefficients[last] = amplitude;
                fit.objective = (&a * &fit.coefficients - &b).norm_squared();
            }
        }
        if fit.coefficients[0] <= 0. {
            return Err(invalid(
                "matching requires a nonpositive scale; check signal direction and fit intervals",
            ));
        }
        Ok(fit)
    }
    fn profile(&self) -> Result<(LinearFit, Option<f64>, usize, Vec<String>)> {
        let Some(term) = self.erfc else {
            return Ok((self.solve(None)?, None, 1, Vec::new()));
        };
        let lo = term.width_ev[0].ln();
        let hi = term.width_ev[1].ln();
        let mut evaluations = 0;
        let mut evaluate = |x: f64| {
            evaluations += 1;
            self.solve(Some(x.exp())).ok()
        };
        // A deterministic logarithmic grid locates distinct candidate basins.
        // Refine each local minimum with bounded golden-section profiling; the
        // inner solve refits every linear coefficient, including active A bounds.
        let grid = (0..=64)
            .map(|i| lo + (hi - lo) * i as f64 / 64.)
            .collect::<Vec<_>>();
        let samples = grid.iter().map(|&x| evaluate(x)).collect::<Vec<_>>();
        let score = |f: &Option<LinearFit>| f.as_ref().map_or(f64::INFINITY, |f| f.objective);
        let mut best = samples
            .iter()
            .enumerate()
            .filter_map(|(i, f)| f.as_ref().map(|f| (grid[i], f.objective)))
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .ok_or_else(|| {
                invalid("no identifiable positive-scale erfc fit inside width/amplitude bounds")
            })?;
        for i in 0..grid.len() {
            if samples[i].is_none()
                || (i > 0 && score(&samples[i]) > score(&samples[i - 1]))
                || (i + 1 < grid.len() && score(&samples[i]) > score(&samples[i + 1]))
            {
                continue;
            }
            let mut a = grid[i.saturating_sub(1)];
            let mut b = grid[(i + 1).min(64)];
            let ratio = (5f64.sqrt() - 1.) / 2.;
            let mut c = b - ratio * (b - a);
            let mut d = a + ratio * (b - a);
            let mut fc = score(&evaluate(c));
            let mut fd = score(&evaluate(d));
            for _ in 0..120 {
                if b - a < 1e-10 {
                    break;
                }
                if fc < fd {
                    b = d;
                    d = c;
                    fd = fc;
                    c = b - ratio * (b - a);
                    fc = score(&evaluate(c));
                } else {
                    a = c;
                    c = d;
                    fc = fd;
                    d = a + ratio * (b - a);
                    fd = score(&evaluate(d));
                }
            }
            for (x, value) in [(c, fc), (d, fd)] {
                if value < best.1 {
                    best = (x, value);
                }
            }
        }
        let width = best.0.exp();
        let fit = evaluate(best.0).ok_or_else(|| invalid("erfc final evaluation failed"))?;
        let mut warnings = Vec::new();
        if (best.0 - lo).abs() < 1e-8 || (best.0 - hi).abs() < 1e-8 {
            warnings.push(
                "The erfc width is at a requested bound; inspect background sensitivity.".into(),
            );
        }
        let amplitude = fit.coefficients[fit.coefficients.len() - 1];
        if amplitude == term.amplitude[0] || amplitude == term.amplitude[1] {
            warnings.push(
                "The erfc amplitude is at a requested bound; inspect background sensitivity."
                    .into(),
            );
        }
        Ok((fit, Some(width), evaluations, warnings))
    }
}

pub(super) fn fit(
    model: &MBack,
    energy: &[f64],
    mu: &[f64],
    data: &dyn AtomicDataProvider,
) -> Result<MbackResult> {
    if energy.len() != mu.len() || energy.len() < 8 {
        return Err(invalid(
            "provide matching energy/mu arrays with at least eight points",
        ));
    }
    if energy.iter().any(|v| !v.is_finite() || *v <= 0.)
        || mu.iter().any(|v| !v.is_finite())
        || energy.windows(2).any(|w| w[0] >= w[1])
    {
        return Err(invalid(
            "energy must be finite, positive and strictly increasing; mu must be finite",
        ));
    }
    let options = &model.options;
    if options.element.is_empty() || options.edge.is_empty() {
        return Err(invalid(
            "select absorber and edge with MBack::for_edge(element, edge)",
        ));
    }
    if options.degree > 5 {
        return Err(invalid("background degree must be 0 through 5"));
    }
    let edge = data
        .edge(&options.element, &options.edge)
        .map_err(invalid)?;
    let energy_vec = DVector::from_column_slice(energy);
    let e0 = match model.e0 {
        Some(e) => e,
        None => super::super::xafsutils::find_e0(&energy_vec, &DVector::from_column_slice(mu))
            .map_err(invalid)?,
    };
    if !e0.is_finite() || e0 <= energy[0] || e0 >= *energy.last().unwrap() {
        return Err(invalid(
            "E0 must be strictly inside the measured energy interval",
        ));
    }
    let edges = data.edges(&edge.element).map_err(invalid)?;
    let mut pre_lo = energy[0] - e0;
    let mut post_hi = energy[energy.len() - 1] - e0;
    for other in &edges {
        if other.edge == edge.edge {
            continue;
        }
        let offset = other.energy_ev - e0;
        if offset < 0. {
            pre_lo = pre_lo.max(offset + 10.);
        } else {
            post_hi = post_hi.min(offset - 10.);
        }
    }
    let pre = options.pre_edge.unwrap_or([pre_lo, pre_lo * 0.2]);
    let post = options.post_edge.unwrap_or([post_hi * 0.2, post_hi]);
    if pre[1] >= 0. || post[0] <= 0. || pre[1] >= post[0] {
        return Err(invalid(
            "pre-edge must lie below E0 and post-edge above E0, without overlap",
        ));
    }
    if post[1] - post[0] < 10. {
        return Err(invalid(
            "post-edge interval must span at least 10 eV for auxiliary normalization",
        ));
    }
    let pre_indices = checked_region("pre-edge", pre, energy, e0)?;
    let post_indices = checked_region("post-edge", post, energy, e0)?;
    for other in &edges {
        if other.edge != edge.edge
            && [pre, post]
                .iter()
                .any(|r| other.energy_ev >= r[0] + e0 && other.energy_ev <= r[1] + e0)
        {
            return Err(invalid(format!(
                "neighboring {} {} edge at {} eV lies in a fit region; revise the interval",
                other.element, other.edge, other.energy_ev
            )));
        }
    }
    if edge.energy_ev >= pre[0] + e0 && edge.energy_ev <= pre[1] + e0
        || edge.energy_ev >= post[0] + e0 && edge.energy_ev <= post[1] + e0
    {
        return Err(invalid("the selected tabulated edge lies in a fit region; check E0/calibration and the excluded interval"));
    }
    let mut indices = pre_indices.clone();
    indices.extend(&post_indices);
    let weights = pre_indices
        .iter()
        .map(|_| 1. / (pre_indices.len() as f64).sqrt())
        .chain(
            post_indices
                .iter()
                .map(|_| 1. / (post_indices.len() as f64).sqrt()),
        )
        .collect::<Vec<_>>();
    let parameters = options.degree + 2 + if options.erfc.is_some() { 2 } else { 0 };
    if indices.len() <= parameters {
        return Err(invalid(
            "not enough included points for the selected background",
        ));
    }
    let reference = data.f2(&edge.element, energy).map_err(invalid)?;
    if reference.reference.table != AtomicTable::ChantlerF2LogLogV1
        || reference.reference.data != *data.identity()
    {
        return Err(invalid("provider returned an incompatible f2 reference"));
    }
    if options
        .reference
        .as_ref()
        .is_some_and(|r| r != &reference.reference)
    {
        return Err(invalid(
            "the archived atomic reference is unavailable; exact recomputation is blocked",
        ));
    }
    if reference.energy_ev != energy
        || reference.values.len() != energy.len()
        || reference.values.iter().any(|v| !v.is_finite() || *v <= 0.)
    {
        return Err(invalid("provider returned invalid reference samples"));
    }
    let emission = if let Some(term) = &options.erfc {
        if term.width_ev.iter().any(|v| !v.is_finite() || *v <= 0.)
            || term.width_ev[0] >= term.width_ev[1]
            || term.amplitude.iter().any(|v| !v.is_finite())
            || term.amplitude[0] >= term.amplitude[1]
        {
            return Err(invalid("erfc needs increasing positive finite width bounds and increasing finite amplitude bounds"));
        }
        let line = data
            .emission(&edge.element, &term.emission)
            .map_err(invalid)?;
        if line.lines.is_empty()
            || line.lines.iter().any(|l| l.initial_level != edge.edge)
            || line.energy_ev >= edge.energy_ev
        {
            return Err(invalid(
                "erfc emission must originate at the selected absorber edge and lie below it",
            ));
        }
        Some(line)
    } else {
        None
    };
    let energy_scale = pre[0].abs().max(post[1].abs());
    let t = energy
        .iter()
        .map(|e| (e - e0) / energy_scale)
        .collect::<Vec<_>>();
    let problem = Problem {
        energy,
        mu,
        f2: &reference.values,
        indices: &indices,
        weights: &weights,
        t: &t,
        degree: options.degree,
        emission: emission.as_ref().map(|l| l.energy_ev),
        erfc: options.erfc.as_ref(),
    };
    let (fit, width, evaluations, mut warnings) = problem.profile()?;
    let scale = fit.coefficients[0];
    let coefficients = fit
        .coefficients
        .rows(1, options.degree + 1)
        .iter()
        .copied()
        .collect::<Vec<_>>();
    let amplitude = if width.is_some() {
        fit.coefficients[fit.coefficients.len() - 1]
    } else {
        0.
    };
    let (rank, condition) = if let Some(width) = width {
        let (linear, _) = problem.system(Some(width));
        let mut jac = DMatrix::zeros(linear.nrows(), linear.ncols() + 1);
        jac.columns_mut(0, linear.ncols()).copy_from(&linear);
        for (row, (&i, &w)) in indices.iter().zip(&weights).enumerate() {
            let z = (energy[i] - emission.as_ref().unwrap().energy_ev) / width;
            jac[(row, linear.ncols())] =
                -amplitude * 2. * z * (-z * z).exp() / (PI.sqrt() * width) * w;
        }
        let (scaled, _) = scale_columns(&jac)?;
        let singular = scaled.svd(false, false).singular_values;
        let rank = singular
            .iter()
            .filter(|&&s| s > singular.max() * RANK_TOL)
            .count();
        if rank < parameters {
            return Err(invalid(format!(
                "erfc width/background are not identifiable: rank {rank}/{parameters}"
            )));
        }
        (rank, singular.max() / singular.min())
    } else {
        (fit.rank, fit.condition)
    };
    if condition > 1e6 {
        warnings.push(format!("Column-scaled condition number is {condition:.3e}; inspect model and range sensitivity."));
    }
    let background = t
        .iter()
        .zip(energy)
        .map(|(&t, &e)| {
            let poly = coefficients
                .iter()
                .rev()
                .fold(0., |value, &c| value * t + c);
            poly + width.map_or(0., |xi| {
                amplitude
                    * RealErrorFunctions::erfc((e - emission.as_ref().unwrap().energy_ev) / xi)
            })
        })
        .collect::<Vec<_>>();
    let matched = reference
        .values
        .iter()
        .zip(&background)
        .map(|(f, b)| f + b)
        .collect::<Vec<_>>();
    let mut auxiliary = PrePostEdge {
        e0: Some(e0),
        pre_edge_start: Some(pre[0]),
        pre_edge_end: Some(pre[1]),
        norm_start: Some(post[0]),
        norm_end: Some(post[1]),
        norm_polyorder: Some(2),
        n_victoreen: Some(0),
        ..PrePostEdge::new()
    };
    auxiliary.normalize(&energy_vec, &DVector::from_vec(matched))?;
    let pre_curve = auxiliary
        .pre_edge
        .as_ref()
        .unwrap()
        .iter()
        .copied()
        .collect::<Vec<_>>();
    let post_curve = auxiliary
        .post_edge
        .as_ref()
        .unwrap()
        .iter()
        .copied()
        .collect::<Vec<_>>();
    let ie0 = energy
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| (*a - e0).abs().total_cmp(&(*b - e0).abs()))
        .unwrap()
        .0;
    let delta = post_curve[ie0] - pre_curve[ie0];
    if !delta.is_finite()
        || delta <= 32. * f64::EPSILON * post_curve[ie0].abs().max(pre_curve[ie0].abs()).max(1.)
    {
        return Err(invalid(
            "auxiliary atomic edge step is nonpositive or numerically singular",
        ));
    }
    let edge_step = delta / scale;
    if !edge_step.is_finite() || edge_step <= 0. {
        return Err(invalid(
            "fitted absorption step is nonpositive or nonfinite",
        ));
    }
    let norm = mu
        .iter()
        .zip(&pre_curve)
        .map(|(mu, p)| (scale * mu - p) / delta)
        .collect::<Vec<_>>();
    let flat = norm
        .iter()
        .enumerate()
        .map(|(i, &n)| {
            if i < ie0 {
                n
            } else {
                n - (post_curve[i] - pre_curve[i]) / delta + 1.
            }
        })
        .collect::<Vec<_>>();
    let fpp = mu
        .iter()
        .zip(&background)
        .map(|(m, b)| scale * m - b)
        .collect::<Vec<_>>();
    let residual = reference
        .values
        .iter()
        .zip(&fpp)
        .map(|(f, m)| f - m)
        .collect::<Vec<_>>();
    if background
        .iter()
        .chain(&norm)
        .chain(&flat)
        .chain(&fpp)
        .chain(&residual)
        .any(|v| !v.is_finite())
    {
        return Err(invalid("normalization produced nonfinite values"));
    }
    Ok(MbackResult {
        method: "mback_chantler_v1".into(),
        requested: options.clone(),
        requested_e0: model.e0,
        reference: reference.reference,
        tabulated_edge_ev: edge.energy_ev,
        e0,
        pre_edge: pre,
        post_edge: post,
        energy: energy.to_vec(),
        fit_indices: indices,
        weights,
        scale,
        energy_scale,
        coefficients,
        emission,
        erfc_width: width,
        erfc_amplitude: amplitude,
        background,
        f2: reference.values,
        fpp,
        residual,
        objective: fit.objective,
        rank,
        condition,
        evaluations,
        edge_step,
        pre_curve,
        post_curve,
        auxiliary_method: "rexafs_prepost_linear_quadratic_v1".into(),
        norm,
        flat,
        warnings,
    })
}
