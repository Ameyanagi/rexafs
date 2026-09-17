//! Composite XANES peak, edge-step and baseline fitting (unreleased).
//!
//! [`PeakFit`] uses normalized μ(E) by default and energy offsets from E₀.
//! It prepares missing normalization on an owned copy and leaves the source
//! spectrum unchanged. Peak areas are whole-axis model integrals in signal
//! units × eV; widths are full widths at half maximum (FWHM) in eV. A step has
//! a height and scale, not a finite peak area. Components have stable names;
//! fitting does not assign chemical identities or choose a component count.
//!
//! Gaussian, Lorentzian, pseudo-Voigt and Voigt definitions follow the
//! [lmfit profile reference](https://lmfit.github.io/lmfit-py/builtin_models.html).
//! Rexafs exposes FWHM rather than sigma and refines baseline and peaks jointly.
//! Local covariance is conditional on the chosen model and supplied noise;
//! nonconvergence, active bounds or deficient rank withhold it.

mod optimizer;
mod profiles;
mod solver;
#[cfg(test)]
mod tests;
use crate::xafs::{
    analysis::metrics::{AxisOrigin, Measurement, MeasurementSpace},
    fitting::{FitVariable, FitVariables, Param},
    xasspectrum::XASSpectrum,
};
pub use profiles::PeakShape;
use serde::{Deserialize, Serialize};
pub use solver::{PeakContribution, PeakFitResult, PeakTermination};
use std::{
    collections::{BTreeMap, BTreeSet},
    ops::RangeInclusive,
};

/// Scientific role, separate from the component's mathematical shape.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PeakRole {
    /// Include this peak's area and center in the model's area-weighted center.
    Peak,
    /// Background contribution; excluded from the peak-center average.
    Baseline,
    /// Absorption step; has no whole-axis area.
    Edge,
}

/// Named component; parameter names refer to the model's restricted expression
/// namespace, allowing widths or centers to be shared without executable code.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PeakComponent {
    /// Stable unique ASCII identifier, such as `white_line`.
    pub name: String,
    /// Shape evaluator and the required parameter order.
    pub shape: PeakShape,
    /// Whether this contribution is a peak, baseline or edge step.
    pub role: PeakRole,
    /// Shape parameter role to variable name, e.g. `width` → `p1_width`.
    pub parameters: BTreeMap<String, String>,
}

/// Frozen composite fit definition. Constructors use Norm and E₀-relative eV.
///
/// Recommended workflow: `PeakFit::new(-20.0..=40.0)
/// .gaussian("p1", 5.0, 2.0, 3.0).linear_baseline(0.0, 0.0).fit(&spectrum)`.
/// Gaussian arguments are name, center, area and FWHM. Use `.flat()` explicitly
/// for flattened absorption. Default bounds keep peak centers inside the fit
/// range, areas nonnegative, widths positive, and mixture fractions in [0,1].
/// Validation happens when evaluating/fitting; malformed names, ties, bounds,
/// nonfinite arrays and insufficient coverage return [`PeakFitError`]. Public
/// fields support advanced editing and serialization. Inputs are never modified.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PeakFit {
    /// Inclusive fit interval in eV under `origin`; native points only.
    pub range: [f64; 2],
    /// Mu, Norm (default), or Flat. k/R/derivative representations are not peaks.
    pub space: MeasurementSpace,
    /// Interpretation of ranges, centers and baseline reference energies.
    pub origin: AxisOrigin,
    /// Inclusive excluded intervals in the same coordinates; no hidden smoothing.
    pub exclude: Vec<[f64; 2]>,
    /// Ordered stable components; baseline terms are fitted jointly.
    pub components: Vec<PeakComponent>,
    /// Free/fixed/bounded/tied values; names generated as `component_parameter`.
    pub parameters: FitVariables,
    /// Maximum damped Gauss–Newton iterations, default 200; must be positive.
    pub max_iterations: usize,
    /// Relative objective/step tolerance, default 1e-10; finite and positive.
    pub tolerance: f64,
}

