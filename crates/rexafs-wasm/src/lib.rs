//! Thin Wasm bindings: stage execution and defaults live in rexafs.
use wasm_bindgen::prelude::*;
fn error(error: rexafs::Error) -> JsValue {
    js_sys::Error::new(&error.to_string()).into()
}

/// Pre/post-edge normalization settings. Defaults adapt to the measured energy range.
#[wasm_bindgen(js_name = PrePostEdge)]
pub struct WasmPrePostEdge {
    inner: rexafs::PrePostEdge,
}
#[wasm_bindgen(js_class = PrePostEdge)]
impl WasmPrePostEdge {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: rexafs::PrePostEdge::new(),
        }
    }
    /// Pre-edge fit start relative to E0, in eV. Default: infer from the measured range.
    #[wasm_bindgen(getter)]
    pub fn pre_edge_start(&self) -> Option<f64> {
        self.inner.pre_edge_start
    }
    #[wasm_bindgen(setter)]
    pub fn set_pre_edge_start(&mut self, value: Option<f64>) {
        self.inner.pre_edge_start = value;
    }
    /// Pre-edge fit end relative to E0, in eV. Default: infer from the pre-edge start.
    #[wasm_bindgen(getter)]
    pub fn pre_edge_end(&self) -> Option<f64> {
        self.inner.pre_edge_end
    }
    #[wasm_bindgen(setter)]
    pub fn set_pre_edge_end(&mut self, value: Option<f64>) {
        self.inner.pre_edge_end = value;
    }
    /// Post-edge fit start relative to E0, in eV. Default: infer from the available range (at most 25 eV).
    #[wasm_bindgen(getter)]
    pub fn norm_start(&self) -> Option<f64> {
        self.inner.norm_start
    }
    #[wasm_bindgen(setter)]
    pub fn set_norm_start(&mut self, value: Option<f64>) {
        self.inner.norm_start = value;
    }
    /// Post-edge fit end relative to E0, in eV. Default: measured upper energy limit.
    #[wasm_bindgen(getter)]
    pub fn norm_end(&self) -> Option<f64> {
        self.inner.norm_end
    }
    #[wasm_bindgen(setter)]
    pub fn set_norm_end(&mut self, value: Option<f64>) {
        self.inner.norm_end = value;
    }
    /// Post-edge polynomial degree, 0 through 5. Default: 0, 1 or 2 for fit spans below 50, below 350, or at least 350 eV.
    #[wasm_bindgen(getter)]
    pub fn norm_polyorder(&self) -> Option<i32> {
        self.inner.norm_polyorder
    }
    #[wasm_bindgen(setter)]
    pub fn set_norm_polyorder(&mut self, value: Option<i32>) {
        self.inner.norm_polyorder = value;
    }
    /// Victoreen energy exponent for the pre-edge fit. Default: 0.
    #[wasm_bindgen(getter)]
    pub fn n_victoreen(&self) -> Option<i32> {
        self.inner.n_victoreen
    }
    #[wasm_bindgen(setter)]
    pub fn set_n_victoreen(&mut self, value: Option<i32>) {
        self.inner.n_victoreen = value;
    }
    /// Edge energy in eV. Default: detect from the spectrum.
    #[wasm_bindgen(getter)]
    pub fn e0(&self) -> Option<f64> {
        self.inner.e0
    }
    #[wasm_bindgen(setter)]
    pub fn set_e0(&mut self, value: Option<f64>) {
        self.inner.e0 = value;
    }
    /// Absorption edge-step override in mu units. Default: estimate from the fitted baselines.
    #[wasm_bindgen(getter)]
    pub fn edge_step(&self) -> Option<f64> {
        self.inner.edge_step
    }
    #[wasm_bindgen(setter)]
    pub fn set_edge_step(&mut self, value: Option<f64>) {
        self.inner.edge_step = value;
    }
}

