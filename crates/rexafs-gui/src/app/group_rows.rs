//! Pure presentation order. Indices are the session adapter; expansion is
//! resolved from durable GroupIds by the caller. No source files are read.
use std::collections::{BTreeMap, BTreeSet};

use super::{DERIVED_BASE, catalog_row_index, filter_match_lower};
use crate::{catalog::Catalog, params::DerivedSpectrum};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Section {
    Results,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Row {
    Primary {
        group: usize,
        expanded: bool,
        extra_channels: usize,
    },
    Child {
        group: usize,
        parent: usize,
    },
    Header(Section),
    Result {
        group: usize,
    },
}

impl Row {
    pub fn group(self) -> Option<usize> {
        match self {
            Self::Primary { group, .. } | Self::Child { group, .. } | Self::Result { group } => {
                Some(group)
            }
            Self::Header(_) => None,
        }
    }
}

/// Only stack metadata and inserted rows are stored. Catalog primaries remain
/// virtual, including filtered catalogs (the filter's Arc is shared).
pub struct Rows {
    catalog_len: usize,
    filtered: Option<std::sync::Arc<Vec<usize>>>,
    base_len: usize,
    primaries: BTreeMap<usize, Row>,
    inserted: Vec<(usize, Row)>,
    inserted_groups: BTreeMap<usize, usize>,
}

impl Rows {
    pub fn row_count(&self) -> usize {
        self.base_len + self.inserted.len()
    }

    pub fn row_at(&self, row: usize) -> Option<Row> {
        if row >= self.row_count() {
            return None;
        }
        let before = self.inserted.partition_point(|(i, _)| *i < row);
        if let Some(&(i, value)) = self.inserted.get(before)
            && i == row
        {
            return Some(value);
        }
        let group = catalog_row_index(
            self.filtered.as_deref().map(Vec::as_slice),
            row - before,
            self.catalog_len,
        )?;
        Some(self.primaries.get(&group).copied().unwrap_or(Row::Primary {
            group,
            expanded: false,
            extra_channels: 0,
        }))
    }

    pub fn row_index(&self, group: usize) -> Option<usize> {
        if let Some(&row) = self.inserted_groups.get(&group) {
            return Some(row);
        }
        if group >= self.catalog_len {
            return None;
        }
        let base = match &self.filtered {
            Some(f) => f.binary_search(&group).ok()?,
            None => group,
        };
        // Insertion anchors are recoverable by subtracting the sparse ordinal.
        let mut lo = 0;
        let mut hi = self.inserted.len();
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            if self.inserted[mid].0 - mid <= base {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        Some(base + lo)
    }

    pub fn neighbor(&self, active: Option<usize>, delta: isize) -> Option<usize> {
        let Some(mut row) = active.and_then(|g| self.row_index(g)) else {
            return (0..self.row_count()).find_map(|i| self.row_at(i)?.group());
        };
        for _ in 0..delta.unsigned_abs() {
            loop {
                row = row.checked_add_signed(delta.signum())?;
                if self.row_at(row)?.group().is_some() {
                    break;
                }
            }
        }
        self.row_at(row)?.group()
    }

    pub fn range(&self, anchor: Option<usize>, endpoint: usize) -> Vec<usize> {
        let Some(end) = self.row_index(endpoint) else {
            return Vec::new();
        };
        let start = anchor.and_then(|g| self.row_index(g)).unwrap_or(end);
        (start.min(end)..=start.max(end))
            .filter_map(|i| self.row_at(i)?.group())
            .collect()
    }
}

/// Catalog order, orphan stacks, then Results. A matching child can reveal
/// its primary without mutating saved expansion. Work scales with derived
/// groups, never with the number of catalog primaries.
pub fn build_rows(
    catalog: &Catalog,
    derived: &[DerivedSpectrum],
    expanded: &BTreeSet<usize>,
    filtered: Option<std::sync::Arc<Vec<usize>>>,
    query: &str,
    standalone: Option<&std::path::Path>,
) -> Rows {
    let query = query.to_ascii_lowercase();
    let base_len = filtered.as_ref().map_or(catalog.len(), |f| {
        f.partition_point(|&ix| ix < catalog.len())
    });
    let mut rows = Rows {
        catalog_len: catalog.len(),
        filtered,
        base_len,
        primaries: BTreeMap::new(),
        inserted: Vec::new(),
        inserted_groups: BTreeMap::new(),
    };
    let mut children: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    let mut orphan_sources = BTreeMap::new();
    let mut orphans = Vec::new();
    let mut results = Vec::new();
    if standalone.is_some() {
        orphans.push(super::NO_ENTRY);
    }
    for (i, d) in derived.iter().enumerate() {
        let group = DERIVED_BASE + i;
        if let Some(path) = &d.source {
            let primary = if standalone == Some(path.as_path()) {
                super::NO_ENTRY
            } else {
                catalog.find_by_canonical_path(path).unwrap_or_else(|| {
                    *orphan_sources.entry(path).or_insert_with(|| {
                        orphans.push(group);
                        group
                    })
                })
            };
            if primary != group {
                children.entry(primary).or_default().push(group);
            }
        } else {
            results.push(group);
        }
    }
    let matches = |group: usize| {
        let d = &derived[group - DERIVED_BASE];
        filter_match_lower(&d.label.to_ascii_lowercase(), &query)
            || filter_match_lower(
                &d.params
                    .as_ref()
                    .map(|p| p.import.mode.label())
                    .unwrap_or("")
                    .to_ascii_lowercase(),
                &query,
            )
    };
    let mut inserts: BTreeMap<usize, Vec<Row>> = BTreeMap::new();
    for group in children
        .keys()
        .copied()
        .filter(|&g| g < catalog.len())
        .chain(orphans)
    {
        let source_matches = if group == super::NO_ENTRY {
            standalone.is_some_and(|path| {
                filter_match_lower(&path.to_string_lossy().to_ascii_lowercase(), &query)
            })
        } else if group < catalog.len() {
            rows.filtered
                .as_ref()
                .is_none_or(|f| f.binary_search(&group).is_ok())
        } else {
            matches(group)
        };
        let members = children.get(&group).map(Vec::as_slice).unwrap_or_default();
        let matching: Vec<_> = members
            .iter()
            .copied()
            .filter(|&g| source_matches || matches(g))
            .collect();
        if !source_matches && matching.is_empty() && group != super::NO_ENTRY {
            continue;
        }
        let expanded = expanded.contains(&group) || (!source_matches && !matching.is_empty());
        let primary = Row::Primary {
            group,
            expanded,
            extra_channels: members.len(),
        };
        let anchor = if group < catalog.len() {
            let base = rows
                .filtered
                .as_ref()
                .map_or(group, |f| f.partition_point(|&g| g < group));
            if source_matches {
                rows.primaries.insert(group, primary);
                base + 1
            } else {
                inserts.entry(base).or_default().push(primary);
                base
            }
        } else {
            inserts.entry(base_len).or_default().push(primary);
            base_len
        };
        if expanded {
            inserts
                .entry(anchor)
                .or_default()
                .extend(matching.into_iter().map(|child| Row::Child {
                    group: child,
                    parent: group,
                }));
        }
    }
    let mut results = results.into_iter().filter(|&g| matches(g)).peekable();
    if results.peek().is_some() {
        let tail = inserts.entry(base_len).or_default();
        tail.push(Row::Header(Section::Results));
        tail.extend(results.map(|group| Row::Result { group }));
    }
    for (base, values) in inserts {
        for value in values {
            let row = base + rows.inserted.len();
            if let Some(group) = value.group() {
                rows.inserted_groups.insert(group, row);
            }
            rows.inserted.push((row, value));
        }
    }
    rows
}

/// Preserve defaults unless an overlay collides, then use compare positions.
pub fn overlay_colors(defaults: &[usize]) -> Vec<usize> {
    let unique: BTreeSet<_> = defaults.iter().copied().collect();
    if unique.len() == defaults.len() {
        defaults.to_vec()
    } else {
        (0..defaults.len()).map(|i| i % 8).collect()
    }
}

/// Compact visible kind; operation names use the persisted tool vocabulary.
pub fn kind(mode: crate::params::DetectionMode, result: Option<&DerivedSpectrum>) -> &'static str {
    use crate::params::{DetectionMode, Quantity};
    if let Some(result) = result {
        return match result.operation.as_ref().map(|op| op.tool.as_str()) {
            Some("merge") => "Merge",
            Some("Difference spectrum") => "Diff",
            Some("Align to reference") => "Align",
            Some("Calibrate energy") => "Calib",
            Some("Deglitch") => "Deglitch",
            Some("Truncate") => "Trunc",
            Some("Rebin") => "Rebin",
            Some("Smooth") => "Smooth",
            _ if result.quantity == Quantity::NormalizedDifference => "Diff",
            _ => result.quantity.label(),
        };
    }
    match mode {
        DetectionMode::Transmission => "Trans",
        DetectionMode::Fluorescence => "Fluo",
        DetectionMode::Reference => "Ref",
        DetectionMode::MuColumn => "μ",
        DetectionMode::Auto => "",
    }
}

/// Deterministic eight-swatch default derived from durable group metadata.
pub fn color_index(id: &crate::group_identity::GroupId) -> usize {
    let text = serde_json::to_string(id).unwrap_or_default();
    text.bytes().fold(0usize, |hash, b| {
        hash.wrapping_mul(31).wrapping_add(b as usize)
    }) % 8
}

/// Current processing diagnostics are independent of the Problems history.
/// Tickets stop older completions from replacing a newer attempt's status.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct DiagnosticTicket {
    id: crate::group_identity::GroupId,
    fingerprint: u64,
    generation: u64,
}

