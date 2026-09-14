//! Counted XTUNES tables, implemented from the retained files and the
//! xtunes-analysis format report. Vendor processing code is not used.
use super::{
    measurement, signals, text, EnergyConversion, Measurement, MeasurementColumn,
    MeasurementDataset, MeasurementScan, ReadError, SignalConversion,
};
use std::collections::BTreeMap;
use std::collections::BTreeSet;

pub(super) fn recognizes(text: &str) -> bool {
    let text = text.trim_start();
    text.starts_with("DataFileName=") || text.starts_with("#XTSP FilePath=")
}

pub(super) fn parse(source: &str) -> Result<Measurement, ReadError> {
    let lines: Vec<_> = source.lines().collect();
    let boundaries: Vec<_> = lines
        .iter()
        .enumerate()
        .filter_map(|(i, s)| s.starts_with("#XTSP FilePath=").then_some(i))
        .collect();
    let project = !boundaries.is_empty();
    if project && lines[..boundaries[0]].iter().any(|s| !s.trim().is_empty()) {
        return Err(ReadError::new("XTUNES project: unexpected preamble"));
    }
    let mut result = measurement(if project { "xtunes_project" } else { "xtunes" }, vec![]);
    let starts = if project { boundaries } else { vec![0] };
    for (ordinal, &start) in starts.iter().enumerate() {
        let end = starts.get(ordinal + 1).copied().unwrap_or(lines.len());
        record(&lines[start..end], start, ordinal, &mut result)?;
    }
    Ok(result)
}

fn quantity(section: &str) -> &'static str {
    match section {
        "BG Plot" | "QD Plot" => {
            "Energy in eV; stored absorption. BG also archives background and baseline."
        }
        "XANES Plot" => "Energy in eV; saved XTUNES normalized absorption.",
        "Xi Plot" => "Wave number in inverse angstroms; saved unweighted chi(k).",
        "ED Plot" => "Wave number in inverse angstroms; saved resampled k-weighted chi(k).",
        "FT Plot" => {
            "Distance in angstroms; saved real, imaginary and magnitude (not squared power)."
        }
        "BackFT Plot" => "Wave number in inverse angstroms; saved back-transform data and fit.",
        "FT-Fit Plot" => "Distance in angstroms; saved Fourier magnitude and fitted magnitude.",
        _ => "Archived XTUNES result; interpret using the original headings and settings.",
    }
}

