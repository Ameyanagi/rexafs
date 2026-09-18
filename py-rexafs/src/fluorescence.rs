//! Owned fluorescence correction settings and historical results.
use numpy::PyArray1;
use pyo3::{exceptions::PyValueError, prelude::*};

pub(crate) fn invalid(e: impl std::fmt::Display) -> PyErr {
    PyValueError::new_err(e.to_string())
}
pub(crate) fn mode_name(mode: rexafs::AbsorptionMode) -> &'static str {
    match mode {
        rexafs::AbsorptionMode::Unknown => "unknown",
        rexafs::AbsorptionMode::Transmission => "transmission",
        rexafs::AbsorptionMode::Fluorescence => "fluorescence",
    }
}
pub(crate) fn mode(value: &str) -> PyResult<rexafs::AbsorptionMode> {
    match value {
        "unknown" => Ok(rexafs::AbsorptionMode::Unknown),
        "transmission" => Ok(rexafs::AbsorptionMode::Transmission),
        "fluorescence" => Ok(rexafs::AbsorptionMode::Fluorescence),
        _ => Err(invalid(
            "mode must be unknown, transmission or fluorescence",
        )),
    }
}

/// Optically thick, homogeneous-sample XANES correction (since 0.2.10).
///
/// FluorescenceCorrection("CuO", "Cu", "K", line="Ka1", angles=(45, 45))
/// requires the complete sample formula, absorber, edge, detected emission and
/// measured incident/exit angles. Angles are degrees FROM THE SAMPLE SURFACE,
/// each in (0, 90]; geometry is never inferred. family=True selects an unresolved
/// within-shell family such as Ka rather than an individual line.
/// Internal conventional normalization runs automatically (degree 1, no Victoreen
/// term). It is separate from final normalization of corrected mu. The output
/// retains original inputs, internal fit, atomic data and numerical diagnostics.
/// This fluo_elam_v1 model is not qualified for EXAFS or finite-thickness samples.
/// See https://xraypy.github.io/xraylarch/xafs_preedge.html#over-absorption-corrections.
#[pyclass(
    name = "FluorescenceCorrection",
    module = "rexafs",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyFluorescenceCorrection {
    pub inner: rexafs::FluorescenceCorrection,
}
#[pymethods]
impl PyFluorescenceCorrection {
    /// Copy settings. e0=None detects the edge. Internal pre_edge/post_edge are
    /// inclusive eV offsets from E0; None uses available low endpoint to -30 eV
    /// and +100 eV to available high endpoint. Complete coverage is required.
    /// degree=1 is the internal post-edge degree (0–5). No final normalization runs.
    #[new]
    #[pyo3(signature=(formula, element, edge, *, line, angles, family=false, e0=None, pre_edge=None, post_edge=None, degree=1))]
    #[allow(clippy::too_many_arguments)] // Explicit scientific options are keyword-only.
    fn new(
        formula: &str,
        element: &str,
        edge: &str,
        line: &str,
        angles: (f64, f64),
        family: bool,
        e0: Option<f64>,
        pre_edge: Option<(f64, f64)>,
        post_edge: Option<(f64, f64)>,
        degree: usize,
    ) -> Self {
        let mut inner = rexafs::FluorescenceCorrection::new(formula, element, edge)
            .angles(angles.0, angles.1)
            .degree(degree);
        inner = if family {
            inner.line_family(line)
        } else {
            inner.line(line)
        };
        inner.e0 = e0;
        inner.pre_edge = pre_edge.map(|(a, b)| [a, b]);
        inner.post_edge = post_edge.map(|(a, b)| [a, b]);
        Self { inner }
    }
    /// Correct original unnormalized fluorescence mu on its measured energy grid.
    /// Copies matching finite 1D arrays, releases the GIL and leaves inputs intact.
    /// Energy is positive, strictly increasing and in eV. This call explicitly
    /// interprets the arrays as fluorescence. Returns original/corrected mu in the
    /// same units. Invalid geometry/composition/coverage, nonpositive fitted edge
    /// step and singular denominators raise ValueError; no clipping is applied.
    /// Corrected-array uncertainty is unavailable. Prefer Spectrum.correct_fluorescence
    /// to retain the XANES-only restriction through further spectrum processing.
    fn apply(
        &self,
        py: Python<'_>,
        energy: &Bound<'_, PyAny>,
        mu: &Bound<'_, PyAny>,
    ) -> PyResult<PyFluorescenceCorrectionResult> {
        let energy = crate::metrics::array(py, energy, "energy")?;
        let mu = crate::metrics::array(py, mu, "mu")?;
        let inner = py
            .detach(|| self.inner.apply(&energy, &mu))
            .map_err(invalid)?;
        Ok(PyFluorescenceCorrectionResult { inner })
    }
    /// Native settings JSON, including any pinned atomic data identity.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner).map_err(invalid)
    }
    /// Restore settings. Calculation checks scientific values and reference availability.
    #[staticmethod]
    fn from_json(json: &str) -> PyResult<Self> {
        Ok(Self {
            inner: serde_json::from_str(json).map_err(invalid)?,
        })
    }
}

