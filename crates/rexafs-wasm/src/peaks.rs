//! Native model construction for the JavaScript peak-fitting facade.
use rexafs::prelude::{PeakFit, PeakRole};
use wasm_bindgen::prelude::*;
fn error(e: impl std::fmt::Display) -> JsValue {
    js_sys::Error::new(&e.to_string()).into()
}

/// Composite peak model; package users construct PeakFit([start, end]).
/// Native construction uses E0-relative eV and Norm; builder methods return copies.
#[wasm_bindgen(js_name=PeakFit)]
pub struct WasmPeakFit {
    pub(crate) inner: PeakFit,
}
#[wasm_bindgen(js_class=PeakFit)]
impl WasmPeakFit {
    /// Empty normalized model; add at least one peak, baseline or step before fitting.
    #[wasm_bindgen(constructor)]
    pub fn new(start: f64, end: f64) -> Self {
        Self {
            inner: PeakFit::new(start..=end),
        }
    }
    /// Use dimensionless flattened mu; prerequisites run on a copy. Returns a new model.
    pub fn flat(&self) -> Self {
        Self {
            inner: self.inner.clone().flat(),
        }
    }
    /// Use the original mapped absorption signal and its units. Returns a new model.
    pub fn raw_mu(&self) -> Self {
        Self {
            inner: self.inner.clone().raw_mu(),
        }
    }
    /// Interpret ranges, centers and baseline references as absolute eV. Returns a new model.
    pub fn absolute(&self) -> Self {
        Self {
            inner: self.inner.clone().absolute(),
        }
    }
    /// Use offsets from this fixed reference energy in eV. Returns a new model.
    pub fn reference(&self, energy_ev: f64) -> Self {
        Self {
            inner: self.inner.clone().reference(energy_ev),
        }
    }
    /// Add a Gaussian: center/FWHM in eV, whole-axis area in signal units times eV. Returns a new model.
    pub fn gaussian(&self, name: &str, center: f64, area: f64, fwhm: f64) -> Self {
        Self {
            inner: self.inner.clone().gaussian(name, center, area, fwhm),
        }
    }
    /// Add a Lorentzian with whole-axis area and FWHM in eV. Returns a new model.
    pub fn lorentzian(&self, name: &str, center: f64, area: f64, fwhm: f64) -> Self {
        Self {
            inner: self.inner.clone().lorentzian(name, center, area, fwhm),
        }
    }
    /// Add a common-FWHM mixture; fraction is the Lorentzian share from zero to one. Returns a new model.
    pub fn pseudo_voigt(
        &self,
        name: &str,
        center: f64,
        area: f64,
        fwhm: f64,
        fraction: f64,
    ) -> Self {
        Self {
            inner: self
                .inner
                .clone()
                .pseudo_voigt(name, center, area, fwhm, fraction),
        }
    }
    /// Add a true Voigt with independent Gaussian/Lorentzian FWHM in eV. Returns a new model.
    pub fn voigt(
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
    pub fn erf_step(&self, name: &str, center: f64, height: f64, scale: f64) -> Self {
        Self {
            inner: self.inner.clone().erf_step(name, center, height, scale),
        }
    }
    /// Add height*(1/2+atan((E-center)/scale)/pi); scale is positive eV. Returns a new model.
    pub fn arctan_step(&self, name: &str, center: f64, height: f64, scale: f64) -> Self {
        Self {
            inner: self.inner.clone().arctan_step(name, center, height, scale),
        }
    }
    /// Add a fitted constant named baseline, in signal units. Returns a new model.
    pub fn constant_baseline(&self, offset: f64) -> Self {
        Self {
            inner: self.inner.clone().constant_baseline(offset),
        }
    }
    /// Add baseline = offset+slope*E_offset; slope is signal units/eV. Returns a new model.
    pub fn linear_baseline(&self, offset: f64, slope: f64) -> Self {
        Self {
            inner: self.inner.clone().linear_baseline(offset, slope),
        }
    }
    /// Exclude an inclusive interval; coordinates use the chosen energy origin.
    pub fn exclude(&self, start: f64, end: f64) -> Self {
        Self {
            inner: self.inner.clone().exclude(start..=end),
        }
    }
    /// Replace an existing parameter. Bounds are optional; an expression disables variation.
    pub fn parameter(
        &self,
        name: &str,
        value: f64,
        vary: bool,
        minimum: Option<f64>,
        maximum: Option<f64>,
        expression: Option<String>,
    ) -> Result<Self, JsValue> {
        if !self.inner.parameters.vars.contains_key(name) {
            return Err(error(format!("Unknown peak parameter: {name}")));
        }
        let mut next = self.inner.clone();
        let mut variable =
            rexafs::prelude::FitVariable::new(value, vary).with_bounds(minimum, maximum);
        if let Some(expression) = expression {
            variable = variable.with_expr(expression);
        }
        next.parameters.insert(name, variable);
        Ok(Self { inner: next })
    }
    /// Change a peak's scientific role to baseline; an edge step is rejected.
    pub fn as_baseline(&self, name: &str) -> Result<Self, JsValue> {
        let mut next = self.inner.clone();
        let component = next
            .components
            .iter_mut()
            .find(|c| c.name == name)
            .ok_or_else(|| error("Unknown component"))?;
        if component.role == PeakRole::Edge {
            return Err(error("An edge step cannot be a baseline"));
        }
        component.role = PeakRole::Baseline;
        Ok(Self { inner: next })
    }
    /// Copy the model with positive iteration/tolerance settings; validated during fitting.
    pub fn solver(&self, max_iterations: usize, tolerance: f64) -> Self {
        let mut next = self.inner.clone();
        next.max_iterations = max_iterations;
        next.tolerance = tolerance;
        Self { inner: next }
    }
    /// Evaluate at absolute energies without fitting or masking; e0 is needed only for relative models.
    pub fn evaluate(&self, energy: &[f64], e0: Option<f64>) -> Result<Vec<f64>, JsValue> {
        self.inner.evaluate(energy, e0).map_err(error)
    }
    /// Serialize the complete initial definition; no source arrays are changed.
    pub fn to_json(&self) -> Result<String, JsValue> {
        serde_json::to_string(&self.inner).map_err(error)
    }
    /// Restore and validate a complete definition. Restricted ties never execute external code.
    pub fn from_json(json: &str) -> Result<Self, JsValue> {
        let inner: PeakFit = serde_json::from_str(json).map_err(error)?;
        inner.validate().map_err(error)?;
        Ok(Self { inner })
    }
    /// Internal result adapter: copy fitted values for explicit reuse without changing history.
    pub fn from_result(json: &str) -> Result<Self, JsValue> {
        let result: rexafs::prelude::PeakFitResult = serde_json::from_str(json).map_err(error)?;
        Ok(Self {
            inner: result.fitted_model(),
        })
    }
}
