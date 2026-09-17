//! Thin owned Cauchy map adapters. JavaScript handles display; Rust owns the science.
use wasm_bindgen::prelude::*;
fn error(e: impl std::fmt::Display) -> JsValue {
    js_sys::Error::new(&e.to_string()).into()
}
/// Versioned native wavelet definition, with checked calculation and allocation limits.
#[wasm_bindgen(js_name=Wavelet)]
pub struct WasmWavelet {
    pub(crate) inner: rexafs::Wavelet,
}
#[wasm_bindgen(js_class=Wavelet)]
impl WasmWavelet {
    /// Decode a complete native definition. Numerical validation occurs on calculation.
    #[wasm_bindgen(constructor)]
    pub fn new(json: &str) -> Result<Self, JsValue> {
        Ok(Self {
            inner: serde_json::from_str(json).map_err(error)?,
        })
    }
    /// Calculate an owned complex map from original unweighted chi and k (inverse angstroms).
    pub fn calculate(&self, k: &[f64], chi: &[f64]) -> Result<WasmWaveletMap, JsValue> {
        Ok(WasmWaveletMap {
            inner: self.inner.calculate(k, chi).map_err(error)?,
        })
    }
    /// Checked dimension and approximate storage estimate, without performing the transform.
    pub fn estimate_json(&self, k: &[f64]) -> Result<String, JsValue> {
        serde_json::to_string(&self.inner.estimate(k).map_err(error)?).map_err(error)
    }
    /// Requested settings without input arrays.
    pub fn to_json(&self) -> Result<String, JsValue> {
        serde_json::to_string(&self.inner).map_err(error)
    }
}
/// Owned immutable scientific map. Every array crossing the Wasm boundary is a copy.
#[wasm_bindgen(js_name=WaveletMap)]
pub struct WasmWaveletMap {
    pub(crate) inner: rexafs::WaveletMap,
}
#[wasm_bindgen(js_class=WaveletMap)]
impl WasmWaveletMap {
    /// Restore checked native dimensions, axes, values and resource limits.
    pub fn from_json(json: &str) -> Result<Self, JsValue> {
        Ok(Self {
            inner: serde_json::from_str(json).map_err(error)?,
        })
    }
    /// Complete native map including original input arrays and preparation provenance.
    pub fn to_json(&self) -> Result<String, JsValue> {
        serde_json::to_string(&self.inner).map_err(error)
    }
    /// Independent complete transform definition.
    pub fn definition_json(&self) -> Result<String, JsValue> {
        serde_json::to_string(self.inner.settings()).map_err(error)
    }
    /// Spectrum preparation metadata; null for a direct array calculation.
    pub fn preparation_json(&self) -> Result<String, JsValue> {
        serde_json::to_string(&self.inner.preparation()).map_err(error)
    }
    /// Interpretation and boundary diagnostics.
    pub fn warnings_json(&self) -> Result<String, JsValue> {
        serde_json::to_string(self.inner.warnings()).map_err(error)
    }
    /// Matrix dimensions: R rows, k columns.
    pub fn shape(&self) -> Vec<u32> {
        vec![self.inner.r().len() as u32, self.inner.k().len() as u32]
    }
    /// Independent inverse-angstrom coordinates, including explicit padding.
    pub fn k(&self) -> Vec<f64> {
        self.inner.k().to_vec()
    }
    /// Independent angstrom coordinates, not phase-corrected distances.
    pub fn r(&self) -> Vec<f64> {
        self.inner.r().to_vec()
    }
    /// Original measured k, before resampling.
    pub fn input_k(&self) -> Vec<f64> {
        self.inner.input_k().to_vec()
    }
    /// Original unweighted chi, unchanged.
    pub fn input_chi(&self) -> Vec<f64> {
        self.inner.input_chi().to_vec()
    }
    /// Resampled unweighted chi; padding outside selected support is zero.
    pub fn prepared_chi(&self) -> Vec<f64> {
        self.inner.prepared_chi().to_vec()
    }
    /// Support/taper multipliers, independent of the display.
    pub fn window(&self) -> Vec<f64> {
        self.inner.window().to_vec()
    }
    /// One for measured support, zero for padding.
    pub fn support(&self) -> Vec<u8> {
        self.inner.support().iter().map(|v| u8::from(*v)).collect()
    }
    /// Flat real values; index row*k_columns+column.
    pub fn real(&self) -> Vec<f64> {
        self.inner.real().to_vec()
    }
    /// Flat imaginary values in the same row-major layout.
    pub fn imaginary(&self) -> Vec<f64> {
        self.inner.imaginary().to_vec()
    }
    /// Full native magnitude, without color normalization or display resampling.
    pub fn magnitude(&self) -> Vec<f64> {
        self.inner.magnitude()
    }
    /// Phase in radians; NaN denotes masked zero/low amplitude. Fraction is in [0,1].
    pub fn phase(&self, relative_floor: f64) -> Result<Vec<f64>, JsValue> {
        Ok(self
            .inner
            .phase(relative_floor)
            .map_err(error)?
            .into_iter()
            .map(|v| v.unwrap_or(f64::NAN))
            .collect())
    }
    /// Native magnitude versus k at a covered R (angstroms).
    pub fn slice_at_r(&self, r: f64) -> Result<Vec<f64>, JsValue> {
        self.inner.slice_at_r(r).map_err(error)
    }
    /// Native magnitude versus R at a covered k (inverse angstroms).
    pub fn slice_at_k(&self, k: f64) -> Result<Vec<f64>, JsValue> {
        self.inner.slice_at_k(k).map_err(error)
    }
    /// Exact covered-rectangle integral of native bilinear magnitude, with units/provenance.
    pub fn integral_json(&self, k0: f64, k1: f64, r0: f64, r1: f64) -> Result<String, JsValue> {
        serde_json::to_string(&self.inner.integral(k0..=k1, r0..=r1).map_err(error)?).map_err(error)
    }
}
