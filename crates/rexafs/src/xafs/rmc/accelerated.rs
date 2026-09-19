#[path = "adaptive_basis.rs"]
mod adaptive_basis;
#[path = "adaptive_control.rs"]
mod adaptive_control;
use super::geometry::distance;
use super::prepared::PathTable;
use super::*;
use ::refeff::CancellationToken;
use adaptive_basis::AdaptiveContext;
pub use adaptive_basis::{AdaptiveBasisReport, AdaptiveBasisSettings};
pub use adaptive_control::*;
use rayon::prelude::*;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Path treatment for the prepared calculator (since 0.2.10). Both modes use
/// fixed reference electronic potentials and an explicit geometric catalogue.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub enum ScatteringBasis {
    /// Recommended default: reuse unchanged cached paths and reevaluate changed
    /// paths with typed ReFEFF GENFMT. Electronic potentials remain fixed.
    #[default]
    Exact,
    /// Experimental, explicit opt-in: freeze representative amplitude/phase tables
    /// from the reference geometry, changing the `2*k*R` propagation phase for each
    /// actual path. This adds an
    /// approximation beyond pinned potentials: angular scattering and amplitude
    /// changes are neglected inside the declared limits. Beyond either limit,
    /// use exact typed paths. Limits are geometric guards, not error guarantees;
    /// validate using `PreparedRefeffCalculator::compare_reference`.
    Frozen {
        /// Maximum absolute change of any leg length, in Å.
        max_leg_change: f64,
        /// Maximum absolute change of any internal angle, in radians.
        max_angle_change: f64,
    },
}

/// Fixed scientific and resource limits for prepared calculations (since 0.2.10).
/// [`Self::default`] and omitted JSON settings select exact affected-path caching:
/// [`ScatteringBasis::Exact`], a 256 MiB cache, no moments and no adaptive basis.
/// This is the recommended mode. It still uses fixed reference potentials.
/// Experimental training manifests can define immutable adaptive stages. No basis
/// adapts to accepted/rejected trials; cold and warm calls evaluate the
/// same function. This preserves exact session resume for the chosen model.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct AccelerationSettings {
    /// Fixed catalogue; radius/order must match every requested ReFEFF option.
    pub catalogue: PathCatalogueSettings,
    /// Exact affected-path caching by default. Frozen representatives are an
    /// experimental approximation and require explicit opt-in.
    pub basis: ScatteringBasis,
    /// Total CPU thread budget for absorber calculations; default 1, range 1..=64.
    /// Independent absorber requests run first. With [`Self::parallel_paths`],
    /// spare threads also evaluate paths within a smaller absorber batch. Both
    /// levels share this one pool; their thread counts do not multiply.
    pub workers: usize,
    /// Since 0.2.11: use spare workers for paths when a batch has fewer absorbers
    /// than workers. Default false retains historical scheduling. Recommended
    /// with multiple workers, especially for a single absorbing site. Path sums
    /// retain catalogue order, including cached/rejected trials. Electronic
    /// preparation remains serial; this does not create backend thread pools.
    pub parallel_paths: bool,
    /// Maximum prepared structure/absorber/edge/settings contexts; default 128.
    /// A 256-site single-element cell averaged over every site needs at least
    /// 256. Shared datasets with the same combinations reuse contexts. Set this
    /// explicitly for larger calculations; sites are never silently sampled.
    pub max_contexts: usize,
    /// Since 0.2.11: share immutable electronic setup when complete FEFF inputs
    /// match after sorting scatterer rows at the existing 12-decimal precision.
    /// Recommended for new periodic jobs; no sites, paths or species are merged.
    /// Defaults to false to preserve historical input ordering and checkpoint
    /// identity. Sorting can change numerical summation and FEFF's representative
    /// atom when several atoms of one potential tie for nearest distance. It is
    /// not guaranteed to reproduce legacy results bit for bit; compare both modes
    /// for order-sensitive environments. The mode cannot change on saved-run resume.
    pub reuse_electronic_inputs: bool,
    /// Maximum catalogue paths across all contexts; default one million.
    pub max_total_paths: usize,
    /// Approximate numerical payload budget for last-geometry path spectra;
    /// default 256 MiB. Oversized results are evaluated but not cached. This
    /// excludes immutable phase tensors, catalogues, and transient batch results.
    pub cache_bytes: usize,
    /// Since 0.2.10: retained geometries per electronic context, 1..=1024 (default 1).
    /// Population searches can use population size plus one. The nearest retained
    /// geometry by number of changed atoms is reused; the global byte limit still
    /// applies. This resource setting is excluded from scientific identity.
    pub snapshots_per_context: usize,
    /// Optional controlled moment summation for frozen-basis groups. Defaults to
    /// None (direct sums). Individual path reporting always uses direct sums.
    pub moments: Option<MomentSettings>,
    /// Experimental, opt-in error-driven training for shared frozen tables.
    /// Default None disables adaptive training; it is never enabled automatically.
    /// Requires Frozen basis and no moment approximation. Its feature radius
    /// replaces the legacy leg/angle guards. Independent accuracy and overall
    /// speed are not guaranteed; use [`AdaptiveBasisController`] for periodic
    /// exact audits and check final spectra against exact paths. Prefer the
    /// default exact caching for routine refinement.
    pub adaptive: Option<AdaptiveBasisSettings>,
}
impl Default for AccelerationSettings {
    fn default() -> Self {
        Self {
            catalogue: Default::default(),
            basis: ScatteringBasis::Exact,
            workers: 1,
            parallel_paths: false,
            max_contexts: 128,
            reuse_electronic_inputs: false,
            max_total_paths: 1_000_000,
            cache_bytes: 256 * 1024 * 1024,
            snapshots_per_context: 1,
            moments: None,
            adaptive: None,
        }
    }
}

