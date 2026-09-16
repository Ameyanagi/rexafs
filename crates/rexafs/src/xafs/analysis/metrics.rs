//! Scalar measurements on native spectral grids (development API).
//!
//! [`measure`] uses linear interpolation and exact integration of that
//! piecewise-linear curve. It neither resamples a whole spectrum nor extrapolates
//! beyond its measured support. A region mean is its integral divided by its
//! width, so densely sampled portions do not receive extra weight. This is a
//! rexafs measurement convention, not a fitted peak area or a concentration.
//!
//! Inputs are borrowed and unchanged. Missing coverage, masked intervals and
//! invalid arrays return typed errors; callers should retain those errors as
//! missing outcomes instead of replacing them with zero. The composite trapezoidal
//! convention is also described in the [SciPy reference](https://docs.scipy.org/doc/scipy/reference/generated/scipy.integrate.trapezoid.html);
//! strict coverage and increasing-axis requirements are rexafs choices. Uncertainty is not
//! inferred from signal amplitude. Independent-error propagation is future work.

use serde::{Deserialize, Serialize};

/// Operation applied to a fully covered point or interval on the native axis.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Metric {
    /// Linear interpolation at `x`; output has the signal's units.
    Point { x: f64 },
    /// Largest value including interpolated boundaries; ties choose lowest x.
    Maximum { start: f64, end: f64 },
    /// Integral of the selected signal, with no implicit baseline subtraction.
    /// Units are signal units multiplied by axis units.
    Integral { start: f64, end: f64 },
    /// Integral divided by interval width, in signal units.
    Mean { start: f64, end: f64 },
    /// First moment divided by integral, in axis units. Signed signals are
    /// rejected: a sign-changing difference does not define a peak centroid.
    /// A baseline must be subtracted explicitly before calling this operation.
    Centroid { start: f64, end: f64 },
}

impl Metric {
    /// Resolved bounds. A point has equal endpoints; regions require end > start.
    pub fn bounds(self) -> (f64, f64) {
        match self {
            Self::Point { x } => (x, x),
            Self::Maximum { start, end }
            | Self::Integral { start, end }
            | Self::Mean { start, end }
            | Self::Centroid { start, end } => (start, end),
        }
    }

    /// Translate a definition, for example from E₀-relative to absolute eV.
    /// `measure` validates the translated coordinates before using them.
    pub fn shifted(self, origin: f64) -> Self {
        match self {
            Self::Point { x } => Self::Point { x: x + origin },
            Self::Maximum { start, end } => Self::Maximum {
                start: start + origin,
                end: end + origin,
            },
            Self::Integral { start, end } => Self::Integral {
                start: start + origin,
                end: end + origin,
            },
            Self::Mean { start, end } => Self::Mean {
                start: start + origin,
                end: end + origin,
            },
            Self::Centroid { start, end } => Self::Centroid {
                start: start + origin,
                end: end + origin,
            },
        }
    }
}

/// One finite scalar and the exact interval used to obtain it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MetricValue {
    pub value: f64,
    /// Position of a maximum or centroid; point position for interpolation.
    pub position: Option<f64>,
    pub start: f64,
    pub end: f64,
    /// Independent statistical information was not supplied to this operator.
    pub standard_error: Option<f64>,
}

/// Reasons a requested measurement is unavailable. No failed result is zero.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, thiserror::Error)]
pub enum MetricError {
    #[error("The axis and signal need equal lengths and at least two points")]
    Shape,
    #[error("Preparation failed: {0}")]
    Preparation(String),
    #[error("Incompatible quantity: {0}")]
    Quantity(String),
    #[error("Axis values must be finite and strictly increasing (index {index})")]
    Axis { index: usize },
    #[error("Signal value is not finite (index {index})")]
    Signal { index: usize },
    #[error("Choose finite coordinates and a positive region width")]
    Range,
    #[error("Requested interval [{start}, {end}] is outside [{available_start}, {available_end}]")]
    Coverage {
        start: f64,
        end: f64,
        available_start: f64,
        available_end: f64,
    },
    #[error("The interval intersects a masked gap [{start}, {end}]")]
    Masked { start: f64, end: f64 },
    #[error("Centroid needs a nonnegative signal with a nonzero integral")]
    Centroid,
    #[error("The scalar calculation exceeded finite numeric range")]
    Nonfinite,
}

