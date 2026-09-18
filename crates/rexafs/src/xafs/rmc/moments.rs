//! Controlled Taylor summation of path-length phases (since 0.2.10).
use super::*;
use num_complex::Complex64;

/// Optional moment approximation for a group sharing one scattering table.
/// These are rexafs error controls, not EVAX's historical expansion settings.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct MomentSettings {
    /// Absolute tolerance for the dimensionless complex sum of unit phasors;
    /// default 1e-10. Multiply by the scattering amplitude to obtain a χ error scale.
    pub tolerance: f64,
    /// Maximum Taylor order, 1..=64; default 24. Failure to meet tolerance falls
    /// back to direct sine/cosine summation rather than accepting a poor expansion.
    pub max_order: usize,
}
impl Default for MomentSettings {
    fn default() -> Self {
        Self {
            tolerance: 1e-10,
            max_order: 24,
        }
    }
}
impl MomentSettings {
    pub(super) fn validate(&self) -> Result<(), RmcError> {
        require(
            self.tolerance.is_finite() && self.tolerance > 0. && (1..=64).contains(&self.max_order),
            "invalid path moment settings",
        )
    }
}
/// One evaluated phasor sum and the method selected for it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MomentEvaluation {
    /// `Σ exp(2 i k R_j)`, dimensionless, for the supplied path half-lengths R_j in Å.
    pub sum: Complex64,
    /// Taylor order if the moment approximation was used; None for direct summation.
    pub order: Option<usize>,
    /// Analytic Taylor remainder bound plus a conservative first-order floating
    /// accumulation estimate. This excludes platform sine/cosine library error.
    /// Direct fallback reports zero Taylor remainder via this value.
    pub truncation_bound: f64,
}
/// Precomputed normalized path-length moments. Each listed path has weight one;
/// weighted structures are mixed by the ensemble calculator after path summation.
/// The expansion centers lengths at their arithmetic mean to reduce its argument.
pub struct PathMomentExpansion {
    lengths: Vec<f64>,
    center: f64,
    radius: f64,
    moments: Vec<f64>,
    settings: MomentSettings,
}
impl PathMomentExpansion {
    /// Copy finite positive half-lengths (Å) and form moments once, in input order.
    /// At most one million paths are allowed. Empty groups are rejected.
    pub fn new(lengths: &[f64], settings: MomentSettings) -> Result<Self, RmcError> {
        settings.validate()?;
        require(
            !lengths.is_empty()
                && lengths.len() <= 1_000_000
                && lengths
                    .iter()
                    .all(|v| v.is_finite() && *v > 0. && *v <= 1e8),
            "invalid path lengths for moment expansion",
        )?;
        let center = lengths
            .iter()
            .map(|v| v / lengths.len() as f64)
            .sum::<f64>();
        let radius = lengths
            .iter()
            .map(|r| (r - center).abs())
            .fold(0., f64::max);
        let mut moments = vec![0.; settings.max_order + 1];
        for &length in lengths {
            let x = if radius > 0. {
                (length - center) / radius
            } else {
                0.
            };
            let mut power = 1.;
            for m in &mut moments {
                *m += power;
                power *= x;
            }
        }
        Ok(Self {
            lengths: lengths.to_vec(),
            center,
            radius,
            moments,
            settings,
        })
    }
    /// Evaluate at one finite wave number in Å⁻¹. The mathematical remainder
    /// after order n is bounded by `N |2 k radius|^(n+1)/(n+1)!`, because all
    /// derivatives of exp(i x) on real x have magnitude one. Cancellation or
    /// excessive work triggers direct summation. No fixed-order accuracy is assumed.
    pub fn evaluate(&self, k: f64) -> Result<MomentEvaluation, RmcError> {
        require(
            k.is_finite() && (2. * k * self.center).is_finite(),
            "invalid moment wave number",
        )?;
        let t = 2. * k * self.radius;
        if self.radius == 0. {
            return Ok(MomentEvaluation {
                sum: Complex64::from_polar(self.lengths.len() as f64, 2. * k * self.center),
                order: Some(0),
                truncation_bound: 0.,
            });
        }
        if t.abs() > 32. || self.lengths.len() < 8 {
            return Ok(self.direct(k));
        }
        let mut coefficient = Complex64::new(1., 0.);
        let mut sum = Complex64::new(0., 0.);
        let mut absolute = 0.;
        let mut power = 1.;
        for (n, &moment) in self.moments.iter().enumerate() {
            let term = coefficient * moment;
            sum += term;
            absolute += term.norm();
            power *= t.abs() / (n + 1) as f64;
            let truncation = self.lengths.len() as f64 * power;
            let rounding = 16. * f64::EPSILON * (n + 1 + self.lengths.len()) as f64 * absolute;
            if truncation + rounding <= self.settings.tolerance {
                return Ok(MomentEvaluation {
                    sum: sum * Complex64::from_polar(1., 2. * k * self.center),
                    order: Some(n),
                    truncation_bound: truncation + rounding,
                });
            }
            coefficient *= Complex64::new(0., t / (n + 1) as f64);
        }
        Ok(self.direct(k))
    }
    fn direct(&self, k: f64) -> MomentEvaluation {
        MomentEvaluation {
            sum: self
                .lengths
                .iter()
                .map(|r| Complex64::from_polar(1., 2. * k * r))
                .sum(),
            order: None,
            truncation_bound: 0.,
        }
    }
    /// Maximum measured absolute difference from direct summation on supplied
    /// wave numbers. Use this diagnostic for representative groups before a run.
    pub fn check_direct(&self, k: &[f64]) -> Result<f64, RmcError> {
        let mut maximum: f64 = 0.;
        for &q in k {
            maximum = maximum.max((self.evaluate(q)?.sum - self.direct(q).sum).norm());
        }
        Ok(maximum)
    }
}