#[derive(Default)]
pub(super) struct Diagnostics {
    generation: u64,
    current: BTreeMap<crate::group_identity::GroupId, (DiagnosticTicket, Vec<super::JobError>)>,
}

impl Diagnostics {
    pub fn begin(
        &mut self,
        id: crate::group_identity::GroupId,
        fingerprint: u64,
    ) -> DiagnosticTicket {
        self.generation += 1;
        let ticket = DiagnosticTicket {
            id: id.clone(),
            fingerprint,
            generation: self.generation,
        };
        self.current.insert(id, (ticket.clone(), Vec::new()));
        ticket
    }

    pub fn finish(&mut self, ticket: &DiagnosticTicket, problems: Vec<super::JobError>) {
        if let Some((current, values)) = self.current.get_mut(&ticket.id)
            && current == ticket
        {
            *values = problems;
        }
    }

    /// A current import preview refreshes parser warnings without erasing a
    /// processing error or superseding an in-flight attempt of the same state.
    pub fn set_warnings(
        &mut self,
        id: crate::group_identity::GroupId,
        fingerprint: u64,
        warnings: Vec<super::JobError>,
    ) {
        if self
            .current
            .get(&id)
            .is_none_or(|(ticket, _)| ticket.fingerprint != fingerprint)
        {
            self.begin(id.clone(), fingerprint);
        }
        if let Some((_, values)) = self.current.get_mut(&id) {
            values.retain(|p| p.severity == super::ProblemSeverity::Error);
            values.extend(warnings);
        }
    }

