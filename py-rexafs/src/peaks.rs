//! Owned Python peak definitions and results; numerical behavior is native Rust.
use numpy::PyArray1;
use pyo3::{exceptions::PyValueError, prelude::*};
use rexafs::prelude::{PeakContribution, PeakFit, PeakFitResult, PeakRole};
use std::collections::BTreeMap;

fn invalid(error: impl std::fmt::Display) -> PyErr {
    PyValueError::new_err(error.to_string())
}

/// Immutable composite XANES peak definition (unreleased).
///
/// PeakFit((-20, 40)).gaussian("p1", 5, 2, 3).linear_baseline(0, 0)
/// starts with normalized mu and E0-relative eV. Arguments are center, whole-axis
/// area (signal units times eV), and FWHM (eV). Each builder returns a NEW model.
/// Fitting prepares missing normalization on a copy and never changes the input.
/// Default bounds keep centers inside the range, areas nonnegative and widths
/// positive. Use flat(), raw_mu(), absolute() or reference(eV) explicitly.
/// No smoothing, component-count selection or chemical assignment is performed.
/// Inspect termination and warnings; covariance is conditional on this model.
#[pyclass(name = "PeakFit", module = "rexafs", frozen, skip_from_py_object)]
#[derive(Clone)]
pub struct PyPeakFit {
    pub inner: PeakFit,
}
#[pymethods]
impl PyPeakFit {
    /// Create an empty model over inclusive E0-relative eV; add components before fitting.
    #[new]
    fn new(range: (f64, f64)) -> Self {
        Self {
            inner: PeakFit::new(range.0..=range.1),
        }
    }
    /// Use dimensionless flattened mu; prerequisites run on a copy. Returns a new model.
    fn flat(&self) -> Self {
        Self {
            inner: self.inner.clone().flat(),
        }
    }
    /// Use the original mapped absorption signal and its units. Returns a new model.
    fn raw_mu(&self) -> Self {
        Self {
            inner: self.inner.clone().raw_mu(),
        }
    }
    /// Interpret ranges, centers and baseline references as absolute eV. Returns a new model.
    fn absolute(&self) -> Self {
        Self {
            inner: self.inner.clone().absolute(),
        }
    }
    /// Use offsets from this fixed reference energy in eV. Returns a new model.
    fn reference(&self, energy_ev: f64) -> Self {
        Self {
            inner: self.inner.clone().reference(energy_ev),
        }
    }
    /// Add a Gaussian: center/FWHM in eV, whole-axis area in signal units times eV. Returns a new model.
    fn gaussian(&self, name: &str, center: f64, area: f64, fwhm: f64) -> Self {
        Self {
            inner: self.inner.clone().gaussian(name, center, area, fwhm),
        }
    }
    /// Add a Lorentzian with whole-axis area and FWHM in eV. Returns a new model.
    fn lorentzian(&self, name: &str, center: f64, area: f64, fwhm: f64) -> Self {
        Self {
            inner: self.inner.clone().lorentzian(name, center, area, fwhm),
        }
    }
    /// Add a common-FWHM mixture; fraction is the Lorentzian share from zero to one. Returns a new model.
    fn pseudo_voigt(&self, name: &str, center: f64, area: f64, fwhm: f64, fraction: f64) -> Self {
        Self {
            inner: self
                .inner
                .clone()
                .pseudo_voigt(name, center, area, fwhm, fraction),
        }
    }
    /// Add a true Voigt with independent Gaussian/Lorentzian FWHM in eV. Returns a new model.
    fn voigt(
        &self,
        name: &str,
        center: f64,
        area: f64,
        gaussian_fwhm: f64,
        lorentzian_fwhm: f64,
    ) -> Self {
        Self {
            inner: self
                .inner
                .clone()
                .voigt(name, center, area, gaussian_fwhm, lorentzian_fwhm),
        }
    }
    /// Add height*(1+erf((E-center)/scale))/2; scale is positive eV. Returns a new model.
    fn erf_step(&self, name: &str, center: f64, height: f64, scale: f64) -> Self {
        Self {
            inner: self.inner.clone().erf_step(name, center, height, scale),
        }
    }
    /// Add height*(1/2+atan((E-center)/scale)/pi); scale is positive eV. Returns a new model.
    fn arctan_step(&self, name: &str, center: f64, height: f64, scale: f64) -> Self {
        Self {
            inner: self.inner.clone().arctan_step(name, center, height, scale),
        }
    }
    /// Add a fitted constant named baseline, in signal units. Returns a new model.
    #[pyo3(signature=(offset=0.0))]
    fn constant_baseline(&self, offset: f64) -> Self {
        Self {
            inner: self.inner.clone().constant_baseline(offset),
        }
    }
    /// Add baseline = offset+slope*E_offset; slope is signal units/eV. Returns a new model.
    #[pyo3(signature=(offset=0.0, slope=0.0))]
    fn linear_baseline(&self, offset: f64, slope: f64) -> Self {
        Self {
            inner: self.inner.clone().linear_baseline(offset, slope),
        }
    }
    /// Exclude an inclusive interval in the model coordinates; masked gaps are not integrated.
    fn exclude(&self, range: (f64, f64)) -> Self {
        Self {
            inner: self.inner.clone().exclude(range.0..=range.1),
        }
    }
    /// Replace an existing parameter (for example p1_width), returning a new model.
    /// Bounds default to unbounded; specify bounds explicitly to retain restrictions.
    /// A restricted expression ties this parameter to others and overrides vary.
    /// Names and expression dependencies are validated when evaluating or fitting.
    #[pyo3(signature=(name, value, *, vary=true, bounds=(None,None), expression=None))]
    fn parameter(
        &self,
        name: &str,
        value: f64,
        vary: bool,
        bounds: (Option<f64>, Option<f64>),
        expression: Option<&str>,
    ) -> PyResult<Self> {
        if !self.inner.parameters.vars.contains_key(name) {
            return Err(invalid(format!("Unknown peak parameter: {name}")));
        }
        let mut next = self.clone();
        let mut variable =
            rexafs::prelude::FitVariable::new(value, vary).with_bounds(bounds.0, bounds.1);
        if let Some(expression) = expression {
            variable = variable.with_expr(expression);
        }
        next.inner.parameters.insert(name, variable);
        Ok(next)
    }
    /// Treat a named peak shape as baseline, excluding it from the area-weighted center.
    /// This changes the role only; steps must remain edges. Returns a new model.
    fn as_baseline(&self, name: &str) -> PyResult<Self> {
        let mut next = self.clone();
        let c = next
            .inner
            .components
            .iter_mut()
            .find(|c| c.name == name)
            .ok_or_else(|| invalid("Unknown component"))?;
        if c.role == PeakRole::Edge {
            return Err(invalid("An edge step cannot be a baseline"));
        }
        c.role = PeakRole::Baseline;
        Ok(next)
    }
    /// Set positive iteration/tolerance limits on a new model. Defaults are 200 and 1e-10.
    #[pyo3(signature=(*, max_iterations=200, tolerance=1e-10))]
    fn solver(&self, max_iterations: usize, tolerance: f64) -> Self {
        let mut next = self.clone();
        next.inner.max_iterations = max_iterations;
        next.inner.tolerance = tolerance;
        next
    }
    /// Evaluate without fitting or masking. Energy is absolute eV; supply e0 for relative models.
    /// Returns a new NumPy float64 array; invalid definitions/energies raise ValueError.
    #[pyo3(signature=(energy, *, e0=None))]
    fn evaluate<'py>(
        &self,
        py: Python<'py>,
        energy: &Bound<'py, PyAny>,
        e0: Option<f64>,
    ) -> PyResult<Bound<'py, PyArray1<f64>>> {
        let energy = crate::metrics::array(py, energy, "energy")?;
        let values = py
            .detach(|| self.inner.evaluate(&energy, e0))
            .map_err(invalid)?;
        Ok(PyArray1::from_vec(py, values))
    }
    /// Return baseline-only initialization outside peak intervals; final masks are unchanged.
    /// Ranges use model coordinates. The source and this starting model stay unchanged.
    fn initialize_baseline(
        &self,
        py: Python<'_>,
        spectrum: &crate::PySpectrum,
        peak_intervals: Vec<(f64, f64)>,
    ) -> PyResult<Self> {
        let ranges = peak_intervals
            .into_iter()
            .map(|(a, b)| a..=b)
            .collect::<Vec<_>>();
        Ok(Self {
            inner: py
                .detach(|| self.inner.initialize_baseline(&spectrum.inner, &ranges))
                .map_err(invalid)?,
        })
    }
    /// Fit every spectrum independently from this starting model, preserving input order.
    /// Returns one outcome per input, with either result or error. A bad frame never removes
    /// a row or stops later fits. Inputs and settings are unchanged; Rust releases the GIL.
    /// Errors are unweighted; for supplied point errors call spectrum.fit_peaks separately.
    fn fit_batch(
        &self,
        py: Python<'_>,
        spectra: Vec<PyRef<'_, crate::PySpectrum>>,
    ) -> Vec<PyPeakOutcome> {
        let spectra = spectra.iter().map(|s| &s.inner).collect::<Vec<_>>();
        py.detach(|| {
            spectra
                .iter()
                .enumerate()
                .map(|(index, spectrum)| match self.inner.fit(spectrum) {
                    Ok(inner) => PyPeakOutcome {
                        index,
                        result: Some(PyPeakFitResult { inner }),
                        error: None,
                    },
                    Err(e) => PyPeakOutcome {
                        index,
                        result: None,
                        error: Some(e.to_string()),
                    },
                })
                .collect()
        })
    }
    /// Serialize all initial settings, components, masks and constraints for reproducibility.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner).map_err(invalid)
    }
    /// Restore and validate a complete saved definition; no expressions execute external code.
    #[staticmethod]
    fn from_json(json: &str) -> PyResult<Self> {
        let inner: PeakFit = serde_json::from_str(json).map_err(invalid)?;
        inner.validate().map_err(invalid)?;
        Ok(Self { inner })
    }
}

