//! Pipeline parameters edited in the context panel. `None` = let the core
//! library auto-determine ("auto" in the UI). The fingerprint keys the
//! processed-spectrum cache so edits invalidate exactly what they change.

use rexafs::prelude::AUTOBKClampScalePolicy;
use std::hash::{Hash, Hasher};
use std::io::Read;

use rexafs::prelude::*;
use rexafs::xafs::background::AUTOBK;
use rexafs::xafs::normalization::PrePostEdge;
use serde::{Deserialize, Serialize};

/// How the measured intensities turn into mu(E).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum DetectionMode {
    /// Infer the mode from named columns in each file.
    #[default]
    Auto,
    /// mu = ln(I0/It)
    Transmission,
    /// mu = sum(ROI columns)/I0
    Fluorescence,
    /// mu = ln(It/Ir) — the reference foil between It and Ir.
    Reference,
    /// A file column already contains mu(E).
    MuColumn,
}

impl DetectionMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Auto => "Auto",
            Self::Transmission => "Transmission",
            Self::Fluorescence => "Fluorescence",
            Self::Reference => "Reference",
            Self::MuColumn => "μ column",
        }
    }
}

/// Configure-once import applied to every file in the catalog. `None` and
/// [`DetectionMode::Auto`] resolve independently from each file's content.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ImportConfig {
    pub mode: DetectionMode,
    pub energy_col: Option<usize>,
    pub i0_col: Option<usize>,
    pub it_col: Option<usize>,
    pub ir_col: Option<usize>,
    /// Fluorescence ROI columns (e.g. SDD elements); their sum is If.
    pub fluor_cols: Option<Vec<usize>>,
    /// Precomputed mu(E), used by [`DetectionMode::MuColumn`].
    pub mu_col: Option<usize>,
}

impl Default for ImportConfig {
    fn default() -> Self {
        Self {
            mode: DetectionMode::Auto,
            energy_col: None,
            i0_col: None,
            it_col: None,
            ir_col: None,
            fluor_cols: None,
            mu_col: None,
        }
    }
}

/// ROI columns are a set; canonical order also keeps persisted edits stable.
fn unique_columns(mut columns: Vec<usize>) -> Vec<usize> {
    columns.sort_unstable();
    columns.dedup();
    columns
}

impl ImportConfig {
    /// None selects Auto. A first manual click edits the resolved automatic set.
    pub(crate) fn toggle_fluor(
        &mut self,
        column: Option<usize>,
        auto: Option<&[usize]>,
    ) -> Result<(), String> {
        self.fluor_cols = match column {
            None => None,
            Some(column) => {
                let seed = self.fluor_cols.as_deref().or(auto).ok_or_else(|| {
                    "ROI columns are not available yet; wait for the source preview or type a column list.".to_string()
                })?;
                let mut columns = unique_columns(seed.to_vec());
                if columns.contains(&column) {
                    columns.retain(|&c| c != column);
                } else {
                    columns.push(column);
                }
                Some(unique_columns(columns))
            }
        };
        Ok(())
    }
}

/// File-derived import assignments after applying any manual overrides.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedImport {
    pub mode: DetectionMode,
    pub energy_col: usize,
    pub i0_col: usize,
    pub it_col: usize,
    pub ir_col: usize,
    pub fluor_cols: Vec<usize>,
    pub mu_col: Option<usize>,
}

/// Count every occurrence while retaining bounded, 1-based source locations.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DiagnosticCategory {
    pub count: usize,
    pub examples: Vec<usize>,
}

impl DiagnosticCategory {
    fn record(&mut self, line: Option<usize>) {
        self.count += 1;
        if let Some(line) = line
            && self.examples.len() < 5
            && !self.examples.contains(&line)
        {
            self.examples.push(line);
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ParserDiagnostics {
    pub malformed_rows: DiagnosticCategory,
    pub short_rows: DiagnosticCategory,
    pub truncated_wide_rows: DiagnosticCategory,
    pub excluded_signal_points: DiagnosticCategory,
    pub header_units_anomalies: DiagnosticCategory,
    pub valid_points: usize,
}

fn point_count(count: usize) -> String {
    let digits = count.to_string();
    let mut result = String::new();
    for (i, digit) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            result.push(',');
        }
        result.push(digit);
    }
    result
}

impl ParserDiagnostics {
    pub fn summary(&self) -> String {
        let mut parts = vec![format!("{} points", point_count(self.valid_points))];
        for (count, label) in [
            (
                self.malformed_rows.count + self.short_rows.count,
                "rows skipped",
            ),
            (self.truncated_wide_rows.count, "wide rows truncated"),
            (self.excluded_signal_points.count, "points excluded"),
            (self.header_units_anomalies.count, "header/units anomalies"),
        ] {
            if count > 0 {
                parts.push(format!("{} {label}", point_count(count)));
            }
        }
        parts.join(" · ")
    }

