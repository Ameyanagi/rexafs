//! Thin Python bindings: all stage execution and defaults live in rexafs.
use numpy::{PyArray1, PyReadonlyArray1};
use pyo3::prelude::*;

fn error(error: rexafs::Error) -> PyErr {
    match error {
        rexafs::Error::Data(_) | rexafs::Error::Normalization(_) => {
            pyo3::exceptions::PyValueError::new_err(error.to_string())
        }
        _ => pyo3::exceptions::PyRuntimeError::new_err(error.to_string()),
    }
}
fn arrays(
    py: Python<'_>,
    energy: &Bound<'_, PyAny>,
    mu: &Bound<'_, PyAny>,
) -> PyResult<rexafs::Spectrum> {
    let asarray = py.import("numpy")?.getattr("asarray")?;
    let energy = asarray.call1((energy, "float64"))?;
    let mu = asarray.call1((mu, "float64"))?;
    let energy = energy
        .extract::<PyReadonlyArray1<'_, f64>>()
        .map_err(|_| pyo3::exceptions::PyValueError::new_err("energy must be one-dimensional"))?;
    let mu = mu
        .extract::<PyReadonlyArray1<'_, f64>>()
        .map_err(|_| pyo3::exceptions::PyValueError::new_err("mu must be one-dimensional"))?;
    let energy = energy.as_array().iter().copied().collect::<Vec<_>>();
    let mu = mu.as_array().iter().copied().collect::<Vec<_>>();
    rexafs::Spectrum::from_arrays(&energy, &mu).map_err(error)
}

/// Pre/post-edge normalization settings. Defaults adapt to the measured energy range.
#[pyclass(name = "PrePostEdge", module = "rexafs", skip_from_py_object)]
#[derive(Clone)]
struct PyPrePostEdge {
    inner: rexafs::PrePostEdge,
}
#[pymethods]
impl PyPrePostEdge {
    /// Create settings with Rust defaults; override only the keyword arguments you need.
    #[new]
    #[pyo3(
        text_signature = "(*, pre_edge_start=None, pre_edge_end=None, norm_start=None, norm_end=None, norm_polyorder=None, n_victoreen=None, e0=None, edge_step=None)"
    )]
    #[pyo3(signature = (*, pre_edge_start=None, pre_edge_end=None, norm_start=None, norm_end=None, norm_polyorder=None, n_victoreen=None, e0=None, edge_step=None))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        pre_edge_start: Option<f64>,
        pre_edge_end: Option<f64>,
        norm_start: Option<f64>,
        norm_end: Option<f64>,
        norm_polyorder: Option<i32>,
        n_victoreen: Option<i32>,
        e0: Option<f64>,
        edge_step: Option<f64>,
    ) -> PyResult<Self> {
        let mut result = Self {
            inner: rexafs::PrePostEdge::new(),
        };
        result.set_pre_edge_start(pre_edge_start);
        result.set_pre_edge_end(pre_edge_end);
        result.set_norm_start(norm_start);
        result.set_norm_end(norm_end);
        result.set_norm_polyorder(norm_polyorder);
        result.set_n_victoreen(n_victoreen);
        result.set_e0(e0);
        result.set_edge_step(edge_step);
        Ok(result)
    }
    /// Pre-edge fit start relative to E0, in eV. Default: infer from the measured range.
    #[getter]
    fn pre_edge_start(&self) -> Option<f64> {
        self.inner.pre_edge_start
    }
    #[setter]
    fn set_pre_edge_start(&mut self, value: Option<f64>) {
        self.inner.pre_edge_start = value;
    }
    /// Pre-edge fit end relative to E0, in eV. Default: infer from the pre-edge start.
    #[getter]
    fn pre_edge_end(&self) -> Option<f64> {
        self.inner.pre_edge_end
    }
    #[setter]
    fn set_pre_edge_end(&mut self, value: Option<f64>) {
        self.inner.pre_edge_end = value;
    }
    /// Post-edge fit start relative to E0, in eV. Default: infer from the available range (at most 25 eV).
    #[getter]
    fn norm_start(&self) -> Option<f64> {
        self.inner.norm_start
    }
    #[setter]
    fn set_norm_start(&mut self, value: Option<f64>) {
        self.inner.norm_start = value;
    }
    /// Post-edge fit end relative to E0, in eV. Default: measured upper energy limit.
    #[getter]
    fn norm_end(&self) -> Option<f64> {
        self.inner.norm_end
    }
    #[setter]
    fn set_norm_end(&mut self, value: Option<f64>) {
        self.inner.norm_end = value;
    }
    /// Post-edge polynomial degree, 0 through 5. Default: 0, 1 or 2 for fit spans below 50, below 350, or at least 350 eV.
    #[getter]
    fn norm_polyorder(&self) -> Option<i32> {
        self.inner.norm_polyorder
    }
    #[setter]
    fn set_norm_polyorder(&mut self, value: Option<i32>) {
        self.inner.norm_polyorder = value;
    }
    /// Victoreen energy exponent for the pre-edge fit. Default: 0.
    #[getter]
    fn n_victoreen(&self) -> Option<i32> {
        self.inner.n_victoreen
    }
    #[setter]
    fn set_n_victoreen(&mut self, value: Option<i32>) {
        self.inner.n_victoreen = value;
    }
    /// Edge energy in eV. Default: detect from the spectrum.
    #[getter]
    fn e0(&self) -> Option<f64> {
        self.inner.e0
    }
    #[setter]
    fn set_e0(&mut self, value: Option<f64>) {
        self.inner.e0 = value;
    }
    /// Absorption edge-step override in mu units. Default: estimate from the fitted baselines.
    #[getter]
    fn edge_step(&self) -> Option<f64> {
        self.inner.edge_step
    }
    #[setter]
    fn set_edge_step(&mut self, value: Option<f64>) {
        self.inner.edge_step = value;
    }
}

