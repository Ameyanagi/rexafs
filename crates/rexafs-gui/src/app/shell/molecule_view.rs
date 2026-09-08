//! Molecular rendering with a fixed orthographic scale. Rotation never fits
//! the projected bounding box; only the wheel and explicit reset change zoom.
use super::bond_geometry::{BondMode, contacts, nearest_bonds};
use super::molecular_geometry::{MolecularComponent, complete_molecule};
use super::structure_depth::{DepthFrame, FadeMode};
use super::structure_view::AtomPick;
use crate::{
    app::StudioApp,
    structure::{Cluster, PathGeometry, covalent_radius, cpk_color},
    theme::Theme,
};
use gpui::{
    Bounds, Context, IntoElement, MouseButton, Pixels, Point, Rgba, Styled, Window, canvas, div,
    point, prelude::*, px, size,
};
use rexafs::xafs::structure as core;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum AtomStyle {
    Balls,
    BallStick,
    Wireframe,
    Polyhedra,
}
impl AtomStyle {
    pub const ALL: [Self; 4] = [
        Self::Balls,
        Self::BallStick,
        Self::Wireframe,
        Self::Polyhedra,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Balls => "Balls",
            Self::BallStick => "Ball + stick",
            Self::Wireframe => "Wireframe",
            Self::Polyhedra => "Polyhedron",
        }
    }
}

#[derive(Clone)]
pub(crate) struct SceneAtom {
    pub pos: [f64; 3],
    pub z: u32,
    pub index: Option<usize>,
    pub shell: usize,
    pub absorber: bool,
    pub faded: bool,
    pub label: String,
}
#[derive(Clone, Copy)]
pub(crate) struct PolyhedronOptions {
    pub network: bool,
    pub ligand: Option<u32>,
    pub cutoff: Option<f64>,
    pub opacity: f32,
    pub color: Option<u32>,
    pub edges: bool,
    pub atoms: PolyAtoms,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum PolyAtoms {
    All,
    Centers,
    None,
}
impl Default for PolyhedronOptions {
    fn default() -> Self {
        Self {
            network: true,
            ligand: None,
            cutoff: None,
            opacity: 0.65,
            color: Some(0x70a9ee),
            edges: true,
            atoms: PolyAtoms::Centers,
        }
    }
}
#[derive(Clone)]
pub(crate) struct PolyFace {
    vertices: Vec<[f64; 3]>,
    normal: [f64; 3],
    z: u32,
}
#[derive(Clone, Default)]
pub(crate) struct CrystalContext {
    pub atoms: Vec<SceneAtom>,
    pub edges: Vec<[[f64; 3]; 2]>,
    pub cells: [usize; 3],
    pub truncated: bool,
    pub radius: f64,
    pub molecule: Option<Result<MolecularComponent, String>>,
}
#[derive(Clone, Default)]
pub(crate) struct MoleculeScene {
    pub atoms: Vec<SceneAtom>,
    pub bonds: Vec<[usize; 2]>,
    pub all_bonds: Vec<[usize; 2]>,
    pub edges: Vec<[[f64; 3]; 2]>,
    pub faces: Vec<PolyFace>,
    pub poly_atoms: Vec<usize>,
    pub poly_centers: Vec<usize>,
    pub poly_count: usize,
    pub poly_options: PolyhedronOptions,
    pub message: Option<String>,
    pub labels: bool,
    pub route: Vec<[f64; 3]>,
    pub radius: f64,
    pub extent: f64,
    pub center: [f64; 3],
}
#[derive(Clone, Copy)]
pub(crate) struct ViewCamera {
    pub az: f64,
    pub el: f64,
    pub zoom: f64,
}
impl Default for ViewCamera {
    fn default() -> Self {
        Self {
            az: -0.6,
            el: 0.45,
            zoom: 1.,
        }
    }
}
impl ViewCamera {
    fn orbit_drag(&mut self, dx: f32, dy: f32) {
        // Grab the structure: its front surface follows the pointer on screen.
        self.az -= f64::from(dx) * 0.008;
        self.el = (self.el + f64::from(dy) * 0.008).clamp(-1.55, 1.55);
    }

    pub(crate) fn zoom_by(&mut self, log_delta: f64) {
        if log_delta.is_finite() {
            self.zoom = (self.zoom * log_delta.exp()).clamp(0.25, 5.);
        }
    }

    fn scroll_zoom(&mut self, delta: gpui::ScrollDelta) {
        let log_delta = match delta {
            gpui::ScrollDelta::Pixels(p) => -f32::from(p.y) as f64 * 0.0015,
            gpui::ScrollDelta::Lines(p) => -p.y as f64 * 0.12,
        };
        // High-resolution wheels and synthetic scroll events can report large
        // deltas. One event must not jump from a fitted view to maximum zoom.
        self.zoom_by(log_delta.clamp(-0.15, 0.15));
    }

    pub(crate) fn rotate(self, p: [f64; 3]) -> [f64; 3] {
        let (sa, ca) = self.az.sin_cos();
        let (se, ce) = self.el.sin_cos();
        let x = ca * p[0] - sa * p[1];
        let y = sa * p[0] + ca * p[1];
        [x, ce * p[2] - se * y, se * p[2] + ce * y]
    }
    fn project(self, p: [f64; 3], bounds: Bounds<Pixels>, extent: f64) -> [f32; 3] {
        self.projector(bounds, extent)(p)
    }
    fn projector(self, bounds: Bounds<Pixels>, extent: f64) -> impl Fn([f64; 3]) -> [f32; 3] {
        let scale = self.scale(bounds, extent);
        let (sa, ca) = self.az.sin_cos();
        let (se, ce) = self.el.sin_cos();
        let origin = [f32::from(bounds.center().x), f32::from(bounds.center().y)];
        move |p| {
            let x = ca * p[0] - sa * p[1];
            let y = sa * p[0] + ca * p[1];
            [
                origin[0] + x as f32 * scale,
                origin[1] - (ce * p[2] - se * y) as f32 * scale,
                (se * p[2] + ce * y) as f32,
            ]
        }
    }
    fn scale(self, b: Bounds<Pixels>, extent: f64) -> f32 {
        f32::from(b.size.width.min(b.size.height)) * 0.42 * self.zoom as f32 / extent.max(1.) as f32
    }
}
/// Screen-only separation for coincident traversals; unique legs stay straight.
fn route_lane_offset(route: &[[f64; 3]], index: usize) -> f32 {
    let pair = &route[index..=index + 1];
    let same = |a: [f64; 3], b: [f64; 3]| norm(sub(a, b)) < 1e-6;
    let coincident = route
        .windows(2)
        .filter(|edge| {
            same(edge[0], pair[0]) && same(edge[1], pair[1])
                || same(edge[0], pair[1]) && same(edge[1], pair[0])
        })
        .count();
    if coincident < 2 {
        return 0.;
    }
    let repeated = route
        .windows(2)
        .take(index)
        .filter(|edge| same(edge[0], pair[0]) && same(edge[1], pair[1]))
        .count();
    4. + repeated as f32 * 6.
}

/// The lane bows only between atoms. Its endpoints remain at their projected
/// centers, and arrowheads follow the local tangent of the same curve.
struct PathStroke {
    start: [f32; 3],
    end: [f32; 3],
    direction: [f32; 2],
    length: f32,
    offset: f32,
}
impl PathStroke {
    fn new(start: [f32; 3], end: [f32; 3], offset: f32) -> Option<Self> {
        let dx = end[0] - start[0];
        let dy = end[1] - start[1];
        let length = dx.hypot(dy);
        if length < 1. {
            return None;
        }
        Some(Self {
            start,
            end,
            direction: [dx / length, dy / length],
            length,
            offset,
        })
    }