/// Measure a native-grid curve with full interval coverage.
///
/// `axis` must be finite and strictly increasing; `signal` must be finite and
/// the same length. `masked` contains closed excluded intervals in the same
/// axis units; intersecting one makes the result unavailable. Empty masks are
/// the recommended default when the source has no declared gaps. Sparse but
/// otherwise valid grids are interpolated linearly, without invented gap rules.
///
/// Region integration uses the trapezoidal rule on clipped native segments;
/// it is exact for their linear interpolant, including fractional boundaries.
/// The centroid's first moment integrates x*y(x) analytically on each segment.
/// Arrays and coordinates are unchanged, and no preparation stages run here.
pub fn measure_masked(
    axis: &[f64],
    signal: &[f64],
    metric: Metric,
    masked: &[(f64, f64)],
) -> Result<MetricValue, MetricError> {
    if axis.len() < 2 || axis.len() != signal.len() {
        return Err(MetricError::Shape);
    }
    for (index, &x) in axis.iter().enumerate() {
        if !x.is_finite() || (index > 0 && x <= axis[index - 1]) {
            return Err(MetricError::Axis { index });
        }
    }
    if let Some(index) = signal.iter().position(|v| !v.is_finite()) {
        return Err(MetricError::Signal { index });
    }
    let (start, end) = metric.bounds();
    if !start.is_finite()
        || !end.is_finite()
        || (!matches!(metric, Metric::Point { .. }) && end <= start)
    {
        return Err(MetricError::Range);
    }
    let last = axis.len() - 1;
    if start < axis[0] || end > axis[last] {
        return Err(MetricError::Coverage {
            start,
            end,
            available_start: axis[0],
            available_end: axis[last],
        });
    }
    for &(lo, hi) in masked {
        if !lo.is_finite() || !hi.is_finite() || hi < lo {
            return Err(MetricError::Range);
        }
        if start <= hi && end >= lo {
            return Err(MetricError::Masked { start: lo, end: hi });
        }
    }
    let interpolate = |x: f64| {
        let i = axis
            .partition_point(|&a| a <= x)
            .saturating_sub(1)
            .min(last - 1);
        let fraction = (x - axis[i]) / (axis[i + 1] - axis[i]);
        // Convex weights avoid overflow in y1-y0 for large opposite signs.
        (1.0 - fraction) * signal[i] + fraction * signal[i + 1]
    };
    let y0 = interpolate(start);
    let mut value = y0;
    let mut position = Some(start);
    if !matches!(metric, Metric::Point { .. }) {
        let mut x0 = start;
        let mut previous = y0;
        let mut area = 0.0;
        let mut moment = 0.0;
        let mut absolute_area = 0.0;
        let mut nonnegative = y0 >= 0.0;
        let first = axis.partition_point(|&x| x <= start);
        let stop = axis.partition_point(|&x| x < end);
        for (x1, next) in (first..stop)
            .map(|i| (axis[i], signal[i]))
            .chain(std::iter::once((end, interpolate(end))))
        {
            let width = x1 - x0;
            let segment_area = width * (previous * 0.5 + next * 0.5);
            area += segment_area;
            absolute_area += width * (previous.abs() * 0.5 + next.abs() * 0.5);
            // Compute around start to avoid subtracting large energy moments.
            moment += (x0 - start) * segment_area + width * width * (previous / 6.0 + next / 3.0);
            nonnegative &= next >= 0.0;
            if next > value {
                value = next;
                position = Some(x1);
            }
            x0 = x1;
            previous = next;
        }
        match metric {
            Metric::Integral { .. } => {
                value = area;
                position = None;
            }
            Metric::Mean { .. } => {
                value = area / (end - start);
                position = None;
            }
            Metric::Centroid { .. } => {
                if !nonnegative || area <= f64::EPSILON * absolute_area || area <= 0.0 {
                    return Err(MetricError::Centroid);
                }
                value = start + moment / area;
                position = Some(value);
            }
            _ => {}
        }
    }
    if !value.is_finite() || position.is_some_and(|x| !x.is_finite()) {
        return Err(MetricError::Nonfinite);
    }
    Ok(MetricValue {
        value,
        position,
        start,
        end,
        standard_error: None,
    })
}

