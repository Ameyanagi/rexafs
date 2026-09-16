//! Explicit, immutable training stages for shared scattering representatives.
use super::*;

/// Unreleased: measured-error training for a shared path basis. Training happens
/// during context preparation, never in response to accepted/rejected moves.
/// Serialize this manifest with acceleration settings for reproducible resume.
/// The result is an approximation away from the training geometries: a geometric
/// radius is not a spectral error bound. Check independent configurations with
/// `compare_reference`, then create a new stage and rescore the optimizer.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct AdaptiveBasisSettings {
    /// Explicit stage number, starting at one; contributes to calculator identity.
    pub epoch: u64,
    /// Training geometries per mixture component. The fixed electronic reference
    /// is always included first. An empty outer list adds no extra configurations.
    /// Use representative population members; all must satisfy the original envelope.
    pub training: Vec<Vec<Configuration>>,
    /// Increasing audit k grid in Å⁻¹. Required, at least two points, within kmax.
    /// Validation concerns this grid only; calls using another grid use exact paths.
    pub k: Vec<f64>,
    /// Exponent applied to χ for training checks, 0..=3; default 2.
    pub kweight: u8,
    /// Maximum coordinate-feature difference in Å for sharing; default 0.02.
    /// Unpolarized features are all pair distances; polarized features are full
    /// absorber-relative coordinates. Species and directed path order must match.
    pub geometry_radius: f64,
    /// Relative L2 error tolerance for each training path AND each summed spectrum;
    /// default 0.001 (0.1%). Cancellation in a total is checked separately.
    pub relative_error: f64,
    /// Absolute L2 allowance in k^w χ units, added to relative_error*reference_norm;
    /// default 1e-10. It also defines behavior for a zero reference spectrum.
    pub absolute_error: f64,
    /// Maximum representatives per electronic context; default 10,000. A family
    /// that cannot be split within this limit falls back entirely to exact paths.
    pub max_representatives: usize,
    /// Cap on retained training path χ samples per context; default two million.
    /// Exceeding this cap is an error before storing the extra spectrum.
    pub max_training_samples: usize,
}
impl Default for AdaptiveBasisSettings {
    fn default() -> Self {
        Self {
            epoch: 1,
            training: Vec::new(),
            k: Vec::new(),
            kweight: 2,
            geometry_radius: 0.02,
            relative_error: 0.001,
            absolute_error: 1e-10,
            max_representatives: 10_000,
            max_training_samples: 2_000_000,
        }
    }
}
impl AdaptiveBasisSettings {
    pub(super) fn validate(&self, references: &[Configuration]) -> Result<(), RmcError> {
        require(
            self.epoch > 0
                && self.kweight <= 3
                && self.k.len() >= 2
                && self.k.iter().all(|v| v.is_finite() && *v >= 0.)
                && self.k.windows(2).all(|v| v[1] > v[0])
                && self.geometry_radius.is_finite()
                && self.geometry_radius > 0.
                && self.relative_error.is_finite()
                && self.relative_error >= 0.
                && self.absolute_error.is_finite()
                && self.absolute_error >= 0.
                && self.max_representatives > 0
                && self.max_representatives <= 1_000_000
                && self.max_training_samples >= self.k.len()
                && (self.training.is_empty() || self.training.len() == references.len()),
            "invalid adaptive basis training settings",
        )?;
        for structures in &self.training {
            require(
                structures.len() <= 1024,
                "adaptive basis training exceeds 1024 geometries per component",
            )?;
        }
        Ok(())
    }
}
/// Measured training evidence for one absorber/edge/options context. This is not
/// an independent validation set or a guarantee for future Monte Carlo proposals.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AdaptiveBasisReport {
    /// Electronic-context identifier, including component, absorber, edge/options.
    pub context: String,
    /// Manifest stage number.
    pub epoch: u64,
    /// Number of geometries, including the fixed reference.
    pub configurations: usize,
    /// Active concrete training paths checked (including repeated geometries).
    pub training_paths: usize,
    /// Retained representative tables.
    pub representatives: usize,
    /// Error-driven additional representatives beyond initial geometric groups.
    pub splits: usize,
    /// Species/order families disabled because the budget or total-error check failed.
    pub exact_families: usize,
    /// Weighted total-spectrum relative L2 error for each training configuration;
    /// None denotes a zero reference norm; absolute acceptance is still checked.
    pub relative_errors: Vec<Option<f64>>,
}
struct Representative {
    features: Vec<f64>,
    table: Arc<PathTable>,
}
struct TrainingPath {
    family: Vec<i64>,
    features: Vec<f64>,
    table: Arc<PathTable>,
    chi: Vec<f64>,
    length: f64,
}
pub(super) struct AdaptiveContext {
    settings: AdaptiveBasisSettings,
    polarized: bool,
    groups: BTreeMap<Vec<i64>, Vec<Representative>>,
    disabled: std::collections::BTreeSet<Vec<i64>>,
    pub(super) report: AdaptiveBasisReport,
}
fn features(
    path: &ScatteringPath,
    configuration: &Configuration,
    polarized: bool,
) -> Result<(Vec<i64>, Vec<f64>), RmcError> {
    let mut signature = super::signature(path, configuration, polarized)?;
    let coordinates = signature.split_off(path.scatterers.len() + 1);
    Ok((
        signature,
        coordinates.into_iter().map(|x| x as f64 * 1e-8).collect(),
    ))
}
fn difference(a: &[f64], b: &[f64]) -> f64 {
    a.iter()
        .zip(b)
        .map(|(x, y)| (x - y).abs())
        .fold(0., f64::max)
}
impl AdaptiveContext {
    fn candidate(&self, family: &[i64], geometry: &[f64]) -> Option<&Representative> {
        if self.disabled.contains(family) {
            return None;
        }
        self.groups
            .get(family)?
            .iter()
            .filter_map(|r| {
                let distance = difference(&r.features, geometry);
                (distance <= self.settings.geometry_radius).then_some((distance, r))
            })
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(_, r)| r)
    }
    pub(super) fn table(
        &self,
        path: &ScatteringPath,
        configuration: &Configuration,
        k: &[f64],
    ) -> Result<Option<&Arc<PathTable>>, RmcError> {
        if self.settings.k != k {
            return Ok(None);
        }
        let (family, geometry) = features(path, configuration, self.polarized)?;
        Ok(self.candidate(&family, &geometry).map(|r| &r.table))
    }
    fn norm(&self, values: &[f64]) -> f64 {
        values
            .iter()
            .zip(&self.settings.k)
            .map(|(v, k)| (v * k.powi(i32::from(self.settings.kweight))).powi(2))
            .sum::<f64>()
            .sqrt()
    }
    fn passes(&self, model: &[f64], reference: &[f64]) -> bool {
        let error: Vec<_> = model.iter().zip(reference).map(|(a, b)| a - b).collect();
        self.norm(&error)
            <= self.settings.absolute_error + self.settings.relative_error * self.norm(reference)
    }
    fn add(&mut self, path: &TrainingPath, split: bool) {
        if self.report.representatives == self.settings.max_representatives {
            self.disabled.insert(path.family.clone());
            return;
        }
        self.groups
            .entry(path.family.clone())
            .or_default()
            .push(Representative {
                features: path.features.clone(),
                table: Arc::clone(&path.table),
            });
        self.report.representatives += 1;
        self.report.splits += usize::from(split);
    }
    pub(super) fn train(
        context: &PreparedRefeffContext,
        catalogue: &PathCatalogue,
        settings: &AdaptiveBasisSettings,
        extra: &[Configuration],
        polarized: bool,
        key: &str,
        control: Option<(&CancellationToken, Instant)>,
    ) -> Result<Self, RmcError> {
        let mut result = Self {
            settings: settings.clone(),
            polarized,
            groups: BTreeMap::new(),
            disabled: Default::default(),
            report: AdaptiveBasisReport {
                context: key.into(),
                epoch: settings.epoch,
                configurations: extra.len() + 1,
                training_paths: 0,
                representatives: 0,
                splits: 0,
                exact_families: 0,
                relative_errors: Vec::new(),
            },
        };
        // Retain exact training samples for revalidation after splitting. Training
        // reference path tables are transient; only chosen representatives survive.
        let mut training = Vec::new();
        for configuration in std::iter::once(catalogue.reference()).chain(extra) {
            catalogue.validate(configuration)?;
            let mut paths = Vec::new();
            for path in catalogue.paths() {
                check_control(control)?;
                let length = path.half_length(configuration)?;
                if length > catalogue.settings().radius {
                    continue;
                }
                require(
                    (result.report.training_paths + 1).saturating_mul(settings.k.len())
                        <= settings.max_training_samples,
                    "adaptive training exceeds sample budget",
                )?;
                let (family, geometry) = features(path, configuration, polarized)?;
                let table = Arc::new(context.scattering(path, configuration)?.table()?);
                let chi = table.sample(&settings.k, length)?;
                let sample = TrainingPath {
                    family,
                    features: geometry,
                    table,
                    chi,
                    length,
                };
                if result.candidate(&sample.family, &sample.features).is_none()
                    && !result.disabled.contains(&sample.family)
                {
                    result.add(&sample, false);
                }
                result.report.training_paths += 1;
                paths.push(sample);
            }
            training.push(paths);
        }
        // A new representative can change nearest membership for earlier samples.
        // Revisit the entire training set until stable; duplicate-feature failures
        // and exhausted budgets disable the family, providing deterministic fallback.
        loop {
            let mut changed = false;
            for path in training.iter().flatten() {
                check_control(control)?;
                let Some(rep) = result.candidate(&path.family, &path.features) else {
                    continue;
                };
                let model = rep.table.sample(&settings.k, path.length)?;
                if !result.passes(&model, &path.chi) {
                    if difference(&rep.features, &path.features) == 0. {
                        result.disabled.insert(path.family.clone());
                    } else {
                        result.add(path, true);
                    }
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        // Path-relative bounds alone do not control cancellation in their sum.
        // Failed total checks conservatively disable all contributing approximate
        // families for that geometry, then recheck every total with the final bank.
        loop {
            let mut changed = false;
            result.report.relative_errors.clear();
            for paths in &training {
                let mut model = vec![0.; settings.k.len()];
                let mut reference = model.clone();
                for path in paths {
                    check_control(control)?;
                    let approximate = result
                        .candidate(&path.family, &path.features)
                        .map(|r| r.table.sample(&settings.k, path.length))
                        .transpose()?;
                    for ((m, r), (a, b)) in model.iter_mut().zip(&mut reference).zip(
                        approximate
                            .as_ref()
                            .unwrap_or(&path.chi)
                            .iter()
                            .zip(&path.chi),
                    ) {
                        *m += a;
                        *r += b;
                    }
                }
                require(
                    result.norm(&model).is_finite() && result.norm(&reference).is_finite(),
                    "adaptive training spectrum overflow",
                )?;
                if !result.passes(&model, &reference) {
                    for path in paths {
                        changed |= result.disabled.insert(path.family.clone());
                    }
                }
                let error: Vec<_> = model.iter().zip(&reference).map(|(a, b)| a - b).collect();
                let norm = result.norm(&reference);
                result
                    .report
                    .relative_errors
                    .push((norm > 0.).then(|| result.norm(&error) / norm));
            }
            if !changed {
                break;
            }
        }
        result.report.exact_families = result.disabled.len();
        Ok(result)
    }
}