/// One batch outcome in input order (unreleased); exactly one of result/error is present.
#[pyclass(
    name = "PeakFitOutcome",
    module = "rexafs",
    frozen,
    skip_from_py_object
)]
pub struct PyPeakOutcome {
    /// Zero-based input index, retained even on failure.
    #[pyo3(get)]
    pub index: usize,
    /// Owned successful numerical result; inspect its termination and warnings.
    #[pyo3(get)]
    pub result: Option<PyPeakFitResult>,
    /// Failure reason, or None. A numerical nonconvergence remains a result.
    #[pyo3(get)]
    pub error: Option<String>,
}

/// Owned peak result (unreleased). Array getters return copies, so editing them cannot
/// alter the fit. Energy/centers are absolute eV; model parameter values retain the
/// selected coordinate origin. Inspect termination and uncertainty_unavailable;
/// local standard errors are conditional, not model-selection confidence intervals.
#[pyclass(name = "PeakFitResult", module = "rexafs", frozen, skip_from_py_object)]
#[derive(Clone)]
pub struct PyPeakFitResult {
    pub inner: PeakFitResult,
}
#[pymethods]
impl PyPeakFitResult {
    /// Initial definition, returned as an independent immutable model.
    #[getter]
    fn definition(&self) -> PyPeakFit {
        PyPeakFit {
            inner: self.inner.definition.clone(),
        }
    }
    /// Copy the fitted values into a reusable model; the initial definition stays unchanged.
    fn fitted_model(&self) -> PyPeakFit {
        PyPeakFit {
            inner: self.inner.fitted_model(),
        }
    }
    /// Named final values; centers/reference values use the model's chosen coordinates.
    #[getter]
    fn parameters(&self) -> BTreeMap<String, f64> {
        self.inner
            .parameters
            .vars
            .iter()
            .map(|(k, v)| (k.clone(), v.value))
            .collect()
    }
    /// Named conditional local standard errors; None means unavailable or not independently estimated.
    #[getter]
    fn parameter_errors(&self) -> BTreeMap<String, Option<f64>> {
        self.inner
            .parameters
            .vars
            .iter()
            .map(|(k, v)| (k.clone(), v.stderr))
            .collect()
    }
    /// Component curves and derived summaries, in model order; copies are independent.
    #[getter]
    fn components(&self) -> Vec<PyPeakContribution> {
        self.inner
            .components
            .iter()
            .cloned()
            .map(|inner| PyPeakContribution { inner })
            .collect()
    }
    /// Full Rust result including initial/final constraints, masks and uncertainty diagnostics.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner).map_err(invalid)
    }
    /// Resolved energy origin in eV, added to parameter centers/reference energies.
    #[getter]
    fn origin_ev(&self) -> f64 {
        self.inner.origin_ev
    }
    /// Absolute energy in eV, only the native points used by this fit.
    #[getter]
    fn energy<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<f64>> {
        PyArray1::from_vec(py, self.inner.energy.clone())
    }
    /// Original zero-based point indices; preserves masks and sampling provenance.
    #[getter]
    fn source_indices(&self) -> Vec<usize> {
        self.inner.source_indices.clone()
    }
    /// Selected representation's measured values, in its signal units.
    #[getter]
    fn data<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<f64>> {
        PyArray1::from_vec(py, self.inner.data.clone())
    }
    /// Joint baseline + peaks + steps, in the same signal units.
    #[getter]
    fn model<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<f64>> {
        PyArray1::from_vec(py, self.inner.model.clone())
    }
    /// Unweighted data minus model, in signal units (also for weighted fits).
    #[getter]
    fn residual<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<f64>> {
        PyArray1::from_vec(py, self.inner.residual.clone())
    }
    /// Supplied selected-space standard deviations on the fitted points, if any.
    #[getter]
    fn standard_deviation(&self) -> Option<Vec<f64>> {
        self.inner.standard_deviation.clone()
    }
    /// Sum of squared residuals, divided by supplied standard deviations if present.
    #[getter]
    fn objective(&self) -> f64 {
        self.inner.objective
    }
    /// Number of fitted native data points (not EXAFS independent-point estimates).
    #[getter]
    fn points(&self) -> usize {
        self.inner.points
    }
    /// Number of independent varying parameters; expression ties are excluded.
    #[getter]
    fn free_parameters(&self) -> usize {
        self.inner.free_parameters
    }
    /// points − free_parameters. Fits with fewer points than variables are rejected.
    #[getter]
    fn degrees_of_freedom(&self) -> usize {
        self.inner.degrees_of_freedom
    }
    /// Weighted numerical Jacobian rank under a 1e-10 relative singular-value cutoff.
    #[getter]
    fn jacobian_rank(&self) -> usize {
        self.inner.jacobian_rank
    }
    /// Sorted independent parameter names defining covariance/correlation axes.
    #[getter]
    fn covariance_names(&self) -> Vec<String> {
        self.inner.covariance_names.clone()
    }
    /// Local covariance; absolute-error scaling when standard deviations were given,
    /// otherwise multiplied by objective/degrees_of_freedom.
    #[getter]
    fn covariance(&self) -> Option<Vec<Vec<f64>>> {
        self.inner.covariance.clone()
    }
    /// Dimensionless correlations corresponding to covariance_names. Absent when
    /// any conditional variance is zero; the warning explains that case.
    #[getter]
    fn correlation(&self) -> Option<Vec<Vec<f64>>> {
        self.inner.correlation.clone()
    }
    /// Why covariance/standard errors were withheld, rather than replaced by zero.
    #[getter]
    fn uncertainty_unavailable(&self) -> Option<String> {
        self.inner.uncertainty_unavailable.clone()
    }
    /// Model peak-area-weighted center in absolute eV, excluding baseline/steps.
    #[getter]
    fn peak_center_ev(&self) -> Option<f64> {
        self.inner.peak_center_ev
    }
    /// Conditional error in that center, using full parameter covariance.
    #[getter]
    fn peak_center_standard_error_ev(&self) -> Option<f64> {
        self.inner.peak_center_standard_error_ev
    }
    /// Explicit numerical termination category.
    #[getter]
    fn termination(&self) -> String {
        format!("{:?}", self.inner.termination)
    }
    /// Solver-specific termination detail, retained verbatim for diagnosis.
    #[getter]
    fn termination_detail(&self) -> String {
        self.inner.termination_detail.clone()
    }
    /// Number of residual-vector evaluations during optimization, including numerical derivatives.
    #[getter]
    fn evaluations(&self) -> usize {
        self.inner.evaluations
    }
    /// Active bounds and other model/uncertainty limitations.
    #[getter]
    fn warnings(&self) -> Vec<String> {
        self.inner.warnings.clone()
    }
}

