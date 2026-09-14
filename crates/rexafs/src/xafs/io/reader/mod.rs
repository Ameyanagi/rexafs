//! Universal, content-detected measurement import (unreleased).
//!
//! Start with [`read_measurement`] or [`parse_measurement`]. Reading retains
//! original columns, ordering and headers. Convert an individual scan with
//! [`MeasurementScan::to_spectrum`]; ambiguous signals require an explicit
//! [`SpectrumMapping`]. No normalization, background subtraction, detector
//! correction, resampling or file writes occur. See the source-checkout
//! `doc/measurement-reader.md` guide for format coverage and conventions.

use std::{collections::BTreeMap, io::Read, path::Path};

mod athena;
mod beamlines;
mod binary;
mod hdf5;
mod larix;
mod model;
mod selection;
mod signals;
mod text;
mod xtunes;

pub use model::{
    EnergyConversion, Measurement, MeasurementColumn, MeasurementDataset, MeasurementScan,
    ReadError, SignalCandidate, SignalConversion, SpectrumMapping,
};
pub use selection::{ColumnSelector, SignalSelection, SpectrumSelection};

/// Read a local measurement file with content-based detection (unreleased).
///
/// Returns owned scans, metadata and numeric datasets, including saved Larix
/// and XTUNES results. Larix stored energy/mu are imported without rerunning its
/// processing. Session commands and Python objects remain inert provenance.
/// The input file is
/// never changed, and no network or external HDF5 links are accessed. Reading
/// and gzip expansion are limited to 256 MiB each. For explicit mappings and
/// signal arithmetic use [`MeasurementScan::arrays`] or `to_spectrum`.
/// Unreadable files, unknown binary formats, corrupt containers and malformed
/// tables return a contextual [`ReadError`]. For browsers use [`parse_measurement`].
pub fn read_measurement(path: impl AsRef<Path>) -> Result<Measurement, ReadError> {
    let path = path.as_ref();
    let file = std::fs::File::open(path)
        .map_err(|e| ReadError::new(format!("{}: {e}", path.display())))?;
    let mut bytes = Vec::new();
    file.take(MAX_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| ReadError::new(e.to_string()))?;
    parse_measurement(&bytes).map_err(|e| ReadError::new(format!("{}: {e}", path.display())))
}

const MAX_BYTES: usize = 256 * 1024 * 1024;

/// Detect and parse measurement bytes without filesystem/network access.
///
/// Same owned result and limits as [`read_measurement`]. Accepts UTF-8 text,
/// legacy encodings recognized by a format adapter, gzip-compressed text,
/// Athena projects, Larix 1.0 sessions and HDF5 containers. Larix commands and
/// saved processing settings remain inert provenance. A recognized damaged format returns
/// its own error instead of silently falling back to unrelated column rules.
/// Gzip must contain exactly one complete member with no trailing data.
/// Input bytes are borrowed during parsing; returned arrays are independent.
pub fn parse_measurement(bytes: &[u8]) -> Result<Measurement, ReadError> {
    if bytes.len() > MAX_BYTES {
        return Err(ReadError::new(
            "Measurement exceeds the 256 MiB input limit",
        ));
    }
    if bytes.starts_with(&[0x1f, 0x8b]) {
        let mut decoded = Vec::new();
        let mut decoder = flate2::bufread::GzDecoder::new(bytes);
        decoder
            .by_ref()
            .take(MAX_BYTES as u64 + 1)
            .read_to_end(&mut decoded)
            .map_err(|e| ReadError::new(format!("Invalid gzip container: {e}")))?;
        if decoded.len() > MAX_BYTES {
            return Err(ReadError::new(
                "Measurement exceeds the 256 MiB gzip expansion limit",
            ));
        }
        if !decoder.into_inner().is_empty() {
            return Err(ReadError::new(
                "Invalid gzip container: trailing data or additional gzip members; supply one measurement per container",
            ));
        }
        if decoded.starts_with(&[0x1f, 0x8b]) {
            return Err(ReadError::new("Nested gzip containers are unsupported"));
        }
        let mut result = parse_measurement(&decoded)?;
        result.warnings.push("Source was gzip-compressed.".into());
        return Ok(result);
    }
    if hdf5_pure::is_hdf5_bytes(bytes) {
        return hdf5::parse(bytes);
    }
    if let Some(result) = binary::parse(bytes) {
        return result;
    }
    let (text, warning) = match std::str::from_utf8(bytes) {
        Ok(text) => (text.to_owned(), None),
        Err(_) => {
            let (decoded, _, errors) = encoding_rs::SHIFT_JIS.decode(bytes);
            if errors {
                return Err(ReadError::new(
                    "Unknown binary format or text encoding; expected UTF-8 or valid Shift-JIS",
                ));
            }
            (
                decoded.into_owned(),
                Some("Decoded legacy text as Shift-JIS; original bytes are unchanged.".into()),
            )
        }
    };
    if text.contains('\0') {
        return Err(ReadError::new(
            "Unrecognized binary file (NUL bytes in text)",
        ));
    }
    let normalized = text
        .trim_start_matches('\u{feff}')
        .replace("\r\n", "\n")
        .replace('\r', "\n");
    let mut result = if normalized.starts_with("##LARIX:") {
        if warning.is_some() {
            return Err(ReadError::new("Larix session text must use UTF-8"));
        }
        larix::parse(&normalized)?
    } else if xtunes::recognizes(&normalized) {
        xtunes::parse(&normalized)?
    } else if normalized.trim_start().starts_with("# Athena project")
        || normalized.trim_start().starts_with('{')
    {
        athena::parse(&normalized)?
    } else {
        text::parse(&normalized)?
    };
    if let Some(w) = warning {
        result.warnings.push(w);
    }
    Ok(result)
}

fn measurement(format: &str, scans: Vec<MeasurementScan>) -> Measurement {
    Measurement {
        format: format.into(),
        scans,
        datasets: Vec::new(),
        metadata: BTreeMap::new(),
        warnings: Vec::new(),
    }
}
