//! Structure sources: a local CIF library, the AMCSD SQLite database
//! (feature `amcsd`) and the Materials Project API (feature
//! `materials-project`), behind one [`StructureSource`] trait.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::cif::read_cif;
use super::model::Structure;
use super::StructureError;

#[cfg(feature = "amcsd")]
pub mod amcsd;
#[cfg(feature = "http")]
pub mod cod;
#[cfg(feature = "materials-project")]
pub mod mp;

/// A search hit that can be fetched as a full [`Structure`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StructureHit {
    /// Source-specific identifier (`/path/x.cif`, `1234`, `mp-33`).
    pub id: String,
    /// Source name (`local`, `amcsd`, `materials-project`).
    pub source: String,
    /// Source formula text; its formatting may differ among databases.
    pub formula: String,
    /// Mineral or compound name.
    pub name: Option<String>,
    /// Source-provided space-group symbol, when known.
    pub space_group: Option<String>,
    /// Element symbols present.
    pub elements: Vec<String>,
    /// Anything else worth showing (cell parameters, energy above hull…).
    pub extra: BTreeMap<String, String>,
}

/// Structure-search terms; element constraints are combined with source-specific text semantics.
///
/// Defaults are no text, no required/excluded elements, exact_elements=false,
/// and limit=0. Local/bundled matching supports names and simple formula ratios;
/// Materials Project interprets text as an identifier, chemical system, or formula.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct StructureQuery {
    /// Search text; local matching checks formula, name, ID, and space-group
    /// metadata, while network sources translate it to their endpoint parameters.
    pub text: Option<String>,
    /// Every listed element must be present.
    pub elements: Vec<String>,
    /// None of these may be present.
    pub exclude: Vec<String>,
    /// Restrict to exactly these elements (chemical system) when set.
    pub exact_elements: bool,
    /// Requested result cap; zero uses the source default. This is not a promise
    /// of exhaustive server pagination or a total count of all database matches.
    pub limit: usize,
}

impl StructureQuery {
    /// Create a query with copied text, no element filters, and a source-default limit.
    pub fn text(text: &str) -> Self {
        Self {
            text: Some(text.to_string()),
            ..Default::default()
        }
    }

    /// Replace required element symbols with owned copies; other query settings remain unchanged.
    pub fn with_elements<I: IntoIterator<Item = S>, S: AsRef<str>>(mut self, elements: I) -> Self {
        self.elements = elements
            .into_iter()
            .map(|s| s.as_ref().to_string())
            .collect();
        self
    }

    /// Whether a hit satisfies the element constraints and text filter.
    pub fn matches(&self, hit: &StructureHit) -> bool {
        let has = |sym: &str| hit.elements.iter().any(|e| e.eq_ignore_ascii_case(sym));
        if !self.elements.iter().all(|e| has(e)) {
            return false;
        }
        if self.exclude.iter().any(|e| has(e)) {
            return false;
        }
        if self.exact_elements
            && (hit.elements.len() != self.elements.len()
                || !hit
                    .elements
                    .iter()
                    .all(|e| self.elements.iter().any(|q| q.eq_ignore_ascii_case(e))))
        {
            return false;
        }
        if let Some(text) = self
            .text
            .as_deref()
            .map(str::trim)
            .filter(|t| !t.is_empty())
        {
            let t = text.to_ascii_lowercase();
            let hay = format!(
                "{} {} {} {}",
                hit.formula,
                hit.name.as_deref().unwrap_or(""),
                hit.id,
                hit.space_group.as_deref().unwrap_or("")
            )
            .to_ascii_lowercase();
            if !hay.contains(&t) && !formula_matches(&hit.formula, text) {
                return false;
            }
        }
        true
    }
}

