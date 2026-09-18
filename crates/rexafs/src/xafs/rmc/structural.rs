//! Structural summaries and explicit histogram conventions (since 0.2.10).
use super::geometry::distance;
use super::*;

/// Histogram and population moments of scalar structural measurements. Counts
/// are raw observations, not density-normalized g(r) or statistical uncertainty.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StructuralDistribution {
    /// Increasing bin edges in the quantity's stated units; final edge exclusive.
    pub edges: Vec<f64>,
    /// Raw counts inside each half-open bin.
    pub counts: Vec<usize>,
    /// Observations below the first edge.
    pub underflow: usize,
    /// Observations at or above the final edge.
    pub overflow: usize,
    /// Total observations, including underflow and overflow.
    pub count: usize,
    /// Arithmetic mean of all observations; None for an empty distribution.
    pub mean: Option<f64>,
    /// Population variance of all observations; None when empty.
    pub variance: Option<f64>,
}
pub(super) fn distribution(
    values: impl IntoIterator<Item = f64>,
    edges: &[f64],
) -> Result<StructuralDistribution, RmcError> {
    require(
        edges.len() >= 2
            && edges.len() <= 1_000_001
            && edges.iter().all(|v| v.is_finite())
            && edges.windows(2).all(|w| w[1] > w[0]),
        "invalid structural histogram edges",
    )?;
    let mut result = StructuralDistribution {
        edges: edges.to_vec(),
        counts: vec![0; edges.len() - 1],
        underflow: 0,
        overflow: 0,
        count: 0,
        mean: None,
        variance: None,
    };
    let mut mean = 0.;
    let mut m2 = 0.;
    for value in values {
        require(value.is_finite(), "nonfinite structural observation")?;
        result.count += 1;
        let delta = value - mean;
        mean += delta / result.count as f64;
        m2 += delta * (value - mean);
        if value < edges[0] {
            result.underflow += 1;
        } else if value >= *edges.last().unwrap() {
            result.overflow += 1;
        } else {
            result.counts[edges.partition_point(|&v| v <= value) - 1] += 1;
        }
    }
    if result.count > 0 {
        result.mean = Some(mean);
        result.variance = Some(m2 / result.count as f64);
    }
    Ok(result)
}

/// Unwrapped atomic displacement statistics. No rotational alignment is applied;
/// periodic crossings retain the same atom/image identities as the RMC trajectory.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DisplacementReport {
    /// Number of selected, distinct atoms.
    pub atoms: usize,
    /// Mean displacement before optional centering, Cartesian Å.
    pub translation: [f64; 3],
    /// Mean outer product of analyzed displacement vectors, in Å².
    pub second_moment: [[f64; 3]; 3],
    /// Trace of the second-moment tensor, in Å².
    pub mean_square: f64,
    /// Square root of mean_square, in Å.
    pub rms: f64,
    /// Largest analyzed displacement magnitude, in Å.
    pub maximum: f64,
    /// Whether the mean translation was subtracted from every displacement.
    pub translation_removed: bool,
}
/// Compare fixed-topology structures. `atoms` must be nonempty and distinct.
/// Removing translation measures internal disorder; retaining it also counts
/// collective drift. This computes an arithmetic mean-square displacement,
/// not EVAX's historically named median-based MSD statistic.
pub fn displacement_report(
    reference: &Configuration,
    current: &Configuration,
    atoms: &[usize],
    remove_translation: bool,
) -> Result<DisplacementReport, RmcError> {
    reference.validate()?;
    current.validate()?;
    require(
        reference.cell == current.cell
            && reference.atoms.len() == current.atoms.len()
            && reference
                .atoms
                .iter()
                .zip(&current.atoms)
                .all(|(a, b)| a.atomic_number == b.atomic_number),
        "displacement report changed topology",
    )?;
    indices(atoms, current.atoms.len())?;
    let delta: Vec<[f64; 3]> = atoms
        .iter()
        .map(|&a| {
            std::array::from_fn(|i| current.atoms[a].position[i] - reference.atoms[a].position[i])
        })
        .collect();
    let translation =
        std::array::from_fn(|i| delta.iter().map(|d| d[i]).sum::<f64>() / atoms.len() as f64);
    let mut second_moment = [[0.; 3]; 3];
    let mut maximum: f64 = 0.;
    for d in delta {
        let d: [f64; 3] = std::array::from_fn(|i| {
            d[i] - if remove_translation {
                translation[i]
            } else {
                0.
            }
        });
        maximum = maximum.max(distance(d, [0.; 3]));
        for i in 0..3 {
            for j in 0..3 {
                second_moment[i][j] += d[i] * d[j] / atoms.len() as f64;
            }
        }
    }
    let mean_square = (0..3).map(|i| second_moment[i][i]).sum::<f64>();
    Ok(DisplacementReport {
        atoms: atoms.len(),
        translation,
        second_moment,
        mean_square,
        rms: mean_square.sqrt(),
        maximum,
        translation_removed: remove_translation,
    })
}
pub(super) fn indices(atoms: &[usize], len: usize) -> Result<(), RmcError> {
    require(
        !atoms.is_empty()
            && atoms.iter().all(|&i| i < len)
            && atoms
                .iter()
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                == atoms.len(),
        "atom selection must be nonempty, distinct and in range",
    )
}

