//! Checked, provenance-preserving spectrum input (since 0.2.10).
use super::*;
use crate::fitting::{FeffFitTransform, FitSpace};
use crate::{BackgroundMethod, Spectrum};

/// Policy for the nominal lower R fit bound relative to the saved AUTOBK radius
/// (since 0.2.10). This is a rexafs input guard, not a leakage correction or a
/// physical convergence test. AUTOBK uses the low-R signal to determine background;
/// see the [algorithm documentation](https://xraypy.github.io/xraylarch/xafs_autobk.html).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub enum RmcRbkgPolicy {
    /// Require rmin >= the resolved Rbkg (default). A larger margin must be chosen
    /// explicitly in the fitting transform; no spectrum or bound is adjusted.
    #[default]
    ExcludeBelowRbkg,
    /// Permit rmin < Rbkg for an explicit comparison. The nonblank reason is
    /// retained in the processing snapshot and checkpoint.
    AllowBelowRbkg {
        /// Scientific reason for including the background-sensitive region.
        reason: String,
    },
}

/// Inputs for [`RmcDataset::from_spectrum`] (since 0.2.10). Use [`Self::new`] with
/// explicit absorbers, edge and the same [`FeffFitTransform`] used by ordinary
/// fitting. Plotting FFT settings are preserved as provenance, not adopted as fit
/// ranges. The constructor selects real + imaginary R fitting with one integer
/// k weight; unsupported fit spaces, fractional/multiple weights are errors.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RmcSpectrumOptions {
    /// Distinct, zero-based absorbing atoms; element and bounds are checked once
    /// a configuration is supplied to a session. Must not be empty.
    pub absorbers: Vec<usize>,
    /// Absorption edge; not inferred from the measured edge energy.
    pub edge: Edge,
    /// Explicit native fitting transform, with fitspace=R and one k weight 0..=3.
    /// k is in Å⁻¹, R in Å. Use the measured support and a sample-specific R range.
    pub transform: FeffFitTransform,
    /// Inclusive measured k interval in Å⁻¹, before the fitting window. None copies
    /// all measured points. Include the window tapers. A positive delta_e0 may
    /// require excluding low k where the shifted theoretical k would be imaginary.
    /// Selection preserves samples exactly: no interpolation or weighting here.
    pub k_range: Option<[f64; 2]>,
    /// Positive dimensionless scales aligned with the FULL spectrum k grid, before
    /// selection. None uses unit numerical scales, not estimated measurement noise.
    /// Absorption-space mu_stddev is never treated as EXAFS uncertainty.
    pub sigma: Option<Vec<f64>>,
    /// Fixed positive S₀²; default 1. Calibrate independently when appropriate.
    pub s02: f64,
    /// Fixed fitting energy shift in eV; default 0. This is not Spectrum::e0().
    pub delta_e0: f64,
    /// Normalize squared complex-R residuals by experimental power in the same
    /// window (default true), as in the Cu₂O comparison. With nonuniform sigma,
    /// both numerator and denominator use the supplied scales before transforming.
    /// False retains the ordinary RMC mean squared objective. Neither is reduced χ².
    pub normalize_by_experiment: bool,
    /// Exclude R below Rbkg by default; exceptions require a recorded reason.
    pub rbkg_policy: RmcRbkgPolicy,
}

impl RmcSpectrumOptions {
    /// Select absorbers, edge and explicit fitting settings. Defaults copy all k,
    /// use unit numerical sigma, S₀²=1, ΔE₀=0, experimental-power normalization,
    /// and exclude R below Rbkg. Nothing is calculated or validated until
    /// [`RmcDataset::from_spectrum`] is called. The defaults are not a material
    /// calibration or an experimental uncertainty model.
    pub fn new(absorbers: Vec<usize>, edge: Edge, transform: FeffFitTransform) -> Self {
        Self {
            absorbers,
            edge,
            transform,
            k_range: None,
            sigma: None,
            s02: 1.,
            delta_e0: 0.,
            normalize_by_experiment: true,
            rbkg_policy: RmcRbkgPolicy::default(),
        }
    }
}

/// Owned input snapshot captured before RMC (since 0.2.10). Includes the complete
/// serialized Spectrum state: baseline/current arrays, normalization, AUTOBK,
/// edge energy, and any Fourier settings/results. This preserves available
/// state, not an immutable raw acquisition or a complete processing history.
/// Private fields prevent accidental mutation through the normal Rust API.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RmcSpectrumSource {
    spectrum: Spectrum,
    selected_indices: Vec<usize>,
    options: RmcSpectrumOptions,
    rexafs_version: String,
}

