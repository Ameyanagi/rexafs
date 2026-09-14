//! Historical X10C, X15B and SSRL binary measurement layouts.
use super::signals::{add, infer};
use super::text::from_rows;
use super::{
    measurement, signals, text, EnergyConversion, Measurement, ReadError, SignalConversion,
};

pub(super) fn parse(bytes: &[u8]) -> Option<Result<Measurement, ReadError>> {
    if bytes.starts_with(&[212, 0, 0, 0]) {
        return Some(x15b(bytes));
    }
    if bytes.starts_with(b"SSRL") && bytes.get(..80).is_some_and(|s| s.contains(&0)) {
        return Some(ssrl(bytes));
    }
    if bytes.starts_with(b"EXAFS") && bytes.windows(10).any(|s| s == b"DATA START") {
        let raw = String::from_utf8_lossy(bytes).replace('\0', "");
        let Some((header, body)) = raw.split_once("DATA START") else {
            unreachable!()
        };
        let regex = regex::Regex::new(r"([eE][+-]\d{1,2})-").unwrap();
        let body = regex.replace_all(body, "$1 -");
        let result = (|| {
            let rows = body
                .lines()
                .filter(|s| !s.trim().is_empty())
                .map(|s| {
                    super::text::numbers(s)
                        .ok_or_else(|| ReadError::new("X10C: invalid fixed-width numeric record"))
                })
                .collect::<Result<Vec<_>, _>>()?;
            let width = rows.first().map_or(0, Vec::len);
            let mut scan = from_rows(
                "scan_1",
                header.into(),
                rows,
                (0..width).map(|i| format!("column_{}", i + 1)).collect(),
            )?;
            if width >= 6 {
                scan.columns[0].name = "energy".into();
                add(
                    &mut scan,
                    "transmission",
                    0,
                    EnergyConversion::Ev,
                    SignalConversion::Transmission {
                        incident: 3,
                        transmitted: 5,
                    },
                );
            }
            Ok(measurement("x10c", vec![scan]))
        })();
        return Some(result);
    }
    None
}

// Historical layouts independently implemented from the format descriptions
// credited to Tim Darling (X15B) and Tsu-Chien Weng/Sam Webb/Bruce Ravel (SSRL):
// https://github.com/bruceravel/demeter/blob/master/lib/Demeter/Plugins/X15B.pm
// https://github.com/bruceravel/demeter/blob/master/lib/Demeter/Plugins/SSRLB.pm
fn x15b(bytes: &[u8]) -> Result<Measurement, ReadError> {
    let body = bytes
        .get(212..)
        .ok_or_else(|| ReadError::new("X15B: truncated 212-byte header"))?;
    if body.len() % 64 != 0 {
        return Err(ReadError::new("X15B: truncated 64-byte record"));
    }
    let rows = body
        .as_chunks::<64>()
        .0
        .iter()
        .map(|row| {
            row[4..]
                .as_chunks::<4>()
                .0
                .iter()
                .map(|v| f32::from_le_bytes(*v) as f64)
                .collect()
        })
        .collect();
    let mut scan = from_rows(
        "scan_1",
        String::from_utf8_lossy(&bytes[..212]).replace('\0', " "),
        rows,
        (0..15).map(|i| format!("column_{}", i + 1)).collect(),
    )?;
    for (i, n) in [
        (0, "energy"),
        (6, "i0"),
        (8, "narrow"),
        (9, "wide"),
        (10, "it"),
    ] {
        scan.columns[i].name = n.into();
    }
    add(
        &mut scan,
        "transmission",
        0,
        EnergyConversion::Ev,
        SignalConversion::Transmission {
            incident: 6,
            transmitted: 10,
        },
    );
    add(
        &mut scan,
        "narrow fluorescence",
        0,
        EnergyConversion::Ev,
        SignalConversion::Ratio {
            detectors: vec![8],
            incident: 6,
        },
    );
    Ok(measurement("x15b", vec![scan]))
}
fn ssrl(bytes: &[u8]) -> Result<Measurement, ReadError> {
    let header = bytes
        .get(..800)
        .ok_or_else(|| ReadError::new("SSRL binary: truncated header"))?;
    let text = String::from_utf8_lossy(header).replace('\0', " ");
    let info = String::from_utf8_lossy(&header[80..120]);
    let dims: Vec<usize> = info
        .split_whitespace()
        .filter_map(|s| s.parse().ok())
        .collect();
    if dims.len() != 2 || dims[1] < 3 || dims[1] > 4096 {
        return Err(ReadError::new("SSRL binary: invalid PTS/COLS record"));
    }
    let (n, w) = (dims[0], dims[1]);
    let offset = 800 + w * 28 + 16;
    let len = n
        .checked_mul(w)
        .and_then(|s| s.checked_mul(4))
        .ok_or_else(|| ReadError::new("SSRL binary dimensions overflow"))?;
    let body = bytes
        .get(
            offset
                ..offset
                    .checked_add(len)
                    .ok_or_else(|| ReadError::new("SSRL binary size overflow"))?,
        )
        .ok_or_else(|| ReadError::new("SSRL binary: truncated samples"))?;
    let labels = bytes
        .get(800 + w * 8..800 + w * 28)
        .ok_or_else(|| ReadError::new("SSRL binary: truncated labels"))?
        .as_chunks::<20>()
        .0
        .iter()
        .map(|b| {
            String::from_utf8_lossy(b)
                .replace('\0', "")
                .trim()
                .to_owned()
        })
        .collect();
    let vax = text.lines().next().unwrap_or("").contains("1.1");
    let rows = body
        .chunks_exact(w * 4)
        .map(|r| {
            r.as_chunks::<4>()
                .0
                .iter()
                .map(|b| {
                    if vax {
                        f32::from_le_bytes([b[2], b[3], b[0], b[1]]) as f64 / 4.
                    } else {
                        f32::from_le_bytes(*b) as f64
                    }
                })
                .collect()
        })
        .collect();
    let mut scan = from_rows("scan_1", text, rows, labels)?;
    scan.warnings.push("Historical SSRL binary: decoded legacy float representation; stored detector offsets and weights have not been applied.".into());
    infer(&mut scan);
    Ok(measurement("ssrl_binary", vec![scan]))
}
