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
    /// Read explicit XDI identity fields before the comment/data separator.
    /// Conflicting declarations remain unknown; filenames and energies are not evidence.
    pub fn from_xdi_header_text(text: &str) -> Option<Self> {
        let mut symbols = std::collections::BTreeSet::new();
        let mut edges = std::collections::BTreeSet::new();
        for line in text.lines() {
            let Some(line) = line.trim().strip_prefix('#').map(str::trim) else {
                continue;
            };
            if line == "///" || line.starts_with("---") {
                break;
            }
            let Some((key, value)) = line.split_once(':') else {
                continue;
            };
            let value = value.trim().to_ascii_lowercase();
            if value.is_empty() {
                continue;
            }
            if key.trim().eq_ignore_ascii_case("Element.symbol") {
                symbols.insert(value);
            } else if key.trim().eq_ignore_ascii_case("Element.edge") {
                edges.insert(value);
            }
        }
        if symbols.len() != 1 || edges.len() != 1 {
            return None;
        }
        let symbol = symbols.into_iter().next()?;
        let mut chars = symbol.chars();
        let element = chars.next()?.to_uppercase().collect::<String>() + chars.as_str();
        Some(Self {
            element,
            edge: edges.into_iter().next()?.to_ascii_uppercase(),
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

#[cfg(test)]
mod xdi_identity_tests {
    use super::*;
    #[test]
    fn retained_xdi_identity_is_explicit_and_unambiguous() {
        let header =
            "# XDI/1.0\n# Element.symbol: cU\n# Element.edge: k\n# ///\n# Element.symbol: Fe\n";
        assert_eq!(
            DeclaredEdge::from_xdi_header_text(header).unwrap().label(),
            "Cu K"
        );
        assert!(
            DeclaredEdge::from_xdi_header_text(
                "# Element.symbol: Cu\n# Element.symbol: Fe\n# Element.edge: K"
            )
            .is_none()
        );
        assert!(DeclaredEdge::from_xdi_header_text("# Cu K at 8979 eV").is_none());
    }
}
