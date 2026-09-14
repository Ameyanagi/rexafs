use super::signals::{add, infer};
use super::{
    beamlines, measurement, signals, text, EnergyConversion, Measurement, MeasurementColumn,
    MeasurementScan, ReadError, SignalConversion,
};
use regex::Regex;
use std::collections::BTreeMap;
use std::sync::LazyLock;

static NUMBER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[+-]?(?:\d+\.?\d*|\.\d+)(?:[eEdD][+-]?\d+)?").unwrap());
static DECLARED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^column\.(\d+)\s*:\s*(.*)$").unwrap());
static FIO_COL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^Col\s+(\d+)\s+(\S+)\s+\S+").unwrap());
static ENUM_COL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?:^|\s)(\d+)\)\s*(.*?)(?:\s{2,}|$)").unwrap());
static SPACES: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s{2,}").unwrap());

pub(super) fn numbers(line: &str) -> Option<Vec<f64>> {
    let line = line.split('#').next().unwrap_or("").trim();
    if line.is_empty() {
        return None;
    }
    let tokens: Vec<_> = if line.contains(',') {
        line.split(',').map(str::trim).collect()
    } else {
        line.split_whitespace().collect()
    };
    tokens
        .into_iter()
        .map(|s| s.replace(['D', 'd'], "E").parse().ok())
        .collect()
}

fn clean(line: &str) -> &str {
    line.trim().trim_start_matches(['#', ';', '!']).trim()
}

pub(super) fn parse(text: &str) -> Result<Measurement, ReadError> {
    if text.starts_with("\"Data\"\t\"Hora\"") {
        return lnls(text);
    }
    if text.contains("#Energy,Ch0(SC)") {
        return ritsumeikan(text);
    }
    if text.trim_start().starts_with("&SRS") {
        return srs(text);
    }
    if text
        .lines()
        .next()
        .is_some_and(|s| s.contains("FDMNES program"))
    {
        return fdmnes(text);
    }
    let pf = text.lines().next().is_some_and(|s| s.contains("9809"));
    let format = if let Some(format) = super::beamlines::identify(text) {
        format
    } else if pf {
        "9809"
    } else if text.trim_start().starts_with("#XDI/") || text.trim_start().starts_with("# XDI/") {
        "xdi"
    } else if text.lines().any(|s| s.starts_with("#S ")) {
        "spec"
    } else if text.lines().any(|s| s.trim() == "%d") {
        "fio"
    } else if text.starts_with("SSRL") {
        "ssrl_ascii"
    } else if text.contains("NPTS") && text.contains("DELEND:") {
        "lytle"
    } else if text.contains("[EX_BEGIN]") {
        "ex3"
    } else {
        "text"
    };
    // SPEC #S records define scans even if the preceding scan is empty.
    let lines: Vec<_> = text.lines().collect();
    let starts: Vec<_> = lines
        .iter()
        .enumerate()
        .filter_map(|(i, s)| s.starts_with("#S ").then_some(i))
        .collect();
    let mut scans = Vec::new();
    if starts.len() > 1 {
        let global = lines[..starts[0]].join("\n");
        for (n, &start) in starts.iter().enumerate() {
            let end = starts.get(n + 1).copied().unwrap_or(lines.len());
            let block = format!("{global}\n{}", lines[start..end].join("\n"));
            scans.push(table(&block, format, n)?);
        }
    } else {
        scans.push(table(text, format, 0)?);
    }
    Ok(measurement(format, scans))
}

