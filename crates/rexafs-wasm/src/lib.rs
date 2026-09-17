//! Thin Wasm bindings: stage execution and defaults live in rexafs.
use wasm_bindgen::prelude::*;
mod fluorescence;
mod mback;
mod peaks;
mod wavelet;
fn error(error: rexafs::Error) -> JsValue {
    js_sys::Error::new(&error.to_string()).into()
}

/// Settings for absorption-edge normalization and flattening.
///
/// The pre-edge fit estimates the smooth baseline before the edge. The post-edge fit estimates
/// the edge step and the trend used by flat(). All energy bounds are offsets from E0 in eV, not
/// absolute energies. Undefined fields select data-dependent values where described; inspect
/// the measured fit regions before interpreting a normalized spectrum.
///
/// Settings are copied when assigned to a spectrum. Changing this object afterwards
/// requires assigning it again; free() releases only this object's Wasm allocation.
///
/// Background and normalization conventions are explained in [Newville, Fundamentals of
/// XAFS](https://docs.xrayabsorption.org/tutorials/XAFS_Fundamentals.pdf). Automatic range and
/// polynomial rules are rexafs implementation choices in
/// [PrePostEdge](https://github.com/Ameyanagi/rexafs/blob/main/crates/rexafs/src/xafs/normalization.rs).
#[wasm_bindgen(js_name = PrePostEdge)]
pub struct WasmPrePostEdge {
    inner: rexafs::PrePostEdge,
}
impl Default for WasmPrePostEdge {
    fn default() -> Self {
        Self::new()
    }
}
#[wasm_bindgen(js_class = PrePostEdge)]
impl WasmPrePostEdge {
    /// Create owned settings with the recommended Rust defaults. Automatic fields are resolved
    /// on the spectrum's copy during processing; resolved values are not written back into
    /// the original settings object. Resolved values stay in the spectrum's settings until
    /// those settings are replaced.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: rexafs::PrePostEdge::new(),
        }
    }
    /// Lower pre-edge fit bound, as an energy offset from E0 in eV. Default: undefined selects
    /// a rounded estimate near the beginning of the measured range. Choose a region below the
    /// absorption edge without other edges or glitches; this region determines the subtracted
    /// baseline.
    #[wasm_bindgen(getter)]
    pub fn pre_edge_start(&self) -> Option<f64> {
        self.inner.pre_edge_start
    }
    /// Store the pre_edge_start setting; see [`Self::pre_edge_start`] for units, defaults and
    /// effects. Configurations previously copied into a spectrum are unchanged.
    #[wasm_bindgen(setter)]
    pub fn set_pre_edge_start(&mut self, value: Option<f64>) {
        self.inner.pre_edge_start = value;
    }
    /// Upper pre-edge fit bound relative to E0, in eV. Default: undefined derives a rounded
    /// value from pre_edge_start. The pre-edge region is normally below E0 (negative offsets);
    /// moving it toward the edge can include near-edge structure in the baseline fit.
    #[wasm_bindgen(getter)]
    pub fn pre_edge_end(&self) -> Option<f64> {
        self.inner.pre_edge_end
    }
    /// Store the pre_edge_end setting; see [`Self::pre_edge_end`] for units, defaults and
    /// effects. Configurations previously copied into a spectrum are unchanged.
    #[wasm_bindgen(setter)]
    pub fn set_pre_edge_end(&mut self, value: Option<f64>) {
        self.inner.pre_edge_end = value;
    }
    /// Lower post-edge fit bound relative to E0, in eV. Default: undefined derives it from
    /// norm_end, normally capped at 25 eV and kept at least 10 eV below norm_end. This region
    /// estimates the absorption edge step and the trend removed by flat().
    #[wasm_bindgen(getter)]
    pub fn norm_start(&self) -> Option<f64> {
        self.inner.norm_start
    }
    /// Store the norm_start setting; see [`Self::norm_start`] for units, defaults and effects.
    /// Configurations previously copied into a spectrum are unchanged.
    #[wasm_bindgen(setter)]
    pub fn set_norm_start(&mut self, value: Option<f64>) {
        self.inner.norm_start = value;
    }
    /// Upper post-edge fit bound relative to E0, in eV. Default: undefined rounds the available
    /// post-edge extent to a nearby 5 eV boundary, without extending beyond the measured range.
    /// Choose the range before a second absorption edge if one is present.
    #[wasm_bindgen(getter)]
    pub fn norm_end(&self) -> Option<f64> {
        self.inner.norm_end
    }
    /// Store the norm_end setting; see [`Self::norm_end`] for units, defaults and effects.
    /// Configurations previously copied into a spectrum are unchanged.
    #[wasm_bindgen(setter)]
    pub fn set_norm_end(&mut self, value: Option<f64>) {
        self.inner.norm_end = value;
    }
    /// Integer degree of the polynomial fitted to pre-edge-subtracted absorption above E0.
    /// Default: undefined selects 0, 1 or 2 for post-edge fit spans below 50, below 350, or at
    /// least 350 eV. Values are clamped to 0 through 5. Higher order follows more curvature but
    /// can absorb spectral structure.
    #[wasm_bindgen(getter)]
    pub fn norm_polyorder(&self) -> Option<i32> {
        self.inner.norm_polyorder
    }
    /// Store the norm_polyorder setting; see [`Self::norm_polyorder`] for units, defaults and
    /// effects. Configurations previously copied into a spectrum are unchanged.
    #[wasm_bindgen(setter)]
    pub fn set_norm_polyorder(&mut self, value: Option<i32>) {
        self.inner.norm_polyorder = value;
    }
    /// Integer energy exponent used to model the pre-edge baseline. Default: undefined resolves
    /// to 0, a straight-line fit to mu(E). For exponent n, the code fits a line to mu(E) * E^n
    /// and divides the fitted line by E^n; E is energy in eV. Use 0 unless this additional
    /// energy dependence is justified.
    #[wasm_bindgen(getter)]
    pub fn n_victoreen(&self) -> Option<i32> {
        self.inner.n_victoreen
    }
    /// Store the n_victoreen setting; see [`Self::n_victoreen`] for units, defaults and
    /// effects. Configurations previously copied into a spectrum are unchanged.
    #[wasm_bindgen(setter)]
    pub fn set_n_victoreen(&mut self, value: Option<i32>) {
        self.inner.n_victoreen = value;
    }
    /// Absorption-edge energy in eV. Default: undefined uses the spectrum E0 if assigned,
    /// otherwise derivative-based edge detection. E0 sets the origin of the fit offsets and the
    /// subsequent energy-to-k conversion. A supplied value must be finite and strictly between
    /// the measured energy endpoints; spectrum processing throws an Error otherwise.
    #[wasm_bindgen(getter)]
    pub fn e0(&self) -> Option<f64> {
        self.inner.e0
    }
    /// Store the e0 setting; see [`Self::e0`] for units, defaults and effects. Configurations
    /// previously copied into a spectrum are unchanged.
    #[wasm_bindgen(setter)]
    pub fn set_e0(&mut self, value: Option<f64>) {
        self.inner.e0 = value;
    }
    /// Absorption step used as the normalization divisor, in the same units as mu. Default:
    /// undefined estimates post_edge - pre_edge at the input sample nearest E0. Use a positive
    /// physical step; the implementation floors finite values below 1e-12 to 1e-12 and rejects
    /// nonfinite steps. Changing it rescales norm() and chi().
    #[wasm_bindgen(getter)]
    pub fn edge_step(&self) -> Option<f64> {
        self.inner.edge_step
    }
    /// Store the edge_step setting; see [`Self::edge_step`] for units, defaults and effects.
    /// Configurations previously copied into a spectrum are unchanged.
    #[wasm_bindgen(setter)]
    pub fn set_edge_step(&mut self, value: Option<f64>) {
        self.inner.edge_step = value;
    }
}