fn record(
    lines: &[&str],
    offset: usize,
    ordinal: usize,
    result: &mut Measurement,
) -> Result<(), ReadError> {
    let id = format!("record_{}", ordinal + 1);
    let fail = |line: usize, message: &str| {
        ReadError::new(format!(
            "XTUNES {id}, line {}: {message}",
            offset + line + 1
        ))
    };
    let mut metadata = BTreeMap::new();
    let mut parameters: Vec<(String, String, String)> = Vec::new();
    let mut section = String::new();
    let mut tables = BTreeSet::new();
    let mut absorption = None;
    let mut pending_functions = 0usize;
    let mut function_ordinal = 0usize;
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        if line.trim().is_empty() {
            i += 1;
            continue;
        }
        if let Some(path) = line.strip_prefix("#XTSP FilePath=") {
            metadata.insert("record_path".into(), path.into());
            i += 1;
            continue;
        }
        if let Some(path) = line.strip_prefix("DataFileName=") {
            if metadata.insert("source_path".into(), path.into()).is_some() {
                return Err(fail(i, "duplicate DataFileName"));
            }
            i += 1;
            continue;
        }
        let fields: Vec<_> = line.split('\t').collect();
        let name = fields[0].trim().trim_start_matches('#');
        if name == "CF ImportFile" {
            if pending_functions != 0 {
                return Err(fail(i, "missing embedded function"));
            }
            pending_functions = fields
                .get(1)
                .and_then(|s| s.parse::<usize>().ok())
                .filter(|&n| n <= lines.len())
                .ok_or_else(|| fail(i, "invalid function count"))?;
            function_ordinal = 0;
            i += 1;
            continue;
        }
        let function = name
            .strip_prefix("Func")
            .and_then(|n| n.parse::<usize>().ok());
        if name.ends_with(" Plot") || function.is_some() {
            if !tables.insert(name.to_owned()) {
                return Err(fail(i, "duplicate counted table"));
            }
            if let Some(n) = function {
                function_ordinal += 1;
                if pending_functions == 0 || n != function_ordinal {
                    return Err(fail(i, "unexpected embedded function ordinal"));
                }
                pending_functions -= 1;
            } else if pending_functions != 0 {
                return Err(fail(i, "missing embedded function"));
            }
            let count = fields
                .get(1)
                .and_then(|s| s.parse::<usize>().ok())
                .ok_or_else(|| fail(i, "missing or invalid table count"))?;
            if count > lines.len().saturating_sub(i + 2) || i + 1 >= lines.len() {
                return Err(fail(i, "truncated counted table"));
            }
            let headings: Vec<_> = lines[i + 1].split_whitespace().collect();
            if headings.is_empty() || headings[0].starts_with('#') {
                return Err(fail(i + 1, "missing table headings"));
            }
            let width = headings.len();
            let mut values = Vec::new();
            for (row, line) in lines.iter().enumerate().skip(i + 2).take(count) {
                let numbers =
                    text::numbers(line).ok_or_else(|| fail(row, "invalid numeric table row"))?;
                if numbers.len() != width {
                    return Err(fail(row, "table width differs from headings"));
                }
                values.extend(numbers);
            }
            let mut attributes = BTreeMap::from([
                ("columns".into(), headings.join("\t")),
                ("quantity".into(), quantity(name).into()),
                (
                    "origin".into(),
                    "Saved XTUNES output; no rexafs processing performed.".into(),
                ),
            ]);
            if function.is_some() {
                attributes.insert(
                    "source_path".into(),
                    fields.get(2..).unwrap_or_default().join("\t"),
                );
            }
            if matches!(name, "BG Plot" | "QD Plot") {
                if absorption.is_some() {
                    return Err(fail(i, "multiple absorption tables in one record"));
                }
                if headings.first() != Some(&"E") || headings.get(1) != Some(&"Mu") {
                    return Err(fail(i + 1, "expected E and Mu absorption headings"));
                }
                let mut cols: Vec<_> = headings
                    .iter()
                    .enumerate()
                    .map(|(c, name)| MeasurementColumn {
                        name: (*name).into(),
                        units: (c == 0).then(|| "eV".into()),
                        values: values.chunks_exact(width).map(|r| r[c]).collect(),
                    })
                    .collect();
                cols[0].name = "energy".into();
                cols[1].name = "mu".into();
                absorption = Some(cols);
            }
            result.datasets.push(MeasurementDataset {
                path: format!("/{id}/{name}"),
                shape: vec![count as u64, width as u64],
                values,
                imaginary: None,
                attributes,
            });
            i += 2 + count;
            section.clear();
            continue;
        }
        if pending_functions != 0 {
            return Err(fail(i, "missing counted function record"));
        }
        if line.starts_with('#') {
            section = line.into();
        } else if let Some((key, value)) = line.split_once('=') {
            parameters.push((section.clone(), key.into(), value.into()));
        } else {
            return Err(fail(i, "unexpected content outside a counted table"));
        }
        i += 1;
    }
    if pending_functions != 0 {
        return Err(fail(lines.len(), "truncated embedded function list"));
    }
    if !metadata.contains_key("source_path") {
        return Err(fail(0, "missing DataFileName"));
    }
    if tables.is_empty() {
        return Err(fail(0, "no counted tables"));
    }
    metadata.insert(
        "ordered_parameters".into(),
        serde_json::to_string(&parameters).unwrap(),
    );
    let label = metadata
        .get("record_path")
        .or_else(|| metadata.get("source_path"))
        .unwrap()
        .rsplit(['\\', '/'])
        .next()
        .unwrap()
        .to_owned();
    let mut scan = MeasurementScan {
        id, label, columns: absorption.unwrap_or_default(), header: lines.join("\n"), metadata,
        signals: vec![], warnings: vec!["Saved XTUNES settings and tables are archived separately. They do not initialize rexafs processing or require the original Windows paths.".into()],
    };
    if scan.columns.first().is_some_and(|c| !c.values.is_empty()) {
        signals::add(
            &mut scan,
            "Stored absorption",
            0,
            EnergyConversion::Ev,
            SignalConversion::Direct { column: 1 },
        );
    } else {
        scan.warnings.push("This record has no absorption points. Saved chi or Fourier tables remain available as archived datasets.".into());
    }
    result.scans.push(scan);
    Ok(())
}
