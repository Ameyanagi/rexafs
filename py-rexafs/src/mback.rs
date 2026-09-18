//! Native MBACK settings/results. Arrays are copied at the Python boundary.
use numpy::PyArray1;
use pyo3::{exceptions::PyValueError, prelude::*};
use rexafs::xafs::mback::{MBack, MbackErfc, MbackResult};
fn invalid(e: impl std::fmt::Display) -> PyErr {
    PyValueError::new_err(e.to_string())
}

/// Optional smooth fluorescence background for MBACK (unreleased).
///
/// The line must originate at the selected absorber edge. width=(low, high)
/// gives positive eV bounds; amplitude=(low, high) gives finite f2-unit bounds.
/// family=False selects one exact line such as Ka1; True selects a within-shell
/// family such as Ka. This is not an over-absorption correction. Settings are copied.
#[pyclass(name = "MbackErfc", module = "rexafs", frozen, skip_from_py_object)]
#[derive(Clone)]
pub struct PyMbackErfc {
    inner: MbackErfc,
}
#[pymethods]
impl PyMbackErfc {
    /// Select an emission and explicit increasing width/amplitude bounds.
    #[new]
    #[pyo3(signature=(line, *, width, amplitude, family=false))]
    fn new(line: &str, width: (f64, f64), amplitude: (f64, f64), family: bool) -> Self {
        let mut inner = MbackErfc::new(line, width.0..=width.1, amplitude.0..=amplitude.1);
        if family {
            inner.emission = rexafs::atomic::EmissionSelection::Family(line.into());
        }
        Self { inner }
    }
}

/// Full Chantler MBACK normalization (unreleased). Example:
/// MBack("Cu", "K", pre_edge=(-200, -50), post_edge=(100, 800)).
///
/// Ranges are eV offsets from E0. Degree defaults to 2; erfc is disabled. E0=None
/// uses the derivative detector. Automatic ranges use the outer 80% of measured
/// pre/post spans with neighboring-edge limits; inspect resolved result ranges.
/// The offline atomic table is loaded automatically and never energy shifted.
/// fit(energy, mu) leaves both inputs/model unchanged and returns owned norm/flat
/// arrays and full diagnostics. set_normalization_method(model) copies settings
/// into a Spectrum; normalize() invalidates its dependent background/FFT results.
/// Invalid ranges, unsupported data, nonidentifiability and nonpositive scale/step
/// raise ValueError. No experimental uncertainty is inferred from fit weights.
#[pyclass(name = "MBack", module = "rexafs", frozen, skip_from_py_object)]
#[derive(Clone)]
pub struct PyMBack {
    pub inner: MBack,
}
#[pymethods]
impl PyMBack {
    /// Create immutable settings. Optional erfc requires an explicit MbackErfc object.
    #[new]
    #[pyo3(signature=(element, edge, *, e0=None, pre_edge=None, post_edge=None, degree=2, erfc=None))]
    #[allow(clippy::too_many_arguments)] // Named scientific settings at the binding boundary.
    fn new(
        element: &str,
        edge: &str,
        e0: Option<f64>,
        pre_edge: Option<(f64, f64)>,
        post_edge: Option<(f64, f64)>,
        degree: usize,
        erfc: Option<&PyMbackErfc>,
    ) -> Self {
        let mut inner = MBack::for_edge(element, edge).degree(degree);
        inner.e0 = e0;
        if let Some((a, b)) = pre_edge {
            inner = inner.pre_edge(a..=b);
        }
        if let Some((a, b)) = post_edge {
            inner = inner.post_edge(a..=b);
        }
        if let Some(term) = erfc {
            inner = inner.erfc(term.inner.clone());
        }
        Self { inner }
    }
    /// Fit matching finite 1D arrays (energy eV, raw absorption). Copies input
    /// buffers, releases the GIL during Rust computation, and leaves inputs intact.
    /// Returns separate dimensionless norm/flat and matched fpp in electron units.
    fn fit(
        &self,
        py: Python<'_>,
        energy: &Bound<'_, PyAny>,
        mu: &Bound<'_, PyAny>,
    ) -> PyResult<PyMbackResult> {
        let energy = crate::metrics::array(py, energy, "energy")?;
        let mu = crate::metrics::array(py, mu, "mu")?;
        let inner = py
            .detach(|| self.inner.fit(&energy, &mu))
            .map_err(invalid)?;
        Ok(PyMbackResult { inner })
    }
    /// Serialize settings in the versioned native schema. No input arrays are added.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner).map_err(invalid)
    }
    /// Restore native settings; fit validates scientific values and reference identity.
    #[staticmethod]
    fn from_json(json: &str) -> PyResult<Self> {
        Ok(Self {
            inner: serde_json::from_str(json).map_err(invalid)?,
        })
    }
}

