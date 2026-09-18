//! Reproducible initialization, resource estimates and streaming analysis (since 0.2.10).
use super::*;
use rand::{seq::SliceRandom, Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::io::{BufRead, Read};
use std::ops::ControlFlow;

/// Return a copy with seeded independent Cartesian displacements uniformly drawn
/// from `[-half_width,half_width]` Å for each selected atom/axis. Atom order and
/// cell remain fixed; the maximum vector displacement is sqrt(3)*half_width.
/// Selection must be nonempty and distinct. This initializes disorder; it does
/// not enforce pair-distance constraints or represent a thermal distribution.
pub fn seeded_disorder(
    c: &Configuration,
    atoms: &[usize],
    half_width: f64,
    seed: u64,
) -> Result<Configuration, RmcError> {
    c.validate()?;
    super::structural::indices(atoms, c.atoms.len())?;
    require(
        half_width.is_finite() && (0. ..=1e6).contains(&half_width),
        "invalid disorder half-width",
    )?;
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut result = c.clone();
    for &atom in atoms {
        for x in &mut result.atoms[atom].position {
            *x += half_width * (2. * rng.random::<f64>() - 1.);
        }
    }
    result.validate()?;
    Ok(result)
}

/// Explicit substitution realization for preparing a new fixed-composition run.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SubstitutionResult {
    /// Owned substituted configuration; coordinates/cell/order are preserved.
    pub configuration: Configuration,
    /// Sorted selected atom indices, sampled without replacement from host sites.
    pub atoms: Vec<usize>,
    /// Original atomic number.
    pub host: u8,
    /// Substituted atomic number.
    pub dopant: u8,
    /// ChaCha8 seed used to select sites.
    pub seed: u64,
}
/// Substitute exactly `count` host atoms in a copied structure. Zero is allowed;
/// count cannot exceed available host sites. This is an initialization helper,
/// not a composition-changing MC move. Prepare a new calculator/reference and
/// absorber lists for the resulting chemistry.
pub fn seeded_substitution(
    c: &Configuration,
    host: u8,
    dopant: u8,
    count: usize,
    seed: u64,
) -> Result<SubstitutionResult, RmcError> {
    c.validate()?;
    require(
        host != dopant
            && [host, dopant]
                .iter()
                .all(|&z| crate::structure::Element::from_z(z).is_some()),
        "invalid substitution elements",
    )?;
    let mut atoms: Vec<_> = c
        .atoms
        .iter()
        .enumerate()
        .filter_map(|(i, a)| (a.atomic_number == host).then_some(i))
        .collect();
    require(
        count <= atoms.len(),
        "substitution count exceeds host sites",
    )?;
    atoms.shuffle(&mut ChaCha8Rng::seed_from_u64(seed));
    atoms.truncate(count);
    atoms.sort_unstable();
    let mut configuration = c.clone();
    for &a in &atoms {
        configuration.atoms[a].atomic_number = dopant;
    }
    Ok(SubstitutionResult {
        configuration,
        atoms,
        host,
        dopant,
        seed,
    })
}

/// Smallest positive repeat counts making all supercell interplanar spacings at
/// least `minimum_span` Å. Works with triclinic cells. A span of twice the local
/// cluster radius is a conservative starting point for avoiding overlapping
/// neighborhoods of periodic copies; structural correlations still need convergence
/// checks. Does not allocate the supercell or claim it fits resource limits.
pub fn suggested_supercell_repeats(
    structure: &crate::structure::Structure,
    minimum_span: f64,
) -> Result<[usize; 3], RmcError> {
    require(
        minimum_span.is_finite() && minimum_span > 0.,
        "supercell span must be positive",
    )?;
    let lattice = crate::structure::Lattice::from_matrix(structure.lattice.matrix)
        .map_err(|e| RmcError::Invalid(e.to_string()))?;
    let repeats: [f64; 3] = std::array::from_fn(|i| {
        (minimum_span / lattice.interplanar_spacing(i))
            .ceil()
            .max(1.)
    });
    require(
        repeats.iter().all(|v| v.is_finite() && *v <= 10_000.),
        "suggested supercell repeats exceed 10000",
    )?;
    Ok(repeats.map(|v| v as usize))
}
/// Geometry-only preparation estimate; electronic setup is not executed.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CatalogueResourceEstimate {
    /// Explicit atoms in the supplied cell.
    pub atoms: usize,
    /// Number of selected absorbers.
    pub absorbers: usize,
    /// Reserved directed paths, including the displacement envelope.
    pub catalogue_paths: usize,
    /// Paths already inside the active radius at the reference geometry.
    pub active_paths: usize,
    /// Sum of path scatterer vertices, including repeated/image vertices.
    pub path_vertices: usize,
    /// Worst-case retained χ array payload if every reserved path is active,
    /// `catalogue_paths * k_points * sizeof(f64)`. Excludes Rust allocation overhead,
    /// phase tensors, catalogue data, representative tables and transient copies.
    pub spectrum_payload_bytes: usize,
}
/// Enumerate/discard catalogues for explicit absorbers and count their work/memory
/// dimensions before electronic setup. Obeys the same enumeration guards as a
/// run; this can take time for large path order/radius/envelopes. No structures
/// or global caches are modified. k_points must be at least two.
pub fn estimate_catalogue_resources(
    c: &Configuration,
    absorbers: &[usize],
    settings: &PathCatalogueSettings,
    k_points: usize,
) -> Result<CatalogueResourceEstimate, RmcError> {
    c.validate()?;
    super::structural::indices(absorbers, c.atoms.len())?;
    require(
        (2..=1_000_000).contains(&k_points),
        "invalid estimate k point count",
    )?;
    let mut result = CatalogueResourceEstimate {
        atoms: c.atoms.len(),
        absorbers: absorbers.len(),
        catalogue_paths: 0,
        active_paths: 0,
        path_vertices: 0,
        spectrum_payload_bytes: 0,
    };
    for &absorber in absorbers {
        let catalogue = PathCatalogue::new(c.clone(), absorber, settings.clone())?;
        result.catalogue_paths = result
            .catalogue_paths
            .checked_add(catalogue.paths().len())
            .ok_or_else(|| RmcError::Invalid("catalogue estimate overflow".into()))?;
        for path in catalogue.paths() {
            result.path_vertices = result
                .path_vertices
                .checked_add(path.scatterers.len())
                .ok_or_else(|| RmcError::Invalid("path vertex estimate overflow".into()))?;
            result.active_paths += usize::from(path.half_length(c)? <= settings.radius);
        }
    }
    result.spectrum_payload_bytes = result
        .catalogue_paths
        .checked_mul(k_points)
        .and_then(|n| n.checked_mul(8))
        .ok_or_else(|| RmcError::Invalid("spectrum payload estimate overflow".into()))?;
    Ok(result)
}