    pub fn get(&self, id: &crate::group_identity::GroupId, fingerprint: u64) -> &[super::JobError] {
        self.current
            .get(id)
            .filter(|(ticket, _)| ticket.fingerprint == fingerprint)
            .map(|(_, values)| values.as_slice())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        catalog::FileMeta,
        params::{DetectionMode, PipelineParams},
    };
    fn build_rows(
        catalog: &Catalog,
        derived: &[DerivedSpectrum],
        expanded: &BTreeSet<usize>,
        filtered: Option<&[usize]>,
        query: &str,
    ) -> Vec<Row> {
        let rows = super::build_rows(
            catalog,
            derived,
            expanded,
            filtered.map(|f| std::sync::Arc::new(f.to_vec())),
            query,
            None,
        );
        (0..rows.row_count())
            .filter_map(|i| rows.row_at(i))
            .collect()
    }
    fn catalog() -> Catalog {
        let mut c = Catalog::default();
        c.extend(
            ["a.dat", "b.dat"]
                .into_iter()
                .map(|name| FileMeta {
                    dir: "/data".into(),
                    name: name.into(),
                    size: 0,
                })
                .collect(),
        );
        c
    }
    fn group(source: Option<&str>, mode: DetectionMode) -> DerivedSpectrum {
        let mut params = PipelineParams::default();
        params.import.mode = mode;
        DerivedSpectrum {
            group_id: None,
            id: 0,
            label: "output".into(),
            energy: vec![],
            mu: vec![],
            source: source.map(Into::into),
            params: Some(params),
            quantity: Default::default(),
            quantity_unconfirmed: false,
            operation: None,
        }
    }
    fn channels() -> Vec<DerivedSpectrum> {
        ["/data/a.dat", "/data/b.dat"]
            .into_iter()
            .flat_map(|p| {
                [
                    group(Some(p), DetectionMode::Fluorescence),
                    group(Some(p), DetectionMode::Reference),
                ]
            })
            .collect()
    }
    #[test]
    fn kinds_distinguish_channels_and_persisted_tool_outputs() {
        use crate::params::{Operation, Quantity};
        assert_eq!(kind(DetectionMode::Auto, None), "");
        assert_eq!(kind(DetectionMode::Transmission, None), "Trans");
        assert_eq!(kind(DetectionMode::Fluorescence, None), "Fluo");
        assert_eq!(kind(DetectionMode::Reference, None), "Ref");
        assert_eq!(kind(DetectionMode::MuColumn, None), "μ");
        let mut result = group(None, DetectionMode::Auto);
        result.quantity = Quantity::NormalizedDifference;
        assert_eq!(kind(DetectionMode::Auto, Some(&result)), "Diff");
        for (tool, expected) in [
            ("merge", "Merge"),
            ("Difference spectrum", "Diff"),
            ("Align to reference", "Align"),
            ("Calibrate energy", "Calib"),
        ] {
            result.operation = Some(Operation {
                tool: tool.into(),
                parameters: serde_json::Value::Null,
                inputs: vec![],
                applied_energy_shift_ev: 0.,
            });
            assert_eq!(kind(DetectionMode::Auto, Some(&result)), expected);
        }
    }