/// Settings for extracting extended X-ray absorption fine structure, chi(k), with a cubic
/// spline background in photoelectron wavenumber k.
///
/// AUTOBK separates slowly varying atomic absorption from oscillations associated with
/// neighboring atoms by suppressing low-R Fourier residuals. Recommended starting values are
/// rbkg=1 angstrom, kstep=0.05 inverse angstroms, kweight=1, Hanning, and LinearDirect with
/// FixedPenalty and clamp_lambda=0.001. Choose rbkg below structural signal; excessive
/// background flexibility can remove that signal.
///
/// The original method is [Newville et al. (1993)](https://doi.org/10.1103/PhysRevB.47.14126).
/// The fixed endpoint penalty and its default strength are rexafs-specific choices, implemented
/// in
/// [background/fixed.rs](https://github.com/Ameyanagi/rexafs/blob/main/crates/rexafs/src/xafs/background/fixed.rs);
/// they are not part of that paper's objective.
///
/// The spectrum copies assigned background settings. Reassign after editing, and release
/// your settings with free() when finished. A processing failure, including an ill-conditioned
/// fixed-penalty fit, throws an Error without silently switching objectives.
#[wasm_bindgen(js_name = AUTOBK)]
pub struct WasmAUTOBK {
    inner: rexafs::AUTOBK,
}
impl Default for WasmAUTOBK {
    fn default() -> Self {
        Self::new()
    }
}
#[wasm_bindgen(js_class = AUTOBK)]
impl WasmAUTOBK {
    /// Create owned settings with the recommended Rust defaults. Automatic fields are resolved
    /// on the spectrum's copy during processing; resolved values are not written back into
    /// the original settings object. Resolved scalar defaults and ek0 are retained in the
    /// spectrum. Automatic kmax and nknots remain unset in stored settings and are calculated
    /// locally for each input.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: rexafs::AUTOBK::new(),
        }
    }
    /// Edge energy used for the energy-to-k conversion, in eV. Default: undefined uses
    /// normalization E0. An in-range override changes the background k grid without redefining
    /// the pre/post-edge fitting regions. Prefer a consistent E0 across both stages; an
    /// out-of-range override is discarded.
    #[wasm_bindgen(getter)]
    pub fn ek0(&self) -> Option<f64> {
        self.inner.ek0
    }
    /// Store the ek0 setting; see [`Self::ek0`] for units, defaults and effects. Configurations
    /// previously copied into a spectrum are unchanged.
    #[wasm_bindgen(setter)]
    pub fn set_ek0(&mut self, value: Option<f64>) {
        self.inner.ek0 = value;
    }
    /// Background cutoff in angstroms. Default: 1.0; undefined restores this default. AUTOBK
    /// minimizes low-R Fourier components of the residual below a cutoff derived from this
    /// value. Increasing rbkg allows a more flexible background and can remove real first-shell
    /// signal; start below the first structural peak.
    #[wasm_bindgen(getter)]
    pub fn rbkg(&self) -> Option<f64> {
        self.inner.rbkg
    }
    /// Store the rbkg setting; see [`Self::rbkg`] for units, defaults and effects.
    /// Configurations previously copied into a spectrum are unchanged.
    #[wasm_bindgen(setter)]
    pub fn set_rbkg(&mut self, value: Option<f64>) {
        self.inner.rbkg = value;
    }
    /// Number of spline control points used to represent the smooth background. Default:
    /// undefined derives the count from rbkg and the fitted k interval; the resolved count is
    /// clamped to 5 through 128. More points add flexibility and can overfit structure. This is
    /// not the length of the repeated endpoint knot vector.
    #[wasm_bindgen(getter)]
    pub fn nknots(&self) -> Option<i32> {
        self.inner.nknots
    }
    /// Store the nknots setting; see [`Self::nknots`] for units, defaults and effects.
    /// Configurations previously copied into a spectrum are unchanged.
    #[wasm_bindgen(setter)]
    pub fn set_nknots(&mut self, value: Option<i32>) {
        self.inner.nknots = value;
    }
    /// Lower background fit/window bound in inverse angstroms. Default: 0.0; undefined restores
    /// this default. Require kmin < the usable kmax. Raising this bound excludes the near-edge
    /// region from the Fourier objective; the returned k() grid still begins at zero.
    #[wasm_bindgen(getter)]
    pub fn kmin(&self) -> Option<f64> {
        self.inner.kmin
    }
    /// Store the kmin setting; see [`Self::kmin`] for units, defaults and effects.
    /// Configurations previously copied into a spectrum are unchanged.
    #[wasm_bindgen(setter)]
    pub fn set_kmin(&mut self, value: Option<f64>) {
        self.inner.kmin = value;
    }
    /// Upper background fit/window bound in inverse angstroms. Default: undefined uses the
    /// available data limit and explicit values are capped at that limit. Reducing it can
    /// exclude noisy high-k data and shortens the returned k()/chi() arrays.
    #[wasm_bindgen(getter)]
    pub fn kmax(&self) -> Option<f64> {
        self.inner.kmax
    }
    /// Store the kmax setting; see [`Self::kmax`] for units, defaults and effects.
    /// Configurations previously copied into a spectrum are unchanged.
    #[wasm_bindgen(setter)]
    pub fn set_kmax(&mut self, value: Option<f64>) {
        self.inner.kmax = value;
    }
    /// Spacing of the uniform output k() grid in inverse angstroms. Default: 0.05; undefined
    /// restores this default. Must be finite and positive. Smaller spacing interpolates the
    /// same measured data more densely and changes the internal R sampling; it does not add
    /// experimental resolution.
    #[wasm_bindgen(getter)]
    pub fn kstep(&self) -> Option<f64> {
        self.inner.kstep
    }
    /// Store the kstep setting; see [`Self::kstep`] for units, defaults and effects.
    /// Configurations previously copied into a spectrum are unchanged.
    #[wasm_bindgen(setter)]
    pub fn set_kstep(&mut self, value: Option<f64>) {
        self.inner.kstep = value;
    }
    /// Number of samples at each enabled endpoint used to discourage large residual chi values.
    /// Default: 3; undefined restores this default. FixedPenalty requires a nonnegative
    /// integer, caps the count at the available samples, and includes the last high-k sample.
    /// Use 0 to disable endpoint clamping.
    #[wasm_bindgen(getter)]
    pub fn nclamp(&self) -> Option<i32> {
        self.inner.nclamp
    }
    /// Store the nclamp setting; see [`Self::nclamp`] for units, defaults and effects.
    /// Configurations previously copied into a spectrum are unchanged.
    #[wasm_bindgen(setter)]
    pub fn set_nclamp(&mut self, value: Option<i32>) {
        self.inner.nclamp = value;
    }
    /// Integer multiplier for the low-k endpoint residuals. Default: 0, which disables that
    /// endpoint. In FixedPenalty the absolute value multiplies each residual, so its square
    /// weights the objective. The active low-k samples begin at k=0 on the output grid.
    #[wasm_bindgen(getter)]
    pub fn clamp_lo(&self) -> Option<i32> {
        self.inner.clamp_lo
    }
    /// Store the clamp_lo setting; see [`Self::clamp_lo`] for units, defaults and effects.
    /// Configurations previously copied into a spectrum are unchanged.
    #[wasm_bindgen(setter)]
    pub fn set_clamp_lo(&mut self, value: Option<i32>) {
        self.inner.clamp_lo = value;
    }
    /// Integer multiplier for the high-k endpoint residuals. Default: 1. In FixedPenalty the
    /// absolute value multiplies each residual, so doubling it quadruples that endpoint
    /// contribution before averaging. Use 0 to disable the high-k endpoint penalty.
    #[wasm_bindgen(getter)]
    pub fn clamp_hi(&self) -> Option<i32> {
        self.inner.clamp_hi
    }
    /// Store the clamp_hi setting; see [`Self::clamp_hi`] for units, defaults and effects.
    /// Configurations previously copied into a spectrum are unchanged.
    #[wasm_bindgen(setter)]
    pub fn set_clamp_hi(&mut self, value: Option<i32>) {
        self.inner.clamp_hi = value;
    }
    /// Numerical strength of the FixedPenalty endpoint term. Recommended default: 0.001;
    /// undefined restores this default. The objective adds lambda times the mean squared
    /// active, weighted endpoint chi residual to the mean squared low-R residual. Require a
    /// finite nonnegative value; 0 disables the endpoint term.
    ///
    /// This empirical balance is tied to the implemented residual convention: the FixedPenalty
    /// Fourier residual uses the fixed numerical factor 0.05/sqrt(pi), while the endpoint
    /// residual uses unweighted, edge-step-normalized chi. Changing kweight or the window
    /// changes the balance at a fixed lambda. The parameter is unused by legacy endpoint
    /// policies; it is not a universal physical constant. See [the fixed-penalty
    /// objective](https://rexafs.com/docs/science/autobk/).
    #[wasm_bindgen(getter)]
    pub fn clamp_lambda(&self) -> Option<f64> {
        self.inner.clamp_lambda
    }
    /// Store the clamp_lambda setting; see [`Self::clamp_lambda`] for units, defaults and
    /// effects. Configurations previously copied into a spectrum are unchanged.
    #[wasm_bindgen(setter)]
    pub fn set_clamp_lambda(&mut self, value: Option<f64>) {
        self.inner.clamp_lambda = value;
    }
    /// FFT length used inside background removal. Default: 2048; undefined restores this
    /// default. Use a positive integer large enough to contain the prepared k grid. It sets the
    /// internal R spacing together with kstep; increasing zero-padding refines that grid
    /// without adding measured information.
    #[wasm_bindgen(getter)]
    pub fn nfft(&self) -> Option<i32> {
        self.inner.nfft
    }
    /// Store the nfft setting; see [`Self::nfft`] for units, defaults and effects.
    /// Configurations previously copied into a spectrum are unchanged.
    #[wasm_bindgen(setter)]
    pub fn set_nfft(&mut self, value: Option<i32>) {
        self.inner.nfft = value;
    }
    /// Integer power of k applied in the background Fourier objective. Default: 1; undefined
    /// restores this default. Larger powers emphasize high-k residuals and their noise. The
    /// returned chi() remains unweighted; the independent XrayFFTF.kweight controls the later
    /// displayed transform.
    #[wasm_bindgen(getter)]
    pub fn kweight(&self) -> Option<i32> {
        self.inner.kweight
    }
    /// Store the kweight setting; see [`Self::kweight`] for units, defaults and effects.
    /// Configurations previously copied into a spectrum are unchanged.
    #[wasm_bindgen(setter)]
    pub fn set_kweight(&mut self, value: Option<i32>) {
        self.inner.kweight = value;
    }
    /// Background window parameter. Default: 0.1; undefined restores this default. For the
    /// default Hanning window it controls endpoint taper widths in inverse angstroms.
    /// KaiserBessel also uses this numeric value as a dimensionless shape parameter; equal dk
    /// does not make different window families equivalent.
    #[wasm_bindgen(getter)]
    pub fn dk(&self) -> Option<f64> {
        self.inner.dk
    }
    /// Store the dk setting; see [`Self::dk`] for units, defaults and effects. Configurations
    /// previously copied into a spectrum are unchanged.
    #[wasm_bindgen(setter)]
    pub fn set_dk(&mut self, value: Option<f64>) {
        self.inner.dk = value;
    }
    /// Ridge strength for the legacy LinearDirect objective. Default: 0.0001; undefined
    /// restores this default. Unused by the recommended FixedPenalty model, which solves the
    /// specified endpoint-penalized least-squares problem without this additional ridge term.
    /// Change only when reproducing a legacy calculation.
    #[wasm_bindgen(getter)]
    pub fn linear_regularization(&self) -> Option<f64> {
        self.inner.linear_regularization
    }
    /// Store the linear_regularization setting; see [`Self::linear_regularization`] for units,
    /// defaults and effects. Configurations previously copied into a spectrum are unchanged.
    #[wasm_bindgen(setter)]
    pub fn set_linear_regularization(&mut self, value: Option<f64>) {
        self.inner.linear_regularization = value;
    }
    /// Largest accepted condition number of the linear system. Default: 1e8; undefined restores
    /// this default. FixedPenalty applies this limit to the column-scaled design matrix and
    /// requires a finite value of at least 1. An ill-conditioned or rank-deficient solve throws
    /// instead of silently changing the objective.
    #[wasm_bindgen(getter)]
    pub fn linear_condition_limit(&self) -> Option<f64> {
        self.inner.linear_condition_limit
    }
    /// Store the linear_condition_limit setting; see [`Self::linear_condition_limit`] for
    /// units, defaults and effects. Configurations previously copied into a spectrum are
    /// unchanged.
    #[wasm_bindgen(setter)]
    pub fn set_linear_condition_limit(&mut self, value: Option<f64>) {
        self.inner.linear_condition_limit = value;
    }
    /// Acceptance threshold for comparing the legacy direct solution residual against its
    /// reference residual. Default: 1.05; undefined restores this default. Unused by
    /// FixedPenalty. This is a numerical fallback criterion, not a statistical uncertainty or
    /// goodness-of-fit probability.
    #[wasm_bindgen(getter)]
    pub fn linear_residual_ratio_limit(&self) -> Option<f64> {
        self.inner.linear_residual_ratio_limit
    }
    /// Store the linear_residual_ratio_limit setting; see [`Self::linear_residual_ratio_limit`]
    /// for units, defaults and effects. Configurations previously copied into a spectrum are
    /// unchanged.
    #[wasm_bindgen(setter)]
    pub fn set_linear_residual_ratio_limit(&mut self, value: Option<f64>) {
        self.inner.linear_residual_ratio_limit = value;
    }
    /// Allow the legacy direct solver to retry with linear_fallback_solver if its checks fail.
    /// Default: true; undefined restores this default. Despite the historical name, the chosen
    /// fallback need not be Levenberg-Marquardt. FixedPenalty never falls back and ignores this
    /// flag.
    #[wasm_bindgen(getter)]
    pub fn linear_fallback_to_lm(&self) -> Option<bool> {
        self.inner.linear_fallback_to_lm
    }
    /// Store the linear_fallback_to_lm setting; see [`Self::linear_fallback_to_lm`] for units,
    /// defaults and effects. Configurations previously copied into a spectrum are unchanged.
    #[wasm_bindgen(setter)]
    pub fn set_linear_fallback_to_lm(&mut self, value: Option<bool>) {
        self.inner.linear_fallback_to_lm = value;
    }
    /// Reuse compatible spline geometry, Fourier operators and matrix factorization. Default:
    /// true; undefined restores this default. Each spectrum still supplies new data and
    /// receives a new solution, with condition checks repeated. Disabling this changes reuse
    /// and runtime, not the specified objective.
    #[wasm_bindgen(getter)]
    pub fn linear_workspace_cache(&self) -> Option<bool> {
        self.inner.linear_workspace_cache
    }
    /// Store the linear_workspace_cache setting; see [`Self::linear_workspace_cache`] for
    /// units, defaults and effects. Configurations previously copied into a spectrum are
    /// unchanged.
    #[wasm_bindgen(setter)]
    pub fn set_linear_workspace_cache(&mut self, value: Option<bool>) {
        self.inner.linear_workspace_cache = value;
    }
    /// Window family used inside the background Fourier objective. Default: Hanning; undefined
    /// restores Hanning. The taper reduces ringing at the selected k boundaries. This setting
    /// is independent of the later XrayFFTF window; see FTWindow for the accepted names.
    #[wasm_bindgen(getter)]
    pub fn window(&self) -> Option<String> {
        Some(format!("{:?}", self.inner.window))
    }
    /// Store the window setting; see [`Self::window`] for units, defaults and effects.
    /// Configurations previously copied into a spectrum are unchanged.
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
    /// Algorithm used to determine background spline coefficients. Recommended default:
    /// LinearDirect, required by FixedPenalty; undefined restores it. LegacyLm solves the
    /// legacy objective iteratively. TrustRegionDogLeg requires a native Rust feature and
    /// throws when processing in the published Wasm package.
    #[wasm_bindgen(getter)]
    pub fn solver(&self) -> Option<String> {
        self.inner.solver.map(|v| format!("{v:?}"))
    }
    /// Store the solver setting; see [`Self::solver`] for units, defaults and effects.
    /// Configurations previously copied into a spectrum are unchanged.
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
    /// Solver used only when an enabled legacy LinearDirect fallback is needed. Default in
    /// Wasm: LegacyLm; undefined also resolves to LegacyLm with the default fallback flag.
    /// FixedPenalty ignores this option. TrustRegionDogLeg is unavailable in Wasm, and
    /// LinearDirect cannot be its own fallback.
    #[wasm_bindgen(getter)]
    pub fn linear_fallback_solver(&self) -> Option<String> {
        self.inner.linear_fallback_solver.map(|v| format!("{v:?}"))
    }
    /// Store the linear_fallback_solver setting; see [`Self::linear_fallback_solver`] for
    /// units, defaults and effects. Configurations previously copied into a spectrum are
    /// unchanged.
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
    /// Endpoint penalty model. Recommended default: FixedPenalty; undefined restores it.
    /// FixedPenalty uses a fixed mean-square endpoint term and requires LinearDirect. Fixed and
    /// TwoPass preserve older residual-dependent scaling choices for reproducing historical
    /// results; they are different objectives.
    #[wasm_bindgen(getter)]
    pub fn clamp_scale_policy(&self) -> Option<String> {
        self.inner.clamp_scale_policy.map(|v| format!("{v:?}"))
    }
    /// Store the clamp_scale_policy setting; see [`Self::clamp_scale_policy`] for units,
    /// defaults and effects. Configurations previously copied into a spectrum are unchanged.
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

