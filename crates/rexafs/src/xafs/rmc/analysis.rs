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

/// Unreleased radial distribution with an explicit normalization convention.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RadialDistribution {
    /// Raw neighbors per center per bin and moments over the supplied interval.
    pub neighbors: DistanceDistribution,
    /// Dimensionless bulk g(r), or None for a finite cluster without a density.
    pub g_r: Option<Vec<f64>>,
    /// Selected species number density in Å⁻³, or None for a finite cluster.
    pub density: Option<f64>,
    /// Half the smallest cell-plane spacing in Å, or None for a finite cluster.
    /// Radii above this conservative limit probe repeated-cell correlations.
    pub independent_radius: Option<f64>,
}

/// Unreleased: compute a species-resolved radial distribution around explicit
/// centers. Inputs and self exclusion follow [`distance_distribution`].
///
/// Periodic cells use `g_i = H_i / (rho * V_i)`, where H_i is the neighbor count
/// per center, rho = N_B / V_cell is the selected neighbor species density in
/// Å⁻³ (all species when None), and V_i = 4π/3 (r_outer³ − r_inner³) is the
/// spherical bin volume in Å³. Thus g_i is dimensionless. This follows the
/// center/shell/density normalization described by
/// [GROMACS](https://manual.gromacs.org/current/onlinehelp/gmx-rdf.html), using
/// the entire explicit cell density rather than a local-density estimate.
/// Same-species density includes all N_B atoms; the central self image is
/// excluded from counts and no finite-N correction is applied. All other
/// periodic images inside the range contribute. Radii beyond half the smallest
/// cell-plane spacing remain allowed but are not independent-cell information.
///
/// Finite clusters return only labeled neighbor counts, since no bulk volume is
/// defined. An absent neighbor species in a periodic cell is an error (zero
/// density cannot normalize g). Returns owned arrays; inputs remain unchanged.
/// Choose bins and a radial interval explicitly; moments describe that entire
/// interval and do not identify a coordination shell automatically.
pub fn radial_distribution(
    configuration: &Configuration,
    centers: &[usize],
    neighbor_element: Option<u8>,
    edges: &[f64],
) -> Result<RadialDistribution, RmcError> {
    let neighbors = distance_distribution(configuration, centers, neighbor_element, edges)?;
    let (g_r, density, independent_radius) = if let Some(lattice) = configuration.lattice()? {
        let count = configuration
            .atoms
            .iter()
            .filter(|atom| neighbor_element.is_none_or(|z| atom.atomic_number == z))
            .count();
        require(
            count > 0,
            "radial distribution needs a nonzero neighbor-species density",
        )?;
        let density = count as f64 / lattice.volume();
        require(
            density.is_finite() && density > 0.,
            "invalid periodic species density",
        )?;
        require(
            edges.windows(2).all(|r| {
                let shell = (4. * std::f64::consts::PI / 3.) * (r[1].powi(3) - r[0].powi(3));
                shell.is_finite() && shell > 0.
            }),
            "spherical shell volumes must be finite and positive",
        )?;
        let values = neighbors
            .counts_per_absorber
            .iter()
            .zip(edges.windows(2))
            .map(|(count, r)| {
                count / (density * (4. * std::f64::consts::PI / 3.) * (r[1].powi(3) - r[0].powi(3)))
            })
            .collect();
        let radius = (0..3)
            .map(|i| lattice.interplanar_spacing(i) / 2.)
            .fold(f64::INFINITY, f64::min);
        (Some(values), Some(density), Some(radius))
    } else {
        (None, None, None)
    };
    Ok(RadialDistribution {
        neighbors,
        g_r,
        density,
        independent_radius,
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
