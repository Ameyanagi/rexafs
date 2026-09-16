use super::{measure, Metric, MetricError};
use crate::xafs::{
    analysis::{spectrum_e0, AnalysisInput, AnalysisSpace},
    xasspectrum::XASSpectrum,
};
use serde::{Deserialize, Serialize};
use std::ops::RangeInclusive;

/// Signal representation. Norm is the default; Flat is always explicit.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum MeasurementSpace {
    /// Mapped absorption μ(E), in its original signal units.
    Mu,
    /// Normalized dimensionless μ(E). Missing normalization runs on a copy.
    #[default]
    Norm,
    /// Flattened dimensionless μ(E). Missing normalization runs on a copy.
    Flat,
    /// χ(k) multiplied by k to this nonnegative integer power; k is Å⁻¹.
    Chi { kweight: u8 },
    /// Magnitude of the existing/configured forward transform. R is Å and is
    /// not phase corrected. Missing prerequisites run on a copy, without IFFT.
    Fourier,
}

/// Interpretation of energy coordinates; k and R must use Absolute.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum AxisOrigin {
    Absolute,
    /// Offsets in eV from this spectrum's resolved edge energy.
    #[default]
    E0,
    /// Offsets in eV from a frozen, named reference energy.
    Reference {
        energy_ev: f64,
    },
}

/// One spectrum measurement, independent of desktop projects and jobs.
///
/// Constructors use normalized μ(E) and energy offsets from E₀. For example,
/// `spectrum.measure(&Measurement::mean(-20.0..=30.0))` prepares normalization
/// if needed, checks coverage and returns an owned scalar result. The borrowed
/// spectrum and its settings remain unchanged. Use `.flat()` or `.absolute()`
/// for those explicit alternatives; public fields allow advanced configuration.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Measurement {
    pub metric: Metric,
    pub space: MeasurementSpace,
    pub origin: AxisOrigin,
}

impl Measurement {
    fn new(metric: Metric) -> Self {
        Self {
            metric,
            space: MeasurementSpace::Norm,
            origin: AxisOrigin::E0,
        }
    }
    /// Interpolate normalized μ at an E₀ offset in eV; no extrapolation.
    pub fn point(offset_ev: f64) -> Self {
        Self::new(Metric::Point { x: offset_ev })
    }
    /// Width-weighted region mean. Endpoints are offsets from E₀ in eV.
    pub fn mean(range_ev: RangeInclusive<f64>) -> Self {
        Self::new(Metric::Mean {
            start: *range_ev.start(),
            end: *range_ev.end(),
        })
    }
    /// Region integral, without baseline subtraction, in signal units × eV.
    pub fn integral(range_ev: RangeInclusive<f64>) -> Self {
        Self::new(Metric::Integral {
            start: *range_ev.start(),
            end: *range_ev.end(),
        })
    }
    /// Region maximum; the result also records its absolute position in eV.
    pub fn maximum(range_ev: RangeInclusive<f64>) -> Self {
        Self::new(Metric::Maximum {
            start: *range_ev.start(),
            end: *range_ev.end(),
        })
    }
    /// Use flattened absorption, preserving the same range and origin.
    pub fn flat(mut self) -> Self {
        self.space = MeasurementSpace::Flat;
        self
    }
    /// Measure mapped raw μ(E), retaining the chosen energy origin.
    /// A typed normalized component cannot be relabeled as raw absorption.
    pub fn raw_mu(mut self) -> Self {
        self.space = MeasurementSpace::Mu;
        self
    }
    /// Measure k-weighted χ(k). Coordinates now mean absolute k in Å⁻¹;
    /// the nonnegative integer weight defaults to zero only when you pass 0.
    /// Background subtraction runs on a copy if necessary, without FFT/IFFT.
    pub fn chi(mut self, kweight: u8) -> Self {
        self.space = MeasurementSpace::Chi { kweight };
        self.origin = AxisOrigin::Absolute;
        self
    }
    /// Measure Fourier magnitude. Coordinates now mean absolute, uncorrected R
    /// in Å. Uses the spectrum's forward-transform settings; never runs IFFT.
    pub fn fourier(mut self) -> Self {
        self.space = MeasurementSpace::Fourier;
        self.origin = AxisOrigin::Absolute;
        self
    }
    /// Interpret coordinates as absolute axis values rather than E₀ offsets.
    pub fn absolute(mut self) -> Self {
        self.origin = AxisOrigin::Absolute;
        self
    }