/// Settings for converting weighted, windowed chi(k) into complex chi(R).
///
/// Recommended starting values: kmin=2, kmax=15 inverse angstroms, kweight=2, KaiserBessel,
/// dk=1 and nfft=2048. Adapt the k interval to the useful measured data. The default Input grid
/// preserves the background grid; automatic kstep normally resolves to AUTOBK's 0.05 inverse
/// angstroms.
///
/// For a uniform zero-origin grid `k[j]=j*kstep`, prepare
/// `g[j] = chi(k[j]) * k[j]^w * window[j]`. The code computes `chiR[m] = (kstep /
/// sqrt(pi)) * sum_j g[j] * exp(-2*pi*i*j*m/N)`, with N=nfft, w=kweight, i^2=-1 and
/// `R[m]=pi*m/(N*kstep)`. Here j indexes prepared k samples and m indexes nonnegative
/// Fourier bins. The sum uses the first N prepared samples and zeros for missing
/// samples. There is no 1/N forward normalization or window-area correction. For dimensionless
/// chi, chi(R) has units angstrom^(-(w+1)).
///
/// This is the [NumPy unnormalized forward DFT
/// convention](https://numpy.org/doc/stable/reference/routines.fft.html#implementation-details)
/// with the explicit factor in
/// [xftf_fast_nalgebra](https://github.com/Ameyanagi/rexafs/blob/main/crates/rexafs/src/xafs/xrayfft.rs).
/// Scattering phases shift the peaks, so R is not automatically a phase-corrected bond
/// distance; see [Rehr and Albers (2000)](https://doi.org/10.1103/RevModPhys.72.621).
///
/// set_fft() copies settings. Reassign after editing, and free() this object when finished.
#[wasm_bindgen(js_name = XrayFFTF)]
pub struct WasmXrayFFTF {
    inner: rexafs::XrayFFTF,
}
impl Default for WasmXrayFFTF {
    fn default() -> Self {
        Self::new()
    }
}
#[wasm_bindgen(js_class = XrayFFTF)]
impl WasmXrayFFTF {
    /// Create owned settings with the recommended Rust defaults. Automatic fields are resolved
    /// on the spectrum's copy during processing; resolved values are not written back into
    /// the original settings object. Resolved values stay in the spectrum's settings until
    /// those settings are replaced.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: rexafs::XrayFFTF::new(),
        }
    }
    /// Sampling and window-construction convention. Default: Input preserves the prepared
    /// background k grid. Larch linearly resamples onto a zero-origin grid and extends the
    /// window domain as needed. Neither changes the returned background k()/chi(); always pair
    /// kwin() with kwin_k().
    #[wasm_bindgen(getter)]
    pub fn grid(&self) -> String {
        format!("{:?}", self.inner.grid)
    }
    /// Store the grid setting; see [`Self::grid`] for units, defaults and effects.
    /// Configurations previously copied into a spectrum are unchanged.
    /// Names are case-sensitive. An unsupported name returns a JavaScript string exception
    /// without changing the current grid.
    #[wasm_bindgen(setter)]
    pub fn set_grid(&mut self, value: &str) -> Result<(), JsValue> {
        self.inner.grid = match value {
            "Input" => rexafs::FFTGrid::Input,
            "Larch" => rexafs::FFTGrid::Larch,
            _ => return Err(JsValue::from_str("FFT grid must be Input or Larch")),
        };
        Ok(())
    }
    /// Maximum reported R in angstroms. Default: 10.0; undefined restores this default. This
    /// limits the r() and chir_*() output arrays, not the internally retained Fourier bins used
    /// by ifft(). It does not change the transform amplitude or frequency resolution. ifft()
    /// nevertheless needs at least two reported R samples to infer/validate their spacing, so
    /// rmax_out=0 is insufficient for a back-transform.
    #[wasm_bindgen(getter)]
    pub fn rmax_out(&self) -> Option<f64> {
        self.inner.rmax_out
    }
    /// Store the rmax_out setting; see [`Self::rmax_out`] for units, defaults and effects.
    /// Configurations previously copied into a spectrum are unchanged.
    #[wasm_bindgen(setter)]
    pub fn set_rmax_out(&mut self, value: Option<f64>) {
        self.inner.rmax_out = value;
    }
    /// Low-k window parameter. Default: 1.0; undefined restores this default. For taper windows
    /// it controls transition geometry in inverse angstroms. KaiserBessel also uses the same
    /// numeric value as a dimensionless shape parameter, so it cannot be compared as a
    /// universal taper width across all windows.
    #[wasm_bindgen(getter)]
    pub fn dk(&self) -> Option<f64> {
        self.inner.dk
    }
    /// Store the dk setting; see [`Self::dk`] for units, defaults and effects. Configurations
    /// previously copied into a spectrum are unchanged.
    #[wasm_bindgen(setter)]
    pub fn set_dk(&mut self, value: Option<f64>) {
        self.inner.dk = value;
    }
    /// High-k window parameter. Default: undefined uses dk. It controls the upper transition
    /// geometry in inverse angstroms; interpretation depends on the window family. Use the same
    /// value as dk for symmetric endpoint settings.
    #[wasm_bindgen(getter)]
    pub fn dk2(&self) -> Option<f64> {
        self.inner.dk2
    }
    /// Store the dk2 setting; see [`Self::dk2`] for units, defaults and effects. Configurations
    /// previously copied into a spectrum are unchanged.
    #[wasm_bindgen(setter)]
    pub fn set_dk2(&mut self, value: Option<f64>) {
        self.inner.dk2 = value;
    }
    /// Lower Fourier window bound in inverse angstroms. Default: 2.0; explicitly assigning
    /// undefined uses the first prepared k sample. Both bounds must be finite with kmin < kmax;
    /// negative bounds are accepted. Select this above the region where the EXAFS approximation
    /// or background subtraction is unreliable.
    #[wasm_bindgen(getter)]
    pub fn kmin(&self) -> Option<f64> {
        self.inner.kmin
    }
    /// Store the kmin setting; see [`Self::kmin`] for units, defaults and effects.
    /// Configurations previously copied into a spectrum are unchanged.
    #[wasm_bindgen(setter)]
    pub fn set_kmin(&mut self, value: Option<f64>) {
        self.inner.kmin = value;
    }
    /// Upper Fourier window bound in inverse angstroms. Default: 15.0; explicitly assigning
    /// undefined uses the last prepared k sample. Require kmax > kmin. Select this within the
    /// useful measured range; high-k noise can dominate after k weighting.
    #[wasm_bindgen(getter)]
    pub fn kmax(&self) -> Option<f64> {
        self.inner.kmax
    }
    /// Store the kmax setting; see [`Self::kmax`] for units, defaults and effects.
    /// Configurations previously copied into a spectrum are unchanged.
    #[wasm_bindgen(setter)]
    pub fn set_kmax(&mut self, value: Option<f64>) {
        self.inner.kmax = value;
    }
    /// Power of k applied before the forward transform. Default: 2.0; undefined restores this
    /// default. Finite nonnegative values are floored to an integer w. Larger w emphasizes
    /// high-k oscillations and noise; for dimensionless chi, the transformed amplitude has
    /// units angstrom^(-(w + 1)).
    #[wasm_bindgen(getter)]
    pub fn kweight(&self) -> Option<f64> {
        self.inner.kweight
    }
    /// Store the kweight setting; see [`Self::kweight`] for units, defaults and effects.
    /// Configurations previously copied into a spectrum are unchanged.
    #[wasm_bindgen(setter)]
    pub fn set_kweight(&mut self, value: Option<f64>) {
        self.inner.kweight = value;
    }
    /// Forward FFT length N. Default: 2048; undefined restores this default. Require an integer
    /// of at least 2. The R spacing is pi / (N * kstep), in angstroms. Use N at least as large
    /// as the prepared data: Input truncates excess samples, whereas Larch rejects a window
    /// grid that does not fit. Larger zero-padding does not improve experimental resolution.
    #[wasm_bindgen(getter)]
    pub fn nfft(&self) -> Option<usize> {
        self.inner.nfft
    }
    /// Store the nfft setting; see [`Self::nfft`] for units, defaults and effects.
    /// Configurations previously copied into a spectrum are unchanged.
    #[wasm_bindgen(setter)]
    pub fn set_nfft(&mut self, value: Option<usize>) {
        self.inner.nfft = value;
    }
    /// k spacing used to scale the transform and label R, in inverse angstroms. Default:
    /// undefined infers the first spacing of the prepared k grid (normally 0.05 from AUTOBK
    /// defaults). Larch also uses this spacing to resample chi. Input does not resample, so
    /// keep it consistent with the input grid. Require a finite positive value. The inferred
    /// value is retained in the spectrum's copied FFT settings. After changing the background
    /// grid, reassign FFT settings with kstep undefined to infer the new spacing.
    #[wasm_bindgen(getter)]
    pub fn kstep(&self) -> Option<f64> {
        self.inner.kstep
    }
    /// Store the kstep setting; see [`Self::kstep`] for units, defaults and effects.
    /// Configurations previously copied into a spectrum are unchanged.
    #[wasm_bindgen(setter)]
    pub fn set_kstep(&mut self, value: Option<f64>) {
        self.inner.kstep = value;
    }
    /// Fourier window family. Constructor default: KaiserBessel; explicitly assigning undefined
    /// selects the window routine's Hanning fallback. The window reduces truncation ringing and
    /// changes amplitude. No correction for window area or coherent gain is applied; see
    /// FTWindow for valid names.
    #[wasm_bindgen(getter)]
    pub fn window(&self) -> Option<String> {
        self.inner.window.map(|v| format!("{v:?}"))
    }
    /// Store the window setting; see [`Self::window`] for units, defaults and effects.
    /// Configurations previously copied into a spectrum are unchanged.
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