/// AUTOBK background settings. Recommended defaults use LinearDirect and FixedPenalty with lambda 0.001.
#[wasm_bindgen(js_name = AUTOBK)]
pub struct WasmAUTOBK {
    inner: rexafs::AUTOBK,
}
#[wasm_bindgen(js_class = AUTOBK)]
impl WasmAUTOBK {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: rexafs::AUTOBK::new(),
        }
    }
    /// Edge energy in eV. Default: use normalization E0.
    #[wasm_bindgen(getter)]
    pub fn ek0(&self) -> Option<f64> {
        self.inner.ek0
    }
    #[wasm_bindgen(setter)]
    pub fn set_ek0(&mut self, value: Option<f64>) {
        self.inner.ek0 = value;
    }
    /// Background cutoff in angstroms. AUTOBK suppresses Fourier residuals below this R. Default: 1.0; increasing it can remove structural signal.
    #[wasm_bindgen(getter)]
    pub fn rbkg(&self) -> Option<f64> {
        self.inner.rbkg
    }
    #[wasm_bindgen(setter)]
    pub fn set_rbkg(&mut self, value: Option<f64>) {
        self.inner.rbkg = value;
    }
    /// Spline knot count. Default: determine from rbkg and the k range.
    #[wasm_bindgen(getter)]
    pub fn nknots(&self) -> Option<i32> {
        self.inner.nknots
    }
    #[wasm_bindgen(setter)]
    pub fn set_nknots(&mut self, value: Option<i32>) {
        self.inner.nknots = value;
    }
    /// Background fit lower k limit in inverse angstroms. Default: 0.0.
    #[wasm_bindgen(getter)]
    pub fn kmin(&self) -> Option<f64> {
        self.inner.kmin
    }
    #[wasm_bindgen(setter)]
    pub fn set_kmin(&mut self, value: Option<f64>) {
        self.inner.kmin = value;
    }
    /// Background fit upper k limit in inverse angstroms. Default: available data limit.
    #[wasm_bindgen(getter)]
    pub fn kmax(&self) -> Option<f64> {
        self.inner.kmax
    }
    #[wasm_bindgen(setter)]
    pub fn set_kmax(&mut self, value: Option<f64>) {
        self.inner.kmax = value;
    }
    /// Uniform output k spacing in inverse angstroms. Default: 0.05.
    #[wasm_bindgen(getter)]
    pub fn kstep(&self) -> Option<f64> {
        self.inner.kstep
    }
    #[wasm_bindgen(setter)]
    pub fn set_kstep(&mut self, value: Option<f64>) {
        self.inner.kstep = value;
    }
    /// Number of samples at each endpoint used by the clamp. Default: 3; 0 disables clamping.
    #[wasm_bindgen(getter)]
    pub fn nclamp(&self) -> Option<i32> {
        self.inner.nclamp
    }
    #[wasm_bindgen(setter)]
    pub fn set_nclamp(&mut self, value: Option<i32>) {
        self.inner.nclamp = value;
    }
    /// Low-k endpoint weight. Default: 0 (disabled).
    #[wasm_bindgen(getter)]
    pub fn clamp_lo(&self) -> Option<i32> {
        self.inner.clamp_lo
    }
    #[wasm_bindgen(setter)]
    pub fn set_clamp_lo(&mut self, value: Option<i32>) {
        self.inner.clamp_lo = value;
    }
    /// High-k endpoint weight. Default: 1.
    #[wasm_bindgen(getter)]
    pub fn clamp_hi(&self) -> Option<i32> {
        self.inner.clamp_hi
    }
    #[wasm_bindgen(setter)]
    pub fn set_clamp_hi(&mut self, value: Option<i32>) {
        self.inner.clamp_hi = value;
    }
    /// FixedPenalty strength. Recommended default: 0.001; 0 disables the endpoint penalty.
    #[wasm_bindgen(getter)]
    pub fn clamp_lambda(&self) -> Option<f64> {
        self.inner.clamp_lambda
    }
    #[wasm_bindgen(setter)]
    pub fn set_clamp_lambda(&mut self, value: Option<f64>) {
        self.inner.clamp_lambda = value;
    }
    /// FFT length for background removal. Default: 2048.
    #[wasm_bindgen(getter)]
    pub fn nfft(&self) -> Option<i32> {
        self.inner.nfft
    }
    #[wasm_bindgen(setter)]
    pub fn set_nfft(&mut self, value: Option<i32>) {
        self.inner.nfft = value;
    }
    /// Power of k used in the background objective. Default: 1.
    #[wasm_bindgen(getter)]
    pub fn kweight(&self) -> Option<i32> {
        self.inner.kweight
    }
    #[wasm_bindgen(setter)]
    pub fn set_kweight(&mut self, value: Option<i32>) {
        self.inner.kweight = value;
    }
    /// Background window taper width in inverse angstroms. Default: 0.1.
    #[wasm_bindgen(getter)]
    pub fn dk(&self) -> Option<f64> {
        self.inner.dk
    }
    #[wasm_bindgen(setter)]
    pub fn set_dk(&mut self, value: Option<f64>) {
        self.inner.dk = value;
    }
    /// Legacy direct-solver ridge strength. Default: 0.0001; unused by FixedPenalty.
    #[wasm_bindgen(getter)]
    pub fn linear_regularization(&self) -> Option<f64> {
        self.inner.linear_regularization
    }
    #[wasm_bindgen(setter)]
    pub fn set_linear_regularization(&mut self, value: Option<f64>) {
        self.inner.linear_regularization = value;
    }
    /// Maximum accepted linear-system condition number. Default: 1e8.
    #[wasm_bindgen(getter)]
    pub fn linear_condition_limit(&self) -> Option<f64> {
        self.inner.linear_condition_limit
    }
    #[wasm_bindgen(setter)]
    pub fn set_linear_condition_limit(&mut self, value: Option<f64>) {
        self.inner.linear_condition_limit = value;
    }
    /// Legacy direct-solver residual acceptance ratio. Default: 1.05; unused by FixedPenalty.
    #[wasm_bindgen(getter)]
    pub fn linear_residual_ratio_limit(&self) -> Option<f64> {
        self.inner.linear_residual_ratio_limit
    }
    #[wasm_bindgen(setter)]
    pub fn set_linear_residual_ratio_limit(&mut self, value: Option<f64>) {
        self.inner.linear_residual_ratio_limit = value;
    }
    /// Allow legacy solver fallback. Default: True; FixedPenalty never falls back.
    #[wasm_bindgen(getter)]
    pub fn linear_fallback_to_lm(&self) -> Option<bool> {
        self.inner.linear_fallback_to_lm
    }
    #[wasm_bindgen(setter)]
    pub fn set_linear_fallback_to_lm(&mut self, value: Option<bool>) {
        self.inner.linear_fallback_to_lm = value;
    }
    /// Reuse compatible spline/FFT geometry and SVD factors. Default: True; each spectrum has a new right-hand side and solution.
    #[wasm_bindgen(getter)]
    pub fn linear_workspace_cache(&self) -> Option<bool> {
        self.inner.linear_workspace_cache
    }
    #[wasm_bindgen(setter)]
    pub fn set_linear_workspace_cache(&mut self, value: Option<bool>) {
        self.inner.linear_workspace_cache = value;
    }
    /// Background Fourier window. Default: Hanning.
    #[wasm_bindgen(getter)]
    pub fn window(&self) -> Option<String> {
        Some(format!("{:?}", self.inner.window))
    }
    #[wasm_bindgen(setter)]
    pub fn set_window(&mut self, value: Option<String>) -> Result<(), JsValue> {
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
                return Err(js_sys::Error::new(&format!("unknown FTWindow: {value}")).into())
            }
        };
        self.inner.window = parsed.unwrap_or_default();
        Ok(())
    }
    /// Background solver. Recommended default: LinearDirect, required by FixedPenalty. TrustRegionDogLeg requires a Rust build with trust-region (included in Python, unavailable in Wasm).
    #[wasm_bindgen(getter)]
    pub fn solver(&self) -> Option<String> {
        self.inner.solver.map(|v| format!("{v:?}"))
    }
    #[wasm_bindgen(setter)]
    pub fn set_solver(&mut self, value: Option<String>) -> Result<(), JsValue> {
        let parsed = match value.as_deref() {
            None => None,
            Some("TrustRegionDogLeg") => Some(rexafs::prelude::AUTOBKSolver::TrustRegionDogLeg),
            Some("LegacyLm") => Some(rexafs::prelude::AUTOBKSolver::LegacyLm),
            Some("LinearDirect") => Some(rexafs::prelude::AUTOBKSolver::LinearDirect),
            Some(value) => {
                return Err(js_sys::Error::new(&format!("unknown AUTOBKSolver: {value}")).into())
            }
        };
        self.inner.solver = parsed;
        Ok(())
    }
    /// Legacy fallback solver. Default: TrustRegionDogLeg in Python, LegacyLm in Wasm; unused by FixedPenalty.
    #[wasm_bindgen(getter)]
    pub fn linear_fallback_solver(&self) -> Option<String> {
        self.inner.linear_fallback_solver.map(|v| format!("{v:?}"))
    }
    #[wasm_bindgen(setter)]
    pub fn set_linear_fallback_solver(&mut self, value: Option<String>) -> Result<(), JsValue> {
        let parsed = match value.as_deref() {
            None => None,
            Some("TrustRegionDogLeg") => Some(rexafs::prelude::AUTOBKSolver::TrustRegionDogLeg),
            Some("LegacyLm") => Some(rexafs::prelude::AUTOBKSolver::LegacyLm),
            Some("LinearDirect") => Some(rexafs::prelude::AUTOBKSolver::LinearDirect),
            Some(value) => {
                return Err(js_sys::Error::new(&format!("unknown AUTOBKSolver: {value}")).into())
            }
        };
        self.inner.linear_fallback_solver = parsed;
        Ok(())
    }
    /// Endpoint model. Recommended default: FixedPenalty with LinearDirect; Fixed and TwoPass are legacy models.
    #[wasm_bindgen(getter)]
    pub fn clamp_scale_policy(&self) -> Option<String> {
        self.inner.clamp_scale_policy.map(|v| format!("{v:?}"))
    }
    #[wasm_bindgen(setter)]
    pub fn set_clamp_scale_policy(&mut self, value: Option<String>) -> Result<(), JsValue> {
        let parsed = match value.as_deref() {
            None => None,
            Some("FixedPenalty") => Some(rexafs::prelude::AUTOBKClampScalePolicy::FixedPenalty),
            Some("Fixed") => Some(rexafs::prelude::AUTOBKClampScalePolicy::Fixed),
            Some("TwoPass") => Some(rexafs::prelude::AUTOBKClampScalePolicy::TwoPass),
            Some(value) => {
                return Err(
                    js_sys::Error::new(&format!("unknown AUTOBKClampScalePolicy: {value}")).into(),
                )
            }
        };
        self.inner.clamp_scale_policy = parsed;
        Ok(())
    }
}

