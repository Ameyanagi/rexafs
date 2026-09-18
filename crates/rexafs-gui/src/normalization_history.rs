//! Independent normalization results. Only small receipts stay in memory;
//! original arrays, model, resolved settings and atomic identity are immutable artifacts.
use crate::{analysis_store, group_identity::GroupId, params::PipelineParams};
use rexafs::prelude::{NormalizationMethod, XASSpectrum};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct NormalizationHistory {
    pub entries: Vec<NormalizationReceipt>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct NormalizationReceipt {
    pub group: GroupId,
    pub label: String,
    pub method: String,
    pub input_digest: String,
    pub path: PathBuf,
    pub digest: String,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct NormalizationRecord {
    pub schema: u32,
    pub group: GroupId,
    pub label: String,
    pub software_version: String,
    pub settings: PipelineParams,
    pub energy: Vec<f64>,
    pub mu: Vec<f64>,
    pub normalization: NormalizationMethod,
}
impl NormalizationRecord {
    pub fn new(
        group: GroupId,
        label: String,
        settings: PipelineParams,
        sp: &XASSpectrum,
    ) -> Result<Self, String> {
        let normalization = sp
            .normalization
            .clone()
            .ok_or("Normalize the current spectrum first")?;
        let energy: Vec<f64> = sp
            .energy
            .as_ref()
            .ok_or("Missing energy")?
            .iter()
            .copied()
            .collect();
        let mu: Vec<f64> = sp
            .mu
            .as_ref()
            .ok_or("Missing absorption")?
            .iter()
            .copied()
            .collect();
        if sp.norm().is_none_or(|n| n.len() != energy.len()) || energy.len() != mu.len() {
            return Err("No complete normalization result to preserve".into());
        }
        Ok(Self {
            schema: 1,
            group,
            label,
            software_version: env!("CARGO_PKG_VERSION").into(),
            settings,
            energy,
            mu,
            normalization,
        })
    }
    pub fn method(&self) -> &'static str {
        match self.normalization {
            NormalizationMethod::MBack(_) => "MBACK",
            _ => "Polynomial",
        }
    }
    pub fn input_digest(&self) -> String {
        let mut bytes = Vec::with_capacity((self.energy.len() + self.mu.len()) * 8);
        for v in self.energy.iter().chain(&self.mu) {
            bytes.extend_from_slice(&v.to_bits().to_le_bytes());
        }
        analysis_store::digest(&bytes)
    }
    pub fn spectrum(&self) -> Result<XASSpectrum, String> {
        let mut sp = XASSpectrum::from_arrays(&self.energy, &self.mu).map_err(|e| e.to_string())?;
        sp.normalization = Some(self.normalization.clone());
        Ok(sp)
    }
}
pub fn root() -> Result<PathBuf, String> {
    let parent = crate::settings::env_var_os("SETTINGS")
        .and_then(|p| PathBuf::from(p).parent().map(Path::to_path_buf))
        .or_else(crate::settings::app_dir)
        .ok_or("Application storage unavailable")?;
    Ok(parent.join("normalization-results"))
}
pub fn retain(root: &Path, record: &NormalizationRecord) -> Result<NormalizationReceipt, String> {
    let (path, digest) = analysis_store::retain(root, record)?;
    Ok(NormalizationReceipt {
        group: record.group.clone(),
        label: record.label.clone(),
        method: record.method().into(),
        input_digest: record.input_digest(),
        path,
        digest,
    })
}
pub fn read(receipt: &NormalizationReceipt) -> Result<NormalizationRecord, String> {
    let r: NormalizationRecord = analysis_store::read(&receipt.path, &receipt.digest)?;
    if r.schema != 1
        || r.group != receipt.group
        || r.input_digest() != receipt.input_digest
        || r.method() != receipt.method
    {
        return Err("Saved normalization identity mismatch".into());
    }
    Ok(r)
}
impl NormalizationHistory {
    pub fn insert(&mut self, receipt: NormalizationReceipt) {
        if !self
            .entries
            .iter()
            .any(|r| r.group == receipt.group && r.digest == receipt.digest)
        {
            self.entries.push(receipt);
        }
    }
    pub fn relocate(
        &mut self,
        f: &mut impl FnMut(&Path) -> Result<PathBuf, String>,
    ) -> Result<(), String> {
        for entry in &mut self.entries {
            entry.path = f(&entry.path)?;
        }
        Ok(())
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::params::{RequiredStage, prepare_arrays};
    use rexafs::prelude::*;
    pub(crate) fn synthetic() -> (Vec<f64>, Vec<f64>, PipelineParams) {
        let energy: Vec<_> = (0..=700).map(|i| 8679. + 2. * i as f64).collect();
        let reference = rexafs::atomic::AtomicData::new()
            .unwrap()
            .f2("Cu", &energy)
            .unwrap();
        let mu: Vec<_> = energy
            .iter()
            .zip(&reference.values)
            .map(|(e, f)| (f + 0.7 + 0.002 * (e - 8979.) + 1e-6 * (e - 8979.).powi(2)) / 3.2)
            .collect();
        let mut model = MBack::for_edge("Cu", "K")
            .pre_edge(-300.0..=-50.0)
            .post_edge(100.0..=1100.0);
        model.options.reference = Some(reference.reference);
        (
            energy,
            mu,
            PipelineParams {
                e0: Some(8979.),
                mback: Some(model.options),
                ..Default::default()
            },
        )
    }
    #[test]
    fn mback_pipeline_matches_core_and_retains_exact_preparation() {
        let (energy, mu, settings) = synthetic();
        let sp = prepare_arrays(
            energy.clone(),
            mu.clone(),
            &settings,
            RequiredStage::Normalized,
        )
        .unwrap();
        let result = MBack {
            e0: settings.e0,
            options: settings.mback.clone().unwrap(),
            ..Default::default()
        }
        .fit(&energy, &mu)
        .unwrap();
        assert_eq!(sp.norm().unwrap().as_slice(), result.norm);
        assert_eq!(sp.flat().unwrap().as_slice(), result.flat);
        assert!(sp.background.is_none());
        let ranges = crate::params::normalization_ranges(&sp).unwrap();
        assert_eq!(ranges, [-300., -50., 100., 1100.]);
        let prepared = crate::series_measurements::resolved_preparation(&sp);
        assert_eq!(
            prepared["normalization"]["resolved"]["reference"],
            serde_json::to_value(&result.reference).unwrap()
        );
        assert_eq!(prepared["normalization"]["resolved"]["scale"], result.scale);
        let mut changed = settings.clone();
        changed.pre_edge_end = Some(-60.);
        let sp2 = prepare_arrays(
            energy.clone(),
            mu.clone(),
            &changed,
            RequiredStage::Normalized,
        )
        .unwrap();
        assert_eq!(crate::params::normalization_ranges(&sp2).unwrap()[1], -60.);
        assert_ne!(changed.fingerprint(), settings.fingerprint());
        assert_eq!(changed.raw_fingerprint(), settings.raw_fingerprint());
        changed
            .mback
            .as_mut()
            .unwrap()
            .reference
            .as_mut()
            .unwrap()
            .data
            .data_version = "unavailable".into();
        assert!(
            prepare_arrays(energy, mu, &changed, RequiredStage::Normalized)
                .unwrap_err()
                .contains("reference")
        );
    }
    #[test]
    fn independent_normalizations_survive_embedded_save_and_detect_corruption() {
        let tmp = tempfile::tempdir().unwrap();
        let (energy, mu, settings) = synthetic();
        let mback = prepare_arrays(
            energy.clone(),
            mu.clone(),
            &settings,
            RequiredStage::Normalized,
        )
        .unwrap();
        let mut poly_settings = settings.clone();
        poly_settings.mback = None;
        let poly = prepare_arrays(energy, mu, &poly_settings, RequiredStage::Normalized).unwrap();
        let id = GroupId::new_result();
        let root = tmp.path().join("artifacts");
        let mut archive = NormalizationHistory::default();
        let mut expected = Vec::new();
        for (params, sp) in [(poly_settings, poly), (settings, mback)] {
            let record =
                NormalizationRecord::new(id.clone(), "Synthetic Cu".into(), params, &sp).unwrap();
            let receipt = retain(&root, &record).unwrap();
            expected.push(
                record
                    .spectrum()
                    .unwrap()
                    .norm()
                    .unwrap()
                    .as_slice()
                    .to_vec(),
            );
            archive.insert(receipt.clone());
            archive.insert(receipt);
        }
        assert_eq!(archive.entries.len(), 2);
        assert_eq!(
            archive.entries[0].input_digest,
            archive.entries[1].input_digest
        );
        let project = crate::project::ProjectFile {
            normalizations: archive,
            ..Default::default()
        };
        let path = tmp.path().join("normalizations.rxs");
        crate::project::save_with_storage(&path, &project, crate::project::DataStorage::Embedded)
            .unwrap();
        std::fs::remove_dir_all(root).unwrap();
        let restored =
            crate::project::load_with_cache_root(&path, || Ok(tmp.path().join("reopened")))
                .unwrap();
        assert!(restored.raw_files.is_empty());
        for (r, values) in restored.normalizations.entries.iter().zip(expected) {
            assert_eq!(
                read(r)
                    .unwrap()
                    .spectrum()
                    .unwrap()
                    .norm()
                    .unwrap()
                    .as_slice(),
                values
            );
        }
        let r = &restored.normalizations.entries[1];
        std::fs::write(&r.path, b"damaged historical result").unwrap();
        assert!(read(r).err().unwrap().contains("checksum"));
    }
}
