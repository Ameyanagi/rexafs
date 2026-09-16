use super::{require, RmcError};
use crate::structure::{Element, Lattice, Structure, Xyz};
use serde::{Deserialize, Serialize};

/// An explicit atom with fixed chemical identity; its vector position is its ID.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Atom {
    /// Atomic number, validated against the rexafs element table.
    pub atomic_number: u8,
    /// Cartesian coordinates in Å. Refinement preserves unwrapped positions.
    pub position: [f64; 3],
}

/// Finite atomic cluster or explicit periodic cell. The cell is fixed throughout
/// refinement; moving an atom moves all its periodic images together.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Configuration {
    /// Nonempty atoms in stable order, with no implicit occupancy or symmetry.
    pub atoms: Vec<Atom>,
    /// Optional Cartesian lattice row vectors a, b, c in Å. None means a finite
    /// cluster with no periodic images. General nonsingular triclinic cells work.
    pub cell: Option<[[f64; 3]; 3]>,
}

impl Configuration {
    /// Copy a parsed XYZ cluster, retaining atom order and Cartesian coordinates.
    /// Validates atomic numbers, symbol consistency and finite coordinates.
    pub fn from_xyz(xyz: &Xyz) -> Result<Self, RmcError> {
        for atom in &xyz.atoms {
            require(
                Element::from_symbol(&atom.symbol).is_some_and(|e| e.z == atom.z),
                "XYZ symbol and atomic number disagree",
            )?;
        }
        let configuration = Self {
            cell: None,
            atoms: xyz
                .atoms
                .iter()
                .map(|a| Atom {
                    atomic_number: a.z,
                    position: a.cart,
                })
                .collect(),
        };
        configuration.validate()?;
        Ok(configuration)
    }

    /// Expand the already symmetry-expanded `structure.sites` into an explicit
    /// periodic supercell. Repetitions along a,b,c must be positive; at most 10,000
    /// atoms are allowed by this reference implementation. Order is a-image,
    /// b-image, c-image, then input site. Coordinates are wrapped into the input
    /// cell first. Mixed or partially occupied sites require a user-created
    /// explicit realization and are rejected instead of selecting a majority.
    pub fn from_structure(structure: &Structure, repeats: [usize; 3]) -> Result<Self, RmcError> {
        require(
            repeats.iter().all(|&n| n > 0),
            "supercell repetitions must be positive",
        )?;
        let count = repeats
            .iter()
            .try_fold(structure.sites.len(), |n, r| n.checked_mul(*r));
        require(
            count.is_some_and(|n| n > 0 && n <= 10_000),
            "supercell must contain 1..=10000 atoms",
        )?;
        let lattice = checked_lattice(structure.lattice.matrix)?;
        let mut species = Vec::new();
        for site in &structure.sites {
            require(
                site.species.len() == 1 && (site.species[0].occupancy - 1.0).abs() <= 1e-12,
                "RMC requires explicit fully occupied single-species sites",
            )?;
            require(
                site.frac.iter().all(|x| x.is_finite()),
                "site coordinates must be finite",
            )?;
            let element = site.species[0].element().ok_or_else(|| {
                RmcError::Invalid(format!("unknown element at site {}", site.label))
            })?;
            species.push(element.z);
        }
        let mut atoms = Vec::with_capacity(count.unwrap());
        for a in 0..repeats[0] {
            for b in 0..repeats[1] {
                for c in 0..repeats[2] {
                    for (site, &atomic_number) in structure.sites.iter().zip(&species) {
                        let frac = std::array::from_fn(|i| {
                            site.frac[i].rem_euclid(1.0) + [a, b, c][i] as f64
                        });
                        atoms.push(Atom {
                            atomic_number,
                            position: lattice.to_cart(frac),
                        });
                    }
                }
            }
        }
        let cell = std::array::from_fn(|i| lattice.matrix[i].map(|v| v * repeats[i] as f64));
        let configuration = Self {
            atoms,
            cell: Some(cell),
        };
        configuration.validate()?;
        Ok(configuration)
    }