    fn point(&self, t: f32) -> [f32; 3] {
        if t <= 0. {
            return self.start;
        }
        if t >= 1. {
            return self.end;
        }
        let mut p = std::array::from_fn(|a| self.start[a] + (self.end[a] - self.start[a]) * t);
        let bow = self.offset * 4. * t * (1. - t);
        p[0] -= self.direction[1] * bow;
        p[1] += self.direction[0] * bow;
        p
    }

    fn tangent(&self, t: f32) -> [f32; 2] {
        let slope = self.offset * 4. * (1. - 2. * t);
        let [ux, uy] = self.direction;
        let x = ux * self.length - uy * slope;
        let y = uy * self.length + ux * slope;
        let length = x.hypot(y);
        [x / length, y / length]
    }

    fn points(&self, from: f32, to: f32) -> Vec<[f32; 3]> {
        let steps = if self.offset == 0. { 1 } else { 16 };
        (0..=steps)
            .map(|i| self.point(from + (to - from) * i as f32 / steps as f32))
            .collect()
    }
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|i| a[i] - b[i])
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    (0..3).map(|i| a[i] * b[i]).sum()
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn norm(a: [f64; 3]) -> f64 {
    dot(a, a).sqrt()
}

/// Whole periodic unit cells covering the calculated sphere, with at least three
/// cells along each axis for context. The FEFF cluster itself is never truncated.
pub(crate) fn crystal_context(s: &core::Structure, c: &core::Cluster) -> CrystalContext {
    let origin = s.lattice.to_cart(s.sites[c.absorber_site].frac);
    let mut lo = [0; 3];
    let mut hi = [0; 3];
    for atom in &c.atoms {
        for k in 0..3 {
            lo[k] = lo[k].min(atom.image[k]);
            hi[k] = hi[k].max(atom.image[k]);
        }
    }
    for k in 0..3 {
        lo[k] = lo[k].min(-1);
        hi[k] = hi[k].max(1);
    }
    let mut out = CrystalContext {
        radius: c.radius,
        molecule: Some(complete_molecule(s, c.absorber_site)),
        cells: std::array::from_fn(|i| (hi[i] - lo[i] + 1) as usize),
        ..Default::default()
    };
    let cart = |f| sub(s.lattice.to_cart(f), origin);
    'cells: for a in lo[0]..=hi[0] {
        for b in lo[1]..=hi[1] {
            for z in lo[2]..=hi[2] {
                for site in &s.sites {
                    let Some(el) = site.element() else {
                        continue;
                    };
                    if el.z == 1 {
                        continue;
                    }
                    let p = cart([
                        site.frac[0] + a as f64,
                        site.frac[1] + b as f64,
                        site.frac[2] + z as f64,
                    ]);
                    if norm(p) <= c.radius + 1e-6 {
                        continue;
                    }
                    if out.atoms.len() >= 12000 {
                        out.truncated = true;
                        break 'cells;
                    }
                    out.atoms.push(SceneAtom {
                        pos: p,
                        z: el.z as u32,
                        index: None,
                        shell: 0,
                        absorber: false,
                        faded: true,
                        label: site.label.clone(),
                    });
                }
            }
        }
    }
    // A lattice grid along each axis, drawn once per edge.
    for axis in 0..3 {
        let j = (axis + 1) % 3;
        let k = (axis + 2) % 3;
        for u in lo[j]..=hi[j] + 1 {
            for v in lo[k]..=hi[k] + 1 {
                let mut p = [0.; 3];
                p[j] = u as f64;
                p[k] = v as f64;
                p[axis] = lo[axis] as f64;
                let mut q = p;
                q[axis] = (hi[axis] + 1) as f64;
                out.edges.push([cart(p), cart(q)]);
            }
        }
    }
    out
}

/// Convex coordination faces, including coplanar polygons as one face.
fn hull_faces(points: &[[f64; 3]]) -> Vec<Vec<[f64; 3]>> {
    let mut faces: Vec<Vec<usize>> = Vec::new();
    for i in 0..points.len() {
        for j in i + 1..points.len() {
            for k in j + 1..points.len() {
                let normal = cross(sub(points[j], points[i]), sub(points[k], points[i]));
                if norm(normal) < 1e-8 {
                    continue;
                }
                let ds: Vec<_> = points
                    .iter()
                    .map(|p| dot(sub(*p, points[i]), normal) / norm(normal))
                    .collect();
                if ds.iter().any(|d| *d > 1e-5) && ds.iter().any(|d| *d < -1e-5) {
                    continue;
                }
                let ids: Vec<_> = ds
                    .iter()
                    .enumerate()
                    .filter(|(_, d)| d.abs() < 1e-5)
                    .map(|(n, _)| n)
                    .collect();
                if !faces.contains(&ids) {
                    faces.push(ids);
                }
            }
        }
    }
    faces
        .into_iter()
        .map(|mut ids| {
            let center = std::array::from_fn(|i| {
                ids.iter().map(|&n| points[n][i]).sum::<f64>() / ids.len() as f64
            });
            let u = sub(points[ids[0]], center);
            let normal = cross(
                sub(points[ids[1]], points[ids[0]]),
                sub(points[ids[2]], points[ids[0]]),
            );
            let v = cross(normal, u);
            ids.sort_by(|&a, &b| {
                let pa = sub(points[a], center);
                let pb = sub(points[b], center);
                (dot(pa, v) / norm(v))
                    .atan2(dot(pa, u) / norm(u))
                    .total_cmp(&(dot(pb, v) / norm(v)).atan2(dot(pb, u) / norm(u)))
            });
            ids.into_iter().map(|i| points[i]).collect()
        })
        .collect()
}

impl MoleculeScene {
    fn on_route(&self, position: [f64; 3]) -> bool {
        self.route
            .iter()
            .any(|point| norm(sub(*point, position)) < 1e-4)
    }