/// Explicit path geometry with stable periodic identities. Lengths are in Å;
/// angles are degrees between rays to neighboring vertices, so a reversal has
/// internal angle zero (scattering deflection 180°). Includes the absorber angle.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PathGeometryReport {
    /// Path as supplied, including stable atom/image identities.
    pub path: ScatteringPath,
    /// Half the closed walk length, in Å.
    pub half_length: f64,
    /// Leg lengths in propagation order, starting at the absorber.
    pub leg_lengths: Vec<f64>,
    /// Internal vertex angles in degrees, absorber first.
    pub angles: Vec<f64>,
}
/// Analyze one explicit path without a scattering calculation. Rejects zero
/// length legs; repeated nonconsecutive vertices are allowed.
pub fn path_geometry_report(
    c: &Configuration,
    path: &ScatteringPath,
) -> Result<PathGeometryReport, RmcError> {
    c.validate()?;
    let p = path.positions(c)?;
    let legs = p.len() - 1;
    let leg_lengths: Vec<_> = p.windows(2).map(|v| distance(v[0], v[1])).collect();
    require(
        leg_lengths.iter().all(|v| *v > 1e-8),
        "path report contains coincident consecutive vertices",
    )?;
    let angles = (0..legs)
        .map(|i| angle(p[(i + legs - 1) % legs], p[i], p[(i + 1) % legs]))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(PathGeometryReport {
        path: path.clone(),
        half_length: leg_lengths.iter().sum::<f64>() / 2.,
        leg_lengths,
        angles,
    })
}
/// Histogram explicit path half-lengths in Å. Every listed directed path counts
/// once; supplying reverse walks intentionally counts both orientations.
pub fn path_length_distribution(
    c: &Configuration,
    paths: &[ScatteringPath],
    edges: &[f64],
) -> Result<StructuralDistribution, RmcError> {
    c.validate()?;
    distribution(
        paths
            .iter()
            .map(|p| p.half_length(c))
            .collect::<Result<Vec<_>, _>>()?,
        edges,
    )
}
/// Histogram bond angles in degrees around selected centers. All unordered pairs
/// of neighbors within the exclusive cutoff (Å) count once. Optional element
/// pairs are unordered. Periodic atom images are explicit; repeated copies of
/// the same supercell atom can form a pair. At most one million angle pairs are
/// visited across all centers; reduce the cutoff/selection if that guard trips.
pub fn angle_distribution(
    c: &Configuration,
    centers: &[usize],
    elements: Option<[u8; 2]>,
    cutoff: f64,
    edges: &[f64],
) -> Result<StructuralDistribution, RmcError> {
    c.validate()?;
    indices(centers, c.atoms.len())?;
    require(
        cutoff.is_finite()
            && cutoff > 0.
            && elements.is_none_or(|z| {
                z.iter()
                    .all(|&z| crate::structure::Element::from_z(z).is_some())
            }),
        "invalid angle cutoff or elements",
    )?;
    let mut angles = Vec::new();
    let mut pairs = 0usize;
    for &center in centers {
        let origin = c.atoms[center].position;
        let neighbors = super::paths::neighbor_images(c, center, cutoff)?;
        for (i, a) in neighbors.iter().enumerate() {
            let pa = a.position(c)?;
            if distance(pa, origin) >= cutoff {
                continue;
            }
            for b in &neighbors[i + 1..] {
                pairs += 1;
                require(
                    pairs <= 1_000_000,
                    "angle report exceeds one million neighbor pairs",
                )?;
                let pb = b.position(c)?;
                if distance(pb, origin) >= cutoff {
                    continue;
                }
                let z = [c.atoms[a.atom].atomic_number, c.atoms[b.atom].atomic_number];
                if elements.is_none_or(|e| e == z || e == [z[1], z[0]]) {
                    angles.push(angle(pa, origin, pb)?);
                }
            }
        }
    }
    distribution(angles, edges)
}
fn angle(a: [f64; 3], center: [f64; 3], b: [f64; 3]) -> Result<f64, RmcError> {
    let norm = distance(a, center) * distance(b, center);
    require(
        norm > 1e-16 && norm.is_finite(),
        "angle has coincident vertices",
    )?;
    Ok(((0..3)
        .map(|i| (a[i] - center[i]) * (b[i] - center[i]))
        .sum::<f64>()
        / norm)
        .clamp(-1., 1.)
        .acos()
        .to_degrees())
}