    #[test]
    fn colour_survives_identity_serialization_and_display_reordering() {
        let id = crate::group_identity::GroupId::source(
            std::path::Path::new("/data/a.dat"),
            DetectionMode::Reference,
        );
        let restored = serde_json::from_str(&serde_json::to_string(&id).unwrap()).unwrap();
        assert_eq!(color_index(&id), color_index(&restored));
        assert!(color_index(&id) < 8);
    }
    #[test]
    fn two_source_stacks_and_more_than_six_results_share_one_order() {
        let mut d = channels();
        d.extend((0..9).map(|_| group(None, DetectionMode::Auto)));
        let rows = build_rows(&catalog(), &d, &BTreeSet::from([0, 1]), None, "");
        assert_eq!(rows.len(), 16);
        assert_eq!(
            rows[1],
            Row::Child {
                group: DERIVED_BASE,
                parent: 0
            }
        );
        assert_eq!(
            rows[5],
            Row::Child {
                group: DERIVED_BASE + 3,
                parent: 1
            }
        );
        assert_eq!(rows[6], Row::Header(Section::Results));
        assert_eq!(
            rows[15],
            Row::Result {
                group: DERIVED_BASE + 12
            }
        );
    }
    #[test]
    fn collapse_counts_children_and_does_not_emit_them() {
        let rows = build_rows(&catalog(), &channels(), &BTreeSet::new(), None, "");
        assert_eq!(
            rows,
            vec![
                Row::Primary {
                    group: 0,
                    expanded: false,
                    extra_channels: 2
                },
                Row::Primary {
                    group: 1,
                    expanded: false,
                    extra_channels: 2
                }
            ]
        );
    }
    #[test]
    fn filtering_reveals_matching_children_and_ignores_stale_indices() {
        let rows = build_rows(
            &catalog(),
            &channels(),
            &BTreeSet::new(),
            Some(&[999]),
            "reference",
        );
        assert_eq!(rows.len(), 4);
        assert_eq!(
            rows[1],
            Row::Child {
                group: DERIVED_BASE + 1,
                parent: 0
            }
        );
        assert!(
            build_rows(
                &catalog(),
                &channels(),
                &BTreeSet::new(),
                Some(&[]),
                "absent"
            )
            .is_empty()
        );
        let rows = build_rows(
            &catalog(),
            &channels(),
            &BTreeSet::from([0]),
            Some(&[0]),
            "a.dat",
        );
        assert_eq!(rows.len(), 3);
    }
    #[test]
    fn orphan_channels_form_a_source_stack_not_results() {
        let d = channels();
        let rows = build_rows(
            &Catalog::default(),
            &d,
            &BTreeSet::from([DERIVED_BASE]),
            None,
            "",
        );
        assert_eq!(rows.len(), 3);
        assert_eq!(
            rows[1],
            Row::Child {
                group: DERIVED_BASE + 1,
                parent: DERIVED_BASE
            }
        );
    }
    #[test]
    fn navigation_and_ranges_follow_display_order_and_skip_headers() {
        let rows = build_rows(
            &catalog(),
            &[group(None, DetectionMode::Auto)],
            &BTreeSet::new(),
            None,
            "",
        );
        assert_eq!(rows.len(), 4);
        let rows = super::build_rows(
            &catalog(),
            &[group(None, DetectionMode::Auto)],
            &BTreeSet::new(),
            None,
            "",
            None,
        );
        assert_eq!(rows.neighbor(Some(1), 1), Some(DERIVED_BASE));
        assert_eq!(rows.neighbor(Some(DERIVED_BASE), -1), Some(1));
        assert_eq!(rows.neighbor(Some(0), -1), None);
        assert_eq!(rows.range(Some(0), DERIVED_BASE), &[0, 1, DERIVED_BASE]);
        assert_eq!(rows.range(Some(999), 1), &[1]);
    }
    #[test]
    fn million_catalog_primaries_remain_virtual_and_filters_are_shared() {
        use std::sync::Arc;
        let mut catalog = Catalog::default();
        catalog.extend(
            (0..1_000_000)
                .map(|i| FileMeta {
                    dir: Arc::from("/scan"),
                    name: format!("{i:07}.dat").into_boxed_str(),
                    size: 1,
                })
                .collect(),
        );
        let derived = [
            group(Some("/scan/0000000.dat"), DetectionMode::Reference),
            group(Some("/scan/0999998.dat"), DetectionMode::Reference),
            group(None, DetectionMode::Auto),
        ];
        let expanded = BTreeSet::from([0, 999_998]);
        let rows = super::build_rows(&catalog, &derived, &expanded, None, "", None);
        assert_eq!(rows.row_count(), 1_000_004);
        assert_eq!(rows.primaries.len(), 2);
        assert_eq!(rows.inserted.len(), 4);
        assert_eq!(rows.row_index(999_999), Some(1_000_001));
        assert_eq!(rows.row_at(1_000_001).and_then(Row::group), Some(999_999));
        assert_eq!(rows.neighbor(Some(999_999), 1), Some(DERIVED_BASE + 2));
        assert_eq!(
            rows.range(Some(999_997), 999_999),
            vec![999_997, 999_998, DERIVED_BASE + 1, 999_999]
        );
        assert_eq!(rows.row_at(rows.row_count()), None);
        let filtered = Arc::new((0..1_000_000).step_by(2).collect::<Vec<_>>());
        let rows = super::build_rows(
            &catalog,
            &derived,
            &expanded,
            Some(filtered.clone()),
            "",
            None,
        );
        assert!(Arc::ptr_eq(rows.filtered.as_ref().unwrap(), &filtered));
        assert_eq!(rows.row_count(), 500_004);
        assert_eq!(rows.inserted.len(), 4);
        assert_eq!(rows.row_index(999_999), None);
        assert_eq!(rows.row_index(999_998), Some(500_000));
        assert_eq!(rows.neighbor(Some(999_998), 1), Some(DERIVED_BASE + 1));
    }

