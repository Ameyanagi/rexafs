//! `feff.inp` writer for a [`Cluster`], compatible with the FEFF6/8/10 style
//! inputs the runners in [`crate::xafs::fitting::runner`] accept.

use serde::{Deserialize, Serialize};

use super::cluster::Cluster;

/// Absorption edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Edge {
    /// K absorption edge; default, written as FEFF HOLE index 1.
    #[default]
    K,
    /// L1 absorption edge, written as FEFF HOLE index 2.
    L1,
    /// L2 absorption edge, written as FEFF HOLE index 3.
    L2,
    /// L3 absorption edge, written as FEFF HOLE index 4.
    L3,
    /// M1 absorption edge, written as FEFF HOLE index 5.
    M1,
    /// M2 absorption edge, written as FEFF HOLE index 6.
    M2,
    /// M3 absorption edge, written as FEFF HOLE index 7.
    M3,
    /// M4 absorption edge, written as FEFF HOLE index 8.
    M4,
    /// M5 absorption edge, written as FEFF HOLE index 9.
    M5,
}

impl Edge {
    /// FEFF `HOLE` index.
    pub fn hole_index(self) -> u32 {
        match self {
            Edge::K => 1,
            Edge::L1 => 2,
            Edge::L2 => 3,
            Edge::L3 => 4,
            Edge::M1 => 5,
            Edge::M2 => 6,
            Edge::M3 => 7,
            Edge::M4 => 8,
            Edge::M5 => 9,
        }
    }

    /// Conventional edge label, such as K or L3.
    pub fn label(self) -> &'static str {
        match self {
            Edge::K => "K",
            Edge::L1 => "L1",
            Edge::L2 => "L2",
            Edge::L3 => "L3",
            Edge::M1 => "M1",
            Edge::M2 => "M2",
            Edge::M3 => "M3",
            Edge::M4 => "M4",
            Edge::M5 => "M5",
        }
    }

    /// Parse a trimmed edge label ignoring ASCII case; L aliases L3 and M aliases M5.
    /// Unknown labels return None.
    pub fn parse(text: &str) -> Option<Edge> {
        match text.trim().to_ascii_uppercase().as_str() {
            "K" => Some(Edge::K),
            "L1" => Some(Edge::L1),
            "L2" => Some(Edge::L2),
            "L3" | "L" => Some(Edge::L3),
            "M1" => Some(Edge::M1),
            "M2" => Some(Edge::M2),
            "M3" => Some(Edge::M3),
            "M4" => Some(Edge::M4),
            "M5" | "M" => Some(Edge::M5),
            _ => None,
        }
    }

    /// Larch's default: K below cerium, L3 from Z = 58 on.
    pub fn default_for_z(z: u8) -> Edge {
        if z < 58 {
            Edge::K
        } else {
            Edge::L3
        }
    }

    /// Supported absorption-edge labels in increasing shell-index order.
    pub const ALL: [Edge; 9] = [
        Edge::K,
        Edge::L1,
        Edge::L2,
        Edge::L3,
        Edge::M1,
        Edge::M2,
        Edge::M3,
        Edge::M4,
        Edge::M5,
    ];
}

/// Header card style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum FeffInputStyle {
    /// `HOLE / CONTROL / PRINT / RMAX / NLEG / EXAFS` — what the embedded
    /// runners (ReFEFF, FEFF10) and FEFF6/8L all accept.
    #[default]
    Classic,
    /// `EDGE / S02 / CONTROL / PRINT / EXAFS / RPATH` (+ optional `SCF`),
    /// the FEFF8/10 spelling Larch writes.
    Feff8,
}

/// Input-card settings; creating text does not verify backend support for every card.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FeffInputOptions {
    /// Absorption edge; default K. This does not automatically use `default_for_z`.
    pub edge: Edge,
    /// Classic `RMAX` in Å; None uses the cluster radius, clamped to 2–50 Å.
    pub rmax: Option<f64>,
    /// Maximum half-path length `RPATH` in Å for Feff8 style; None uses rmax.
    /// This field is unused by Classic style.
    pub rpath: Option<f64>,
    /// Dimensionless S₀² amplitude factor; default 1.0.
    pub s02: f64,
    /// Emit `SCF 5.0` for a 5 Å self-consistent-potential region; default false.
    pub scf: bool,
    /// Emit `EXAFS 20` when true (default), `XANES 4.0` otherwise.
    /// Actual XANES capability depends on the chosen calculation backend.
    pub exafs: bool,
    /// Maximum number of path legs; default 4, written with a minimum of 2.
    pub nleg: u8,
    /// Header-card dialect; defaults to Classic.
    pub style: FeffInputStyle,
    /// Extra `TITLE` lines.
    pub titles: Vec<String>,
}

