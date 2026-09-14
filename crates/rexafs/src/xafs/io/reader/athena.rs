//! Athena project groups and inert saved metadata.
use super::signals::add;
use super::text::from_rows;
use super::{
    measurement, signals, EnergyConversion, Measurement, MeasurementColumn, MeasurementScan,
    ReadError, SignalConversion,
};
use std::collections::BTreeMap;

pub(super) fn parse(source: &str) -> Result<Measurement, ReadError> {
    let mut scans = Vec::new();
    if source.trim_start().starts_with('{') {
        let doc: serde_json::Value = serde_json::from_str(source)
            .map_err(|e| ReadError::new(format!("Athena JSON: {e}")))?;
        let order = doc
            .get("_____order")
            .and_then(|v| v.as_array())
            .ok_or_else(|| ReadError::new("JSON is not an Athena project: missing _____order"))?;
        for key in order {
            let key = key
                .as_str()
                .ok_or_else(|| ReadError::new("Athena: invalid group order"))?;
            let group = &doc[key];
            let mut scan = MeasurementScan {
                id: key.into(),
                label: group["args"]["label"].as_str().unwrap_or(key).into(),
                columns: Vec::new(),
                header: serde_json::to_string(&group["args"]).unwrap(),
                metadata: BTreeMap::new(),
                signals: Vec::new(),
                warnings: Vec::new(),
            };
            for name in ["x", "y", "i0", "signal", "stddev"] {
                if let Some(array) = group.get(name) {
                    let values: Vec<f64> = array
                        .as_array()
                        .ok_or_else(|| ReadError::new("Athena: expected numeric array"))?
                        .iter()
                        .map(|v| {
                            (if v.is_null() {
                                Some(f64::NAN)
                            } else {
                                v.as_f64()
                            })
                            .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
                            .ok_or_else(|| {
                                ReadError::new(format!(
                                    "Athena group {key}, {name}: invalid numeric value"
                                ))
                            })
                        })
                        .collect::<Result<_, _>>()?;
                    if name != "x"
                        && name != "y"
                        && scan
                            .columns
                            .first()
                            .is_none_or(|c| values.len() != c.values.len())
                    {
                        scan.metadata
                            .insert(format!("athena.{name}"), array.to_string());
                    } else {
                        scan.columns.push(MeasurementColumn {
                            name: name.into(),
                            units: None,
                            values,
                        });
                    }
                }
            }
            if scan.columns.len() < 2 || scan.columns[0].name != "x" || scan.columns[1].name != "y"
            {
                return Err(ReadError::new(format!(
                    "Athena group {key}: missing x/y arrays"
                )));
            }
            if scan.columns[0].values.len() != scan.columns[1].values.len() {
                return Err(ReadError::new(format!(
                    "Athena group {key}: x/y lengths differ"
                )));
            }
            let args = &group["args"];
            let is_pixel =
                args["is_pixel"].as_str() == Some("1") || args["is_pixel"].as_i64() == Some(1);
            let is_xmu = args["is_xmu"].as_str() == Some("1") || args["is_xmu"].as_i64() == Some(1);
            if is_xmu && !is_pixel {
                add(
                    &mut scan,
                    "stored absorption",
                    0,
                    EnergyConversion::Ev,
                    SignalConversion::Direct { column: 1 },
                );
            } else {
                scan.warnings.push("Historical project axis is not declared mu(E); calibration or interpretation is required.".into());
            }
            scans.push(scan);
        }
    } else {
        let project = super::super::AthenaProject::from_text(source)
            .map_err(|e| ReadError::new(format!("Athena: {e}")))?;
        let project_extra = project.extra.join("\n");
        let journal = serde_json::to_string(&project.journal).unwrap();
        for group in project.groups {
            let header = group
                .args
                .iter()
                .map(|(k, v)| format!("{k}: {v}"))
                .collect::<Vec<_>>()
                .join("\n");
            let flag = |key: &str| {
                group
                    .args
                    .iter()
                    .find(|(k, _)| k == key)
                    .map(|(_, v)| v.to_string().trim_matches('\'').to_owned())
                    .is_some_and(|v| v == "1")
            };
            let is_xmu = flag("is_xmu") && !flag("is_pixel");
            if group.x.len() != group.y.len() {
                return Err(ReadError::new(format!(
                    "Athena group {}: x/y lengths differ",
                    group.tag
                )));
            }
            let rows = group
                .x
                .iter()
                .zip(&group.y)
                .map(|(&x, &y)| vec![x, y])
                .collect();
            let mut scan = from_rows(&group.tag, header, rows, vec!["x".into(), "y".into()])?;
            for (name, values) in [
                ("i0", group.i0),
                ("signal", group.signal),
                ("stddev", group.stddev),
            ] {
                if let Some(values) = values {
                    if values.len() == scan.columns[0].values.len() {
                        scan.columns.push(MeasurementColumn {
                            name: name.into(),
                            units: None,
                            values,
                        });
                    } else {
                        scan.metadata.insert(
                            format!("athena.{name}"),
                            serde_json::to_string(&values).unwrap(),
                        );
                    }
                }
            }
            scan.metadata
                .insert("athena.extra".into(), group.extra.join("\n"));
            scan.metadata
                .insert("athena.journal".into(), journal.clone());
            scan.metadata
                .insert("athena.project_extra".into(), project_extra.clone());
            if !project_extra.is_empty() {
                scan.warnings.push("Historical project statements and non-list journals are retained as opaque metadata; no embedded expressions are executed.".into());
            }
            scan.label = group.label;
            if is_xmu {
                add(
                    &mut scan,
                    "stored absorption",
                    0,
                    EnergyConversion::Ev,
                    SignalConversion::Direct { column: 1 },
                );
            } else {
                scan.warnings.push("Historical project axis requires explicit interpretation; pixel axes are not energy.".into());
            }
            scans.push(scan);
        }
    }
    if scans.is_empty() {
        return Err(ReadError::new("Athena project contains no groups"));
    }
    Ok(measurement("athena", scans))
}