/// Measured work for a prepared calculator. Cache state/timings are excluded
/// from scientific checkpoint identity and may change after resume.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct PreparedRefeffStats {
    /// Electronic contexts prepared, each containing fixed POT/XSPH results.
    pub contexts: usize,
    /// Fresh electronic preparations in this calculator (since 0.2.10).
    #[serde(default)]
    pub electronic_preparations: usize,
    /// Contexts reused from an immutable stage, exact audit, or matching canonical
    /// electronic input. All absorber-specific catalogues remain separate.
    #[serde(default)]
    pub shared_electronic_contexts: usize,
    /// Directed concrete paths, including paths reserved by the displacement envelope.
    pub catalogue_paths: usize,
    /// Distinct frozen representative tables computed (within each context).
    pub representatives: usize,
    /// Exact typed GENFMT calls during candidate evaluation, including fallbacks.
    pub exact_paths: u64,
    /// Candidate paths evaluated using frozen amplitude/phase tables.
    pub basis_paths: u64,
    /// Paths reused because none of their atoms moved and the k grid matched.
    pub reused_paths: u64,
    /// Total catalogue entries visited, including inactive envelope paths.
    #[serde(default)]
    pub visited_paths: u64,
    /// Reused active entries, a subset of `reused_paths`; moment entries count
    /// even when their individual sampled spectrum is represented by a group.
    #[serde(default)]
    pub reused_active_paths: u64,
    /// Changed entries excluded by the current path-radius test.
    #[serde(default)]
    pub outside_radius_paths: u64,
    /// Number of retained geometry snapshots across all contexts.
    #[serde(default)]
    pub cached_snapshots: usize,
    /// Requests served from an identical retained configuration and k grid.
    #[serde(default)]
    pub identical_geometry_hits: u64,
    /// Absorber requests successfully evaluated.
    pub requests: u64,
    /// Wall time preparing contexts, catalogues and reference representatives.
    pub setup_seconds: f64,
    /// Wall time evaluating batches, excluding preparation.
    pub evaluation_seconds: f64,
    /// Retained last-geometry numerical payload estimate, in bytes.
    pub cached_bytes: usize,
}

/// Spectrum comparison against direct typed paths with the same fixed potentials
/// and catalogue. This tests the representative basis, not electronic-state or
/// path-order convergence; use the full `RefeffCalculator` for those checks.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScatteringAccuracy {
    /// Square-root sum of squared errors divided by reference L2 norm; None
    /// when the reference norm vanishes. Uses unweighted χ on the supplied grid.
    pub relative_l2: Option<f64>,
    /// Largest absolute difference in dimensionless χ.
    pub max_absolute: f64,
    /// Approximate result including deterministic geometric fallbacks.
    pub model: Vec<f64>,
    /// Direct typed-path result on the same grid.
    pub reference: Vec<f64>,
}

/// One active concrete path with geometry, basis membership and optional accuracy
/// diagnostics (since 0.2.10). Entries retain catalogue order; reverse paths remain
/// separate. Transform `contribution.chi` with the same Fourier/local-spectrum
/// settings as the total to obtain additive complex path maps.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PreparedPathReport {
    /// Stable atom/image membership, lengths in Å and internal angles in degrees.
    pub geometry: PathGeometryReport,
    /// One-based representative catalogue index, or None in exact mode. The label
    /// is stable only within this electronic context and calculator identity.
    /// Adaptive stages use shared feature groups rather than catalogue-index labels
    /// and return None here; consult `adaptive_reports` for training evidence.
    pub reference_family: Option<usize>,
    /// True when the current geometry passes the frozen-basis guards.
    pub used_basis: bool,
    /// Individual spectrum, before S₀² or mixture/absorber averaging.
    pub contribution: PathContribution,
    /// L2 norm of k^w χ_path divided by that of k^w χ_total on the supplied grid.
    /// None for a zero total. Cancellation allows values above one; these are
    /// importance diagnostics, not fractions that sum to one or screening bounds.
    pub relative_importance: Option<f64>,
    /// Optional direct typed-path comparison at the same electronic reference.
    pub accuracy: Option<ScatteringAccuracy>,
}

struct PreparedAbsorber {
    catalogue: Arc<PathCatalogue>,
    context: Arc<PreparedRefeffContext>,
    bases: Vec<Option<Arc<PathTable>>>,
    basis_groups: Vec<usize>,
    reference_shape: Vec<Vec<f64>>,
    adaptive: Option<AdaptiveContext>,
}
#[derive(Clone)]
struct CachedPath {
    length: f64,
    chi: Option<Arc<Vec<f64>>>,
    basis: Option<usize>,
}
struct Snapshot {
    configuration: Configuration,
    k: Vec<f64>,
    paths: Vec<Option<CachedPath>>,
    tick: u64,
}
impl Snapshot {
    fn bytes(&self) -> usize {
        self.paths
            .iter()
            .filter_map(Option::as_ref)
            .map(|p| p.chi.as_ref().map_or(0, |chi| chi.len() * 8))
            .sum::<usize>()
            + self.paths.len() * std::mem::size_of::<Option<CachedPath>>()
            + self.configuration.atoms.len() * std::mem::size_of::<Atom>()
            + self.k.len() * 8
    }
}
struct Work {
    snapshot: Snapshot,
    spectrum: CalculatedSpectrum,
    exact: u64,
    basis: u64,
    reused: u64,
    reused_active: u64,
    outside_radius: u64,
    identical_geometry: bool,
}