    /// Owned native arrays and resolved edge energy for a preview or measurement.
    /// Only necessary prerequisites run; no display sampling or interpolation is
    /// applied here. Errors identify unavailable preparation or quantity mismatch.
    pub fn arrays(&self, spectrum: &XASSpectrum) -> Result<MeasurementArrays, MetricError> {
        let failure =
            |e: crate::xafs::errors::AnalysisError| MetricError::Preparation(e.to_string());
        let mut prepared;
        let source = if matches!(self.origin, AxisOrigin::E0) && spectrum_e0(spectrum).is_none() {
            prepared = spectrum.clone();
            prepared
                .find_e0()
                .map_err(|e| MetricError::Preparation(e.to_string()))?;
            &prepared
        } else {
            spectrum
        };
        if !matches!(
            self.space,
            MeasurementSpace::Mu | MeasurementSpace::Norm | MeasurementSpace::Flat
        ) && self.origin != AxisOrigin::Absolute
        {
            return Err(MetricError::Quantity(
                "k and R coordinates must be absolute".into(),
            ));
        }
        match self.space {
            MeasurementSpace::Mu => {
                if source.preserves_prepared_values() {
                    return Err(MetricError::Quantity(
                        "prepared absorption is not raw μ(E)".into(),
                    ));
                }
                let x = source
                    .energy
                    .as_ref()
                    .ok_or(MetricError::Shape)?
                    .as_slice()
                    .to_vec();
                let y = source
                    .mu
                    .as_ref()
                    .ok_or(MetricError::Shape)?
                    .as_slice()
                    .to_vec();
                Ok(MeasurementArrays {
                    axis: x,
                    signal: y,
                    e0_ev: spectrum_e0(source),
                })
            }
            MeasurementSpace::Norm | MeasurementSpace::Flat | MeasurementSpace::Chi { .. } => {
                let space = match self.space {
                    MeasurementSpace::Flat => AnalysisSpace::Flat,
                    MeasurementSpace::Chi { kweight } => AnalysisSpace::Chi {
                        kweight: kweight as f64,
                    },
                    _ => AnalysisSpace::Norm,
                };
                let input = AnalysisInput::new(source, space).map_err(failure)?;
                Ok(MeasurementArrays {
                    axis: input.x.as_slice().to_vec(),
                    signal: input.y.as_slice().to_vec(),
                    e0_ev: input.e0,
                })
            }
            MeasurementSpace::Fourier => {
                let mut sp = source.clone();
                if sp.r().is_none() || sp.chir_mag().is_none() {
                    sp.fft()
                        .map_err(|e| MetricError::Preparation(e.to_string()))?;
                }
                Ok(MeasurementArrays {
                    axis: sp.r().ok_or(MetricError::Shape)?.as_slice().to_vec(),
                    signal: sp.chir_mag().ok_or(MetricError::Shape)?.as_slice().to_vec(),
                    e0_ev: spectrum_e0(&sp),
                })
            }
        }
    }

    /// Resolve the actual native-axis interval; energies use eV.
    pub fn resolved_metric(&self, e0: Option<f64>) -> Result<Metric, MetricError> {
        let origin = match self.origin {
            AxisOrigin::Absolute => 0.0,
            AxisOrigin::E0 => {
                e0.ok_or_else(|| MetricError::Preparation("E₀ is unavailable".into()))?
            }
            AxisOrigin::Reference { energy_ev } => energy_ev,
        };
        if !origin.is_finite() {
            return Err(MetricError::Range);
        }
        Ok(self.metric.shifted(origin))
    }