    /// One warning per category. Missing XDI metadata has no source line.
    pub fn warnings(&self) -> Vec<String> {
        [
            (&self.malformed_rows, "malformed rows skipped"),
            (&self.short_rows, "short rows skipped"),
            (&self.truncated_wide_rows, "wide rows truncated"),
            (
                &self.excluded_signal_points,
                "non-finite signal/energy points excluded",
            ),
            (&self.header_units_anomalies, "header/units anomalies"),
        ]
        .into_iter()
        .filter(|(category, _)| category.count > 0)
        .map(|(category, label)| {
            let locations = if category.examples.is_empty() {
                "source line unavailable".into()
            } else {
                format!(
                    "example lines: {}",
                    category
                        .examples
                        .iter()
                        .map(usize::to_string)
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            };
            format!("{} {label} ({locations})", point_count(category.count))
        })
        .collect()
    }
}

/// Raw arrays and their diagnostics travel together, including through caches.
#[derive(Debug, Clone, PartialEq)]
pub struct RawData {
    pub energy: Vec<f64>,
    pub mu: Vec<f64>,
    pub diagnostics: ParserDiagnostics,
}

/// Three original rows and complete diagnostics shown in the Import panel.
#[derive(Clone, Debug, PartialEq)]
pub struct ImportPreview {
    pub column_count: usize,
    pub names: Option<Vec<String>>,
    pub rows: Vec<Vec<f64>>,
    /// Fully automatic assignments, used for role-picker auto labels.
    pub detected: ResolvedImport,
    /// Mode that Auto would choose while retaining current manual columns.
    pub auto_mode: DetectionMode,
    /// Current assignments after applying all manual overrides.
    pub resolved: ResolvedImport,
    /// Original XDI metadata, comments and units for the import inspector.
    pub xdi: Option<XdiHeader>,
    pub diagnostics: ParserDiagnostics,
    /// Mapping/axis errors do not hide the column preview or row diagnostics.
    pub signal_error: Option<String>,
}

/// Prefix-only layout evidence. Deliberately has no row/point diagnostics or
/// signal validation result: even a short source is not validated at intake.
#[derive(Clone, Debug, PartialEq)]
pub struct ImportDetection {
    pub column_count: usize,
    pub names: Option<Vec<String>>,
    pub rows: Vec<Vec<f64>>,
    /// Fully automatic assignments, used for role-picker auto labels.
    pub detected: ResolvedImport,
    /// Mode that Auto would choose while retaining current manual columns.
    pub auto_mode: DetectionMode,
    /// Current assignments after applying all manual overrides.
    pub resolved: ResolvedImport,
    /// Original XDI metadata, comments and units for the import inspector.
    pub xdi: Option<XdiHeader>,
    /// Only mapping errors decidable from the prefix; no signal construction.
    pub mapping_error: Option<String>,
}

impl ImportDetection {
    pub fn available_channels(&self) -> Vec<DetectionMode> {
        available_channels(&self.names, self.resolved.mode)
    }
}

impl ImportPreview {
    pub fn available_channels(&self) -> Vec<DetectionMode> {
        available_channels(&self.names, self.resolved.mode)
    }
}

/// Only named detector channels are offered automatically. Positional
/// fallbacks for unnamed data are not evidence of a reference detector.
fn available_channels(names: &Option<Vec<String>>, mode: DetectionMode) -> Vec<DetectionMode> {
    let Some(names) = names else {
        return vec![mode];
    };
    let named = |synonyms: &[&str]| names.iter().any(|n| name_matches(n, synonyms));
    let mut channels = Vec::new();
    if named(I0_NAMES) && named(IT_NAMES) {
        channels.push(DetectionMode::Transmission);
    }
    if named(I0_NAMES) && names.iter().any(|n| fluorescence_name_matches(n)) {
        channels.push(DetectionMode::Fluorescence);
    }
    if (named(IT_NAMES) && named(IR_NAMES)) || named(REF_MU_NAMES) {
        channels.push(DetectionMode::Reference);
    }
    if named(MU_NAMES) {
        channels.push(DetectionMode::MuColumn);
    }
    if channels.is_empty() {
        channels.push(mode);
    }
    channels
}

#[derive(Debug)]
struct ParsedData {
    names: Option<Vec<String>>,
    rows: Vec<Vec<f64>>,
    xdi: Option<XdiHeader>,
    source_lines: Vec<usize>,
    diagnostics: ParserDiagnostics,
}

#[derive(Debug)]
struct DetectedRoles {
    energy_col: usize,
    i0_col: usize,
    it_col: usize,
    ir_col: usize,
    fluor_cols: Vec<usize>,
    mu_col: Option<usize>,
    named_i0: bool,
    named_it: bool,
    named_ir: bool,
    named_fluor: bool,
}

const ENERGY_NAMES: &[&str] = &["energy", "angle", "e", "mono_e", "energy_ev", "en"];
const I0_NAMES: &[&str] = &["i0", "io", "i_0", "monitor", "mon"];
const IT_NAMES: &[&str] = &["it", "i1", "i_t", "itrans", "trans", "transmission"];
const IR_NAMES: &[&str] = &["ir", "i2", "iref", "irefer", "i_r", "ref", "reference"];
const FLUOR_NAMES: &[&str] = &["iff", "if", "i_f", "fluo", "fluor", "fl", "pips", "ifluor"];
const MU_NAMES: &[&str] = &[
    "mu",
    "xmu",
    "mutrans",
    "mu_t",
    "norm",
    "mufluor",
    "normtrans",
    "normfluor",
];
const REF_MU_NAMES: &[&str] = &["murefer", "normrefer"];

fn name_matches(name: &str, exact: &[&str]) -> bool {
    let name = name.to_ascii_lowercase();
    exact.contains(&name.as_str())
}

fn fluorescence_name_matches(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    FLUOR_NAMES.contains(&name.as_str()) || name.starts_with("sdd") || name.starts_with("roi")
}

fn parse_data(text: &str) -> Result<ParsedData, String> {
    if io::xdi::is_xdi(text) {
        let normalized = text
            .trim_start_matches('\u{feff}')
            .replace("\r\n", "\n")
            .replace('\r', "\n");
        let file = XdiFile::parse(text).map_err(|e| e.to_string())?;
        let mut diagnostics = ParserDiagnostics::default();
        for warning in &file.header.warnings {
            let line = warning
                .strip_prefix("Line ")
                .and_then(|s| s.split_once(':'))
                .and_then(|(n, _)| n.parse().ok());
            diagnostics.header_units_anomalies.record(line);
        }
        for (index, column) in file.header.columns.iter().enumerate() {
            let units = column.units.as_deref().unwrap_or("").to_ascii_lowercase();
            let unsupported = match column.label.to_ascii_lowercase().as_str() {
                "energy" => !matches!(units.as_str(), "ev" | "kev"),
                "angle" => !matches!(
                    units.as_str(),
                    "deg" | "degree" | "degrees" | "rad" | "radian" | "radians"
                ),
                _ => false,
            };
            if unsupported {
                let key = format!("column.{}:", index + 1);
                let line = normalized.lines().enumerate().find_map(|(i, line)| {
                    line.trim()
                        .trim_start_matches('#')
                        .trim_start()
                        .to_ascii_lowercase()
                        .starts_with(&key)
                        .then_some(i + 1)
                });
                diagnostics.header_units_anomalies.record(line);
            }
        }
        // The core XDI parser rejects malformed tables. Every non-comment,
        // nonempty line in a successfully parsed file is a retained data row.
        let source_lines = normalized
            .lines()
            .enumerate()
            .filter_map(|(i, line)| {
                let line = line.trim();
                (!line.is_empty() && !line.starts_with('#')).then_some(i + 1)
            })
            .collect();
        return Ok(ParsedData {
            source_lines,
            diagnostics,
            names: Some(
                file.header
                    .columns
                    .iter()
                    .map(|c| c.label.clone())
                    .collect(),
            ),
            rows: file.data,
            xdi: Some(file.header),
        });
    }
    let mut rows = Vec::new();
    let mut source_lines = Vec::new();
    let mut diagnostics = ParserDiagnostics::default();
    let mut width = 0usize;
    let mut last_header = None;
    for (index, line) in text.lines().enumerate() {
        let line_number = index + 1;
        let line = line.trim();
        if rows.is_empty() && line.starts_with('#') {
            last_header = Some((line_number, line));
            continue;
        }
        if line.is_empty() || line.starts_with('#') || line.starts_with('*') {
            continue;
        }
        let values: Option<Vec<f64>> = line
            .split_whitespace()
            .map(|token| token.parse::<f64>().ok())
            .collect();
        let Some(values) = values else {
            if rows.is_empty() {
                last_header = Some((line_number, line));
            } else {
                diagnostics.malformed_rows.record(Some(line_number));
            }
            continue;
        };
        if values.is_empty() {
            continue;
        }
        if rows.is_empty() {
            width = values.len();
        }
        if values.len() >= width {
            if values.len() > width {
                diagnostics.truncated_wide_rows.record(Some(line_number));
            }
            rows.push(values[..width].to_vec());
            source_lines.push(line_number);
        } else {
            diagnostics.short_rows.record(Some(line_number));
        }
    }
    if rows.is_empty() {
        return Err("no numeric data rows".into());
    }
    let names = last_header.and_then(|(line_number, line)| {
        let names = line
            .trim_start_matches('#')
            .split(|c: char| c.is_whitespace() || c == ',')
            .filter(|token| !token.is_empty())
            .map(str::to_string)
            .collect::<Vec<_>>();
        // Separators and metadata labels ("----", "Data:") are not name rows.
        let looks_like_names = names.len() > 1
            && names.iter().all(|name| {
                name.parse::<f64>().is_err()
                    && name.chars().any(char::is_alphabetic)
                    && name
                        .chars()
                        .all(|c| c.is_alphanumeric() || "_-./()[]".contains(c))
            });
        if !looks_like_names {
            return None;
        }
        if names.len() != width {
            diagnostics.header_units_anomalies.record(Some(line_number));
        }
        (names.len() == width).then_some(names)
    });
    Ok(ParsedData {
        names,
        rows,
        xdi: None,
        source_lines,
        diagnostics,
    })
}

fn parse_file_data(text: &str, path: &std::path::Path) -> Result<ParsedData, String> {
    if path
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("xdi"))
        && !io::xdi::is_xdi(text)
    {
        return Err(format!(
            "{}: XDI file is missing its '# XDI/1.0' signature",
            path.display()
        ));
    }
    parse_data(text).map_err(|e| format!("{}: {e}", path.display()))
}

fn monotonic_energy_column(rows: &[Vec<f64>], width: usize) -> Option<usize> {
    (0..width).find(|&column| {
        let values = rows.iter().map(|row| row[column]).collect::<Vec<_>>();
        let increasing = values.windows(2).all(|pair| pair[1] > pair[0]);
        let decreasing = values.windows(2).all(|pair| pair[1] < pair[0]);
        let span = (values[values.len() - 1] - values[0]).abs();
        values.iter().all(|value| value.is_finite())
            && (increasing || decreasing)
            && (10.0..=10_000_000.0).contains(&span)
            && values.iter().any(|value| value.abs() >= 100.0)
    })
}

fn detect_roles(data: &ParsedData) -> DetectedRoles {
    let width = data.rows[0].len();
    let find = |synonyms: &[&str]| {
        data.names
            .as_ref()
            .and_then(|names| names.iter().position(|name| name_matches(name, synonyms)))
    };
    let named_energy = find(ENERGY_NAMES);
    let named_i0 = find(I0_NAMES);
    let named_it = find(IT_NAMES);
    let named_ir = find(IR_NAMES);
    let fluor_cols = data
        .names
        .as_ref()
        .map(|names| {
            names
                .iter()
                .enumerate()
                .filter_map(|(column, name)| fluorescence_name_matches(name).then_some(column))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let has_sample_intensities =
        named_i0.is_some() && (named_it.is_some() || !fluor_cols.is_empty());
    let mu_col = find(MU_NAMES).or_else(|| {
        if has_sample_intensities {
            None
        } else {
            find(REF_MU_NAMES)
        }
    });
    let energy_col = named_energy
        .or_else(|| {
            data.names
                .is_none()
                .then(|| monotonic_energy_column(&data.rows, width))
                .flatten()
        })
        .unwrap_or(0);
    DetectedRoles {
        energy_col,
        i0_col: named_i0.unwrap_or(1),
        it_col: named_it.unwrap_or(2),
        ir_col: named_ir.unwrap_or(3),
        fluor_cols: if fluor_cols.is_empty() {
            vec![4]
        } else {
            fluor_cols.clone()
        },
        mu_col,
        named_i0: named_i0.is_some(),
        named_it: named_it.is_some(),
        named_ir: named_ir.is_some(),
        named_fluor: !fluor_cols.is_empty(),
    }
}

fn resolve_import(data: &ParsedData, import: &ImportConfig) -> ResolvedImport {
    let detected = detect_roles(data);
    let mode = match import.mode {
        DetectionMode::Auto if import.mu_col.is_some() || detected.mu_col.is_some() => {
            DetectionMode::MuColumn
        }
        DetectionMode::Auto
            if (import.i0_col.is_some() && import.it_col.is_some())
                || (detected.named_i0 && detected.named_it) =>
        {
            DetectionMode::Transmission
        }
        DetectionMode::Auto
            if (import.i0_col.is_some() && import.fluor_cols.is_some())
                || (detected.named_i0 && detected.named_fluor) =>
        {
            DetectionMode::Fluorescence
        }
        DetectionMode::Auto if detected.named_it && detected.named_ir => DetectionMode::Reference,
        DetectionMode::Auto => DetectionMode::Transmission,
        mode => mode,
    };
    ResolvedImport {
        mode,
        energy_col: import.energy_col.unwrap_or(detected.energy_col),
        i0_col: import.i0_col.unwrap_or(detected.i0_col),
        it_col: import.it_col.unwrap_or(detected.it_col),
        ir_col: import.ir_col.unwrap_or(detected.ir_col),
        fluor_cols: unique_columns(import.fluor_cols.clone().unwrap_or(detected.fluor_cols)),
        mu_col: import.mu_col.or(detected.mu_col),
    }
}

/// Parse a column list like "4, 6-8" (commas/spaces, inclusive ranges).
pub fn parse_cols(text: &str) -> Option<Vec<usize>> {
    let mut out = Vec::new();
    for token in text.split([',', ' ']).filter(|t| !t.is_empty()) {
        match token.split_once('-') {
            Some((a, b)) => {
                let (a, b) = (
                    a.trim().parse::<usize>().ok()?,
                    b.trim().parse::<usize>().ok()?,
                );
                if a > b {
                    return None;
                }
                out.extend(a..=b);
            }
            None => out.push(token.trim().parse::<usize>().ok()?),
        }
    }
    (!out.is_empty()).then(|| unique_columns(out))
}

const DETECTION_BYTES: u64 = 64 * 1024;

/// Detect layout from at most 64 KiB. Full diagnostics and signal validation
/// belong to preview_import/load_mu_with_diagnostics, never to intake.
/// None means the prefix ended before data; channel detection is undetermined.
pub fn detect_import(
    path: &std::path::Path,
    import: &ImportConfig,
) -> Result<Option<ImportDetection>, String> {
    let file = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    detect_import_reader(file, path, import)
}

fn detect_import_reader(
    reader: impl Read,
    path: &std::path::Path,
    import: &ImportConfig,
) -> Result<Option<ImportDetection>, String> {
    let mut bytes = Vec::new();
    reader
        .take(DETECTION_BYTES)
        .read_to_end(&mut bytes)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    let exhausted = bytes.len() == DETECTION_BYTES as usize;
    if exhausted {
        // Do not probe one byte past the limit, or parse a cut row/token/UTF-8
        // sequence. Keep a complete final line if the bound ends at a newline.
        let end = bytes
            .iter()
            .rposition(|b| matches!(b, b'\n' | b'\r'))
            .map_or(0, |i| i + 1);
        bytes.truncate(end);
    }
    let text = std::str::from_utf8(&bytes).map_err(|e| {
        format!(
            "{}: stream did not contain valid UTF-8: {e}",
            path.display()
        )
    })?;
    // A single cut line leaves no evidence, including an XDI signature.
    if exhausted && text.is_empty() {
        return Ok(None);
    }
    let data = match parse_file_data(text, path) {
        Ok(data) => data,
        Err(error)
            if exhausted
                && [
                    "no numeric data rows",
                    "XDI: no numeric data rows",
                    "XDI: missing '# ---' header-end separator",
                ]
                .iter()
                .any(|message| error == format!("{}: {message}", path.display())) =>
        {
            // These parser failures mean the prefix has no data yet. Other
            // failures (including malformed XDI rows) remain intake errors.
            return Ok(None);
        }
        Err(error) => return Err(error),
    };
    Ok(Some(import_detection(&data, path, import)))
}

fn import_detection(
    data: &ParsedData,
    path: &std::path::Path,
    import: &ImportConfig,
) -> ImportDetection {
    let detected = resolve_import(data, &ImportConfig::default());
    let mut auto_import = import.clone();
    auto_import.mode = DetectionMode::Auto;
    let auto_mode = resolve_import(data, &auto_import).mode;
    let resolved = resolve_import(data, import);
    let mapping_error = validate_mapping(data, path, import, &resolved).err();
    ImportDetection {
        column_count: data.rows[0].len(),
        names: data.names.clone(),
        rows: data.rows.iter().take(3).cloned().collect(),
        detected,
        auto_mode,
        resolved,
        xdi: data.xdi.clone(),
        mapping_error,
    }
}

/// Column metadata, three original rows, and full-source diagnostics. Runs on
/// the background executor; totals must include rows beyond a header prefix.
pub fn preview_import(
    path: &std::path::Path,
    import: &ImportConfig,
) -> Result<ImportPreview, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut data = parse_file_data(&text, path)?;
    let detection = import_detection(&data, path, import);
    let signal_error = construct_mu(&mut data, path, import).err();
    Ok(ImportPreview {
        diagnostics: data.diagnostics,
        signal_error,
        column_count: detection.column_count,
        names: detection.names,
        rows: detection.rows,
        detected: detection.detected,
        auto_mode: detection.auto_mode,
        resolved: detection.resolved,
        xdi: detection.xdi,
    })
}

/// Energy and mu(E) for one file under the import configuration. Rows whose
/// math is non-finite (e.g. log of a non-positive ratio) are dropped. Energy
/// and mu are sorted together, including descending monochromator-angle scans.
pub fn load_mu(
    path: &std::path::Path,
    import: &ImportConfig,
) -> Result<(Vec<f64>, Vec<f64>), String> {
    let raw = load_mu_with_diagnostics(path, import)?;
    Ok((raw.energy, raw.mu))
}

pub fn load_mu_with_diagnostics(
    path: &std::path::Path,
    import: &ImportConfig,
) -> Result<RawData, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut data = parse_file_data(&text, path)?;
    let (energy, mu) = construct_mu(&mut data, path, import)?;
    Ok(RawData {
        energy,
        mu,
        diagnostics: data.diagnostics,
    })
}

/// Validate only column assignments, independently of finite point counts or
/// axis conversion. The optional return value is a named reference-mu column.
fn validate_mapping(
    data: &ParsedData,
    path: &std::path::Path,
    import: &ImportConfig,
    resolved: &ResolvedImport,
) -> Result<Option<usize>, String> {
    let ref_mu_col = data
        .names
        .as_ref()
        .and_then(|names| names.iter().position(|n| name_matches(n, REF_MU_NAMES)))
        .filter(|_| import.it_col.is_none() && import.ir_col.is_none());
    let rows = &data.rows;
    let width = rows[0].len();
    let need = |col: usize, name: &str| -> Result<usize, String> {
        if col < width {
            Ok(col)
        } else {
            Err(format!(
                "{}: {name} column {col} out of range (file has {width} columns)",
                path.display()
            ))
        }
    };
    need(resolved.energy_col, "energy")?;
    match resolved.mode {
        DetectionMode::Auto => unreachable!("auto mode is resolved before mapping validation"),
        DetectionMode::Transmission => {
            need(resolved.i0_col, "I0")?;
            need(resolved.it_col, "It")?;
        }
        DetectionMode::Fluorescence => {
            need(resolved.i0_col, "I0")?;
            for &col in &resolved.fluor_cols {
                need(col, "ROI")?;
            }
            if resolved.fluor_cols.is_empty() {
                return Err("no fluorescence ROI columns configured".into());
            }
        }
        DetectionMode::Reference if ref_mu_col.is_none() => {
            need(resolved.it_col, "It")?;
            need(resolved.ir_col, "Ir")?;
        }
        DetectionMode::Reference => {}
        DetectionMode::MuColumn => {
            let col = resolved
                .mu_col
                .ok_or_else(|| format!("{}: no precomputed mu column detected", path.display()))?;
            need(col, "mu")?;
        }
    }
    Ok(ref_mu_col)
}

fn construct_mu(
    data: &mut ParsedData,
    path: &std::path::Path,
    import: &ImportConfig,
) -> Result<(Vec<f64>, Vec<f64>), String> {
    let resolved = resolve_import(data, import);
    if let Some(header) = &data.xdi {
        for row in &mut data.rows {
            let value = row
                .get_mut(resolved.energy_col)
                .ok_or_else(|| format!("{}: energy column out of range", path.display()))?;
            *value = header
                .energy_ev(resolved.energy_col, *value)
                .map_err(|e| format!("{}: {e}", path.display()))?;
        }
    }
    let ref_mu_col = validate_mapping(data, path, import, &resolved)?;
    let rows = &data.rows;
    let e = resolved.energy_col;
    let mut energy = Vec::with_capacity(rows.len());
    let mut mu = Vec::with_capacity(rows.len());
    match resolved.mode {
        DetectionMode::Auto => unreachable!("auto mode is resolved before import math"),
        DetectionMode::Transmission => {
            let (i0, it) = (resolved.i0_col, resolved.it_col);
            for (row, &line) in rows.iter().zip(&data.source_lines) {
                let m = (row[i0] / row[it]).ln();
                if m.is_finite() && row[e].is_finite() {
                    energy.push(row[e]);
                    mu.push(m);
                } else {
                    data.diagnostics.excluded_signal_points.record(Some(line));
                }
            }
        }
        DetectionMode::Fluorescence => {
            let i0 = resolved.i0_col;
            let cols = &resolved.fluor_cols;
            for (row, &line) in rows.iter().zip(&data.source_lines) {
                let m = cols.iter().map(|&c| row[c]).sum::<f64>() / row[i0];
                if m.is_finite() && row[e].is_finite() {
                    energy.push(row[e]);
                    mu.push(m);
                } else {
                    data.diagnostics.excluded_signal_points.record(Some(line));
                }
            }
        }
        DetectionMode::Reference => {
            if let Some(col) = ref_mu_col {
                for (row, &line) in rows.iter().zip(&data.source_lines) {
                    if row[col].is_finite() && row[e].is_finite() {
                        energy.push(row[e]);
                        mu.push(row[col]);
                    } else {
                        data.diagnostics.excluded_signal_points.record(Some(line));
                    }
                }
            } else {
                let (it, ir) = (resolved.it_col, resolved.ir_col);
                for (row, &line) in rows.iter().zip(&data.source_lines) {
                    let m = (row[it] / row[ir]).ln();
                    if m.is_finite() && row[e].is_finite() {
                        energy.push(row[e]);
                        mu.push(m);
                    } else {
                        data.diagnostics.excluded_signal_points.record(Some(line));
                    }
                }
            }
        }
        DetectionMode::MuColumn => {
            let mu_col = resolved
                .mu_col
                .ok_or_else(|| format!("{}: no precomputed mu column detected", path.display()))?;
            for (row, &line) in rows.iter().zip(&data.source_lines) {
                if row[mu_col].is_finite() && row[e].is_finite() {
                    energy.push(row[e]);
                    mu.push(row[mu_col]);
                } else {
                    data.diagnostics.excluded_signal_points.record(Some(line));
                }
            }
        }
    }
    data.diagnostics.valid_points = energy.len();
    if energy.len() < 2 {
        return Err(format!(
            "{}: fewer than 2 finite data points",
            path.display()
        ));
    }
    if energy.windows(2).any(|pair| pair[0] > pair[1]) {
        let mut paired = energy.into_iter().zip(mu).collect::<Vec<_>>();
        paired.sort_by(|a, b| a.0.total_cmp(&b.0));
        (energy, mu) = paired.into_iter().unzip();
    }
    Ok((energy, mu))
}

/// Unweighted standard χ(k), embedded in the project for reproducibility.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct ChiStandard {
    pub label: String,
    pub k: Vec<f64>,
    pub chi: Vec<f64>,
}
impl ChiStandard {
    pub fn validate(&self) -> Result<(), String> {
        if self.k.len() < 2
            || self.k.len() != self.chi.len()
            || self.k.iter().any(|k| !k.is_finite() || *k < 0.)
            || self.chi.iter().any(|v| !v.is_finite())
            || self.k.windows(2).any(|k| k[0] >= k[1])
        {
            return Err("Standard χ(k) needs matching finite arrays and a strictly increasing, nonnegative k grid.".into());
        }
        Ok(())
    }
    pub fn load(path: &std::path::Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        let mut rows = Vec::new();
        for (line_number, line) in text.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with(['#', '*']) {
                continue;
            }
            let values = line
                .split(|c: char| c.is_whitespace() || c == ',')
                .filter(|s| !s.is_empty())
                .map(str::parse::<f64>)
                .collect::<Result<Vec<_>, _>>();
            match values {
                Ok(row) if row.len() == 2 => rows.push(row),
                _ => {
                    return Err(format!(
                        "Line {}: expected exactly two numeric columns, k (Å⁻¹) and unweighted χ(k). Use # for header comments.",
                        line_number + 1
                    ));
                }
            }
        }
        let standard = Self {
            label: path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
            k: rows.iter().map(|r| r[0]).collect(),
            chi: rows.iter().map(|r| r[1]).collect(),
        };
        standard.validate()?;
        Ok(standard)
    }
}

