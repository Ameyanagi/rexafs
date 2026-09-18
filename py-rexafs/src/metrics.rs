//! Python argument conversion for native scalar spectrum measurements.
use numpy::PyReadonlyArray1;
use pyo3::{exceptions::PyValueError, prelude::*};
use rexafs::prelude::{AxisOrigin, Measurement, MeasurementSpace, Metric};

pub fn definition(
    operation: &str,
    coordinates: &Bound<'_, PyAny>,
    space: &str,
    origin: Option<&str>,
    kweight: u8,
) -> PyResult<Measurement> {
    let metric = if operation == "point" {
        Metric::Point {
            x: coordinates
                .extract()
                .map_err(|_| PyValueError::new_err("point requires one numeric coordinate"))?,
        }
    } else {
        let (start, end): (f64, f64) = coordinates
            .extract()
            .map_err(|_| PyValueError::new_err("region measurements require (start, end)"))?;
        match operation {
            "mean" => Metric::Mean { start, end },
            "integral" => Metric::Integral { start, end },
            "maximum" => Metric::Maximum { start, end },
            _ => {
                return Err(PyValueError::new_err(
                    "operation must be point, mean, integral, or maximum",
                ))
            }
        }
    };
    if space != "chi" && kweight != 0 {
        return Err(PyValueError::new_err("kweight applies only to chi"));
    }
    let space = match space {
        "mu" => MeasurementSpace::Mu,
        "norm" => MeasurementSpace::Norm,
        "flat" => MeasurementSpace::Flat,
        "chi" => MeasurementSpace::Chi { kweight },
        "fourier" => MeasurementSpace::Fourier,
        _ => {
            return Err(PyValueError::new_err(
                "space must be mu, norm, flat, chi, or fourier",
            ))
        }
    };
    let energy = matches!(
        space,
        MeasurementSpace::Mu | MeasurementSpace::Norm | MeasurementSpace::Flat
    );
    let origin = match origin.unwrap_or(if energy { "e0" } else { "absolute" }) {
        "e0" => AxisOrigin::E0,
        "absolute" => AxisOrigin::Absolute,
        _ => return Err(PyValueError::new_err("origin must be e0 or absolute")),
    };
    Ok(Measurement {
        metric,
        space,
        origin,
    })
}

pub fn errors(py: Python<'_>, input: &Bound<'_, PyAny>) -> PyResult<Vec<f64>> {
    array(py, input, "errors")
}

/// Copy a named one-dimensional numeric argument with contextual shape errors.
pub fn array(py: Python<'_>, input: &Bound<'_, PyAny>, name: &str) -> PyResult<Vec<f64>> {
    let array = py
        .import("numpy")?
        .getattr("asarray")?
        .call1((input, "float64"))?;
    let array = array
        .extract::<PyReadonlyArray1<'_, f64>>()
        .map_err(|_| PyValueError::new_err(format!("{name} must be one-dimensional")))?;
    Ok(array.as_array().iter().copied().collect())
}

/// Owned scalar result from Spectrum.measure (since 0.2.10).
///
/// value is in unit; range and optional position use the absolute native axis
/// (eV, inverse angstroms, or angstroms). standard_error is absent unless
/// independent point standard deviations were supplied. It is not a confidence
/// interval. to_json() includes the exact measurement definition and resolved E0.
#[pyclass(name = "MeasurementResult", module = "rexafs", frozen)]
pub struct PyMeasurementResult {
    pub inner: rexafs::prelude::MeasurementResult,
}

#[pymethods]
impl PyMeasurementResult {
    /// Finite scalar in unit.
    #[getter]
    fn value(&self) -> f64 {
        self.inner.value
    }
    /// Signal unit for point/mean/maximum; signal times axis unit for integral.
    #[getter]
    fn unit(&self) -> &str {
        &self.inner.unit
    }
    /// Absolute point/maximum coordinate, otherwise None.
    #[getter]
    fn position(&self) -> Option<f64> {
        self.inner.position
    }
    /// Absolute native-axis bounds: eV, inverse angstroms, or angstroms.
    #[getter]
    fn range(&self) -> (f64, f64) {
        (self.inner.range[0], self.inner.range[1])
    }
    /// Propagated independent standard error when supplied, otherwise None.
    #[getter]
    fn standard_error(&self) -> Option<f64> {
        self.inner.standard_error
    }
    /// Resolved absorption-edge energy in eV, or None when unnecessary.
    #[getter]
    fn e0_ev(&self) -> Option<f64> {
        self.inner.e0_ev
    }
    /// Serialize the owned result and definition as JSON, without changing inputs.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner).map_err(|e| PyValueError::new_err(e.to_string()))
    }
}
