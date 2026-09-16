//! Reuse a reviewed mapping only for compatible sources in one intake batch.
use super::{ImportConfig, Measurement, MeasurementImport, ScanChoice, read_source};
use crate::params::DerivedSpectrum;
use sha2::{Digest, Sha256};
use std::{path::PathBuf, sync::Arc};

/// Only paths and content digests are retained for siblings. Large drops do not
/// keep every parsed table in memory while the user reviews the first spectrum.
#[derive(Clone)]
pub(crate) struct BatchReview {
    pub id: usize,
    pub matching: Vec<BatchFile>,
    pub separate: Vec<(PathBuf, String)>,
}

#[derive(Clone)]
pub(crate) struct BatchFile {
    path: PathBuf,
    digest: [u8; 32],
}

/// Match the reader's scientific interpretation, not extensions or row counts.
/// Containers and undeclared generic columns still require individual review.
fn compatible(a: &Measurement, b: &Measurement) -> bool {
    let ([a_scan], [b_scan]) = (a.scans.as_slice(), b.scans.as_slice()) else {
        return false;
    };
    a.format == b.format
        && a.datasets.is_empty()
        && b.datasets.is_empty()
        && a.warnings == b.warnings
        && a_scan.warnings == b_scan.warnings
        && a_scan.metadata.get("9809_modes") == b_scan.metadata.get("9809_modes")
        && a_scan.signals == b_scan.signals
        && (a.format != "9809"
            || crystal_spacing(&a_scan.header) == crystal_spacing(&b_scan.header))
        && a_scan.columns.len() == b_scan.columns.len()
        && a_scan
            .columns
            .iter()
            .zip(&b_scan.columns)
            .all(|(a, b)| a.name == b.name && a.units == b.units && !a.name.starts_with("column_"))
}

// Ambiguous detector roles can leave a 9809 scan without signal candidates.
// Its crystal spacing still matters when reusing an explicit angle mapping.
fn crystal_spacing(header: &str) -> Option<f64> {
    header
        .lines()
        .find(|line| line.contains("Mono"))?
        .split_once("D=")?
        .1
        .split_whitespace()
        .next()?
        .parse()
        .ok()
}

impl MeasurementImport {
    pub fn review_batch(&mut self, id: usize, paths: Vec<PathBuf>) {
        let mut review = BatchReview {
            id,
            matching: Vec::new(),
            separate: Vec::new(),
        };
        for path in paths {
            if path == self.path {
                continue;
            }
            match read_source(&path) {
                Ok((document, bytes)) if compatible(&self.document, &document) => {
                    review.matching.push(BatchFile {
                        path,
                        digest: Sha256::digest(&bytes).into(),
                    });
                }
                Ok(_) => review
                    .separate
                    .push((path, "Different layout or conversion".into())),
                Err(error) => review.separate.push((path, error)),
            }
        }
        self.apply_to_batch = !review.matching.is_empty();
        self.batch = Some(Arc::new(review));
    }

    pub fn batch_file_count(&self) -> usize {
        1 + self.batch.as_ref().map_or(0, |b| b.matching.len())
    }

    pub fn total_import_count(&self) -> usize {
        self.import_count()
            * if self.apply_to_batch {
                self.batch_file_count()
            } else {
                1
            }
    }

    /// Reread and validate all selected files on the worker before adding any
    /// groups. A changed or invalid sibling aborts the operation with its path;
    /// it never silently disappears from the drop. Source evidence remains local.
    pub fn materialize_batch(
        &self,
        current: &ImportConfig,
    ) -> Result<Vec<DerivedSpectrum>, String> {
        let mut groups = self.materialize_import(current)?;
        if !self.apply_to_batch {
            return Ok(groups);
        }
        let Some(batch) = &self.batch else {
            return Ok(groups);
        };
        let mut reviewed = self.clone();
        reviewed.remember_config(current);
        let choice = ScanChoice::capture(&reviewed);
        for file in &batch.matching {
            let result = (|| {
                let (document, bytes) = read_source(&file.path)?;
                if <[u8; 32]>::from(Sha256::digest(&bytes)) != file.digest {
                    return Err("Source changed; reopen the import to review this batch.".into());
                }
                if !compatible(&self.document, &document) {
                    return Err("Layout changed; review this file separately.".into());
                }
                let mut source = MeasurementImport::new(file.path.clone(), document, bytes);
                choice.clone().restore(&mut source);
                source.selected_scans.clone_from(&reviewed.selected_scans);
                // Revalidate every selected mapping against this file's values.
                // The first file's validation result must never bless a sibling.
                if source.signal.is_none() {
                    source.remember_config(&source.config());
                } else {
                    let selected = source.signal;
                    for index in 0..source.configs.len() {
                        source.signal = Some(index);
                        source.remember_config(&source.config());
                    }
                    source.signal = selected;
                }
                if let Some(error) = source
                    .custom_error
                    .as_ref()
                    .filter(|_| source.signal.is_none())
                    .or_else(|| {
                        source
                            .included
                            .iter()
                            .zip(&source.candidate_errors)
                            .find_map(|(include, error)| {
                                include.then_some(error.as_ref()).flatten()
                            })
                    })
                {
                    return Err(error.clone());
                }
                source.materialize_import(&source.config())
            })();
            groups.extend(result.map_err(|error| format!("{}: {error}", file.path.display()))?);
        }
        Ok(groups)
    }
}

#[cfg(test)]
mod tests;
