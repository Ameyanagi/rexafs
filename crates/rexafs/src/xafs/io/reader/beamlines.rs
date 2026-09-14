//! Header-specific conventions independently checked against Demeter plugins
//! (06afc8d) and Larch beamline readers (e3c9328). See
//! `doc/reference-format-audit.md` for pinned sources and qualification limits.
use super::signals::add;
use super::{signals, text, EnergyConversion, MeasurementScan, ReadError, SignalConversion};

pub(super) fn identify(text: &str) -> Option<&'static str> {
    let mut lines = text.lines();
    let first = lines.next()?.trim();
    let second = lines.next().unwrap_or("");
    if first.contains("CLS Data Acquisition") {
        Some("cls_acquisition")
    } else if first.starts_with("SSRL") && first.contains("MicroEXAFS Data Collector") {
        Some("ssrl_micro")
    } else if first.contains("BM23") && first.contains("E.S.R.F.") {
        Some("bm23")
    } else if first.contains("Diamond") && second.contains("B18-CORE XAS") {
        Some("b18")
    } else if text.contains("exafsscan") && text.contains("exafs_region") {
        Some("aps_12bm")
    } else {
        None
    }
}

/// Match an unambiguous source heading, without discarding or renaming columns.
fn unique(scan: &MeasurementScan, predicate: impl Fn(&str) -> bool) -> Option<usize> {
    let mut matches = scan
        .columns
        .iter()
        .enumerate()
        .filter_map(|(i, c)| predicate(&c.name.to_ascii_lowercase()).then_some(i));
    let first = matches.next()?;
    matches.next().is_none().then_some(first)
}

fn named(scan: &MeasurementScan, name: &str) -> Option<usize> {
    unique(scan, |n| n == name)
}

fn ratio(scan: &mut MeasurementScan, energy: usize, incident: usize, detector: usize) {
    let label = format!(
        "{} / {} (uncorrected)",
        scan.columns[detector].name, scan.columns[incident].name
    );
    add(
        scan,
        &label,
        energy,
        EnergyConversion::Ev,
        SignalConversion::Ratio {
            detectors: vec![detector],
            incident,
        },
    );
}