fn fdmnes(text: &str) -> Result<Measurement, ReadError> {
    let mut scan = table(text, "fdmnes", 0)?;
    let edge = scan
        .header
        .lines()
        .find_map(|line| {
            let (values, names) = clean(line).split_once('=')?;
            let names: Vec<_> = names.split(',').map(str::trim).collect();
            let edge = names.iter().position(|&n| n == "E_edge")?;
            numbers(values)?.get(edge).copied()
        })
        .filter(|v| v.is_finite() && *v > 0.)
        .ok_or_else(|| {
            ReadError::new("FDMNES: missing finite positive E_edge in the original header")
        })?;
    let energy = scan
        .columns
        .iter()
        .position(|c| c.name.eq_ignore_ascii_case("energy"))
        .ok_or_else(|| ReadError::new("FDMNES: missing Energy column"))?;
    scan.signals.clear();
    scan.columns[energy].units = Some("eV relative to E_edge".into());
    scan.metadata
        .insert("fdmnes.e_edge_ev".into(), edge.to_string());
    scan.metadata
        .insert("data_kind".into(), "calculated reference".into());
    for column in 0..scan.columns.len() {
        if scan.columns[column].name.eq_ignore_ascii_case("<xanes>") {
            let name = scan.columns[column].name.clone();
            add(
                &mut scan,
                &name,
                energy,
                EnergyConversion::OffsetEv { offset_ev: edge },
                SignalConversion::Direct { column },
            );
        }
    }
    scan.warnings.retain(|w| {
        !w.starts_with("Energy units are not declared") && !w.starts_with("Select axis units")
    });
    scan.warnings.push("FDMNES is a calculated reference: conversion adds the declared E_edge (eV) to the original relative axis, following Larch read_fdmnes. Stored amplitudes and convolution are retained; no additional Shift or broadening is applied.".into());
    Ok(measurement("fdmnes", vec![scan]))
}

pub(super) fn from_rows(
    id: &str,
    header: String,
    rows: Vec<Vec<f64>>,
    names: Vec<String>,
) -> Result<MeasurementScan, ReadError> {
    let width = names.len();
    if width == 0 || rows.iter().any(|r| r.len() != width) {
        return Err(ReadError::new("Inconsistent numeric table width"));
    }
    let columns = names
        .into_iter()
        .enumerate()
        .map(|(i, name)| MeasurementColumn {
            name,
            units: None,
            values: rows.iter().map(|r| r[i]).collect(),
        })
        .collect();
    Ok(MeasurementScan {
        id: id.into(),
        label: id.into(),
        columns,
        header,
        metadata: BTreeMap::new(),
        signals: Vec::new(),
        warnings: Vec::new(),
    })
}