/// AUTOBK background settings. Recommended defaults use LinearDirect and FixedPenalty with lambda 0.001.
#[pyclass(name = "AUTOBK", module = "rexafs", skip_from_py_object)]
#[derive(Clone)]
struct PyAUTOBK {
    inner: rexafs::AUTOBK,
}
#[pymethods]
impl PyAUTOBK {
    /// Create settings with Rust defaults; override only the keyword arguments you need.
    #[new]
    #[pyo3(
        text_signature = "(*, ek0=None, rbkg=1.0, nknots=None, kmin=0.0, kmax=None, kstep=0.05, nclamp=3, clamp_lo=0, clamp_hi=1, clamp_lambda=0.001, nfft=2048, kweight=1, dk=0.1, linear_regularization=0.0001, linear_condition_limit=100000000.0, linear_residual_ratio_limit=1.05, linear_fallback_to_lm=True, linear_workspace_cache=True, window='Hanning', solver='LinearDirect', linear_fallback_solver='TrustRegionDogLeg', clamp_scale_policy='FixedPenalty')"
    )]
    #[pyo3(signature = (*, ek0=None, rbkg=Some(1.0), nknots=None, kmin=Some(0.0), kmax=None, kstep=Some(0.05), nclamp=Some(3), clamp_lo=Some(0), clamp_hi=Some(1), clamp_lambda=Some(0.001), nfft=Some(2048), kweight=Some(1), dk=Some(0.1), linear_regularization=Some(0.0001), linear_condition_limit=Some(100000000.0), linear_residual_ratio_limit=Some(1.05), linear_fallback_to_lm=Some(true), linear_workspace_cache=Some(true), window=Some("Hanning"), solver=Some("LinearDirect"), linear_fallback_solver=Some("TrustRegionDogLeg"), clamp_scale_policy=Some("FixedPenalty")))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        ek0: Option<f64>,
        rbkg: Option<f64>,
        nknots: Option<i32>,
        kmin: Option<f64>,
        kmax: Option<f64>,
        kstep: Option<f64>,
        nclamp: Option<i32>,
        clamp_lo: Option<i32>,
        clamp_hi: Option<i32>,
        clamp_lambda: Option<f64>,
        nfft: Option<i32>,
        kweight: Option<i32>,
        dk: Option<f64>,
        linear_regularization: Option<f64>,
        linear_condition_limit: Option<f64>,
        linear_residual_ratio_limit: Option<f64>,
        linear_fallback_to_lm: Option<bool>,
        linear_workspace_cache: Option<bool>,
        window: Option<&str>,
        solver: Option<&str>,
        linear_fallback_solver: Option<&str>,
        clamp_scale_policy: Option<&str>,
    ) -> PyResult<Self> {
        let mut result = Self {
            inner: rexafs::AUTOBK::new(),
        };
        result.set_ek0(ek0);
        result.set_rbkg(rbkg);
        result.set_nknots(nknots);
        result.set_kmin(kmin);
        result.set_kmax(kmax);
        result.set_kstep(kstep);
        result.set_nclamp(nclamp);
        result.set_clamp_lo(clamp_lo);
        result.set_clamp_hi(clamp_hi);
        result.set_clamp_lambda(clamp_lambda);
        result.set_nfft(nfft);
        result.set_kweight(kweight);
        result.set_dk(dk);
        result.set_linear_regularization(linear_regularization);
        result.set_linear_condition_limit(linear_condition_limit);
        result.set_linear_residual_ratio_limit(linear_residual_ratio_limit);
        result.set_linear_fallback_to_lm(linear_fallback_to_lm);
        result.set_linear_workspace_cache(linear_workspace_cache);
        result.set_window(window.map(str::to_owned))?;
        result.set_solver(solver.map(str::to_owned))?;
        result.set_linear_fallback_solver(linear_fallback_solver.map(str::to_owned))?;
        result.set_clamp_scale_policy(clamp_scale_policy.map(str::to_owned))?;
        Ok(result)
    }
    /// Edge energy in eV. Default: use normalization E0.
    #[getter]
    fn ek0(&self) -> Option<f64> {
        self.inner.ek0
    }
    #[setter]
    fn set_ek0(&mut self, value: Option<f64>) {
        self.inner.ek0 = value;
    }
    /// Background cutoff in angstroms. AUTOBK suppresses Fourier residuals below this R. Default: 1.0; increasing it can remove structural signal.
    #[getter]
    fn rbkg(&self) -> Option<f64> {
        self.inner.rbkg
    }
    #[setter]
    fn set_rbkg(&mut self, value: Option<f64>) {
        self.inner.rbkg = value;
    }
    /// Spline knot count. Default: determine from rbkg and the k range.
    #[getter]
    fn nknots(&self) -> Option<i32> {
        self.inner.nknots
    }
    #[setter]
    fn set_nknots(&mut self, value: Option<i32>) {
        self.inner.nknots = value;
    }
    /// Background fit lower k limit in inverse angstroms. Default: 0.0.
    #[getter]
    fn kmin(&self) -> Option<f64> {
        self.inner.kmin
    }
    #[setter]
    fn set_kmin(&mut self, value: Option<f64>) {
        self.inner.kmin = value;
    }
    /// Background fit upper k limit in inverse angstroms. Default: available data limit.
    #[getter]
    fn kmax(&self) -> Option<f64> {
        self.inner.kmax
    }
    #[setter]
    fn set_kmax(&mut self, value: Option<f64>) {
        self.inner.kmax = value;
    }
    /// Uniform output k spacing in inverse angstroms. Default: 0.05.
    #[getter]
    fn kstep(&self) -> Option<f64> {
        self.inner.kstep
    }
    #[setter]
    fn set_kstep(&mut self, value: Option<f64>) {
        self.inner.kstep = value;
    }
    /// Number of samples at each endpoint used by the clamp. Default: 3; 0 disables clamping.
    #[getter]
    fn nclamp(&self) -> Option<i32> {
        self.inner.nclamp
    }
    #[setter]
    fn set_nclamp(&mut self, value: Option<i32>) {
        self.inner.nclamp = value;
    }
    /// Low-k endpoint weight. Default: 0 (disabled).
    #[getter]
    fn clamp_lo(&self) -> Option<i32> {
        self.inner.clamp_lo
    }
    #[setter]
    fn set_clamp_lo(&mut self, value: Option<i32>) {
        self.inner.clamp_lo = value;
    }
    /// High-k endpoint weight. Default: 1.
    #[getter]
    fn clamp_hi(&self) -> Option<i32> {
        self.inner.clamp_hi
    }
    #[setter]
    fn set_clamp_hi(&mut self, value: Option<i32>) {
        self.inner.clamp_hi = value;
    }
    /// FixedPenalty strength. Recommended default: 0.001; 0 disables the endpoint penalty.
    #[getter]
    fn clamp_lambda(&self) -> Option<f64> {
        self.inner.clamp_lambda
    }
    #[setter]
    fn set_clamp_lambda(&mut self, value: Option<f64>) {
        self.inner.clamp_lambda = value;
    }
    /// FFT length for background removal. Default: 2048.
    #[getter]
    fn nfft(&self) -> Option<i32> {
        self.inner.nfft
    }
    #[setter]
    fn set_nfft(&mut self, value: Option<i32>) {
        self.inner.nfft = value;
    }
    /// Power of k used in the background objective. Default: 1.
    #[getter]
    fn kweight(&self) -> Option<i32> {
        self.inner.kweight
    }
    #[setter]
    fn set_kweight(&mut self, value: Option<i32>) {
        self.inner.kweight = value;
    }
    /// Background window taper width in inverse angstroms. Default: 0.1.
    #[getter]
    fn dk(&self) -> Option<f64> {
        self.inner.dk
    }
    #[setter]
    fn set_dk(&mut self, value: Option<f64>) {
        self.inner.dk = value;
    }
    /// Legacy direct-solver ridge strength. Default: 0.0001; unused by FixedPenalty.
    #[getter]
    fn linear_regularization(&self) -> Option<f64> {
        self.inner.linear_regularization
    }
    #[setter]
    fn set_linear_regularization(&mut self, value: Option<f64>) {
        self.inner.linear_regularization = value;
    }
    /// Maximum accepted linear-system condition number. Default: 1e8.
    #[getter]
    fn linear_condition_limit(&self) -> Option<f64> {
        self.inner.linear_condition_limit
    }
    #[setter]
    fn set_linear_condition_limit(&mut self, value: Option<f64>) {
        self.inner.linear_condition_limit = value;
    }
    /// Legacy direct-solver residual acceptance ratio. Default: 1.05; unused by FixedPenalty.
    #[getter]
    fn linear_residual_ratio_limit(&self) -> Option<f64> {
        self.inner.linear_residual_ratio_limit
    }
    #[setter]
    fn set_linear_residual_ratio_limit(&mut self, value: Option<f64>) {
        self.inner.linear_residual_ratio_limit = value;
    }
    /// Allow legacy solver fallback. Default: True; FixedPenalty never falls back.
    #[getter]
    fn linear_fallback_to_lm(&self) -> Option<bool> {
        self.inner.linear_fallback_to_lm
    }
    #[setter]
    fn set_linear_fallback_to_lm(&mut self, value: Option<bool>) {
        self.inner.linear_fallback_to_lm = value;
    }
    /// Reuse compatible spline/FFT geometry and SVD factors. Default: True; each spectrum has a new right-hand side and solution.
    #[getter]
    fn linear_workspace_cache(&self) -> Option<bool> {
        self.inner.linear_workspace_cache
    }
    #[setter]
    fn set_linear_workspace_cache(&mut self, value: Option<bool>) {
        self.inner.linear_workspace_cache = value;
    }
    /// Background Fourier window. Default: Hanning.
    #[getter]
    fn window(&self) -> Option<String> {
        Some(format!("{:?}", self.inner.window))
    }
    #[setter]
    fn set_window(&mut self, value: Option<String>) -> PyResult<()> {
        let parsed = match value.as_deref() {
            None => None,
            Some("Hanning") => Some(rexafs::prelude::FTWindow::Hanning),
            Some("Parzen") => Some(rexafs::prelude::FTWindow::Parzen),
            Some("Welch") => Some(rexafs::prelude::FTWindow::Welch),
            Some("Gaussian") => Some(rexafs::prelude::FTWindow::Gaussian),
            Some("Sine") => Some(rexafs::prelude::FTWindow::Sine),
            Some("KaiserBessel") => Some(rexafs::prelude::FTWindow::KaiserBessel),
            Some("FHanning") => Some(rexafs::prelude::FTWindow::FHanning),
            Some(value) => {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "unknown FTWindow: {value}"
                )))
            }
        };
        self.inner.window = parsed.unwrap_or_default();
        Ok(())
    }
    /// Background solver. Recommended default: LinearDirect, required by FixedPenalty. TrustRegionDogLeg requires a Rust build with trust-region (included in Python, unavailable in Wasm).
    #[getter]
    fn solver(&self) -> Option<String> {
        self.inner.solver.map(|v| format!("{v:?}"))
    }
    #[setter]
    fn set_solver(&mut self, value: Option<String>) -> PyResult<()> {
        let parsed = match value.as_deref() {
            None => None,
            Some("TrustRegionDogLeg") => Some(rexafs::prelude::AUTOBKSolver::TrustRegionDogLeg),
            Some("LegacyLm") => Some(rexafs::prelude::AUTOBKSolver::LegacyLm),
            Some("LinearDirect") => Some(rexafs::prelude::AUTOBKSolver::LinearDirect),
            Some(value) => {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "unknown AUTOBKSolver: {value}"
                )))
            }
        };
        self.inner.solver = parsed;
        Ok(())
    }
    /// Legacy fallback solver. Default: TrustRegionDogLeg in Python, LegacyLm in Wasm; unused by FixedPenalty.
    #[getter]
    fn linear_fallback_solver(&self) -> Option<String> {
        self.inner.linear_fallback_solver.map(|v| format!("{v:?}"))
    }
    #[setter]
    fn set_linear_fallback_solver(&mut self, value: Option<String>) -> PyResult<()> {
        let parsed = match value.as_deref() {
            None => None,
            Some("TrustRegionDogLeg") => Some(rexafs::prelude::AUTOBKSolver::TrustRegionDogLeg),
            Some("LegacyLm") => Some(rexafs::prelude::AUTOBKSolver::LegacyLm),
            Some("LinearDirect") => Some(rexafs::prelude::AUTOBKSolver::LinearDirect),
            Some(value) => {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "unknown AUTOBKSolver: {value}"
                )))
            }
        };
        self.inner.linear_fallback_solver = parsed;
        Ok(())
    }
    /// Endpoint model. Recommended default: FixedPenalty with LinearDirect; Fixed and TwoPass are legacy models.
    #[getter]
    fn clamp_scale_policy(&self) -> Option<String> {
        self.inner.clamp_scale_policy.map(|v| format!("{v:?}"))
    }
    #[setter]
    fn set_clamp_scale_policy(&mut self, value: Option<String>) -> PyResult<()> {
        let parsed = match value.as_deref() {
            None => None,
            Some("FixedPenalty") => Some(rexafs::prelude::AUTOBKClampScalePolicy::FixedPenalty),
            Some("Fixed") => Some(rexafs::prelude::AUTOBKClampScalePolicy::Fixed),
            Some("TwoPass") => Some(rexafs::prelude::AUTOBKClampScalePolicy::TwoPass),
            Some(value) => {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "unknown AUTOBKClampScalePolicy: {value}"
                )))
            }
        };
        self.inner.clamp_scale_policy = parsed;
        Ok(())
    }
}

