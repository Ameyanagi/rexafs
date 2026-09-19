use super::geometry::{distance, image_bounds};
use super::*;

/// Coordinate frame sampled after an attempted move, including rejected moves.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrajectoryFrame {
    /// Since 0.2.11: per-dataset theoretical ΔE₀ in eV at this frame. Empty
    /// historical/fixed frames use the original problem's fixed shifts.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub delta_e0: Vec<f64>,
    /// Attempt count; zero denotes the initial structure.
    pub step: usize,
    /// Unwrapped coordinates, cells and mixture weights.
    pub structures: Vec<WeightedStructure>,
    /// Total numerical objective at this frame.
    pub score: f64,
}

/// Per-absorber distance histogram, without a bulk-density g(r) normalization.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DistanceDistribution {
    /// Bin edges in Å; the final edge is exclusive.
    pub edges: Vec<f64>,
    /// Neighbor count in each bin divided by the number of supplied absorbers.
    pub counts_per_absorber: Vec<f64>,
    /// Total neighbors inside the histogram range, divided by absorber count.
    pub coordination: f64,
    /// Mean included distance in Å; None when there are no neighbors.
    pub mean: Option<f64>,
    /// Population variance of included distances in Å²; None when empty.
    pub variance: Option<f64>,
}

/// Count finite-cluster or periodic-image neighbors around explicit absorbers.
/// `neighbor_element=None` includes all elements. Distinct absorbers are averaged
/// equally; periodic images of an absorber are included except its central image.
/// At most one million bins and ten million candidate images per absorber are allowed.
pub fn distance_distribution(
    configuration: &Configuration,
    absorbers: &[usize],
    neighbor_element: Option<u8>,
    edges: &[f64],
) -> Result<DistanceDistribution, RmcError> {
    configuration.validate()?;
    require(
        !absorbers.is_empty()
            && absorbers.iter().all(|&i| i < configuration.atoms.len())
            && absorbers
                .iter()
                .collect::<std::collections::HashSet<_>>()
                .len()
                == absorbers.len(),
        "invalid distribution absorber indices",
    )?;
    require(
        neighbor_element.is_none_or(|z| crate::structure::Element::from_z(z).is_some()),
        "invalid neighbor element",
    )?;
    require(
        edges.len() >= 2
            && edges.len() <= 1_000_001
            && edges.iter().all(|v| v.is_finite() && *v >= 0.)
            && edges.windows(2).all(|v| v[1] > v[0]),
        "distance bin edges must be finite, nonnegative and increasing",
    )?;
    let mut counts = vec![0.; edges.len() - 1];
    let mut n = 0usize;
    let mut mean = 0.;
    let mut m2 = 0.;
    for &i in absorbers {
        visit_neighbors(configuration, i, *edges.last().unwrap(), |j, r| {
            if neighbor_element.is_none_or(|z| configuration.atoms[j].atomic_number == z)
                && r >= edges[0]
            {
                let bin = edges.partition_point(|&v| v <= r) - 1;
                counts[bin] += 1.;
                n += 1;
                let delta = r - mean;
                mean += delta / n as f64;
                m2 += delta * (r - mean);
            }
            true
        })?;
    }
    for v in &mut counts {
        *v /= absorbers.len() as f64;
    }
    Ok(DistanceDistribution {
        edges: edges.to_vec(),
        counts_per_absorber: counts,
        coordination: n as f64 / absorbers.len() as f64,
        mean: (n > 0).then_some(mean),
        variance: (n > 0).then_some(m2 / n.max(1) as f64),
    })
}

// Enumerate all images inside a cutoff, including skew cells. Callback false
// stops early; fractional rounding only recenters the enumeration, never selects
// a purported nearest image.
pub(super) fn visit_neighbors(
    configuration: &Configuration,
    center: usize,
    cutoff: f64,
    mut visit: impl FnMut(usize, f64) -> bool,
) -> Result<bool, RmcError> {
    let a = configuration.atoms[center].position;
    if let Some(lattice) = configuration.lattice()? {
        let bounds = image_bounds(&lattice, cutoff)?;
        require(
            bounds
                .iter()
                .map(|n| (2 * n + 1) as usize)
                .product::<usize>()
                .saturating_mul(configuration.atoms.len())
                <= 10_000_000,
            "neighbor enumeration exceeds ten million candidate images per absorber",
        )?;
        for (j, b) in configuration.atoms.iter().enumerate() {
            let frac = lattice
                .to_frac(std::array::from_fn(|axis| b.position[axis] - a[axis]))
                .map(|v| v - v.round());
            for x in -bounds[0]..=bounds[0] {
                for y in -bounds[1]..=bounds[1] {
                    for z in -bounds[2]..=bounds[2] {
                        if center == j && [x, y, z] == [0, 0, 0] {
                            continue;
                        }
                        let r = distance(
                            lattice.to_cart([
                                frac[0] + x as f64,
                                frac[1] + y as f64,
                                frac[2] + z as f64,
                            ]),
                            [0.; 3],
                        );
                        if r < cutoff && !visit(j, r) {
                            return Ok(false);
                        }
                    }
                }
            }
        }
    } else {
        for (j, b) in configuration.atoms.iter().enumerate() {
            if j != center {
                let r = distance(a, b.position);
                if r < cutoff && !visit(j, r) {
                    return Ok(false);
                }
            }
        }
    }
    Ok(true)
}

pub(super) fn bond_distance(c: &Configuration, a: usize, b: usize) -> Result<f64, RmcError> {
    let direct = distance(c.atoms[a].position, c.atoms[b].position);
    if c.cell.is_none() {
        return Ok(direct);
    }
    // A centered fractional image supplies a finite upper bound even for large
    // unwrapped displacements. Enumerating that sphere finds the true nearest image.
    let lattice = c.lattice()?.unwrap();
    let frac = lattice
        .to_frac(std::array::from_fn(|i| {
            c.atoms[b].position[i] - c.atoms[a].position[i]
        }))
        .map(|v| v - v.round());
    let mut best = distance(lattice.to_cart(frac), [0.; 3]);
    visit_neighbors(c, a, best + 1e-10, |j, r| {
        if j == b {
            best = best.min(r);
        }
        true
    })?;
    Ok(best)
}