fn table(text: &str, format: &str, index: usize) -> Result<MeasurementScan, ReadError> {
    let lines: Vec<_> = text.lines().collect();
    let marker = if format == "9809" {
        lines
            .iter()
            .position(|s| s.trim_start().starts_with("Offset"))
            .map(|i| i + 1)
    } else if matches!(format, "ssrl_ascii" | "ssrl_micro") {
        lines
            .iter()
            .position(|s| s.trim() == "Data:")
            .map(|i| i + 1)
    } else if format == "ex3" {
        lines
            .iter()
            .position(|s| s.trim() == "[EX_BEGIN]")
            .map(|i| i + 1)
    } else if format == "lytle" {
        Some(2)
    } else {
        None
    };
    let search = marker.unwrap_or(0);
    let mut start = (search..lines.len())
        .find(|&i| {
            !lines[i].trim_start().starts_with(['#', ';', '!', '*'])
                && numbers(lines[i]).is_some_and(|r| r.len() >= 2)
        })
        .ok_or_else(|| ReadError::new(format!("{format}: no numeric data table")))?;
    if format == "9809" {
        start = (search..lines.len())
            .find(|&i| !lines[i].trim().is_empty())
            .ok_or_else(|| ReadError::new("9809: no measurement rows after Offset"))?;
    } else if let Some((i, previous)) = lines[..start]
        .iter()
        .enumerate()
        .rev()
        .find(|(_, s)| !s.trim().is_empty())
    {
        let first = previous.trim().split([',', ' ', '\t']).next().unwrap_or("");
        if first.parse::<f64>().is_ok() {
            return Err(ReadError::new(format!(
                "{format}, line {}: invalid first numeric row",
                i + 1
            )));
        }
    }
    let expected_9809_width = if format == "9809" {
        lines[..start].iter().find_map(|s| {
            s.trim()
                .strip_prefix("Mode")
                .and_then(numbers)
                .map(|m| m.len() + 1)
        })
    } else {
        None
    };
    let mut rows = Vec::new();
    let mut width = 0;
    let lnls = text.starts_with("#") && text.contains("Data") && text.contains("Date");
    let mut trailers = Vec::new();
    let mut row_comments = Vec::new();
    let mut body_comments = Vec::new();
    for (i, line) in lines.iter().enumerate().skip(start) {
        let line = line.trim();
        if format == "bm23" && !line.is_empty() && line.chars().all(|c| c == '-') {
            body_comments.push((i + 1, line.to_owned()));
            continue;
        }
        if line.is_empty() || line.starts_with(['#', ';', '!', '*']) {
            if !line.is_empty() {
                body_comments.push((i + 1, line.to_owned()));
            }
            continue;
        }
        if line == "[EX_END]" || line == "END" || line == "\u{1a}" || line.starts_with("END OF") {
            trailers.extend_from_slice(&lines[i..]);
            break;
        }
        let values = numbers(line)
            .filter(|v| expected_9809_width.is_none_or(|n| v.len() == n))
            .or_else(|| {
                let n = expected_9809_width?;
                let raw = lines[i];
                if !raw.is_ascii() || raw.len() < n * 10 || !raw[n * 10..].trim().is_empty() {
                    return None;
                }
                (0..n)
                    .map(|c| raw[c * 10..(c + 1) * 10].trim().parse::<f64>().ok())
                    .collect()
            })
            .or_else(|| {
                if lnls {
                    let tokens: Vec<_> = line.split_whitespace().collect();
                    tokens
                        .iter()
                        .take_while(|v| v.parse::<f64>().is_ok())
                        .map(|v| v.parse().ok())
                        .collect()
                } else {
                    None
                }
            })
            .ok_or_else(|| {
                ReadError::new(format!("{format}, line {}: invalid numeric row", i + 1))
            })?;
        if rows.is_empty() {
            width = values.len();
        }
        if values.len() != width {
            return Err(ReadError::new(format!(
                "{format}, line {}: {} columns; expected {width}",
                i + 1,
                values.len()
            )));
        }
        row_comments.push(
            line.split_once('#')
                .map_or("", |(_, comment)| comment)
                .to_owned(),
        );
        rows.push(values);
    }
    let mut header = lines[..start].join("\n");
    if !trailers.is_empty() {
        header.push('\n');
        header.push_str(&trailers.join("\n"));
    }
    let (names, units, mut warnings) = if format == "ex3" {
        // Rigaku REX2000 manual MJ13242B02, Appendix III: the two
        // EX_BEGIN/EX_END columns are energy (eV) and stored absorbance.
        // EX_CRYSTAL and other starred fields describe acquisition metadata.
        if width != 2 {
            return Err(ReadError::new(
                "EX3: expected energy and absorbance columns",
            ));
        }
        (
            vec!["energy".into(), "mu".into()],
            vec![Some("eV".into()), None],
            Vec::new(),
        )
    } else {
        labels(&lines[..start], width)
    };
    let id = lines
        .iter()
        .find_map(|s| s.strip_prefix("#S "))
        .unwrap_or("")
        .trim();
    let mut scan = from_rows(
        &if id.is_empty() {
            format!("scan_{}", index + 1)
        } else {
            id.into()
        },
        header,
        rows,
        names,
    )?;
    for (col, unit) in scan.columns.iter_mut().zip(units) {
        col.units = unit;
    }
    scan.warnings.append(&mut warnings);
    if !body_comments.is_empty() {
        scan.metadata.insert(
            "body_comments".into(),
            serde_json::to_string(&body_comments).unwrap(),
        );
    }
    if row_comments.iter().any(|c| !c.is_empty()) {
        scan.metadata.insert(
            "row_comments".into(),
            serde_json::to_string(&row_comments).unwrap(),
        );
    }
    for line in &lines[..start] {
        if let Some((key, value)) = clean(line).split_once(':') {
            scan.metadata
                .insert(key.trim().to_lowercase(), value.trim().into());
        }
    }
    if format == "9809" {
        if width < 5 {
            return Err(ReadError::new(
                "9809: expected angle, angle, time and at least two detectors",
            ));
        }
        let d = lines
            .iter()
            .find(|s| s.contains("Mono") && s.contains("D="))
            .and_then(|s| s.split_once("D="))
            .and_then(|(_, s)| numbers_in(s).first().copied())
            .ok_or_else(|| ReadError::new("9809: missing monochromator D spacing"))?;
        // Generic heading inference can mistake the scan-plan Step/deg field
        // for a detector unit. The 9809 body declares angles and dwell time;
        // its detector Mode codes describe roles, not physical units.
        for column in &mut scan.columns {
            column.units = None;
        }
        for (i, name) in [
            "requested_angle",
            "observed_angle",
            "time",
            "i0",
            "detector_1",
        ]
        .iter()
        .enumerate()
        {
            scan.columns[i].name = (*name).into();
        }
        scan.columns[1].units = Some("deg".into());
        let axis = EnergyConversion::Bragg {
            d_spacing: d,
            degrees_per_unit: 1.,
        };
        scan.columns[0].units = Some("deg".into());
        scan.columns[2].units = Some("s".into());
        let modes = lines
            .iter()
            .find_map(|line| line.trim().strip_prefix("Mode").and_then(numbers));
        if let Some(modes) = modes.filter(|m| m.len() + 1 == width) {
            scan.metadata
                .insert("9809_modes".into(), serde_json::to_string(&modes).unwrap());
            let incidents: Vec<_> = modes
                .iter()
                .enumerate()
                .filter_map(|(i, &m)| (m == 1.).then_some(i + 1))
                .collect();
            for (i, &mode) in modes.iter().enumerate().skip(2) {
                let col = i + 1;
                scan.columns[col].name = match mode as i32 {
                    1 => format!("i0_{col}"),
                    2 => format!("it_{col}"),
                    3 => format!("fluorescence_{col}"),
                    4 => format!("yield_{col}"),
                    101 | 103 => format!("auxiliary_{col}"),
                    _ => format!("detector_{col}"),
                };
                if let [incident] = incidents.as_slice() {
                    let signal = match mode as i32 {
                        2 => Some(SignalConversion::Transmission {
                            incident: *incident,
                            transmitted: col,
                        }),
                        3 | 4 => Some(SignalConversion::Ratio {
                            detectors: vec![col],
                            incident: *incident,
                        }),
                        _ => None,
                    };
                    if let Some(signal) = signal {
                        let label = scan.columns[col].name.clone();
                        add(&mut scan, &label, 1, axis.clone(), signal);
                    }
                }
            }
        } else {
            scan.warnings.push("9809 Mode fields are missing or disagree with the row width; detector roles require explicit selection.".into());
        }
        let fluorescence_scan = lines.iter().any(|line| {
            line.contains("Repetition")
                && line.contains("Points")
                && line.to_ascii_lowercase().contains("fluorescence")
        });
        if fluorescence_scan
            && !scan.signals.is_empty()
            && scan.signals.iter().all(|signal| {
                matches!(signal.mapping.signal, SignalConversion::Transmission { .. })
            })
        {
            // Observed in KEK BL9A QXAFS files: the acquisition summary says
            // fluorescence while Mode marks the only detector as transmitted.
            // Preserve both declarations without choosing ln(I0/It) or If/I0.
            scan.signals.clear();
            scan.warnings.push("9809 fluorescence acquisition conflicts with transmission-only detector Mode fields; select the signal arithmetic explicitly.".into());
        }
        scan.warnings.push("9809 uses the observed Bragg angle and header crystal spacing. Counts and offsets are preserved without applying an additional offset or dead-time correction; auxiliary modes 101/103 are never summed automatically.".into());
    } else if format == "lytle" {
        let params =
            numbers(lines[1]).ok_or_else(|| ReadError::new("Lytle: invalid calibration record"))?;
        if params.len() < 6 {
            return Err(ReadError::new(
                "Lytle: missing d spacing or steps per degree",
            ));
        }
        for (col, name) in scan
            .columns
            .iter_mut()
            .zip(["motor_steps", "i0", "it", "if", "ir"])
        {
            col.name = name.into();
        }
        add(
            &mut scan,
            "transmission",
            0,
            EnergyConversion::Bragg {
                d_spacing: params[4],
                degrees_per_unit: 1. / params[5],
            },
            SignalConversion::Transmission {
                incident: 1,
                transmitted: 2,
            },
        );
    } else {
        infer(&mut scan);
    }
    super::beamlines::configure(&mut scan, format)?;
    if scan
        .warnings
        .iter()
        .any(|w| w.contains("conflicts with the table"))
        || text.contains("Sample I1 = col")
    {
        scan.signals.clear();
        scan.warnings
            .push("Conflicting channel metadata requires explicit signal selection.".into());
    }
    if scan.signals.is_empty() {
        scan.warnings.push(
            "Select axis units and detector arithmetic explicitly before creating a spectrum."
                .into(),
        );
    }
    if scan
        .columns
        .iter()
        .any(|c| c.values.iter().any(|v| !v.is_finite()))
    {
        scan.warnings.push(
            "Nonfinite source values are retained; conversion rejects affected selected rows."
                .into(),
        );
    }
    Ok(scan)
}