/// Forward Fourier-transform settings. Defaults: k=2..15 inverse angstroms, kweight=2, KaiserBessel window.
#[pyclass(name = "XrayFFTF", module = "rexafs", skip_from_py_object)]
#[derive(Clone)]
struct PyXrayFFTF {
    inner: rexafs::XrayFFTF,
}
#[pymethods]
impl PyXrayFFTF {
    /// Create settings with Rust defaults; override only the keyword arguments you need.
    #[new]
    #[pyo3(
        text_signature = "(*, grid='Input', rmax_out=10.0, dk=1.0, dk2=None, kmin=2.0, kmax=15.0, kweight=2.0, nfft=2048, kstep=None, window='KaiserBessel')"
    )]
    #[pyo3(signature = (*, grid="Input", rmax_out=Some(10.0), dk=Some(1.0), dk2=None, kmin=Some(2.0), kmax=Some(15.0), kweight=Some(2.0), nfft=Some(2048), kstep=None, window=Some("KaiserBessel")))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        grid: &str,
        rmax_out: Option<f64>,
        dk: Option<f64>,
        dk2: Option<f64>,
        kmin: Option<f64>,
        kmax: Option<f64>,
        kweight: Option<f64>,
        nfft: Option<usize>,
        kstep: Option<f64>,
        window: Option<&str>,
    ) -> PyResult<Self> {
        let mut result = Self {
            inner: rexafs::XrayFFTF::new(),
        };
        result.set_grid(grid)?;
        result.set_rmax_out(rmax_out);
        result.set_dk(dk);
        result.set_dk2(dk2);
        result.set_kmin(kmin);
        result.set_kmax(kmax);
        result.set_kweight(kweight);
        result.set_nfft(nfft);
        result.set_kstep(kstep);
        result.set_window(window.map(str::to_owned))?;
        Ok(result)
    }
    /// Sampling/window domain. Default: Input (existing k grid). Larch resamples on the extended FFT window grid.
    #[getter]
    fn grid(&self) -> String {
        format!("{:?}", self.inner.grid)
    }
    #[setter]
    fn set_grid(&mut self, value: &str) -> PyResult<()> {
        self.inner.grid = match value {
            "Input" => rexafs::FFTGrid::Input,
            "Larch" => rexafs::FFTGrid::Larch,
            _ => {
                return Err(pyo3::exceptions::PyValueError::new_err(
                    "FFT grid must be Input or Larch",
                ))
            }
        };
        Ok(())
    }
    /// Maximum displayed R in angstroms. Default: 10.0; does not truncate the inverse-transform filter.
    #[getter]
    fn rmax_out(&self) -> Option<f64> {
        self.inner.rmax_out
    }
    #[setter]
    fn set_rmax_out(&mut self, value: Option<f64>) {
        self.inner.rmax_out = value;
    }
    /// Low-k taper width in inverse angstroms. Default: 1.0.
    #[getter]
    fn dk(&self) -> Option<f64> {
        self.inner.dk
    }
    #[setter]
    fn set_dk(&mut self, value: Option<f64>) {
        self.inner.dk = value;
    }
    /// High-k taper width in inverse angstroms. Default: use dk.
    #[getter]
    fn dk2(&self) -> Option<f64> {
        self.inner.dk2
    }
    #[setter]
    fn set_dk2(&mut self, value: Option<f64>) {
        self.inner.dk2 = value;
    }
    /// Lower Fourier window limit in inverse angstroms. Default: 2.0; None/undefined uses the first k sample.
    #[getter]
    fn kmin(&self) -> Option<f64> {
        self.inner.kmin
    }
    #[setter]
    fn set_kmin(&mut self, value: Option<f64>) {
        self.inner.kmin = value;
    }
    /// Upper Fourier window limit in inverse angstroms. Default: 15.0; None/undefined uses the last k sample.
    #[getter]
    fn kmax(&self) -> Option<f64> {
        self.inner.kmax
    }
    #[setter]
    fn set_kmax(&mut self, value: Option<f64>) {
        self.inner.kmax = value;
    }
    /// Power of k applied before FFT. Default: 2.0; nonnegative values are floored to an integer.
    #[getter]
    fn kweight(&self) -> Option<f64> {
        self.inner.kweight
    }
    #[setter]
    fn set_kweight(&mut self, value: Option<f64>) {
        self.inner.kweight = value;
    }
    /// Forward FFT length. Default: 2048.
    #[getter]
    fn nfft(&self) -> Option<usize> {
        self.inner.nfft
    }
    #[setter]
    fn set_nfft(&mut self, value: Option<usize>) {
        self.inner.nfft = value;
    }
    /// FFT k spacing in inverse angstroms. Default: infer from input k.
    #[getter]
    fn kstep(&self) -> Option<f64> {
        self.inner.kstep
    }
    #[setter]
    fn set_kstep(&mut self, value: Option<f64>) {
        self.inner.kstep = value;
    }
    /// Fourier window shape. Default: KaiserBessel; explicitly unset uses Hanning.
    #[getter]
    fn window(&self) -> Option<String> {
        self.inner.window.map(|v| format!("{v:?}"))
    }
    #[setter]
    fn set_window(&mut self, value: Option<String>) -> PyResult<()> {
        let parsed = match value.as_deref() {
            None => None,
            Some("Hanning") => Some(rexafs::prelude::FTWindow::Hanning),
            Some("Parzen") => Some(rexafs::prelude::FTWindow::Parzen),
            Some("Welch") => Some(rexafs::prelude::FTWindow::Welch),
            Some("Gaussian") => Some(rexafs::prelude::FTWindow::Gaussian),
            Some("Sine") => Some(rexafs::prelude::FTWindow::Sine),
            Some("KaiserBessel") => Some(rexafs::prelude::FTWindow::KaiserBessel),
            Some("FHanning") => Some(rexafs::prelude::FTWindow::FHanning),
            Some(value) => {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "unknown FTWindow: {value}"
                )))
            }
        };
        self.inner.window = parsed;
        Ok(())
    }
}