#[derive(Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PipelineParams {
    pub import: ImportConfig,
    /// Shift each spectrum's energy axis so its reference-channel E0 lands
    /// on `align_target` (requires an Ir column; no-op when target unset).
    pub align_to_ref: bool,
    pub align_target: Option<f64>,
    // Normalization (pre/post-edge); energies relative to E0.
    pub e0: Option<f64>,
    /// Positive measured edge-step override; None derives it from the fits.
    pub edge_step: Option<f64>,
    pub pre_edge_start: Option<f64>,
    pub pre_edge_end: Option<f64>,
    pub norm_start: Option<f64>,
    pub norm_end: Option<f64>,
    /// Advanced: polynomial order of the post-edge fit.
    pub norm_polyorder: Option<i32>,
    /// Advanced: Victoreen exponent for the pre-edge fit.
    pub n_victoreen: Option<i32>,
    // AUTOBK background.
    pub rbkg: Option<f64>,
    pub bkg_kmin: Option<f64>,
    pub bkg_kmax: Option<f64>,
    // Advanced AUTOBK.
    pub bkg_kstep: Option<f64>,
    pub bkg_nknots: Option<i32>,
    pub bkg_kweight: Option<i32>,
    pub bkg_clamp_lo: Option<i32>,
    pub bkg_clamp_hi: Option<i32>,
    /// Number of points at each active clamp endpoint (default 3).
    pub bkg_nclamp: Option<i32>,
    /// Missing fields in v1 projects retain the historical clamp model.
    #[serde(default = "legacy_bkg_clamp_policy")]
    pub bkg_clamp_policy: AUTOBKClampScalePolicy,
    /// None selects the fixed penalty default (0.001); zero disables it.
    pub bkg_clamp_lambda: Option<f64>,
    pub bkg_window: Option<FTWindow>,
    pub bkg_dk: Option<f64>,
    pub bkg_solver: Option<AUTOBKSolver>,
    pub bkg_nfft: Option<i32>,
    pub bkg_ek0: Option<f64>,
    pub bkg_linear_regularization: Option<f64>,
    pub bkg_linear_condition_limit: Option<f64>,
    pub bkg_linear_residual_ratio_limit: Option<f64>,
    pub bkg_linear_fallback_to_lm: Option<bool>,
    pub bkg_linear_workspace_cache: Option<bool>,
    pub bkg_linear_fallback_solver: Option<AUTOBKSolver>,
    pub bkg_standard: Option<ChiStandard>,
    // Forward FFT.
    pub fft_kmin: Option<f64>,
    pub fft_kmax: Option<f64>,
    pub fft_dk: Option<f64>,
    pub fft_kweight: Option<f64>,
    // Advanced FFT.
    pub fft_dk2: Option<f64>,
    pub fft_rmax: Option<f64>,
    pub fft_window: Option<FTWindow>,
    pub fft_kstep: Option<f64>,
    pub fft_nfft: Option<i32>,
    // Back FT (R -> q).
    pub bft_rmin: Option<f64>,
    pub bft_rmax: Option<f64>,
    pub bft_dr: Option<f64>,
    pub bft_window: Option<FTWindow>,
    pub bft_qmax: Option<f64>,
    pub bft_dr2: Option<f64>,
    pub bft_rweight: Option<f64>,
    pub bft_kstep: Option<f64>,
    pub bft_nfft: Option<i32>,
}

/// Human label for a window choice (`None` = the core default).
pub fn window_label(window: Option<FTWindow>) -> &'static str {
    match window {
        Some(FTWindow::Hanning) => "Hanning",
        Some(FTWindow::Parzen) => "Parzen",
        Some(FTWindow::Welch) => "Welch",
        Some(FTWindow::Gaussian) => "Gaussian",
        Some(FTWindow::Sine) => "Sine",
        Some(FTWindow::KaiserBessel) => "Kaiser",
        Some(FTWindow::FHanning) => "FHanning",
        None => "Kaiser",
    }
}

/// Cycle order for the window selector chips.
pub const FT_WINDOWS: [FTWindow; 7] = [
    FTWindow::Hanning,
    FTWindow::Parzen,
    FTWindow::Welch,
    FTWindow::Gaussian,
    FTWindow::Sine,
    FTWindow::KaiserBessel,
    FTWindow::FHanning,
];

pub const AUTOBK_SOLVERS: [AUTOBKSolver; 3] = [
    AUTOBKSolver::LinearDirect,
    AUTOBKSolver::TrustRegionDogLeg,
    AUTOBKSolver::LegacyLm,
];

fn legacy_bkg_clamp_policy() -> AUTOBKClampScalePolicy {
    AUTOBKClampScalePolicy::Fixed
}

impl PipelineParams {
    /// Inherit settings for already materialized, energy-shifted arrays.
    /// Pre/post-edge ranges are relative to E0; only the two absolute edge
    /// overrides move. Reference alignment belongs to the input read, not replay.
    pub fn for_materialized(&self, applied_shift_ev: f64) -> Self {
        let mut params = self.clone();
        params.e0 = params.e0.map(|e| e + applied_shift_ev);
        params.bkg_ek0 = params.bkg_ek0.map(|e| e + applied_shift_ev);
        params.align_to_ref = false;
        params.align_target = None;
        params
    }

    pub fn legacy_defaults() -> Self {
        Self {
            bkg_clamp_policy: legacy_bkg_clamp_policy(),
            ..Self::default()
        }
    }

    /// Fingerprint of the import + alignment settings only: two parameter
    /// sets with the same raw fingerprint read the same (energy, mu) from a
    /// file, so the raw arrays can be cached across pipeline-only edits.
    pub fn raw_fingerprint(&self) -> u64 {
        let mut hasher = std::hash::DefaultHasher::new();
        self.hash_raw_fields(&mut hasher);
        hasher.finish()
    }

    fn hash_raw_fields(&self, hasher: &mut std::hash::DefaultHasher) {
        format!("{:?}", self.import.mode).hash(hasher);
        self.align_to_ref.hash(hasher);
        self.align_target.map(f64::to_bits).hash(hasher);
        self.import.energy_col.hash(hasher);
        self.import.i0_col.hash(hasher);
        self.import.it_col.hash(hasher);
        self.import.ir_col.hash(hasher);
        self.import.fluor_cols.hash(hasher);
        self.import.mu_col.hash(hasher);
    }

    pub fn fingerprint(&self) -> u64 {
        let mut hasher = std::hash::DefaultHasher::new();
        self.hash_raw_fields(&mut hasher);
        for v in [
            self.e0,
            self.edge_step,
            self.pre_edge_start,
            self.pre_edge_end,
            self.norm_start,
            self.norm_end,
            self.rbkg,
            self.bkg_kmin,
            self.bkg_kmax,
            self.bkg_kstep,
            self.bkg_dk,
            self.bkg_clamp_lambda,
            self.fft_kmin,
            self.fft_kmax,
            self.fft_dk,
            self.fft_kweight,
            self.fft_dk2,
            self.fft_rmax,
            self.fft_kstep,
            self.bft_rmin,
            self.bft_rmax,
            self.bft_dr,
            self.bkg_ek0,
            self.bkg_linear_regularization,
            self.bkg_linear_condition_limit,
            self.bkg_linear_residual_ratio_limit,
            self.bft_qmax,
            self.bft_dr2,
            self.bft_rweight,
            self.bft_kstep,
        ] {
            v.map(f64::to_bits).hash(&mut hasher);
        }
        for v in [
            self.norm_polyorder,
            self.n_victoreen,
            self.bkg_nknots,
            self.bkg_kweight,
            self.bkg_clamp_lo,
            self.bkg_clamp_hi,
            self.bkg_nclamp,
            self.bkg_nfft,
            self.fft_nfft,
            self.bft_nfft,
        ] {
            v.hash(&mut hasher);
        }
        format!(
            "{:?}|{:?}|{:?}|{:?}",
            self.bkg_window, self.bkg_solver, self.fft_window, self.bft_window
        )
        .hash(&mut hasher);
        format!("{:?}", self.bkg_clamp_policy).hash(&mut hasher);
        self.bkg_linear_fallback_to_lm.hash(&mut hasher);
        self.bkg_linear_workspace_cache.hash(&mut hasher);
        format!("{:?}", self.bkg_linear_fallback_solver).hash(&mut hasher);
        self.bkg_standard.is_some().hash(&mut hasher);
        if let Some(standard) = &self.bkg_standard {
            standard.k.len().hash(&mut hasher);
            for value in standard.k.iter().chain(&standard.chi) {
                value.to_bits().hash(&mut hasher);
            }
        }
        hasher.finish()
    }
}

/// Load a file and run the full pipeline with the given parameters.
/// Runs on the background executor.
pub fn process_file(
    path: &std::path::Path,
    params: &PipelineParams,
) -> Result<XASSpectrum, String> {
    let (energy, mu) = load_raw(path, params)?;
    process_arrays(energy, mu, params)
}

