//! Crystal structures for EXAFS modelling: CIF import, space-group
//! expansion, cluster generation around an absorber, `feff.inp` export,
//! structure databases (bundled/local CIF libraries, AMCSD, COD, Materials Project) and
//! FEFF path geometry for visualisation.
//!
//! Pipeline: `Structure` (from a CIF, a database, or built by hand) →
//! [`cluster::build_cluster`] → [`feffinp::write_feff_inp`] → the existing
//! FEFF runners in [`crate::xafs::fitting::runner`] → path files whose leg
//! geometry [`paths::PathGeometry`] maps back onto the cluster atoms.
//!
//! # Minimal in-memory workflow
//!
//! The following face-centered-cubic cell uses an illustrative 3.61 Å lattice
//! constant; use a structure appropriate to the measured sample. Coordinates
//! passed to Site::new are fractional, while the generated cluster uses Å.
//!
//! ```
//! use rexafs::structure::{
//!     build_cluster, write_feff_inp, AbsorberSelection, ClusterOptions,
//!     FeffInputOptions, Lattice, Site, Structure,
//! };
//! # fn main() -> Result<(), rexafs::structure::StructureError> {
//! let sites = vec![
//!     Site::new("Cu1", "Cu", [0.0, 0.0, 0.0]),
//!     Site::new("Cu2", "Cu", [0.0, 0.5, 0.5]),
//!     Site::new("Cu3", "Cu", [0.5, 0.0, 0.5]),
//!     Site::new("Cu4", "Cu", [0.5, 0.5, 0.0]),
//! ];
//! let structure = Structure::new("Illustrative fcc Cu", Lattice::cubic(3.61)?, sites);
//! let cluster = build_cluster(
//!     &structure, &AbsorberSelection::SiteIndex(0), &ClusterOptions::default(),
//! )?;
//! let input_text = write_feff_inp(&cluster, &FeffInputOptions::default());
//! assert!(input_text.contains("ATOMS"));
//! # Ok(())
//! # }
//! ```
//!
//! Defaults make an 8 Å periodic cluster without non-absorber hydrogen, resolve
//! mixed sites to their majority species, and write K-edge EXAFS input text.
//! This creates no files and runs no scattering calculation. Basis/symmetry
//! conventions are explained in [`lattice`] and [`symmetry`]; cluster occupancy
//! choices and their limits are documented on [`OccupancyPolicy`].

pub mod builtin;
pub mod cif;
pub mod cluster;
pub mod db;
pub mod element;
mod element_table;
pub mod feffinp;
pub mod lattice;
pub mod model;
pub mod pathrank;
pub mod paths;
pub mod symmetry;
pub mod xyz;

pub use builtin::{BuiltinEntry, BuiltinLibrary};
pub use cif::{parse_cif, read_cif, structure_from_cif, structure_to_cif, CifBlock, CifLoop};
pub use cluster::{
    absorber_sites, build_cluster, AbsorberSelection, Cluster, ClusterAtom, ClusterOptions,
    OccupancyPolicy, Potential, Shell,
};
pub use db::{LocalCifLibrary, StructureHit, StructureQuery, StructureSource};
pub use element::Element;
pub use feffinp::{write_feff_inp, Edge, FeffInputOptions, FeffInputStyle};
pub use lattice::Lattice;
pub use model::{Site, SpaceGroupInfo, Species, Structure};
pub use pathrank::{
    path_label, rank_paths, select_by, select_default, shells_of, PathInfo, ShellInfo,
};
pub use paths::{PathGeometry, PathLeg};
pub use symmetry::{expand_sites, find_space_group, SpaceGroupEntry, SymOp};
pub use xyz::{parse_xyz, read_xyz, Xyz, XyzAbsorber, XyzAtom};

use thiserror::Error;

/// Errors raised by the structure module.
#[derive(Debug, Error)]
pub enum StructureError {
    #[error("CIF parse error at line {line}: {message}")]
    /// CIF tokenization or block parsing failed.
    CifParse {
        /// One-based input line number reported by the CIF parser.
        line: usize,
        /// Human-readable parsing explanation.
        message: String,
    },
    #[error("CIF has no crystal structure data: {reason}")]
    /// The CIF input does not contain enough usable structural data.
    CifNoStructure {
        /// Explanation of the failed validation or underlying operation.
        reason: String,
    },
    #[error("unknown element or site label '{label}'")]
    /// An element label could not be resolved by the selected operation.
    UnknownElement {
        /// Unresolved element symbol or source label.
        label: String,
    },
    #[error("unknown space group ({reason})")]
    /// The requested space-group identity is not supported by the lookup.
    UnknownSpaceGroup {
        /// Explanation of the failed validation or underlying operation.
        reason: String,
    },
    #[error("invalid symmetry operation '{op}': {reason}")]
    /// A fractional symmetry-operation string is malformed.
    InvalidSymOp {
        /// Fractional-coordinate operation text that could not be parsed.
        op: String,
        /// Explanation of the failed validation or underlying operation.
        reason: String,
    },
    #[error("invalid lattice: {reason}")]
    /// Cell geometry is invalid or its matrix cannot be inverted.
    InvalidLattice {
        /// Explanation of the failed validation or underlying operation.
        reason: String,
    },
    #[error("absorber not found: {reason}")]
    /// The selected calculation absorber could not be found or resolved.
    AbsorberNotFound {
        /// Explanation of the failed validation or underlying operation.
        reason: String,
    },
    #[error("invalid cluster request: {reason}")]
    /// A requested atom cluster cannot be constructed from the inputs.
    InvalidCluster {
        /// Explanation of the failed validation or underlying operation.
        reason: String,
    },
    #[error("structure database error: {reason}")]
    /// A database query, record conversion, or bundled catalog operation failed.
    Database {
        /// Explanation of the failed validation or underlying operation.
        reason: String,
    },
    #[error("network error: {reason}")]
    /// A remote request or response conversion failed.
    Network {
        /// Explanation of the failed validation or underlying operation.
        reason: String,
    },
    #[error("I/O error for {path}: {source}")]
    /// A structure-related filesystem operation failed.
    Io {
        /// Source or destination filesystem path associated with the failure.
        path: String,
        #[source]
        /// Underlying operating-system I/O error.
        source: std::io::Error,
    },
    #[error("path geometry error: {reason}")]
    /// Path atom coordinates cannot be interpreted or mapped as requested.
    PathGeometry {
        /// Explanation of the failed validation or underlying operation.
        reason: String,
    },
}

impl From<serde_json::Error> for StructureError {
    fn from(err: serde_json::Error) -> Self {
        StructureError::Database {
            reason: format!("JSON: {err}"),
        }
    }
}