fn numbers_in(s: &str) -> Vec<f64> {
    NUMBER
        .find_iter(s)
        .filter_map(|m| m.as_str().parse().ok())
        .collect()
}

fn split_names(s: &str, width: usize) -> Option<Vec<String>> {
    let s = clean(s).trim_start_matches("L ").trim();
    for parts in [
        s.split(',')
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .collect::<Vec<_>>(),
        s.split('\t')
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .collect(),
        SPACES
            .split(s)
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .collect(),
        s.split_whitespace().collect(),
    ] {
        if parts.len() == width
            && parts
                .iter()
                .all(|p| p.parse::<f64>().is_err() && p.chars().any(char::is_alphabetic))
        {
            return Some(parts.iter().map(|s| s.to_string()).collect());
        }
    }
    None
}

fn label_unit(name: &str) -> (String, Option<String>) {
    for unit in ["keV", "eV", "deg", "counts", "cps", "s"] {
        for suffix in [
            format!("({unit})"),
            format!("[{unit}]"),
            format!(" {unit}"),
            format!("_{unit}"),
        ] {
            if name.ends_with(&suffix) {
                return (
                    name[..name.len() - suffix.len()].trim().to_string(),
                    Some(unit.into()),
                );
            }
        }
    }
    (name.to_string(), None)
}