    #[test]
    fn sparse_mapping_round_trips_filters_stacks_orphans_and_results() {
        let mut derived = channels();
        derived.push(group(Some("/elsewhere/a.dat"), DetectionMode::Transmission));
        derived.push(group(Some("/elsewhere/a.dat"), DetectionMode::Reference));
        derived.push(group(None, DetectionMode::Auto));
        for filter in [
            None,
            Some(vec![]),
            Some(vec![0]),
            Some(vec![1]),
            Some(vec![0, 1, 999]),
        ] {
            for query in ["", "reference", "absent"] {
                for expanded in [BTreeSet::new(), BTreeSet::from([0, 1, DERIVED_BASE + 4])] {
                    let rows = super::build_rows(
                        &catalog(),
                        &derived,
                        &expanded,
                        filter.clone().map(std::sync::Arc::new),
                        query,
                        None,
                    );
                    let groups = (0..rows.row_count())
                        .filter_map(|i| {
                            let row = rows.row_at(i).expect("every visible row maps");
                            row.group().inspect(|&g| {
                                assert_eq!(rows.row_index(g), Some(i));
                            })
                        })
                        .collect::<Vec<_>>();
                    for (i, &g) in groups.iter().enumerate() {
                        assert_eq!(rows.neighbor(Some(g), 1), groups.get(i + 1).copied());
                        assert_eq!(
                            rows.neighbor(Some(g), -1),
                            i.checked_sub(1).map(|i| groups[i])
                        );
                    }
                    if let Some(&last) = groups.last() {
                        assert_eq!(rows.range(groups.first().copied(), last), groups);
                    }
                }
            }
        }
    }