impl Default for FeffInputOptions {
    fn default() -> Self {
        Self {
            edge: Edge::K,
            rmax: None,
            rpath: None,
            s02: 1.0,
            scf: false,
            exafs: true,
            nleg: 4,
            style: FeffInputStyle::Classic,
            titles: Vec::new(),
        }
    }
}

/// Render owned `feff.inp` text without writing files or running a calculation.
///
/// Atom positions and distances use Å. Every atom already present in the cluster
/// is written; changing the RMAX/RPATH card does not regenerate the cluster.
/// A nonempty cluster with its absorber first is required. Preserve the returned
/// text with results to record the exact cards and geometry used.
pub fn write_feff_inp(cluster: &Cluster, opts: &FeffInputOptions) -> String {
    let absorber = cluster.absorber();
    let rmax = opts.rmax.unwrap_or(cluster.radius).clamp(2.0, 50.0);
    let rpath = opts.rpath.unwrap_or(rmax);
    let mut s = String::new();
    s.push_str("* feff.inp generated by rexafs\n");
    s.push_str(&format!(
        "TITLE {} {} edge, {} (cluster {:.2} Å)\n",
        absorber.symbol,
        opts.edge.label(),
        cluster.structure_title,
        cluster.radius
    ));
    s.push_str(&format!("TITLE Formula: {}\n", cluster.formula));
    if let Some(sg) = &cluster.space_group {
        s.push_str(&format!("TITLE SpaceGroup: {sg}\n"));
    }
    for t in &opts.titles {
        let t = t.trim();
        if !t.is_empty() {
            s.push_str(&format!("TITLE {t}\n"));
        }
    }
    for w in &cluster.warnings {
        s.push_str(&format!("* {w}\n"));
    }
    s.push('\n');
    match opts.style {
        FeffInputStyle::Classic => {
            s.push_str(&format!(
                "HOLE {}   {:.3}\n\n",
                opts.edge.hole_index(),
                opts.s02
            ));
            s.push_str("CONTROL   1      1     1     1     1     1\n");
            s.push_str("PRINT     1      0     0     0     0     3\n");
            s.push_str(&format!("RMAX      {rmax:.2}\n"));
            s.push_str(&format!("NLEG      {}\n", opts.nleg.max(2)));
            if opts.exafs {
                s.push_str("EXAFS     20\n");
            } else {
                s.push_str("XANES     4.0\n");
            }
            if opts.scf {
                s.push_str("SCF       5.0\n");
            }
        }
        FeffInputStyle::Feff8 => {
            s.push_str(&format!("EDGE      {}\n", opts.edge.label()));
            s.push_str(&format!("S02       {:.3}\n", opts.s02));
            s.push_str("CONTROL   1 1 1 1 1 1\n");
            s.push_str("PRINT     1 0 0 0 0 3\n");
            if opts.exafs {
                s.push_str("EXAFS     20.0\n");
            } else {
                s.push_str("XANES     4.0\n");
            }
            s.push_str(&format!("RPATH     {rpath:.2}\n"));
            s.push_str(&format!("NLEG      {}\n", opts.nleg.max(2)));
            if opts.scf {
                s.push_str("SCF       5.0\n");
            } else {
                s.push_str("*SCF      5.0\n");
            }
            s.push_str("EXCHANGE  0\n");
        }
    }
    s.push_str("\nPOTENTIALS\n*   ipot   Z   tag\n");
    for p in &cluster.potentials {
        let tag = if p.ipot == 0 {
            format!("{}0", p.symbol)
        } else {
            p.symbol.clone()
        };
        s.push_str(&format!("    {:4}  {:3}   {}\n", p.ipot, p.z, tag));
    }
    s.push_str("\nATOMS\n*      x          y          z     ipot  tag      distance  site\n");
    for atom in &cluster.atoms {
        let tag = if atom.ipot == 0 {
            format!("{}0", atom.symbol)
        } else {
            atom.symbol.clone()
        };
        s.push_str(&format!(
            "  {:9.5}  {:9.5}  {:9.5}  {:3}   {:<6} {:9.5}  * {}\n",
            atom.cart[0], atom.cart[1], atom.cart[2], atom.ipot, tag, atom.distance, atom.label
        ));
    }
    s.push_str("END\n");
    s
}
