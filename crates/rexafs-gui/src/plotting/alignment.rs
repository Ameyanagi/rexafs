//! Alignment display: native-grid derivatives in the reference's energy window.
use super::*;

struct Curve {
    energy: Vec<f64>,
    derivative: Vec<f64>,
}

fn curve(spectrum: &XASSpectrum, bounds: (f64, f64)) -> Result<Curve, String> {
    let energy = spectrum
        .energy
        .as_ref()
        .ok_or("Spectrum has no energy grid")?;
    let mu = spectrum
        .mu
        .as_ref()
        .ok_or("Spectrum has no absorption values")?;
    if energy.len() < 3
        || energy.len() != mu.len()
        || energy.iter().chain(mu.iter()).any(|v| !v.is_finite())
        || energy.as_slice().windows(2).any(|pair| pair[0] >= pair[1])
    {
        return Err("Alignment preview needs finite spectra with increasing energy grids".into());
    }
    let mut derivative = rexafs::xafs::tools::dmude(energy, mu);
    let peak = energy
        .iter()
        .zip(derivative.iter())
        .filter(|(e, _)| **e >= bounds.0 && **e <= bounds.1)
        .map(|(_, y)| y.abs())
        .fold(0.0, f64::max);
    if peak > 0.0 {
        derivative /= peak;
    }
    Ok(Curve {
        energy: vecs(energy),
        derivative: vecs(&derivative),
    })
}

pub(super) fn build(
    before: &XASSpectrum,
    after: &XASSpectrum,
    (name, reference): (&str, &XASSpectrum),
    window: (f64, f64),
    theme: &Theme,
) -> Result<Plot, String> {
    let e0 = match reference.e0() {
        Some(e0) => e0,
        None => reference
            .edge_feature_energy(rexafs::xafs::tools::EdgeFeature::DerivativeMax)
            .map_err(|e| e.to_string())?,
    };
    let bounds = (e0 + window.0, e0 + window.1);
    if !bounds.0.is_finite() || !bounds.1.is_finite() || bounds.0 >= bounds.1 {
        return Err("Enter an increasing, finite alignment window".into());
    }
    let curves = [
        curve(before, bounds)?,
        curve(after, bounds)?,
        curve(reference, bounds)?,
    ];
    let mut plot = Plot::new().theme(theme.plot_theme());
    let labels = [
        "Original".to_string(),
        "Aligned".to_string(),
        format!("Reference: {name}"),
    ];
    let mut ymin: f64 = 0.;
    let mut ymax: f64 = 0.;
    for (i, (curve, label)) in curves.iter().zip(labels).enumerate() {
        for (&energy, &y) in curve.energy.iter().zip(&curve.derivative) {
            if energy >= bounds.0 && energy <= bounds.1 {
                ymin = ymin.min(y);
                ymax = ymax.max(y);
            }
        }
        plot = plot
            .line(&curve.energy, &curve.derivative)
            .color(trace_color(theme, i))
            .line_style(if i == 0 {
                LineStyle::Dashed
            } else {
                LineStyle::Solid
            })
            .line_width(if i == 1 { 1.8 } else { 1.1 })
            .label(label)
            .into();
    }
    let padding = (ymax - ymin).max(1.) * 0.05;
    Ok(plot
        .xlabel("Energy (eV)")
        .ylabel("scaled dμ/dE")
        .xlim(bounds.0, bounds.1)
        .ylim(ymin - padding, ymax + padding)
        .legend_position(LegendPosition::UpperRight))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derivative_uses_native_spacing_and_scales_only_for_display() {
        let mut spectrum = XASSpectrum::new();
        spectrum.set_spectrum(vec![0., 1., 2., 4., 7.], vec![10., 12., 14., 18., 24.]);
        let original = (spectrum.energy.clone(), spectrum.mu.clone());
        let result = curve(&spectrum, (0., 4.)).unwrap();
        assert_eq!(result.energy, vec![0., 1., 2., 4., 7.]);
        assert_eq!(result.derivative, vec![1.; 5]);
        assert_eq!((spectrum.energy.clone(), spectrum.mu.clone()), original);
        spectrum.shift_energy(0.4);
        let shifted = curve(&spectrum, (0., 4.)).unwrap();
        for (before, after) in result.energy.iter().zip(shifted.energy) {
            assert!((after - before - 0.4).abs() < 1e-12);
        }
        assert!(shifted.derivative.iter().all(|y| (y - 1.).abs() < 1e-12));
    }
}
