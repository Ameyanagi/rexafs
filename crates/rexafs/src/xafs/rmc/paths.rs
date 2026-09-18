//! Stable, explicit scattering walks; no crystal degeneracy is frozen into disorder.
use super::geometry::{distance, image_bounds};
use super::*;

/// Stable atom and lattice-image identity, independent of neighbor sorting.
/// Added in 0.2.10. Images translate the unwrapped atom by integer
/// multiples of the fixed cell's row vectors; finite clusters use `[0; 3]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AtomImage {
    /// Zero-based index in `Configuration::atoms`.
    pub atom: usize,
    /// Integer cell translation, in a, b, c order.
    pub image: [i32; 3],
}
impl AtomImage {
    /// Cartesian position in Å. Rejects an absent atom or nonzero finite-cluster image.
    pub fn position(&self, c: &Configuration) -> Result<[f64; 3], RmcError> {
        require(self.atom < c.atoms.len(), "path atom index out of range")?;
        require(
            c.cell.is_some() || self.image == [0; 3],
            "finite cluster has a lattice image",
        )?;
        Ok(std::array::from_fn(|axis| {
            c.atoms[self.atom].position[axis]
                + c.cell.map_or(0., |cell| {
                    (0..3)
                        .map(|i| cell[i][axis] * self.image[i] as f64)
                        .sum::<f64>()
                })
        }))
    }
}

/// A directed, closed scattering walk. The absorber starts and ends the walk
/// implicitly; `scatterers` contains its intermediate vertices. Reverse walks
/// are distinct, so every contribution has degeneracy one. Repeated scatterers
/// and intermediate visits to the central absorber are allowed, as in ReFEFF's
/// multiple-scattering search; consecutive identical vertices are excluded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScatteringPath {
    /// Central atom in image zero.
    pub absorber: usize,
    /// Intermediate atom/image identities, in propagation order.
    pub scatterers: Vec<AtomImage>,
}
impl ScatteringPath {
    /// Total propagation length divided by two, in Å.
    pub fn half_length(&self, c: &Configuration) -> Result<f64, RmcError> {
        let vertices = self.positions(c)?;
        Ok(vertices
            .windows(2)
            .map(|p| distance(p[0], p[1]))
            .sum::<f64>()
            * 0.5)
    }
    /// Coordinates in Å, starting and ending at the absorber. Returns owned data.
    pub fn positions(&self, c: &Configuration) -> Result<Vec<[f64; 3]>, RmcError> {
        require(
            !self.scatterers.is_empty(),
            "a scattering path needs a scatterer",
        )?;
        let origin = AtomImage {
            atom: self.absorber,
            image: [0; 3],
        }
        .position(c)?;
        let mut result = vec![origin];
        for v in &self.scatterers {
            result.push(v.position(c)?);
        }
        result.push(origin);
        Ok(result)
    }
}

/// Fixed path search limits. These are rexafs enumeration limits, not FEFF's
/// amplitude screening. A conservative displacement envelope admits paths that
/// can enter the radius later; it may substantially increase preparation cost.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct PathCatalogueSettings {
    /// Maximum current half-path length in Å; default 4.
    pub radius: f64,
    /// Maximum number of legs, 2..=6; default 4.
    pub max_legs: usize,
    /// Maximum unwrapped displacement of each atom from the reference, in Å;
    /// default 0.2. Requests outside this envelope fail, never silently omit paths.
    pub displacement: f64,
    /// Maximum retained directed paths per absorber; default 100,000.
    pub max_paths: usize,
    /// Maximum candidate extensions during search; default ten million.
    pub max_extensions: usize,
}
impl Default for PathCatalogueSettings {
    fn default() -> Self {
        Self {
            radius: 4.,
            max_legs: 4,
            displacement: 0.2,
            max_paths: 100_000,
            max_extensions: 10_000_000,
        }
    }
}