fn labels(lines: &[&str], width: usize) -> (Vec<String>, Vec<Option<String>>, Vec<String>) {
    let mut declared = BTreeMap::new();
    let mut warnings = Vec::new();
    for line in lines {
        let s = clean(line);
        if let Some(c) = DECLARED.captures(s) {
            if let Ok(i) = c[1].parse::<usize>() {
                let tokens: Vec<_> = c[2].split_whitespace().collect();
                if let Some(name) = tokens.first() {
                    declared.insert(
                        i,
                        (
                            (*name).to_string(),
                            tokens.get(1).filter(|s| **s != "||").map(|s| s.to_string()),
                        ),
                    );
                }
            }
        }
        if let Some(c) = FIO_COL.captures(s) {
            if let Ok(index) = c[1].parse::<usize>() {
                declared.insert(index, (c[2].into(), None));
            } else {
                warnings
                    .push("FIO column index is out of range; select columns explicitly.".into());
            }
        }
    }
    let mut names = lines.iter().rev().find_map(|s| split_names(s, width));
    if declared.len() == width && (1..=width).all(|i| declared.contains_key(&i)) {
        let (n, u) = declared.into_values().unzip();
        return (n, u, warnings);
    }
    if !declared.is_empty() {
        warnings.push(format!("Declared column count ({}) conflicts with the table ({width}); retained physical table labels and raw header. Review detector roles.",declared.len()));
    }
    if names.is_none() {
        // SSRL stores one label per line after Data:.
        if let Some(start) = lines.iter().position(|s| s.trim() == "Data:") {
            let n: Vec<_> = lines[start + 1..]
                .iter()
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
                .collect();
            if n.len() == width {
                names = Some(n);
            }
        }
    }
    if names.is_none() {
        let mut enumerated = BTreeMap::new();
        for line in lines {
            for c in ENUM_COL.captures_iter(clean(line)) {
                if let Ok(index) = c[1].parse::<usize>() {
                    enumerated.insert(index, c[2].trim().to_owned());
                }
            }
        }
        if enumerated.len() == width && (1..=width).all(|i| enumerated.contains_key(&i)) {
            names = Some(enumerated.into_values().collect());
        }
    }
    let names = names.unwrap_or_else(|| (1..=width).map(|i| format!("column_{i}")).collect());
    let (mut n, u): (Vec<_>, Vec<_>) = names.iter().map(|n| label_unit(n)).unzip();
    // EPICS aliases map the physical P1/D1 headings to named channels.
    for name in &mut n {
        if let Some(value) = lines.iter().find_map(|line| {
            let s = clean(line);
            let (key, value) = s.split_once(" = {")?;
            (key == name).then(|| value.split('}').next().unwrap().to_owned())
        }) {
            *name = value;
        }
    }
    (n, u, warnings)
}