    #[test]
    fn standalone_primary_survives_results_and_owns_its_channels() {
        use super::super::NO_ENTRY;
        let mut derived = vec![group(Some("/solo/scan.dat"), DetectionMode::Reference)];
        derived.extend((0..9).map(|_| group(None, DetectionMode::Auto)));
        let catalog = Catalog::default();
        let path = std::path::Path::new("/solo/scan.dat");
        let rows = super::build_rows(
            &catalog,
            &derived,
            &BTreeSet::from([NO_ENTRY]),
            None,
            "",
            Some(path),
        );
        assert_eq!(rows.row_count(), 12);
        assert_eq!(
            rows.row_at(0),
            Some(Row::Primary {
                group: NO_ENTRY,
                expanded: true,
                extra_channels: 1
            })
        );
        assert_eq!(
            rows.row_at(1),
            Some(Row::Child {
                group: DERIVED_BASE,
                parent: NO_ENTRY
            })
        );
        assert_eq!(rows.row_at(2), Some(Row::Header(Section::Results)));
        assert_eq!(rows.row_index(NO_ENTRY), Some(0));
        assert_eq!(rows.neighbor(Some(DERIVED_BASE), -1), Some(NO_ENTRY));
        assert_eq!(rows.neighbor(Some(NO_ENTRY), 1), Some(DERIVED_BASE));
        let rows = super::build_rows(&catalog, &derived, &BTreeSet::new(), None, "", Some(path));
        assert_eq!(rows.row_count(), 11);
        assert_eq!(rows.neighbor(Some(DERIVED_BASE + 1), -1), Some(NO_ENTRY));
        let rows = super::build_rows(
            &catalog,
            &derived,
            &BTreeSet::new(),
            None,
            "reference",
            Some(path),
        );
        assert_eq!(rows.row_count(), 2);
        assert_eq!(
            rows.row_at(1),
            Some(Row::Child {
                group: DERIVED_BASE,
                parent: NO_ENTRY
            })
        );
        let rows = super::build_rows(
            &catalog,
            &derived,
            &BTreeSet::new(),
            None,
            "absent",
            Some(path),
        );
        assert_eq!(rows.row_count(), 1);
        assert_eq!(rows.row_at(0).and_then(Row::group), Some(NO_ENTRY));
    }