/// Inverse Fourier-transform settings. Set rmin/rmax to select an R-space shell.
#[pyclass(name = "XrayFFTR", module = "rexafs", skip_from_py_object)]
#[derive(Clone)]
struct PyXrayFFTR {
    inner: rexafs::XrayFFTR,
}
#[pymethods]
impl PyXrayFFTR {
    /// Create settings with Rust defaults; override only the keyword arguments you need.
    #[new]
    #[pyo3(
        text_signature = "(*, qmax_out=10.0, dr=1.0, dr2=None, rmin=0.0, rmax=20.0, rweight=0.0, nfft=2048, kstep=None, window='KaiserBessel')"
    )]
    #[pyo3(signature = (*, qmax_out=Some(10.0), dr=Some(1.0), dr2=None, rmin=Some(0.0), rmax=Some(20.0), rweight=Some(0.0), nfft=Some(2048), kstep=None, window=Some("KaiserBessel")))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        qmax_out: Option<f64>,
        dr: Option<f64>,
        dr2: Option<f64>,
        rmin: Option<f64>,
        rmax: Option<f64>,
        rweight: Option<f64>,
        nfft: Option<usize>,
        kstep: Option<f64>,
        window: Option<&str>,
    ) -> PyResult<Self> {
        let mut result = Self {
            inner: rexafs::XrayFFTR::new(),
        };
        result.set_qmax_out(qmax_out);
        result.set_dr(dr);
        result.set_dr2(dr2);
        result.set_rmin(rmin);
        result.set_rmax(rmax);
        result.set_rweight(rweight);
        result.set_nfft(nfft);
        result.set_kstep(kstep);
        result.set_window(window.map(str::to_owned))?;
        Ok(result)
    }
    /// Maximum back-transform q in inverse angstroms. Default: 10.0.
    #[getter]
    fn qmax_out(&self) -> Option<f64> {
        self.inner.qmax_out
    }
    #[setter]
    fn set_qmax_out(&mut self, value: Option<f64>) {
        self.inner.qmax_out = value;
    }
    /// Low-R taper width in angstroms. Default: 1.0.
    #[getter]
    fn dr(&self) -> Option<f64> {
        self.inner.dr
    }
    #[setter]
    fn set_dr(&mut self, value: Option<f64>) {
        self.inner.dr = value;
    }
    /// High-R taper width in angstroms. Default: use dr.
    #[getter]
    fn dr2(&self) -> Option<f64> {
        self.inner.dr2
    }
    #[setter]
    fn set_dr2(&mut self, value: Option<f64>) {
        self.inner.dr2 = value;
    }
    /// Lower inverse-transform window limit in angstroms. Default: 0.0.
    #[getter]
    fn rmin(&self) -> Option<f64> {
        self.inner.rmin
    }
    #[setter]
    fn set_rmin(&mut self, value: Option<f64>) {
        self.inner.rmin = value;
    }
    /// Upper inverse-transform window limit in angstroms. Default: 20.0; choose a shell range for R filtering.
    #[getter]
    fn rmax(&self) -> Option<f64> {
        self.inner.rmax
    }
    #[setter]
    fn set_rmax(&mut self, value: Option<f64>) {
        self.inner.rmax = value;
    }
    /// Power of R applied before IFFT. Default: 0.0; nonnegative values are floored to an integer.
    #[getter]
    fn rweight(&self) -> Option<f64> {
        self.inner.rweight
    }
    #[setter]
    fn set_rweight(&mut self, value: Option<f64>) {
        self.inner.rweight = value;
    }
    /// Inverse FFT length. Default: 2048; leave kstep automatic when changing this.
    #[getter]
    fn nfft(&self) -> Option<usize> {
        self.inner.nfft
    }
    #[setter]
    fn set_nfft(&mut self, value: Option<usize>) {
        self.inner.nfft = value;
    }
    /// Output q spacing in inverse angstroms. Default: infer from input R and nfft; an explicit value must match that spacing.
    #[getter]
    fn kstep(&self) -> Option<f64> {
        self.inner.kstep
    }
    #[setter]
    fn set_kstep(&mut self, value: Option<f64>) {
        self.inner.kstep = value;
    }
    /// Inverse Fourier window shape. Default: KaiserBessel; explicitly unset uses Hanning.
    #[getter]
    fn window(&self) -> Option<String> {
        self.inner.window.map(|v| format!("{v:?}"))
    }
    #[setter]
    fn set_window(&mut self, value: Option<String>) -> PyResult<()> {
        let parsed = match value.as_deref() {
            None => None,
            Some("Hanning") => Some(rexafs::prelude::FTWindow::Hanning),
            Some("Parzen") => Some(rexafs::prelude::FTWindow::Parzen),
            Some("Welch") => Some(rexafs::prelude::FTWindow::Welch),
            Some("Gaussian") => Some(rexafs::prelude::FTWindow::Gaussian),
            Some("Sine") => Some(rexafs::prelude::FTWindow::Sine),
            Some("KaiserBessel") => Some(rexafs::prelude::FTWindow::KaiserBessel),
            Some("FHanning") => Some(rexafs::prelude::FTWindow::FHanning),
            Some(value) => {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "unknown FTWindow: {value}"
                )))
            }
        };
        self.inner.window = parsed;
        Ok(())
    }
}