fn srs(text: &str) -> Result<Measurement, ReadError> {
    let lines: Vec<_> = text.lines().collect();
    let start = lines
        .iter()
        .position(|s| s.trim() == "&END")
        .ok_or_else(|| ReadError::new("SRS: missing &END header terminator"))?
        + 1;
    let mut rows: Vec<Vec<f64>> = Vec::new();
    for (i, line) in lines.iter().enumerate().skip(start) {
        let line = line.trim();
        if line.is_empty() || line.starts_with('C') {
            continue;
        }
        if line.starts_with("END") || line.starts_with("DATA ABORTED") {
            break;
        }
        let r = numbers(line)
            .ok_or_else(|| ReadError::new(format!("SRS line {}: invalid numeric row", i + 1)))?;
        if r.len() >= 6 {
            rows.push(r);
        } else if r.len() <= 4 && !rows.is_empty() {
            rows.last_mut().unwrap().extend(r);
        } else {
            return Err(ReadError::new(format!(
                "SRS line {}: expected six-column record or detector continuation",
                i + 1
            )));
        }
    }
    let width = rows.first().map_or(0, Vec::len);
    let mut scan = from_rows(
        "scan_1",
        lines[..start].join("\n"),
        rows,
        (0..width)
            .map(|i| {
                ["axis", "time", "i0", "it", "if", "im"]
                    .get(i)
                    .map_or_else(|| format!("detector_{}", i - 5), |s| s.to_string())
            })
            .collect(),
    )?;
    scan.warnings.push("Historical SRS axis may be energy or a millidegree encoder; choose units/calibration explicitly. Wrapped detector records are joined without applying offsets.".into());
    Ok(measurement("srs", vec![scan]))
}