/// One peak/step/baseline contribution with an independent NumPy curve copy (unreleased).
/// Centers and FWHM use absolute eV and eV respectively; areas use signal units times eV.
#[pyclass(
    name = "PeakContribution",
    module = "rexafs",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyPeakContribution {
    inner: PeakContribution,
}
#[pymethods]
impl PyPeakContribution {
    /// Stable component identity from the initial definition.
    #[getter]
    fn name(&self) -> String {
        self.inner.name.clone()
    }
    /// Scientific role, independent of mathematical shape.
    #[getter]
    fn role(&self) -> String {
        format!("{:?}", self.inner.role)
    }
    /// Mathematical shape used for evaluation.
    #[getter]
    fn shape(&self) -> String {
        format!("{:?}", self.inner.shape)
    }
    /// Component values at the result's absolute-energy points, in signal units.
    #[getter]
    fn curve<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<f64>> {
        PyArray1::from_vec(py, self.inner.curve.clone())
    }
    /// Peak/step center in absolute eV; None for polynomial baselines.
    #[getter]
    fn center_ev(&self) -> Option<f64> {
        self.inner.center_ev
    }
    /// Whole-axis model area in signal units × eV; None for steps/polynomials.
    #[getter]
    fn area(&self) -> Option<f64> {
        self.inner.area
    }
    /// Peak contribution at its center, excluding all other components.
    #[getter]
    fn height(&self) -> Option<f64> {
        self.inner.height
    }
    /// Peak FWHM in eV; true Voigt uses a numerical half-height root.
    #[getter]
    fn fwhm_ev(&self) -> Option<f64> {
        self.inner.fwhm_ev
    }
    /// Conditional errors propagated with the full joint covariance; absent when
    /// local uncertainty is unavailable or the quantity does not apply.
    #[getter]
    fn center_standard_error_ev(&self) -> Option<f64> {
        self.inner.center_standard_error_ev
    }
    /// Conditional whole-axis area error, in signal units × eV.
    #[getter]
    fn area_standard_error(&self) -> Option<f64> {
        self.inner.area_standard_error
    }
    /// Conditional peak-height error, in signal units.
    #[getter]
    fn height_standard_error(&self) -> Option<f64> {
        self.inner.height_standard_error
    }
    /// Conditional FWHM error, in eV, including both true-Voigt width parameters.
    #[getter]
    fn fwhm_standard_error_ev(&self) -> Option<f64> {
        self.inner.fwhm_standard_error_ev
    }
    /// Trapezoidal component integral over included native-grid segments only.
    /// Masked gaps are not bridged; this is not its whole-axis analytic area.
    #[getter]
    fn sampled_integral(&self) -> f64 {
        self.inner.sampled_integral
    }
}