/// Settings for filtering selected R-space contributions and returning a real signal chi(q).
///
/// Choose rmin/rmax in angstroms to enclose the contributions of interest. Defaults are rmin=0,
/// rmax=20, dr=1, KaiserBessel, rweight=0, nfft=2048 and qmax_out=10 inverse angstroms.
/// Automatic kstep preserves consistency with the input R spacing.
///
/// rexafs windows and optionally R-weights the complex forward bins, supplies their conjugate
/// negative-frequency partners, and performs a real inverse DFT with scale
/// sqrt(pi)/(kstep*nfft). This implements a real filtered back-transform; forward k-weighting
/// and windowing remain in the result. It is not a general recovery of the original unweighted
/// chi(k), and is not Larch's complex, one-sided inverse representation. See
/// [inverse_fft.rs](https://github.com/Ameyanagi/rexafs/blob/main/crates/rexafs/src/xafs/inverse_fft.rs)
/// and the [DFT normalization
/// convention](https://numpy.org/doc/stable/reference/routines.fft.html#normalization).
///
/// set_ifft() copies settings. Reassign after editing, and free() this object when finished.
#[wasm_bindgen(js_name = XrayFFTR)]
pub struct WasmXrayFFTR {
    inner: rexafs::XrayFFTR,
}
impl Default for WasmXrayFFTR {
    fn default() -> Self {
        Self::new()
    }
}
#[wasm_bindgen(js_class = XrayFFTR)]
impl WasmXrayFFTR {
    /// Create owned settings with the recommended Rust defaults. Automatic fields are resolved
    /// on the spectrum's copy during processing; resolved values are not written back into
    /// the original settings object. Resolved values stay in the spectrum's settings until
    /// those settings are replaced.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: rexafs::XrayFFTR::new(),
        }
    }
    /// Largest reported back-transform q in inverse angstroms. Default: 10.0; undefined
    /// restores this default. This crops q()/chiq() to the available output range without
    /// changing the inverse calculation. Require a finite nonnegative value.
    #[wasm_bindgen(getter)]
    pub fn qmax_out(&self) -> Option<f64> {
        self.inner.qmax_out
    }
    /// Store the qmax_out setting; see [`Self::qmax_out`] for units, defaults and effects.
    /// Configurations previously copied into a spectrum are unchanged.
    #[wasm_bindgen(setter)]
    pub fn set_qmax_out(&mut self, value: Option<f64>) {
        self.inner.qmax_out = value;
    }
    /// Low-R window parameter. Default: 1.0; undefined restores this default. For taper windows
    /// it controls transition geometry in angstroms. KaiserBessel also uses this numeric value
    /// as a dimensionless shape parameter; different windows do not have equivalent shape for
    /// the same dr.
    #[wasm_bindgen(getter)]
    pub fn dr(&self) -> Option<f64> {
        self.inner.dr
    }
    /// Store the dr setting; see [`Self::dr`] for units, defaults and effects. Configurations
    /// previously copied into a spectrum are unchanged.
    #[wasm_bindgen(setter)]
    pub fn set_dr(&mut self, value: Option<f64>) {
        self.inner.dr = value;
    }
    /// High-R window parameter. Default: undefined uses dr. It controls the upper transition
    /// geometry in angstroms, with interpretation depending on the selected window family. Use
    /// matching dr and dr2 for symmetric endpoint settings.
    #[wasm_bindgen(getter)]
    pub fn dr2(&self) -> Option<f64> {
        self.inner.dr2
    }
    /// Store the dr2 setting; see [`Self::dr2`] for units, defaults and effects. Configurations
    /// previously copied into a spectrum are unchanged.
    #[wasm_bindgen(setter)]
    pub fn set_dr2(&mut self, value: Option<f64>) {
        self.inner.dr2 = value;
    }
    /// Lower inverse-transform window bound in angstroms. Default: 0.0; explicitly assigning
    /// undefined uses the first input R sample. Require 0 <= rmin < rmax. A nonzero lower bound
    /// can exclude low-R background contributions.
    #[wasm_bindgen(getter)]
    pub fn rmin(&self) -> Option<f64> {
        self.inner.rmin
    }
    /// Store the rmin setting; see [`Self::rmin`] for units, defaults and effects.
    /// Configurations previously copied into a spectrum are unchanged.
    #[wasm_bindgen(setter)]
    pub fn set_rmin(&mut self, value: Option<f64>) {
        self.inner.rmin = value;
    }
    /// Upper inverse-transform window bound in angstroms. Default: 20.0; explicitly assigning
    /// undefined uses the last reported input R sample. Require rmax > rmin. Choose the R
    /// interval around the shell contribution of interest; uncorrected Fourier peaks are not
    /// directly bond lengths.
    #[wasm_bindgen(getter)]
    pub fn rmax(&self) -> Option<f64> {
        self.inner.rmax
    }
    /// Store the rmax setting; see [`Self::rmax`] for units, defaults and effects.
    /// Configurations previously copied into a spectrum are unchanged.
    #[wasm_bindgen(setter)]
    pub fn set_rmax(&mut self, value: Option<f64>) {
        self.inner.rmax = value;
    }
    /// Power of R applied before the inverse transform. Default: 0.0; undefined restores this
    /// default. Finite nonnegative values are floored to an integer. Leave at 0 for ordinary
    /// shell filtering; a positive value additionally emphasizes larger-R contributions and
    /// changes the signal units.
    #[wasm_bindgen(getter)]
    pub fn rweight(&self) -> Option<f64> {
        self.inner.rweight
    }
    /// Store the rweight setting; see [`Self::rweight`] for units, defaults and effects.
    /// Configurations previously copied into a spectrum are unchanged.
    #[wasm_bindgen(setter)]
    pub fn set_rweight(&mut self, value: Option<f64>) {
        self.inner.rweight = value;
    }
    /// Inverse FFT length N. Default: 2048; undefined restores this default. Require an integer
    /// of at least 2. Keeping kstep automatic resolves output spacing from the existing R grid
    /// and N; reducing N discards high-R bins and increasing it pads them with zeros.
    #[wasm_bindgen(getter)]
    pub fn nfft(&self) -> Option<usize> {
        self.inner.nfft
    }
    /// Store the nfft setting; see [`Self::nfft`] for units, defaults and effects.
    /// Configurations previously copied into a spectrum are unchanged.
    #[wasm_bindgen(setter)]
    pub fn set_nfft(&mut self, value: Option<usize>) {
        self.inner.nfft = value;
    }
    /// Output q spacing in inverse angstroms. Default: undefined computes pi / (nfft *
    /// delta_R), where delta_R is the input R spacing in angstroms. An explicit value must
    /// agree with that relationship or processing throws. Leave automatic when changing nfft.
    /// The resolved value is retained in the spectrum's copy. After changing the forward R
    /// grid, reassign inverse settings whose kstep is undefined to resolve it again.
    #[wasm_bindgen(getter)]
    pub fn kstep(&self) -> Option<f64> {
        self.inner.kstep
    }
    /// Store the kstep setting; see [`Self::kstep`] for units, defaults and effects.
    /// Configurations previously copied into a spectrum are unchanged.
    #[wasm_bindgen(setter)]
    pub fn set_kstep(&mut self, value: Option<f64>) {
        self.inner.kstep = value;
    }
    /// R-space window family. Constructor default: KaiserBessel; explicitly assigning undefined
    /// selects Hanning. The window selects and tapers the R contributions retained in chiq().
    /// The forward k weight and window are not divided out by the inverse transform.
    #[wasm_bindgen(getter)]
    pub fn window(&self) -> Option<String> {
        self.inner.window.map(|v| format!("{v:?}"))
    }
    /// Store the window setting; see [`Self::window`] for units, defaults and effects.
    /// Configurations previously copied into a spectrum are unchanged.
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