/// Measure arrays without excluded intervals. See [`measure_masked`] when the
/// source declares gaps. Coordinates use the units of `axis`.
pub fn measure(axis: &[f64], signal: &[f64], metric: Metric) -> Result<MetricValue, MetricError> {
    measure_masked(axis, signal, metric, &[])
}

mod spectrum;
pub use spectrum::{
    AxisOrigin, Measurement, MeasurementArrays, MeasurementResult, MeasurementSpace,
};

#[cfg(test)]
mod tests {
    use super::measure_masked as measure;
    use super::*;

    #[test]
    fn irregular_grid_clipped_linear_regions_are_exact() {
        let x = [0., 0.1, 0.4, 2., 5.];
        let y = x.map(|x| 2. * x + 1.);
        for (metric, expected) in [
            (Metric::Point { x: 0.25 }, 1.5),
            (
                Metric::Integral {
                    start: 0.25,
                    end: 3.,
                },
                11.6875,
            ),
            (
                Metric::Mean {
                    start: 0.25,
                    end: 3.,
                },
                4.25,
            ),
            (
                Metric::Maximum {
                    start: 0.25,
                    end: 3.,
                },
                7.,
            ),
        ] {
            assert!((measure(&x, &y, metric, &[]).unwrap().value - expected).abs() < 1e-13);
        }
        let centroid = measure(
            &[0., 0.01, 1., 2.],
            &[0., 0.01, 1., 2.],
            Metric::Centroid { start: 0., end: 2. },
            &[],
        )
        .unwrap();
        assert!((centroid.value - 4. / 3.).abs() < 1e-14);
    }

    #[test]
    fn coverage_masks_and_corrupt_axes_never_produce_zero() {
        let x = [0., 1., 2.];
        let y = [1., 2., 1.];
        assert!(matches!(
            measure(&x, &y, Metric::Point { x: -0.01 }, &[]),
            Err(MetricError::Coverage { .. })
        ));
        assert!(matches!(
            measure(
                &x,
                &y,
                Metric::Integral { start: 0., end: 2. },
                &[(0.5, 0.6)]
            ),
            Err(MetricError::Masked { .. })
        ));
        assert_eq!(
            measure(&x, &y, Metric::Mean { start: 1., end: 1. }, &[]),
            Err(MetricError::Range)
        );
        assert!(matches!(
            measure(&[0., 1., 1.], &y, Metric::Point { x: 0. }, &[]),
            Err(MetricError::Axis { index: 2 })
        ));
        assert!(matches!(
            measure(&x, &[1., f64::NAN, 1.], Metric::Point { x: 0. }, &[]),
            Err(MetricError::Signal { index: 1 })
        ));
        assert_eq!(
            measure(
                &x,
                &[1., -1., 1.],
                Metric::Centroid { start: 0., end: 2. },
                &[]
            ),
            Err(MetricError::Centroid)
        );
    }

    #[test]
    fn maxima_ties_choose_first_and_boundary_points_are_valid() {
        let x = [100., 101., 102., 103.];
        let y = [0., 2., 2., 0.];
        let max = measure(
            &x,
            &y,
            Metric::Maximum {
                start: 100.5,
                end: 102.5,
            },
            &[],
        )
        .unwrap();
        assert_eq!(max.position, Some(101.));
        assert_eq!(
            measure(&x, &y, Metric::Point { x: 103. }, &[])
                .unwrap()
                .value,
            0.
        );
        assert_eq!(
            Metric::Point { x: 2. }.shifted(100.),
            Metric::Point { x: 102. }
        );
    }
}