    pub fn new(
        cluster: &Cluster,
        context: Option<&CrystalContext>,
        radius: f64,
        style: AtomStyle,
        path: Option<&PathGeometry>,
        picked: Option<usize>,
        poly: PolyhedronOptions,
    ) -> Self {
        let mut scene = Self {
            radius,
            extent: radius.max(1.),
            ..Default::default()
        };
        scene.atoms = cluster
            .atoms
            .iter()
            .enumerate()
            .map(|(i, a)| SceneAtom {
                pos: a.pos,
                z: a.z,
                index: Some(i),
                shell: a.shell,
                absorber: a.ipot == 0,
                faded: false,
                label: a.tag.clone(),
            })
            .collect();
        if style != AtomStyle::Balls {
            scene.all_bonds = contacts(&scene.atoms);
            scene.bonds = nearest_bonds(&scene.atoms, &scene.all_bonds);
        }
        if let Some(context) = context {
            scene.atoms.extend(context.atoms.clone());
            scene.edges = context.edges.clone();
        }
        if style == AtomStyle::Polyhedra {
            scene.build_polyhedra(picked.unwrap_or(0), poly);
        }
        for a in &scene.atoms {
            scene.extent = scene.extent.max(norm(a.pos));
        }
        if let Some(p) = path {
            scene.route = p.polyline();
        }
        scene
    }
    pub fn apply_bond_mode(&mut self, mode: BondMode) {
        match mode {
            BondMode::Auto => (),
            BondMode::Absorber => self
                .bonds
                .retain(|&[a, b]| self.atoms[a].absorber || self.atoms[b].absorber),
            BondMode::AllContacts => self.bonds = self.all_bonds.clone(),
            BondMode::None => self.bonds.clear(),
        }
    }
    fn build_polyhedra(&mut self, center: usize, options: PolyhedronOptions) {
        self.poly_options = options;
        let Some(seed) = self.atoms.get(center) else {
            return;
        };
        let z = seed.z;
        let centers = self
            .atoms
            .iter()
            .enumerate()
            .filter(|(i, a)| !a.faded && a.z == z && (options.network || *i == center))
            .map(|(i, _)| i)
            .collect::<Vec<_>>();
        let mut selected = std::collections::BTreeSet::new();
        let mut bonds = std::collections::BTreeSet::new();
        let mut oversized = 0;
        for center in centers {
            let origin = self.atoms[center].pos;
            // Auto follows the nearest unlike-element coordination shell when
            // one lies within a plausible bond range, otherwise the metal shell.
            let unlike = self.atoms.iter().any(|a| {
                a.z != z && a.z != 1 && {
                    let d = norm(sub(a.pos, origin));
                    d > 0.4 && d <= 1.4 * (covalent_radius(a.z) + covalent_radius(z)) as f64
                }
            });
            let candidates = self
                .atoms
                .iter()
                .enumerate()
                .filter(|(_, a)| {
                    options
                        .ligand
                        .map(|el| a.z == el)
                        .unwrap_or(a.z != 1 && (!unlike || a.z != z))
                })
                .map(|(i, a)| (i, norm(sub(a.pos, origin))))
                .filter(|(_, d)| *d > 0.4)
                .collect::<Vec<_>>();
            let nearest = candidates
                .iter()
                .map(|(_, d)| *d)
                .fold(f64::INFINITY, f64::min);
            let cutoff = options.cutoff.unwrap_or(nearest * 1.25);
            let neighbors = candidates
                .into_iter()
                .filter(|(_, d)| *d <= cutoff + 1e-6)
                .map(|(i, _)| i)
                .collect::<Vec<_>>();
            if neighbors.len() > 24 {
                oversized += 1;
                continue;
            }
            if neighbors.len() < 3 {
                continue;
            }
            let points = neighbors
                .iter()
                .map(|&i| self.atoms[i].pos)
                .collect::<Vec<_>>();
            let faces = hull_faces(&points);
            if faces.is_empty() {
                continue;
            }
            self.poly_count += 1;
            self.poly_centers.push(center);
            selected.insert(center);
            for &n in &neighbors {
                selected.insert(n);
                bonds.insert([center.min(n), center.max(n)]);
            }
            for vertices in faces {
                let mut normal =
                    cross(sub(vertices[1], vertices[0]), sub(vertices[2], vertices[0]));
                let midpoint = std::array::from_fn(|i| {
                    vertices.iter().map(|p| p[i]).sum::<f64>() / vertices.len() as f64
                });
                if dot(normal, sub(midpoint, origin)) < 0. {
                    normal = normal.map(|v| -v);
                }
                let length = norm(normal);
                if length <= 1e-8 {
                    continue;
                }
                self.faces.push(PolyFace {
                    vertices,
                    normal: normal.map(|v| v / length),
                    z,
                });
            }
        }
        self.poly_atoms = selected.into_iter().collect();
        self.bonds = if options.atoms == PolyAtoms::All {
            bonds.into_iter().collect()
        } else {
            Vec::new()
        };
        if self.poly_count == 0 {
            self.message = Some(
                "No coordination faces. Choose another center, neighbour element or bond limit."
                    .into(),
            );
        } else if oversized > 0 {
            self.message = Some(format!(
                "{oversized} centers exceed 24 neighbours; reduce the bond limit to display them."
            ));
        }
    }

    pub fn molecule(
        component: &MolecularComponent,
        cluster: &Cluster,
        radius: f64,
        hydrogens: bool,
        style: AtomStyle,
    ) -> Self {
        let mut scene = Self {
            radius,
            ..Default::default()
        };
        for axis in 0..3 {
            let lo = component
                .atoms
                .iter()
                .map(|a| a.pos[axis])
                .fold(f64::INFINITY, f64::min);
            let hi = component
                .atoms
                .iter()
                .map(|a| a.pos[axis])
                .fold(f64::NEG_INFINITY, f64::max);
            scene.center[axis] = (lo + hi) * 0.5;
        }
        scene.extent = component
            .atoms
            .iter()
            .map(|a| norm(sub(a.pos, scene.center)))
            .fold(1.5, f64::max);
        let mut indices = std::collections::BTreeMap::new();
        for (i, a) in component.atoms.iter().enumerate() {
            if a.z == 1 && !hydrogens {
                continue;
            }
            let index = cluster
                .atoms
                .iter()
                .position(|b| b.z == a.z && norm(sub(a.pos, b.pos)) < 1e-4);
            indices.insert(i, scene.atoms.len());
            scene.atoms.push(SceneAtom {
                pos: a.pos,
                z: a.z,
                index,
                shell: index.map(|i| cluster.atoms[i].shell).unwrap_or(0),
                absorber: norm(a.pos) < 1e-6,
                faded: false,
                label: a.label.clone(),
            });
        }
        if style != AtomStyle::Balls {
            for [a, b] in &component.bonds {
                if let (Some(a), Some(b)) = (indices.get(a), indices.get(b)) {
                    scene.bonds.push([*a, *b]);
                }
            }
        }
        scene.extent = scene.extent.max(1.5);
        // Complete-molecule connectivity is already reconstructed across CIF
        // boundaries. Preserve its C–C, C–H etc. bonds in Auto mode.
        scene.all_bonds = scene.bonds.clone();
        scene
    }
}

fn alpha(mut color: Rgba, a: f32) -> Rgba {
    color.a = a;
    color
}
fn tint(c: Rgba, light: f32) -> Rgba {
    Rgba {
        r: (c.r * light).min(1.),
        g: (c.g * light).min(1.),
        b: (c.b * light).min(1.),
        ..c
    }
}
/// Blend toward the canvas, keeping element hue and explicit alpha separate.
/// Applied after lighting so rear silhouettes recede in either theme.
pub(super) fn depth_cue_color(color: Rgba, backdrop: Rgba, amount: f32) -> Rgba {
    Rgba {
        r: color.r * (1. - amount) + backdrop.r * amount,
        g: color.g * (1. - amount) + backdrop.g * amount,
        b: color.b * (1. - amount) + backdrop.b * amount,
        ..color
    }
}
fn line(window: &mut Window, pts: &[[f32; 3]], color: Rgba, width: f32, closed: bool) {
    if pts.is_empty() {
        return;
    }
    let mut b = gpui::PathBuilder::stroke(px(width));
    b.move_to(point(px(pts[0][0]), px(pts[0][1])));
    for p in &pts[1..] {
        b.line_to(point(px(p[0]), px(p[1])));
    }
    if closed {
        b.close();
    }
    if let Ok(p) = b.build() {
        window.paint_path(p, color);
    }
}
fn disk(window: &mut Window, p: [f32; 3], r: f32, c: Rgba) {
    window.paint_quad(gpui::quad(
        Bounds::new(
            point(px(p[0] - r), px(p[1] - r)),
            size(px(2. * r), px(2. * r)),
        ),
        gpui::Corners::all(px(r)),
        c,
        gpui::Edges::all(px(0.)),
        c,
        gpui::BorderStyle::Solid,
    ));
}