    /// Scalar unit under the recorded representation and forward-FFT k weight.
    /// Raw μ units remain unspecified when the source did not declare them.
    pub fn unit(&self, fourier_kweight: f64) -> String {
        let (signal, axis) = match self.space {
            MeasurementSpace::Mu => ("source μ units".into(), "eV"),
            MeasurementSpace::Norm | MeasurementSpace::Flat => ("1".into(), "eV"),
            MeasurementSpace::Chi { kweight } => (
                if kweight == 0 {
                    "1".into()
                } else {
                    format!("Å^-{kweight}")
                },
                "Å^-1",
            ),
            MeasurementSpace::Fourier => (format!("Å^-{}", fourier_kweight + 1.0), "Å"),
        };
        match self.metric {
            Metric::Integral { .. } => format!("{signal} · {axis}"),
            Metric::Centroid { .. } => axis.into(),
            _ => signal,
        }
    }
}

/// Owned native arrays for explicit preview/export, without display sampling.
#[derive(Clone, Debug, PartialEq)]
pub struct MeasurementArrays {
    /// Energy (eV), k (Å⁻¹) or uncorrected R (Å), according to the definition.
    pub axis: Vec<f64>,
    /// Values in the chosen representation, before point/region evaluation.
    pub signal: Vec<f64>,
    /// Edge origin found or retained during preparation, when available.
    pub e0_ev: Option<f64>,
}

/// Scalar, resolved coordinates and quantity definition needed to interpret it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MeasurementResult {
    pub measurement: Measurement,
    /// Finite scalar in `unit`.
    pub value: f64,
    /// Maximum/centroid/point position on the absolute native axis, if relevant.
    pub position: Option<f64>,
    /// Actual native-axis bounds, inclusive; a point has equal bounds.
    pub range: [f64; 2],
    /// Absent: no independent input-error model was supplied.
    pub standard_error: Option<f64>,
    pub unit: String,
    pub e0_ev: Option<f64>,
}

impl XASSpectrum {
    /// Measure a point/region, automatically preparing only necessary stages on
    /// a private copy. Recommended default: `Measurement::mean(-20.0..=30.0)`
    /// measures normalized μ(E), with coordinates relative to E₀. Select Flat
    /// explicitly with `.flat()`. Missing coverage is an error, not clipping.
    /// No files, threads or project records are created; inputs remain unchanged.
    pub fn measure(&self, measurement: &Measurement) -> Result<MeasurementResult, MetricError> {
        let MeasurementArrays {
            axis: x,
            signal: y,
            e0_ev: e0,
        } = measurement.arrays(self)?;
        let value = measure(&x, &y, measurement.resolved_metric(e0)?)?;
        let weight = self
            .xftf
            .as_ref()
            .and_then(|fft| fft.kweight)
            .unwrap_or(2.)
            .max(0.)
            .floor();
        Ok(MeasurementResult {
            measurement: measurement.clone(),
            value: value.value,
            position: value.position,
            range: [value.start, value.end],
            standard_error: value.standard_error,
            unit: measurement.unit(weight),
            e0_ev: e0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_call_api_preserves_inputs_and_requires_explicit_flat() {
        let x = [98., 99., 100., 101., 102., 103.];
        let y = [0., 0.1, 0.5, 0.8, 1., 1.];
        let spectrum = XASSpectrum::from_prepared(&x, &y, AnalysisSpace::Norm, 100.).unwrap();
        let before = serde_json::to_value(&spectrum).unwrap();
        let result = spectrum.measure(&Measurement::mean(0.0..=2.0)).unwrap();
        assert_eq!(result.range[0], 100.);
        assert!((result.value - 0.775).abs() < 1e-14);
        assert_eq!(result.unit, "1");
        assert!(spectrum.measure(&Measurement::point(0.).flat()).is_err());
        assert!(spectrum
            .measure(&Measurement::point(100.).raw_mu().absolute())
            .is_err());
        assert_eq!(serde_json::to_value(&spectrum).unwrap(), before);
    }
}