impl RmcSpectrumSource {
    /// Borrow the original captured spectrum without processing or cloning.
    pub fn spectrum(&self) -> &Spectrum {
        &self.spectrum
    }
    /// Original full-grid indices corresponding to the RMC experimental arrays.
    pub fn selected_indices(&self) -> &[usize] {
        &self.selected_indices
    }
    /// Options used at import. Later explicit edits to public dataset fields do
    /// not rewrite this historical record; the dataset holds the current settings.
    pub fn options(&self) -> &RmcSpectrumOptions {
        &self.options
    }
    /// rexafs package version that captured the snapshot. Unreleased checkouts
    /// can share a version; record the source revision separately for reproducibility.
    pub fn rexafs_version(&self) -> &str {
        &self.rexafs_version
    }
}

fn processed(spectrum: &Spectrum) -> Result<(&[f64], &[f64], f64), RmcError> {
    spectrum
        .ensure_exafs_allowed()
        .map_err(|e| RmcError::Invalid(e.to_string()))?;
    let k = spectrum.k().ok_or_else(|| {
        RmcError::Invalid(
            "spectrum has no processed k; call calc_background() explicitly first".into(),
        )
    })?;
    let chi = spectrum.chi().ok_or_else(|| {
        RmcError::Invalid(
            "spectrum has no processed chi; call calc_background() explicitly first".into(),
        )
    })?;
    require(
        k.len() >= 2
            && k.len() == chi.len()
            && k.iter().all(|x| x.is_finite() && *x >= 0.)
            && k.windows(2).all(|w| w[1] > w[0])
            && chi.iter().all(|x| x.is_finite()),
        "spectrum needs finite, paired chi and strictly increasing nonnegative k",
    )?;
    let rbkg = match spectrum.background.as_ref() {
        Some(BackgroundMethod::AUTOBK(background)) => background.rbkg,
        _ => None,
    };
    let rbkg = rbkg.filter(|r| r.is_finite() && *r > 0.).ok_or_else(|| {
        RmcError::Invalid("processed spectrum needs a resolved, positive AUTOBK Rbkg".into())
    })?;
    Ok((k, chi, rbkg))
}

fn check_rbkg(objective: &Objective, rbkg: f64, policy: &RmcRbkgPolicy) -> Result<(), RmcError> {
    if let RmcRbkgPolicy::AllowBelowRbkg { reason } = policy {
        require(
            !reason.trim().is_empty(),
            "Rbkg override needs a nonblank reason",
        )?;
    }
    if let Objective::R(t) | Objective::Q(t) = objective {
        require(
            t.rmin >= rbkg || matches!(policy, RmcRbkgPolicy::AllowBelowRbkg { .. }),
            format!("R fit minimum {} Å is below AUTOBK Rbkg {rbkg} Å; choose a higher rmin or explicitly use AllowBelowRbkg with a reason", t.rmin),
        )?;
    }
    Ok(())
}