fn atom_in_style(scene: &MoleculeScene, style: AtomStyle, index: usize) -> bool {
    style != AtomStyle::Polyhedra
        || match scene.poly_options.atoms {
            PolyAtoms::None => false,
            PolyAtoms::Centers => scene.poly_centers.contains(&index),
            PolyAtoms::All => true,
        }
}

fn atom_radius(z: u32, style: AtomStyle, scale: f32) -> f32 {
    if style == AtomStyle::Wireframe {
        2.
    } else {
        (covalent_radius(z)
            * scale
            * if style == AtomStyle::Balls {
                0.62
            } else {
                0.31
            })
        .clamp(3., 24.)
    }
}

// Non-overlapping lighting bands preserve the same spherical shading at every
// alpha. Cache tessellation once; frames only position the triangles.
fn sphere_lighting() -> &'static [(gpui::Path<Pixels>, f32)] {
    static LAYERS: std::sync::OnceLock<Vec<(gpui::Path<Pixels>, f32)>> = std::sync::OnceLock::new();
    LAYERS.get_or_init(|| {
        let mut circles = vec![(0., 0., 24., 0.48)];
        for layer in 0..12 {
            let f = layer as f32 / 12.;
            circles.push((
                -24. * 0.23 * f,
                -24. * 0.26 * f,
                24. * (0.94 - 0.65 * f),
                0.55 + 0.85 * f,
            ));
        }
        let circle = |path: &mut gpui::PathBuilder, (x, y, r, _), sweep| {
            path.move_to(point(px(x + r), px(y)));
            path.arc_to(
                point(px(r), px(r)),
                px(0.),
                false,
                sweep,
                point(px(x - r), px(y)),
            );
            path.arc_to(
                point(px(r), px(r)),
                px(0.),
                false,
                sweep,
                point(px(x + r), px(y)),
            );
            path.close();
        };
        circles
            .iter()
            .enumerate()
            .map(|(i, &outer)| {
                let mut path = gpui::PathBuilder::fill();
                circle(&mut path, outer, true);
                if let Some(&inner) = circles.get(i + 1) {
                    circle(&mut path, inner, false);
                }
                (path.build().expect("valid sphere lighting bands"), outer.3)
            })
            .collect()
    })
}

fn shaded_ball(w: &mut Window, p: [f32; 3], r: f32, color: Rgba, cue: impl Fn(Rgba) -> Rgba) {
    for (template, brightness) in sphere_lighting() {
        let mut path = template.clone();
        let scale = r / 24.;
        let transform = |q: Point<Pixels>| {
            point(
                px(p[0] + f32::from(q.x) * scale),
                px(p[1] + f32::from(q.y) * scale),
            )
        };
        for vertex in &mut path.vertices {
            vertex.xy_position = transform(vertex.xy_position);
        }
        path.bounds = Bounds::new(
            transform(path.bounds.origin),
            path.bounds.size.map(|v| v * scale),
        );
        w.paint_path(path, cue(tint(color, *brightness)));
    }
}

#[allow(clippy::too_many_arguments)]
fn depth_line(
    w: &mut Window,
    edge: [[f64; 3]; 2],
    depth: DepthFrame,
    project: &impl Fn([f64; 3]) -> [f32; 3],
    color: Rgba,
    width: f32,
    backdrop: Rgba,
) {
    for (part, inside) in depth.segments(edge) {
        let steps = if depth.options.fade != FadeMode::Off {
            8
        } else if depth.options.depth_cue {
            2
        } else {
            1
        };
        for n in 0..steps {
            let at = |t: f64| std::array::from_fn(|a| part[0][a] + (part[1][a] - part[0][a]) * t);
            let midpoint = at((n as f64 + 0.5) / steps as f64);
            let opacity = depth.alpha(midpoint, inside) * color.a;
            if opacity > 0.002 {
                line(
                    w,
                    &[
                        project(at(n as f64 / steps as f64)),
                        project(at((n + 1) as f64 / steps as f64)),
                    ],
                    alpha(
                        depth_cue_color(color, backdrop, depth.fog_amount(midpoint)),
                        opacity,
                    ),
                    width,
                    false,
                );
            }
        }
    }
}

impl StudioApp {
    pub(crate) fn find_structure_absorber(&mut self, cx: &mut Context<Self>) {
        let index = self
            .structure
            .scene
            .as_ref()
            .and_then(|s| s.atoms.iter().find(|a| a.absorber))
            .and_then(|a| a.index);
        if let Some(atom) = index {
            self.structure.pick = Some(AtomPick { atom });
            self.structure.highlight_absorber = true;
            self.structure.absorber_label = true;
            self.structure.depth.options.offset = 0.;
            self.structure.camera.zoom = 1.;
            self.rebuild_structure_plot(cx);
            cx.notify();
        }
    }
    pub(crate) fn molecule_canvas(
        &mut self,
        cx: &mut Context<Self>,
    ) -> gpui::Entity<MoleculeViewport> {
        let state = MoleculePaintState {
            scene: self.structure.scene.clone().unwrap_or_default(),
            camera: self.structure.camera,
            style: self.structure.atom_style,
            shading: self.structure.shading,
            shells: self.structure.color_by_shell,
            step: self.structure.path_leg,
            highlight_absorber: self.structure.highlight_absorber,
            absorber_label: self.structure.absorber_label,
            depth: self.structure.depth.options,
            picked: self.structure.pick.as_ref().map(|p| p.atom),
            theme: self.theme,
        };
        if let Some(view) = self.structure.viewport.clone() {
            view.update(cx, |view, cx| {
                if !std::sync::Arc::ptr_eq(&view.state.scene, &state.scene) {
                    view.drag = None;
                }
                view.state = state;
                cx.notify();
            });
            view
        } else {
            let owner = cx.entity().downgrade();
            let view = cx.new(|_| MoleculeViewport {
                owner,
                state,
                drag: None,
                scroll_generation: 0,
            });
            self.structure.viewport = Some(view.clone());
            view
        }
    }
    fn pick_molecule_atom(&mut self, position: Point<Pixels>, cx: &mut Context<Self>) {
        let (Some(scene), Some(bounds)) = (&self.structure.scene, self.structure.view_bounds)
        else {
            return;
        };
        let depth = DepthFrame::new(
            self.structure.depth.options,
            scene,
            self.structure.camera,
            self.structure.pick.as_ref().map(|p| p.atom),
        );
        let mut hits: Vec<_> = scene
            .atoms
            .iter()
            .enumerate()
            .filter_map(|(index, a)| {
                let highlighted = a.absorber || scene.on_route(a.pos);
                let path_focus = depth.options.path_focus && scene.route.len() > 1;
                if !depth.pickable(a.pos, highlighted)
                    || path_focus && !highlighted
                    || !(atom_in_style(scene, self.structure.atom_style, index)
                        || path_focus && highlighted)
                {
                    return None;
                }
                let i = a.index?;
                let p =
                    self.structure
                        .camera
                        .project(sub(a.pos, scene.center), bounds, scene.extent);
                let d = (p[0] - f32::from(position.x)).hypot(p[1] - f32::from(position.y));
                let radius = atom_radius(
                    a.z,
                    self.structure.atom_style,
                    self.structure.camera.scale(bounds, scene.extent),
                );
                (d <= radius.max(6.)).then_some((i, p[2]))
            })
            .collect();
        hits.sort_by(|a, b| b.1.total_cmp(&a.1));
        // Empty space must not silently move an active slice back to the absorber.
        if hits.is_empty() && depth.options.active() {
            return;
        }
        self.structure.pick = hits.first().map(|(atom, _)| AtomPick { atom: *atom });
        self.structure.absorber_label = false;
        if self.structure.atom_style == AtomStyle::Polyhedra {
            self.rebuild_structure_plot(cx);
        }
        cx.notify();
    }
}