/// Raw (energy, mu) after import math and optional reference alignment —
/// the inputs both processing and merging start from.
pub fn load_raw(
    path: &std::path::Path,
    params: &PipelineParams,
) -> Result<(Vec<f64>, Vec<f64>), String> {
    let raw = load_raw_with_diagnostics(path, params)?;
    Ok((raw.energy, raw.mu))
}

pub fn load_raw_with_diagnostics(
    path: &std::path::Path,
    params: &PipelineParams,
) -> Result<RawData, String> {
    let mut raw = load_mu_with_diagnostics(path, &params.import)?;
    if params.align_to_ref
        && let Some(target) = params.align_target
    {
        let shift = target - reference_e0(path, &params.import)?;
        for e in &mut raw.energy {
            *e += shift;
        }
    }
    Ok(raw)
}

/// Shared uncached access for file channels and materialized results.
pub(crate) fn load_group_raw(
    path: &std::path::Path,
    params: &PipelineParams,
    derived: Option<&DerivedSpectrum>,
) -> Result<(Vec<f64>, Vec<f64>), String> {
    let raw = load_group_raw_with_diagnostics(path, params, derived)?;
    Ok((raw.energy, raw.mu))
}

pub(crate) fn load_group_raw_with_diagnostics(
    path: &std::path::Path,
    params: &PipelineParams,
    derived: Option<&DerivedSpectrum>,
) -> Result<RawData, String> {
    match derived {
        Some(group) => match &group.source {
            Some(source) => load_raw_with_diagnostics(source, params),
            None => Ok(RawData {
                energy: group.energy.clone(),
                mu: group.mu.clone(),
                diagnostics: ParserDiagnostics {
                    valid_points: group.energy.len(),
                    ..Default::default()
                },
            }),
        },
        None => load_raw_with_diagnostics(path, params),
    }
}

/// An additional spectrum: an in-memory result or an independently processed
/// channel of a source file. File channels remain lazy; no raw arrays need to
/// be retained until the group is viewed or analyzed.
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct DerivedSpectrum {
    pub label: String,
    pub energy: Vec<f64>,
    pub mu: Vec<f64>,
    #[serde(default)]
    pub id: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<std::path::PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<PipelineParams>,
    #[serde(default)]
    pub quantity: Quantity,
    /// Legacy materialized arrays have no trustworthy scientific type.
    #[serde(default)]
    pub quantity_unconfirmed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operation: Option<Operation>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Quantity {
    #[default]
    RawMu,
    NormalizedMu,
    NormalizedDifference,
    ChiK,
}

impl Quantity {
    /// Compatibility predicate for raw absorption operations (including Merge).
    pub fn is_absorption(self) -> bool {
        self == Self::RawMu
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::RawMu => "μ(E)",
            Self::NormalizedMu => "μnorm",
            Self::NormalizedDifference => "Δμnorm",
            Self::ChiK => "χ(k)",
        }
    }
}

/// Portable subset of a bound ToolTarget. Session indices/generations are not
/// durable identities; preserve the source path or derived id and revision.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperationInput {
    pub label: String,
    pub path: std::path::PathBuf,
    pub derived_id: Option<u64>,
    pub fingerprint: u64,
    pub size: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Operation {
    pub tool: String,
    pub parameters: serde_json::Value,
    /// Target first, then the explicit standard/baseline, when present.
    pub inputs: Vec<OperationInput>,
    /// Historical correction already baked into energy and inherited E0s.
    /// Loading/reprocessing must never replay it.
    pub applied_energy_shift_ev: f64,
}

impl DerivedSpectrum {
    pub fn fingerprint(&self, params: &PipelineParams) -> u64 {
        let mut hasher = std::hash::DefaultHasher::new();
        params.fingerprint().hash(&mut hasher);
        self.quantity.hash(&mut hasher);
        self.quantity_unconfirmed.hash(&mut hasher);
        hasher.finish()
    }

    pub fn confirm_quantity(&mut self, quantity: Quantity) -> bool {
        if !self.quantity_unconfirmed && self.quantity == quantity {
            return false;
        }
        self.quantity = quantity;
        self.quantity_unconfirmed = false;
        true
    }

    pub fn display_label(&self) -> String {
        let mut label = self.label.clone();
        if !self.quantity.is_absorption() {
            label.push_str(&format!(" · {}", self.quantity.label()));
        }
        if self.quantity_unconfirmed {
            label.push_str(" · quantity unconfirmed");
        }
        label
    }

    pub fn processing_block_reason(&self) -> Option<String> {
        if self.quantity_unconfirmed {
            Some("Quantity unconfirmed: confirm the quantity before normalization/AUTOBK; plotting/export available.".into())
        } else if !self.quantity.is_absorption() {
            Some(format!(
                "{}: normalization/AUTOBK disabled; plotting/export available.",
                self.quantity.label()
            ))
        } else {
            None
        }
    }

    pub fn process(&self, params: &PipelineParams) -> Result<XASSpectrum, String> {
        if let Some(reason) = self.processing_block_reason() {
            return Err(reason);
        }
        let (energy, mu) = self.raw(params)?;
        process_arrays(energy, mu, params)
    }

    /// Display/export bypass for quantities that must not enter the pipeline.
    /// No normalization/background objects are manufactured for differences.
    pub fn for_display(&self, params: &PipelineParams) -> Result<XASSpectrum, String> {
        if self.processing_block_reason().is_none() {
            return self.process(params);
        }
        let (energy, mu) = self.raw(params)?;
        let mut sp = XASSpectrum::new();
        sp.set_name(self.display_label());
        if self.quantity == Quantity::ChiK {
            sp.k = Some(energy.into());
            sp.chi = Some(mu.into());
            // XASSpectrum's plotting getters borrow these buffers from AUTOBK.
            // This is only an array adapter: no background fit is performed,
            // and no normalization, spline or Fourier output is supplied.
            sp.background = Some(BackgroundMethod::AUTOBK(AUTOBK {
                k: sp.k.clone(),
                chi: sp.chi.clone(),
                ..Default::default()
            }));
        } else {
            sp.set_spectrum(energy, mu);
        }
        Ok(sp)
    }
    pub fn raw(&self, params: &PipelineParams) -> Result<(Vec<f64>, Vec<f64>), String> {
        match &self.source {
            Some(source) => load_raw(source, params),
            None => Ok((self.energy.clone(), self.mu.clone())),
        }
    }
}

/// Running average over spectra streamed one at a time, so merging N files
/// needs memory for the accumulator plus a single input — never all N at
/// once. The first spectrum's energy axis hosts a running sum; the overlap
/// window shrinks as inputs arrive and the grid is trimmed to it at the
/// end, which reproduces [`average_spectra`]'s all-at-once result exactly.
pub struct StreamingAverage {
    grid: Vec<f64>,
    sum: Vec<f64>,
    count: usize,
    lo: f64,
    hi: f64,
}

impl StreamingAverage {
    pub fn new(energy: Vec<f64>, mu: Vec<f64>) -> Self {
        let lo = *energy.first().unwrap_or(&f64::MAX);
        let hi = *energy.last().unwrap_or(&f64::MIN);
        Self {
            grid: energy,
            sum: mu,
            count: 1,
            lo,
            hi,
        }
    }

    /// Fold one spectrum (>= 2 points, ascending energy — what `load_raw`
    /// guarantees) into the running sum via clamped linear interpolation.
    pub fn add(&mut self, energy: &[f64], mu: &[f64]) {
        self.lo = self.lo.max(*energy.first().unwrap_or(&f64::MAX));
        self.hi = self.hi.min(*energy.last().unwrap_or(&f64::MIN));
        let mut j = 0usize;
        for (gi, &g) in self.grid.iter().enumerate() {
            while j + 2 < energy.len() && energy[j + 1] < g {
                j += 1;
            }
            let (e0v, e1v) = (energy[j], energy[j + 1]);
            let t = if e1v > e0v {
                ((g - e0v) / (e1v - e0v)).clamp(0.0, 1.0)
            } else {
                0.0
            };
            self.sum[gi] += mu[j] + t * (mu[j + 1] - mu[j]);
        }
        self.count += 1;
    }

    pub fn finish(self) -> Result<(Vec<f64>, Vec<f64>), String> {
        if self.count < 2 {
            return Err("need at least 2 spectra to merge".into());
        }
        if self.hi <= self.lo {
            return Err("selected spectra have no overlapping energy range".into());
        }
        let n = self.count as f64;
        let (lo, hi) = (self.lo, self.hi);
        let (grid, avg): (Vec<f64>, Vec<f64>) = self
            .grid
            .into_iter()
            .zip(self.sum)
            .filter(|&(g, _)| g >= lo && g <= hi)
            .map(|(g, s)| (g, s / n))
            .unzip();
        if grid.len() < 2 {
            return Err("overlap region too small to merge".into());
        }
        Ok((grid, avg))
    }
}

/// Average spectra on the first input's energy grid, restricted to the
/// overlap region; the others are linearly interpolated onto it.
/// Test-only reference: production merging streams through [`StreamingAverage`].
#[cfg(test)]
pub fn average_spectra(inputs: &[(Vec<f64>, Vec<f64>)]) -> Result<(Vec<f64>, Vec<f64>), String> {
    let mut iter = inputs.iter();
    let Some((energy, mu)) = iter.next() else {
        return Err("need at least 2 spectra to merge".into());
    };
    let mut acc = StreamingAverage::new(energy.clone(), mu.clone());
    for (energy, mu) in iter {
        acc.add(energy, mu);
    }
    acc.finish()
}

/// E0 of the reference channel ln(It/Ir) of this file.
pub fn reference_e0(path: &std::path::Path, import: &ImportConfig) -> Result<f64, String> {
    let mut ref_import = import.clone();
    ref_import.mode = DetectionMode::Reference;
    let (energy, mu) = load_mu(path, &ref_import)?;
    let mut sp = XASSpectrum::new();
    sp.set_spectrum(energy, mu);
    sp.find_e0()
        .map_err(|e| format!("reference E0 failed: {e}"))?;
    sp.e0().ok_or_else(|| "reference E0 not found".to_string())
}