/// Select the normalization algorithm and hold an owned copy of its settings.
///
/// Use PrePostEdge(settings) for customized pre/post-edge fits or new_prepostedge() for
/// automatic defaults. The no-argument MBack factory is a historical empty selector and cannot
/// normalize data. MBACK was unimplemented through version 0.2.9. Copy this method into Spectrum.set_normalization_method(), then free() the
/// wrapper when no longer needed.
#[wasm_bindgen(js_name = NormalizationMethod)]
pub struct WasmNormalizationMethod {
    inner: rexafs::NormalizationMethod,
}
#[wasm_bindgen(js_class = NormalizationMethod)]
impl WasmNormalizationMethod {
    /// Copy the supplied pre/post-edge configuration into a new owned algorithm wrapper. The
    /// input settings are not consumed. Assign the wrapper to
    /// Spectrum.set_normalization_method(), then free() it when no longer needed; the spectrum
    /// retains its own copy.
    #[wasm_bindgen(js_name = PrePostEdge)]
    pub fn configured(parameters: &WasmPrePostEdge) -> Self {
        Self {
            inner: rexafs::NormalizationMethod::PrePostEdge(parameters.inner.clone()),
        }
    }
    /// Create a new owned pre/post-edge normalization wrapper with automatic E0, fit ranges,
    /// polynomial order and edge step. It does not process any spectrum. Assign it to
    /// Spectrum.set_normalization_method() and free() the wrapper when finished.
    pub fn new_prepostedge() -> Self {
        Self {
            inner: rexafs::NormalizationMethod::new_prepostedge(),
        }
    }
    /// Create the historical empty MBack selector. Selecting it makes normalize() throw.
    /// Use new_prepostedge() for automatic polynomial normalization. MBACK was unimplemented
    /// through version 0.2.9; this no-argument selector remains unusable for normalization.
    /// Free this wrapper when finished.
    pub fn new_mback() -> Self {
        Self {
            inner: rexafs::NormalizationMethod::new_mback(),
        }
    }
}

