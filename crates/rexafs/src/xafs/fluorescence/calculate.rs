use super::*;
use crate::atomic::AtomicDataProvider;
use crate::xafs::normalization::{Normalization, PrePostEdge};
use nalgebra::DVector;

pub(super) fn apply(
    model: &FluorescenceCorrection,
    energy: &[f64],
    mu: &[f64],
    data: &dyn AtomicDataProvider,
) -> Result<FluorescenceCorrectionResult> {
    if energy.len() != mu.len()
        || energy.len() < 8
        || energy.iter().any(|v| !v.is_finite() || *v <= 0.)
        || mu.iter().any(|v| !v.is_finite())
        || energy.windows(2).any(|w| w[0] >= w[1])
    {
        return Err(invalid("provide matching finite arrays with at least eight points and positive, strictly increasing energies"));
    }
    if model
        .reference
        .as_ref()
        .is_some_and(|r| r != data.identity())
    {
        return Err(invalid("the recorded atomic data version is unavailable"));
    }
    if model.degree > 5 {
        return Err(invalid("internal normalization degree must be 0 through 5"));
    }
    let angle = |value: Option<f64>, name: &str| -> Result<f64> {
        let value = value.ok_or_else(|| {
            invalid(format!(
                "supply the measured {name} angle from the sample surface"
            ))
        })?;
        if !value.is_finite() || value <= 0. || value > 90. {
            return Err(invalid(format!(
                "{name} angle must be greater than 0 and at most 90 degrees from the surface"
            )));
        }
        Ok(value)
    };
    let incident = angle(model.incidence_deg, "incident")?;
    let exit = angle(model.exit_deg, "exit")?;
    let edge = data.edge(&model.element, &model.edge).map_err(invalid)?;
    let emission = data
        .emission(
            &model.element,
            model
                .emission
                .as_ref()
                .ok_or_else(|| invalid("select the detected emission line or family"))?,
        )
        .map_err(invalid)?;
    if emission.energy_ev >= edge.energy_ev
        || emission.lines.iter().any(|l| l.initial_level != edge.edge)
    {
        return Err(invalid(
            "the emission selection must originate from the selected edge and lie below it",
        ));
    }
    let attenuation = data
        .compound_attenuation(
            &model.formula,
            &[
                emission.energy_ev,
                edge.energy_ev - 10.,
                edge.energy_ev + 10.,
            ],
        )
        .map_err(invalid)?;
    if !attenuation
        .composition
        .iter()
        .any(|v| v.element == edge.element && v.mass_fraction > 0.)
    {
        return Err(invalid(
            "the complete sample formula must contain the absorbing element",
        ));
    }
    let a = &attenuation.curve.values;
    let jump = a[2] - a[1];
    if !jump.is_finite() || jump <= 0. {
        return Err(invalid("compound attenuation has no positive jump at this edge; inspect composition and adjacent edges"));
    }
    let geometry_ratio = incident.to_radians().sin() / exit.to_radians().sin();
    let alpha = (a[0] * geometry_ratio + a[1]) / jump;
    if !alpha.is_finite() || alpha <= 0. {
        return Err(invalid(
            "geometry and attenuation do not yield a finite positive correction constant",
        ));
    }
    let energy_array = DVector::from_vec(energy.to_vec());
    let mu_array = DVector::from_vec(mu.to_vec());
    let mut fit = PrePostEdge::new();
    // Resolve E0 through the common detector, then set every scientific interval
    // explicitly. Do not accept the legacy helper's range swapping or clipping.
    fit.e0 = model.e0;
    if model
        .e0
        .is_some_and(|v| !v.is_finite() || v <= energy[0] || v >= energy[energy.len() - 1])
    {
        return Err(invalid(
            "measured E0 must be finite and strictly inside the energy range",
        ));
    }
    if model.e0.is_none() {
        fit.fill_parameter(&energy_array, &mu_array)
            .map_err(invalid)?;
    }
    let e0 = fit
        .e0
        .ok_or_else(|| invalid("could not determine measured E0"))?;
    let pre = model.pre_edge.unwrap_or([energy[0] - e0, -30.]);
    let post = model
        .post_edge
        .unwrap_or([100., energy[energy.len() - 1] - e0]);
    for (name, range, points) in [("pre-edge", pre, 2), ("post-edge", post, model.degree + 1)] {
        if range.iter().any(|v| !v.is_finite())
            || range[0] >= range[1]
            || range[0] + e0 < energy[0] - 1e-8
            || range[1] + e0 > energy[energy.len() - 1] + 1e-8
        {
            return Err(invalid(format!(
                "{name} interval must be increasing and fully covered by the measurement"
            )));
        }
        if energy
            .iter()
            .filter(|e| **e >= range[0] + e0 && **e < range[1] + e0)
            .count()
            < points.max(2)
        {
            return Err(invalid(format!(
                "not enough {name} points for the requested normalization degree"
            )));
        }
    }
    if pre[1] >= 0. || post[0] <= 0. || post[1] - post[0] < 10. {
        return Err(invalid(
            "pre-edge must be below E0; post-edge must be above E0 and span at least 10 eV",
        ));
    }
    fit.pre_edge_start = Some(pre[0]);
    fit.pre_edge_end = Some(pre[1]);
    fit.norm_start = Some(post[0]);
    fit.norm_end = Some(post[1]);
    fit.norm_polyorder = Some(model.degree as i32);
    fit.n_victoreen = Some(0);
    fit.edge_step = None;
    fit.normalize(&energy_array, &mu_array).map_err(invalid)?;
    let pre_curve: Vec<_> = fit
        .pre_edge
        .as_ref()
        .ok_or_else(|| invalid("internal pre-edge fit missing"))?
        .iter()
        .copied()
        .collect();
    let post_curve: Vec<_> = fit
        .post_edge
        .as_ref()
        .ok_or_else(|| invalid("internal post-edge fit missing"))?
        .iter()
        .copied()
        .collect();
    let at_edge = energy
        .iter()
        .enumerate()
        .min_by(|a, b| (a.1 - e0).abs().total_cmp(&(b.1 - e0).abs()))
        .unwrap()
        .0;
    // Read the unfloored jump from the curves, not the historical helper's floored step.
    let edge_step = post_curve[at_edge] - pre_curve[at_edge];
    if !edge_step.is_finite() || edge_step <= 0. {
        return Err(invalid(
            "internal normalization has no positive fitted edge step",
        ));
    }
    let norm: Vec<_> = mu
        .iter()
        .zip(&pre_curve)
        .map(|(m, p)| (m - p) / edge_step)
        .collect();
    let singularity_threshold = 64. * f64::EPSILON * (alpha + 1.).max(1.);
    let mut denominator = Vec::with_capacity(mu.len());
    let mut factor = Vec::with_capacity(mu.len());
    let mut corrected_mu = Vec::with_capacity(mu.len());
    for (i, (&m, &n)) in mu.iter().zip(&norm).enumerate() {
        let d = alpha + 1. - n;
        if !d.is_finite() || d <= singularity_threshold {
            return Err(invalid(format!("nonpositive or numerically singular denominator at point {i} ({} eV); inspect internal normalization, composition and geometry", energy[i])));
        }
        let f = alpha / d;
        let corrected = m * f;
        if !f.is_finite() || !corrected.is_finite() {
            return Err(invalid(format!("nonfinite corrected signal at point {i}")));
        }
        denominator.push(d);
        factor.push(f);
        corrected_mu.push(corrected);
    }
    let minimum_denominator = denominator.iter().copied().fold(f64::INFINITY, f64::min);
    let maximum_amplification = factor.iter().copied().fold(0., f64::max);
    let mut warnings = vec!["Homogeneous, optically thick XANES model; EXAFS and finite-thickness use are not qualified.".into(), "Corrected-array uncertainties are unavailable because the internal normalization depends on the input.".into()];
    if incident < 5. || exit < 5. {
        warnings.push("An angle is below 5 degrees; near-grazing geometry is sensitive to angular uncertainty.".into());
    }
    if maximum_amplification > 10. || minimum_denominator / (alpha + 1.) < 0.05 {
        warnings.push("Large amplification or a denominator below 5% of alpha+1; inspect sensitivity before interpreting this result.".into());
    }
    Ok(FluorescenceCorrectionResult {
        method: "fluo_elam_v1".into(),
        requested: model.clone(),
        input_mode: AbsorptionMode::Fluorescence,
        energy: energy.to_vec(),
        original_mu: mu.to_vec(),
        corrected_mu,
        edge,
        emission,
        attenuation,
        geometry_ratio,
        alpha,
        internal: FluorescenceInternalNormalization {
            e0,
            pre_edge: pre,
            post_edge: post,
            degree: model.degree,
            edge_step,
            pre_curve,
            post_curve,
            norm,
        },
        denominator,
        factor,
        minimum_denominator,
        maximum_amplification,
        singularity_threshold,
        warnings,
    })
}