/// ReFEFF calculator with immutable prepared contexts, stable path identities,
/// local updates and bounded parallel batches. Unlike FEFF's screened path search,
/// this explicitly sums all directed walks within its radius/order, including
/// reverse paths separately. Use `[0,0]` path criteria for comparisons; nonzero
/// criteria are rejected to avoid implying equivalent path selection.
/// The calculator retains immutable references and never learns from MC history.
/// Cache eviction/rejected trials therefore cannot alter scientific results.
/// Pass [`AccelerationSettings::default`] for the recommended exact caching.
/// Frozen/adaptive representatives are experimental approximations, disabled by
/// default, and need independent accuracy and end-to-end timing checks.
pub struct PreparedRefeffCalculator {
    options: RefeffOptions,
    references: Vec<Configuration>,
    settings: AccelerationSettings,
    identity: String,
    pool: rayon::ThreadPool,
    contexts: HashMap<String, PreparedAbsorber>,
    shared_contexts: HashMap<String, (Arc<PathCatalogue>, Arc<PreparedRefeffContext>)>,
    electronic_inputs: HashMap<String, Arc<PreparedRefeffContext>>,
    snapshots: HashMap<String, Vec<Snapshot>>,
    stats: PreparedRefeffStats,
    tick: u64,
    cancellation: CancellationToken,
}
impl PreparedRefeffCalculator {
    /// Validate/copy settings and fixed mixture references. Electronic setup is
    /// lazy, once per absorber/edge/settings combination. No scattering runs here.
    pub fn new(
        options: RefeffOptions,
        references: Vec<Configuration>,
        settings: AccelerationSettings,
    ) -> Result<Self, RmcError> {
        options.validate()?;
        require(
            !references.is_empty(),
            "prepared calculator needs reference structures",
        )?;
        for reference in &references {
            reference.validate()?;
        }
        if let Some(moments) = &settings.moments {
            moments.validate()?;
        }
        require(
            (1..=64).contains(&settings.workers)
                && settings.max_contexts > 0
                && settings.max_total_paths > 0
                && (1..=1024).contains(&settings.snapshots_per_context),
            "invalid prepared calculator resource limits",
        )?;
        if let ScatteringBasis::Frozen {
            max_leg_change,
            max_angle_change,
        } = settings.basis
        {
            require(
                max_leg_change.is_finite()
                    && max_leg_change >= 0.
                    && max_angle_change.is_finite()
                    && (0. ..=std::f64::consts::PI).contains(&max_angle_change),
                "invalid frozen basis limits",
            )?;
        }
        if let Some(adaptive) = &settings.adaptive {
            adaptive.validate(&references)?;
            require(
                matches!(settings.basis, ScatteringBasis::Frozen { .. })
                    && settings.moments.is_none(),
                "adaptive training requires frozen basis without moments",
            )?;
        }
        // Validate catalogue settings before paying for any electronic setup.
        PathCatalogue::new(
            Configuration {
                atoms: vec![references[0].atoms[0].clone()],
                cell: None,
            },
            0,
            settings.catalogue.clone(),
        )?;
        // Serialize through the legacy field order for backward identity stability.
        let mut settings_bytes =
            serde_json::to_string(&settings).map_err(|e| RmcError::Invalid(e.to_string()))?;
        settings_bytes = settings_bytes.replace(
            &format!(
                ",\"snapshots_per_context\":{}",
                settings.snapshots_per_context
            ),
            "",
        );
        if settings.adaptive.is_none() {
            settings_bytes = settings_bytes.replace(",\"adaptive\":null", "");
        }
        if !settings.reuse_electronic_inputs {
            settings_bytes = settings_bytes.replace(",\"reuse_electronic_inputs\":false", "");
        }
        if !settings.parallel_paths {
            settings_bytes = settings_bytes.replace(",\"parallel_paths\":false", "");
        }
        let bytes = format!(
            "[{},{},{}]",
            serde_json::to_string(&options).unwrap(),
            serde_json::to_string(&references).unwrap(),
            settings_bytes
        )
        .into_bytes();
        let hash: String = Sha256::digest(bytes)
            .iter()
            .map(|v| format!("{v:02x}"))
            .collect();
        let identity = format!("rexafs-prepared-refeff-0.4.0-v1/{hash}");
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(settings.workers)
            .build()
            .map_err(|e| RmcError::Calculator(e.to_string()))?;
        let calculator = Self {
            options,
            references,
            settings,
            identity,
            pool,
            contexts: HashMap::new(),
            shared_contexts: HashMap::new(),
            electronic_inputs: HashMap::new(),
            snapshots: HashMap::new(),
            stats: Default::default(),
            tick: 0,
            cancellation: CancellationToken::default(),
        };
        calculator.validate_options(&calculator.options)?;
        Ok(calculator)
    }
    fn validate_options(&self, options: &RefeffOptions) -> Result<(), RmcError> {
        options.validate()?;
        require(
            options.path_criteria == [0., 0.]
                && options.path_radius == self.settings.catalogue.radius
                && options.max_legs as usize == self.settings.catalogue.max_legs,
            "prepared calculator needs zero path criteria and matching catalogue radius/order",
        )
    }
    fn key(&self, request: CalculationRequest<'_>) -> Result<String, RmcError> {
        let options = request.options.unwrap_or(&self.options);
        self.validate_options(options)?;
        require(
            request.structure < self.references.len(),
            "missing prepared reference component",
        )?;
        require(
            request.absorber < self.references[request.structure].atoms.len(),
            "prepared absorber out of range",
        )?;
        require(
            request.k.len() >= 2
                && request.k.windows(2).all(|w| w[1] > w[0])
                && request
                    .k
                    .iter()
                    .all(|v| v.is_finite() && *v >= 0. && *v <= options.kmax),
            "invalid prepared calculator k grid",
        )?;
        Ok(format!(
            "{}/{}/{}/{}",
            request.structure,
            request.absorber,
            request.edge.hole_index(),
            serde_json::to_string(options).unwrap()
        ))
    }
    fn ensure(&mut self, key: &str, request: CalculationRequest<'_>) -> Result<(), RmcError> {
        if self.contexts.contains_key(key) {
            return Ok(());
        }
        require(
            self.contexts.len() < self.settings.max_contexts,
            format!("Prepared RMC needs more than {} absorber contexts (structure, atom, edge and settings combinations). Increase AccelerationSettings.max_contexts or explicitly select fewer absorbing sites.", self.settings.max_contexts),
        )?;
        let start = Instant::now();
        let deadline = start
            + Duration::from_secs_f64(request.options.unwrap_or(&self.options).timeout_seconds);
        check_control(Some((&self.cancellation, deadline)))?;
        let options = request.options.unwrap_or(&self.options).clone();
        let shared = self.shared_contexts.get(key).cloned();
        let mut reused_electronic = shared.is_some();
        let (catalogue, context) = if let Some(shared) = shared {
            shared
        } else {
            let reference = self.references[request.structure].clone();
            let catalogue = Arc::new(PathCatalogue::new(
                reference.clone(),
                request.absorber,
                self.settings.catalogue.clone(),
            )?);
            catalogue.validate(request.configuration)?;
            require(
                self.stats
                    .catalogue_paths
                    .saturating_add(catalogue.paths().len())
                    <= self.settings.max_total_paths,
                format!("Prepared RMC needs {} catalogue paths; the limit is {}. Reduce path radius, scattering order or the selected absorbing sites, or explicitly increase AccelerationSettings.max_total_paths.", self.stats.catalogue_paths.saturating_add(catalogue.paths().len()), self.settings.max_total_paths),
            )?;
            let context = if self.settings.reuse_electronic_inputs {
                let input = PreparedRefeffContext::canonical_input(
                    &reference,
                    request.absorber,
                    request.edge,
                    &options,
                )?;
                // Retain exact strings rather than a lossy geometric signature.
                let input_key = format!("{}\n{input}", serde_json::to_string(&options).unwrap());
                if let Some(shared) = self.electronic_inputs.get(&input_key) {
                    reused_electronic = true;
                    Arc::new(shared.with_equivalent_input(
                        reference,
                        request.absorber,
                        options.clone(),
                    ))
                } else {
                    let context = Arc::new(PreparedRefeffContext::prepare_input(
                        reference,
                        request.absorber,
                        options.clone(),
                        input,
                        self.cancellation.clone(),
                    )?);
                    self.electronic_inputs
                        .insert(input_key, Arc::clone(&context));
                    context
                }
            } else {
                Arc::new(PreparedRefeffContext::prepare_controlled(
                    reference,
                    request.absorber,
                    request.edge,
                    options.clone(),
                    self.cancellation.clone(),
                )?)
            };
            (catalogue, context)
        };
        catalogue.validate(request.configuration)?;
        require(
            self.stats
                .catalogue_paths
                .saturating_add(catalogue.paths().len())
                <= self.settings.max_total_paths,
            format!("Prepared RMC needs {} catalogue paths; the limit is {}. Reduce path radius, scattering order or the selected absorbing sites, or explicitly increase AccelerationSettings.max_total_paths.", self.stats.catalogue_paths.saturating_add(catalogue.paths().len()), self.settings.max_total_paths),
        )?;
        let mut bases = Vec::new();
        let mut basis_groups = Vec::new();
        let mut reference_shape = Vec::new();
        let mut representatives: BTreeMap<Vec<i64>, (usize, Arc<PathTable>)> = BTreeMap::new();
        if self.settings.basis != ScatteringBasis::Exact && self.settings.adaptive.is_none() {
            for path in catalogue.paths() {
                check_control(Some((&self.cancellation, deadline)))?;
                let signature =
                    signature(path, catalogue.reference(), options.polarization.is_some())?;
                let (group, table) = if let Some((index, table)) = representatives.get(&signature) {
                    (*index, Arc::clone(table))
                } else {
                    let table = Arc::new(context.scattering(path, catalogue.reference())?.table()?);
                    let index = bases.len();
                    representatives.insert(signature, (index, Arc::clone(&table)));
                    (index, table)
                };
                bases.push(Some(table));
                basis_groups.push(group);
                reference_shape.push(shape(path, catalogue.reference())?);
            }
        }
        check_control(Some((&self.cancellation, deadline)))?;
        let adaptive = if let Some(settings) = &self.settings.adaptive {
            require(
                settings.k.last().is_some_and(|k| *k <= options.kmax),
                "adaptive audit grid exceeds kmax",
            )?;
            let extra = settings
                .training
                .get(request.structure)
                .map_or(&[][..], Vec::as_slice);
            Some(AdaptiveContext::train(
                &context,
                &catalogue,
                settings,
                extra,
                options.polarization.is_some(),
                key,
                Some((&self.cancellation, deadline)),
            )?)
        } else {
            None
        };
        self.stats.contexts += 1;
        if reused_electronic {
            self.stats.shared_electronic_contexts += 1;
        } else {
            self.stats.electronic_preparations += 1;
        }
        self.stats.catalogue_paths += catalogue.paths().len();
        self.stats.representatives += adaptive
            .as_ref()
            .map_or(representatives.len(), |a| a.report.representatives);
        self.stats.setup_seconds += start.elapsed().as_secs_f64();
        self.contexts.insert(
            key.to_owned(),
            PreparedAbsorber {
                catalogue,
                context,
                bases,
                basis_groups,
                reference_shape,
                adaptive,
            },
        );
        Ok(())
    }
    fn closest_snapshot(&self, key: &str, request: CalculationRequest<'_>) -> Option<&Snapshot> {
        self.snapshots
            .get(key)?
            .iter()
            .filter(|p| p.k == request.k)
            .min_by_key(|p| {
                (
                    p.configuration
                        .atoms
                        .iter()
                        .zip(&request.configuration.atoms)
                        .filter(|(a, b)| a.position != b.position)
                        .count(),
                    std::cmp::Reverse(p.tick),
                )
            })
    }
    /// Owned training evidence, sorted by electronic-context key. Only contexts
    /// actually prepared so far appear. This does not run extra calculations.
    pub fn adaptive_reports(&self) -> Vec<AdaptiveBasisReport> {
        let mut reports: Vec<_> = self
            .contexts
            .values()
            .filter_map(|p| p.adaptive.as_ref().map(|a| a.report.clone()))
            .collect();
        reports.sort_by(|a, b| a.context.cmp(&b.context));
        reports
    }
    /// Experimental: construct a new immutable adaptive basis stage with the same
    /// electronic references and options. Prepared electronic contexts/catalogues
    /// are shared immutably;
    /// basis training is lazy and spectra start with empty caches. Both stages share
    /// cancellation. Use an increased epoch and retained
    /// training geometries; then explicitly rebase an optimizer before stepping.
    /// Existing calculator/caches remain available if preparation or rebasing fails.
    pub fn refreshed_basis(&self, adaptive: AdaptiveBasisSettings) -> Result<Self, RmcError> {
        require(
            self.settings
                .adaptive
                .as_ref()
                .is_none_or(|old| adaptive.epoch > old.epoch),
            "a refreshed adaptive basis needs a larger epoch",
        )?;
        let mut settings = self.settings.clone();
        settings.adaptive = Some(adaptive);
        self.with_shared_contexts(settings)
    }
    fn with_shared_contexts(&self, settings: AccelerationSettings) -> Result<Self, RmcError> {
        let mut next = Self::new(self.options.clone(), self.references.clone(), settings)?;
        next.shared_contexts = self.shared_contexts.clone();
        next.electronic_inputs = self.electronic_inputs.clone();
        for (key, prepared) in &self.contexts {
            next.shared_contexts.insert(
                key.clone(),
                (
                    Arc::clone(&prepared.catalogue),
                    Arc::clone(&prepared.context),
                ),
            );
        }
        next.cancellation = self.cancellation.clone();
        Ok(next)
    }
    /// Since 0.2.10: construct an exact typed-path calculator with the same pinned
    /// electronic references. Already prepared electronic contexts/catalogues are
    /// shared immutably; approximate spectra and tables are never reused. Fresh
    /// contexts remain lazy. This does not recompute self-consistent potentials
    /// and is not a test of electronic or path-order convergence. Both calculators
    /// share cancellation. Its identity matches a cold exact calculator.
    pub fn exact_reference(&self) -> Result<Self, RmcError> {
        let mut settings = self.settings.clone();
        settings.basis = ScatteringBasis::Exact;
        settings.adaptive = None;
        settings.moments = None;
        self.with_shared_contexts(settings)
    }
    /// Shared cancellation handle, checked between explicit paths, including cache
    /// hits, and during electronic setup. A single typed path kernel is not
    /// interruptible. After cancellation, resume with a new identical calculator.
    /// Per-absorber timeouts from RefeffOptions also cover preparation and updates.
    pub fn cancellation_token(&self) -> CancellationToken {
        self.cancellation.clone()
    }
    /// Snapshot work counters and the current numerical cache payload.
    pub fn stats(&self) -> PreparedRefeffStats {
        self.stats.clone()
    }
    /// Prepare this absorber if needed and borrow its stable path identities.
    /// Reported path number `i+1` corresponds to catalogue entry `i`. Preparation
    /// can run electronic setup; the current geometry must satisfy the envelope.
    pub fn catalogue(
        &mut self,
        request: CalculationRequest<'_>,
    ) -> Result<&PathCatalogue, RmcError> {
        let key = self.key(request)?;
        self.ensure(&key, request)?;
        let catalogue = &self.contexts[&key].catalogue;
        catalogue.validate(request.configuration)?;
        Ok(catalogue)
    }
    /// Drop last-geometry caches, preserving fixed contexts and representatives.
    pub fn clear_cache(&mut self) {
        self.snapshots.clear();
        self.stats.cached_bytes = 0;
        self.stats.cached_snapshots = 0;
    }
    /// Inspect active path membership, frozen-family labels and k-weighted
    /// importance without changing the model. `kweight` is 0..=3. If `check_exact`
    /// is true, every active path is also recalculated directly and its unweighted
    /// χ error is returned; this can be expensive. This report never drops weak
    /// paths, since disorder can strengthen them. Returned arrays are owned.
    pub fn path_reports(
        &mut self,
        request: CalculationRequest<'_>,
        kweight: u8,
        check_exact: bool,
    ) -> Result<Vec<PreparedPathReport>, RmcError> {
        require(kweight <= 3, "path report kweight must be 0..=3")?;
        let spectrum = self.calculate_request(CalculationRequest {
            paths: true,
            ..request
        })?;
        let key = self.key(request)?;
        let prepared = &self.contexts[&key];
        let norm = |chi: &[f64]| {
            chi.iter()
                .zip(request.k)
                .map(|(v, k)| (v * k.powi(i32::from(kweight))).powi(2))
                .sum::<f64>()
                .sqrt()
        };
        let total = norm(&spectrum.chi);
        let deadline = Instant::now()
            + Duration::from_secs_f64(request.options.unwrap_or(&self.options).timeout_seconds);
        let mut reports = Vec::with_capacity(spectrum.paths.len());
        for contribution in spectrum.paths {
            check_control(Some((&self.cancellation, deadline)))?;
            let i = contribution.index - 1;
            let path = &prepared.catalogue.paths()[i];
            let accuracy = if check_exact {
                let reference = prepared
                    .context
                    .scattering(path, request.configuration)?
                    .sample(request.k)?;
                Some(accuracy(contribution.chi.clone(), reference))
            } else {
                None
            };
            reports.push(PreparedPathReport {
                geometry: path_geometry_report(request.configuration, path)?,
                reference_family: prepared.basis_groups.get(i).map(|i| i + 1),
                used_basis: prepared
                    .table(i, request, &self.settings.basis)?
                    .0
                    .is_some(),
                relative_importance: (total > 0.).then(|| norm(&contribution.chi) / total),
                contribution,
                accuracy,
            });
        }
        Ok(reports)
    }
    /// Compare frozen-basis output to exact typed paths for this request. Never
    /// changes the basis or MC state. A large error calls for tighter basis guards
    /// or `ScatteringBasis::Exact`, followed by a new run with that identity.
    pub fn compare_reference(
        &mut self,
        request: CalculationRequest<'_>,
    ) -> Result<ScatteringAccuracy, RmcError> {
        let model = self.calculate_request(request)?.chi;
        let key = self.key(request)?;
        let deadline = Instant::now()
            + Duration::from_secs_f64(request.options.unwrap_or(&self.options).timeout_seconds);
        let work = self.contexts[&key].evaluate(
            request,
            None,
            &ScatteringBasis::Exact,
            None,
            Some((&self.cancellation, deadline)),
            false,
        )?;
        let reference = work.spectrum.chi;
        Ok(accuracy(model, reference))
    }
}
fn accuracy(model: Vec<f64>, reference: Vec<f64>) -> ScatteringAccuracy {
    let norm = reference.iter().map(|v| v * v).sum::<f64>();
    let errors: Vec<_> = model.iter().zip(&reference).map(|(a, b)| a - b).collect();
    ScatteringAccuracy {
        relative_l2: (norm > 0.).then(|| (errors.iter().map(|v| v * v).sum::<f64>() / norm).sqrt()),
        max_absolute: errors.iter().map(|v| v.abs()).fold(0., f64::max),
        model,
        reference,
    }
}
fn check_control(control: Option<(&CancellationToken, Instant)>) -> Result<(), RmcError> {
    if let Some((token, deadline)) = control {
        if token.is_cancelled() {
            return Err(RmcError::Calculator(
                "prepared calculation cancelled".into(),
            ));
        }
        if Instant::now() >= deadline {
            return Err(RmcError::Calculator(
                "prepared calculation timed out".into(),
            ));
        }
    }
    Ok(())
}
impl PreparedAbsorber {
    fn uses_basis(
        &self,
        i: usize,
        c: &Configuration,
        basis: &ScatteringBasis,
    ) -> Result<bool, RmcError> {
        match basis {
            ScatteringBasis::Exact => Ok(false),
            ScatteringBasis::Frozen {
                max_leg_change,
                max_angle_change,
            } => {
                let current = shape(&self.catalogue.paths()[i], c)?;
                let legs = self.catalogue.paths()[i].scatterers.len() + 1;
                Ok(current
                    .iter()
                    .zip(&self.reference_shape[i])
                    .enumerate()
                    .all(|(j, (a, b))| {
                        (a - b).abs()
                            <= if j < legs {
                                *max_leg_change
                            } else {
                                *max_angle_change
                            }
                    }))
            }
        }
    }
    fn table(
        &self,
        i: usize,
        request: CalculationRequest<'_>,
        basis: &ScatteringBasis,
    ) -> Result<(Option<&Arc<PathTable>>, Option<usize>), RmcError> {
        if matches!(basis, ScatteringBasis::Exact) {
            return Ok((None, None));
        }
        if let Some(adaptive) = &self.adaptive {
            return Ok((
                adaptive.table(&self.catalogue.paths()[i], request.configuration, request.k)?,
                None,
            ));
        }
        if self.uses_basis(i, request.configuration, basis)? {
            Ok((self.bases[i].as_ref(), Some(self.basis_groups[i])))
        } else {
            Ok((None, None))
        }
    }
    fn evaluate(
        &self,
        request: CalculationRequest<'_>,
        previous: Option<&Snapshot>,
        basis: &ScatteringBasis,
        moments: Option<&MomentSettings>,
        control: Option<(&CancellationToken, Instant)>,
        parallel_paths: bool,
    ) -> Result<Work, RmcError> {
        check_control(control)?;
        let moments = moments.filter(|_| !request.paths);
        self.catalogue.validate(request.configuration)?;
        let count = self.catalogue.paths().len();
        let previous = previous.filter(|p| p.k == request.k);
        let mut changed = vec![true; count];
        if let Some(p) = previous {
            let atoms: Vec<_> = request
                .configuration
                .atoms
                .iter()
                .zip(&p.configuration.atoms)
                .enumerate()
                .filter_map(|(i, (a, b))| (a.position != b.position).then_some(i))
                .collect();
            changed.fill(false);
            for i in self.catalogue.affected_paths(&atoms)? {
                changed[i] = true;
            }
            if moments.is_none() {
                for (i, path) in p.paths.iter().enumerate() {
                    if path.as_ref().is_some_and(|p| p.chi.is_none()) {
                        changed[i] = true;
                    }
                }
            }
        }
        let mut paths = previous.map_or_else(|| vec![None; count], |p| p.paths.clone());
        let update_path = |(i, cached): (usize, &mut Option<CachedPath>)| {
            check_control(control)?;
            if !changed[i] {
                return Ok([0, 0, 1, u64::from(cached.is_some()), 0]);
            }
            let path = &self.catalogue.paths()[i];
            let length = path.half_length(request.configuration)?;
            if length > self.catalogue.settings().radius {
                *cached = None;
                return Ok([0, 0, 0, 0, 1]);
            }
            let (table, selected_group) = self.table(i, request, basis)?;
            let fast = u64::from(table.is_some());
            let (chi, group) = if let Some(table) = table {
                (
                    if moments.is_none() {
                        Some(Arc::new(table.sample(request.k, length)?))
                    } else {
                        None
                    },
                    selected_group,
                )
            } else {
                (
                    Some(Arc::new(
                        self.context
                            .scattering(path, request.configuration)?
                            .sample(request.k)?,
                    )),
                    None,
                )
            };
            *cached = Some(CachedPath {
                length,
                chi,
                basis: group,
            });
            Ok::<_, RmcError>([1 - fast, fast, 0, 0, 0])
        };
        let add_counts =
            |a: [u64; 5], b: [u64; 5]| Ok::<_, RmcError>(std::array::from_fn(|i| a[i] + b[i]));
        // Only called from the calculator's bounded pool. Parallelize independent
        // path evaluations, then sum floating-point spectra below in index order.
        let [exact, fast, reused, reused_active, outside_radius] = if parallel_paths {
            paths
                .par_iter_mut()
                .enumerate()
                .map(update_path)
                .try_reduce(|| [0; 5], add_counts)?
        } else {
            paths
                .iter_mut()
                .enumerate()
                .map(update_path)
                .try_fold([0; 5], |counts, result| add_counts(counts, result?))?
        };
        // Fixed index order ensures cache history/parallel scheduling cannot change reductions.
        let mut chi = vec![0.; request.k.len()];
        let mut contributions = Vec::new();
        let mut groups: BTreeMap<usize, Vec<f64>> = BTreeMap::new();
        for (i, p) in paths.iter().enumerate() {
            if let Some(p) = p {
                if let (Some(_), Some(group)) = (moments, p.basis) {
                    groups.entry(group).or_default().push(p.length);
                    continue;
                }
                let path_chi = p.chi.as_ref().expect("direct path contributions prepared");
                for (v, &x) in chi.iter_mut().zip(path_chi.iter()) {
                    *v += x;
                }
                if request.paths {
                    contributions.push(PathContribution {
                        index: i + 1,
                        legs: self.catalogue.paths()[i].scatterers.len() + 1,
                        degeneracy: 1.,
                        half_length: p.length,
                        chi: path_chi.as_ref().clone(),
                    });
                }
            }
        }
        for (group, lengths) in groups {
            let table = self.bases[group].as_ref().unwrap();
            let contribution = table.sample_group(request.k, &lengths, moments.unwrap())?;
            for (value, x) in chi.iter_mut().zip(contribution) {
                *value += x;
            }
        }
        require(
            chi.iter().all(|v| v.is_finite()),
            "prepared path sum is nonfinite",
        )?;
        check_control(control)?;
        Ok(Work {
            snapshot: Snapshot {
                configuration: request.configuration.clone(),
                k: request.k.to_vec(),
                paths,
                tick: 0,
            },
            spectrum: CalculatedSpectrum {
                chi,
                paths: contributions,
            },
            exact,
            basis: fast,
            reused,
            reused_active,
            outside_radius,
            identical_geometry: previous.is_some_and(|p| p.configuration == *request.configuration),
        })
    }
}
impl ExafsCalculator for PreparedRefeffCalculator {
    fn name(&self) -> &str {
        "ReFEFF (prepared explicit paths)"
    }
    fn identity(&self) -> String {
        self.identity.clone()
    }
    fn calculate(
        &mut self,
        c: &Configuration,
        absorber: usize,
        edge: Edge,
        k: &[f64],
    ) -> Result<Vec<f64>, RmcError> {
        Ok(self
            .calculate_request(CalculationRequest {
                structure: 0,
                configuration: c,
                absorber,
                edge,
                k,
                options: None,
                paths: false,
            })?
            .chi)
    }
    fn calculate_request(
        &mut self,
        request: CalculationRequest<'_>,
    ) -> Result<CalculatedSpectrum, RmcError> {
        Ok(self.calculate_batch(&[request])?.remove(0))
    }
    fn calculate_batch(
        &mut self,
        requests: &[CalculationRequest<'_>],
    ) -> Result<Vec<CalculatedSpectrum>, RmcError> {
        // Bound simultaneous transient results by processing at most one request per worker.
        let mut spectra = Vec::with_capacity(requests.len());
        for chunk in requests.chunks(self.settings.workers) {
            let mut keys = Vec::new();
            for &request in chunk {
                let key = self.key(request)?;
                self.ensure(&key, request)?;
                keys.push(key);
            }
            let start = Instant::now();
            let work: Vec<Work> = self.pool.install(|| {
                chunk
                    .par_iter()
                    .zip(&keys)
                    .map(|(&r, key)| {
                        self.contexts[key].evaluate(
                            r,
                            self.closest_snapshot(key, r),
                            &self.settings.basis,
                            self.settings.moments.as_ref(),
                            Some((
                                &self.cancellation,
                                Instant::now()
                                    + Duration::from_secs_f64(
                                        r.options.unwrap_or(&self.options).timeout_seconds,
                                    ),
                            )),
                            self.settings.parallel_paths && chunk.len() < self.settings.workers,
                        )
                    })
                    .collect::<Result<Vec<_>, _>>()
            })?;
            self.stats.evaluation_seconds += start.elapsed().as_secs_f64();
            for (key, mut result) in keys.into_iter().zip(work) {
                self.stats.requests += 1;
                self.stats.exact_paths += result.exact;
                self.stats.basis_paths += result.basis;
                self.stats.reused_paths += result.reused;
                self.stats.visited_paths += result.snapshot.paths.len() as u64;
                self.stats.reused_active_paths += result.reused_active;
                self.stats.outside_radius_paths += result.outside_radius;
                self.stats.identical_geometry_hits += u64::from(result.identical_geometry);
                spectra.push(result.spectrum);
                self.tick += 1;
                result.snapshot.tick = self.tick;
                let bytes = result.snapshot.bytes();
                // Replace duplicates rather than letting repeated parent evaluation
                // consume population slots. Eviction affects work only, never χ.
                if let Some(entries) = self.snapshots.get_mut(&key) {
                    if let Some(i) = entries.iter().position(|p| {
                        p.k == result.snapshot.k && p.configuration == result.snapshot.configuration
                    }) {
                        self.stats.cached_bytes -= entries.remove(i).bytes();
                        self.stats.cached_snapshots -= 1;
                    }
                    if entries.len() >= self.settings.snapshots_per_context {
                        let i = entries
                            .iter()
                            .enumerate()
                            .min_by_key(|(_, p)| p.tick)
                            .unwrap()
                            .0;
                        self.stats.cached_bytes -= entries.remove(i).bytes();
                        self.stats.cached_snapshots -= 1;
                    }
                }
                if bytes <= self.settings.cache_bytes {
                    while self.stats.cached_bytes.saturating_add(bytes) > self.settings.cache_bytes
                    {
                        let (oldest, i) = self
                            .snapshots
                            .iter()
                            .flat_map(|(key, entries)| {
                                entries
                                    .iter()
                                    .enumerate()
                                    .map(move |(i, p)| (key, i, p.tick))
                            })
                            .min_by_key(|(_, _, tick)| *tick)
                            .map(|(key, i, _)| (key.clone(), i))
                            .unwrap();
                        self.stats.cached_bytes -=
                            self.snapshots.get_mut(&oldest).unwrap().remove(i).bytes();
                        self.stats.cached_snapshots -= 1;
                    }
                    self.stats.cached_bytes += bytes;
                    self.stats.cached_snapshots += 1;
                    self.snapshots.entry(key).or_default().push(result.snapshot);
                }
            }
        }
        Ok(spectra)
    }
}

fn shape(path: &ScatteringPath, c: &Configuration) -> Result<Vec<f64>, RmcError> {
    let positions = path.positions(c)?;
    let legs = positions.len() - 1;
    let mut out: Vec<_> = positions.windows(2).map(|p| distance(p[0], p[1])).collect();
    require(
        out.iter().all(|v| v.is_finite() && *v > 1e-8),
        "degenerate scattering path",
    )?;
    for i in 0..legs {
        let before = positions[(i + legs - 1) % legs];
        let after = positions[(i + 1) % legs];
        let center = positions[i];
        let a: [f64; 3] = std::array::from_fn(|j| before[j] - center[j]);
        let b: [f64; 3] = std::array::from_fn(|j| after[j] - center[j]);
        let dot = (0..3).map(|j| a[j] * b[j]).sum::<f64>()
            / (distance(before, center) * distance(after, center));
        out.push(dot.clamp(-1., 1.).acos());
    }
    Ok(out)
}
fn signature(
    path: &ScatteringPath,
    c: &Configuration,
    polarized: bool,
) -> Result<Vec<i64>, RmcError> {
    let positions = path.positions(c)?;
    let mut key: Vec<_> = path
        .scatterers
        .iter()
        .map(|v| {
            if v.atom == path.absorber && v.image == [0; 3] {
                0
            } else {
                c.atoms[v.atom].atomic_number as i64
            }
        })
        .collect();
    key.insert(0, path.scatterers.len() as i64);
    // Fixed 1e-8 Å tolerance only coalesces round-off-equivalent reference paths;
    // includes species and all pair distances (or oriented coordinates for polarization).
    if polarized {
        for p in positions.iter().skip(1) {
            for axis in 0..3 {
                key.push(((p[axis] - positions[0][axis]) * 1e8).round() as i64);
            }
        }
    } else {
        for i in 0..positions.len() - 1 {
            for j in i + 1..positions.len() - 1 {
                key.push((distance(positions[i], positions[j]) * 1e8).round() as i64);
            }
        }
    }
    Ok(key)
}