    #[test]
    fn overlay_collisions_use_distinct_compare_positions_up_to_palette_size() {
        assert_eq!(overlay_colors(&[3, 6, 1]), vec![3, 6, 1]);
        assert_eq!(overlay_colors(&[3, 3]), vec![0, 1]);
        assert_eq!(overlay_colors(&[7; 8]), (0..8).collect::<Vec<_>>());
        assert_eq!(overlay_colors(&[7; 9]), vec![0, 1, 2, 3, 4, 5, 6, 7, 0]);
        assert!(overlay_colors(&[]).is_empty());
    }

    #[test]
    fn diagnostics_use_identity_fingerprint_and_attempt_not_historical_labels() {
        use super::super::{JobError, ProblemSeverity, push_problem};
        use crate::group_identity::GroupId;
        let first = GroupId::source(std::path::Path::new("/a/scan.dat"), DetectionMode::Auto);
        let second = GroupId::source(std::path::Path::new("/b/scan.dat"), DetectionMode::Auto);
        let channel = GroupId::source(
            std::path::Path::new("/a/scan.dat"),
            DetectionMode::Reference,
        );
        let error = JobError {
            severity: ProblemSeverity::Error,
            label: "scan.dat".into(),
            message: "failed".into(),
        };
        let mut history = Vec::new();
        push_problem(&mut history, error.clone());
        let mut diagnostics = Diagnostics::default();
        let failed = diagnostics.begin(first.clone(), 10);
        diagnostics.finish(&failed, vec![error.clone()]);
        assert_eq!(diagnostics.get(&first, 10), std::slice::from_ref(&error));
        let parser_warning =
            JobError::warning(std::path::Path::new("/a/scan.dat"), "parser warning".into());
        diagnostics.set_warnings(first.clone(), 10, vec![parser_warning.clone()]);
        assert_eq!(
            diagnostics.get(&first, 10),
            &[error.clone(), parser_warning]
        );
        diagnostics.set_warnings(first.clone(), 10, vec![]);
        assert_eq!(diagnostics.get(&first, 10), std::slice::from_ref(&error));
        assert!(diagnostics.get(&second, 10).is_empty());
        assert!(diagnostics.get(&channel, 10).is_empty());
        assert!(diagnostics.get(&first, 11).is_empty());
        let success = diagnostics.begin(first.clone(), 11);
        diagnostics.finish(&success, vec![]);
        diagnostics.finish(&failed, vec![error.clone()]);
        assert!(diagnostics.get(&first, 11).is_empty());
        let older = diagnostics.begin(first.clone(), 11);
        let newer = diagnostics.begin(first.clone(), 11);
        let warning =
            JobError::warning(std::path::Path::new("/a/scan.dat"), "parser warning".into());
        diagnostics.finish(&newer, vec![warning.clone()]);
        diagnostics.finish(&older, vec![error.clone()]);
        assert_eq!(diagnostics.get(&first, 11), &[warning]);
        let retry = diagnostics.begin(first.clone(), 11);
        diagnostics.finish(&retry, vec![]);
        assert!(diagnostics.get(&first, 11).is_empty());
        assert_eq!(
            history,
            vec![error],
            "Problems history survives successful processing"
        );
    }
}
