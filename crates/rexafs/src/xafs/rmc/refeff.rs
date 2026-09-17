use super::geometry::{distance, image_bounds};
use super::*;
use crate::structure::{
    build_cluster, AbsorberSelection, Cluster, ClusterOptions, Element, OccupancyPolicy, Site,
    Structure, Xyz, XyzAbsorber, XyzAtom,
};
use ::refeff::{ArtifactSelection, CancellationToken, MemoryRunRequest, Runner};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// ReFEFF work counters. Counts include failed pipeline attempts; byte counts
/// measure retained cache keys and numerical/artifact payloads, not total process RAM.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct RefeffCacheStats {
    /// Full potential/phase/path pipeline attempts, including pinned-reference setup.
    pub full_calculations: u64,
    /// Geometry/path-only pipeline attempts using pinned potentials and phases.
    pub path_calculations: u64,
    /// Requests served entirely from the exact local-input spectrum cache.
    pub spectrum_hits: u64,
    /// Requests finding an already prepared pinned reference.
    pub reference_hits: u64,
    /// LRU entries removed to respect byte limits.
    pub evictions: u64,
    /// Retained native-spectrum and key payload bytes.
    pub spectrum_bytes: usize,
    /// Retained pinned-reference artifact and key payload bytes.
    pub reference_bytes: usize,
    /// Time generating local clusters and FEFF input for requested geometries,
    /// in seconds. Includes cache-hit requests, excludes lazy reference setup.
    pub input_seconds: f64,
    /// Total wall time inside the pipeline wrapper, in seconds, including
    /// artifact transport, output decoding and failed attempts.
    pub pipeline_seconds: f64,
    /// Completed ReFEFF stage timings, aggregated over full and path pipelines.
    /// Stage sums can differ from pipeline wall time; transport/setup is separate.
    pub stages: BTreeMap<String, RefeffStageTiming>,
}

/// Aggregated timing from ReFEFF's completed stage reports. This is runtime
/// instrumentation, not part of the scientific calculator/checkpoint identity.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct RefeffStageTiming {
    /// Number of completed stage reports, including reused stages.
    pub calls: u64,
    /// Sum of ReFEFF-reported wall durations, in milliseconds.
    pub milliseconds: u64,
    /// Sum of the stage's handled rows/artifacts; interpret using `unit`.
    pub count: u64,
    /// Backend unit for `count`; a changed unit is reported as `mixed`.
    pub unit: String,
}
struct Cache<T> {
    entries: HashMap<String, (Arc<T>, usize, u64)>,
    bytes: usize,
    limit: usize,
    tick: u64,
}
impl<T> Cache<T> {
    fn new(limit: usize) -> Self {
        Self {
            entries: HashMap::new(),
            bytes: 0,
            limit,
            tick: 0,
        }
    }
    fn get(&mut self, key: &str) -> Option<Arc<T>> {
        self.tick += 1;
        self.entries.get_mut(key).map(|(value, _, tick)| {
            *tick = self.tick;
            Arc::clone(value)
        })
    }
    fn insert(&mut self, key: String, value: Arc<T>, payload: usize) -> u64 {
        let size = payload.saturating_add(key.len());
        if size > self.limit {
            return 0;
        }
        let mut evictions = 0;
        if let Some((_, bytes, _)) = self.entries.remove(&key) {
            self.bytes -= bytes;
        }
        while self.bytes + size > self.limit {
            let key = self
                .entries
                .iter()
                .min_by_key(|(_, (_, _, tick))| tick)
                .unwrap()
                .0
                .clone();
            let (_, bytes, _) = self.entries.remove(&key).unwrap();
            self.bytes -= bytes;
            evictions += 1;
        }
        self.tick += 1;
        self.entries.insert(key, (value, size, self.tick));
        self.bytes += size;
        evictions
    }
}
struct NativeSpectrum {
    grid: Vec<f64>,
    chi: Vec<f64>,
    paths: Vec<PathContribution>,
}
impl NativeSpectrum {
    fn bytes(&self) -> usize {
        8 * (self.grid.len()
            + self.chi.len()
            + self.paths.iter().map(|p| p.chi.len()).sum::<usize>())
            + self.paths.len() * std::mem::size_of::<PathContribution>()
    }
    fn sample(&self, k: &[f64], paths: bool) -> Result<CalculatedSpectrum, RmcError> {
        let chi = interpolate(&self.grid, &self.chi, k)?;
        let mut contributions = Vec::new();
        if paths {
            for p in &self.paths {
                let mut p = p.clone();
                p.chi = interpolate(&self.grid, &p.chi, k)?;
                contributions.push(p);
            }
        }
        Ok(CalculatedSpectrum {
            chi,
            paths: contributions,
        })
    }
}
struct ReferenceArtifacts {
    artifacts: BTreeMap<String, Vec<u8>>,
    potentials: String,
    input: String,
    spectrum: NativeSpectrum,
}
impl ReferenceArtifacts {
    fn bytes(&self) -> usize {
        self.artifacts
            .iter()
            .map(|(k, v)| k.len() + v.len())
            .sum::<usize>()
            + self.potentials.len()
            + self.input.len()
            + self.spectrum.bytes()
    }
}