/// Invalid models/data fail before numerical optimization; optimizer termination
/// is also recorded in successful result objects, including nonconvergence.
#[derive(Debug, thiserror::Error)]
pub enum PeakFitError {
    /// Unsupported or inconsistent model, range, bounds, masks or source arrays.
    #[error("Invalid peak fit: {0}")]
    Invalid(String),
    /// Required source representation could not be prepared without guessing.
    #[error("Peak-fit preparation: {0}")]
    Preparation(String),
    /// A model evaluation or optimizer operation produced invalid arithmetic.
    #[error("Peak-fit numerical error: {0}")]
    Numerical(String),
}

impl PeakFit {
    /// Fit normalized μ(E) over inclusive E₀-relative offsets, in eV.
    /// The model initially has no components; add at least one before fitting.
    pub fn new(range: RangeInclusive<f64>) -> Self {
        Self {
            range: [*range.start(), *range.end()],
            space: MeasurementSpace::Norm,
            origin: AxisOrigin::E0,
            exclude: Vec::new(),
            components: Vec::new(),
            parameters: FitVariables::new(),
            max_iterations: 200,
            tolerance: 1e-10,
        }
    }
    /// Select flattened dimensionless μ(E); missing preparation runs on a copy.
    pub fn flat(mut self) -> Self {
        self.space = MeasurementSpace::Flat;
        self
    }
    /// Select mapped raw μ(E), with its original signal units.
    pub fn raw_mu(mut self) -> Self {
        self.space = MeasurementSpace::Mu;
        self
    }
    /// Use absolute eV for ranges and component centers/reference energies.
    pub fn absolute(mut self) -> Self {
        self.origin = AxisOrigin::Absolute;
        self
    }
    /// Use offsets from a frozen reference energy in eV, shared across spectra.
    pub fn reference(mut self, energy_ev: f64) -> Self {
        self.origin = AxisOrigin::Reference { energy_ev };
        self
    }
    /// Exclude an inclusive interval from the final fit; coordinates use `origin`.
    pub fn exclude(mut self, range: RangeInclusive<f64>) -> Self {
        self.exclude.push([*range.start(), *range.end()]);
        self
    }
    /// Add a Gaussian peak: center and FWHM in eV, area in signal units × eV.
    pub fn gaussian(self, name: impl Into<String>, center: f64, area: f64, fwhm: f64) -> Self {
        self.component(
            name.into(),
            PeakShape::Gaussian,
            PeakRole::Peak,
            &[center, area, fwhm],
        )
    }
    /// Add a Lorentzian peak with a whole-axis area and FWHM in eV.
    pub fn lorentzian(self, name: impl Into<String>, center: f64, area: f64, fwhm: f64) -> Self {
        self.component(
            name.into(),
            PeakShape::Lorentzian,
            PeakRole::Peak,
            &[center, area, fwhm],
        )
    }
    /// Add a common-FWHM mixture. `fraction` is the Lorentzian share [0,1].
    pub fn pseudo_voigt(
        self,
        name: impl Into<String>,
        center: f64,
        area: f64,
        fwhm: f64,
        fraction: f64,
    ) -> Self {
        self.component(
            name.into(),
            PeakShape::PseudoVoigt,
            PeakRole::Peak,
            &[center, area, fwhm, fraction],
        )
    }
    /// Add a true convolution with independent Gaussian/Lorentzian FWHM in eV.
    /// One width may be fixed to zero for a limiting profile; not both.
    pub fn voigt(
        self,
        name: impl Into<String>,
        center: f64,
        area: f64,
        gaussian_fwhm: f64,
        lorentzian_fwhm: f64,
    ) -> Self {
        self.component(
            name.into(),
            PeakShape::Voigt,
            PeakRole::Peak,
            &[center, area, gaussian_fwhm, lorentzian_fwhm],
        )
    }
    /// Add `height*(1+erf((E-center)/scale))/2`. Scale is positive eV.
    pub fn erf_step(self, name: impl Into<String>, center: f64, height: f64, scale: f64) -> Self {
        self.component(
            name.into(),
            PeakShape::ErfStep,
            PeakRole::Edge,
            &[center, height, scale],
        )
    }
    /// Add `height*(1/2+atan((E-center)/scale)/pi)`. Scale is positive eV.
    pub fn arctan_step(
        self,
        name: impl Into<String>,
        center: f64,
        height: f64,
        scale: f64,
    ) -> Self {
        self.component(
            name.into(),
            PeakShape::ArctanStep,
            PeakRole::Edge,
            &[center, height, scale],
        )
    }
    /// Add a fitted constant baseline named `baseline`, in signal units.
    pub fn constant_baseline(self, offset: f64) -> Self {
        self.component(
            "baseline".into(),
            PeakShape::Constant,
            PeakRole::Baseline,
            &[offset],
        )
    }
    /// Add `offset+slope*E_offset`, named `baseline`. Slope is signal units/eV;
    /// the fixed reference is zero in the chosen coordinate system.
    pub fn linear_baseline(self, offset: f64, slope: f64) -> Self {
        self.component(
            "baseline".into(),
            PeakShape::Linear,
            PeakRole::Baseline,
            &[offset, slope, 0.],
        )
    }
    /// Insert/replace a named parameter. Use existing [`Param::fixed`],
    /// [`Param::bounds`] or [`Param::expr`] for fixed values, bounds or ties.
    pub fn parameter(mut self, parameter: Param) -> Self {
        self.parameters
            .insert(parameter.name.clone(), parameter.to_fit_variable());
        self
    }
    fn component(mut self, name: String, shape: PeakShape, role: PeakRole, values: &[f64]) -> Self {
        let mut parameters = BTreeMap::new();
        for (&key, &value) in shape.parameter_names().iter().zip(values) {
            let symbol = format!("{name}_{key}");
            let (min, max) = match key {
                "center" => (Some(self.range[0]), Some(self.range[1])),
                "area" => (Some(0.), None),
                "width" | "lorentz_width" if shape == PeakShape::Voigt => (Some(0.), None),
                "width" | "scale" => (Some(1e-8), None),
                "fraction" => (Some(0.), Some(1.)),
                _ => (None, None),
            };
            self.parameters.insert(
                symbol.clone(),
                FitVariable::new(value, key != "reference").with_bounds(min, max),
            );
            parameters.insert(key.into(), symbol);
        }
        self.components.push(PeakComponent {
            name,
            shape,
            role,
            parameters,
        });
        self
    }
    /// Fit on an immutable spectrum. Missing μ normalization runs on a copy;
    /// no interpolation, smoothing or EXAFS background processing is introduced.
    /// Inspect termination and uncertainty availability even after `Ok`.
    pub fn fit(&self, spectrum: &XASSpectrum) -> Result<PeakFitResult, PeakFitError> {
        self.fit_with_errors(spectrum, None)
    }
    /// Fit with optional independent one-standard-deviation errors already in
    /// the selected signal representation, one per native spectrum point.
    /// Errors must be finite and positive; `None` means unweighted least squares.
    /// Covariance then uses residual-based variance scaling, not known detector noise.
    pub fn fit_with_errors(
        &self,
        spectrum: &XASSpectrum,
        errors: Option<&[f64]>,
    ) -> Result<PeakFitResult, PeakFitError> {
        self.validate()?;
        let arrays = self
            .measurement()
            .arrays(spectrum)
            .map_err(|e| PeakFitError::Preparation(e.to_string()))?;
        self.fit_prepared(&arrays.axis, &arrays.signal, arrays.e0_ev, errors)
    }
    /// Estimate only baseline-role parameters outside additional peak intervals,
    /// then return a new starting model for the final joint fit. The source and
    /// this definition are unchanged. These extra intervals are not retained as
    /// final-fit masks. Existing final masks still apply during initialization.
    /// At least one baseline component is required. Ties to non-baseline variables
    /// use those variables' fixed initial values during this preliminary fit.
    pub fn initialize_baseline(
        &self,
        spectrum: &XASSpectrum,
        peak_intervals: &[RangeInclusive<f64>],
    ) -> Result<Self, PeakFitError> {
        self.initialize_baseline_with_progress(spectrum, peak_intervals, |_, _| true)
    }
    /// Baseline initialization with cancellation between optimizer iterations.
    /// The callback receives iteration and squared residual objective; returning
    /// false rejects the incomplete initialization and leaves this model unchanged.
    pub fn initialize_baseline_with_progress(
        &self,
        spectrum: &XASSpectrum,
        peak_intervals: &[RangeInclusive<f64>],
        progress: impl FnMut(usize, f64) -> bool,
    ) -> Result<Self, PeakFitError> {
        self.validate()?;
        let mut baseline = self.clone();
        baseline.components.retain(|c| c.role == PeakRole::Baseline);
        let names: BTreeSet<_> = baseline
            .components
            .iter()
            .flat_map(|c| c.parameters.values().cloned())
            .collect();
        for (name, value) in &mut baseline.parameters.vars {
            if !names.contains(name) {
                value.vary = false;
            }
        }
        baseline
            .exclude
            .extend(peak_intervals.iter().map(|r| [*r.start(), *r.end()]));
        let fit = baseline.fit_with_progress(spectrum, progress)?;
        if !matches!(
            fit.termination,
            PeakTermination::Converged | PeakTermination::FixedModel
        ) {
            return Err(PeakFitError::Numerical(format!(
                "baseline initialization: {}",
                fit.termination_detail
            )));
        }
        let mut initialized = self.clone();
        for name in names {
            let variable = initialized.parameters.vars.get_mut(&name).unwrap();
            variable.value = fit.parameters.vars[&name].value;
            variable.init_value = variable.value;
            variable.stderr = None;
        }
        Ok(initialized)
    }
    /// Advanced entry point for already-prepared arrays. Energy is absolute eV;
    /// `e0` is required only for E₀-relative definitions. No preparation is done.
    /// Arrays and optional errors must match, be finite and use increasing energy.
    pub fn fit_prepared(
        &self,
        energy: &[f64],
        signal: &[f64],
        e0: Option<f64>,
        errors: Option<&[f64]>,
    ) -> Result<PeakFitResult, PeakFitError> {
        self.fit_prepared_with_progress(energy, signal, e0, errors, |_, _| true)
    }
    /// Fit prepared arrays with cancellation between solver iterations. The
    /// callback receives the zero-based iteration and current weighted objective;
    /// returning false yields a Cancelled result with the best accepted model.
    /// Cancellation withholds uncertainty; it is never labeled convergence.
    pub fn fit_prepared_with_progress(
        &self,
        energy: &[f64],
        signal: &[f64],
        e0: Option<f64>,
        errors: Option<&[f64]>,
        mut progress: impl FnMut(usize, f64) -> bool,
    ) -> Result<PeakFitResult, PeakFitError> {
        solver::fit(self, energy, signal, e0, errors, &mut progress)
    }
    /// Prepare the selected representation on a copy and fit with per-iteration
    /// progress/cancellation, as in [`Self::fit_prepared_with_progress`].
    pub fn fit_with_progress(
        &self,
        spectrum: &XASSpectrum,
        progress: impl FnMut(usize, f64) -> bool,
    ) -> Result<PeakFitResult, PeakFitError> {
        self.validate()?;
        let arrays = self
            .measurement()
            .arrays(spectrum)
            .map_err(|e| PeakFitError::Preparation(e.to_string()))?;
        self.fit_prepared_with_progress(&arrays.axis, &arrays.signal, arrays.e0_ev, None, progress)
    }
    fn measurement(&self) -> Measurement {
        let mut measurement = Measurement::mean(self.range[0]..=self.range[1]);
        measurement.space = self.space;
        measurement.origin = self.origin;
        measurement
    }
    /// Evaluate the starting model at absolute-energy points without fitting or
    /// masking. This is useful for previews. `e0` is required only for E₀-relative
    /// definitions. Points and model values must be finite; output is owned.
    pub fn evaluate(&self, energy: &[f64], e0: Option<f64>) -> Result<Vec<f64>, PeakFitError> {
        self.validate()?;
        let origin = match self.origin {
            AxisOrigin::Absolute => Some(0.),
            AxisOrigin::Reference { energy_ev } => Some(energy_ev),
            AxisOrigin::E0 => e0,
        }
        .filter(|e| e.is_finite())
        .ok_or_else(|| PeakFitError::Invalid("finite E₀ is required".into()))?;
        let values = self
            .parameters
            .resolve_values()
            .map_err(|e| PeakFitError::Invalid(e.to_string()))?;
        let components = self
            .components
            .iter()
            .map(|c| Ok((c.shape, c.values(&values)?)))
            .collect::<Result<Vec<_>, PeakFitError>>()?;
        energy
            .iter()
            .map(|&e| {
                let y = components
                    .iter()
                    .map(|(shape, p)| profiles::evaluate(*shape, e - origin, p))
                    .sum::<f64>();
                if e.is_finite() && (e - origin).is_finite() && y.is_finite() {
                    Ok(y)
                } else {
                    Err(PeakFitError::Numerical(
                        "nonfinite preview energy or model".into(),
                    ))
                }
            })
            .collect()
    }
    /// Independent fits with the same frozen starting model in input order.
    /// Each source keeps its own success/error; one bad spectrum never removes a row.
    pub fn fit_batch(&self, spectra: &[XASSpectrum]) -> Vec<Result<PeakFitResult, PeakFitError>> {
        spectra.iter().map(|s| self.fit(s)).collect()
    }
    /// Validate model structure, finite settings, physical domains and restricted
    /// expression ties. Does not calculate normalization or change parameters.
    pub fn validate(&self) -> Result<(), PeakFitError> {
        let invalid = |s: &str| PeakFitError::Invalid(s.into());
        if !self.range.iter().all(|v| v.is_finite()) || self.range[0] >= self.range[1] {
            return Err(invalid("fit range must have finite increasing bounds"));
        }
        if matches!(self.origin, AxisOrigin::Reference { energy_ev } if !energy_ev.is_finite()) {
            return Err(invalid("reference energy must be finite"));
        }
        if !matches!(
            self.space,
            MeasurementSpace::Mu | MeasurementSpace::Norm | MeasurementSpace::Flat
        ) {
            return Err(invalid("peak fitting requires Mu, Norm or Flat"));
        }
        if self.components.is_empty()
            || self.max_iterations == 0
            || !self.tolerance.is_finite()
            || self.tolerance <= 0.
        {
            return Err(invalid(
                "add components and use positive finite solver settings",
            ));
        }
        if self
            .exclude
            .iter()
            .any(|r| !r.iter().all(|v| v.is_finite()) || r[0] > r[1])
        {
            return Err(invalid("excluded intervals need finite ordered bounds"));
        }
        let identifier = |name: &str| {
            !name.is_empty()
                && name.len() <= 100
                && name.bytes().enumerate().all(|(i, b)| {
                    b == b'_' || b.is_ascii_alphabetic() || (i > 0 && b.is_ascii_digit())
                })
        };
        let mut names = BTreeSet::new();
        for component in &self.components {
            if !identifier(&component.name) || !names.insert(&component.name) {
                return Err(invalid("component names must be unique ASCII identifiers"));
            }
            if component.parameters.len() != component.shape.parameter_names().len()
                || component
                    .shape
                    .parameter_names()
                    .iter()
                    .any(|key| !component.parameters.contains_key(*key))
            {
                return Err(invalid("component parameter roles do not match its shape"));
            }
            if component.shape.is_step() && component.role != PeakRole::Edge
                || component.shape.is_peak() && component.role == PeakRole::Edge
                || matches!(component.shape, PeakShape::Constant | PeakShape::Linear)
                    && component.role != PeakRole::Baseline
            {
                return Err(invalid(
                    "steps require Edge roles and polynomials require Baseline roles",
                ));
            }
        }
        for (name, value) in &self.parameters.vars {
            if !identifier(name)
                || !value.value.is_finite()
                || !value.init_value.is_finite()
                || value.stderr.is_some_and(|v| !v.is_finite() || v < 0.)
                || value
                    .min
                    .into_iter()
                    .chain(value.max)
                    .any(|v| !v.is_finite())
                || value
                    .min
                    .zip(value.max)
                    .is_some_and(|(a, b)| a > b || (a == b && value.vary && value.expr.is_none()))
            {
                return Err(invalid(
                    "parameter values/bounds must be finite, ordered and identifiable",
                ));
            }
        }
        let resolved = self
            .parameters
            .resolve_values()
            .map_err(|e| PeakFitError::Invalid(e.to_string()))?;
        self.validate_values(&resolved)
    }
    pub(crate) fn validate_values(
        &self,
        values: &BTreeMap<String, f64>,
    ) -> Result<(), PeakFitError> {
        for (name, variable) in &self.parameters.vars {
            let value = *values
                .get(name)
                .ok_or_else(|| PeakFitError::Invalid(format!("missing parameter {name}")))?;
            if !value.is_finite()
                || variable.min.is_some_and(|min| value < min)
                || variable.max.is_some_and(|max| value > max)
            {
                return Err(PeakFitError::Invalid(format!(
                    "parameter {name} is nonfinite or outside its bounds"
                )));
            }
        }
        for component in &self.components {
            let p = component.values(values)?;
            let shape = component.shape;
            if shape.is_peak() && p[1] < 0.
                || (shape.is_step()
                    || matches!(
                        shape,
                        PeakShape::Gaussian | PeakShape::Lorentzian | PeakShape::PseudoVoigt
                    ))
                    && p[2] <= 0.
                || shape == PeakShape::PseudoVoigt && !(0.0..=1.0).contains(&p[3])
                || shape == PeakShape::Voigt && (p[2] < 0. || p[3] < 0. || p[2] + p[3] <= 0.)
            {
                return Err(PeakFitError::Invalid(format!(
                    "invalid physical parameters for {}",
                    component.name
                )));
            }
        }
        Ok(())
    }
}
impl XASSpectrum {
    /// Joint XANES peak/baseline fit using the supplied frozen model. Norm and
    /// E₀-relative eV are the model defaults. Missing preparation runs on a copy;
    /// neither source arrays nor settings are changed. Inspect result termination
    /// and uncertainty availability. Equivalent to `model.fit(self)`.
    pub fn fit_peaks(&self, model: &PeakFit) -> Result<PeakFitResult, PeakFitError> {
        model.fit(self)
    }
}
impl PeakComponent {
    pub(crate) fn values(&self, values: &BTreeMap<String, f64>) -> Result<Vec<f64>, PeakFitError> {
        self.shape
            .parameter_names()
            .iter()
            .map(|key| {
                let name = self
                    .parameters
                    .get(*key)
                    .ok_or_else(|| PeakFitError::Invalid(format!("missing role {key}")))?;
                values
                    .get(name)
                    .copied()
                    .ok_or_else(|| PeakFitError::Invalid(format!("undefined parameter {name}")))
            })
            .collect()
    }
}