/// Compare normalized element/count maps: RuO2 matches Ru O2, O2Ru, and Ru2O4.
/// Element capitalization must be valid; this function does not parse `ruo2`.
/// See [`parse_formula`] for limitations; empty parsed queries never match.
pub fn formula_matches(formula: &str, query: &str) -> bool {
    let a = parse_formula(formula);
    let b = parse_formula(query);
    !b.is_empty() && a == b
}

/// Extract element/count ratios from simple formula text, normalized to the
/// smallest positive count and rounded to three decimal places.
///
/// Recognizes capitalized element symbols and immediate decimal counts; spaces
/// and punctuation are skipped. Parenthesized group multipliers, hydrates, and
/// isotope/charge syntax are not interpreted chemically. For example,
/// `(Fe0.5Ni0.5)O` is supported, but `Ca(OH)2` is not a grouped formula parser.
/// This search helper is not suitable for computing exact stoichiometry.
pub fn parse_formula(text: &str) -> BTreeMap<String, f64> {
    let mut out = BTreeMap::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_ascii_uppercase() {
            let mut sym = c.to_string();
            i += 1;
            while i < chars.len() && chars[i].is_ascii_lowercase() {
                sym.push(chars[i]);
                i += 1;
            }
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                i += 1;
            }
            let count: f64 = if start == i {
                1.0
            } else {
                chars[start..i]
                    .iter()
                    .collect::<String>()
                    .parse()
                    .unwrap_or(1.0)
            };
            if super::element::Element::from_symbol(&sym).is_some() {
                *out.entry(sym).or_insert(0.0) += count;
            }
        } else {
            i += 1;
        }
    }
    // Normalise so RuO2 == Ru2O4.
    let min = out.values().cloned().fold(f64::INFINITY, f64::min);
    if min.is_finite() && min > 0.0 {
        for v in out.values_mut() {
            *v = (*v / min * 1000.0).round() / 1000.0;
        }
    }
    out
}

/// A searchable source of crystal structures.
pub trait StructureSource {
    /// Borrow the source's identifying name.
    fn name(&self) -> &str;
    /// Find owned hits according to this source's query semantics and limits.
    /// Network implementations block on requests; local sources read their index
    /// or database. Database/network failures return StructureError.
    fn search(&self, query: &StructureQuery) -> Result<Vec<StructureHit>, StructureError>;
    /// Load an owned structure from a hit belonging to this source.
    /// Remote sources download it; local sources may reread its source file.
    /// Source availability, parsing, and conversion errors are returned to the caller.
    fn fetch(&self, hit: &StructureHit) -> Result<Structure, StructureError>;
}

/// A folder of `.cif` files, indexed once.
#[derive(Debug, Clone, Default)]
pub struct LocalCifLibrary {
    root: PathBuf,
    entries: Vec<(PathBuf, StructureHit)>,
    /// Files that failed to parse: (path, error).
    pub failures: Vec<(PathBuf, String)>,
}

impl LocalCifLibrary {
    /// Scan `root` recursively for `*.cif` (case-insensitive) and index
    /// them by loading each file through the structure/CIF conversion pipeline.
    /// Failed files are retained in failures rather than aborting the index;
    /// directory traversal failures can abort the scan. This snapshot is not
    /// refreshed automatically when files change. Fetching rereads the current file.
    pub fn scan<P: AsRef<Path>>(root: P) -> Result<Self, StructureError> {
        let root = root.as_ref().to_path_buf();
        let mut files = Vec::new();
        collect_cifs(&root, &mut files)?;
        files.sort();
        let mut lib = Self {
            root,
            entries: Vec::new(),
            failures: Vec::new(),
        };
        for path in files {
            match read_cif(&path) {
                Ok(structure) => {
                    let hit = hit_from_structure(&structure, "local", &path.display().to_string());
                    lib.entries.push((path, hit));
                }
                Err(e) => lib.failures.push((path, e.to_string())),
            }
        }
        Ok(lib)
    }

