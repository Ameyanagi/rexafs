//! Frozen two-dimensional wavelet measurements. Results retain both physical
//! intervals and the numerical convention, without storing every frame's map.
use super::*;
use rexafs::{Wavelet, WaveletMap, WaveletRegionValue};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WaveletStatistic {
    Integral,
    Maximum,
    Mean,
}
impl WaveletStatistic {
    pub fn label(self) -> &'static str {
        match self {
            Self::Integral => "Integral",
            Self::Maximum => "Maximum",
            Self::Mean => "Mean",
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WaveletTrend {
    /// A snapshot of Transform's wavelet settings, shared by every frame.
    pub transform: Wavelet,
    pub statistic: WaveletStatistic,
    pub k_range: [f64; 2],
    pub r_range: [f64; 2],
}
impl WaveletTrend {
    pub fn validate(&self) -> Result<(), String> {
        let [ka, kb] = self.k_range;
        let [ra, rb] = self.r_range;
        if ![ka, kb, ra, rb].iter().all(|v| v.is_finite()) || ka >= kb || ra >= rb || ra <= 0. {
            return Err("Choose increasing k and R ranges; R must be positive".into());
        }
        if ka < self.transform.k_range[0] || kb > self.transform.k_range[1] {
            return Err("The region must be inside the Transform k range".into());
        }
        let (r_min, r_max) = self
            .transform
            .radii
            .as_ref()
            .filter(|r| !r.is_empty())
            .map(|r| (r[0], *r.last().unwrap()))
            .unwrap_or((0., self.transform.rmax));
        if ra < r_min || rb > r_max {
            return Err("The region must be inside the Transform R range".into());
        }
        // Validate the definition and resource limits before starting a job.
        self.transform
            .estimate(&self.transform.k_range)
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    pub fn measure(&self, map: &WaveletMap) -> Result<WaveletRegionValue, String> {
        self.validate()?;
        if map.settings() != &self.transform {
            return Err("The preview uses different Transform settings; refresh it".into());
        }
        let k = self.k_range[0]..=self.k_range[1];
        let r = self.r_range[0]..=self.r_range[1];
        match self.statistic {
            WaveletStatistic::Integral => map.integral(k, r),
            WaveletStatistic::Maximum => map.maximum(k, r),
            WaveletStatistic::Mean => map.mean(k, r),
        }
        .map_err(|e| e.to_string())
    }
}

/// Common scalar columns, with either a one-dimensional measurement or a
/// two-dimensional wavelet result. Legacy MeasurementResult JSON remains readable.
#[derive(Clone, Serialize, Deserialize)]
pub struct MetricValue {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub measurement: Option<Measurement>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wavelet: Option<WaveletRegionValue>,
    pub value: f64,
    pub position: Option<f64>,
    /// One-dimensional axis interval, or k interval when `wavelet` is present.
    pub range: [f64; 2],
    pub standard_error: Option<f64>,
    pub unit: String,
    pub e0_ev: Option<f64>,
}
impl From<MeasurementResult> for MetricValue {
    fn from(v: MeasurementResult) -> Self {
        Self {
            measurement: Some(v.measurement),
            wavelet: None,
            value: v.value,
            position: v.position,
            range: v.range,
            standard_error: v.standard_error,
            unit: v.unit,
            e0_ev: v.e0_ev,
        }
    }
}
impl From<WaveletRegionValue> for MetricValue {
    fn from(v: WaveletRegionValue) -> Self {
        Self {
            measurement: None,
            value: v.value,
            position: None,
            range: v.k_range,
            standard_error: None,
            unit: v.unit.clone(),
            e0_ev: None,
            wavelet: Some(v),
        }
    }
}