/// Forward Fourier-transform settings. Defaults: k=2..15 inverse angstroms, kweight=2, KaiserBessel window.
#[wasm_bindgen(js_name = XrayFFTF)]
pub struct WasmXrayFFTF {
    inner: rexafs::XrayFFTF,
}
#[wasm_bindgen(js_class = XrayFFTF)]
impl WasmXrayFFTF {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: rexafs::XrayFFTF::new(),
        }
    }
    /// Sampling/window domain. Default: Input (existing k grid). Larch resamples on the extended FFT window grid.
    #[wasm_bindgen(getter)]
    pub fn grid(&self) -> String {
        format!("{:?}", self.inner.grid)
    }
    #[wasm_bindgen(setter)]
    pub fn set_grid(&mut self, value: &str) -> Result<(), JsValue> {
        self.inner.grid = match value {
            "Input" => rexafs::FFTGrid::Input,
            "Larch" => rexafs::FFTGrid::Larch,
            _ => return Err(JsValue::from_str("FFT grid must be Input or Larch")),
        };
        Ok(())
    }
    /// Maximum displayed R in angstroms. Default: 10.0; does not truncate the inverse-transform filter.
    #[wasm_bindgen(getter)]
    pub fn rmax_out(&self) -> Option<f64> {
        self.inner.rmax_out
    }
    #[wasm_bindgen(setter)]
    pub fn set_rmax_out(&mut self, value: Option<f64>) {
        self.inner.rmax_out = value;
    }
    /// Low-k taper width in inverse angstroms. Default: 1.0.
    #[wasm_bindgen(getter)]
    pub fn dk(&self) -> Option<f64> {
        self.inner.dk
    }
    #[wasm_bindgen(setter)]
    pub fn set_dk(&mut self, value: Option<f64>) {
        self.inner.dk = value;
    }
    /// High-k taper width in inverse angstroms. Default: use dk.
    #[wasm_bindgen(getter)]
    pub fn dk2(&self) -> Option<f64> {
        self.inner.dk2
    }
    #[wasm_bindgen(setter)]
    pub fn set_dk2(&mut self, value: Option<f64>) {
        self.inner.dk2 = value;
    }
    /// Lower Fourier window limit in inverse angstroms. Default: 2.0; None/undefined uses the first k sample.
    #[wasm_bindgen(getter)]
    pub fn kmin(&self) -> Option<f64> {
        self.inner.kmin
    }
    #[wasm_bindgen(setter)]
    pub fn set_kmin(&mut self, value: Option<f64>) {
        self.inner.kmin = value;
    }
    /// Upper Fourier window limit in inverse angstroms. Default: 15.0; None/undefined uses the last k sample.
    #[wasm_bindgen(getter)]
    pub fn kmax(&self) -> Option<f64> {
        self.inner.kmax
    }
    #[wasm_bindgen(setter)]
    pub fn set_kmax(&mut self, value: Option<f64>) {
        self.inner.kmax = value;
    }
    /// Power of k applied before FFT. Default: 2.0; nonnegative values are floored to an integer.
    #[wasm_bindgen(getter)]
    pub fn kweight(&self) -> Option<f64> {
        self.inner.kweight
    }
    #[wasm_bindgen(setter)]
    pub fn set_kweight(&mut self, value: Option<f64>) {
        self.inner.kweight = value;
    }
    /// Forward FFT length. Default: 2048.
    #[wasm_bindgen(getter)]
    pub fn nfft(&self) -> Option<usize> {
        self.inner.nfft
    }
    #[wasm_bindgen(setter)]
    pub fn set_nfft(&mut self, value: Option<usize>) {
        self.inner.nfft = value;
    }
    /// FFT k spacing in inverse angstroms. Default: infer from input k.
    #[wasm_bindgen(getter)]
    pub fn kstep(&self) -> Option<f64> {
        self.inner.kstep
    }
    #[wasm_bindgen(setter)]
    pub fn set_kstep(&mut self, value: Option<f64>) {
        self.inner.kstep = value;
    }
    /// Fourier window shape. Default: KaiserBessel; explicitly unset uses Hanning.
    #[wasm_bindgen(getter)]
    pub fn window(&self) -> Option<String> {
        self.inner.window.map(|v| format!("{v:?}"))
    }
    #[wasm_bindgen(setter)]
    pub fn set_window(&mut self, value: Option<String>) -> Result<(), JsValue> {
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
                return Err(js_sys::Error::new(&format!("unknown FTWindow: {value}")).into())
            }
        };
        self.inner.window = parsed;
        Ok(())
    }
}