    /// Validate finite coordinates, elements, atom count and cell geometry.
    /// Pair-distance constraints are checked by `refine`, not by this operation.
    pub fn validate(&self) -> Result<(), RmcError> {
        require(
            !self.atoms.is_empty() && self.atoms.len() <= 10_000,
            "configuration needs 1..=10000 atoms",
        )?;
        for atom in &self.atoms {
            require(
                Element::from_z(atom.atomic_number).is_some(),
                "unknown atomic number",
            )?;
            require(
                atom.position
                    .iter()
                    .all(|x| x.is_finite() && x.abs() <= 1e8),
                "Cartesian coordinates must be finite and within ±1e8 Å",
            )?;
        }
        self.lattice()?;
        Ok(())
    }

    pub(crate) fn lattice(&self) -> Result<Option<Lattice>, RmcError> {
        self.cell.map(checked_lattice).transpose()
    }

    /// Export one frame in plain XYZ, retaining unwrapped coordinates and order.
    /// Periodic cell information is not represented; retain the JSON result too.
    pub fn to_xyz(&self) -> Result<String, RmcError> {
        self.validate()?;
        let mut out = format!(
            "{}\nrexafs RMC; Cartesian Å; cell retained in JSON only\n",
            self.atoms.len()
        );
        for atom in &self.atoms {
            let [x, y, z] = atom.position;
            out.push_str(&format!(
                "{} {x:.12} {y:.12} {z:.12}\n",
                Element::from_z(atom.atomic_number).unwrap().symbol
            ));
        }
        Ok(out)
    }
}

fn checked_lattice(matrix: [[f64; 3]; 3]) -> Result<Lattice, RmcError> {
    require(
        matrix
            .iter()
            .flatten()
            .all(|x| x.is_finite() && x.abs() <= 1e8),
        "invalid cell vectors",
    )?;
    let lattice = Lattice::from_matrix(matrix).map_err(|e| RmcError::Invalid(e.to_string()))?;
    require(
        (0..3).all(|i| {
            let h = lattice.interplanar_spacing(i);
            h.is_finite() && h > 1e-6
        }),
        "cell plane spacings must be finite and greater than 1e-6 Å",
    )?;
    Ok(lattice)
}

pub(crate) fn distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    (a[0] - b[0]).hypot(a[1] - b[1]).hypot(a[2] - b[2])
}

// Enumerating images avoids the incorrect fractional-rounding minimum-image
// shortcut for skew cells. If |r|<R then each fractional component is <R/h_i,
// where h_i is the corresponding interplanar spacing.
pub(crate) fn image_bounds(lattice: &Lattice, radius: f64) -> Result<[i32; 3], RmcError> {
    let bounds: [f64; 3] =
        std::array::from_fn(|i| (radius / lattice.interplanar_spacing(i) + 1.0).ceil());
    require(
        bounds.iter().all(|n| n.is_finite() && *n <= 100.0)
            && bounds.iter().map(|n| 2.0 * n + 1.0).product::<f64>() <= 100_000.0,
        "cell needs too many periodic images; use a larger or better-conditioned cell",
    )?;
    Ok(bounds.map(|n| n as i32))
}

pub(crate) fn distances_allowed(
    configuration: &Configuration,
    lattice: Option<&Lattice>,
    minimum: f64,
    moved: Option<usize>,
) -> Result<bool, RmcError> {
    let bounds = lattice.map(|l| image_bounds(l, minimum)).transpose()?;
    let centers: Vec<_> =
        moved.map_or_else(|| (0..configuration.atoms.len()).collect(), |i| vec![i]);
    for i in centers {
        let first = if moved.is_some() { 0 } else { i };
        for j in first..configuration.atoms.len() {
            let a = configuration.atoms[i].position;
            let b = configuration.atoms[j].position;
            if let (Some(lattice), Some(bounds)) = (lattice, bounds) {
                let delta = std::array::from_fn(|axis| a[axis] - b[axis]);
                let frac = lattice.to_frac(delta).map(|v| v - v.round());
                for x in -bounds[0]..=bounds[0] {
                    for y in -bounds[1]..=bounds[1] {
                        for z in -bounds[2]..=bounds[2] {
                            if i == j && [x, y, z] == [0, 0, 0] {
                                continue;
                            }
                            let image =
                                [frac[0] + x as f64, frac[1] + y as f64, frac[2] + z as f64];
                            if distance(lattice.to_cart(image), [0.0; 3]) < minimum {
                                return Ok(false);
                            }
                        }
                    }
                }
            } else if i != j && distance(a, b) < minimum {
                return Ok(false);
            }
        }
    }
    Ok(true)
}