impl RmcDataset {
    /// Capture a processed [`Spectrum`] as an owned complex-R dataset (since 0.2.10).
    /// Reads authoritative [`Spectrum::k`] and [`Spectrum::chi`] getters, copies
    /// selected unweighted samples and the complete processing state, and validates
    /// the transform, shifted k support and Rbkg policy before scattering starts.
    /// Uses the existing RMC/native fitting transform, not cached plotted χ(R).
    ///
    /// No normalization, AUTOBK or FFT processing stage runs on the input. Call
    /// `spectrum.calc_background()` explicitly first if needed. Legacy public
    /// field edits require `invalidate_derived()` and reprocessing, as for the
    /// ordinary Spectrum API; stale caches cannot be detected automatically.
    /// Captured state is copied on problem/checkpoint cloning, not per MC move.
    /// The caller's spectrum, including its settings and cached results, is unchanged.
    ///
    /// The default objective is the squared real-plus-imaginary residual divided
    /// by experimental power in the SAME R window. Missing/invalid processing,
    /// empty selections, invalid scales, zero normalization power and unsupported
    /// fitting settings return [`RmcError::Invalid`]. A fluorescence-corrected
    /// XANES-only spectrum is rejected even if legacy EXAFS buffers are present.
    /// Geometry-dependent absorber
    /// checks occur at session creation. No files are written here.
    ///
    /// ```no_run
    /// use rexafs::{Spectrum, fitting::FeffFitTransform, rmc::*, structure::Edge};
    /// fn dataset(spectrum: &Spectrum, absorbers: Vec<usize>, transform: FeffFitTransform)
    ///     -> Result<RmcDataset, RmcError>
    /// {
    ///     let mut options = RmcSpectrumOptions::new(absorbers, Edge::K, transform);
    ///     options.k_range = Some([2.5, 12.]); // Sample-specific support, including tapers.
    ///     options.s02 = 0.98; // Example independently calibrated amplitude.
    ///     options.delta_e0 = 8.7; // Fitting shift in eV, not the edge energy.
    ///     RmcDataset::from_spectrum(spectrum, options)
    /// }
    /// ```
    pub fn from_spectrum(
        spectrum: &Spectrum,
        options: RmcSpectrumOptions,
    ) -> Result<Self, RmcError> {
        let (k, chi, rbkg) = processed(spectrum)?;
        let weights = options.transform.effective_kweights();
        require(
            options.transform.fitspace == FitSpace::R
                && weights.len() == 1
                && weights[0].is_finite()
                && (0. ..=3.).contains(&weights[0])
                && weights[0].fract() == 0.,
            "spectrum RMC input requires fitspace=R and one integer k weight in 0..=3",
        )?;
        let mut distinct = std::collections::HashSet::new();
        require(
            !options.absorbers.is_empty() && options.absorbers.iter().all(|a| distinct.insert(a)),
            "spectrum RMC input needs nonempty, distinct absorber indices",
        )?;
        require(
            options.s02.is_finite() && options.s02 > 0.,
            "S02 must be finite and positive",
        )?;
        require(
            options.delta_e0.is_finite(),
            "fitting delta_e0 must be finite",
        )?;
        if let Some(sigma) = &options.sigma {
            require(
                sigma.len() == k.len() && sigma.iter().all(|s| s.is_finite() && *s > 0.),
                "sigma must contain one finite positive scale per full-spectrum k point",
            )?;
        }
        let [lo, hi] = options.k_range.unwrap_or([k[0], k[k.len() - 1]]);
        require(
            lo.is_finite() && hi.is_finite() && lo < hi && lo >= k[0] && hi <= k[k.len() - 1],
            "k_range must be increasing and lie inside the measured k support",
        )?;
        let indices: Vec<_> = k
            .iter()
            .enumerate()
            .filter_map(|(i, &x)| (x >= lo && x <= hi).then_some(i))
            .collect();
        require(
            indices.len() >= 2,
            "k_range must select at least two measured samples",
        )?;
        let exafs = ExafsDataset {
            name: spectrum
                .name
                .as_deref()
                .filter(|s| !s.trim().is_empty())
                .unwrap_or("spectrum")
                .to_owned(),
            absorbers: options.absorbers.clone(),
            edge: options.edge,
            k: indices.iter().map(|&i| k[i]).collect(),
            chi: indices.iter().map(|&i| chi[i]).collect(),
            sigma: indices
                .iter()
                .map(|&i| options.sigma.as_ref().map_or(1., |s| s[i]))
                .collect(),
            weight: 1.,
            kweight: weights[0] as u8,
            s02: options.s02,
            delta_e0: options.delta_e0,
        };
        let objective = Objective::R(options.transform.clone());
        check_rbkg(&objective, rbkg, &options.rbkg_policy)?;
        super::engine::shifted_grid(&exafs).map_err(|e| {
            RmcError::Invalid(format!(
                "{e}; select k_range above the fitting energy-shift threshold"
            ))
        })?;
        let mut dataset: Self = exafs.into();
        dataset.objective = objective;
        // Validate Fourier support even when experimental normalization is disabled.
        dataset
            .objective
            .score(&dataset.exafs, &dataset.exafs.chi)?;
        if options.normalize_by_experiment {
            dataset.normalize_experimental_power()?;
        }
        dataset.source = Some(Box::new(RmcSpectrumSource {
            spectrum: spectrum.clone(),
            selected_indices: indices,
            options,
            rexafs_version: env!("CARGO_PKG_VERSION").into(),
        }));
        Ok(dataset)
    }

    /// Set dataset weight so a zero model scores 1 in its current objective
    /// (since 0.2.10). Applies noise and k weights once to both data and residual.
    /// Call again after changing the transform, kweight or sigma if this scaling
    /// is wanted; later field edits never silently renormalize the objective.
    /// Replaces the prior dataset weight. No scattering or preprocessing runs.
    /// Invalid/zero power returns an error and leaves the weight unchanged.
    pub fn normalize_experimental_power(&mut self) -> Result<(), RmcError> {
        let mut data = self.exafs.clone();
        data.weight = 1.;
        let power = self.objective.score(&data, &vec![0.; data.k.len()])?;
        let weight = 1. / power;
        require(
            power > 0. && weight.is_finite() && weight > 0.,
            "experimental objective power must be finite and positive",
        )?;
        self.exafs.weight = weight;
        Ok(())
    }

    pub(super) fn validate_spectrum_source(&self) -> Result<(), RmcError> {
        let Some(source) = &self.source else {
            return Ok(());
        };
        let (k, chi, rbkg) = processed(&source.spectrum)?;
        let indices = &source.selected_indices;
        require(indices.len() == self.exafs.k.len() && indices.len() == self.exafs.chi.len()
            && indices.windows(2).all(|w| w[1] > w[0])
            && indices.iter().enumerate().all(|(j, &i)| i < k.len() && k[i] == self.exafs.k[j] && chi[i] == self.exafs.chi[j]),
            "RMC experimental arrays no longer match their captured spectrum; rebuild with from_spectrum")?;
        check_rbkg(&self.objective, rbkg, &source.options.rbkg_policy)
    }
}
