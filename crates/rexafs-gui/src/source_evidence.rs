//! Portable evidence from the last full read, separate from live readiness.
use crate::params::{ImportConfig, ParserDiagnostics};
use rexafs::prelude::XdiHeader;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DeclaredEdge {
    pub element: String,
    pub edge: String,
}

impl DeclaredEdge {
    pub fn from_header(header: Option<&XdiHeader>) -> Option<Self> {
        let header = header?;
        let element = header.get("element.symbol")?.trim();
        let edge = header.get("element.edge")?.trim();
        if element.is_empty() || edge.is_empty() {
            return None;
        }
        let mut chars = element.chars();
        let element =
            chars.next()?.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase();
        Some(Self {
            element,
            edge: edge.to_ascii_uppercase(),
        })
    }
    pub fn label(&self) -> String {
        format!("{} {}", self.element, self.edge)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ParserRecord {
    pub path: PathBuf,
    #[serde(default)]
    pub channel: crate::params::DetectionMode,
    pub mapping_revision: u64,
    pub diagnostics: ParserDiagnostics,
    pub declared_edge: Option<DeclaredEdge>,
}

impl ParserRecord {
    pub fn matches(&self, mapping: &ImportConfig) -> bool {
        self.mapping_revision == crate::import_recipes::mapping_revision(mapping)
    }
}