/// Rust normalization algorithm selection. Prefer passing PrePostEdge settings directly to set_normalization_method.
#[pyclass(name = "NormalizationMethod", module = "rexafs", skip_from_py_object)]
#[derive(Clone)]
struct PyNormalizationMethod {
    inner: rexafs::NormalizationMethod,
}
#[pymethods]
impl PyNormalizationMethod {
    /// Copy pre/post-edge settings into a normalization method.
    #[staticmethod]
    #[pyo3(name = "PrePostEdge")]
    fn configured(parameters: &PyPrePostEdge) -> Self {
        Self {
            inner: rexafs::NormalizationMethod::PrePostEdge(parameters.inner.clone()),
        }
    }
    /// Create automatic pre/post-edge normalization settings.
    #[staticmethod]
    fn new_prepostedge() -> Self {
        Self {
            inner: rexafs::NormalizationMethod::new_prepostedge(),
        }
    }
    /// Create an unimplemented MBack placeholder; processing raises ValueError.
    #[staticmethod]
    fn new_mback() -> Self {
        Self {
            inner: rexafs::NormalizationMethod::new_mback(),
        }
    }
}

/// Rust background algorithm selection. Prefer passing AUTOBK settings directly to set_background_method.
#[pyclass(name = "BackgroundMethod", module = "rexafs", skip_from_py_object)]
#[derive(Clone)]
struct PyBackgroundMethod {
    inner: rexafs::BackgroundMethod,
}
#[pymethods]
impl PyBackgroundMethod {
    /// Copy AUTOBK settings into a background method.
    #[staticmethod]
    #[pyo3(name = "AUTOBK")]
    fn configured(parameters: &PyAUTOBK) -> Self {
        Self {
            inner: rexafs::BackgroundMethod::AUTOBK(parameters.inner.clone()),
        }
    }
    /// Create the recommended default AUTOBK method.
    #[staticmethod]
    fn new_autobk() -> Self {
        Self {
            inner: rexafs::BackgroundMethod::new_autobk(),
        }
    }
    /// Create an unimplemented ILPBkg placeholder; processing raises RuntimeError.
    #[staticmethod]
    fn new_ilpbkg() -> Self {
        Self {
            inner: rexafs::BackgroundMethod::new_ilpbkg(),
        }
    }
}