/// Select the background-removal algorithm and hold an owned copy of its settings.
///
/// Use AUTOBK(settings) for a configured spline background or new_autobk() for the recommended
/// defaults. The ILPBkg factory is only a placeholder; it does not implement that algorithm.
/// Copy this method into Spectrum.set_background_method(), then free() the wrapper when no
/// longer needed.
#[wasm_bindgen(js_name = BackgroundMethod)]
pub struct WasmBackgroundMethod {
    inner: rexafs::BackgroundMethod,
}
#[wasm_bindgen(js_class = BackgroundMethod)]
impl WasmBackgroundMethod {
    /// Copy the supplied AUTOBK configuration into a new owned algorithm wrapper. The input
    /// settings are not consumed. Assign the wrapper to Spectrum.set_background_method(), then
    /// free() it when no longer needed; the spectrum retains its own copy.
    #[wasm_bindgen(js_name = AUTOBK)]
    pub fn configured(parameters: &WasmAUTOBK) -> Self {
        Self {
            inner: rexafs::BackgroundMethod::AUTOBK(parameters.inner.clone()),
        }
    }
    /// Create a new owned AUTOBK wrapper with the recommended LinearDirect/FixedPenalty
    /// defaults, rbkg=1 angstrom and clamp_lambda=0.001. It does not process any spectrum.
    /// Assign it to Spectrum.set_background_method() and free() the wrapper when finished.
    pub fn new_autobk() -> Self {
        Self {
            inner: rexafs::BackgroundMethod::new_autobk(),
        }
    }
    /// Create an owned ILPBkg placeholder for API compatibility. This background method is not
    /// implemented and calc_background() throws if selected. Use new_autobk() for supported
    /// background removal, and free() any placeholder you create.
    pub fn new_ilpbkg() -> Self {
        Self {
            inner: rexafs::BackgroundMethod::new_ilpbkg(),
        }
    }
}