#[derive(Clone)]
struct MoleculePaintState {
    scene: std::sync::Arc<MoleculeScene>,
    camera: ViewCamera,
    style: AtomStyle,
    shading: bool,
    shells: bool,
    step: Option<usize>,
    highlight_absorber: bool,
    absorber_label: bool,
    depth: super::structure_depth::DepthOptions,
    picked: Option<usize>,
    theme: Theme,
}

/// Camera events invalidate only this view, avoiding re-layout of the group,
/// path and inspector controls for every high-resolution scroll event.
pub(crate) struct MoleculeViewport {
    owner: gpui::WeakEntity<StudioApp>,
    state: MoleculePaintState,
    drag: Option<(Point<Pixels>, Point<Pixels>, bool)>,
    scroll_generation: u64,
}
impl MoleculeViewport {
    fn sync_camera(&self, cx: &mut Context<Self>) {
        self.owner
            .update(cx, |app, _| app.structure.camera = self.state.camera)
            .ok();
    }

    fn refresh_zoom_readout_after_scroll(&mut self, cx: &mut Context<Self>) {
        self.scroll_generation += 1;
        let generation = self.scroll_generation;
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(120))
                .await;
            this.update(cx, |this, cx| {
                if generation == this.scroll_generation {
                    this.owner.update(cx, |_, cx| cx.notify()).ok();
                }
            })
            .ok();
        })
        .detach();
    }
}
impl gpui::Render for MoleculeViewport {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state = self.state.clone();
        let depth = DepthFrame::new(state.depth, &state.scene, state.camera, state.picked);
        let owner = self.owner.clone();
        let view = canvas(
            move |bounds, _, cx| {
                owner
                    .update(cx, |app, _| app.structure.view_bounds = Some(bounds))
                    .ok();
            },
            move |bounds, _, window, cx| {
                paint_scene(
                    &state.scene,
                    state.camera,
                    state.style,
                    state.shading,
                    state.shells,
                    state.step,
                    depth,
                    state.highlight_absorber,
                    state.absorber_label,
                    state.theme,
                    bounds,
                    window,
                    cx,
                )
            },
        )
        .size_full();
        div()
            .id("molecular-canvas")
            .size_full()
            .cursor_grab()
            .child(view)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, ev: &gpui::MouseDownEvent, _, cx| {
                    this.drag = Some((ev.position, ev.position, false));
                    cx.stop_propagation();
                }),
            )
            .on_mouse_move(cx.listener(|this, ev: &gpui::MouseMoveEvent, _, cx| {
                if ev.pressed_button != Some(MouseButton::Left) {
                    return;
                }
                if let Some((start, last, moved)) = this.drag {
                    let dx = f32::from(ev.position.x - last.x);
                    let dy = f32::from(ev.position.y - last.y);
                    let distance = f32::from(ev.position.x - start.x)
                        .hypot(f32::from(ev.position.y - start.y));
                    if moved || distance > 4. {
                        this.state.camera.orbit_drag(dx, dy);
                        this.drag = Some((start, ev.position, true));
                        this.sync_camera(cx);
                        cx.notify();
                    }
                }
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, ev: &gpui::MouseUpEvent, _, cx| {
                    if let Some((_, _, moved)) = this.drag.take() {
                        this.owner
                            .update(cx, |app, cx| {
                                if moved {
                                    cx.notify();
                                } else {
                                    app.pick_molecule_atom(ev.position, cx);
                                }
                            })
                            .ok();
                    }
                    cx.stop_propagation();
                }),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    if this.drag.take().is_some() {
                        this.owner.update(cx, |_, cx| cx.notify()).ok();
                    }
                }),
            )
            .on_scroll_wheel(cx.listener(|this, ev: &gpui::ScrollWheelEvent, _, cx| {
                cx.stop_propagation();
                if this.drag.is_some() {
                    return;
                }
                let before = this.state.camera.zoom;
                this.state.camera.scroll_zoom(ev.delta);
                if this.state.camera.zoom != before {
                    this.sync_camera(cx);
                    this.refresh_zoom_readout_after_scroll(cx);
                    cx.notify();
                }
            }))
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_scene(
    scene: &MoleculeScene,
    camera: ViewCamera,
    style: AtomStyle,
    shading: bool,
    shells: bool,
    leg: Option<usize>,
    depth: DepthFrame,
    highlight_absorber: bool,
    absorber_label: bool,
    t: Theme,
    b: Bounds<Pixels>,
    w: &mut Window,
    cx: &mut gpui::App,
) {
    let path_focus = depth.options.path_focus && scene.route.len() > 1;
    let atom_alpha = |atom: &SceneAtom| {
        let highlighted = atom.absorber || scene.on_route(atom.pos);
        depth.display_atom_alpha(atom.pos, highlighted)
            * if path_focus && !highlighted { 0.07 } else { 1. }
    };
    let projection = camera.projector(b, scene.extent);
    let project = |p| projection(sub(p, scene.center));
    let scale = camera.scale(b, scene.extent);
    let context_alpha = if path_focus { 0.2 } else { 1. };
    for edge in &scene.edges {
        depth_line(
            w,
            *edge,
            depth,
            &project,
            alpha(t.text_muted, 0.12 * context_alpha),
            0.65,
            t.raised,
        );
    }
    // Radius guides are a true sphere cut through the absorber, not a fitted box.
    if !scene.edges.is_empty() {
        for axis in 0..3 {
            let ring: Vec<_> = (0..=96)
                .map(|n| {
                    let ang = n as f64 * std::f64::consts::TAU / 96.;
                    let mut p = [0.; 3];
                    p[(axis + 1) % 3] = scene.radius * ang.cos();
                    p[(axis + 2) % 3] = scene.radius * ang.sin();
                    p
                })
                .collect();
            for points in ring.windows(2) {
                depth_line(
                    w,
                    [points[0], points[1]],
                    depth,
                    &project,
                    alpha(t.accent, 0.28 * context_alpha),
                    1.,
                    t.raised,
                );
            }
        }
    }
    enum Primitive {
        Atom(usize),
        Bond(usize),
        Face(usize, Vec<[f64; 3]>, bool),
    }
    let mut draw = Vec::new();
    for (i, a) in scene.atoms.iter().enumerate() {
        if (atom_in_style(scene, style, i) || path_focus && scene.on_route(a.pos))
            && atom_alpha(a) > 0.002
        {
            // Highlight changes the material color, never the geometric depth.
            draw.push((project(a.pos)[2], Primitive::Atom(i)));
        }
    }
    for (i, ids) in scene.bonds.iter().enumerate().filter(|_| !path_focus) {
        draw.push((
            (project(scene.atoms[ids[0]].pos)[2] + project(scene.atoms[ids[1]].pos)[2]) * 0.5,
            Primitive::Bond(i),
        ));
    }
    for (i, f) in scene.faces.iter().enumerate() {
        for (vertices, inside) in depth.polygons(&f.vertices) {
            draw.push((
                vertices.iter().map(|p| project(*p)[2]).sum::<f32>() / vertices.len() as f32,
                Primitive::Face(i, vertices, inside),
            ));
        }
    }
    draw.sort_by(|a, b| a.0.total_cmp(&b.0));
    for (_, p) in draw {
        match p {
            Primitive::Bond(i) => {
                let ids = scene.bonds[i];
                let centers = ids.map(|i| scene.atoms[i].pos);
                let projected = centers.map(project);
                let length =
                    (projected[1][0] - projected[0][0]).hypot(projected[1][1] - projected[0][1]);
                let radii = ids.map(|i| {
                    if depth.contains(scene.atoms[i].pos) && atom_in_style(scene, style, i) {
                        atom_radius(scene.atoms[i].z, style, scale)
                    } else {
                        0.
                    }
                });
                if length <= radii[0] + radii[1] {
                    continue;
                }
                // Trim to the projected sphere boundaries, so sticks cannot
                // stripe their own endpoint balls when depth sorting interleaves them.
                let at = |t: f64| {
                    std::array::from_fn(|a| centers[0][a] + (centers[1][a] - centers[0][a]) * t)
                };
                let pts = [
                    at((radii[0] / length) as f64),
                    at(1. - (radii[1] / length) as f64),
                ];
                let midpoint =
                    at(0.5_f64.clamp((radii[0] / length) as f64, 1. - (radii[1] / length) as f64));
                let width = if style == AtomStyle::Wireframe {
                    1.0
                } else {
                    (scale * 0.11).clamp(2.5, 7.)
                };
                for n in 0..2 {
                    let atom = &scene.atoms[ids[n]];
                    let color = alpha(
                        gpui::rgb(cpk_color(atom.z)),
                        if atom.faded { 0.14 } else { 1. },
                    );
                    let edge = [pts[n], midpoint];
                    if depth.options.active() || color.a < 0.99 {
                        depth_line(w, edge, depth, &project, color, width * 0.8, t.raised);
                    } else {
                        depth_line(w, edge, depth, &project, tint(color, 0.5), width, t.raised);
                        depth_line(w, edge, depth, &project, color, width * 0.67, t.raised);
                        if shading && style != AtomStyle::Wireframe {
                            depth_line(
                                w,
                                edge,
                                depth,
                                &project,
                                tint(color, 1.35),
                                width * 0.22,
                                t.raised,
                            );
                        }
                    }
                }
            }
            Primitive::Face(i, vertices, inside) => {
                let face = &scene.faces[i];
                let center: [f64; 3] = std::array::from_fn(|axis| {
                    vertices.iter().map(|p| p[axis]).sum::<f64>() / vertices.len() as f64
                });
                let opacity = depth.alpha(center, inside) * if path_focus { 0.06 } else { 1. };
                if opacity < 0.002 {
                    continue;
                }
                let pts: Vec<_> = vertices.iter().copied().map(project).collect();
                let mut path = gpui::PathBuilder::fill();
                path.move_to(point(px(pts[0][0]), px(pts[0][1])));
                for p in &pts[1..] {
                    path.line_to(point(px(p[0]), px(p[1])));
                }
                path.close();
                let normal = camera.rotate(face.normal);
                let light = if shading {
                    0.48 + 0.52 * dot(normal, [-0.35, 0.45, 0.822]).abs() as f32
                } else {
                    0.85
                };
                let base = gpui::rgb(
                    scene
                        .poly_options
                        .color
                        .unwrap_or_else(|| cpk_color(face.z)),
                );
                let cue = |color| depth_cue_color(color, t.raised, depth.fog_amount(center));
                let color = cue(tint(base, light));
                if let Ok(path) = path.build() {
                    w.paint_path(path, alpha(color, scene.poly_options.opacity * opacity));
                }
                if scene.poly_options.edges {
                    line(
                        w,
                        &pts,
                        alpha(cue(tint(base, 0.3)), 0.85 * opacity),
                        1.25,
                        true,
                    );
                }
            }
            Primitive::Atom(i) => {
                if style == AtomStyle::Polyhedra
                    && !(path_focus && scene.on_route(scene.atoms[i].pos))
                {
                    match scene.poly_options.atoms {
                        PolyAtoms::None => continue,
                        PolyAtoms::Centers if !scene.poly_centers.contains(&i) => continue,
                        _ => (),
                    }
                }
                let a = &scene.atoms[i];
                let p = project(a.pos);
                let outside = a.faded
                    || (style == AtomStyle::Polyhedra
                        && scene.poly_count > 0
                        && !scene.poly_atoms.contains(&i));
                let mut color = if a.absorber && highlight_absorber {
                    gpui::rgb(0x67e8f9)
                } else if shells && !outside {
                    crate::plotting::trace_rgba(&t, a.shell % 8)
                } else {
                    gpui::rgb(cpk_color(a.z))
                };
                color.a = (if outside && !(path_focus && scene.on_route(a.pos)) {
                    0.14
                } else {
                    1.
                }) * atom_alpha(a);
                let radius = atom_radius(a.z, style, scale);
                let cue = |color| {
                    depth_cue_color(
                        color,
                        t.raised,
                        if a.absorber {
                            0.
                        } else {
                            depth.fog_amount(a.pos)
                        },
                    )
                };
                if !a.absorber && depth.options.active() && norm(sub(a.pos, depth.origin)) < 1e-6 {
                    disk(w, p, radius + 2., alpha(t.accent, 0.4 * color.a));
                }
                if shading && !outside && style != AtomStyle::Wireframe {
                    shaded_ball(w, p, radius, color, cue);
                } else {
                    disk(w, p, radius, cue(color));
                }
            }
        }
    }
    if scene.labels {
        for (_, atom) in scene.atoms.iter().enumerate().filter(|(i, a)| {
            !a.faded
                && (atom_in_style(scene, style, *i) || path_focus && scene.on_route(a.pos))
                && depth.contains(a.pos)
                && atom_alpha(a) >= 0.2
        }) {
            let p = project(atom.pos);
            let text = gpui::SharedString::from(atom.label.clone());
            let run = gpui::TextRun {
                len: text.len(),
                font: w.text_style().font(),
                color: alpha(
                    depth_cue_color(t.text, t.raised, depth.fog_amount(atom.pos)),
                    atom_alpha(atom),
                )
                .into(),
                background_color: None,
                underline: None,
                strikethrough: None,
            };
            let shaped = w.text_system().shape_line(text, px(11.), &[run], None);
            shaped
                .paint(
                    point(px(p[0] + 8.), px(p[1] - 15.)),
                    px(13.),
                    gpui::TextAlign::Left,
                    None,
                    w,
                    cx,
                )
                .ok();
        }
    }
    // Anchor legs at atom centers; only coincident traversals bow apart.
    for (i, pair) in scene.route.windows(2).enumerate() {
        let Some(stroke) = PathStroke::new(
            project(pair[0]),
            project(pair[1]),
            route_lane_offset(&scene.route, i),
        ) else {
            continue;
        };
        let vector = sub(pair[1], pair[0]);
        let length_squared = dot(vector, vector);
        let parameter =
            |pos| (dot(sub(pos, pair[0]), vector) / length_squared).clamp(0., 1.) as f32;
        for (part, inside) in depth.segments([pair[0], pair[1]]) {
            let (from, to) = (parameter(part[0]), parameter(part[1]));
            let active = leg.is_none_or(|n| n == i);
            let midpoint = std::array::from_fn(|a| (part[0][a] + part[1][a]) * 0.5);
            let color = alpha(
                depth_cue_color(
                    crate::plotting::trace_rgba(&t, i % 8),
                    t.raised,
                    depth.fog_amount(midpoint) * 0.6,
                ),
                (if active { 1. } else { 0.16 }) * depth.highlight_alpha(midpoint, inside),
            );
            line(
                w,
                &stroke.points(from, to),
                color,
                if active { 2.5 } else { 1. },
                false,
            );
            // A clipped leg still has one arrow, at the same full-leg position.
            if !(from..to).contains(&0.67) {
                continue;
            }
            let tip = stroke.point(0.67);
            let [ux, uy] = stroke.tangent(0.67);
            let head = 7_f32.min(stroke.length * 0.2);
            line(
                w,
                &[
                    [
                        tip[0] - ux * head - uy * head * 0.55,
                        tip[1] - uy * head + ux * head * 0.55,
                        0.,
                    ],
                    tip,
                    [
                        tip[0] - ux * head + uy * head * 0.55,
                        tip[1] - uy * head - ux * head * 0.55,
                        0.,
                    ],
                ],
                color,
                2.5,
                false,
            );
            if active && inside && color.a > 0.2 {
                let text = gpui::SharedString::from((i + 1).to_string());
                let style = w.text_style();
                let run = gpui::TextRun {
                    len: text.len(),
                    font: style.font(),
                    color: color.into(),
                    background_color: None,
                    underline: None,
                    strikethrough: None,
                };
                let text = w.text_system().shape_line(text, px(11.), &[run], None);
                text.paint(
                    point(px(tip[0] - uy * 10.), px(tip[1] + ux * 10. - 6.)),
                    px(13.),
                    gpui::TextAlign::Left,
                    None,
                    w,
                    cx,
                )
                .ok();
            }
        }
    }
    // Only the explicit "Find absorber" annotation overlays the scene. The
    // highlighted sphere itself keeps its geometric depth and requested opacity.
    if highlight_absorber
        && absorber_label
        && let Some(atom) = scene.atoms.iter().find(|a| a.absorber)
        && depth.contains(atom.pos)
    {
        let p = project(atom.pos);
        if p[0] >= f32::from(b.left())
            && p[0] <= f32::from(b.right())
            && p[1] >= f32::from(b.top())
            && p[1] <= f32::from(b.bottom())
        {
            paint_absorber_marker(w, cx, p, atom, style, scale, t, b);
        }
    }
}