/// Inverse Fourier-transform settings. Set rmin/rmax to select an R-space shell.
#[wasm_bindgen(js_name = XrayFFTR)]
pub struct WasmXrayFFTR {
    inner: rexafs::XrayFFTR,
}
#[wasm_bindgen(js_class = XrayFFTR)]
impl WasmXrayFFTR {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: rexafs::XrayFFTR::new(),
        }
    }
    /// Maximum back-transform q in inverse angstroms. Default: 10.0.
    #[wasm_bindgen(getter)]
    pub fn qmax_out(&self) -> Option<f64> {
        self.inner.qmax_out
    }
    #[wasm_bindgen(setter)]
    pub fn set_qmax_out(&mut self, value: Option<f64>) {
        self.inner.qmax_out = value;
    }
    /// Low-R taper width in angstroms. Default: 1.0.
    #[wasm_bindgen(getter)]
    pub fn dr(&self) -> Option<f64> {
        self.inner.dr
    }
    #[wasm_bindgen(setter)]
    pub fn set_dr(&mut self, value: Option<f64>) {
        self.inner.dr = value;
    }
    /// High-R taper width in angstroms. Default: use dr.
    #[wasm_bindgen(getter)]
    pub fn dr2(&self) -> Option<f64> {
        self.inner.dr2
    }
    #[wasm_bindgen(setter)]
    pub fn set_dr2(&mut self, value: Option<f64>) {
        self.inner.dr2 = value;
    }
    /// Lower inverse-transform window limit in angstroms. Default: 0.0.
    #[wasm_bindgen(getter)]
    pub fn rmin(&self) -> Option<f64> {
        self.inner.rmin
    }
    #[wasm_bindgen(setter)]
    pub fn set_rmin(&mut self, value: Option<f64>) {
        self.inner.rmin = value;
    }
    /// Upper inverse-transform window limit in angstroms. Default: 20.0; choose a shell range for R filtering.
    #[wasm_bindgen(getter)]
    pub fn rmax(&self) -> Option<f64> {
        self.inner.rmax
    }
    #[wasm_bindgen(setter)]
    pub fn set_rmax(&mut self, value: Option<f64>) {
        self.inner.rmax = value;
    }
    /// Power of R applied before IFFT. Default: 0.0; nonnegative values are floored to an integer.
    #[wasm_bindgen(getter)]
    pub fn rweight(&self) -> Option<f64> {
        self.inner.rweight
    }
    #[wasm_bindgen(setter)]
    pub fn set_rweight(&mut self, value: Option<f64>) {
        self.inner.rweight = value;
    }
    /// Inverse FFT length. Default: 2048; leave kstep automatic when changing this.
    #[wasm_bindgen(getter)]
    pub fn nfft(&self) -> Option<usize> {
        self.inner.nfft
    }
    #[wasm_bindgen(setter)]
    pub fn set_nfft(&mut self, value: Option<usize>) {
        self.inner.nfft = value;
    }
    /// Output q spacing in inverse angstroms. Default: infer from input R and nfft; an explicit value must match that spacing.
    #[wasm_bindgen(getter)]
    pub fn kstep(&self) -> Option<f64> {
        self.inner.kstep
    }
    #[wasm_bindgen(setter)]
    pub fn set_kstep(&mut self, value: Option<f64>) {
        self.inner.kstep = value;
    }
    /// Inverse Fourier window shape. Default: KaiserBessel; explicitly unset uses Hanning.
    #[wasm_bindgen(getter)]
    pub fn window(&self) -> Option<String> {
        self.inner.window.map(|v| format!("{v:?}"))
    }
    #[wasm_bindgen(setter)]
    pub fn set_window(&mut self, value: Option<String>) -> Result<(), JsValue> {
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
                return Err(js_sys::Error::new(&format!("unknown FTWindow: {value}")).into())
            }
        };
        self.inner.window = parsed;
        Ok(())
    }
}