/// Normalize/AUTOBK/FFT chain on raw arrays (shared by file loads and
/// derived/merged spectra).
pub fn process_arrays(
    energy: Vec<f64>,
    mu: Vec<f64>,
    params: &PipelineParams,
) -> Result<XASSpectrum, String> {
    let mut sp = XASSpectrum::new();
    sp.set_spectrum(energy, mu);

    match params.e0 {
        Some(e0) => {
            sp.set_e0(e0);
        }
        None => {
            sp.find_e0().map_err(|e| e.to_string())?;
        }
    }

    if params
        .edge_step
        .is_some_and(|value| !value.is_finite() || value <= 0.0)
    {
        return Err("Edge step must be finite and greater than zero.".into());
    }
    if params.bkg_nclamp.is_some_and(|value| value < 0) {
        return Err("Clamp points must be zero or greater.".into());
    }
    let mut ppe = PrePostEdge::new();
    ppe.edge_step = params.edge_step;
    let defaults = PrePostEdge::default();
    ppe.pre_edge_start = params.pre_edge_start.or(defaults.pre_edge_start);
    ppe.pre_edge_end = params.pre_edge_end.or(defaults.pre_edge_end);
    ppe.norm_start = params.norm_start.or(defaults.norm_start);
    ppe.norm_end = params.norm_end.or(defaults.norm_end);
    ppe.norm_polyorder = params.norm_polyorder.or(defaults.norm_polyorder);
    ppe.n_victoreen = params.n_victoreen.or(defaults.n_victoreen);
    sp.set_normalization_method(Some(NormalizationMethod::PrePostEdge(ppe)))
        .map_err(|e| e.to_string())?;
    sp.normalize().map_err(|e| e.to_string())?;

    let mut autobk = AUTOBK::new();
    if let Some(ek0) = params.bkg_ek0 {
        let energy = sp.energy.as_ref().unwrap();
        if !ek0.is_finite() || ek0 < energy[0] || ek0 > energy[energy.len() - 1] {
            return Err("Background k-origin E₀ must lie inside the measured energy range.".into());
        }
        autobk.ek0 = Some(ek0);
    }
    autobk.linear_regularization = params.bkg_linear_regularization;
    autobk.linear_condition_limit = params.bkg_linear_condition_limit;
    autobk.linear_residual_ratio_limit = params.bkg_linear_residual_ratio_limit;
    autobk.linear_fallback_to_lm = params.bkg_linear_fallback_to_lm;
    autobk.linear_fallback_solver = params.bkg_linear_fallback_solver;
    autobk.linear_workspace_cache = params.bkg_linear_workspace_cache;
    if let Some(standard) = &params.bkg_standard {
        standard.validate()?;
        autobk.k_std = Some(nalgebra::DVector::from_vec(standard.k.clone()));
        autobk.chi_std = Some(nalgebra::DVector::from_vec(standard.chi.clone()));
    }
    if params.rbkg.is_some() {
        autobk.rbkg = params.rbkg;
    }
    if params.bkg_kmin.is_some() {
        autobk.kmin = params.bkg_kmin;
    }
    if params.bkg_kmax.is_some() {
        autobk.kmax = params.bkg_kmax;
    }
    if params.bkg_kstep.is_some() {
        autobk.kstep = params.bkg_kstep;
    }
    if params.bkg_nknots.is_some() {
        autobk.nknots = params.bkg_nknots;
    }
    if params.bkg_kweight.is_some() {
        autobk.kweight = params.bkg_kweight;
    }
    if params.bkg_clamp_lo.is_some() {
        autobk.clamp_lo = params.bkg_clamp_lo;
    }
    if params.bkg_clamp_hi.is_some() {
        autobk.clamp_hi = params.bkg_clamp_hi;
    }
    autobk.clamp_scale_policy = Some(params.bkg_clamp_policy);
    autobk.clamp_lambda = params.bkg_clamp_lambda;
    autobk.nclamp = params.bkg_nclamp;
    if let Some(window) = params.bkg_window {
        autobk.window = window;
    }
    if params.bkg_dk.is_some() {
        autobk.dk = params.bkg_dk;
    }
    if params.bkg_solver.is_some() {
        autobk.solver = params.bkg_solver;
    }
    if let Some(nfft) = params.bkg_nfft {
        autobk.nfft = Some(nfft);
    }
    sp.set_background_method(Some(BackgroundMethod::AUTOBK(autobk)))
        .map_err(|e| e.to_string())?;
    sp.calc_background().map_err(|e| e.to_string())?;

    let mut xftf = XrayFFTF::default();
    if params.fft_kmin.is_some() {
        xftf.kmin = params.fft_kmin;
    }
    if params.fft_kmax.is_some() {
        xftf.kmax = params.fft_kmax;
    }
    if params.fft_dk.is_some() {
        xftf.dk = params.fft_dk;
    }
    if params.fft_kweight.is_some() {
        xftf.kweight = params.fft_kweight;
    }
    if params.fft_dk2.is_some() {
        xftf.dk2 = params.fft_dk2;
    }
    if params.fft_rmax.is_some() {
        xftf.rmax_out = params.fft_rmax;
    }
    if params.fft_window.is_some() {
        xftf.window = params.fft_window;
    }
    if params.fft_kstep.is_some() {
        xftf.kstep = params.fft_kstep;
    }
    if let Some(nfft) = params.fft_nfft {
        xftf.nfft = Some(usize::try_from(nfft).map_err(|_| "Forward NFFT must be at least 2")?);
    }
    sp.xftf = Some(xftf);
    sp.fft().map_err(|e| e.to_string())?;

    // Back transform (chi(q)); report invalid settings instead of silently dropping the result.
    let mut xftr = XrayFFTR::default();
    xftr.qmax_out = params.bft_qmax.or(xftr.qmax_out);
    xftr.dr2 = params.bft_dr2;
    xftr.rweight = params.bft_rweight.or(xftr.rweight);
    xftr.kstep = params.bft_kstep;
    if let Some(nfft) = params.bft_nfft {
        xftr.nfft = Some(usize::try_from(nfft).map_err(|_| "Inverse NFFT must be at least 2")?);
    }
    if params.bft_rmin.is_some() {
        xftr.rmin = params.bft_rmin;
    }
    if params.bft_rmax.is_some() {
        xftr.rmax = params.bft_rmax;
    }
    if params.bft_dr.is_some() {
        xftr.dr = params.bft_dr;
    }
    if params.bft_window.is_some() {
        xftr.window = params.bft_window;
    }
    sp.xftr = Some(xftr);
    sp.ifft().map_err(|e| format!("Inverse transform: {e}"))?;

    Ok(sp)
}