/// Mutable Rust spectrum. Stages return the same object and compute missing prerequisites. Energy is in eV; array results are independent NumPy copies.
#[pyclass(name = "Spectrum", module = "rexafs", skip_from_py_object)]
struct PySpectrum {
    inner: rexafs::Spectrum,
}
#[pymethods]
impl PySpectrum {
    /// Copy energy (eV) and absorption mu into a spectrum. Inputs must be finite, one-dimensional, equal-length, with strictly increasing energy.
    #[new]
    fn new(py: Python<'_>, energy: &Bound<'_, PyAny>, mu: &Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(Self {
            inner: arrays(py, energy, mu)?,
        })
    }
    /// Create a spectrum from energy (eV) and absorption mu. Copies input arrays; equivalent to the constructor.
    #[staticmethod]
    fn from_arrays(
        py: Python<'_>,
        energy: &Bound<'_, PyAny>,
        mu: &Bound<'_, PyAny>,
    ) -> PyResult<Self> {
        Self::new(py, energy, mu)
    }
    /// Replace energy (eV) and mu, copy the inputs and clear E0 and derived results. Returns this spectrum.
    fn set_spectrum<'py>(
        mut slf: PyRefMut<'py, Self>,
        energy: &Bound<'py, PyAny>,
        mu: &Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let input = arrays(slf.py(), energy, mu)?;
        slf.inner
            .set_spectrum(input.energy.unwrap(), input.mu.unwrap());
        Ok(slf)
    }
    /// Set edge energy in eV and invalidate normalization and downstream results. Returns this spectrum.
    fn set_e0(mut slf: PyRefMut<'_, Self>, e0: f64) -> PyRefMut<'_, Self> {
        slf.inner.set_e0(e0);
        slf
    }
    /// Edge energy in eV, or None before detection or assignment.
    fn e0(&self) -> Option<f64> {
        self.inner.e0()
    }
    /// Clear all calculated results while retaining stage settings for recomputation. Returns this spectrum.
    fn invalidate_derived(mut slf: PyRefMut<'_, Self>) -> PyRefMut<'_, Self> {
        slf.inner.invalidate_derived();
        slf
    }
    /// Copy inverse-transform settings; clear q and chi(q) while preserving forward results. Returns this spectrum.
    fn set_ifft<'py>(mut slf: PyRefMut<'py, Self>, parameters: &PyXrayFFTR) -> PyRefMut<'py, Self> {
        slf.inner.set_ifft(parameters.inner.clone());
        slf
    }
    /// Copy forward-transform settings; clear Fourier and inverse results while preserving normalization and chi(k). Returns this spectrum.
    fn set_fft<'py>(mut slf: PyRefMut<'py, Self>, parameters: &PyXrayFFTF) -> PyRefMut<'py, Self> {
        slf.inner.set_fft(parameters.inner.clone());
        slf
    }
    /// Copy normalization settings and invalidate normalization and downstream results. Accepts PrePostEdge directly or a NormalizationMethod; omitted/None restores automatic pre/post-edge normalization.
    #[pyo3(signature = (method=None))]
    fn set_normalization_method<'py>(
        mut slf: PyRefMut<'py, Self>,
        method: Option<&Bound<'py, PyAny>>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let selected = match method {
            None => None,
            Some(value) => {
                if let Ok(parameters) = value.extract::<PyRef<'_, PyPrePostEdge>>() {
                    Some(rexafs::NormalizationMethod::PrePostEdge(
                        parameters.inner.clone(),
                    ))
                } else {
                    Some(
                        value
                            .extract::<PyRef<'_, PyNormalizationMethod>>()?
                            .inner
                            .clone(),
                    )
                }
            }
        };
        slf.inner
            .set_normalization_method(selected)
            .map_err(error)?;
        Ok(slf)
    }
    /// Copy background settings and invalidate background and downstream results. Accepts AUTOBK directly or a BackgroundMethod; omitted/None restores default AUTOBK.
    #[pyo3(signature = (method=None))]
    fn set_background_method<'py>(
        mut slf: PyRefMut<'py, Self>,
        method: Option<&Bound<'py, PyAny>>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let selected = match method {
            None => None,
            Some(value) => {
                if let Ok(parameters) = value.extract::<PyRef<'_, PyAUTOBK>>() {
                    Some(rexafs::BackgroundMethod::AUTOBK(parameters.inner.clone()))
                } else {
                    Some(
                        value
                            .extract::<PyRef<'_, PyBackgroundMethod>>()?
                            .inner
                            .clone(),
                    )
                }
            }
        };
        slf.inner.set_background_method(selected).map_err(error)?;
        Ok(slf)
    }
    /// Detect edge energy from mu and invalidate dependent results. Returns this spectrum.
    fn find_e0(mut slf: PyRefMut<'_, Self>) -> PyResult<PyRefMut<'_, Self>> {
        let py = slf.py();
        let inner = &mut slf.inner;
        py.detach(|| inner.find_e0().map(|_| ())).map_err(error)?;
        Ok(slf)
    }
    /// Run pre/post-edge normalization, finding E0 if needed. Returns this spectrum.
    fn normalize(mut slf: PyRefMut<'_, Self>) -> PyResult<PyRefMut<'_, Self>> {
        let py = slf.py();
        let inner = &mut slf.inner;
        py.detach(|| inner.normalize().map(|_| ())).map_err(error)?;
        Ok(slf)
    }
    /// Run AUTOBK, computing missing normalization first. Returns this spectrum.
    fn calc_background(mut slf: PyRefMut<'_, Self>) -> PyResult<PyRefMut<'_, Self>> {
        let py = slf.py();
        let inner = &mut slf.inner;
        py.detach(|| inner.calc_background().map(|_| ()))
            .map_err(error)?;
        Ok(slf)
    }
    /// Compute chi(R), running missing normalization and AUTOBK first. Defaults: k=2..15 inverse angstroms, kweight=2, KaiserBessel, nfft=2048. Returns this spectrum.
    fn fft(mut slf: PyRefMut<'_, Self>) -> PyResult<PyRefMut<'_, Self>> {
        let py = slf.py();
        let inner = &mut slf.inner;
        py.detach(|| inner.fft().map(|_| ())).map_err(error)?;
        Ok(slf)
    }
    /// Back-transform chi(R) to chi(q), running missing forward stages first. Configure the R window with set_ifft(XrayFFTR(...)). Returns this spectrum.
    fn ifft(mut slf: PyRefMut<'_, Self>) -> PyResult<PyRefMut<'_, Self>> {
        let py = slf.py();
        let inner = &mut slf.inner;
        py.detach(|| inner.ifft().map(|_| ())).map_err(error)?;
        Ok(slf)
    }
    /// Uniform background k axis in inverse angstroms; pairs with chi(). Returns an independent array copy, or None before its stage runs.
    fn k<'py>(&self, py: Python<'py>) -> Option<Bound<'py, PyArray1<f64>>> {
        self.inner.k().map(|v| PyArray1::from_vec(py, v.to_vec()))
    }
    /// Unweighted EXAFS chi(k) = (mu - smooth background) / edge_step; dimensionless and paired with k(). Returns an independent array copy, or None before its stage runs.
    fn chi<'py>(&self, py: Python<'py>) -> Option<Bound<'py, PyArray1<f64>>> {
        self.inner.chi().map(|v| PyArray1::from_vec(py, v.to_vec()))
    }
    /// Normalized absorption (mu - pre_edge) / edge_step on the input energy grid; dimensionless. Returns an independent array copy, or None before its stage runs.
    fn norm<'py>(&self, py: Python<'py>) -> Option<Bound<'py, PyArray1<f64>>> {
        self.inner
            .norm()
            .map(|v| PyArray1::from_vec(py, v.as_slice().to_vec()))
    }
    /// Normalized absorption with its fitted post-edge trend removed, preserving the edge value; dimensionless. Returns an independent array copy, or None before its stage runs.
    fn flat<'py>(&self, py: Python<'py>) -> Option<Bound<'py, PyArray1<f64>>> {
        self.inner
            .flat()
            .map(|v| PyArray1::from_vec(py, v.as_slice().to_vec()))
    }
    /// Fitted pre-edge baseline in mu units on the input energy grid. Returns an independent array copy, or None before its stage runs.
    fn pre_edge<'py>(&self, py: Python<'py>) -> Option<Bound<'py, PyArray1<f64>>> {
        self.inner
            .pre_edge()
            .map(|v| PyArray1::from_vec(py, v.as_slice().to_vec()))
    }
    /// Fitted post-edge baseline in mu units on the input energy grid. Returns an independent array copy, or None before its stage runs.
    fn post_edge<'py>(&self, py: Python<'py>) -> Option<Bound<'py, PyArray1<f64>>> {
        self.inner
            .post_edge()
            .map(|v| PyArray1::from_vec(py, v.as_slice().to_vec()))
    }
    /// Forward-transform R axis in angstroms; pairs with chir_mag/real/imag(). Peaks are not phase-corrected bond lengths. Returns an independent array copy, or None before its stage runs.
    fn r<'py>(&self, py: Python<'py>) -> Option<Bound<'py, PyArray1<f64>>> {
        self.inner
            .r()
            .map(|v| PyArray1::from_vec(py, v.as_slice().to_vec()))
    }
    /// Forward Fourier window values; use kwin_k() for the matching axis. Returns an independent array copy, or None before its stage runs.
    fn kwin<'py>(&self, py: Python<'py>) -> Option<Bound<'py, PyArray1<f64>>> {
        self.inner
            .kwin()
            .map(|v| PyArray1::from_vec(py, v.as_slice().to_vec()))
    }
    /// Forward Fourier window k axis in inverse angstroms; may differ from k() with grid=Larch. Returns an independent array copy, or None before its stage runs.
    fn kwin_k<'py>(&self, py: Python<'py>) -> Option<Bound<'py, PyArray1<f64>>> {
        self.inner
            .kwin_k()
            .map(|v| PyArray1::from_vec(py, v.as_slice().to_vec()))
    }
    /// Magnitude of chi(R); pairs with r(). Returns an independent array copy, or None before its stage runs.
    fn chir_mag<'py>(&self, py: Python<'py>) -> Option<Bound<'py, PyArray1<f64>>> {
        self.inner
            .chir_mag()
            .map(|v| PyArray1::from_vec(py, v.as_slice().to_vec()))
    }
    /// Real component of chi(R); pairs with r(). Returns an independent array copy, or None before its stage runs.
    fn chir_real<'py>(&self, py: Python<'py>) -> Option<Bound<'py, PyArray1<f64>>> {
        self.inner
            .chir_real()
            .map(|v| PyArray1::from_vec(py, v.as_slice().to_vec()))
    }
    /// Imaginary component of chi(R); pairs with r(). Returns an independent array copy, or None before its stage runs.
    fn chir_imag<'py>(&self, py: Python<'py>) -> Option<Bound<'py, PyArray1<f64>>> {
        self.inner
            .chir_imag()
            .map(|v| PyArray1::from_vec(py, v.as_slice().to_vec()))
    }
    /// Back-transform q axis in inverse angstroms; pairs with chiq(). Returns an independent array copy, or None before its stage runs.
    fn q<'py>(&self, py: Python<'py>) -> Option<Bound<'py, PyArray1<f64>>> {
        self.inner
            .q()
            .map(|v| PyArray1::from_vec(py, v.as_slice().to_vec()))
    }
    /// Real R-filtered signal on q(); forward k-weighting and windowing remain, so this is not generally the unweighted chi(k). Returns an independent array copy, or None before its stage runs.
    fn chiq<'py>(&self, py: Python<'py>) -> Option<Bound<'py, PyArray1<f64>>> {
        self.inner
            .chiq()
            .map(|v| PyArray1::from_vec(py, v.as_slice().to_vec()))
    }
}
#[pyfunction]
fn read_qas_transmission(path: &str) -> PyResult<PySpectrum> {
    Ok(PySpectrum {
        inner: rexafs::io::read_qas_transmission(path).map_err(|e| error(e.into()))?,
    })
}
#[pymodule]
fn _core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyPrePostEdge>()?;
    m.add_class::<PyAUTOBK>()?;
    m.add_class::<PyXrayFFTF>()?;
    m.add_class::<PyXrayFFTR>()?;
    m.add_class::<PyNormalizationMethod>()?;
    m.add_class::<PyBackgroundMethod>()?;
    m.add_class::<PySpectrum>()?;
    m.add_function(wrap_pyfunction!(read_qas_transmission, m)?)?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}