/// Stream conventional multi-frame XYZ through a fixed RMC problem and emit
/// spectra one frame at a time. `component` selects the structure to replace;
/// the template's cell, species order, mixture fractions, datasets and displacement
/// reference stay fixed. Extended-XYZ cell metadata is not interpreted. Each
/// frame needs an atom-count line, a comment line and exactly that many atom rows.
/// Blank lines between frames are allowed; incomplete frames or changed species
/// are errors. Lines are limited to 16 KiB and configurations to 10,000 atoms.
///
/// The visitor receives an owned evaluated state and a zero-based frame index.
/// Return `ControlFlow::Break(())` for early completion. Returns the number emitted.
/// On failure, previously emitted frames remain with the caller; no later frame
/// is evaluated. Memory is bounded to one frame plus calculator caches unless the
/// visitor deliberately retains states. This function does not optimize coordinates.
pub fn stream_xyz_spectra<R: BufRead, C: ExafsCalculator + ?Sized>(
    mut reader: R,
    problem: &EnsembleProblem,
    component: usize,
    settings: &SessionSettings,
    calculator: &mut C,
    mut visitor: impl FnMut(usize, EnsembleState) -> Result<ControlFlow<()>, RmcError>,
) -> Result<usize, RmcError> {
    require(
        component < problem.structures.len(),
        "trajectory component out of range",
    )?;
    let mut template = problem.clone();
    super::session::normalize(&mut template.structures)?;
    let prepared = super::session::prepare(&template, settings)?;
    let reference = &template.structures[component].configuration;
    let mut emitted = 0;
    loop {
        let first = loop {
            let Some(line) = line(&mut reader)? else {
                return Ok(emitted);
            };
            if !line.trim().is_empty() {
                break line;
            }
        };
        let count = first
            .trim()
            .parse::<usize>()
            .map_err(|_| RmcError::Invalid(format!("XYZ frame {emitted} needs an atom count")))?;
        require(
            count == reference.atoms.len(),
            format!("XYZ frame {emitted} changed atom count"),
        )?;
        let comment = line(&mut reader)?
            .ok_or_else(|| RmcError::Invalid(format!("truncated XYZ frame {emitted} comment")))?;
        let mut text = first;
        text.push_str(&comment);
        for _ in 0..count {
            text.push_str(
                &line(&mut reader)?
                    .ok_or_else(|| RmcError::Invalid(format!("truncated XYZ frame {emitted}")))?,
            );
        }
        let xyz = crate::structure::parse_xyz(&text)
            .map_err(|e| RmcError::Invalid(format!("XYZ frame {emitted}: {e}")))?;
        let mut configuration = Configuration::from_xyz(&xyz)?;
        configuration.cell = reference.cell;
        require(
            configuration.atoms.len() == count
                && configuration
                    .atoms
                    .iter()
                    .zip(&reference.atoms)
                    .all(|(a, b)| a.atomic_number == b.atomic_number),
            format!("XYZ frame {emitted} changed species/order"),
        )?;
        let mut structures = template.structures.clone();
        structures[component].configuration = configuration;
        require(
            settings.constraints.allowed(
                &structures,
                &template.structures,
                &settings.moves,
                None,
            )?,
            format!("XYZ frame {emitted} violates fixed-reference constraints"),
        )?;
        let state = super::session::evaluate_prepared(
            &template, settings, &prepared, structures, None, calculator,
        )?;
        let flow = visitor(emitted, state)?;
        emitted += 1;
        if flow.is_break() {
            return Ok(emitted);
        }
    }
}
fn line(reader: &mut impl BufRead) -> Result<Option<String>, RmcError> {
    let mut text = String::new();
    let bytes = reader
        .by_ref()
        .take(16385)
        .read_line(&mut text)
        .map_err(|e| RmcError::Invalid(format!("trajectory read: {e}")))?;
    require(bytes <= 16384, "XYZ line exceeds 16 KiB")?;
    Ok((bytes > 0).then_some(text))
}
