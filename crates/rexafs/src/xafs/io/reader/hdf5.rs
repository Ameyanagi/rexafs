//! Bounded HDF5 traversal and owned numeric dataset recovery.
use super::signals::infer;
use super::{
    measurement, signals, Measurement, MeasurementColumn, MeasurementDataset, MeasurementScan,
    ReadError, MAX_BYTES,
};
use std::collections::BTreeMap;

fn attribute(v: hdf5_pure::AttrValue) -> String {
    match v {
        hdf5_pure::AttrValue::AsciiString(s) => s,
        hdf5_pure::AttrValue::String(s) => s,
        v => format!("{v:?}"),
    }
}

pub(super) fn parse(bytes: &[u8]) -> Result<Measurement, ReadError> {
    let file = hdf5_pure::File::from_bytes(bytes.to_vec())
        .map_err(|e| ReadError::new(format!("HDF5: {e}")))?;
    let mut result = measurement("hdf5", Vec::new());
    let mut pending = vec![("/".to_string(), 0usize)];
    let mut budget = MAX_BYTES / 8;
    let mut groups = 0;
    while let Some((path, depth)) = pending.pop() {
        groups += 1;
        if depth > 32 || groups > 10000 {
            return Err(ReadError::new(
                "HDF5 group traversal limit exceeded (possible link cycle)",
            ));
        }
        let group = file
            .group(&path)
            .map_err(|e| ReadError::new(format!("HDF5 {path}: {e}")))?;
        let attrs: BTreeMap<_, _> = group
            .attrs()
            .map_err(|e| ReadError::new(format!("HDF5 {path} attributes: {e}")))?
            .into_iter()
            .map(|(k, v)| (k, attribute(v)))
            .collect();
        let mut columns_by_len: BTreeMap<usize, Vec<MeasurementColumn>> = BTreeMap::new();
        let mut names = match group.datasets() {
            Ok(names) => names,
            Err(e) => {
                result.warnings.push(format!("HDF5 {path}: cannot enumerate this group ({e}); other groups and numeric datasets are retained."));
                continue;
            }
        };
        names.sort();
        for name in names {
            let p = format!("{}/{name}", path.trim_end_matches('/'));
            let ds = group
                .dataset(&name)
                .map_err(|e| ReadError::new(format!("HDF5 {p}: {e}")))?;
            let shape = ds
                .shape()
                .map_err(|e| ReadError::new(format!("HDF5 {p}: {e}")))?;
            let dtype = ds
                .dtype()
                .map_err(|e| ReadError::new(format!("HDF5 {p}: {e}")))?;
            if !matches!(
                dtype,
                hdf5_pure::DType::F32
                    | hdf5_pure::DType::F64
                    | hdf5_pure::DType::I8
                    | hdf5_pure::DType::I16
                    | hdf5_pure::DType::I32
                    | hdf5_pure::DType::I64
                    | hdf5_pure::DType::U8
                    | hdf5_pure::DType::U16
                    | hdf5_pure::DType::U32
                    | hdf5_pure::DType::U64
            ) {
                continue;
            }
            let n = shape
                .iter()
                .try_fold(1usize, |a, &b| {
                    usize::try_from(b).ok().and_then(|b| a.checked_mul(b))
                })
                .ok_or_else(|| ReadError::new(format!("HDF5 {p}: dimensions overflow")))?;
            if n > budget {
                return Err(ReadError::new(
                    "HDF5 numeric arrays exceed the 256 MiB decoded-data limit",
                ));
            }
            budget -= n;
            let values = ds
                .read_f64()
                .map_err(|e| ReadError::new(format!("HDF5 {p}: {e}")))?;
            if values.len() != n {
                return Err(ReadError::new(format!(
                    "HDF5 {p}: decoded values disagree with dimensions"
                )));
            }
            if matches!(dtype, hdf5_pure::DType::I64 | hdf5_pure::DType::U64)
                && values.iter().any(|v| v.abs() > 9_007_199_254_740_992.)
            {
                result.warnings.push(format!("HDF5 {p}: integers beyond 2^53 may lose precision in f64 arrays; original file bytes remain the exact source."));
            }
            let attributes: BTreeMap<_, _> = ds
                .attrs()
                .map_err(|e| ReadError::new(format!("HDF5 {p} attributes: {e}")))?
                .into_iter()
                .map(|(k, v)| (k, attribute(v)))
                .collect();
            if shape.len() == 1 && n > 1 {
                columns_by_len
                    .entry(n)
                    .or_default()
                    .push(MeasurementColumn {
                        name: name.clone(),
                        units: attributes.get("units").cloned(),
                        values: values.clone(),
                    });
            }
            result.datasets.push(MeasurementDataset {
                path: p,
                shape,
                values,
                imaginary: None,
                attributes,
            });
        }
        for (n, columns) in columns_by_len {
            if columns.len() < 2 {
                continue;
            }
            let mut scan = MeasurementScan {
                id: format!("{path}[{n}]"),
                label: path.clone(),
                columns,
                header: String::new(),
                metadata: attrs.clone(),
                signals: Vec::new(),
                warnings: Vec::new(),
            };
            infer(&mut scan);
            if scan.signals.is_empty() {
                scan.warnings.push("Same-length HDF5 channels are available; choose dataset roles and units explicitly.".into());
            }
            result.scans.push(scan);
        }
        let mut children = group
            .groups()
            .map_err(|e| ReadError::new(format!("HDF5 {path}: {e}")))?;
        children.sort();
        for child in children.into_iter().rev() {
            pending.push((format!("{}/{child}", path.trim_end_matches('/')), depth + 1));
        }
    }
    // BLISS stores canonical detector arrays in instrument/<channel>/data;
    // its measurement group may consist entirely of soft-link aliases. Group
    // those canonical vectors by instrument and length without copying offsets.
    let mut instruments: BTreeMap<(String, usize), Vec<MeasurementColumn>> = BTreeMap::new();
    for ds in &result.datasets {
        if ds.shape.len() != 1 || ds.values.len() < 2 {
            continue;
        }
        if let Some((parent, rest)) = ds.path.split_once("/instrument/") {
            if let Some(name) = rest.strip_suffix("/data").filter(|s| !s.contains('/')) {
                instruments
                    .entry((parent.into(), ds.values.len()))
                    .or_default()
                    .push(MeasurementColumn {
                        name: name.into(),
                        units: ds.attributes.get("units").cloned(),
                        values: ds.values.clone(),
                    });
            }
        }
    }
    for ((parent, n), columns) in instruments {
        if columns.len() < 2 {
            continue;
        }
        let mut scan=MeasurementScan{id:format!("{parent}/instrument[{n}]"),label:parent,columns,header:String::new(),metadata:BTreeMap::new(),signals:Vec::new(),warnings:vec!["Channels assembled from canonical instrument datasets; choose calibrated energy and corrected detector channels explicitly when several variants exist.".into()]};
        infer(&mut scan);
        result.scans.push(scan);
    }
    if result.datasets.is_empty() {
        return Err(ReadError::new(
            "HDF5 contains no supported numeric datasets",
        ));
    }
    if result.scans.is_empty() {
        result.warnings.push("Numeric detector arrays are retained; this container has no paired one-dimensional scan channels. It requires axis calibration or detector reduction.".into());
    }
    Ok(result)
}