    /// Borrow the root directory recorded when this library was scanned.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Number of successfully parsed/indexed CIF files; excludes failures.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the index contains no successfully parsed CIF files.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Borrow indexed hits in sorted source-file order without reading files again.
    pub fn hits(&self) -> impl Iterator<Item = &StructureHit> {
        self.entries.iter().map(|(_, h)| h)
    }
}

fn collect_cifs(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), StructureError> {
    let rd = std::fs::read_dir(dir).map_err(|source| StructureError::Io {
        path: dir.display().to_string(),
        source,
    })?;
    for entry in rd.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_cifs(&path, out)?;
        } else if path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("cif"))
        {
            out.push(path);
        }
    }
    Ok(())
}

/// Build a hit describing `structure`.
pub fn hit_from_structure(structure: &Structure, source: &str, id: &str) -> StructureHit {
    let mut extra = BTreeMap::new();
    let l = &structure.lattice;
    extra.insert(
        "cell".into(),
        format!(
            "a={:.4} b={:.4} c={:.4} α={:.2} β={:.2} γ={:.2}",
            l.a, l.b, l.c, l.alpha, l.beta, l.gamma
        ),
    );
    extra.insert("sites".into(), structure.num_sites().to_string());
    StructureHit {
        id: id.to_string(),
        source: source.to_string(),
        formula: structure.formula(),
        name: structure.mineral.clone().or_else(|| {
            if structure.title.is_empty() {
                None
            } else {
                Some(structure.title.clone())
            }
        }),
        space_group: structure
            .space_group
            .hm_symbol
            .clone()
            .or_else(|| structure.space_group.number.map(|n| format!("#{n}"))),
        elements: structure.elements(),
        extra,
    }
}

impl StructureSource for LocalCifLibrary {
    fn name(&self) -> &str {
        "local"
    }

    fn search(&self, query: &StructureQuery) -> Result<Vec<StructureHit>, StructureError> {
        let limit = if query.limit == 0 {
            usize::MAX
        } else {
            query.limit
        };
        Ok(self
            .entries
            .iter()
            .map(|(_, h)| h)
            .filter(|h| query.matches(h))
            .take(limit)
            .cloned()
            .collect())
    }

    fn fetch(&self, hit: &StructureHit) -> Result<Structure, StructureError> {
        let path = self
            .entries
            .iter()
            .find(|(p, _)| p.display().to_string() == hit.id)
            .map(|(p, _)| p.clone())
            .unwrap_or_else(|| PathBuf::from(&hit.id));
        read_cif(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formulas_compare_by_composition() {
        assert!(formula_matches("Ru O2", "RuO2"));
        assert!(formula_matches("O2Ru", "RuO2"));
        assert!(formula_matches("Fe2 O3", "Fe4O6"));
        assert!(!formula_matches("FeO", "Fe2O3"));
        assert!(!formula_matches("RuO2", "Ru"));
        let parsed = parse_formula("Fe2O3");
        assert_eq!(parsed.len(), 2);
        assert!(parse_formula("").is_empty());
    }

    #[test]
    fn query_filters_hits() {
        let hit = StructureHit {
            id: "x".into(),
            source: "local".into(),
            formula: "RuO2".into(),
            name: Some("Rutile-type ruthenium oxide".into()),
            space_group: Some("P 42/m n m".into()),
            elements: vec!["Ru".into(), "O".into()],
            extra: BTreeMap::new(),
        };
        assert!(StructureQuery::text("ruthenium").matches(&hit));
        assert!(StructureQuery::text("Ru O2").matches(&hit));
        assert!(StructureQuery::default()
            .with_elements(["Ru"])
            .matches(&hit));
        assert!(!StructureQuery::default()
            .with_elements(["Fe"])
            .matches(&hit));
        let mut q = StructureQuery::default().with_elements(["Ru"]);
        q.exact_elements = true;
        assert!(!q.matches(&hit));
        let q = StructureQuery {
            exclude: vec!["O".into()],
            ..Default::default()
        };
        assert!(!q.matches(&hit));
    }
}
