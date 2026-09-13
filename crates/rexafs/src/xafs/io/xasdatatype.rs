//! Legacy versioned envelopes for Rust JSON/BSON group serialization.
//! These types are not the desktop `.rxs` project schema.

use serde::{Deserialize, Serialize};
use version::version;

use crate::xafs::xasgroup::XASGroup;

/// Serialized data-kind discriminator. Writers currently emit only `XASGroup`.
#[derive(Serialize, Deserialize, Default, Debug)]
pub enum XASDataType {
    /// Collection of spectra; the supported group-envelope payload.
    #[default]
    XASGroup,
    /// Historical reserved variant; no separate spectrum-envelope writer is provided.
    XASSpectrum,
}

/// Owned group and descriptive metadata used by the legacy serializers.
/// This schema does not implement the desktop project compatibility policy.
#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(default)]
pub struct XASGroupFile {
    /// Producer version string. Writers replace it with the current crate version.
    pub version: String,
    /// Caller-supplied descriptive file name; it does not control the output path.
    pub name: String,
    /// Payload discriminator, set to `XASGroup` by current writers.
    pub datatype: XASDataType,
    /// Owned spectra, configurations and any serialized cached results.
    pub data: XASGroup,
}

impl XASGroupFile {
    /// Create an empty group envelope with the current crate version and an empty name.
    pub fn new() -> XASGroupFile {
        XASGroupFile {
            version: version!().to_string(),
            name: String::new(),
            datatype: XASDataType::XASGroup,
            data: XASGroup::new(),
        }
    }
}
