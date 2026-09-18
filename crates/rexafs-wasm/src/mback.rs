//! Thin native MBACK adapters; JavaScript supplies named options, Rust the science.
use serde::Deserialize;
use wasm_bindgen::prelude::*;
fn error(e: impl std::fmt::Display) -> JsValue {
    js_sys::Error::new(&e.to_string()).into()
}
#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct Options {
    e0: Option<f64>,
    pre_edge: Option<[f64; 2]>,
    post_edge: Option<[f64; 2]>,
    degree: Option<usize>,
    erfc: Option<rexafs::xafs::mback::MbackErfc>,
}

/// Full Chantler MBACK with explicit absorber/edge and checked fitting ranges.
#[wasm_bindgen(js_name=MBack)]
pub struct WasmMBack {
    pub(crate) inner: rexafs::MBack,
}
#[wasm_bindgen(js_class=MBack)]
impl WasmMBack {
    /// Construct settings from named facade options. All calculations run in Rust.
    #[wasm_bindgen(constructor)]
    pub fn new(element: &str, edge: &str, options_json: &str) -> Result<Self, JsValue> {
        let options: Options = serde_json::from_str(options_json).map_err(error)?;
        let mut inner = rexafs::MBack::for_edge(element, edge);
        inner.e0 = options.e0;
        inner.options.pre_edge = options.pre_edge;
        inner.options.post_edge = options.post_edge;
        inner.options.degree = options.degree.unwrap_or(2);
        inner.options.erfc = options.erfc;
        Ok(Self { inner })
    }
    /// Fit borrowed raw absorption arrays (energy eV); retain complete native results.
    pub fn fit_json(&self, energy: &[f64], mu: &[f64]) -> Result<String, JsValue> {
        let result = self.inner.fit(energy, mu).map_err(error)?;
        serde_json::to_string(&result).map_err(error)
    }
    /// Serialize the model without adding input arrays.
    pub fn to_json(&self) -> Result<String, JsValue> {
        serde_json::to_string(&self.inner).map_err(error)
    }
    /// Restore native settings; numerical/data identity checks occur during fit.
    pub fn from_json(json: &str) -> Result<Self, JsValue> {
        Ok(Self {
            inner: serde_json::from_str(json).map_err(error)?,
        })
    }
}