#[wasm_bindgen(js_name = NormalizationMethod)]
pub struct WasmNormalizationMethod {
    inner: rexafs::NormalizationMethod,
}
#[wasm_bindgen(js_class = NormalizationMethod)]
impl WasmNormalizationMethod {
    /// Pre/post-edge normalization settings. Defaults adapt to the measured energy range.
    #[wasm_bindgen(js_name = PrePostEdge)]
    pub fn configured(parameters: &WasmPrePostEdge) -> Self {
        Self {
            inner: rexafs::NormalizationMethod::PrePostEdge(parameters.inner.clone()),
        }
    }
    pub fn new_prepostedge() -> Self {
        Self {
            inner: rexafs::NormalizationMethod::new_prepostedge(),
        }
    }
    pub fn new_mback() -> Self {
        Self {
            inner: rexafs::NormalizationMethod::new_mback(),
        }
    }
}

#[wasm_bindgen(js_name = BackgroundMethod)]
pub struct WasmBackgroundMethod {
    inner: rexafs::BackgroundMethod,
}
#[wasm_bindgen(js_class = BackgroundMethod)]
impl WasmBackgroundMethod {
    /// AUTOBK background settings. Recommended defaults use LinearDirect and FixedPenalty with lambda 0.001.
    #[wasm_bindgen(js_name = AUTOBK)]
    pub fn configured(parameters: &WasmAUTOBK) -> Self {
        Self {
            inner: rexafs::BackgroundMethod::AUTOBK(parameters.inner.clone()),
        }
    }
    pub fn new_autobk() -> Self {
        Self {
            inner: rexafs::BackgroundMethod::new_autobk(),
        }
    }
    pub fn new_ilpbkg() -> Self {
        Self {
            inner: rexafs::BackgroundMethod::new_ilpbkg(),
        }
    }
}