/// Native WebAssembly spectrum used by the JavaScript Spectrum facade.
/// Inputs are copied and checked; result arrays are independent JavaScript copies. Rust stage
/// errors are converted to JavaScript Error objects. The facade supplies fluent return values
/// and additional JavaScript type checks; numerical processing and invalidation live in
/// rexafs::Spectrum.
#[wasm_bindgen(js_name = Spectrum)]
pub struct WasmSpectrum {
    inner: rexafs::Spectrum,
}
#[wasm_bindgen(js_class = Spectrum)]
impl WasmSpectrum {
    /// Correct into an independently owned native spectrum; retain XANES-only history.
    pub fn correct_fluorescence(&self, definition: &str) -> Result<Self, JsValue> {
        let model = fluorescence::WasmFluorescenceCorrection::new(definition)?;
        Ok(Self {
            inner: self
                .inner
                .correct_fluorescence(&model.inner)
                .map_err(fluorescence::error)?,
        })
    }
    /// Historical result plus native replay definition, or None for uncorrected data.
    pub fn fluorescence_correction_json(&self) -> Result<Option<String>, JsValue> {
        self.inner
            .fluorescence_correction()
            .map(fluorescence::result_json)
            .transpose()
    }
    /// Original or explicitly revised acquisition interpretation.
    pub fn absorption_mode(&self) -> String {
        fluorescence::mode_name(self.inner.absorption_mode()).into()
    }
    /// Revise evidence without altering arrays/caches or clearing correction history.
    pub fn set_absorption_mode(&mut self, mode: &str) -> Result<(), JsValue> {
        self.inner.set_absorption_mode(fluorescence::mode(mode)?);
        Ok(())
    }
    /// Copy MBACK settings into this spectrum, invalidating dependent results.
    pub fn set_mback_json(&mut self, json: &str) -> Result<(), JsValue> {
        let model = mback::WasmMBack::from_json(json)?;
        self.inner
            .set_normalization_method(model.inner)
            .map_err(error)?;
        Ok(())
    }
    /// Latest owned full MBACK result JSON, or None after invalidation/non-MBACK processing.
    pub fn mback_result_json(&self) -> Result<Option<String>, JsValue> {
        let Some(rexafs::NormalizationMethod::MBack(model)) = self.inner.normalization.as_ref()
        else {
            return Ok(None);
        };
        model
            .result
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }
    /// Prepare missing normalization/background on a copy and return an owned native map.
    /// The facade supplies versioned settings JSON; original spectrum state is untouched.
    pub fn wavelet(&self, definition_json: &str) -> Result<wavelet::WasmWaveletMap, JsValue> {
        let definition = wavelet::WasmWavelet::new(definition_json)?;
        let inner = self
            .inner
            .wavelet(&definition.inner)
            .map_err(|e| js_sys::Error::new(&e.to_string()))?;
        Ok(wavelet::WasmWaveletMap { inner })
    }
    /// Internal bridge for Spectrum.fit_peaks. The native model prepares on a copy.
    /// Optional errors must describe the selected signal on the original native grid.
    pub fn fit_peaks_json(
        &self,
        definition: &str,
        errors: Option<Vec<f64>>,
    ) -> Result<String, JsValue> {
        let model: rexafs::prelude::PeakFit =
            serde_json::from_str(definition).map_err(|e| js_sys::Error::new(&e.to_string()))?;
        let result = model
            .fit_with_errors(&self.inner, errors.as_deref())
            .map_err(|e| js_sys::Error::new(&e.to_string()))?;
        serde_json::to_string(&result).map_err(|e| js_sys::Error::new(&e.to_string()).into())
    }
    /// Internal bridge for baseline-only initialization outside peak intervals.
    /// Returns a new definition; source, initial model and final masks are unchanged.
    pub fn initialize_peaks_json(
        &self,
        definition: &str,
        intervals: &str,
    ) -> Result<String, JsValue> {
        let model: rexafs::prelude::PeakFit =
            serde_json::from_str(definition).map_err(|e| js_sys::Error::new(&e.to_string()))?;
        let intervals: Vec<[f64; 2]> =
            serde_json::from_str(intervals).map_err(|e| js_sys::Error::new(&e.to_string()))?;
        let ranges = intervals
            .into_iter()
            .map(|r| r[0]..=r[1])
            .collect::<Vec<_>>();
        let result = model
            .initialize_baseline(&self.inner, &ranges)
            .map_err(|e| js_sys::Error::new(&e.to_string()))?;
        serde_json::to_string(&result).map_err(|e| js_sys::Error::new(&e.to_string()).into())
    }
    /// Internal JSON bridge for the documented JavaScript Spectrum.measure facade.
    /// Prepares only required stages on a private copy; never modifies this spectrum.
    /// Optional errors describe the selected signal on its native grid, not raw
    /// counts to be propagated through processing. Returns the owned core result.
    pub fn measure_json(
        &self,
        definition_json: &str,
        errors: Option<Vec<f64>>,
    ) -> Result<String, JsValue> {
        let definition: rexafs::prelude::Measurement = serde_json::from_str(definition_json)
            .map_err(|e| js_sys::Error::new(&e.to_string()))?;
        let result = match errors {
            Some(errors) => self.inner.measure_with_errors(&definition, &errors),
            None => self.inner.measure(&definition),
        }
        .map_err(|e| js_sys::Error::new(&e.to_string()))?;
        serde_json::to_string(&result).map_err(|e| js_sys::Error::new(&e.to_string()).into())
    }
    /// Copy energy in eV and absorption mu from equal-length finite slices. Requires at least
    /// two samples and strictly increasing energy; returns a JavaScript Error when validation
    /// fails. No stages run on construction.
    #[wasm_bindgen(constructor)]
    pub fn new(energy: &[f64], mu: &[f64]) -> Result<Self, JsValue> {
        Ok(Self {
            inner: rexafs::Spectrum::from_arrays(energy, mu).map_err(error)?,
        })
    }
    /// Construct from copied energy in eV and absorption mu; equivalent to [`Self::new`].
    pub fn from_arrays(energy: &[f64], mu: &[f64]) -> Result<Self, JsValue> {
        Self::new(energy, mu)
    }
    /// Replace measured energy (eV) and absorption mu with independent copies. Uses the same
    /// validation as the constructor and preserves the previous spectrum if input validation
    /// fails. Clears E0 and all calculated results while retaining stage settings. Recompute
    /// the desired stages after replacement.
    pub fn set_spectrum(&mut self, energy: &[f64], mu: &[f64]) -> Result<(), JsValue> {
        let input = rexafs::Spectrum::from_arrays(energy, mu).map_err(error)?;
        self.inner
            .set_spectrum(input.energy.unwrap(), input.mu.unwrap());
        Ok(())
    }
    /// Assign E0 in eV and clear normalization and downstream results. The JavaScript facade
    /// checks that the supplied value is finite.
    pub fn set_e0(&mut self, e0: f64) {
        self.inner.set_e0(e0);
    }
    /// Return the selected or detected absorption-edge energy in eV, or undefined before
    /// assignment/detection. Reading this value does not detect an edge or run any processing.
    pub fn e0(&self) -> Option<f64> {
        self.inner.e0()
    }
    /// Clear normalization, background, forward and inverse calculated arrays without
    /// discarding the measured inputs, selected E0 or stage settings. User-specified edge-step
    /// overrides are retained; a previously estimated step is recomputed by the next
    /// normalization. Subsequent getters return undefined until their stages run again.
    /// Resolved automatic settings, such as FFT kstep, are retained. Reassign the affected
    /// stage settings to request fresh automatic values for changed input grids.
    pub fn invalidate_derived(&mut self) {
        self.inner.invalidate_derived();
    }
    /// Copy inverse-transform settings and clear q()/chiq() while preserving normalization,
    /// background and forward results. Settings can be freed after assignment; later edits
    /// require reassignment. Invalid R ranges or inconsistent kstep/nfft are reported when
    /// ifft() runs.
    pub fn set_ifft(&mut self, parameters: &WasmXrayFFTR) {
        self.inner.set_ifft(parameters.inner.clone());
    }
    /// Copy forward-transform settings and clear r(), chir_*(), kwin(), kwin_k(), q() and
    /// chiq(). Normalization and background k()/chi() are preserved. Settings can be freed
    /// after assignment; later edits require reassignment. Invalid numerical settings are
    /// reported when fft() runs. Assign settings with kstep undefined to request fresh spacing
    /// inference, for example after changing AUTOBK.kstep. Inverse settings are retained; if
    /// their spacing was already resolved, they may also need replacement before ifft().
    pub fn set_fft(&mut self, parameters: &WasmXrayFFTF) {
        self.inner.set_fft(parameters.inner.clone());
    }
    /// Copy the selected normalization method and clear normalization, background, forward and
    /// inverse results. A specified method E0 overrides the spectrum E0; otherwise the existing
    /// E0 is retained. The caller keeps ownership of the settings and wrapper and may free them
    /// after assignment. Later edits require reassignment.
    pub fn set_normalization_method(
        &mut self,
        method: &WasmNormalizationMethod,
    ) -> Result<(), JsValue> {
        self.inner
            .set_normalization_method(Some(method.inner.clone()))
            .map(|_| ())
            .map_err(error)
    }
    /// Copy the selected background method and clear background, forward and inverse results
    /// while retaining normalization. The caller keeps ownership of the settings and wrapper
    /// and may free them after assignment. Later edits require reassignment.
    pub fn set_background_method(&mut self, method: &WasmBackgroundMethod) -> Result<(), JsValue> {
        self.inner
            .set_background_method(Some(method.inner.clone()))
            .map(|_| ())
            .map_err(error)
    }
    /// Estimate the absorption-edge energy from the derivative of measured mu(E), including
    /// local smoothing/refinement. Clears normalization and all downstream results, then
    /// retains the selected E0. Throws if the input cannot be used for edge detection. Inspect
    /// the result for noisy, multiple-edge or unusual spectra and use set_e0() for an explicit
    /// choice.
    pub fn find_e0(&mut self) -> Result<(), JsValue> {
        self.inner.find_e0().map(|_| ()).map_err(error)
    }
    /// Fit the selected pre/post-edge model, find E0 if needed, and calculate dimensionless
    /// norm()/flat() plus pre_edge()/post_edge() baselines on the original energy grid. Clears
    /// background and all Fourier results even when normalization was already present. Uses
    /// automatic PrePostEdge defaults if no method was selected. Throws on unsupported methods
    /// or failed baseline fits.
    pub fn normalize(&mut self) -> Result<(), JsValue> {
        self.inner.normalize().map(|_| ()).map_err(error)
    }
    /// Fit the selected smooth background and calculate unweighted, dimensionless chi(k) on
    /// k(). Runs missing normalization first and uses default AUTOBK if no background method
    /// was selected. Clears forward and inverse results on every call. Throws on invalid
    /// parameters, unsupported methods, insufficient data or a failed spline solve.
    pub fn calc_background(&mut self) -> Result<(), JsValue> {
        self.inner.calc_background().map(|_| ()).map_err(error)
    }
    /// Calculate complex chi(R) from chi(k), computing missing normalization and background
    /// first. Defaults: k=2..15 inverse angstroms, kweight=2, KaiserBessel, nfft=2048 and kstep
    /// inferred from the background grid. The unnormalized forward DFT is multiplied by
    /// kstep/sqrt(pi), with no additional 1/N; XrayFFTF explains the formula and units. Clears
    /// inverse results and throws on invalid settings or failed prerequisite stages.
    pub fn fft(&mut self) -> Result<(), JsValue> {
        self.inner.fft().map(|_| ()).map_err(error)
    }
    /// Calculate real chi(q) by windowing the retained complex Fourier bins in R and performing
    /// a conjugate-symmetric inverse transform. Runs missing forward and prerequisite stages
    /// first. Forward k-weighting and windowing remain in the output, so this is not generally
    /// unweighted chi(k). Throws on inconsistent transform settings or failed prerequisite
    /// stages. At least two reported R samples are required even though filtering uses the full
    /// internal Fourier bins; rmax_out=0 therefore fails. When reusing a spectrum with a
    /// different forward grid, reset previously resolved inverse settings with set_ifft()
    /// (added in 0.2.5), or create a fresh spectrum in 0.2.4.
    pub fn ifft(&mut self) -> Result<(), JsValue> {
        self.inner.ifft().map(|_| ()).map_err(error)
    }
    /// Return an independent copy of the uniform background k axis in inverse angstroms,
    /// beginning at zero and paired with chi(). Its spacing is AUTOBK.kstep (default 0.05).
    /// Returns undefined before background removal or after invalidation; this getter never
    /// runs a stage.
    pub fn k(&self) -> Option<js_sys::Float64Array> {
        self.inner.k().map(js_sys::Float64Array::from)
    }
    /// Return an independent copy of unweighted EXAFS chi(k) = (mu - smooth background) /
    /// edge_step, dimensionless and paired with k(). The measured absorption and smooth
    /// background are resampled according to the selected background method. Returns undefined
    /// before background removal or after invalidation. Forward kweight and window settings do
    /// not change this array.
    pub fn chi(&self) -> Option<js_sys::Float64Array> {
        self.inner.chi().map(js_sys::Float64Array::from)
    }
    /// Return an independent copy of dimensionless normalized absorption, (mu - pre_edge) /
    /// edge_step, on the original input energy grid. Here pre_edge is the fitted baseline and
    /// edge_step is the selected or fitted absorption jump. Returns undefined before
    /// normalization or after invalidation; does not calculate missing results.
    pub fn norm(&self) -> Option<js_sys::Float64Array> {
        self.inner
            .norm()
            .map(|v| js_sys::Float64Array::from(v.as_slice()))
    }
    /// Return an independent copy of dimensionless flattened absorption on the input energy
    /// grid. Above E0 this subtracts the fitted post-edge trend from norm(), with an offset
    /// that preserves the value at the edge; below E0 it equals norm(). This is a
    /// presentation/near-edge quantity, not the background chi(k). Returns undefined before
    /// normalization or after invalidation.
    pub fn flat(&self) -> Option<js_sys::Float64Array> {
        self.inner
            .flat()
            .map(|v| js_sys::Float64Array::from(v.as_slice()))
    }
    /// Return an independent copy of the fitted pre-edge baseline, in the same units as input
    /// mu and evaluated across the entire original energy grid. The fit uses the selected
    /// pre-edge interval, and its extrapolation is subtracted during normalization. Returns
    /// undefined before normalization or after invalidation.
    pub fn pre_edge(&self) -> Option<js_sys::Float64Array> {
        self.inner
            .pre_edge()
            .map(|v| js_sys::Float64Array::from(v.as_slice()))
    }
    /// Return an independent copy of the fitted post-edge baseline in input mu units, evaluated
    /// on the original energy grid. It includes the pre-edge baseline plus the fitted
    /// polynomial for pre-edge-subtracted absorption. It determines the edge step and
    /// flattening trend; it is not the AUTOBK background. Returns undefined before
    /// normalization or after invalidation.
    pub fn post_edge(&self) -> Option<js_sys::Float64Array> {
        self.inner
            .post_edge()
            .map(|v| js_sys::Float64Array::from(v.as_slice()))
    }
    /// Return an independent copy of the reported Fourier R axis in angstroms, paired with
    /// chir_mag(), chir_real() and chir_imag(). Its spacing is pi/(nfft*kstep), and its extent
    /// is limited by rmax_out and the available positive-frequency bins. Peaks are not
    /// phase-corrected bond lengths. Returns undefined before fft() or after invalidation.
    pub fn r(&self) -> Option<js_sys::Float64Array> {
        self.inner
            .r()
            .map(|v| js_sys::Float64Array::from(v.as_slice()))
    }
    /// Return an independent copy of the dimensionless forward window values, paired with
    /// kwin_k(). These values exclude the k^kweight factor. The array can have a different
    /// length/grid from background k()/chi() with grid=Larch. Returns undefined before fft() or
    /// after invalidation.
    pub fn kwin(&self) -> Option<js_sys::Float64Array> {
        self.inner
            .kwin()
            .map(|v| js_sys::Float64Array::from(v.as_slice()))
    }
    /// Return an independent copy of the forward window axis in inverse angstroms, paired with
    /// kwin(). Input uses the prepared background grid; Larch returns its resampled and
    /// possibly extended window grid. This getter does not alter the background k()/chi()
    /// arrays. Returns undefined before fft() or after invalidation.
    pub fn kwin_k(&self) -> Option<js_sys::Float64Array> {
        self.inner
            .kwin_k()
            .map(|v| js_sys::Float64Array::from(v.as_slice()))
    }
    /// Return an independent copy of the magnitude sqrt(real^2 + imag^2) of complex chi(R),
    /// paired with r(). For dimensionless chi and forward kweight w, units are
    /// angstrom^(-(w+1)); at w=2 they are inverse cubic angstroms. No peak-height or
    /// window-area normalization is applied. Returns undefined before fft() or after
    /// invalidation.
    pub fn chir_mag(&self) -> Option<js_sys::Float64Array> {
        self.inner
            .chir_mag()
            .map(|v| js_sys::Float64Array::from(v.as_slice()))
    }
    /// Return an independent copy of the real component of chi(R), paired with r(). Units are
    /// angstrom^(-(w+1)), where w is the forward kweight and chi is dimensionless. Uses the
    /// negative-exponent forward DFT with amplitude factor kstep/sqrt(pi). Returns undefined
    /// before fft() or after invalidation.
    pub fn chir_real(&self) -> Option<js_sys::Float64Array> {
        self.inner
            .chir_real()
            .map(|v| js_sys::Float64Array::from(v.as_slice()))
    }
    /// Return an independent copy of the imaginary component of chi(R), paired with r(). Units
    /// are angstrom^(-(w+1)), where w is the forward kweight and chi is dimensionless. Its sign
    /// follows exp(-2*pi*i*j*m/nfft); reversing the Fourier convention changes that sign.
    /// Returns undefined before fft() or after invalidation.
    pub fn chir_imag(&self) -> Option<js_sys::Float64Array> {
        self.inner
            .chir_imag()
            .map(|v| js_sys::Float64Array::from(v.as_slice()))
    }
    /// Return an independent copy of the real back-transform axis in inverse angstroms,
    /// beginning at zero and paired with chiq(). Its spacing follows the inverse FFT settings
    /// and the forward R grid; q distinguishes this possibly filtered/resized grid from
    /// background k(). Returns undefined before ifft() or after invalidation.
    pub fn q(&self) -> Option<js_sys::Float64Array> {
        self.inner
            .q()
            .map(|v| js_sys::Float64Array::from(v.as_slice()))
    }
    /// Return an independent copy of the real R-filtered signal, paired with q(). Forward
    /// k-weighting and windowing remain, so this is not generally unweighted chi(k). With
    /// dimensionless chi, forward kweight w and inverse rweight v, units are angstrom^(v-w);
    /// ordinary v=0 retains the units of k^w*chi. Returns undefined before ifft() or after
    /// invalidation.
    pub fn chiq(&self) -> Option<js_sys::Float64Array> {
        self.inner
            .chiq()
            .map(|v| js_sys::Float64Array::from(v.as_slice()))
    }
}

