//! Thin correction adapter; all numerical/domain checks remain in the native core.
use wasm_bindgen::prelude::*;
pub(crate) fn error(e: impl std::fmt::Display) -> JsValue {
    js_sys::Error::new(&e.to_string()).into()
}
pub(crate) fn mode_name(mode: rexafs::AbsorptionMode) -> &'static str {
    match mode {
        rexafs::AbsorptionMode::Unknown => "unknown",
        rexafs::AbsorptionMode::Transmission => "transmission",
        rexafs::AbsorptionMode::Fluorescence => "fluorescence",
    }
}
pub(crate) fn mode(value: &str) -> Result<rexafs::AbsorptionMode, JsValue> {
    match value {
        "unknown" => Ok(rexafs::AbsorptionMode::Unknown),
        "transmission" => Ok(rexafs::AbsorptionMode::Transmission),
        "fluorescence" => Ok(rexafs::AbsorptionMode::Fluorescence),
        _ => Err(error("mode must be unknown, transmission or fluorescence")),
    }
}
pub(crate) fn result_json(
    result: &rexafs::FluorescenceCorrectionResult,
) -> Result<String, JsValue> {
    // Include the core's replay definition; JavaScript must not derive scientific settings.
    serde_json::to_string(&(result, result.definition())).map_err(error)
}
/// Owned native correction settings. Explicit composition, emission and surface angles.
#[wasm_bindgen(js_name=FluorescenceCorrection)]
pub struct WasmFluorescenceCorrection {
    pub(crate) inner: rexafs::FluorescenceCorrection,
}
#[wasm_bindgen(js_class=FluorescenceCorrection)]
impl WasmFluorescenceCorrection {
    /// Decode a native definition. Calculation validates scientific settings.
    #[wasm_bindgen(constructor)]
    pub fn new(json: &str) -> Result<Self, JsValue> {
        Ok(Self {
            inner: serde_json::from_str(json).map_err(error)?,
        })
    }
    /// Copy original fluorescence arrays and calculate on the native energy grid (eV).
    /// The result includes historical inputs, internal normalization and replay settings.
    pub fn apply_json(&self, energy: &[f64], mu: &[f64]) -> Result<String, JsValue> {
        result_json(&self.inner.apply(energy, mu).map_err(error)?)
    }
    /// Native settings with requested automatic choices or pinned replay identity.
    pub fn to_json(&self) -> Result<String, JsValue> {
        serde_json::to_string(&self.inner).map_err(error)
    }
}
