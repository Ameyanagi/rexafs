//! Readers and interchange formats for measured XAS data.
//!
//! For automatic import since 0.2.6, start with [`read_measurement`] (path) or
//! [`parse_measurement`] (bytes), inspect scans/warnings, then select a mapping.
//! The shared reader covers beamline tables, Athena, XTUNES and HDF5 containers.
//!
//! [`read_qas_transmission`] reads whitespace-delimited energy/I0/It columns.
//! [`XdiFile`] imports self-described XAS Data Interchange files and retains
//! metadata; [`AthenaProject`] reads and writes Athena `.prj` collections.
//! These readers return owned data and do not normalize or transform it.
//!
//! The [`xafs_json`] and [`xafs_bson`] modules serialize legacy Rust group
//! envelopes. They are separate from the desktop `.rxs` project format and do
//! not provide that format's portability, migration, backup or atomic-save policy.

#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]

pub mod athena;
pub mod reader;
pub mod xafs_bson;
pub mod xafs_json;
pub mod xasdatatype;
pub mod xdi;

pub use athena::{AthenaGroup, AthenaParams, AthenaProject, AthenaValue};
pub use load_spectrum_QAS_trans as read_qas_transmission;
pub use reader::{
    parse_measurement, read_measurement, ColumnSelector, EnergyConversion, Measurement,
    MeasurementColumn, MeasurementDataset, MeasurementScan, ReadError, SignalCandidate,
    SignalConversion, SignalSelection, SpectrumMapping, SpectrumSelection,
};
pub use xdi::{XdiColumn, XdiError, XdiFile, XdiHeader, XdiSignal};

use crate::xafs::errors::IOError;
use crate::xafs::xasspectrum::XASSpectrum;
use data_reader::reader::{load_txt_f64, Delimiter, ReaderParams};
use std::error::Error;
use std::path::Path;

/// Read a QAS-style transmission scan into an unprocessed, owned spectrum.
///
/// The first three whitespace-delimited columns must be energy (eV), incident
/// intensity `I0`, and transmitted intensity `It`; lines starting with `#` are
/// comments. Extra columns are ignored. The stored absorption is the natural
/// logarithm `mu = ln(I0 / It)`, a dimensionless optical thickness. Dividing by
/// sample thickness to obtain an absorption coefficient is the caller's choice.
/// See [Newville, Fundamentals of XAFS (2014)](https://doi.org/10.2138/rmg.2014.78.2)
/// for the transmission measurement convention.
/// Both intensities should be positive for a physically meaningful transmission
/// measurement. This legacy reader does not reject invalid intensity ratios:
/// they may yield non-finite absorption that later processing rejects.
///
/// Returns [`IOError`] for unreadable/invalid tables or fewer than three columns.
/// Energy is sorted together with absorption by the legacy spectrum setter;
/// duplicate energies are not removed. Also exported as [`read_qas_transmission`].
#[allow(non_snake_case)]
pub fn load_spectrum_QAS_trans<P: AsRef<Path>>(path: P) -> Result<XASSpectrum, IOError> {
    let path_ref = path.as_ref();
    let path_string = path_ref.to_string_lossy().to_string();
    let params = ReaderParams {
        comments: Some(b'#'),
        delimiter: Delimiter::WhiteSpace,
        ..Default::default()
    };

    let data = load_txt_f64(&path_string, &params).map_err(|_e| IOError::ReadFailed {
        path: path_ref.display().to_string(),
        kind: std::io::ErrorKind::Other,
    })?;
    if data.get_num_fields() < 3 {
        return Err(IOError::ReadFailed {
            path: path_ref.display().to_string(),
            kind: std::io::ErrorKind::InvalidData,
        });
    }
    let energy = data.get_col(0);
    let i0 = data.get_col(1);
    let it = data.get_col(2);

    let mut xafs_group = XASSpectrum::new();
    xafs_group.set_spectrum(
        energy,
        i0.iter()
            .zip(it)
            .map(|(i0, it)| (i0 / it).ln())
            .collect::<Vec<_>>(),
    );

    Ok(xafs_group)
}

mod tests {
    use super::*;

    const TOP_DIR: &str = env!("CARGO_MANIFEST_DIR");

    #[test]
    fn test_load_spectrum() {
        let path = String::from(TOP_DIR) + "/tests/testfiles/Ru_QAS.dat";
        let result = load_spectrum_QAS_trans(path).unwrap();
        println!("{:?}", result);
    }
}
