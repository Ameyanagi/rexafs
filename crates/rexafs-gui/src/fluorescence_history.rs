//! Immutable correction evidence. Descendants retain references to the original
//! calculation; a reference never claims their current arrays were corrected again.
use crate::{
    analysis_store,
    params::{OperationInput, PipelineParams},
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const XANES_ONLY: &str =
    "Fluorescence-corrected lineage: XANES only. Use the original for EXAFS.";
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CorrectionReceipt {
    pub path: PathBuf,
    pub digest: String,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct CorrectionRecord {
    pub schema: u32,
    pub input: OperationInput,
    pub source_digest: String,
    pub settings: PipelineParams,
    pub result: rexafs::FluorescenceCorrectionResult,
}
pub fn root() -> Result<PathBuf, String> {
    let parent = crate::settings::env_var_os("SETTINGS")
        .and_then(|p| PathBuf::from(p).parent().map(Path::to_path_buf))
        .or_else(crate::settings::app_dir)
        .ok_or("Application storage unavailable")?;
    Ok(parent.join("fluorescence-results"))
}
pub fn retain(root: &Path, record: &CorrectionRecord) -> Result<CorrectionReceipt, String> {
    let (path, digest) = analysis_store::retain(root, record)?;
    Ok(CorrectionReceipt { path, digest })
}
pub fn read(receipt: &CorrectionReceipt) -> Result<CorrectionRecord, String> {
    let record: CorrectionRecord = analysis_store::read(&receipt.path, &receipt.digest)?;
    if record.schema != 1 {
        return Err("Unsupported fluorescence history schema".into());
    }
    Ok(record)
}
pub fn combine<'a>(
    sources: impl IntoIterator<Item = &'a [CorrectionReceipt]>,
) -> Vec<CorrectionReceipt> {
    let mut out: Vec<CorrectionReceipt> = Vec::new();
    for source in sources {
        for receipt in source {
            if !out.iter().any(|r| r.digest == receipt.digest) {
                out.push(receipt.clone());
            }
        }
    }
    out
}