/// Primary ReFEFF calculator with bounded, exact local-input caching. The default
/// regenerates potentials whenever local geometry changes. [`Self::with_frozen_potentials`]
/// opts into faster geometry/path updates with potentials and phases pinned to
/// explicit reference structures. That mode is an approximation requiring validation.
/// Neither mode reuses trial-dependent potentials. See `doc/rmc.md`.
pub struct RefeffCalculator {
    options: RefeffOptions,
    cancellation: CancellationToken,
    diagnostics: BTreeSet<String>,
    references: Option<Vec<Configuration>>,
    spectra: Cache<NativeSpectrum>,
    reference_cache: Cache<ReferenceArtifacts>,
    stats: RefeffCacheStats,
    identity: String,
}
impl RefeffCalculator {
    /// Create a full-physics calculator with a 64 MiB native-spectrum cache. No
    /// scattering runs here. Disable caching with `set_cache_capacity(0)` for a baseline.
    pub fn new(options: RefeffOptions) -> Result<Self, RmcError> {
        options.validate()?;
        let mut calculator = Self {
            options,
            cancellation: CancellationToken::default(),
            diagnostics: BTreeSet::new(),
            references: None,
            spectra: Cache::new(64 * 1024 * 1024),
            reference_cache: Cache::new(64 * 1024 * 1024),
            stats: RefeffCacheStats::default(),
            identity: String::new(),
        };
        calculator.update_identity();
        Ok(calculator)
    }
    /// Pin one reference configuration per mixture component. ReFEFF computes
    /// potential/phase artifacts lazily for each absorber/edge/options combination;
    /// subsequent moves rerun PATH, GENFMT and FF2X with new coordinates. Species,
    /// atom ordering and cells must remain fixed; a changed local potential species
    /// set fails explicitly. Reference coordinates never update during a run.
    /// Requires a new calculator to change references; clears existing caches.
    pub fn with_frozen_potentials(
        mut self,
        references: Vec<Configuration>,
    ) -> Result<Self, RmcError> {
        require(
            !references.is_empty(),
            "frozen potentials need explicit reference structures",
        )?;
        for c in &references {
            c.validate()?;
        }
        self.references = Some(references);
        self.clear_cache();
        self.update_identity();
        Ok(self)
    }
    fn update_identity(&mut self) {
        let bytes = serde_json::to_vec(&(&self.options, &self.references))
            .expect("validated finite options");
        let digest: String = Sha256::digest(bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        self.identity = format!("rexafs-rmc-v2/refeff-0.4.0/{digest}");
    }
    /// Set the byte limit for each of the spectrum and reference caches, clearing
    /// both. Default 64 MiB each. Zero disables reuse; oversized entries are not kept.
    /// This changes resource use only, so checkpoint scientific identity is unchanged.
    pub fn set_cache_capacity(&mut self, bytes: usize) {
        self.spectra = Cache::new(bytes);
        self.reference_cache = Cache::new(bytes);
    }
    /// Discard cached spectra/artifacts without changing references or work counters.
    pub fn clear_cache(&mut self) {
        self.spectra = Cache::new(self.spectra.limit);
        self.reference_cache = Cache::new(self.reference_cache.limit);
    }
    /// Snapshot work counts and currently retained payload bytes.
    pub fn stats(&self) -> RefeffCacheStats {
        let mut stats = self.stats.clone();
        stats.spectrum_bytes = self.spectra.bytes;
        stats.reference_bytes = self.reference_cache.bytes;
        stats
    }
    /// Borrow default calculator settings; per-dataset overrides are independent.
    pub fn options(&self) -> &RefeffOptions {
        &self.options
    }
    pub(super) fn set_cancellation(&mut self, token: CancellationToken) {
        self.cancellation = token;
    }
    /// Shared cooperative cancellation token. Cancellation is checked even on
    /// cache hits. Create a new calculator after cancellation to resume a session.
    pub fn cancellation_token(&self) -> CancellationToken {
        self.cancellation.clone()
    }
    /// Nonfatal diagnostics accumulated from completed ReFEFF pipelines.
    pub fn diagnostics(&self) -> &BTreeSet<String> {
        &self.diagnostics
    }
    /// Full-calculation FEFF input for inspection, with default options and atom
    /// coordinates printed to 12 decimal places in Å. S₀² is fixed at one.
    pub fn input_for(
        &self,
        configuration: &Configuration,
        absorber: usize,
        edge: Edge,
    ) -> Result<String, RmcError> {
        self.input_for_with_options(configuration, absorber, edge, &self.options)
    }
    /// Generate full-calculation input using explicit dataset options. Scattering
    /// potentials are numbered by ascending atomic number to stay stable when
    /// neighbor distances reorder. The absorbing potential is always zero.
    pub fn input_for_with_options(
        &self,
        configuration: &Configuration,
        absorber: usize,
        edge: Edge,
        options: &RefeffOptions,
    ) -> Result<String, RmcError> {
        options.validate()?;
        configuration.validate()?;
        require(
            absorber < configuration.atoms.len(),
            "absorber index out of range",
        )?;
        let cluster = Self::cluster(configuration, absorber, options.cluster_radius)?;
        require(
            cluster.atoms.len() > 1 && cluster.atoms.len() <= 1000,
            "absorber cluster needs 2..=1000 atoms",
        )?;
        require(
            cluster.atoms.iter().skip(1).all(|a| a.distance > 1e-6),
            "absorber coincides with a scattering atom",
        )?;
        let mut input=format!("TITLE rexafs RMC geometry calculation\nHOLE {} 1.0\nCONTROL 1 1 1 1 1 1\nPRINT 0 0 0 0 0 3\nRPATH {:.12}\nNLEG {}\nEXAFS {:.12}\nCRITERIA {:.12} {:.12}\n",edge.hole_index(),options.path_radius,options.max_legs,options.kmax,options.path_criteria[0],options.path_criteria[1]);
        if let Some(r) = options.scf_radius {
            input.push_str(&format!("SCF {r:.12}\n"));
        }
        if let Some(vector) = options.polarization {
            let norm = distance(vector, [0.; 3]);
            let [x, y, z] = vector.map(|v| v / norm);
            input.push_str(&format!("POLARIZATION {x:.12} {y:.12} {z:.12}\n"));
        }
        let species: Vec<_> = cluster
            .atoms
            .iter()
            .skip(1)
            .map(|a| a.z)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        input.push_str(&format!(
            "POTENTIALS\n0 {} {}\n",
            cluster.atoms[0].z, cluster.atoms[0].symbol
        ));
        for (i, &z) in species.iter().enumerate() {
            input.push_str(&format!(
                "{} {} {}\n",
                i + 1,
                z,
                Element::from_z(z).unwrap().symbol
            ));
        }
        input.push_str("ATOMS\n");
        for (i, atom) in cluster.atoms.iter().enumerate() {
            let [x, y, z] = atom.cart.map(|v| if v.abs() < 0.5e-12 { 0. } else { v });
            let ipot = if i == 0 {
                0
            } else {
                species.binary_search(&atom.z).unwrap() + 1
            };
            input.push_str(&format!("{x:.12} {y:.12} {z:.12} {ipot} {}\n", atom.symbol));
        }
        input.push_str("END\n");
        Ok(input)
    }
    fn cluster(
        configuration: &Configuration,
        absorber: usize,
        radius: f64,
    ) -> Result<Cluster, RmcError> {
        let result = if let Some(lattice) = configuration.lattice()? {
            let bounds = image_bounds(&lattice, radius)?;
            let candidates = bounds
                .iter()
                .map(|n| (2 * n + 1) as usize)
                .product::<usize>();
            require(
                candidates.saturating_mul(configuration.atoms.len()) <= 2_000_000,
                "periodic cluster enumeration exceeds the reference backend's resource limit",
            )?;
            let sites = configuration
                .atoms
                .iter()
                .enumerate()
                .map(|(i, a)| {
                    Site::new(
                        &format!("atom{i}"),
                        Element::from_z(a.atomic_number).unwrap().symbol,
                        lattice.to_frac(a.position).map(|v| v.rem_euclid(1.0)),
                    )
                })
                .collect();
            let structure = Structure::new("RMC explicit periodic cell", lattice, sites);
            build_cluster(
                &structure,
                &AbsorberSelection::SiteIndex(absorber),
                &ClusterOptions {
                    radius,
                    include_hydrogen: true,
                    occupancy: OccupancyPolicy::Majority,
                },
            )
        } else {
            let xyz = Xyz {
                comment: "RMC explicit finite cluster".into(),
                atoms: configuration
                    .atoms
                    .iter()
                    .map(|a| XyzAtom {
                        symbol: Element::from_z(a.atomic_number).unwrap().symbol.into(),
                        z: a.atomic_number,
                        cart: a.position,
                    })
                    .collect(),
            };
            xyz.to_cluster(&XyzAbsorber::Index(absorber), Some(radius))
        };
        result.map_err(|e| RmcError::Invalid(e.to_string()))
    }
    pub(super) fn run_pipeline(
        &mut self,
        input: String,
        options: &RefeffOptions,
        selection: ArtifactSelection,
        artifacts: Option<&BTreeMap<String, Vec<u8>>>,
    ) -> Result<::refeff::MemoryRunResult, RmcError> {
        let started = Instant::now();
        let mut request = MemoryRunRequest::new(input.into_bytes());
        if let Some(artifacts) = artifacts {
            for (path, bytes) in artifacts {
                request
                    .insert_artifact(path, bytes.clone())
                    .map_err(|e| RmcError::Calculator(e.to_string()))?;
            }
        }
        let result = Runner::new()
            .with_threads(NonZeroUsize::new(options.threads).unwrap())
            .with_cancellation(self.cancellation.clone())
            .with_deadline(Instant::now() + Duration::from_secs_f64(options.timeout_seconds))
            .with_artifacts(selection)
            .run_in_memory(request);
        self.stats.pipeline_seconds += started.elapsed().as_secs_f64();
        let output = result.map_err(|e| RmcError::Calculator(e.to_string()))?;
        for stage in &output.report.stages {
            let timing = self.stats.stages.entry(stage.name.clone()).or_default();
            if timing.calls == 0 {
                timing.unit = stage.unit.clone();
            } else if timing.unit != stage.unit {
                timing.unit = "mixed".into();
            }
            timing.calls += 1;
            timing.milliseconds += stage.duration_ms;
            timing.count += stage.count as u64;
        }
        for d in &output.report.diagnostics {
            self.diagnostics
                .insert(format!("{}: {}", d.code, d.message));
        }
        Ok(output)
    }
    fn native(output: &::refeff::MemoryRunResult, paths: bool) -> Result<NativeSpectrum, RmcError> {
        let chi = output
            .spectra
            .chi
            .as_ref()
            .ok_or_else(|| RmcError::Calculator("ReFEFF returned no chi".into()))?;
        let grid = chi.wave_number.to_vec();
        let values = chi.chi.to_vec();
        interpolate(&grid, &values, &[])?;
        let mut contributions = Vec::new();
        if paths {
            let metadata = output
                .paths
                .as_ref()
                .ok_or_else(|| RmcError::Calculator("missing ReFEFF path metadata".into()))?;
            for artifact in output.artifacts.iter() {
                let name = artifact.path.to_string_lossy();
                let Some(index) = name
                    .strip_prefix("chip")
                    .and_then(|s| s.strip_suffix(".dat"))
                    .and_then(|s| s.parse::<usize>().ok())
                else {
                    continue;
                };
                let p = metadata
                    .paths
                    .iter()
                    .find(|p| p.index == index)
                    .ok_or_else(|| {
                        RmcError::Calculator(format!("missing metadata for path {index}"))
                    })?;
                let text = std::str::from_utf8(artifact.bytes)
                    .map_err(|e| RmcError::Calculator(e.to_string()))?;
                let path = ::refeff::io::chi_dat::parse_chi_dat(text)
                    .map_err(|e| RmcError::Calculator(e.to_string()))?;
                contributions.push(PathContribution {
                    index,
                    legs: p.leg_count(),
                    degeneracy: p.degeneracy,
                    half_length: p.effective_half_path_length_angstrom,
                    chi: interpolate(&path.wave_number.to_vec(), &path.chi.to_vec(), &grid)?,
                });
            }
            contributions.sort_by_key(|p| p.index);
            require(
                !contributions.is_empty() || values.iter().all(|v| *v == 0.),
                "requested path output is missing",
            )?;
        }
        Ok(NativeSpectrum {
            grid,
            chi: values,
            paths: contributions,
        })
    }
}
fn potential_block(input: &str) -> &str {
    input
        .split_once("POTENTIALS\n")
        .unwrap()
        .1
        .split_once("ATOMS\n")
        .unwrap()
        .0
}
impl ExafsCalculator for RefeffCalculator {
    fn name(&self) -> &str {
        if self.references.is_some() {
            "ReFEFF (pinned potentials, geometry/path updates)"
        } else {
            "ReFEFF (full geometry, exact local-input cache)"
        }
    }
    fn identity(&self) -> String {
        self.identity.clone()
    }
    fn calculate(
        &mut self,
        configuration: &Configuration,
        absorber: usize,
        edge: Edge,
        k: &[f64],
    ) -> Result<Vec<f64>, RmcError> {
        Ok(self
            .calculate_request(CalculationRequest {
                structure: 0,
                configuration,
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
        if self.cancellation.is_cancelled() {
            return Err(RmcError::Calculator("ReFEFF cancelled".into()));
        }
        let options = request.options.unwrap_or(&self.options).clone();
        options.validate()?;
        require(
            request.k.len() >= 2
                && request
                    .k
                    .iter()
                    .all(|v| v.is_finite() && *v >= 0. && *v <= options.kmax)
                && request.k.windows(2).all(|v| v[1] > v[0]),
            "requested ReFEFF k grid is invalid or exceeds kmax",
        )?;
        let input_started = Instant::now();
        let input = self.input_for_with_options(
            request.configuration,
            request.absorber,
            request.edge,
            &options,
        );
        self.stats.input_seconds += input_started.elapsed().as_secs_f64();
        let mut input = input?;
        if !request.paths {
            input = input.replace("PRINT 0 0 0 0 0 3", "PRINT 0 0 0 0 0 0");
        }
        let option_key = serde_json::to_string(&options).unwrap();
        let reference_key = if self.references.is_some() {
            format!(
                "{}/{}/{}/{}",
                request.structure,
                request.absorber,
                request.edge.hole_index(),
                option_key
            )
        } else {
            String::new()
        };
        if let Some(references) = &self.references {
            require(
                request.structure < references.len(),
                "missing pinned reference for mixture component",
            )?;
            let reference = &references[request.structure];
            require(
                reference.cell == request.configuration.cell
                    && reference.atoms.len() == request.configuration.atoms.len()
                    && reference
                        .atoms
                        .iter()
                        .zip(&request.configuration.atoms)
                        .all(|(a, b)| a.atomic_number == b.atomic_number),
                "frozen-potential calculation changed reference topology",
            )?;
        }
        let key = format!("{reference_key}/{option_key}/{input}");
        if let Some(spectrum) = self.spectra.get(&key) {
            self.stats.spectrum_hits += 1;
            return spectrum.sample(request.k, request.paths);
        }
        let spectrum = if let Some(references) = &self.references {
            require(
                request.structure < references.len(),
                "missing pinned reference for mixture component",
            )?;
            let reference = &references[request.structure];
            require(
                reference.cell == request.configuration.cell
                    && reference.atoms.len() == request.configuration.atoms.len()
                    && reference
                        .atoms
                        .iter()
                        .zip(&request.configuration.atoms)
                        .all(|(a, b)| a.atomic_number == b.atomic_number),
                "frozen-potential calculation changed reference topology",
            )?;
            let reference = if let Some(reference) = self.reference_cache.get(&reference_key) {
                self.stats.reference_hits += 1;
                reference
            } else {
                let ref_input = self
                    .input_for_with_options(reference, request.absorber, request.edge, &options)?
                    .replace("PRINT 0 0 0 0 0 3", "PRINT 0 0 0 0 0 0");
                self.stats.full_calculations += 1;
                let selected = ["pot.bin", "phase.bin", "xsect.dat"];
                let output = self.run_pipeline(
                    ref_input.clone(),
                    &options,
                    ArtifactSelection::Paths(
                        selected.iter().map(std::path::PathBuf::from).collect(),
                    ),
                    None,
                )?;
                let mut artifacts = BTreeMap::new();
                for path in selected {
                    let bytes = output.artifacts.get(path).ok_or_else(|| {
                        RmcError::Calculator(format!("ReFEFF reference missing {path}"))
                    })?;
                    artifacts.insert(path.into(), bytes.to_vec());
                }
                let reference = Arc::new(ReferenceArtifacts {
                    artifacts,
                    potentials: potential_block(&ref_input).to_owned(),
                    input: ref_input,
                    spectrum: Self::native(&output, false)?,
                });
                self.stats.evictions += self.reference_cache.insert(
                    reference_key,
                    Arc::clone(&reference),
                    reference.bytes(),
                );
                reference
            };
            require(
                reference.potentials == potential_block(&input),
                "local scattering species changed; pinned potentials cannot represent this cluster",
            )?;
            if !request.paths && input == reference.input {
                Arc::new(NativeSpectrum {
                    grid: reference.spectrum.grid.clone(),
                    chi: reference.spectrum.chi.clone(),
                    paths: Vec::new(),
                })
            } else {
                let input = input.replace("CONTROL 1 1 1 1 1 1", "CONTROL 0 0 0 1 1 1");
                self.stats.path_calculations += 1;
                let output = self.run_pipeline(
                    input,
                    &options,
                    if request.paths {
                        ArtifactSelection::All
                    } else {
                        ArtifactSelection::None
                    },
                    Some(&reference.artifacts),
                )?;
                Arc::new(Self::native(&output, request.paths)?)
            }
        } else {
            self.stats.full_calculations += 1;
            let output = self.run_pipeline(
                input,
                &options,
                if request.paths {
                    ArtifactSelection::All
                } else {
                    ArtifactSelection::None
                },
                None,
            )?;
            Arc::new(Self::native(&output, request.paths)?)
        };
        let result = spectrum.sample(request.k, request.paths)?;
        self.stats.evictions += self
            .spectra
            .insert(key, Arc::clone(&spectrum), spectrum.bytes());
        Ok(result)
    }
}
pub(super) fn interpolate(
    grid: &[f64],
    values: &[f64],
    query: &[f64],
) -> Result<Vec<f64>, RmcError> {
    require(
        grid.len() >= 2
            && grid.len() == values.len()
            && grid.iter().chain(values).all(|v| v.is_finite())
            && grid.windows(2).all(|w| w[1] > w[0]),
        "ReFEFF returned invalid chi arrays",
    )?;
    let mut result = Vec::with_capacity(query.len());
    for &q in query {
        require(
            q >= grid[0] && q <= grid[grid.len() - 1],
            format!(
                "requested k={q} outside calculated support [{}, {}] Å⁻¹",
                grid[0],
                grid[grid.len() - 1]
            ),
        )?;
        let high = grid.partition_point(|&k| k < q).clamp(1, grid.len() - 1);
        let low = high - 1;
        let t = (q - grid[low]) / (grid[high] - grid[low]);
        result.push((1.0 - t) * values[low] + t * values[high]);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_eviction_preserves_recent_entries_and_byte_limit() {
        let mut cache = Cache::new(4);
        assert_eq!(cache.insert("a".into(), Arc::new(1), 1), 0);
        assert_eq!(cache.insert("b".into(), Arc::new(2), 1), 0);
        assert_eq!(*cache.get("a").unwrap(), 1);
        assert_eq!(cache.insert("c".into(), Arc::new(3), 1), 1);
        assert!(cache.get("b").is_none());
        assert_eq!(*cache.get("a").unwrap(), 1);
        assert_eq!(cache.bytes, 4);
        cache.insert("oversized".into(), Arc::new(4), 100);
        assert_eq!(cache.bytes, 4);
        assert!(cache.get("oversized").is_none());
    }

    #[test]
    fn interpolation_preserves_endpoints_and_forbids_extrapolation() {
        assert_eq!(
            interpolate(&[1.0, 2.0, 3.0], &[4.0, 8.0, 4.0], &[1.0, 1.5, 3.0]).unwrap(),
            vec![4.0, 6.0, 4.0]
        );
        assert!(interpolate(&[1.0, 2.0], &[0.0, 1.0], &[2.01]).is_err());
        assert!(interpolate(&[1.0, 1.0], &[0.0, 1.0], &[1.0]).is_err());
    }
}
