//! Larix 1.0 session framing and inert object inspection. Independently follows
//! Larch 2026.3.1 `larch/io/save_restore.py`; see `doc/larix-import-plan.md`.
use super::{
    measurement, signals, text, EnergyConversion, Measurement, MeasurementColumn, MeasurementScan,
    ReadError, SignalConversion,
};
use std::collections::BTreeMap;
mod array;
mod json;
use json::parse as json;
use serde_json::Value;
use std::collections::BTreeSet;

pub(super) fn error(context: &str, message: impl std::fmt::Display) -> ReadError {
    ReadError::new(format!("Larix {context}: {message}"))
}

struct Symbol<'a> {
    name: &'a str,
    raw: &'a str,
    value: Value,
}

pub(super) fn parse(source: &str) -> Result<Measurement, ReadError> {
    let mut lines = source.lines().enumerate();
    let first = lines.next().map(|(_, s)| s).unwrap_or("");
    if first.split_whitespace().nth(1) != Some("1.0") {
        return Err(error("format", "unsupported session version; expected 1.0"));
    }
    let mut header = vec![first];
    let mut history = Vec::new();
    let mut section = "header";
    let mut seen = BTreeSet::new();
    let mut symbols = Vec::new();
    let mut expected = None;
    let mut closed = false;
    let mut config_seen = false;
    let mut history_seen = false;
    while let Some((line, text)) = lines.next() {
        let fail = |msg| error(&format!("line {}", line + 1), msg);
        if closed {
            if !text.trim().is_empty() {
                return Err(fail("content after Symbols section"));
            }
        } else if section == "symbols" {
            if text == "##</Symbols>" {
                closed = true;
                continue;
            }
            let name = text
                .strip_prefix("<:")
                .and_then(|s| s.strip_suffix(":>"))
                .filter(|s| !s.is_empty())
                .ok_or_else(|| fail("expected a symbol marker or section ending"))?;
            if symbols.len() >= 10_000 {
                return Err(fail("symbol count exceeds 10000"));
            }
            if !seen.insert(name) {
                return Err(fail("duplicate symbol identifier"));
            }
            let (_, raw) = lines.next().ok_or_else(|| fail("missing symbol JSON"))?;
            symbols.push(Symbol {
                name,
                raw,
                value: json(raw, &format!("symbol {name}"))?,
            });
        } else if section == "history" {
            if text == "##</Session Commands>" {
                section = "header";
            } else {
                history.push(text);
            }
        } else if section == "config" {
            header.push(text);
            if text == "##</CONFIG>" {
                section = "header";
            } else if text.starts_with("##<") {
                return Err(fail("unclosed CONFIG section"));
            }
        } else if text == "##<CONFIG>" {
            if config_seen {
                return Err(fail("duplicate CONFIG section"));
            }
            config_seen = true;
            section = "config";
            header.push(text);
        } else if text == "##<Session Commands>" {
            if history_seen {
                return Err(fail("duplicate command history section"));
            }
            history_seen = true;
            section = "history";
        } else if let Some(count) = text
            .strip_prefix("##<Symbols: count=")
            .and_then(|s| s.strip_suffix('>'))
        {
            expected = Some(
                count
                    .parse::<usize>()
                    .map_err(|_| fail("invalid symbol count"))?,
            );
            section = "symbols";
        } else if text.starts_with("##<") {
            return Err(fail("unexpected section marker"));
        } else {
            header.push(text);
        }
    }
    if !closed {
        return Err(error("format", "missing complete Symbols section"));
    }
    let mut result = measurement("larix", Vec::new());
    let count = expected.unwrap_or(0);
    if count != symbols.len() {
        if count.checked_add(1) == Some(symbols.len()) && seen.contains("_xasgroups") {
            result.warnings.push("Larix symbol count excludes the writer-added _xasgroups index; all serialized symbols were retained.".into());
        } else {
            return Err(error(
                "format",
                "declared symbol count disagrees with serialized symbols",
            ));
        }
    }
    let mut labels = BTreeMap::new();
    if let Some(index) = symbols.iter().find(|s| s.name == "_xasgroups") {
        let entries = index
            .value
            .as_object()
            .ok_or_else(|| error("reference", "_xasgroups is not a dictionary"))?;
        for (label, value) in entries {
            if label == "__class__" {
                continue;
            }
            let name = value
                .as_str()
                .ok_or_else(|| error("reference", "_xasgroups target must be a symbol name"))?;
            if !symbols
                .iter()
                .any(|s| s.name == name && s.value["__class__"] == "Group")
            {
                return Err(error(
                    "reference",
                    format!("display name {label:?} points to missing Group {name:?}"),
                ));
            }
            labels.entry(name).or_insert(label.as_str());
        }
    }
    result
        .metadata
        .insert("larix.session_text".into(), source.into());
    result.metadata.insert(
        "larix.command_history".into(),
        serde_json::to_string(&history).unwrap(),
    );
    result.metadata.insert(
        "larix.symbol_order".into(),
        serde_json::to_string(&symbols.iter().map(|s| s.name).collect::<Vec<_>>()).unwrap(),
    );
    result
        .metadata
        .insert("larix.header".into(), header.join("\n"));
    let (mut budget, mut nodes) = (0, 0);
    for symbol in &symbols {
        let id = escape(symbol.name);
        let start = result.datasets.len();
        walk(
            &symbol.value,
            &format!("/{id}"),
            symbol.name,
            0,
            &mut budget,
            &mut nodes,
            &mut result,
        )?;
        if symbol.value["__class__"] != "Group" {
            continue;
        }
        let mut scan = MeasurementScan {
            id: id.clone(),
            label: labels.get(symbol.name).copied()
                .or_else(|| symbol.value["filename"].as_str()).unwrap_or(symbol.name).into(),
            columns: Vec::new(),
            header: format!("{}\n<:{}:>\n{}", header.join("\n"), symbol.name, symbol.raw),
            metadata: BTreeMap::from([("larix.symbol".into(), symbol.name.into())]),
            signals: Vec::new(),
            warnings: vec!["Saved Larix arrays and settings are archival. Import uses stored energy/mu and does not rerun normalization, background subtraction, Fourier transforms or detector corrections.".into()],
        };
        let datasets = &result.datasets[start..];
        let member = |name| datasets.iter().find(|d| d.path == format!("/{id}/{name}"));
        let energy = member("energy");
        let mu = member("mu");
        let absorption = if let (Some(e), Some(m)) = (energy, mu) {
            if e.shape.len() != 1
                || e.shape != m.shape
                || e.imaginary.is_some()
                || m.imaginary.is_some()
            {
                return Err(error(
                    &format!("mapping for {}", symbol.name),
                    "energy/mu require matching real one-dimensional arrays",
                ));
            }
            Some((e, m))
        } else {
            None
        };
        // Place the authoritative stored absorption pair first. Other equal-grid
        // top-level vectors remain selectable; independent/nested arrays stay datasets.
        let length = absorption.map(|(e, _)| e.values.len()).or_else(|| {
            datasets
                .iter()
                .find(|d| {
                    d.imaginary.is_none()
                        && d.shape.len() == 1
                        && !d.path[id.len() + 2..].contains('/')
                })
                .map(|d| d.values.len())
        });
        let mut chosen = BTreeSet::new();
        if let Some((e, m)) = absorption {
            for (name, ds) in [("energy", e), ("mu", m)] {
                chosen.insert(ds.path.as_str());
                scan.columns.push(MeasurementColumn {
                    name: name.into(),
                    units: ds.attributes.get("units").cloned(),
                    values: ds.values.clone(),
                });
            }
        }
        for ds in datasets {
            let name = &ds.path[id.len() + 2..];
            if !name.contains('/')
                && ds.shape.len() == 1
                && ds.imaginary.is_none()
                && length == Some(ds.values.len())
                && chosen.insert(ds.path.as_str())
            {
                scan.columns.push(MeasurementColumn {
                    name: name.into(),
                    units: ds.attributes.get("units").cloned(),
                    values: ds.values.clone(),
                });
            }
        }
        let datatype = symbol.value["datatype"].as_str();
        if absorption.is_some() && datatype.is_none_or(|t| t.eq_ignore_ascii_case("xas")) {
            let units = symbol.value["energy_units"].as_str();
            let conversion = match units.map(str::to_ascii_lowercase).as_deref() {
                Some("ev") | None => Some(EnergyConversion::Ev),
                Some("kev") => Some(EnergyConversion::Kev),
                _ => None,
            };
            if units.is_none() {
                scan.warnings.push("Larix energy/mu uses eV by the Larch XAS convention; no energy_units declaration was saved.".into());
            }
            if let Some(conversion) = conversion {
                scan.columns[0].units = Some(units.unwrap_or("eV").into());
                super::signals::add(
                    &mut scan,
                    "stored Larix absorption",
                    0,
                    conversion,
                    SignalConversion::Direct { column: 1 },
                );
            } else {
                scan.warnings
                    .push("Unrecognized Larix energy units require an explicit mapping.".into());
            }
        } else {
            scan.warnings.push("This Larix group is not a declared energy/mu absorption pair; inspect its saved quantities before mapping.".into());
        }
        result.scans.push(scan);
    }
    result.warnings.push("Larix session configuration, commands and Python objects are preserved as text; no code or external source paths are evaluated.".into());
    Ok(result)
}