/// Linearly resample the k-weighted chi(k) onto a fixed grid (0 outside the
/// data range), so operando frames share one heatmap axis.
pub fn resample_chik(sp: &XASSpectrum, grid: &[f64]) -> Option<Vec<f64>> {
    let k = sp.k()?;
    let chi = sp.chi_kweighted()?;
    if k.len() < 2 || k.len() != chi.len() {
        return None;
    }
    let mut out = Vec::with_capacity(grid.len());
    let mut j = 0usize;
    for &g in grid {
        if g < k[0] || g > k[k.len() - 1] {
            out.push(0.0);
            continue;
        }
        while j + 2 < k.len() && k[j + 1] < g {
            j += 1;
        }
        let (k0, k1) = (k[j], k[j + 1]);
        let t = if k1 > k0 { (g - k0) / (k1 - k0) } else { 0.0 };
        out.push(chi[j] + t * (chi[j + 1] - chi[j]));
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_outputs_refuse_processing_but_preserve_display_arrays() {
        let params = PipelineParams {
            edge_step: Some(-1.0),
            ..Default::default()
        };
        for quantity in [
            Quantity::NormalizedDifference,
            Quantity::NormalizedMu,
            Quantity::ChiK,
        ] {
            let group = DerivedSpectrum {
                quantity,
                label: "renamed result".into(),
                energy: vec![1.0, 2.0, 3.0],
                mu: vec![-0.1, 0.0, 0.2],
                ..Default::default()
            };
            let reason = group.process(&params).unwrap_err();
            assert!(reason.contains(quantity.label()));
            assert!(reason.contains("normalization/AUTOBK disabled"));
            let sp = group.for_display(&params).unwrap();
            assert!(sp.normalization.is_none() && sp.xftf.is_none());
            let (x, y) = if quantity == Quantity::ChiK {
                (sp.k(), sp.chi())
            } else {
                assert!(sp.background.is_none());
                (
                    sp.energy.as_ref().map(|v| v.as_slice()),
                    sp.mu.as_ref().map(|v| v.as_slice()),
                )
            };
            assert_eq!(x.unwrap(), group.energy);
            assert_eq!(y.unwrap(), group.mu);
            if quantity == Quantity::ChiK {
                let figures = crate::publication::figures::quantity_figures(
                    std::sync::Arc::new(sp),
                    "chi",
                    Some(quantity),
                );
                assert_eq!(figures.len(), 1);
                assert_eq!(figures[0].key, "chi-k");
                assert_eq!(figures[0].series[0].x, group.energy);
                assert_eq!(figures[0].series[0].y, vec![-0.1, 0.0, 1.8]);
                assert_eq!(
                    figures[0].csv(&Default::default()).unwrap().lines().count(),
                    4
                );
            }
            assert!(group.display_label().contains(quantity.label()));
            assert!(!quantity.is_absorption());
        }
        assert!(Quantity::RawMu.is_absorption());
    }

    #[test]
    fn typed_outputs_confirmation_changes_revision_and_allows_correction() {
        let mut group = DerivedSpectrum {
            quantity_unconfirmed: true,
            ..Default::default()
        };
        let params = PipelineParams::default();
        let fingerprint = group.fingerprint(&params);
        assert!(
            group
                .process(&params)
                .unwrap_err()
                .contains("Quantity unconfirmed")
        );
        assert!(group.display_label().contains("quantity unconfirmed"));
        assert!(group.confirm_quantity(Quantity::NormalizedDifference));
        assert_ne!(fingerprint, group.fingerprint(&params));
        assert!(!group.confirm_quantity(Quantity::NormalizedDifference));
        assert_eq!(group.quantity, Quantity::NormalizedDifference);
        assert!(!group.display_label().contains("unconfirmed"));
        assert!(group.confirm_quantity(Quantity::RawMu));
        assert!(group.processing_block_reason().is_none());
    }

    #[test]
    fn typed_outputs_shift_only_absolute_energy_overrides() {
        let params = PipelineParams {
            e0: Some(9000.0),
            bkg_ek0: Some(9001.0),
            pre_edge_start: Some(-150.0),
            pre_edge_end: Some(-30.0),
            norm_start: Some(50.0),
            norm_end: Some(500.0),
            align_to_ref: true,
            align_target: Some(8980.0),
            ..Default::default()
        };
        for shift in [-3.0, 0.0, 4.5] {
            let shifted = params.for_materialized(shift);
            assert_eq!(shifted.e0, Some(9000.0 + shift));
            assert_eq!(shifted.bkg_ek0, Some(9001.0 + shift));
            let mut expected = params.clone();
            expected.e0 = shifted.e0;
            expected.bkg_ek0 = shifted.bkg_ek0;
            expected.align_to_ref = false;
            expected.align_target = None;
            assert!(shifted == expected);
        }
        let auto = PipelineParams::default().for_materialized(3.0);
        assert_eq!(auto.e0, None);
        assert_eq!(auto.bkg_ek0, None);
    }

    #[test]
    fn mapping_roi_parse_deduplicates_overlapping_ranges() {
        assert_eq!(parse_cols("8, 4-6 5,8,4"), Some(vec![4, 5, 6, 8]));
        assert_eq!(parse_cols("0,0"), Some(vec![0]));
        for invalid in ["", "  ", "6-4", "4,x", "-1", "4-", "1-2-3"] {
            assert_eq!(parse_cols(invalid), None, "{invalid}");
        }
    }

    #[test]
    fn mapping_roi_toggle_seeds_auto_and_preserves_manual_empty() {
        let mut import = ImportConfig::default();
        import.toggle_fluor(Some(5), Some(&[4, 5, 6, 6])).unwrap();
        assert_eq!(import.fluor_cols, Some(vec![4, 6]));
        import.toggle_fluor(Some(7), None).unwrap();
        assert_eq!(import.fluor_cols, Some(vec![4, 6, 7]));
        import.fluor_cols = Some(vec![4, 4]);
        import.toggle_fluor(Some(4), Some(&[4, 5])).unwrap();
        assert_eq!(import.fluor_cols, Some(vec![]));
        import.toggle_fluor(Some(7), Some(&[4, 5])).unwrap();
        assert_eq!(import.fluor_cols, Some(vec![7]));
        import.toggle_fluor(None, None).unwrap();
        assert_eq!(import.fluor_cols, None);
        assert!(import.toggle_fluor(Some(4), None).is_err());
        assert_eq!(import.fluor_cols, None);
        import.toggle_fluor(Some(7), Some(&[4, 5])).unwrap();
        assert_eq!(import.fluor_cols, Some(vec![4, 5, 7]));
    }

    #[test]
    fn mapping_roi_loaded_duplicates_are_previewed_and_summed_once() {
        let path =
            std::env::temp_dir().join(format!("rexafs-mapping-roi-{}.dat", std::process::id()));
        std::fs::write(&path, "# energy i0 roi1 roi2\n100 10 2 3\n101 10 4 5\n").unwrap();
        let mut import = ImportConfig {
            mode: DetectionMode::Fluorescence,
            energy_col: Some(0),
            i0_col: Some(1),
            fluor_cols: Some(vec![3, 2, 3, 2]),
            ..Default::default()
        };
        let preview = preview_import(&path, &import).unwrap();
        assert_eq!(preview.resolved.fluor_cols, vec![2, 3]);
        assert_eq!(load_mu(&path, &import).unwrap().1, vec![0.5, 0.9]);
        import.fluor_cols = None;
        let auto = preview_import(&path, &import).unwrap();
        import
            .toggle_fluor(Some(2), Some(&auto.resolved.fluor_cols))
            .unwrap();
        assert_eq!(import.fluor_cols, Some(vec![3]));
        assert_eq!(load_mu(&path, &import).unwrap().1, vec![0.3, 0.5]);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn advanced_processing_preserves_standard_and_controls_actual_transforms() {
        let file = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../rexafs/tests/testfiles/xraylarch_d867/xafsdata/cu_150k.xmu");
        let base = process_file(&file, &PipelineParams::default()).unwrap();
        let standard = ChiStandard {
            label: "Cu standard".into(),
            k: base.k().unwrap().to_vec(),
            chi: base.chi().unwrap().iter().map(|v| v * 0.8).collect(),
        };
        let p = PipelineParams {
            bkg_ek0: Some(base.e0().unwrap() + 0.5),
            bkg_standard: Some(standard.clone()),
            bkg_linear_condition_limit: Some(1e9),
            bkg_linear_workspace_cache: Some(false),
            bkg_linear_regularization: Some(0.02),
            bkg_linear_residual_ratio_limit: Some(1.1),
            bkg_linear_fallback_to_lm: Some(false),
            bkg_linear_fallback_solver: Some(AUTOBKSolver::LegacyLm),
            bft_rmin: Some(1.),
            bft_rmax: Some(3.),
            bft_dr2: Some(0.25),
            bft_rweight: Some(2.),
            bft_qmax: Some(10.13),
            bft_nfft: Some(4096),
            ..Default::default()
        };
        let restored: PipelineParams =
            serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
        assert_eq!(restored.fingerprint(), p.fingerprint());
        let spectrum = process_file(&file, &restored).unwrap();
        assert_eq!(
            spectrum.e0(),
            base.e0(),
            "background E0 must not move the normalization edge"
        );
        let Some(BackgroundMethod::AUTOBK(a)) = &spectrum.background else {
            panic!("AUTOBK missing")
        };
        assert_eq!(a.ek0, p.bkg_ek0);
        assert_eq!(a.k_std.as_ref().unwrap().as_slice(), standard.k);
        assert_eq!(a.chi_std.as_ref().unwrap().as_slice(), standard.chi);
        assert_eq!(a.linear_condition_limit, p.bkg_linear_condition_limit);
        assert_eq!(a.linear_workspace_cache, Some(false));
        assert_eq!(a.linear_regularization, p.bkg_linear_regularization);
        assert_eq!(
            a.linear_residual_ratio_limit,
            p.bkg_linear_residual_ratio_limit
        );
        assert_eq!(a.linear_fallback_solver, p.bkg_linear_fallback_solver);
        assert_eq!(a.linear_fallback_to_lm, Some(false));
        assert_eq!(a.solver, Some(AUTOBKSolver::LinearDirect));
        let inverse = spectrum.xftr.as_ref().unwrap();
        assert_eq!(inverse.dr2, Some(0.25));
        assert_eq!(inverse.rweight, Some(2.));
        assert_eq!(inverse.nfft, Some(4096));
        let q = spectrum.q().unwrap();
        assert!((q[1] - 0.025).abs() < 1e-12);
        assert!((q[q.len() - 1] - 10.125).abs() < 1e-12);
        assert_eq!(q.len(), spectrum.chiq().unwrap().len());
        let mut incompatible = p.clone();
        incompatible.bft_kstep = Some(0.05);
        assert!(
            process_file(&file, &incompatible)
                .err()
                .unwrap()
                .contains("kstep/nfft")
        );
        let mut changed = p.clone();
        changed.bkg_standard.as_mut().unwrap().chi[1] += 0.1;
        assert_ne!(changed.fingerprint(), p.fingerprint());
        assert_eq!(changed.raw_fingerprint(), p.raw_fingerprint());
        let mut no_standard = p.clone();
        no_standard.bkg_standard = None;
        let without = process_file(&file, &no_standard).unwrap();
        assert!(
            spectrum
                .chi()
                .unwrap()
                .iter()
                .zip(without.chi().unwrap())
                .any(|(a, b)| (a - b).abs() > 1e-6)
        );
    }

    #[test]
    fn standard_import_rejects_malformed_rows_without_silent_truncation() {
        let file =
            std::env::temp_dir().join(format!("rexafs-chi-standard-{}.dat", std::process::id()));
        for text in [
            "0 1\n1 2 3\n",
            "0 1\n1 broken\n2 3\n",
            "0 1\n0 2\n",
            "0 1\n1 NaN\n",
            "0 1\n",
            "-1 2\n0 1\n",
        ] {
            std::fs::write(&file, text).unwrap();
            assert!(ChiStandard::load(&file).is_err(), "accepted {text:?}");
        }
        std::fs::write(&file, "# k chi\n0,0.25\n0.05,-0.125\n").unwrap();
        let loaded = ChiStandard::load(&file).unwrap();
        assert_eq!(loaded.k, vec![0., 0.05]);
        assert_eq!(loaded.chi, vec![0.25, -0.125]);
        std::fs::remove_file(file).unwrap();
    }

    #[test]
    fn available_channels_require_named_evidence_and_keep_qas_reference_separate() {
        let qas = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../rexafs/tests/testfiles/Ru_QAS.dat");
        let preview = preview_import(&qas, &ImportConfig::default()).unwrap();
        assert_eq!(
            preview.available_channels(),
            vec![
                DetectionMode::Transmission,
                DetectionMode::Fluorescence,
                DetectionMode::Reference
            ]
        );
        let raw = preview.rows[0].clone();
        let sample = process_file(&qas, &PipelineParams::default()).unwrap();
        let params = PipelineParams {
            import: ImportConfig {
                mode: DetectionMode::Reference,
                ..Default::default()
            },
            ..Default::default()
        };
        let reference = DerivedSpectrum {
            source: Some(qas),
            params: Some(params.clone()),
            ..Default::default()
        }
        .process(&params)
        .unwrap();
        assert!((sample.mu.as_ref().unwrap()[0] - (raw[1] / raw[2]).ln()).abs() < 1e-14);
        assert!((reference.mu.as_ref().unwrap()[0] - (raw[2] / raw[3]).ln()).abs() < 1e-14);
        assert_ne!(sample.mu, reference.mu);
        let mut unnamed = preview.clone();
        unnamed.names = None;
        assert_eq!(unnamed.available_channels(), vec![preview.resolved.mode]);
        let mut precomputed = preview;
        precomputed.names = Some(vec!["energy".into(), "mutrans".into(), "murefer".into()]);
        assert!(
            precomputed
                .available_channels()
                .contains(&DetectionMode::Reference)
        );
    }

    #[test]
    fn gui_edge_step_override_reaches_normalization_and_invalidates_cache() {
        let file = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../rexafs/tests/testfiles/xraylarch_d867/xafsdata/cu_150k.xmu");
        let defaults = PipelineParams::default();
        let automatic = process_file(&file, &defaults).unwrap();
        let step = automatic
            .normalization
            .as_ref()
            .unwrap()
            .get_edge_step()
            .unwrap()
            * 1.5;
        let custom = PipelineParams {
            edge_step: Some(step),
            ..defaults.clone()
        };
        assert_ne!(defaults.fingerprint(), custom.fingerprint());
        let spectrum = process_file(&file, &custom).unwrap();
        assert_eq!(
            spectrum.normalization.as_ref().unwrap().get_edge_step(),
            Some(step)
        );
        for (value, reference) in spectrum
            .norm()
            .unwrap()
            .iter()
            .zip(automatic.norm().unwrap().iter())
        {
            assert!((value * 1.5 - reference).abs() < 1e-12);
        }
        let no_clamps = PipelineParams {
            bkg_nclamp: Some(0),
            ..defaults.clone()
        };
        assert_ne!(defaults.fingerprint(), no_clamps.fingerprint());
        let spectrum = process_file(&file, &no_clamps).unwrap();
        assert!(
            matches!(spectrum.background, Some(BackgroundMethod::AUTOBK(a)) if a.nclamp == Some(0))
        );
    }

    #[test]
    fn xdi_import_uses_declared_columns_units_and_signal_math() {
        let path = std::env::temp_dir().join(format!("xts-xdi-signals-{}.XDI", std::process::id()));
        for (labels, values, mode, mu) in [
            (
                vec!["energy keV", "i0", "itrans", "irefer"],
                "8.9 10 5 2.5\n9 10 5 2.5",
                DetectionMode::Auto,
                2.0_f64.ln(),
            ),
            (
                vec!["energy keV", "i0", "ifluor"],
                "8.9 10 2\n9 10 2",
                DetectionMode::Auto,
                0.2,
            ),
            (
                vec!["energy keV", "i0", "itrans", "irefer"],
                "8.9 10 5 2.5\n9 10 5 2.5",
                DetectionMode::Reference,
                2.0_f64.ln(),
            ),
            (
                vec!["energy keV", "murefer", "mutrans"],
                "8.9 0.1 0.4\n9 0.1 0.4",
                DetectionMode::Auto,
                0.4,
            ),
            (
                vec!["energy keV", "murefer", "mutrans"],
                "8.9 0.1 0.4\n9 0.1 0.4",
                DetectionMode::Reference,
                0.1,
            ),
        ] {
            let columns = labels
                .iter()
                .enumerate()
                .map(|(i, label)| format!("# Column.{}: {label}\n", i + 1))
                .collect::<String>();
            // No optional label line: declarations alone must work.
            std::fs::write(
                &path,
                format!(
                    "# XDI/1.0\n{columns}# Element.symbol: Cu\n# Element.edge: K\n# ---\n{values}\n"
                ),
            )
            .unwrap();
            let import = ImportConfig {
                mode,
                ..Default::default()
            };
            let preview = preview_import(&path, &import).unwrap();
            assert_eq!(preview.names.as_ref().unwrap()[1], labels[1]);
            assert_eq!(
                preview.xdi.as_ref().unwrap().get("element.symbol"),
                Some("Cu")
            );
            assert_eq!(preview.rows[0][0], 8.9); // preview displays original units
            let (energy, actual) = load_mu(&path, &import).unwrap();
            assert_eq!(energy, [8900., 9000.]);
            assert!((actual[0] - mu).abs() < 1e-12);
        }
        // A signature remains authoritative when the extension is .dat.
        let renamed = path.with_extension("dat");
        std::fs::copy(&path, &renamed).unwrap();
        assert!(
            preview_import(&renamed, &ImportConfig::default())
                .unwrap()
                .xdi
                .is_some()
        );
        std::fs::write(&path, "# energy mu\n8900 .4\n9000 .5\n").unwrap();
        assert!(
            load_mu(&path, &ImportConfig::default())
                .unwrap_err()
                .contains("signature")
        );
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(renamed);
    }

    #[test]
    fn xdi_nickel_foil_auto_import_preserves_measured_mu() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../rexafs/tests/testfiles/xraylarch_d867/xafsdata/ni_metal_rt.xdi");
        let preview = preview_import(&path, &ImportConfig::default()).unwrap();
        assert_eq!(preview.resolved.mode, DetectionMode::MuColumn);
        assert_eq!(preview.resolved.mu_col, Some(1));
        let (energy, mu) = load_mu(&path, &ImportConfig::default()).unwrap();
        assert_eq!(energy[0], 8133.0);
        assert_eq!(mu[0], -1.1873423); // already mu: never take a second logarithm
        let spectrum = process_file(&path, &PipelineParams::default()).unwrap();
        assert!(spectrum.k().unwrap().len() > 200);
        assert!(spectrum.chi().unwrap().iter().all(|x| x.is_finite()));
    }

    #[test]
    fn xdi_angle_import_sorts_energy_and_signal_for_merge() {
        let path = std::env::temp_dir().join(format!("xts-xdi-angle-{}.xdi", std::process::id()));
        std::fs::write(&path, "# XDI/1.0\n# Column.1: angle degrees\n# Column.2: mutrans\n# Mono.d_spacing: 3.1356\n# ---\n29 .2\n30 .4\n").unwrap();
        let (energy, mu) = load_raw(&path, &PipelineParams::default()).unwrap();
        assert!(energy[0] < energy[1]);
        assert!((energy[0] - 3954.0821033677844).abs() < 1e-6);
        assert_eq!(mu, [0.4, 0.2]);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn parse_cols_lists_and_ranges() {
        assert_eq!(parse_cols("4, 6-8"), Some(vec![4, 6, 7, 8]));
        assert_eq!(parse_cols("3"), Some(vec![3]));
        assert_eq!(parse_cols("1 2 5-6"), Some(vec![1, 2, 5, 6]));
        assert_eq!(parse_cols(""), None);
        assert_eq!(parse_cols("8-4"), None);
        assert_eq!(parse_cols("a"), None);
    }

    #[test]
    fn header_names_come_from_last_comment_and_must_match_width() {
        let data =
            parse_data("# metadata that is not a header\n# Energy, I0, It\n7000 10 5\n7010 10 4\n")
                .unwrap();
        assert_eq!(
            data.names,
            Some(vec!["Energy".into(), "I0".into(), "It".into()])
        );

        let no_header = parse_data("7000 10 5\n7010 10 4\n").unwrap();
        assert_eq!(no_header.names, None);
        assert!(no_header.diagnostics.warnings().is_empty());

        let mismatch = parse_data("# energy i0\n7000 10 5\n7010 10 4\n").unwrap();
        assert_eq!(mismatch.names, None);
        assert_eq!(mismatch.diagnostics.header_units_anomalies.count, 1);
        assert_eq!(
            mismatch.diagnostics.header_units_anomalies.examples,
            vec![1]
        );
    }

    #[test]
    fn plain_headers_are_not_malformed_data_or_spurious_anomalies() {
        for header in ["Energy I0 It", "# Energy I0 It"] {
            let data = parse_data(&format!(
                "Beamline scan title\n{header}\n7000 10 5\nbroken data row\n7010 10 4\n"
            ))
            .unwrap();
            assert_eq!(
                data.names,
                Some(vec!["Energy".into(), "I0".into(), "It".into()])
            );
            assert_eq!(data.diagnostics.malformed_rows.count, 1);
            assert_eq!(data.diagnostics.malformed_rows.examples, vec![4]);
            assert_eq!(data.diagnostics.header_units_anomalies.count, 0);
        }
        for header in ["# ----", "# Data:", "# scan 123"] {
            let data =
                parse_data(&format!("Scan title\n{header}\n7000 10 5\n7010 10 4\n")).unwrap();
            assert_eq!(data.names, None);
            assert!(data.diagnostics.warnings().is_empty());
        }
        let mismatch = parse_data("Energy I0\n7000 10 5\n7010 10 4\n").unwrap();
        assert_eq!(mismatch.diagnostics.malformed_rows.count, 0);
        assert_eq!(mismatch.diagnostics.header_units_anomalies.count, 1);
        assert_eq!(
            mismatch.diagnostics.header_units_anomalies.examples,
            vec![1]
        );
    }

    #[test]
    fn synonyms_match_roles_case_insensitively() {
        for name in ENERGY_NAMES {
            assert!(name_matches(&name.to_ascii_uppercase(), ENERGY_NAMES));
        }
        for name in I0_NAMES {
            assert!(name_matches(&name.to_ascii_uppercase(), I0_NAMES));
        }
        for name in IT_NAMES {
            assert!(name_matches(&name.to_ascii_uppercase(), IT_NAMES));
        }
        for name in IR_NAMES {
            assert!(name_matches(&name.to_ascii_uppercase(), IR_NAMES));
        }
        for name in MU_NAMES {
            assert!(name_matches(&name.to_ascii_uppercase(), MU_NAMES));
        }
        for name in FLUOR_NAMES {
            assert!(fluorescence_name_matches(&name.to_ascii_uppercase()));
        }
        assert!(fluorescence_name_matches("SDD_1"));
        assert!(fluorescence_name_matches("roi7"));

        let data = parse_data(
            "# MONO_E MON ITRANS IREF SDD1 roi_2 XMU\n7000 10 5 2 1 2 0.3\n7010 10 4 2 1 2 0.4\n",
        )
        .unwrap();
        let detected = detect_roles(&data);
        assert_eq!(detected.energy_col, 0);
        assert_eq!(detected.i0_col, 1);
        assert_eq!(detected.it_col, 2);
        assert_eq!(detected.ir_col, 3);
        assert_eq!(detected.fluor_cols, vec![4, 5]);
        assert_eq!(detected.mu_col, Some(6));
    }

    #[test]
    fn auto_mode_inference_has_stable_precedence() {
        let mu = parse_data("# energy mu i0 it iff\n7000 .1 10 5 2\n7010 .2 10 4 2\n").unwrap();
        assert_eq!(
            resolve_import(&mu, &ImportConfig::default()).mode,
            DetectionMode::MuColumn
        );

        let transmission = parse_data("# energy i0 it iff\n7000 10 5 2\n7010 10 4 2\n").unwrap();
        assert_eq!(
            resolve_import(&transmission, &ImportConfig::default()).mode,
            DetectionMode::Transmission
        );

        let fluorescence = parse_data("# energy monitor sdd1\n7000 10 2\n7010 10 3\n").unwrap();
        assert_eq!(
            resolve_import(&fluorescence, &ImportConfig::default()).mode,
            DetectionMode::Fluorescence
        );
    }

    #[test]
    fn no_header_energy_fallback_uses_plausible_monotonic_column() {
        let data = parse_data("0 7000 10 5\n1 7020 10 4\n2 7040 10 3\n").unwrap();
        let resolved = resolve_import(&data, &ImportConfig::default());
        assert_eq!(resolved.energy_col, 1);
        assert_eq!(resolved.i0_col, 1);
        assert_eq!(resolved.it_col, 2);
        assert_eq!(resolved.ir_col, 3);
        assert_eq!(resolved.mode, DetectionMode::Transmission);
    }

    fn fixture(dir: &std::path::Path, name: &str) -> std::path::PathBuf {
        let path = dir.join(name);
        std::fs::write(
            &path,
            "# energy i0 it ir if1 if2\n\
             100.0 10.0 5.0 2.5 1.0 2.0\n\
             101.0 10.0 4.0 2.0 1.5 2.5\n\
             102.0 10.0 2.0 1.0 2.0 3.0\n",
        )
        .unwrap();
        path
    }

    #[test]
    fn detection_modes_compute_expected_mu() {
        let dir = std::env::temp_dir();
        let path = fixture(&dir, "detection_modes_fixture.dat");
        let mut import = ImportConfig::default();

        let (e, mu) = load_mu(&path, &import).unwrap();
        assert_eq!(e, vec![100.0, 101.0, 102.0]);
        assert!((mu[0] - (10.0f64 / 5.0).ln()).abs() < 1e-12);
        assert!((mu[2] - (10.0f64 / 2.0).ln()).abs() < 1e-12);

        import.mode = DetectionMode::Fluorescence;
        import.fluor_cols = Some(vec![4, 5]);
        let (_, mu) = load_mu(&path, &import).unwrap();
        assert!((mu[0] - (1.0 + 2.0) / 10.0).abs() < 1e-12);
        assert!((mu[1] - (1.5 + 2.5) / 10.0).abs() < 1e-12);

        import.mode = DetectionMode::Reference;
        let (_, mu) = load_mu(&path, &import).unwrap();
        assert!((mu[0] - (5.0f64 / 2.5).ln()).abs() < 1e-12);

        let mu_path = dir.join("precomputed_mu.dat");
        std::fs::write(&mu_path, "# energy mu\n100 0.25\n101 0.5\n").unwrap();
        let (energy, mu) = load_mu(&mu_path, &ImportConfig::default()).unwrap();
        assert_eq!(energy, vec![100.0, 101.0]);
        assert_eq!(mu, vec![0.25, 0.5]);
    }

    #[test]
    fn dirty_parser_diagnostics_count_and_locate_every_category() {
        let path = std::env::temp_dir().join("rexafs-dirty-diagnostics.dat");
        std::fs::write(
            &path,
            "# energy i0 it extra\n100 10 5\n101 bad 5\n102 10\n103 10 2 999\n104 10 0\n105 10 1\n",
        )
        .unwrap();
        let raw = load_mu_with_diagnostics(&path, &ImportConfig::default()).unwrap();
        assert_eq!(raw.energy, vec![100., 103., 105.]);
        assert_eq!(raw.mu, vec![2.0f64.ln(), 5.0f64.ln(), 10.0f64.ln()]);
        let d = &raw.diagnostics;
        for (category, line) in [
            (&d.malformed_rows, 3),
            (&d.short_rows, 4),
            (&d.truncated_wide_rows, 5),
            (&d.excluded_signal_points, 6),
            (&d.header_units_anomalies, 1),
        ] {
            assert_eq!(category.count, 1);
            assert_eq!(category.examples, vec![line]);
        }
        assert_eq!(d.valid_points, 3);
        assert_eq!(d.warnings().len(), 5);
        assert!(d.warnings().iter().all(|w| w.contains("example lines:")));
        assert_eq!(
            d.summary(),
            "3 points · 2 rows skipped · 1 wide rows truncated · 1 points excluded · 1 header/units anomalies"
        );
        let preview = preview_import(&path, &ImportConfig::default()).unwrap();
        assert_eq!(preview.diagnostics, *d);
        assert!(preview.signal_error.is_none());
        let params = PipelineParams::default();
        assert_eq!(load_raw_with_diagnostics(&path, &params).unwrap(), raw);
        let channel = DerivedSpectrum {
            source: Some(path.clone()),
            ..Default::default()
        };
        assert_eq!(
            load_group_raw_with_diagnostics(&path, &params, Some(&channel)).unwrap(),
            raw
        );
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn clean_diagnostics_preserve_exact_detection_mode_numbers() {
        let path = fixture(&std::env::temp_dir(), "clean-diagnostics.dat");
        for (mode, expected) in [
            (
                DetectionMode::Transmission,
                vec![2.0f64.ln(), 2.5f64.ln(), 5.0f64.ln()],
            ),
            (DetectionMode::Fluorescence, vec![0.3, 0.4, 0.5]),
            (DetectionMode::Reference, vec![2.0f64.ln(); 3]),
            (DetectionMode::MuColumn, vec![1., 1.5, 2.]),
        ] {
            let import = ImportConfig {
                mode,
                fluor_cols: Some(vec![4, 5]),
                mu_col: Some(4),
                ..Default::default()
            };
            let raw = load_mu_with_diagnostics(&path, &import).unwrap();
            assert_eq!(raw.energy, vec![100., 101., 102.]);
            assert_eq!(raw.mu, expected);
            assert_eq!(
                raw.diagnostics,
                ParserDiagnostics {
                    valid_points: 3,
                    ..Default::default()
                }
            );
            assert!(raw.diagnostics.warnings().is_empty());
            assert_eq!(load_mu(&path, &import).unwrap(), (raw.energy, raw.mu));
        }
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn excluded_signal_locations_cover_all_modes_and_nonfinite_energy() {
        let path = std::env::temp_dir().join("rexafs-excluded-modes.dat");
        for (header, invalid, mode) in [
            ("energy i0 it", "101 10 0", DetectionMode::Transmission),
            ("energy i0 if", "101 0 1", DetectionMode::Fluorescence),
            ("energy it ir", "101 10 0", DetectionMode::Reference),
            (
                "energy murefer extra",
                "101 NaN 1",
                DetectionMode::Reference,
            ),
            ("energy mu extra", "101 inf 1", DetectionMode::MuColumn),
        ] {
            std::fs::write(
                &path,
                format!("# {header}\n102 10 5\n{invalid}\nNaN 10 5\n100 10 5\n"),
            )
            .unwrap();
            let import = ImportConfig {
                mode,
                ..Default::default()
            };
            let raw = load_mu_with_diagnostics(&path, &import).unwrap();
            assert_eq!(raw.energy, vec![100., 102.]);
            assert_eq!(
                raw.diagnostics.excluded_signal_points,
                DiagnosticCategory {
                    count: 2,
                    examples: vec![3, 4]
                }
            );
            assert_eq!(raw.diagnostics.valid_points, 2);
        }
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn diagnostics_examples_are_bounded_and_summary_formats_thousands() {
        let mut d = ParserDiagnostics {
            valid_points: 1284,
            ..Default::default()
        };
        for line in 1..=12 {
            d.malformed_rows.record(Some(line));
        }
        assert_eq!(d.malformed_rows.count, 12);
        assert_eq!(d.malformed_rows.examples, vec![1, 2, 3, 4, 5]);
        assert_eq!(d.summary(), "1,284 points · 12 rows skipped");
        assert_eq!(ParserDiagnostics::default().summary(), "0 points");
        let mut category = DiagnosticCategory::default();
        category.record(None);
        assert_eq!(category.count, 1);
        assert!(category.examples.is_empty());
    }

    #[test]
    fn bounded_detection_reads_at_most_64_kib_and_keeps_layout_only() {
        let path = std::env::temp_dir().join("rexafs-bounded-detection.dat");
        let mut text = String::from("# energy i0 it ir roi1\n");
        for i in 0..50_000 {
            text.push_str(&format!("{} 100.0 50.0 25.0 10.0\n", 1000 + i));
        }
        assert!(text.len() > 1_000_000);
        text.push_str("51000 malformed 50 25 10\n");
        std::fs::write(&path, &text).unwrap();
        let mut reader = std::io::Cursor::new(text.as_bytes());
        let detection = detect_import_reader(&mut reader, &path, &ImportConfig::default())
            .unwrap()
            .unwrap();
        assert_eq!(reader.position(), DETECTION_BYTES);
        assert_eq!(
            detect_import(&path, &ImportConfig::default())
                .unwrap()
                .unwrap(),
            detection
        );
        // Exhaustive destructuring pins the absence of any diagnostics/totals
        // or full-signal result on the intake API.
        let ImportDetection {
            column_count,
            names,
            rows,
            detected,
            auto_mode,
            resolved,
            xdi,
            mapping_error,
        } = detection;
        assert_eq!(column_count, 5);
        assert_eq!(names.unwrap(), ["energy", "i0", "it", "ir", "roi1"]);
        assert_eq!(
            rows,
            (1000..1003)
                .map(|e| vec![e as f64, 100., 50., 25., 10.])
                .collect::<Vec<_>>()
        );
        assert_eq!(detected.mode, DetectionMode::Transmission);
        assert_eq!(auto_mode, DetectionMode::Transmission);
        assert_eq!(resolved.mode, DetectionMode::Transmission);
        assert!(xdi.is_none());
        assert!(mapping_error.is_none());
        let bounded = detect_import(&path, &ImportConfig::default())
            .unwrap()
            .unwrap();
        assert_eq!(
            bounded.available_channels(),
            vec![
                DetectionMode::Transmission,
                DetectionMode::Fluorescence,
                DetectionMode::Reference
            ]
        );
        let full = preview_import(&path, &ImportConfig::default()).unwrap();
        assert_eq!(full.diagnostics.valid_points, 50_000);
        assert_eq!(full.diagnostics.malformed_rows.count, 1);
        assert_eq!(full.diagnostics.malformed_rows.examples, vec![50_002]);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn bounded_detection_and_full_reads_reject_latin1_header_comments() {
        let path = std::env::temp_dir().join("rexafs-latin1-detection.dat");
        for byte in [0xb5, 0xb0] {
            let mut bytes = b"# units: ".to_vec();
            bytes.push(byte);
            bytes.extend_from_slice(b"\n# energy i0 it ir\n100 10 5 2\n101 10 4 2\n");
            // Check both a short file and a complete header line within a
            // bounded prefix of a larger file.
            for large in [false, true] {
                if large {
                    bytes.resize(DETECTION_BYTES as usize + 100, b' ');
                }
                std::fs::write(&path, &bytes).unwrap();
                let import = ImportConfig::default();
                assert!(detect_import(&path, &import).unwrap_err().contains("UTF-8"));
                assert!(
                    preview_import(&path, &import)
                        .unwrap_err()
                        .contains("UTF-8")
                );
                assert!(load_mu_with_diagnostics(&path, &import).is_err());
            }
        }
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn bounded_detection_is_undetermined_when_headers_exhaust_the_prefix() {
        for (extension, header, tail) in [
            ("dat", "", "# energy i0 it ir\n100 10 5 2\n101 10 4 2\n"),
            (
                "xdi",
                "# XDI/1.0\n# Column.1: energy eV\n# Column.2: i0\n# Column.3: it\n# Column.4: ir\n# ///\n",
                "# ---\n100 10 5 2\n101 10 4 2\n",
            ),
        ] {
            let path = std::env::temp_dir().join(format!("rexafs-long-header.{extension}"));
            let text = format!("{header}{}{tail}", "# metadata\n".repeat(7000));
            std::fs::write(&path, &text).unwrap();
            let import = ImportConfig::default();
            let mut reader = std::io::Cursor::new(text.as_bytes());
            assert!(
                detect_import_reader(&mut reader, &path, &import)
                    .unwrap()
                    .is_none()
            );
            assert_eq!(reader.position(), DETECTION_BYTES);
            assert!(detect_import(&path, &import).unwrap().is_none());
            let full = preview_import(&path, &import).unwrap();
            assert_eq!(full.diagnostics.valid_points, 2);
            assert!(
                full.available_channels()
                    .contains(&DetectionMode::Reference)
            );
            assert!(load_mu_with_diagnostics(&path, &import).is_ok());
            std::fs::remove_file(path).unwrap();
        }
    }

    #[test]
    fn bounded_detection_defers_only_incomplete_prefix_errors() {
        let import = ImportConfig::default();
        for name in ["empty.dat", "empty.xdi"] {
            let path = std::path::Path::new(name);
            // Short, genuinely empty inputs remain errors; a cut first line
            // has no complete evidence and must be undetermined.
            assert!(detect_import_reader(b"".as_slice(), path, &import).is_err());
            let bytes = vec![b'#'; DETECTION_BYTES as usize + 100];
            assert!(
                detect_import_reader(bytes.as_slice(), path, &import)
                    .unwrap()
                    .is_none()
            );
        }
        for prefix in [
            "# metadata\n",
            "metadata without numeric rows\n",
            "# XDI/1.0\n# ---\n",
        ] {
            let path = std::path::Path::new("no-rows.dat");
            assert!(detect_import_reader(prefix.as_bytes(), path, &import).is_err());
            let text = format!("{prefix}{}", " \n".repeat(DETECTION_BYTES as usize));
            assert!(
                detect_import_reader(text.as_bytes(), path, &import)
                    .unwrap()
                    .is_none()
            );
        }
        for prefix in [
            "# XDI/9.0\n",
            "# XDI/1.0\n# ---\n100 invalid\n",
            "# missing signature\n100 1\n",
        ] {
            let text = format!("{prefix}{}", "# metadata\n".repeat(7000));
            assert!(
                detect_import_reader(
                    text.as_bytes(),
                    std::path::Path::new("invalid.xdi"),
                    &import
                )
                .is_err()
            );
        }
    }

    #[test]
    fn bounded_detection_drops_cut_lines_and_utf8_without_probing_tail() {
        let path = std::path::Path::new("bounded.dat");
        for newline in ["\n", "\r\n"] {
            // Only one complete data row: detection must not demand two points.
            let mut bytes = format!("# energy i0 it{newline}100 10 5{newline}# ").into_bytes();
            bytes.resize(DETECTION_BYTES as usize - 1, b' ');
            bytes.extend_from_slice("é".as_bytes());
            bytes.extend_from_slice(b"\n101 10 5\n");
            let mut reader = std::io::Cursor::new(bytes);
            let detection = detect_import_reader(&mut reader, path, &ImportConfig::default())
                .unwrap()
                .unwrap();
            assert_eq!(reader.position(), DETECTION_BYTES);
            assert_eq!(detection.rows, vec![vec![100., 10., 5.]]);
            assert!(detection.mapping_error.is_none());
        }
        // Short files retain their final unterminated row.
        let mut reader = std::io::Cursor::new(b"# energy i0 it\n100 10 5");
        let detection = detect_import_reader(&mut reader, path, &ImportConfig::default())
            .unwrap()
            .unwrap();
        assert_eq!(reader.position(), reader.get_ref().len() as u64);
        assert_eq!(detection.rows.len(), 1);
        // A complete line exactly at the limit is retained; a numeric partial
        // row that could otherwise alter column detection is dropped.
        for complete in [true, false] {
            let suffix = if complete { "100 10 5\n" } else { "100 10 5" };
            let mut bytes = b"# energy i0 it\n100 10 5\n#".to_vec();
            bytes.resize(DETECTION_BYTES as usize - suffix.len() - 1, b' ');
            bytes.push(b'\n');
            bytes.extend_from_slice(suffix.as_bytes());
            let detection = detect_import_reader(bytes.as_slice(), path, &ImportConfig::default())
                .unwrap()
                .unwrap();
            assert_eq!(detection.rows.len(), if complete { 2 } else { 1 });
        }
    }

    #[test]
    fn bounded_detection_reports_mapping_errors_but_defers_signal_validation() {
        let path = std::path::Path::new("mapping.dat");
        let text = b"# energy i0 it\n100 10 0\n101 10 0\n";
        for import in [
            ImportConfig {
                energy_col: Some(9),
                ..Default::default()
            },
            ImportConfig {
                i0_col: Some(9),
                ..Default::default()
            },
            ImportConfig {
                mode: DetectionMode::Reference,
                ..Default::default()
            },
            ImportConfig {
                mode: DetectionMode::Fluorescence,
                fluor_cols: Some(vec![]),
                ..Default::default()
            },
            ImportConfig {
                mode: DetectionMode::MuColumn,
                ..Default::default()
            },
        ] {
            let detection = detect_import_reader(text.as_slice(), path, &import)
                .unwrap()
                .unwrap();
            let mut data = parse_file_data(std::str::from_utf8(text).unwrap(), path).unwrap();
            assert!(detection.mapping_error.is_some());
            assert_eq!(
                detection.mapping_error,
                construct_mu(&mut data, path, &import).err()
            );
        }
        assert!(
            detect_import_reader(text.as_slice(), path, &ImportConfig::default())
                .unwrap()
                .unwrap()
                .mapping_error
                .is_none()
        );
        let xdi = b"# XDI/1.0\n# Column.1: energy joules\n# Column.2: mutrans\n# ---\n100 1\n";
        let detection = detect_import_reader(
            xdi.as_slice(),
            std::path::Path::new("mapping.xdi"),
            &ImportConfig::default(),
        )
        .unwrap()
        .unwrap();
        assert_eq!(detection.resolved.mode, DetectionMode::MuColumn);
        // Axis conversion/units errors also remain with full signal validation.
        assert!(detection.mapping_error.is_none());
        assert!(detection.xdi.is_some());
    }

    #[test]
    fn preview_diagnostics_include_tail_and_failed_signal_construction() {
        let path = std::env::temp_dir().join("rexafs-tail-diagnostics.dat");
        let mut text = String::from("# energy i0 it\n");
        for i in 0..6000 {
            text.push_str(&format!("{} 10.0 5.0\n", 1000 + i));
        }
        assert!(text.len() > 64 * 1024);
        text.push_str("7000 broken 5\n7001 10 0\n");
        std::fs::write(&path, text).unwrap();
        let preview = preview_import(&path, &ImportConfig::default()).unwrap();
        assert_eq!(preview.rows.len(), 3);
        assert_eq!(preview.diagnostics.valid_points, 6000);
        assert_eq!(preview.diagnostics.malformed_rows.examples, vec![6002]);
        assert_eq!(
            preview.diagnostics.excluded_signal_points.examples,
            vec![6003]
        );
        std::fs::write(&path, "# energy i0 it\n100 10 0\n101 10 0\n").unwrap();
        let preview = preview_import(&path, &ImportConfig::default()).unwrap();
        assert_eq!(preview.diagnostics.valid_points, 0);
        assert_eq!(preview.diagnostics.excluded_signal_points.count, 2);
        assert!(
            preview
                .signal_error
                .as_ref()
                .unwrap()
                .contains("fewer than 2")
        );
        assert!(load_mu_with_diagnostics(&path, &ImportConfig::default()).is_err());
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn xdi_diagnostics_retain_header_warnings_and_signal_line_numbers() {
        let path = std::env::temp_dir().join("rexafs-xdi-diagnostics.xdi");
        let text = "# XDI/1.0\n# malformed field\n# Column.1: energy eV\n# Column.2: i0\n# Column.3: it\n# ---\n100 10 5\n101 10 0\n102 10 2\n";
        std::fs::write(&path, text.replace('\n', "\r\n")).unwrap();
        let raw = load_mu_with_diagnostics(&path, &ImportConfig::default()).unwrap();
        assert_eq!(raw.diagnostics.header_units_anomalies.count, 3);
        assert_eq!(raw.diagnostics.header_units_anomalies.examples, vec![2]);
        assert_eq!(raw.diagnostics.excluded_signal_points.examples, vec![8]);
        assert_eq!(raw.diagnostics.valid_points, 2);
        std::fs::write(&path, text.replace("energy eV", "energy joules")).unwrap();
        let preview = preview_import(&path, &ImportConfig::default()).unwrap();
        assert_eq!(preview.diagnostics.header_units_anomalies.count, 4);
        assert_eq!(
            preview.diagnostics.header_units_anomalies.examples,
            vec![2, 3]
        );
        assert!(
            preview
                .signal_error
                .as_ref()
                .unwrap()
                .contains("unsupported energy units")
        );
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn bad_column_is_reported() {
        let dir = std::env::temp_dir();
        let path = fixture(&dir, "bad_column_fixture.dat");
        let import = ImportConfig {
            mode: DetectionMode::Transmission,
            i0_col: Some(9),
            ..Default::default()
        };
        let err = load_mu(&path, &import).unwrap_err();
        assert!(err.contains("I0 column 9"), "{err}");
    }

    #[test]
    fn alignment_shifts_to_target() {
        // Reference channel ln(it/ir) forms a step at ~105 eV; aligning to
        // target 100 must shift energies by 100 - e0_ref.
        let dir = std::env::temp_dir();
        let path = dir.join("align_fixture.dat");
        let mut text = String::from("# e i0 it ir\n");
        for i in 0..200 {
            let e = i as f64;
            // it/ir sigmoid edge centered at 105 eV (width 2 eV)
            let step = 1.0 / (1.0 + (-(e - 105.0) / 2.0).exp());
            let ratio: f64 = step.exp();
            // transmission channel: flat-ish
            let it = 5.0_f64;
            let i0 = 10.0_f64;
            let ir = it / ratio;
            text.push_str(&format!("{e} {i0} {it} {ir}\n"));
        }
        std::fs::write(&path, text).unwrap();
        let import = ImportConfig::default();
        let e0_ref = reference_e0(&path, &import).unwrap();
        assert!((e0_ref - 105.0).abs() < 2.0, "ref e0 = {e0_ref}");

        let _params = PipelineParams {
            align_to_ref: true,
            align_target: Some(100.0),
            ..Default::default()
        };
        let (energy, _) = load_mu(&path, &import).unwrap();
        // emulate the shift process_file applies
        let shift = 100.0 - e0_ref;
        let shifted0 = energy[0] + shift;
        assert!((shifted0 - (0.0 + shift)).abs() < 1e-9);
        assert!(shift < 0.0 && shift > -10.0);
    }

    #[test]
    fn average_of_two_known_spectra() {
        let a = (
            (0..10).map(|i| i as f64).collect::<Vec<_>>(),
            (0..10).map(|i| i as f64).collect::<Vec<_>>(),
        );
        let b = (
            (0..10).map(|i| i as f64 + 0.5).collect::<Vec<_>>(),
            (0..10).map(|_| 1.0).collect::<Vec<_>>(),
        );
        let (grid, avg) = average_spectra(&[a, b]).unwrap();
        // overlap region [0.5, 9.0]; grid from first input
        assert!(*grid.first().unwrap() >= 0.5 && *grid.last().unwrap() <= 9.0);
        // avg = (g + 1.0)/2 at every grid point
        for (g, v) in grid.iter().zip(avg.iter()) {
            assert!((v - (g + 1.0) / 2.0).abs() < 1e-9, "g={g} v={v}");
        }
    }

    /// Transmission via the generic loader must match the legacy QAS loader.
    #[test]
    fn transmission_matches_legacy_qas_loader() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../rexafs/tests/testfiles/Ru_QAS.dat");
        let (e, mu) = load_mu(&path, &ImportConfig::default()).unwrap();
        let legacy = rexafs::xafs::io::load_spectrum_QAS_trans(&path).unwrap();
        let le = legacy.raw_energy.as_ref().unwrap();
        let lm = legacy.raw_mu.as_ref().unwrap();
        assert_eq!(e.len(), le.len());
        for i in [0, 100, e.len() - 1] {
            assert!((e[i] - le[i]).abs() < 1e-9);
            assert!((mu[i] - lm[i]).abs() < 1e-9);
        }
    }

    #[test]
    fn ru_qas_auto_detection_golden() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../rexafs/tests/testfiles/Ru_QAS.dat");
        let text = std::fs::read_to_string(path).unwrap();
        let data = parse_data(&text).unwrap();
        let resolved = resolve_import(&data, &ImportConfig::default());
        assert_eq!(resolved.energy_col, 0);
        assert_eq!(resolved.i0_col, 1);
        assert_eq!(resolved.it_col, 2);
        assert_eq!(resolved.ir_col, 3);
        assert_eq!(resolved.fluor_cols, vec![4]);
        assert_eq!(resolved.mode, DetectionMode::Transmission);
    }

    #[test]
    fn legacy_import_json_deserializes_as_manual_overrides() {
        let import: ImportConfig = serde_json::from_str(
            r#"{
                "mode":"Transmission",
                "energy_col":0,
                "i0_col":1,
                "it_col":2,
                "ir_col":3,
                "fluor_cols":[4]
            }"#,
        )
        .unwrap();
        assert_eq!(import.energy_col, Some(0));
        assert_eq!(import.i0_col, Some(1));
        assert_eq!(import.it_col, Some(2));
        assert_eq!(import.ir_col, Some(3));
        assert_eq!(import.fluor_cols, Some(vec![4]));
        assert_eq!(import.mu_col, None);
    }
}
