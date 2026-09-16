//! Independent point-error propagation for linear scalar measurements.
use super::{measure, Metric, MetricError, MetricValue};

/// Measure arrays and propagate supplied independent point standard deviations.
///
/// `standard_errors[i]` describes uncertainty of **`signal[i]` in its current
/// representation**, in the same units. It must be finite and nonnegative.
/// Point, integral and mean are linear combinations `m = Σ w_i y_i`; the
/// returned standard error is `sqrt(Σ (w_i σ_i)^2)`. Native-point weights are
/// accumulated before squaring, so adjacent interpolated segments do not
/// incorrectly count a shared endpoint as independent observations.
///
/// This is the independent-input special case of the
/// [NIST law of propagation of uncertainty](https://www.nist.gov/pml/nist-technical-note-1297/nist-tn-1297-appendix-law-propagation-uncertainty).
/// The weights come from rexafs's piecewise-linear measurement convention.
/// The axis and boundaries are treated as exact. Correlations, uncertainty in
/// normalization/alignment/baselines, and confidence intervals are not inferred.
/// Maximum and centroid require other error models and return an explicit error.
/// Arrays remain unchanged; no preparation runs in this array-level operation.
pub fn measure_with_errors(
    axis: &[f64],
    signal: &[f64],
    standard_errors: &[f64],
    metric: Metric,
) -> Result<MetricValue, MetricError> {
    let mut result = measure(axis, signal, metric)?;
    if standard_errors.len() != axis.len() {
        return Err(MetricError::UncertaintyShape);
    }
    if let Some(index) = standard_errors
        .iter()
        .position(|v| !v.is_finite() || *v < 0.)
    {
        return Err(MetricError::UncertaintyValue { index });
    }
    let mut weights = vec![0.; axis.len()];
    let (start, end) = metric.bounds();
    match metric {
        Metric::Point { x } => {
            let i = axis
                .partition_point(|&a| a <= x)
                .saturating_sub(1)
                .min(axis.len() - 2);
            let t = (x - axis[i]) / (axis[i + 1] - axis[i]);
            weights[i] = 1. - t;
            weights[i + 1] = t;
        }
        Metric::Integral { .. } | Metric::Mean { .. } => {
            for i in 0..axis.len() - 1 {
                let a = start.max(axis[i]);
                let b = end.min(axis[i + 1]);
                if b <= a {
                    continue;
                }
                let width = axis[i + 1] - axis[i];
                let ta = (a - axis[i]) / width;
                let tb = (b - axis[i]) / width;
                let right = (b - a) * (ta + tb) * 0.5;
                weights[i] += b - a - right;
                weights[i + 1] += right;
            }
            if matches!(metric, Metric::Mean { .. }) {
                for weight in &mut weights {
                    *weight /= end - start;
                }
            }
        }
        _ => return Err(MetricError::UncertaintyModel),
    }
    let error = weights
        .iter()
        .zip(standard_errors)
        .fold(0_f64, |error, (&weight, &sigma)| {
            error.hypot(weight * sigma)
        });
    if !error.is_finite() {
        return Err(MetricError::Nonfinite);
    }
    result.standard_error = Some(error);
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn clipped_irregular_integral_propagates_native_point_weights() {
        let x = [0., 1., 3.];
        let y = [1., 3., 7.];
        let sigma = [1., 2., 3.];
        let metric = Metric::Integral {
            start: 0.5,
            end: 2.,
        };
        let result = measure_with_errors(&x, &y, &sigma, metric).unwrap();
        // Exact weights are [0.125, 1.125, 0.25]; sigma = 2.375.
        assert!((result.standard_error.unwrap() - 2.375).abs() < 1e-14);
        let mean = measure_with_errors(
            &x,
            &y,
            &sigma,
            Metric::Mean {
                start: 0.5,
                end: 2.,
            },
        )
        .unwrap();
        assert!((mean.standard_error.unwrap() - 2.375 / 1.5).abs() < 1e-14);
        let point = measure_with_errors(&x, &y, &sigma, Metric::Point { x: 0.5 }).unwrap();
        assert!((point.standard_error.unwrap() - 1.25_f64.sqrt()).abs() < 1e-14);
        let shared = measure_with_errors(
            &[0., 1., 2.],
            &[1., 1., 1.],
            &[1., 1., 1.],
            Metric::Integral { start: 0., end: 2. },
        )
        .unwrap();
        assert!((shared.standard_error.unwrap() - 1.5_f64.sqrt()).abs() < 1e-14);
    }
    #[test]
    fn unknown_or_invalid_error_models_are_not_invented() {
        assert!(matches!(
            measure_with_errors(&[0., 1.], &[1., 2.], &[1.], Metric::Point { x: 0. }),
            Err(MetricError::UncertaintyShape)
        ));
        assert!(matches!(
            measure_with_errors(&[0., 1.], &[1., 2.], &[1., -1.], Metric::Point { x: 0. }),
            Err(MetricError::UncertaintyValue { index: 1 })
        ));
        assert!(matches!(
            measure_with_errors(
                &[0., 1.],
                &[1., 2.],
                &[1., 1.],
                Metric::Maximum { start: 0., end: 1. }
            ),
            Err(MetricError::UncertaintyModel)
        ));
        assert_eq!(
            measure(&[0., 1.], &[1., 2.], Metric::Point { x: 0. })
                .unwrap()
                .standard_error,
            None
        );
    }
}