/// Owned, content-detected measurement import, available since 0.2.6.
/// Parsing and conversion use rexafs::io on both native and WebAssembly targets.
/// Release its native storage with free() after copying the arrays you need.
#[wasm_bindgen(js_name = Measurement)]
pub struct WasmMeasurement {
    inner: rexafs::io::Measurement,
}
#[wasm_bindgen(js_class = Measurement)]
impl WasmMeasurement {
    /// Parse text, gzip, project or HDF5 bytes. Returns owned scans and metadata;
    /// no network access, detector correction or spectrum processing occurs.
    /// Inputs and expanded gzip text are limited to 256 MiB. Invalid files throw.
    #[wasm_bindgen(constructor)]
    pub fn new(bytes: &[u8]) -> Result<WasmMeasurement, JsValue> {
        Ok(Self {
            inner: rexafs::io::parse_measurement(bytes)
                .map_err(|e| js_sys::Error::new(&e.to_string()))?,
        })
    }
    /// Append a scan from explicit HDF5 vector paths and return its zero-based index.
    /// Paths must be distinct nonempty vectors of equal length; arrays are copied.
    pub fn select_datasets(&mut self, paths_json: &str) -> Result<usize, JsValue> {
        let paths: Vec<String> =
            serde_json::from_str(paths_json).map_err(|e| js_sys::Error::new(&e.to_string()))?;
        let scan = self
            .inner
            .dataset_scan(&paths)
            .map_err(|e| js_sys::Error::new(&e.to_string()))?;
        let index = self.inner.scans.len();
        self.inner.scans.push(scan);
        Ok(index)
    }
    /// Copy the import document as JSON. Nonfinite raw cells appear as null;
    /// conversion still validates the original numeric arrays held in Rust.
    pub fn document_json(&self) -> Result<String, JsValue> {
        serde_json::to_string(&self.inner).map_err(|e| js_sys::Error::new(&e.to_string()).into())
    }
    /// Copy a scan's converted energy and signal as JSON, in acquisition order.
    /// Omit mapping_json only when the scan has exactly one signal candidate.
    /// Explicit mappings accept exact column names or zero-based indices and energy units.
    /// Errors identify invalid roles, units, ratios and nonfinite selected cells.
    pub fn arrays_json(
        &self,
        scan: usize,
        mapping_json: Option<String>,
    ) -> Result<String, JsValue> {
        let mapping = mapping_json
            .map(|s| serde_json::from_str::<rexafs::io::SpectrumSelection>(&s))
            .transpose()
            .map_err(|e| js_sys::Error::new(&e.to_string()))?;
        let selected = self
            .inner
            .scans
            .get(scan)
            .ok_or_else(|| js_sys::Error::new("Scan index out of range"))?;
        let mapping = mapping
            .as_ref()
            .map(|m| m.resolve(selected))
            .transpose()
            .map_err(|e| js_sys::Error::new(&e.to_string()))?;
        let arrays = selected
            .arrays(mapping.as_ref())
            .map_err(|e| js_sys::Error::new(&e.to_string()))?;
        serde_json::to_string(&arrays).map_err(|e| js_sys::Error::new(&e.to_string()).into())
    }
}