/// Immutable catalogue valid within a declared displacement envelope. Enumeration
/// uses only geometry; no amplitude criteria or reference crystal degeneracies
/// are carried into subsequent disorder. Its atom-to-path map supports local updates.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct PathCatalogue {
    reference: Configuration,
    settings: PathCatalogueSettings,
    paths: Vec<ScatteringPath>,
    by_atom: Vec<Vec<usize>>,
}
impl PathCatalogue {
    /// Enumerate paths once in deterministic atom/image order. Uses a conservative
    /// search radius `radius + max_legs * displacement` (Å): each leg can shorten
    /// by at most twice the per-atom displacement. Resource overflow is an error.
    pub fn new(
        reference: Configuration,
        absorber: usize,
        settings: PathCatalogueSettings,
    ) -> Result<Self, RmcError> {
        reference.validate()?;
        require(
            absorber < reference.atoms.len(),
            "catalogue absorber out of range",
        )?;
        require(
            settings.radius.is_finite()
                && settings.radius > 0.
                && settings.displacement.is_finite()
                && settings.displacement >= 0.
                && (2..=6).contains(&settings.max_legs)
                && settings.max_paths > 0
                && settings.max_extensions > 0,
            "invalid path catalogue limits",
        )?;
        let limit = settings.radius + settings.max_legs as f64 * settings.displacement;
        let mut vertices = neighbor_images(&reference, absorber, limit)?;
        vertices.push(AtomImage {
            atom: absorber,
            image: [0; 3],
        });
        require(
            vertices.len() <= 2000,
            "path catalogue exceeds 2000 local vertices",
        )?;
        let origin = reference.atoms[absorber].position;
        let positions = vertices
            .iter()
            .map(|v| v.position(&reference))
            .collect::<Result<Vec<_>, _>>()?;
        struct Search<'a> {
            vertices: &'a [AtomImage],
            positions: &'a [[f64; 3]],
            origin: [f64; 3],
            limit: f64,
            absorber: usize,
            settings: &'a PathCatalogueSettings,
            paths: Vec<ScatteringPath>,
            extensions: usize,
        }
        impl Search<'_> {
            fn extend(
                &mut self,
                last: [f64; 3],
                length: f64,
                stack: &mut Vec<usize>,
            ) -> Result<(), RmcError> {
                for i in 0..self.positions.len() {
                    let pos = self.positions[i];
                    self.extensions += 1;
                    require(
                        self.extensions <= self.settings.max_extensions,
                        "path search exceeds max_extensions",
                    )?;
                    if stack.last() == Some(&i) {
                        continue;
                    }
                    let central = self.vertices[i]
                        == (AtomImage {
                            atom: self.absorber,
                            image: [0; 3],
                        });
                    if stack.is_empty() && central {
                        continue;
                    }
                    let leg = distance(last, pos);
                    require(leg > 1e-8, "coincident vertices in path catalogue")?;
                    let next = length + leg;
                    let total = next + distance(pos, self.origin);
                    if total > 2. * self.limit + 1e-10 {
                        continue;
                    }
                    stack.push(i);
                    // Use the tighter displacement envelope for this leg count.
                    if !central
                        && total
                            <= 2.
                                * (self.settings.radius
                                    + (stack.len() + 1) as f64 * self.settings.displacement)
                                + 1e-10
                    {
                        require(
                            self.paths.len() < self.settings.max_paths,
                            "path catalogue exceeds max_paths",
                        )?;
                        self.paths.push(ScatteringPath {
                            absorber: self.absorber,
                            scatterers: stack.iter().map(|&j| self.vertices[j]).collect(),
                        });
                    }
                    if stack.len() + 1 < self.settings.max_legs {
                        self.extend(pos, next, stack)?;
                    }
                    stack.pop();
                }
                Ok(())
            }
        }
        let mut search = Search {
            vertices: &vertices,
            positions: &positions,
            origin,
            limit,
            absorber,
            settings: &settings,
            paths: Vec::new(),
            extensions: 0,
        };
        search.extend(origin, 0., &mut Vec::new())?;
        let paths = search.paths;
        let mut by_atom = vec![Vec::new(); reference.atoms.len()];
        for (i, path) in paths.iter().enumerate() {
            let atoms: std::collections::BTreeSet<_> = std::iter::once(absorber)
                .chain(path.scatterers.iter().map(|v| v.atom))
                .collect();
            for atom in atoms {
                by_atom[atom].push(i);
            }
        }
        Ok(Self {
            reference,
            settings,
            paths,
            by_atom,
        })
    }
    /// Borrow the fixed directed paths, including currently out-of-radius paths.
    pub fn paths(&self) -> &[ScatteringPath] {
        &self.paths
    }
    /// Borrow the reference geometry without changing catalogue validity.
    pub fn reference(&self) -> &Configuration {
        &self.reference
    }
    /// Borrow validated enumeration limits.
    pub fn settings(&self) -> &PathCatalogueSettings {
        &self.settings
    }
    /// Check unchanged species/order/cell and the unwrapped displacement envelope.
    pub fn validate(&self, c: &Configuration) -> Result<(), RmcError> {
        c.validate()?;
        require(
            c.cell == self.reference.cell
                && c.atoms.len() == self.reference.atoms.len()
                && c.atoms.iter().zip(&self.reference.atoms).all(|(a, b)| {
                    a.atomic_number == b.atomic_number
                        && distance(a.position, b.position) <= self.settings.displacement + 1e-12
                }),
            "geometry exceeds fixed path catalogue topology/displacement envelope",
        )
    }
    /// Sorted unique paths containing any listed atom (including absorber images).
    /// Validates indices; an empty list returns no paths.
    pub fn affected_paths(&self, atoms: &[usize]) -> Result<Vec<usize>, RmcError> {
        let mut selected = std::collections::BTreeSet::new();
        for &a in atoms {
            require(a < self.by_atom.len(), "changed atom out of range")?;
            selected.extend(self.by_atom[a].iter().copied());
        }
        Ok(selected.into_iter().collect())
    }
}

pub(super) fn neighbor_images(
    c: &Configuration,
    absorber: usize,
    radius: f64,
) -> Result<Vec<AtomImage>, RmcError> {
    let origin = c.atoms[absorber].position;
    let lattice = c.lattice()?;
    let bounds = lattice
        .as_ref()
        .map(|l| image_bounds(l, radius))
        .transpose()?
        .unwrap_or([0; 3]);
    require(
        bounds
            .iter()
            .map(|&n| (2 * n + 1) as usize)
            .product::<usize>()
            .saturating_mul(c.atoms.len())
            <= 10_000_000,
        "path neighbor search exceeds ten million images",
    )?;
    let mut vertices = Vec::new();
    for (atom, a) in c.atoms.iter().enumerate() {
        let nearest = lattice
            .as_ref()
            .map(|l| {
                l.to_frac(std::array::from_fn(|i| origin[i] - a.position[i]))
                    .map(|v| v.round() as i32)
            })
            .unwrap_or([0; 3]);
        for x in -bounds[0]..=bounds[0] {
            for y in -bounds[1]..=bounds[1] {
                for z in -bounds[2]..=bounds[2] {
                    let image = [nearest[0] + x, nearest[1] + y, nearest[2] + z];
                    if atom == absorber && image == [0; 3] {
                        continue;
                    }
                    let vertex = AtomImage { atom, image };
                    if distance(vertex.position(c)?, origin) <= radius + 1e-10 {
                        vertices.push(vertex);
                    }
                }
            }
        }
    }
    Ok(vertices)
}