/// Immutable historical correction (since 0.2.10). Arrays/dictionaries are independent
/// copies. Original inputs, atomic data and internal normalization remain available
/// after editing or normalizing a corrected Spectrum. This record describes the
/// calculation-time arrays; later spectrum edits never rewrite it. No experimental
/// uncertainty is claimed. Inspect warnings and maximum_amplification before use.
#[pyclass(
    name = "FluorescenceCorrectionResult",
    module = "rexafs",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyFluorescenceCorrectionResult {
    pub inner: rexafs::FluorescenceCorrectionResult,
}
#[pymethods]
impl PyFluorescenceCorrectionResult {
    /// Original measured energy, in eV; no resampling.
    #[getter]
    fn energy<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<f64>> {
        PyArray1::from_vec(py, self.inner.energy.clone())
    }
    /// Original uncorrected absorption, in its supplied units.
    #[getter]
    fn original_mu<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<f64>> {
        PyArray1::from_vec(py, self.inner.original_mu.clone())
    }
    /// Corrected absorption, same grid/units. Final normalization is separate.
    #[getter]
    fn corrected_mu<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<f64>> {
        PyArray1::from_vec(py, self.inner.corrected_mu.clone())
    }
    /// Dimensionless multiplicative alpha/denominator, without clipping.
    #[getter]
    fn factor<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<f64>> {
        PyArray1::from_vec(py, self.inner.factor.clone())
    }
    /// Dimensionless alpha+1-internal_norm, without clipping.
    #[getter]
    fn denominator<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<f64>> {
        PyArray1::from_vec(py, self.inner.denominator.clone())
    }
    /// Named algorithm and normalization convention, fluo_elam_v1.
    #[getter]
    fn method(&self) -> &str {
        &self.inner.method
    }
    /// Input acquisition interpretation. Unknown records an explicit caller assumption.
    #[getter]
    fn input_mode(&self) -> &'static str {
        mode_name(self.inner.input_mode)
    }
    /// Dimensionless attenuation/geometry constant of the named FLUO model.
    #[getter]
    fn alpha(&self) -> f64 {
        self.inner.alpha
    }
    /// sin(incidence)/sin(exit), using surface angles; dimensionless.
    #[getter]
    fn geometry_ratio(&self) -> f64 {
        self.inner.geometry_ratio
    }
    /// Smallest dimensionless denominator on the entire measured grid.
    #[getter]
    fn minimum_denominator(&self) -> f64 {
        self.inner.minimum_denominator
    }
    /// Largest dimensionless factor; a large value indicates noise amplification.
    #[getter]
    fn maximum_amplification(&self) -> f64 {
        self.inner.maximum_amplification
    }
    /// Numerical rejection threshold, 64*machine_epsilon*max(1, alpha+1).
    #[getter]
    fn singularity_threshold(&self) -> f64 {
        self.inner.singularity_threshold
    }
    /// Domain, explicit interpretation and numerical diagnostics, not confidence intervals.
    #[getter]
    fn warnings(&self) -> Vec<String> {
        self.inner.warnings.clone()
    }
    /// Independent settings pinned to resolved internal ranges/E0 and atomic data.
    #[getter]
    fn definition(&self) -> PyFluorescenceCorrection {
        PyFluorescenceCorrection {
            inner: self.inner.definition(),
        }
    }
    /// Internal conventional fit of ORIGINAL mu: resolved E0/ranges, degree, edge_step,
    /// pre_curve, post_curve and norm. Dictionary arrays are independent Python lists.
    /// This fit is distinct from final normalization of the corrected Spectrum.
    #[getter]
    fn internal<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let json = serde_json::to_string(&self.inner.internal).map_err(invalid)?;
        py.import("json")?.getattr("loads")?.call1((json,))
    }
    /// Independent dictionary with edge, emission and compound attenuation records:
    /// energies (eV), mass fractions, cm²/g values, table identities and checksums.
    #[getter]
    fn atomic<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let json = serde_json::json!({"edge":self.inner.edge,"emission":self.inner.emission,"attenuation":self.inner.attenuation});
        py.import("json")?
            .getattr("loads")?
            .call1((json.to_string(),))
    }
    /// Full native historical record, including original arrays and atomic evidence.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner).map_err(invalid)
    }
}