pub(super) fn configure(scan: &mut MeasurementScan, format: &str) -> Result<(), ReadError> {
    if format == "aps_12bm" {
        let heading = scan
            .header
            .lines()
            .rev()
            .find(|s| s.contains("1_Energy"))
            .ok_or_else(|| ReadError::new("APS 12BM: missing numbered Energy heading"))?;
        let mut labels: Vec<(String, Option<String>)> = Vec::new();
        for word in heading.split_whitespace() {
            if let Some((number, name)) = word.split_once('_') {
                if number.parse::<usize>().ok() == Some(labels.len() + 1) {
                    labels.push((name.into(), None));
                    continue;
                }
            }
            if let Some(unit) = word.strip_prefix('(').and_then(|s| s.strip_suffix(')')) {
                if let Some((_, units)) = labels.last_mut() {
                    *units = Some(unit.into());
                }
            }
        }
        if labels.len() != scan.columns.len() {
            return Err(ReadError::new(
                "APS 12BM: numbered headings disagree with the numeric row width",
            ));
        }
        for (col, (name, units)) in scan.columns.iter_mut().zip(labels) {
            col.name = name;
            col.units = units;
        }
        scan.signals.clear();
        super::signals::infer(scan);
    }
    if format == "ssrl_ascii" {
        if let Some(e) = named(scan, "achieved energy") {
            for signal in &mut scan.signals {
                signal.mapping.energy_column = e;
            }
            scan.warnings.push("SSRL uses achieved energy when available, following the Larch beamline reader. Requested energy and detector offsets remain in the original record.".into());
        }
    }
    if format == "cls_acquisition" {
        // #(1) describes event-1 data. The following macro headings and #(2)
        // background event have different meanings and must not shift columns.
        let labels = scan.header.lines().find_map(|line| {
            let line = line.trim().strip_prefix("#(1)")?;
            if line.contains("$(") {
                return None;
            }
            let labels: Vec<_> = line
                .split('"')
                .skip(1)
                .step_by(2)
                .map(str::to_owned)
                .collect();
            (!labels.is_empty()).then_some(labels)
        });
        scan.signals.clear();
        if let Some(labels) = labels {
            if labels.len() != scan.columns.len() {
                return Err(ReadError::new(
                    "CLS acquisition: event-1 headings disagree with the numeric row width",
                ));
            }
            for (column, label) in scan.columns.iter_mut().zip(labels) {
                column.name = label;
            }
        }
        // Demeter HXMA.pm uses Energy:sp and mcs04/05/03. Preserve all other
        // axes, event IDs, monitors and the original quoted EPICS PV names.
        if scan.header.contains("BL1606-I") {
            let energy = unique(scan, |n| n.ends_with(":energy:sp"));
            let incident = unique(scan, |n| n.ends_with(":mcs04:fbk"));
            if let (Some(e), Some(i0)) = (energy, incident) {
                if let Some(it) = unique(scan, |n| n.ends_with(":mcs05:fbk")) {
                    add(
                        scan,
                        "HXMA transmission",
                        e,
                        EnergyConversion::Ev,
                        SignalConversion::Transmission {
                            incident: i0,
                            transmitted: it,
                        },
                    );
                }
                if let Some(detector) = unique(scan, |n| n.ends_with(":mcs03:fbk")) {
                    ratio(scan, e, i0, detector);
                }
            }
        }
        scan.warnings.push("CLS event columns are retained. HXMA uses the Energy:sp axis in eV by the documented acquisition convention; unknown PV roles require manual selection. Background events are metadata; no correction is applied.".into());
    } else if format == "ssrl_micro" {
        if let (Some(e), Some(i0)) = (named(scan, "requested energy"), named(scan, "i0")) {
            for i in 0..scan.columns.len() {
                let n = scan.columns[i].name.to_ascii_lowercase();
                if n.strip_prefix("sca").is_some_and(|suffix| {
                    !suffix.is_empty() && suffix.chars().all(|c| c.is_ascii_digit() || c == '.')
                }) {
                    ratio(scan, e, i0, i);
                }
            }
        }
        scan.warnings.push("MicroEXAFS retains every SCA signal and ICR count-rate column. Select detector elements explicitly; no sum, offset subtraction or dead-time correction is applied.".into());
    } else if format == "bm23" {
        if let Some(e) = unique(scan, |n| {
            matches!(n, "e_kev_" | "e" | "e(kev)" | "energy" | "energy(kev)")
        }) {
            let units = match scan.columns[e]
                .units
                .as_deref()
                .map(str::to_ascii_lowercase)
                .as_deref()
            {
                Some("ev") => EnergyConversion::Ev,
                Some("kev") | None => EnergyConversion::Kev,
                _ => return Ok(()),
            };
            for signal in &mut scan.signals {
                signal.mapping.energy_column = e;
                signal.mapping.energy = units.clone();
            }
            if let (Some(i0), Some(it)) =
                (named(scan, "i0"), unique(scan, |n| n == "i1" || n == "it"))
            {
                if scan.signals.is_empty() {
                    add(
                        scan,
                        "BM23 transmission",
                        e,
                        units,
                        SignalConversion::Transmission {
                            incident: i0,
                            transmitted: it,
                        },
                    );
                }
            }
        }
        scan.warnings.push("Historical BM23 energy defaults to keV only when units are undeclared; explicit eV/keV headings take precedence. All rows and original headings remain intact.".into());
    } else if format == "b18" {
        scan.warnings.push("B18 rows and all detector columns are retained without thinning. Select fluorescence elements explicitly; the historical Demeter 36-element sum is not applied automatically.".into());
    }

    if scan
        .header
        .lines()
        .next()
        .is_some_and(|s| s.trim_start().starts_with("XDAC"))
    {
        if let Some(e) = named(scan, "energy") {
            let pairs: Option<Vec<_>> = (1..=4)
                .map(|channel| {
                    let incident = if channel == 1 {
                        unique(scan, |n| n == "i0" || n == "i01")
                    } else {
                        named(scan, &format!("i0{channel}"))
                    }?;
                    Some((incident, named(scan, &format!("it{channel}"))?))
                })
                .collect();
            if let Some(pairs) = pairs {
                scan.signals.clear();
                for (i, (incident, transmitted)) in pairs.into_iter().enumerate() {
                    add(
                        scan,
                        &format!("sample {} transmission", i + 1),
                        e,
                        EnergyConversion::Ev,
                        SignalConversion::Transmission {
                            incident,
                            transmitted,
                        },
                    );
                }
                scan.warnings.push("Four independent XDAC transmission pairs are available. Choose a sample; reference geometry and saved energy shifts are not inferred.".into());
            } else if let Some(i0) = named(scan, "i0") {
                let detectors: Vec<_> = (1..=4)
                    .filter_map(|i| named(scan, &format!("ifch{i}")))
                    .collect();
                if !detectors.is_empty() {
                    for detector in detectors {
                        ratio(scan, e, i0, detector);
                    }
                    scan.warnings.push("XDAC multi-element signals are uncorrected ratios. ROI, slow/fast counts, integration time and original offsets are retained. Apply any required detector correction separately.".into());
                }
            }
        }
    }
    if scan.header.contains("User = bmexafs") {
        scan.warnings.push("CMC raw and previously corrected columns are retained separately, including nonfinite values. Select corrected channels explicitly; no dark-current estimate or replacement of NaN with zero is applied.".into());
    }
    Ok(())
}