fn lnls(text: &str) -> Result<Measurement, ReadError> {
    let mut lines = text.lines();
    let header = lines.next().unwrap();
    let names: Vec<_> = header
        .split('\t')
        .skip(2)
        .map(|s| s.trim_matches('"').to_owned())
        .collect();
    let mut timestamps = Vec::new();
    let mut rows = Vec::new();
    for (i, line) in lines.enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let cells: Vec<_> = line.split_whitespace().collect();
        if cells.len() != names.len() + 2 {
            return Err(ReadError::new(format!(
                "LNLS line {}: invalid row width",
                i + 2
            )));
        }
        timestamps.push(format!("{} {}", cells[0], cells[1]));
        rows.push(
            cells[2..]
                .iter()
                .map(|s| {
                    s.parse::<f64>()
                        .map_err(|_| ReadError::new(format!("LNLS line {}: invalid number", i + 2)))
                })
                .collect::<Result<Vec<_>, _>>()?,
        );
    }
    let mut scan = from_rows("scan_1", header.into(), rows, names)?;
    scan.metadata
        .insert("row_date_time".into(), timestamps.join("\n"));
    scan.columns[0].name = "energy".into();
    scan.columns[0].units = Some("eV".into());
    scan.warnings.push("Original LNLS date/time cells are retained in row_date_time metadata, one entry per row. Detector column meanings require explicit selection.".into());
    Ok(measurement("lnls", vec![scan]))
}
fn ritsumeikan(text: &str) -> Result<Measurement, ReadError> {
    // These acquisition exports have one deliberately empty spacer column.
    // Preserve its position and missing values, without accepting blank numeric
    // cells in arbitrary CSV files.
    let (header, body) = text.split_once("#Energy,Ch0(SC)").unwrap();
    let (labels, body) = body
        .split_once('\n')
        .ok_or_else(|| ReadError::new("Ritsumeikan: no table"))?;
    let names: Vec<_> = format!("Energy,Ch0(SC){labels}")
        .split(',')
        .map(str::to_owned)
        .collect();
    let blank = names
        .iter()
        .position(String::is_empty)
        .ok_or_else(|| ReadError::new("Ritsumeikan: missing spacer"))?;
    let mut rows = Vec::new();
    for (i, line) in body.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let cells: Vec<_> = line.split(',').collect();
        if cells.len() != names.len() || !cells[blank].is_empty() {
            return Err(ReadError::new(format!(
                "Ritsumeikan row {}: invalid spacer or width",
                i + 1
            )));
        }
        rows.push(
            cells
                .iter()
                .enumerate()
                .map(|(j, s)| {
                    if j == blank {
                        Ok(f64::NAN)
                    } else {
                        s.parse::<f64>().map_err(|_| {
                            ReadError::new(format!(
                                "Ritsumeikan row {}: invalid numeric cell",
                                i + 1
                            ))
                        })
                    }
                })
                .collect::<Result<Vec<_>, _>>()?,
        );
    }
    let mut scan = from_rows(
        "scan_1",
        format!("{header}#Energy,Ch0(SC){labels}"),
        rows,
        names,
    )?;
    scan.columns[blank].name = "empty_spacer".into();
    let energy = scan
        .columns
        .iter()
        .position(|c| c.name == "Real eV")
        .ok_or_else(|| ReadError::new("Ritsumeikan: missing Real eV column"))?;
    scan.columns[energy].units = Some("eV".into());
    for (label, col) in [("SC yield", 13), ("MCP yield", 14), ("SDD yield", 15)] {
        add(
            &mut scan,
            label,
            energy,
            EnergyConversion::Ev,
            SignalConversion::Direct { column: col },
        );
    }
    scan.warnings.push("The intentional empty CSV spacer is retained as NaN. Real eV and stored channel/monitor ratios are available; select the experiment's active channel.".into());
    Ok(measurement("ritsumeikan", vec![scan]))
}