/// Owned full-MBACK output (unreleased). Arrays are returned as independent copies.
///
/// norm=(scale*mu-pre_curve)/Delta is dimensionless; fpp=scale*mu-background
/// remains in f2 units. flat separately removes the auxiliary post-edge trend.
/// objective uses balanced pre/post sample counts, not inverse measurement
/// variances. Inspect warnings/condition and resolved ranges. No covariance or
/// experimental confidence interval is implied. to_json retains all provenance.
#[pyclass(name = "MbackResult", module = "rexafs", frozen, skip_from_py_object)]
#[derive(Clone)]
pub struct PyMbackResult {
    pub inner: MbackResult,
}
#[pymethods]
impl PyMbackResult {
    /// Complete result JSON, including reference identity, settings, weights and curves.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner).map_err(invalid)
    }
    /// Immutable replay definition, pinned to the original table. Automatic E0/ranges
    /// remain automatic where originally requested; explicit settings remain explicit.
    #[getter]
    fn definition(&self) -> PyMBack {
        let mut options = self.inner.requested.clone();
        options.reference = Some(self.inner.reference.clone());
        PyMBack {
            inner: MBack {
                e0: self.inner.requested_e0,
                options,
                ..MBack::new()
            },
        }
    }
    /// Independent dictionary with provider, data checksum and table identity.
    #[getter]
    fn reference<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let json = serde_json::to_string(&self.inner.reference).map_err(invalid)?;
        py.import("json")?.getattr("loads")?.call1((json,))
    }
    /// Table/provider identity JSON, including actual dataset checksum.
    #[getter]
    fn reference_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner.reference).map_err(invalid)
    }
    /// Resolved inclusive pre-edge offsets in eV from E0.
    #[getter]
    fn pre_edge(&self) -> (f64, f64) {
        (self.inner.pre_edge[0], self.inner.pre_edge[1])
    }
    /// Resolved inclusive post-edge offsets in eV from E0.
    #[getter]
    fn post_edge(&self) -> (f64, f64) {
        (self.inner.post_edge[0], self.inner.post_edge[1])
    }
    /// Original zero-based indices included in the objective.
    #[getter]
    fn fit_indices(&self) -> Vec<usize> {
        self.inner.fit_indices.clone()
    }
    /// Nonfatal boundary/conditioning diagnostics. Empty does not establish physical validity.
    #[getter]
    fn warnings(&self) -> Vec<String> {
        self.inner.warnings.clone()
    }
    /// Positive erfc width in eV, or None when disabled.
    #[getter]
    fn erfc_width(&self) -> Option<f64> {
        self.inner.erfc_width
    }
    /// Resolved fixed energy origin in eV.
    #[getter]
    fn e0(&self) -> f64 {
        self.inner.e0
    }
    /// Positive fitted absorption step in input mu units.
    #[getter]
    fn edge_step(&self) -> f64 {
        self.inner.edge_step
    }
    /// Positive conversion from input absorption to f2.
    #[getter]
    fn scale(&self) -> f64 {
        self.inner.scale
    }
    /// Sum of squared region-balanced residuals, in squared f2 units.
    #[getter]
    fn objective(&self) -> f64 {
        self.inner.objective
    }
    /// Condition number of the column-scaled final Jacobian.
    #[getter]
    fn condition(&self) -> f64 {
        self.inner.condition
    }
    /// Rank of the final Jacobian, including erfc width when enabled.
    #[getter]
    fn rank(&self) -> usize {
        self.inner.rank
    }
    /// Number of linear solves used by the fit.
    #[getter]
    fn evaluations(&self) -> usize {
        self.inner.evaluations
    }
    /// Fitted erfc amplitude in f2 units; zero when disabled.
    #[getter]
    fn erfc_amplitude(&self) -> f64 {
        self.inner.erfc_amplitude
    }
    /// Polynomial coordinate scale in eV.
    #[getter]
    fn energy_scale(&self) -> f64 {
        self.inner.energy_scale
    }
    /// Original energy grid in eV. Returns a copy.
    #[getter]
    fn energy<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<f64>> {
        PyArray1::from_vec(py, self.inner.energy.clone())
    }
    /// Unshifted atomic scattering factor, in electron units. Returns a copy.
    #[getter]
    fn f2<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<f64>> {
        PyArray1::from_vec(py, self.inner.f2.clone())
    }
    /// Matched scale*mu-background, in electron units; distinct from norm. Returns a copy.
    #[getter]
    fn fpp<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<f64>> {
        PyArray1::from_vec(py, self.inner.fpp.clone())
    }
    /// Dimensionless normalized absorption. Returns a copy.
    #[getter]
    fn norm<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<f64>> {
        PyArray1::from_vec(py, self.inner.norm.clone())
    }
    /// Dimensionless flattened absorption using the auxiliary post-edge trend. Returns a copy.
    #[getter]
    fn flat<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<f64>> {
        PyArray1::from_vec(py, self.inner.flat.clone())
    }
    /// Fitted smooth background in f2 units. Returns a copy.
    #[getter]
    fn background<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<f64>> {
        PyArray1::from_vec(py, self.inner.background.clone())
    }
    /// Auxiliary pre-edge line on f2+background, in f2 units. Returns a copy.
    #[getter]
    fn pre_curve<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<f64>> {
        PyArray1::from_vec(py, self.inner.pre_curve.clone())
    }
    /// Auxiliary post-edge curve on f2+background, in f2 units. Returns a copy.
    #[getter]
    fn post_curve<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<f64>> {
        PyArray1::from_vec(py, self.inner.post_curve.clone())
    }
    /// Unweighted f2+background-scale*mu on every energy point. Returns a copy.
    #[getter]
    fn residual<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<f64>> {
        PyArray1::from_vec(py, self.inner.residual.clone())
    }
    /// Increasing polynomial powers of (energy-e0)/energy_scale, in f2 units. Returns a copy.
    #[getter]
    fn coefficients<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<f64>> {
        PyArray1::from_vec(py, self.inner.coefficients.clone())
    }
    /// 1/sqrt(region sample count), in fit_indices order. Returns a copy.
    #[getter]
    fn weights<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<f64>> {
        PyArray1::from_vec(py, self.inner.weights.clone())
    }
}
