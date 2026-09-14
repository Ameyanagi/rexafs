//! Shared, conservative inference of axis units and detector roles.
use super::{
    signals, EnergyConversion, MeasurementScan, SignalCandidate, SignalConversion, SpectrumMapping,
};

pub(super) fn add(
    scan: &mut MeasurementScan,
    name: &str,
    energy_column: usize,
    energy: EnergyConversion,
    signal: SignalConversion,
) {
    scan.signals.push(SignalCandidate {
        name: name.into(),
        mapping: SpectrumMapping {
            energy_column,
            energy,
            signal,
        },
    });
}

pub(super) fn infer(scan: &mut MeasurementScan) {
    let normalized: Vec<_> = scan
        .columns
        .iter()
        .map(|c| c.name.to_lowercase().replace([' ', '_', '-'], ""))
        .collect();
    let find = |aliases: &[&str]| {
        normalized
            .iter()
            .position(|n| aliases.contains(&n.as_str()))
    };
    let axis = find(&[
        "energy",
        "e",
        "ev",
        "hv",
        "hν",
        "monoenergy",
        "monoenergy(alt)*",
        "monoenergy*",
        "ebraggenergy",
        "achievedenergy",
        "requestedenergy",
        "mono1energyacs",
        "zapenergy",
        "encenergy",
        "energysetpoint.x",
        "shiftedenergy",
    ]);
    let Some(e) = axis else {
        return;
    };
    let unit = scan.columns[e].units.as_deref().unwrap_or("");
    let conversion = match unit.to_lowercase().as_str() {
        "ev" => EnergyConversion::Ev,
        "kev" => EnergyConversion::Kev,
        "" if normalized[e] == "zapenergy" => EnergyConversion::Kev,
        "" => {
            scan.warnings.push("Energy units are not declared; the named energy axis is interpreted as eV. Override the mapping if needed.".into());
            EnergyConversion::Ev
        }
        _ => return,
    };
    let direct = find(&[
        "mu",
        "mutrans",
        "normtrans",
        "xmu",
        "mufluor",
        "normfluor",
        "normalized",
        "ln(i0/i1)",
        "mu01",
    ]);
    if let Some(column) = direct {
        add(
            scan,
            &scan.columns[column].name.clone(),
            e,
            conversion.clone(),
            SignalConversion::Direct { column },
        );
        // Precomputed sample signals take precedence over raw channels. Their
        // scale/corrections remain the source's responsibility.
        return;
    }
    let i0 = find(&[
        "i0",
        "io",
        "monitor",
        "mon",
        "prekbi0",
        "i0eh1",
        "i0detector",
        "i0detectordarkcorrect",
    ]);
    let it = find(&[
        "it",
        "i1",
        "itrans",
        "trans",
        "transmission",
        "i1eh1",
        "i2detectordarkcorrect",
    ]);
    let fluor = find(&[
        "if",
        "iff",
        "ifluor",
        "fluor",
        "fluo",
        "pips",
        "pipsdarkcorrect",
    ]);
    if let (Some(incident), Some(transmitted)) = (i0, it) {
        if scan.columns[transmitted].values.iter().any(|v| *v != 0.) {
            add(
                scan,
                "transmission",
                e,
                conversion.clone(),
                SignalConversion::Transmission {
                    incident,
                    transmitted,
                },
            );
        }
    }
    if let (Some(incident), Some(detector)) = (i0, fluor) {
        if scan.columns[detector].values.iter().any(|v| *v != 0.) {
            add(
                scan,
                "fluorescence",
                e,
                conversion.clone(),
                SignalConversion::Ratio {
                    detectors: vec![detector],
                    incident,
                },
            );
        }
    }
    // A foil downstream of the sample uses It as its incident monitor.
    // Ir/Iref are explicit reference labels; a generic I2 is not sufficient.
    if let (Some(incident), Some(transmitted)) = (it, find(&["ir", "iref"])) {
        if scan.columns[transmitted].values.iter().any(|v| *v != 0.) {
            add(
                scan,
                "reference",
                e,
                conversion.clone(),
                SignalConversion::Transmission {
                    incident,
                    transmitted,
                },
            );
        }
    }
    for name in ["tey", "pey", "cey"] {
        if let (Some(incident), Some(detector)) = (i0, find(&[name])) {
            if scan.columns[detector].values.iter().any(|v| *v != 0.) {
                add(
                    scan,
                    name,
                    e,
                    conversion.clone(),
                    SignalConversion::Ratio {
                        detectors: vec![detector],
                        incident,
                    },
                );
            }
        }
    }
}