fn paint_absorber_marker(
    w: &mut Window,
    cx: &mut gpui::App,
    p: [f32; 3],
    atom: &SceneAtom,
    style: AtomStyle,
    scale: f32,
    t: Theme,
    b: Bounds<Pixels>,
) {
    let gold = gpui::rgb(0x67e8f9);
    let radius = if style == AtomStyle::Wireframe {
        8.
    } else {
        (covalent_radius(atom.z)
            * scale
            * if style == AtomStyle::Balls {
                0.62
            } else {
                0.31
            })
        .clamp(3., 24.)
            + 5.
    };
    let ring: Vec<_> = (0..=48)
        .map(|i| {
            let a = i as f32 * std::f32::consts::TAU / 48.;
            [p[0] + radius * a.cos(), p[1] + radius * a.sin(), p[2]]
        })
        .collect();
    line(w, &ring, gpui::rgb(0x151b23), 4.5, false);
    line(w, &ring, gold, 2.2, false);
    let text = gpui::SharedString::from(format!(
        "Absorber · {}",
        crate::structure::element_symbol(atom.z)
    ));
    let run = gpui::TextRun {
        len: text.len(),
        font: w.text_style().font(),
        color: gold.into(),
        background_color: None,
        underline: None,
        strikethrough: None,
    };
    let shaped = w.text_system().shape_line(text, px(12.), &[run], None);
    let width = f32::from(shaped.width) + 14.;
    let x = (p[0] + radius + 15.).clamp(
        f32::from(b.left()) + 6.,
        (f32::from(b.right()) - width - 6.).max(f32::from(b.left()) + 6.),
    );
    let y = (p[1] - radius - 35.).clamp(
        f32::from(b.top()) + 6.,
        (f32::from(b.bottom()) - 28.).max(f32::from(b.top()) + 6.),
    );
    let leader = [
        [p[0] + radius * 0.7, p[1] - radius * 0.7, p[2]],
        [x, y + 22., p[2]],
    ];
    line(w, &leader, gpui::rgb(0x151b23), 3., false);
    line(w, &leader, gold, 1.4, false);
    w.paint_quad(gpui::quad(
        Bounds::new(point(px(x), px(y)), size(px(width), px(24.))),
        gpui::Corners::all(px(5.)),
        alpha(t.surface, 0.96),
        gpui::Edges::all(px(1.)),
        gold,
        gpui::BorderStyle::Solid,
    ));
    shaped
        .paint(
            point(px(x + 7.), px(y + 4.)),
            px(16.),
            gpui::TextAlign::Left,
            None,
            w,
            cx,
        )
        .ok();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translucent_sphere_lighting_covers_one_disk_without_stacking_opacity() {
        let layers = sphere_lighting();
        assert_eq!(layers.len(), 13);
        assert!(
            std::ptr::eq(layers, sphere_lighting()),
            "lighting mesh is reused"
        );
        let area: f32 = layers
            .iter()
            .flat_map(|(path, _)| path.vertices.chunks_exact(3))
            .map(|triangle| {
                let p = triangle
                    .iter()
                    .map(|v| [f32::from(v.xy_position.x), f32::from(v.xy_position.y)])
                    .collect::<Vec<_>>();
                ((p[1][0] - p[0][0]) * (p[2][1] - p[0][1])
                    - (p[1][1] - p[0][1]) * (p[2][0] - p[0][0]))
                    .abs()
                    * 0.5
            })
            .sum();
        let disk_area = std::f32::consts::PI * 24_f32.powi(2);
        assert!(
            (area / disk_area - 1.).abs() < 0.08,
            "annular mesh area {area} vs disk {disk_area}"
        );
        for opacity in [1., 0.99, 0.98, 0.5, 0.02] {
            for theme in [Theme::dark(), Theme::light()] {
                let color = alpha(gpui::rgb(0xc58a42), opacity);
                let light =
                    depth_cue_color(tint(color, layers.last().unwrap().1), theme.raised, 0.36);
                let dark = depth_cue_color(tint(color, layers[0].1), theme.raised, 0.36);
                assert_eq!((light.a, dark.a), (opacity, opacity));
                assert!(
                    light.r - dark.r > 0.25,
                    "lighting must survive a small fade"
                );
            }
        }
    }

    #[test]
    fn distinct_path_legs_connect_atom_centers_at_every_view_angle() {
        let route = [[0., 0., 0.], [-2., 1., 1.], [1., 2., -1.], [0., 0., 0.]];
        let bounds = Bounds::new(point(px(0.), px(0.)), size(px(600.), px(400.)));
        for az in [-0.6, 0., 1.7] {
            for el in [-0.8, 0., 0.45, 0.9] {
                let camera = ViewCamera { az, el, zoom: 1. };
                for (i, pair) in route.windows(2).enumerate() {
                    let offset = route_lane_offset(&route, i);
                    assert_eq!(offset, 0.);
                    let start = camera.project(pair[0], bounds, 8.);
                    let end = camera.project(pair[1], bounds, 8.);
                    let stroke = PathStroke::new(start, end, offset).unwrap();
                    assert_eq!(stroke.points(0., 1.), vec![start, end]);
                }
            }
        }
    }

    #[test]
    fn repeated_legs_separate_between_shared_atom_centers() {
        let route = [[0., 0., 0.], [1., 0., 0.], [0., 0., 0.], [1., 0., 0.]];
        let a = [20., 30., -2.];
        let b = [180., 130., 3.];
        let forward = PathStroke::new(a, b, route_lane_offset(&route, 0)).unwrap();
        let reverse = PathStroke::new(b, a, route_lane_offset(&route, 1)).unwrap();
        let repeat = PathStroke::new(a, b, route_lane_offset(&route, 2)).unwrap();
        for (stroke, start, end) in [(&forward, a, b), (&reverse, b, a), (&repeat, a, b)] {
            let points = stroke.points(0., 1.);
            assert_eq!(points.first(), Some(&start));
            assert_eq!(points.last(), Some(&end));
        }
        let [x, y, _] = forward.point(0.5);
        let [rx, ry, _] = reverse.point(0.5);
        assert!(((x - rx).hypot(y - ry) - 8.).abs() < 1e-4);
        let [tx, ty, _] = repeat.point(0.5);
        assert!(((x - tx).hypot(y - ty) - 6.).abs() < 1e-4);
    }

    #[test]
    fn curved_leg_clipping_and_arrow_tangents_follow_the_same_curve() {
        for offset in [-10., 0., 4., 10.] {
            let stroke = PathStroke::new([20., 30., -2.], [180., 130., 3.], offset).unwrap();
            let first = stroke.points(0., 0.4);
            let second = stroke.points(0.4, 0.8);
            assert_eq!(first.last(), second.first());
            assert_eq!(second.last(), Some(&stroke.point(0.8)));
            let before = stroke.point(0.669);
            let after = stroke.point(0.671);
            let delta = [after[0] - before[0], after[1] - before[1]];
            let length = delta[0].hypot(delta[1]);
            let tangent = stroke.tangent(0.67);
            for axis in 0..2 {
                assert!((delta[axis] / length - tangent[axis]).abs() < 1e-4);
            }
        }
    }

    #[test]
    fn rear_material_has_less_contrast_without_changing_opacity_in_both_themes() {
        let material = alpha(gpui::rgb(0xc58a42), 0.35);
        for theme in [Theme::dark(), Theme::light()] {
            let front = depth_cue_color(material, theme.raised, 0.);
            let rear = depth_cue_color(material, theme.raised, 0.72);
            let contrast = |c: Rgba| {
                (c.r - theme.raised.r).abs()
                    + (c.g - theme.raised.g).abs()
                    + (c.b - theme.raised.b).abs()
            };
            assert!(contrast(rear) < contrast(front) * 0.3);
            assert_eq!(rear.a, material.a);
            assert_eq!(front, material);
        }
    }
    #[test]
    fn rutile_repeats_complete_titanium_oxygen_octahedra() {
        let s = core::read_cif(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../rexafs/data/builtin_cifs/tio2_rutile.cif"),
        )
        .unwrap();
        let c = core::build_cluster(
            &s,
            &core::AbsorberSelection::Element("Ti".into()),
            &core::ClusterOptions::default(),
        )
        .unwrap();
        let context = crystal_context(&s, &c);
        let cluster = Cluster::from_core(&c);
        let scene = MoleculeScene::new(
            &cluster,
            Some(&context),
            8.,
            AtomStyle::Polyhedra,
            None,
            None,
            PolyhedronOptions::default(),
        );
        let centers = c.atoms.iter().filter(|a| a.symbol == "Ti").count();
        assert_eq!(scene.poly_count, centers);
        assert_eq!(scene.faces.len(), centers * 8);
        assert!(
            scene
                .faces
                .iter()
                .all(|f| f.vertices.len() == 3 && f.z == 22)
        );
        let single = MoleculeScene::new(
            &cluster,
            Some(&context),
            8.,
            AtomStyle::Polyhedra,
            None,
            None,
            PolyhedronOptions {
                network: false,
                ligand: Some(8),
                atoms: PolyAtoms::All,
                ..Default::default()
            },
        );
        assert_eq!(single.poly_count, 1);
        assert_eq!(single.bonds.len(), 6);
        assert_eq!(single.poly_atoms.len(), 7);
        assert!(
            single
                .faces
                .iter()
                .all(|f| (norm(f.normal) - 1.).abs() < 1e-8)
        );
    }
    #[test]
    fn coordination_hulls_keep_coplanar_faces() {
        let cube: Vec<_> = (0..8)
            .map(|n| std::array::from_fn(|i| if n & (1 << i) == 0 { -1. } else { 1. }))
            .collect();
        let faces = hull_faces(&cube);
        assert_eq!(faces.len(), 6);
        assert!(faces.iter().all(|f| f.len() == 4));
        assert_eq!(
            hull_faces(&[
                [1., 0., 0.],
                [-1., 0., 0.],
                [0., 1., 0.],
                [0., -1., 0.],
                [0., 0., 1.],
                [0., 0., -1.]
            ])
            .len(),
            8
        );
    }
    #[test]
    fn large_scroll_event_keeps_cluster_in_view() {
        let mut camera = ViewCamera::default();
        camera.scroll_zoom(gpui::ScrollDelta::Lines(point(0., -120.)));
        assert!((1.0..1.17).contains(&camera.zoom));
        camera.scroll_zoom(gpui::ScrollDelta::Lines(point(0., 120.)));
        assert!((camera.zoom - 1.).abs() < 1e-12);
        let before = camera;
        camera.zoom_by(1.2_f64.ln());
        assert!((camera.zoom - 1.2).abs() < 1e-12);
        assert_eq!((camera.az, camera.el), (before.az, before.el));
    }

    #[test]
    fn dragging_follows_the_pointer_in_both_screen_directions() {
        let bounds = Bounds::new(point(px(0.), px(0.)), size(px(600.), px(400.)));
        for az in [-0.6, 0., 1.7] {
            for el in [-0.8, 0., 0.45, 0.9] {
                let camera = ViewCamera { az, el, zoom: 1. };
                let front = super::super::structure_depth::DepthAxis::View.normal(camera);
                let before = camera.project(front, bounds, 8.);
                for delta in [-10., 10.] {
                    let mut horizontal = camera;
                    horizontal.orbit_drag(delta, 0.);
                    let after = horizontal.project(front, bounds, 8.);
                    assert!((after[0] - before[0]) * delta > 0.);
                    assert_eq!(horizontal.el, camera.el);
                    assert_eq!(horizontal.zoom, camera.zoom);

                    let mut vertical = camera;
                    vertical.orbit_drag(0., delta);
                    let after = vertical.project(front, bounds, 8.);
                    assert!((after[1] - before[1]) * delta > 0.);
                    assert_eq!(vertical.az, camera.az);
                    assert_eq!(vertical.zoom, camera.zoom);
                }
            }
        }
    }

    #[test]
    fn orbit_preserves_scale_and_distance() {
        let b = Bounds::new(point(px(0.), px(0.)), size(px(600.), px(400.)));
        let a = ViewCamera::default();
        let c = ViewCamera {
            az: 2.,
            el: 0.9,
            ..a
        };
        assert_eq!(a.scale(b, 8.), c.scale(b, 8.));
        assert!((norm(a.rotate([1., 2., 3.])) - norm(c.rotate([1., 2., 3.]))).abs() < 1e-10);
    }
}