/// JSON Pointer escaping prevents symbol/attribute names from colliding in paths.
fn escape(name: &str) -> String {
    name.replace('~', "~0").replace('/', "~1")
}

fn walk(
    value: &Value,
    path: &str,
    symbol: &str,
    depth: usize,
    budget: &mut usize,
    nodes: &mut usize,
    result: &mut Measurement,
) -> Result<(), ReadError> {
    *nodes += 1;
    if depth > 64 || *nodes > 1_000_000 {
        return Err(error(path, "object nesting/node limit exceeded"));
    }
    if let Some(class) = value.get("__class__").and_then(Value::as_str) {
        if class == "b64ndarray" || class == "Array" {
            let (mut dataset, imprecise) = array::decode(value, path, budget)?;
            dataset
                .attributes
                .insert("larix.symbol".into(), symbol.into());
            if imprecise {
                result.warnings.push(format!("Larix {path}: integers outside exact f64 precision have rounded numeric views; exact dtype bytes remain in larix.bytes_base64."));
            }
            result.datasets.push(dataset);
            return Ok(());
        }
    }
    match value {
        Value::Object(fields) => {
            for (key, child) in fields {
                if key != "__class__" {
                    walk(
                        child,
                        &format!("{path}/{}", escape(key)),
                        symbol,
                        depth + 1,
                        budget,
                        nodes,
                        result,
                    )?;
                }
            }
        }
        Value::Array(items) => {
            for (i, child) in items.iter().enumerate() {
                walk(
                    child,
                    &format!("{path}/{i}"),
                    symbol,
                    depth + 1,
                    budget,
                    nodes,
                    result,
                )?;
            }
        }
        _ => (),
    }
    Ok(())
}