#[wasm_bindgen(js_name = Spectrum)]
pub struct WasmSpectrum {
    inner: rexafs::Spectrum,
}
#[wasm_bindgen(js_class = Spectrum)]
impl WasmSpectrum {
    #[wasm_bindgen(constructor)]
    pub fn new(energy: &[f64], mu: &[f64]) -> Result<Self, JsValue> {
        Ok(Self {
            inner: rexafs::Spectrum::from_arrays(energy, mu).map_err(error)?,
        })
    }
    pub fn from_arrays(energy: &[f64], mu: &[f64]) -> Result<Self, JsValue> {
        Self::new(energy, mu)
    }
    pub fn set_spectrum(&mut self, energy: &[f64], mu: &[f64]) -> Result<(), JsValue> {
        let input = rexafs::Spectrum::from_arrays(energy, mu).map_err(error)?;
        self.inner
            .set_spectrum(input.energy.unwrap(), input.mu.unwrap());
        Ok(())
    }
    pub fn set_e0(&mut self, e0: f64) {
        self.inner.set_e0(e0);
    }
    pub fn e0(&self) -> Option<f64> {
        self.inner.e0()
    }
    pub fn invalidate_derived(&mut self) {
        self.inner.invalidate_derived();
    }
    pub fn set_ifft(&mut self, parameters: &WasmXrayFFTR) {
        self.inner.set_ifft(parameters.inner.clone());
    }
    pub fn set_fft(&mut self, parameters: &WasmXrayFFTF) {
        self.inner.set_fft(parameters.inner.clone());
    }
    pub fn set_normalization_method(
        &mut self,
        method: &WasmNormalizationMethod,
    ) -> Result<(), JsValue> {
        self.inner
            .set_normalization_method(Some(method.inner.clone()))
            .map(|_| ())
            .map_err(error)
    }
    pub fn set_background_method(&mut self, method: &WasmBackgroundMethod) -> Result<(), JsValue> {
        self.inner
            .set_background_method(Some(method.inner.clone()))
            .map(|_| ())
            .map_err(error)
    }
    pub fn find_e0(&mut self) -> Result<(), JsValue> {
        self.inner.find_e0().map(|_| ()).map_err(error)
    }
    pub fn normalize(&mut self) -> Result<(), JsValue> {
        self.inner.normalize().map(|_| ()).map_err(error)
    }
    pub fn calc_background(&mut self) -> Result<(), JsValue> {
        self.inner.calc_background().map(|_| ()).map_err(error)
    }
    pub fn fft(&mut self) -> Result<(), JsValue> {
        self.inner.fft().map(|_| ()).map_err(error)
    }
    pub fn ifft(&mut self) -> Result<(), JsValue> {
        self.inner.ifft().map(|_| ()).map_err(error)
    }
    pub fn k(&self) -> Option<js_sys::Float64Array> {
        self.inner.k().map(|v| js_sys::Float64Array::from(v))
    }
    pub fn chi(&self) -> Option<js_sys::Float64Array> {
        self.inner.chi().map(|v| js_sys::Float64Array::from(v))
    }
    pub fn norm(&self) -> Option<js_sys::Float64Array> {
        self.inner
            .norm()
            .map(|v| js_sys::Float64Array::from(v.as_slice()))
    }
    pub fn flat(&self) -> Option<js_sys::Float64Array> {
        self.inner
            .flat()
            .map(|v| js_sys::Float64Array::from(v.as_slice()))
    }
    pub fn pre_edge(&self) -> Option<js_sys::Float64Array> {
        self.inner
            .pre_edge()
            .map(|v| js_sys::Float64Array::from(v.as_slice()))
    }
    pub fn post_edge(&self) -> Option<js_sys::Float64Array> {
        self.inner
            .post_edge()
            .map(|v| js_sys::Float64Array::from(v.as_slice()))
    }
    pub fn r(&self) -> Option<js_sys::Float64Array> {
        self.inner
            .r()
            .map(|v| js_sys::Float64Array::from(v.as_slice()))
    }
    pub fn kwin(&self) -> Option<js_sys::Float64Array> {
        self.inner
            .kwin()
            .map(|v| js_sys::Float64Array::from(v.as_slice()))
    }
    pub fn kwin_k(&self) -> Option<js_sys::Float64Array> {
        self.inner
            .kwin_k()
            .map(|v| js_sys::Float64Array::from(v.as_slice()))
    }
    pub fn chir_mag(&self) -> Option<js_sys::Float64Array> {
        self.inner
            .chir_mag()
            .map(|v| js_sys::Float64Array::from(v.as_slice()))
    }
    pub fn chir_real(&self) -> Option<js_sys::Float64Array> {
        self.inner
            .chir_real()
            .map(|v| js_sys::Float64Array::from(v.as_slice()))
    }
    pub fn chir_imag(&self) -> Option<js_sys::Float64Array> {
        self.inner
            .chir_imag()
            .map(|v| js_sys::Float64Array::from(v.as_slice()))
    }
    pub fn q(&self) -> Option<js_sys::Float64Array> {
        self.inner
            .q()
            .map(|v| js_sys::Float64Array::from(v.as_slice()))
    }
    pub fn chiq(&self) -> Option<js_sys::Float64Array> {
        self.inner
            .chiq()
            .map(|v| js_sys::Float64Array::from(v.as_slice()))
    }
}
